// the code keeps the synthesis literature's names (F1, AV, m_sr, formant_tbl) over Rust's naming style
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

mod _utils;
mod _text;
mod _text_rules;
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

use crate::_utils::cbuf;
use crate::_dread::dictreader;
use crate::_utt::{Utterance, Boundary, PAU};

type Wav = hound::WavWriter<BufWriter<File>>;

// mono at the synth's rate, as 16-bit integers or 32-bit floats
fn wav_spec(pcm16: bool) -> hound::WavSpec {
	let (bits, format) = if pcm16 { (16, hound::SampleFormat::Int) } else { (32, hound::SampleFormat::Float) };
	hound::WavSpec { channels: 1, sample_rate: m_sr as u32, bits_per_sample: bits, sample_format: format }
}

// command line options
struct Opts {
	text:  String,
	rate:  f32,
	trace: bool,
	text_only: bool,	// print the normalized text instead of speaking
	seg_dump: Option<String>,	// file for the playing segment index of every sample
	pcm16: bool,	// write 16-bit integer samples instead of 32-bit float
	voice: usize,	// voice profile number
}

fn parse_args() -> Result<Opts, String> {
	let mut opts = Opts { text: String::new(), rate: 1.0, trace: false, text_only: false, seg_dump: None, pcm16: false, voice: 0 };
	let mut have_text = false;
	let mut args = env::args().skip(1);
	while let Some(a) = args.next() {
		if a == "--trace" {
			opts.trace = true;
		} else if a == "--16bit" {
			opts.pcm16 = true;
		} else if a == "--text" {
			opts.text_only = true;
		} else if a == "--seg-dump" {
			opts.seg_dump = Some(args.next().ok_or("--seg-dump needs a file name")?);
		} else if a == "--voice" {
			let n = args.next().and_then(|v| v.parse().ok()).unwrap_or(usize::MAX);
			if n >= _vtm::VOICES.len() { return Err(format!("voice must be between 0 and {}", _vtm::VOICES.len() - 1)); }
			opts.voice = n;
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
			let x = eng.synth.sample(f0);
			if writer.spec().sample_format == hound::SampleFormat::Int {
				writer.write_sample((x.clamp(-1.0, 1.0) * 32767.0).round() as i16).unwrap();
			} else {
				writer.write_sample(x).unwrap();
			}
			if let Some(d) = eng.dump.as_mut() { d.write_all(&(s as u32).to_le_bytes()).unwrap(); }
		}
	}
}

fn new_engines() -> Engines {
	Engines {
		synth:    _synth::engine::new(),
		contour:  _f0::Contour::new(),
		next_seg: 0,
		started:  false,
		last:     0,
		dump:     None,
	}
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

	let mut writer = hound::WavWriter::create("tts_demo.wav", wav_spec(opts.pcm16))?;

	let mut buf  = cbuf::new();				// text stream buffer
	let mut wbuf = cbuf::new();				// word buffer

	let mut dict_reader = dictreader::new();

	// read in text stream into input ring buffer
	buf.write(0x20);
	buf.write_text(&opts.text);
	buf.write(0x20);
	buf.write(0x00);

	let mut eng = new_engines();
	eng.synth.set_voice(_vtm::VOICES[opts.voice]);
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

	writer.finalize()?;

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	// render one sentence of tokens through the driver into samples
	fn render(tokens: &[u8], texts: &[&str]) -> Vec<f32> {
		let path = render_file(tokens, texts, false, "f");
		let out: Vec<f32> = hound::WavReader::open(&path).unwrap().samples::<f32>().map(|x| x.unwrap()).collect();
		let _ = std::fs::remove_file(&path);
		out
	}

	// render one sentence of tokens into a WAV file, 16-bit or float
	fn render_file(tokens: &[u8], texts: &[&str], pcm16: bool, tag: &str) -> std::path::PathBuf {
		let path = env::temp_dir().join(format!("tts_synth_test_{}_{}.wav", std::process::id(), tag));
		let mut writer = hound::WavWriter::create(&path, wav_spec(pcm16)).unwrap();
		let opts = Opts { text: String::new(), rate: 1.0, trace: false, text_only: false, seg_dump: None, pcm16: false, voice: 0 };
		let texts: Vec<String> = texts.iter().map(|t| t.to_string()).collect();
		let mut eng = new_engines();
		synth_sentence(tokens, &texts, &opts, &mut eng, &mut writer);
		finish(&mut eng, &mut writer);
		writer.finalize().unwrap();
		path
	}

	#[test]
	fn sixteen_bit_output_is_the_float_output_rounded() {
		// it is .
		let (tokens, texts) = (&[95, 53, 76, 95, 53, 87, 92], &["it", "is"]);
		let fp = render_file(tokens, texts, false, "f32");
		let ip = render_file(tokens, texts, true, "i16");
		let mut r = hound::WavReader::open(&ip).unwrap();
		assert_eq!((r.spec().bits_per_sample, r.spec().sample_format), (16, hound::SampleFormat::Int));
		let ints: Vec<i16> = r.samples::<i16>().map(|x| x.unwrap()).collect();
		let floats: Vec<f32> = hound::WavReader::open(&fp).unwrap().samples::<f32>().map(|x| x.unwrap()).collect();
		let _ = (std::fs::remove_file(&fp), std::fs::remove_file(&ip));
		assert_eq!(ints.len(), floats.len());
		assert!(ints.iter().any(|&x| x != 0), "16-bit output is silent");
		for (i, (&a, &b)) in ints.iter().zip(&floats).enumerate() {
			assert!((a as f32 - b.clamp(-1.0, 1.0) * 32767.0).abs() <= 0.5, "sample {}: {} for {}", i, a, b);
		}
	}

	#[test]
	fn playback_follows_the_schedule() {
		// it is , cat . : every segment plays in turn, starting where the durations before it add up to, less the
		// transition the phone before it carries in
		let (tokens, texts) = ([95, 53, 76, 95, 53, 87, 91, 90, 56, 16, 76, 92], ["it", "is", "cat"]);
		let wav = env::temp_dir().join(format!("tts_synth_test_{}_sched.wav", std::process::id()));
		let dump = env::temp_dir().join(format!("tts_synth_test_{}_sched.bin", std::process::id()));
		let mut writer = hound::WavWriter::create(&wav, wav_spec(false)).unwrap();
		let opts = Opts { text: String::new(), rate: 1.0, trace: false, text_only: false, seg_dump: None, pcm16: false, voice: 0 };
		let texts: Vec<String> = texts.iter().map(|t| t.to_string()).collect();
		let mut eng = new_engines();
		eng.dump = Some(BufWriter::new(File::create(&dump).unwrap()));
		synth_sentence(&tokens, &texts, &opts, &mut eng, &mut writer);
		finish(&mut eng, &mut writer);
		eng.dump.take().unwrap().flush().unwrap();
		writer.finalize().unwrap();
		let segs: Vec<u32> = std::fs::read(&dump).unwrap().chunks(4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect();
		let _ = (std::fs::remove_file(&wav), std::fs::remove_file(&dump));

		// the schedule the driver plays: the start pause, then each phrase's segments, release and pause
		let mut utt = Utterance::build(&tokens, &texts);
		_dur::assign(&mut utt, 1.0);
		let mut durs = vec![(_dur::START_PAUSE_MS, true)];		// (ms, is a pause)
		for ph in &utt.phrases {
			let range = utt.words[ph.words.start].segs.start..utt.words[ph.words.end - 1].segs.end;
			durs.extend(range.map(|i| (utt.segs[i].dur_ms, false)));
			if ph.end != Boundary::None { durs.extend([(_dur::RELEASE_MS, false), (_dur::pause_after(ph.end, 1.0), true)]); }
		}
		let samples = |ms: f32| (ms.round() * m_sr as f32 / 1000.0) as i64;
		let ms = |n: i64| n as f32 * 1000.0 / m_sr as f32;
		assert!(segs.windows(2).all(|w| w[0] <= w[1]), "segments play out of order");
		let mut due = 0;
		for (k, &(d, _)) in durs.iter().enumerate() {
			let start = segs.iter().position(|&s| s == k as u32).unwrap_or_else(|| panic!("segment {} never plays", k)) as i64;
			let off = ms(start - due);
			if k > 0 {
				let (prev, after_pause) = durs[k - 1];
				// a pause carries nothing over, so the timeline is back on schedule after one
				if after_pause { assert!(off.abs() <= 1.0, "segment {} after a pause starts {} ms off", k, off); }
				assert!(off <= 1.0 && off >= -prev, "segment {} starts {} ms off (the phone before lasts {} ms)", k, off, prev);
			}
			due += samples(d);
		}
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

	// the normalizer's output with spaces collapsed
	fn says(t: &str) -> String { normalize(t).split_whitespace().collect::<Vec<_>>().join(" ") }

	#[test]
	fn long_input_does_not_crash() {
		let long = "Dr. Smith paid $45.50 for 3 km of road. ".repeat(250);
		assert!(says(&long).matches("doctor smith").count() == 250);
		let digits = "7".repeat(3000);
		assert_eq!(says(&digits).matches("seven").count(), 3000);
	}

	#[test]
	fn units_follow_the_number() {
		assert!(says("It is 100 km away.").contains("one hundred kilometers"), "{}", says("It is 100 km away."));
		assert!(says("It is 1 km away.").contains("one kilometer away"), "{}", says("It is 1 km away."));
	}

	#[test]
	fn times_and_ranges() {
		assert!(says("10:30 am").contains("ten thirty A m"), "{}", says("10:30 am"));
		assert!(says("at 3pm").contains("three p m"), "{}", says("at 3pm"));
		assert!(says("at 5 a.m. today").contains("five A m today"), "{}", says("at 5 a.m. today"));
		assert!(says("pages 3-5").contains("three through five"), "{}", says("pages 3-5"));
	}

	#[test]
	fn years_and_decades() {
		for (t, want) in [("In 1999 we", "nineteen ninety nine"), ("In 1905 we", "nineteen oh five"),
			("In 1900 we", "nineteen hundred"), ("In 2026 we", "twenty twenty six"), ("In 2005 we", "two thousand five"),
			("the 1990s", "nineteen nineties"), ("the '90s", "the nineties"), ("the 2000s", "two thousands"),
			("the 1900s", "nineteen hundreds"), ("it cost $1999", "one thousand nine hundred ninety nine dollars"),
			("1,999 people", "one thousand nine hundred ninety nine people")] {
			assert!(says(t).contains(want), "{} -> {}", t, says(t));
		}
	}

	#[test]
	fn dates_fractions_and_money() {
		for (t, want) in [("on 10/23/2026.", "october twenty third , twenty twenty six"), ("on 1/5/99.", "january fifth , ninety nine"),
			("add 1/2 cup", "one half cup"), ("add 3/4 cup", "three quarters cup"), ("a 5/8 inch", "five eighths inch"),
			("add 1/3 cup", "one third cup"), ("It was $1.5 million.", "one point five million dollars"),
			("and $20 billion.", "twenty billion dollars"), ("on Jan. 5 we", "january fifth we"), ("on March 3 we", "march third we")] {
			assert!(says(t).contains(want), "{} -> {}", t, says(t));
		}
	}

	#[test]
	fn abbreviations_keep_the_sentence_end() {
		for (t, want) in [("He lives on Elm St.", "elm street ."), ("St. Louis is far.", "saint louis is far"),
			("They met at the IEEE.", "i e e e ."), ("He got a Grade A.", "grade A ."), ("Visit www.example.com now.", "w w w dot example dot com")] {
			assert!(says(t).contains(want), "{} -> {}", t, says(t));
		}
	}

	#[test]
	fn capitals_numerals_and_quotes() {
		for (t, want) in [("NASA and the", "nasa and the"), ("HELLO WORLD", "hello world"), ("a PAT. PEND. sign", "patent pending"), ("Chapter IV begins", "chapter four begins"),
			("World War II ended", "world war two ended"), ("Henry VIII was", "henry the eighth was"), ("'Twas the night", "'twas the night"),
			("'Hello,' she said", "hello , ' she said"), ("A cat sat.", "a cat sat"), ("She left. \"Bye,\" he said.", "she left . bye ,"), ("He said \"hi\" to me.", "he said hi"), ("A B C.", "A b c"), ("Hi :) there", "hi happy face there")] {
			assert!(says(t).contains(want), "{} -> {}", t, says(t));
		}
	}

	#[test]
	fn times_web_addresses_and_versions() {
		for (t, want) in [("at 10:05 today", "ten oh five today"), ("at 3:07 pm", "three oh seven p m"), ("Wow :0 there", "wow shocked face there"),
			("Visit https://example.com/page now.", "visit example dot com slash page now"), ("See http://a.org.", "see a dot org ."),
			("on 2026-10-23.", "october twenty third , twenty twenty six"), ("Version 2.0.1 is out.", "version two point oh point one is out")] {
			assert!(says(t).contains(want), "{} -> {}", t, says(t));
		}
	}

	#[test]
	fn units_stay_out_of_words() {
		for t in ["background", "bookmark", "acme", "a kgb agent", "kganakga"] {
			assert_eq!(says(t), t);
		}
		assert!(says("5 kg").contains("five kilograms"), "{}", says("5 kg"));
		assert!(says("5kg of rice").contains("five kilograms of rice"), "{}", says("5kg of rice"));
	}

	#[test]
	fn units_after_numbers() {
		for (t, want) in [("5ft tall", "five feet tall"), ("1 ft", "one foot"), ("6ft2in", "six feet two inches"), ("2hrs later", "two hours later"),
			("30mins", "thirty minutes"), ("50mg dose", "fifty milligrams dose"), ("5kgs", "five kilograms"), ("2.5kg", "two point five kilograms"),
			("12oz can", "twelve ounces can"), ("1 mph", "one mile per hour"), ("1,000 ft", "one thousand feet"), ("10ms delay", "ten milliseconds delay"),
			("100km/h", "one hundred kilometers per hour"), ("1 km/h", "one kilometer per hour"), ("100 km/hr", "one hundred kilometers per hour"),
			("6 in a row", "six in a row"), ("5 minutes", "five minutes"), ("3 secrets", "three secrets")] {
			assert!(says(t).contains(want), "{} -> {}", t, says(t));
		}
	}

	#[test]
	fn pounds_euros_yen_and_heights() {
		for (t, want) in [("It costs \u{a3}20 now", "twenty pounds now"), ("just \u{a3}1 each", "one pound each"), ("about \u{20ac}15 now", "fifteen euros now"),
			("\u{20ac}1.50 each", "one euro and fifty cents each"), ("\u{a3}3.50 each", "three pounds and fifty pence each"), ("\u{a5}500 now", "five hundred yen now"), ("only \u{20ac}0.99 each", "only ninety nine cents each"),
			("a \u{a3}2 million deal", "two million pounds deal"), ("He's 6'2\" tall.", "six foot two tall"), ("She is 5'10\" tall.", "five foot ten tall"),
			("It costs 12.50 dollars.", "twelve dollars and fifty cents ."), ("only 1.01 euros", "one euro and one cent"), ("0.25 dollars", "twenty five cents"),
			("3.00 dollars", "three dollars"), ("It rose 1.50 percent.", "one point five oh percent")] {
			assert!(says(t).contains(want), "{} -> {}", t, says(t));
		}
	}

	#[test]
	fn a_closing_quote_keeps_its_punctuation() {
		for (t, want) in [("She said \"Hello.\" and left.", "she said hello , and left ."), ("He asked \"Why?\" and left.", "why ? and left ."),
			("She left. \"Bye,\" he said.", "she left . bye ,")] {
			assert!(says(t).contains(want), "{} -> {}", t, says(t));
		}
		assert_eq!(says("He said \"Bye.\""), "he said bye .");
	}

	#[test]
	fn years_by_context() {
		for (t, want) in [("There were 2015 votes.", "two thousand fifteen votes"), ("In 2015 votes were cast", "twenty fifteen votes"),
			("He was born in 1066.", "ten sixty six"), ("the 2020 election", "twenty twenty election"), ("1066 people came", "one thousand sixty six people"),
			("It ran 1500 miles", "one thousand five hundred miles"), ("In 1999 we", "nineteen ninety nine"), ("1999 was good", "nineteen ninety nine was")] {
			assert!(says(t).contains(want), "{} -> {}", t, says(t));
		}
	}

	#[test]
	fn homographs_follow_their_context() {
		// the capitalized spelling selects the dictionary's other reading
		for (t, want) in [("I will record a record.", "i will RECORD a record ."), ("I object to that object.", "i object to that OBJECT"),
			("We present a present.", "we present a PRESENT"), ("Please close it, it is close.", "please close it , it is CLOSE"),
			("The subject is hard.", "the SUBJECT is hard"), ("Prices will increase after the increase.", "will increase after thee INCREASE"),
			("They export grain.", "they EXPORT grain"), ("a separate room", "a SEPARATE room"), ("to separate them", "to separate them"),
			("an estimate", "an ESTIMATE"), ("to estimate it", "to estimate it"), ("The project is late.", "the PROJECT is late"),
			("They project growth.", "they project growth"), ("It affects us.", "it AFFECTS us"), ("He records it.", "he RECORDS it"),
			("Objects fell.", "OBJECTS fell"), ("the objective", "thee objective")] {
			assert!(says(t).contains(want), "{} -> {}", t, says(t));
		}
	}

	#[test]
	fn a_run_of_marks_is_one_mark() {
		// a question if any of them asks, else an exclamation; one mark, so one sentence
		for (t, want) in [("You did that?!", "you did that ?"), ("You did that!?", "you did that ?"), ("No way!!!", "no way !"),
			("Really??", "really ?"), ("Seriously\u{203d}", "seriously ?"), ("What\u{2049}", "what ?")] {
			assert_eq!(says(t), want, "{}", t);
		}
		// a mark after a space is still read out
		assert!(says("Is it ? yes").contains("question mark"), "{}", says("Is it ? yes"));
	}
}
