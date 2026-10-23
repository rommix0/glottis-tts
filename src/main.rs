// the code keeps the synthesis literature's names (F1, AV, m_sr, formant_tbl) over Rust's naming style
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

mod _utils;
mod _text;
mod _lts;
mod _lts_tree;
mod _morph;
mod _known;
mod _vtm;
mod _vtbl;
mod _synth;
mod _dict;
mod _dread;
mod _utt;
mod _dur;
mod _f0;

use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use _vtm::m_sr;
use hound;

use crate::_utils::cbuf;
use crate::_dread::dictreader;
use crate::_utt::{Utterance, Boundary, PAU};

type Wav = hound::WavWriter<BufWriter<File>>;

// command line options
struct Opts {
	text:  String,
	rate:  f32,
	trace: bool,
	text_only: bool,	// print the normalized text instead of speaking
	seg_dump: Option<String>,	// file for the playing segment index of every sample
}

fn parse_args() -> Result<Opts, String> {
	let mut opts = Opts { text: String::new(), rate: 1.0, trace: false, text_only: false, seg_dump: None };
	let mut have_text = false;
	let mut args = env::args().skip(1);
	while let Some(a) = args.next() {
		if a == "--trace" {
			opts.trace = true;
		} else if a == "--text" {
			opts.text_only = true;
		} else if a == "--seg-dump" {
			opts.seg_dump = Some(args.next().ok_or("--seg-dump needs a file name")?);
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

// a word the dictionary lacks, built from a known stem and suffixes (letters and apostrophes only)
fn morph_word(word: &str, dict_reader: &mut dictreader) -> Option<Vec<u8>> {
	if word.len() < 4 || !word.bytes().all(|b| b.is_ascii_alphabetic() || b == b'\'') { return None; }
	let mut lookup = |w: &str| {
		dict_reader.reset();
		for b in w.bytes() { dict_reader.readIn(b); }
		if !dict_reader.search() { return None; }
		let mut out = Vec::new();
		loop {
			let p = dict_reader.readOut();
			if p == 0xFF { break; }
			out.push(p);
		}
		Some(out)
	};
	_morph::compose(&word.to_ascii_lowercase(), &mut lookup, 2)
}

// the text normalizer's output for a whole input
fn normalize(text: &str) -> String {
	let mut buf = cbuf::new();
	buf.write(0x20);
	buf.write_text(text);
	buf.write(0x20);
	buf.write(0x00);
	let mut eng = _text::engine::new();
	let mut out = Vec::new();
	while buf.is_not_empty() {
		let done = eng.lex(&mut buf);
		while eng.is_not_empty() { out.push(eng.read()); }
		if done { break; }
	}
	String::from_utf8_lossy(&out).into_owned()
}

// the engines that run across sentences
struct Engines {
	synth:    _synth::engine,
	contour:  _f0::Contour,
	next_seg: usize,		// stream index for the next segment
	started:  bool,
	last:     u8,			// last phone loaded
	dump:     Option<BufWriter<File>>,
}

fn play(eng: &mut Engines, writer: &mut Wav, phone: u8, dur_ms: f32, stressed: bool, seg: usize, word_start: bool) {
	// feed one phone into the synthesizer and write out what it plays
	eng.last = phone;
	if eng.synth.load(phone, dur_ms.round() as i32, stressed, seg, word_start) {
		while eng.synth.is_running() {
			let (s, f) = eng.synth.position();
			let f0 = eng.contour.hz_at(s, f);
			writer.write_sample(eng.synth.sample(f0)).unwrap();
			if let Some(d) = eng.dump.as_mut() { d.write_all(&(s as u32).to_le_bytes()).unwrap(); }
		}
	}
}

fn new_engines() -> Engines {
	let mut eng = Engines {
		synth:    _synth::engine::new(),
		contour:  _f0::Contour::new(),
		next_seg: 0,
		started:  false,
		last:     0,
		dump:     None,
	};
	eng.synth.init();
	eng.synth.reset();
	eng
}

fn finish(eng: &mut Engines, writer: &mut Wav) {
	// play out what is still in the synthesizer
	let mut finish_off = true;
	while eng.synth.is_not_last() {
		let seg = take_seg(eng);
		if finish_off {
			finish_off = false;
			if eng.last == PAU {
				// the sentence already ended with its release and pause
				play(eng, writer, PAU, 50.0, false, seg, true);
			} else {
				// unpunctuated end of input: this is its release
				play(eng, writer, 48, 100.0, false, seg, true);
			}
		} else {
			play(eng, writer, 0, 50.0, false, seg, true);
		}
	}
}

fn take_seg(eng: &mut Engines) -> usize {
	let s = eng.next_seg;
	eng.next_seg += 1;
	s
}

fn synth_sentence(tokens: &[u8], texts: &[String], opts: &Opts, eng: &mut Engines, writer: &mut Wav) {
	// synthesize one sentence of phone tokens
	let mut utt = Utterance::build(tokens, texts);
	_dur::assign(&mut utt, opts.rate);

	if opts.trace {
		for w in &utt.words {
			eprintln!("WORD {} fn={} phrase={} syls={}", w.text, w.function as u8, w.phrase, w.syls.len());
		}
	}

	if !eng.started {
		eng.started = true;
		let seg = take_seg(eng);
		play(eng, writer, PAU, _dur::START_PAUSE_MS, false, seg, true);
	}

	// stream indices: the segments of each phrase, then its release and pause
	let mut stream: Vec<usize> = vec![0; utt.segs.len()];
	let mut ends: Vec<(usize, usize)> = Vec::new();		// (release, pause) per phrase
	for ph in &utt.phrases {
		let segs = utt.words[ph.words.start].segs.start..utt.words[ph.words.end - 1].segs.end;
		for i in segs { stream[i] = take_seg(eng); }
		if ph.end != Boundary::None {
			let r = take_seg(eng);
			let p = take_seg(eng);
			ends.push((r, p));
		}
	}
	eng.contour.add_utterance(&utt, &stream);

	let mut end_i = 0;
	for ph in &utt.phrases {
		let segs = utt.words[ph.words.start].segs.start..utt.words[ph.words.end - 1].segs.end;
		for i in segs.clone() {
			let s = &utt.segs[i];
			if opts.trace {
				let tg: Vec<String> = eng.contour.targets_of(stream[i]).iter().map(|t| format!("{:?}@{:.2}:{:+.1}/{:+.1}", t.kind, t.frac, t.st, t.base)).collect();
				eprintln!("TRACE {} {} seg={} dur={:.0} rules={} targets={}", s.phone, s.stress, stream[i], s.dur_ms, s.rules.join(","), tg.join(" "));
			}
			play(eng, writer, s.phone, s.dur_ms, s.stress != 0, stream[i], utt.words[s.word].segs.start == i);
		}
		if ph.end != Boundary::None {
			// phrase-end release, then the pause
			let (r, p) = ends[end_i];
			end_i += 1;
			let release = match utt.segs[segs.end - 1].phone {
				11|36 => 49,			// is alveolar. use IXP release
				_     => 48,			// otherwise, use AXP release
			};
			let pause = _dur::pause_after(ph.end, opts.rate);
			if opts.trace {
				eprintln!("TRACE {} 0 seg={} dur={:.0} rules=release", release, r, _dur::RELEASE_MS);
				eprintln!("TRACE {} 0 seg={} dur={:.0} rules=pause", PAU, p, pause);
			}
			play(eng, writer, release, _dur::RELEASE_MS, false, r, true);
			play(eng, writer, PAU, pause, false, p, true);
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

	if opts.text_only {
		println!("{}", normalize(&opts.text));
		return Ok(());
	}

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
	let mut eng = new_engines();
	if let Some(path) = &opts.seg_dump { eng.dump = Some(BufWriter::new(File::create(path)?)); }
	let mut text_engine  = _text::engine::new();

	// current sentence: phone tokens and the text of each word
	let mut tokens: Vec<u8>      = Vec::new();
	let mut texts:  Vec<String>  = Vec::new();

	let mut pout: u8;
	let mut cc: u8;
	let mut cin: u8;

	// proceed with the text normalizer/LTS pipeline
	while buf.is_not_empty() {
		if text_engine.lex(&mut buf) { break; }
		while text_engine.is_not_empty() {
			cc = text_engine.read();
			if wbuf.is_not_empty() && cc == 0x20 {
				// process word when whitespace is
				// hit and word buffer is not empty
				dict_reader.reset();
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
				} else if let Some(classes) = morph_word(&word_text, &mut dict_reader) {
					// a known stem with suffixes
					tokens.push(90);
					texts.push(word_text);
					tokens.extend(classes);
				} else {
					// if no dictionary match, then
					// do letter-to-sound stuff here!
					let mut wb_written: bool = false;
					let mut run: Vec<u8> = Vec::new();
					for &b in word_text.as_bytes().iter().chain(std::iter::once(&0u8)) {
						let code = _lts::letter_code(b);
						if code != 0 { run.push(code); continue; }
						// a run of letters ends: say it, then the punctuation that ended it
						if !run.is_empty() {
							let classes = _lts::word(&run);
							if !classes.is_empty() && !wb_written {
								// content word boundary once, before the word's first phoneme
								tokens.push(90);
								texts.push(word_text.clone());
								wb_written = true;
							}
							tokens.extend(classes);
							run.clear();
						}
						match b {
							b'!' => tokens.push(94),
							b',' => tokens.push(91),
							b'.' => tokens.push(92),
							b'?' => tokens.push(93),
							_ => {}
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
	finish(&mut eng, &mut writer);

	// flush WAV file and write to it
	writer.finalize()?;

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	// render one sentence of tokens through the driver into samples
	fn render(tokens: &[u8], texts: &[&str]) -> Vec<f32> {
		let path = env::temp_dir().join(format!("tts_synth_test_{}.wav", std::process::id()));
		let spec = hound::WavSpec { channels: 1, sample_rate: m_sr as u32, bits_per_sample: 32, sample_format: hound::SampleFormat::Float };
		let mut writer = hound::WavWriter::create(&path, spec).unwrap();
		let opts = Opts { text: String::new(), rate: 1.0, trace: false, text_only: false, seg_dump: None };
		let texts: Vec<String> = texts.iter().map(|t| t.to_string()).collect();
		let mut eng = new_engines();
		synth_sentence(tokens, &texts, &opts, &mut eng, &mut writer);
		finish(&mut eng, &mut writer);
		writer.finalize().unwrap();
		let out: Vec<f32> = hound::WavReader::open(&path).unwrap().samples::<f32>().map(|x| x.unwrap()).collect();
		let _ = std::fs::remove_file(&path);
		out
	}

	#[test]
	fn nothing_sounds_after_the_final_pause() {
		// it is .
		let out = render(&[95, 53, 76, 95, 53, 87, 92], &["it", "is"]);
		// the final pause: the last run of silence longer than 300 ms
		let silent = |x: &f32| x.abs() < 1e-4;
		let mut run = 0;
		let mut pause_end = None;
		for (i, x) in out.iter().enumerate() {
			if silent(x) { run += 1; } else { if run > m_sr * 3 / 10 { pause_end = Some(i); } run = 0; }
		}
		if let Some(i) = pause_end {
			let peak = out[i..].iter().fold(0.0f32, |m, x| m.max(x.abs()));
			panic!("sound after the final pause, peak {}", peak);
		}
	}
}
