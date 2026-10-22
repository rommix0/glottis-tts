mod _utils;
mod _text;
mod _duration_new;
//mod _duration;
//mod _pitch;
mod _smooth;
mod _lts;
mod _vtm;
mod _vtbl;
mod _synth;
mod _dict;
mod _dread;
mod _utt;
mod _dur;

use std::env;
use std::fs::File;
use std::io::BufWriter;
use _vtm::m_sr;
use hound;

use crate::_utils::cbuf;
use crate::_utils::PHNIDToPhoneme;
use crate::_dread::dictreader;
use crate::_lts::lts;
use crate::_utt::{Utterance, Boundary, PAU};

type Wav = hound::WavWriter<BufWriter<File>>;

// command line options
struct Opts {
	text:  String,
	rate:  f32,
	trace: bool,
}

fn parse_args() -> Result<Opts, String> {
	let mut opts = Opts { text: String::new(), rate: 1.0, trace: false };
	let mut have_text = false;
	let mut args = env::args().skip(1);
	while let Some(a) = args.next() {
		if a == "--trace" {
			opts.trace = true;
		} else if a == "--rate" {
			let r: f32 = args.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
			if !(0.5..=2.0).contains(&r) { return Err("rate must be between 0.5 and 2.0".to_string()); }
			opts.rate = r;
		} else if !have_text {
			opts.text = a;
			have_text = true;
		}
	}
	if !have_text { return Err("please enter text!".to_string()); }
	Ok(opts)
}

// the engines that run across sentences
struct Engines {
	synth:   _synth::engine,
	dur:     _duration_new::model,
	pit:     _smooth::model,
	started: bool,
}

fn play(eng: &mut Engines, writer: &mut Wav, phone: u8, dur_ms: f32, stressed: bool, pitch: f32) {
	// feed one phone into the synthesizer and write out what it plays
	if eng.synth.load(phone, dur_ms.round() as i32, stressed, pitch) {
		while eng.synth.is_running() {
			eng.pit.set(eng.synth.pit_c);
			let f0 = eng.pit.step();
			writer.write_sample(eng.synth.sample(f0)).unwrap();
		}
	}
}

fn cart_pitch(tokens: &[u8], eng: &mut Engines) -> Vec<(u8, f32)> {
	// run the trained model over the sentence for its pitch: (phone, pitch) per phone and punctuation
	let mut tk: Vec<u8> = tokens.to_vec();
	tk.push(90);
	let mut out: Vec<(u8, f32)> = Vec::new();
	let mut pa: u8;
	let mut pb: u8;
	let mut sa: u8;
	let mut sb: u8;
	let mut past_accent: u8 = 0;
	let mut curr_accent: u8 = 0;
	let phn_sz: usize = tk.len();
	let mut phn_cnt: usize = 0;
	let mut next = tk.iter();

	eng.dur.reset();
	while eng.dur.is_not_last() {
		pa = 0;
		pb = 0;
		sa = 0;
		sb = 0;
		if let Some(&t) = next.next() {
			(pa, pb, sa, sb) = PHNIDToPhoneme(t);
			phn_cnt += 1;
		}
		loop {
			eng.dur.step(pa, sa, past_accent, curr_accent, phn_cnt, phn_sz);
			if eng.dur.is_ready() {
				let (_, phone, _, _, pitch) = eng.dur.fetch();
				if phone != 50 && phone != 55 {
					out.push((phone, pitch));
				} else {
					past_accent = curr_accent;
					// content words are accented, function words are left alone
					curr_accent = if phone == 50 { 1 } else { 0 };
				}
			}
			pa = pb;
			sa = sb;
			pb = 0;
			sb = 0;
			if pa == 0 { break; }
		}
	}
	out
}

fn synth_sentence(tokens: &[u8], texts: &[String], opts: &Opts, eng: &mut Engines, writer: &mut Wav) {
	// synthesize one sentence of phone tokens
	let mut utt = Utterance::build(tokens, texts);
	_dur::assign(&mut utt, opts.rate);
	let pitches = cart_pitch(tokens, eng);

	if opts.trace {
		for w in &utt.words {
			eprintln!("WORD {} fn={} phrase={} syls={}", w.text, w.function as u8, w.phrase, w.syls.len());
		}
	}

	// pitch for each segment, and for each phrase end, from the trained model's output order
	let mut seg_pitch: Vec<f32> = Vec::new();
	let mut end_pitch: Vec<f32> = Vec::new();
	for &(phone, pitch) in &pitches {
		if (51..=54).contains(&phone) { end_pitch.push(pitch); } else if phone != 0 { seg_pitch.push(pitch); }
	}
	let last_pitch = seg_pitch.last().copied().unwrap_or(110.0);

	if !eng.started {
		eng.started = true;
		play(eng, writer, PAU, _dur::START_PAUSE_MS, false, seg_pitch.first().copied().unwrap_or(110.0));
	}

	for (pi, ph) in utt.phrases.iter().enumerate() {
		let segs = utt.words[ph.words.start].segs.start..utt.words[ph.words.end - 1].segs.end;
		for i in segs.clone() {
			let s = &utt.segs[i];
			let pitch = seg_pitch.get(i).copied().unwrap_or(last_pitch);
			if opts.trace { eprintln!("TRACE {} {} dur={:.0} f0={:.0} rules={}", s.phone, s.stress, s.dur_ms, pitch, s.rules.join(",")); }
			play(eng, writer, s.phone, s.dur_ms, s.stress != 0, pitch);
		}
		if ph.end != Boundary::None {
			// phrase-end release, then the pause
			let pitch = end_pitch.get(pi).copied().unwrap_or(last_pitch);
			let release = match utt.segs[segs.end - 1].phone {
				11|36 => 49,			// is alveolar. use IXP release
				_     => 48,			// otherwise, use AXP release
			};
			let pause = _dur::pause_after(ph.end, opts.rate);
			if opts.trace { eprintln!("TRACE {} 0 dur={:.0} f0={:.0} rules=release", release, _dur::RELEASE_MS, pitch); }
			if opts.trace { eprintln!("TRACE {} 0 dur={:.0} f0={:.0} rules=pause", PAU, pause, pitch); }
			play(eng, writer, release, _dur::RELEASE_MS, false, pitch);
			play(eng, writer, PAU, pause, false, pitch);
		}
	}
}

fn main() -> Result<(), Box<dyn std::error::Error>> {

	// read in options and text from arguments
	let opts = match parse_args() {
		Ok(o) => o,
		Err(e) => {
			println!("{}", e);
			std::process::exit(1);
		}
	};

	// setup wav file stuff here
	let spec = hound::WavSpec {
		channels: 1,
		sample_rate: m_sr as u32,
		bits_per_sample: 32,
		sample_format: hound::SampleFormat::Float,
	};
	let mut writer = hound::WavWriter::create("tts_demo.wav", spec)?;

	// initialize ring buffers
	let mut buf  = cbuf::new();				// text stream buffer
	let mut wbuf = cbuf::new();				// word buffer

	// initialize dictionary reader
	let mut dict_reader = dictreader::new();

	// read in text stream into input ring buffer
	buf.write(0x20);
	buf.write_text(&opts.text);
	buf.write(0x20);
	buf.write(0x00);

	// initialize engines
	let mut eng = Engines {
		synth: _synth::engine::new(),
		dur:   _duration_new::model::new(),
		pit:   _smooth::model::new(),
		started: false,
	};
	let mut text_engine  = _text::engine::new();
	let mut lts_engine   = _lts::pvec::new();
	eng.synth.init();
	eng.pit.reset();
	eng.synth.reset();

	// current sentence: phone tokens and the text of each word
	let mut tokens: Vec<u8>      = Vec::new();
	let mut texts:  Vec<String>  = Vec::new();

	let mut l4: u8;
	let mut l3: u8;
	let mut l2: u8;
	let mut l1: u8;
	let mut c:  u8;
	let mut r1: u8;
	let mut r2: u8;
	let mut r3: u8;
	let mut r4: u8;
	let mut pout: u8;
	let mut prev: u8;
	let mut cc: u8;
	let mut cin: u8;
	let mut yyreset: usize;

	// proceed with the text normalizer/LTS pipeline
	while buf.is_not_empty() {
		if text_engine.lex(&mut buf) { break; }
		while text_engine.is_not_empty() {
			cc = text_engine.read();
			if wbuf.is_not_empty() && cc == 0x20 {
				// process word when whitespace is
				// hit and word buffer is not empty
				dict_reader.reset();
				yyreset = wbuf.getcurs();
				let mut word_text = String::new();
				while wbuf.is_not_empty() {
					// read word into dictionary reader
					cin = wbuf.read();
					wbuf.advance();
					if cin == 0x20 { continue; }
					dict_reader.readIn(cin);
					word_text.push(cin as char);
				}

				if dict_reader.search() {
					// get phoneme stream for word
					// if found in the dictionary
					if dict_reader.isFunctionWord() {
						// function word boundary
						tokens.push(95);
					} else {
						// content word boundary
						tokens.push(90);
					}
					texts.push(word_text);

					// retrieve the phoneme sequence from dictionary
					// until null character 0xFF is hit
					loop {
						pout = dict_reader.readOut();
						if pout == 0xFF { break; }
						tokens.push(pout);
					}
				} else {
					// if no dictionary match, then
					// do letter-to-sound stuff here!
					lts_engine.reset();
					wbuf.setcurs(yyreset);
					prev = 0;
					let mut wb_written: bool = false;
					while lts_engine.is_not_last() {
						cin = wbuf.read();
						wbuf.advance();
						(l4, l3, l2, l1, c, r1, r2, r3, r4) = lts_engine.step(cin);
						if lts_engine.is_ready() {
							if c == 28 {
								// exclamation point
								tokens.push(94);
							} else if c == 29 {
								// comma
								tokens.push(91);
							} else if c == 30 {
								// period
								tokens.push(92);
							} else if c == 31 {
								// question mark
								tokens.push(93);
							} else {
								// normal case
								pout = lts(l4, l3, l2, l1, c, r1, r2, r3, r4);
								if pout != 31 && pout != prev {
									// content word boundary once, before the word's first phoneme
									if !wb_written {
										tokens.push(90);
										texts.push(word_text.clone());
										wb_written = true;
									}
									// write out phoneme when not null and not a run-on
									tokens.push(pout);
									prev = pout;
								}
							}
						}
					}
				}

				// synthesize when the sentence is complete
				if matches!(tokens.last(), Some(92..=94)) {
					synth_sentence(&tokens, &texts, &opts, &mut eng, &mut writer);
					tokens.clear();
					texts.clear();
				}
			} else {
				// write to word buffer if not ready to parse yet
				wbuf.write(cc);
			}
		}
	}

	// synthesize whatever is left at the end of input
	if !tokens.is_empty() {
		synth_sentence(&tokens, &texts, &opts, &mut eng, &mut writer);
	}

	// if formant synth not done, let's finish up the synthesis
	let mut finish_off = true;
	while eng.synth.is_not_last() {
		let ready = if finish_off {
			finish_off = false;
			eng.synth.load(48, 100, false, 50.0)
		} else {
			eng.synth.load(0, 50, false, 50.0)
		};
		if ready {
			while eng.synth.is_running() {
				let f0 = eng.pit.step();
				writer.write_sample(eng.synth.sample(f0)).unwrap();
			}
		}
	}

	// flush WAV file and write to it
	writer.finalize()?;

	Ok(())
}
