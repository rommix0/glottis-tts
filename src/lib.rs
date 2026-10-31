// the code keeps the synthesis literature's names (F1, AV, m_sr, formant_tbl) over Rust's naming style
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
// the synthesizer needs an allocator but no operating system; main.rs is the command line around it
#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod _utils;
pub mod _text;
pub mod _text_rules;
pub mod _lts;
pub mod _lts_tree;
pub mod _morph;
pub mod _known;
pub mod _vtm;
pub mod _vtbl;
pub mod _synth;
pub mod _dict;
pub mod _dread;
pub mod _utt;
pub mod _dur;
pub mod _f0;

use alloc::{format, string::String, vec, vec::Vec};
use core::future::Future;
use _utils::cbuf;
use _dread::dictreader;
use _utt::{Utterance, Boundary, PAU};

// samples per second (mono)
pub const SAMPLE_RATE: u32 = _vtm::m_sr;

// where the application takes the speech, one signed 16-bit sample at a time; it may wait (to keep pace with an audio
// device, say), and its error ends the synthesis
pub trait WriteSample {
	type Error: core::error::Error;

	fn write_sample(&mut self, sample: i16) -> impl Future<Output = Result<(), Self::Error>>;
}

// so the application can lend its writer and keep it afterwards
impl<W: WriteSample> WriteSample for &mut W {
	type Error = W::Error;

	fn write_sample(&mut self, sample: i16) -> impl Future<Output = Result<(), Self::Error>> {
		(**self).write_sample(sample)
	}
}

// speak text into writer with the default voice; dropping the future stops it
pub async fn synthesize<W: WriteSample>(text: String, writer: W) -> Result<(), W::Error> {
	Speaker::new().synthesize(&text, writer).await
}

// where the speech goes as the synthesizer makes it, for tools that want more than synthesize gives: the float
// samples, the segment each belongs to, and the trace
pub trait Out {
	type Error;

	// one sample (-1 to 1), and the stream index of the segment playing
	fn sample(&mut self, x: f32, seg: usize) -> impl Future<Output = Result<(), Self::Error>>;
	// one line of the trace
	fn trace(&mut self, _line: &str) {}
}

// a float sample as a signed 16-bit one
pub fn to_i16(x: f32) -> i16 {
	libm::roundf(x.clamp(-1.0, 1.0) * 32767.0) as i16
}

// a WriteSample as an Out
struct Pcm<W>(W);

impl<W: WriteSample> Out for Pcm<W> {
	type Error = W::Error;

	async fn sample(&mut self, x: f32, _seg: usize) -> Result<(), W::Error> {
		self.0.write_sample(to_i16(x)).await
	}
}

// the token for one phone alone, with the most stress a vowel can carry
pub fn phone_token(p: u8) -> Option<u8> {
	if p == 0 { return None; }
	(0..=255u8).filter(|&t| matches!(_utils::PHNIDToPhoneme(t), (a, 0, _, _) if a == p))
		.max_by_key(|&t| _utils::PHNIDToPhoneme(t).2)
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
pub fn normalize(text: &str) -> String {
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
pub struct Speaker {
	pub synth: _synth::engine,
	pub rate:  f32,			// speaking rate, 0.5 to 2
	pub trace: bool,		// send each word's and segment's plan to Out::trace
	contour:   _f0::Contour,
	next_seg:  usize,		// stream index for the next segment
	started:   bool,
	last:      u8,			// last phone loaded
}

impl Speaker {
	pub fn new() -> Speaker {
		Speaker {
			synth:    _synth::engine::new(),
			rate:     1.0,
			trace:    false,
			contour:  _f0::Contour::new(),
			next_seg: 0,
			started:  false,
			last:     0,
		}
	}

	async fn play<O: Out>(&mut self, out: &mut O, phone: u8, dur_ms: f32, stressed: bool, seg: usize, word_start: bool) -> Result<(), O::Error> {
		// feed one phone into the synthesizer and send out what it plays
		self.last = phone;
		if self.synth.load(phone, libm::roundf(dur_ms) as i32, stressed, seg, word_start) {
			while self.synth.is_running() {
				let (s, f) = self.synth.position();
				let f0 = self.contour.hz_at(s, f);
				out.sample(self.synth.sample(f0), s).await?;
			}
		}
		Ok(())
	}

	// play out what is still in the synthesizer
	pub async fn finish<O: Out>(&mut self, out: &mut O) -> Result<(), O::Error> {
		let mut finish_off = true;
		while self.synth.is_not_last() {
			let seg = self.take_seg();
			if finish_off {
				finish_off = false;
				if self.last == PAU {
					// the sentence already ended with its release and pause
					self.play(out, PAU, 50.0, false, seg, true).await?;
				} else {
					// unpunctuated end of input: this is its release
					self.play(out, 48, 100.0, false, seg, true).await?;
				}
			} else {
				self.play(out, 0, 50.0, false, seg, true).await?;
			}
		}
		Ok(())
	}

	fn take_seg(&mut self) -> usize {
		let s = self.next_seg;
		self.next_seg += 1;
		s
	}

	// synthesize one sentence of phone tokens
	pub async fn sentence<O: Out>(&mut self, tokens: &[u8], texts: &[String], out: &mut O) -> Result<(), O::Error> {
		let mut utt = Utterance::build(tokens, texts);
		_dur::assign(&mut utt, self.rate);

		if self.trace {
			for w in &utt.words {
				out.trace(&format!("WORD {} fn={} phrase={} syls={}", w.text, w.function as u8, w.phrase, w.syls.len()));
			}
		}

		if !self.started {
			self.started = true;
			let seg = self.take_seg();
			self.play(out, PAU, _dur::START_PAUSE_MS, false, seg, true).await?;
		}

		// stream indices: the segments of each phrase, then its release and pause
		let mut stream: Vec<usize> = vec![0; utt.segs.len()];
		let mut ends: Vec<(usize, usize)> = Vec::new();		// (release, pause) per phrase
		for ph in &utt.phrases {
			let segs = utt.words[ph.words.start].segs.start..utt.words[ph.words.end - 1].segs.end;
			for i in segs { stream[i] = self.take_seg(); }
			if ph.end != Boundary::None {
				let r = self.take_seg();
				let p = self.take_seg();
				ends.push((r, p));
			}
		}
		self.contour.add_utterance(&utt, &stream);

		let mut end_i = 0;
		for ph in &utt.phrases {
			let segs = utt.words[ph.words.start].segs.start..utt.words[ph.words.end - 1].segs.end;
			for i in segs.clone() {
				let s = &utt.segs[i];
				if self.trace {
					let tg: Vec<String> = self.contour.targets_of(stream[i]).iter().map(|t| format!("{:?}@{:.2}:{:+.1}/{:+.1}", t.kind, t.frac, t.st, t.base)).collect();
					out.trace(&format!("TRACE {} {} seg={} dur={:.0} rules={} targets={}", s.phone, s.stress, stream[i], s.dur_ms, s.rules.join(","), tg.join(" ")));
				}
				self.play(out, s.phone, s.dur_ms, s.stress != 0, stream[i], utt.words[s.word].segs.start == i).await?;
			}
			if ph.end != Boundary::None {
				// phrase-end release, then the pause
				let (r, p) = ends[end_i];
				end_i += 1;
				let release = match utt.segs[segs.end - 1].phone {
					11|36 => 49,			// is alveolar. use IXP release
					_     => 48,			// otherwise, use AXP release
				};
				let pause = _dur::pause_after(ph.end, self.rate);
				if self.trace {
					out.trace(&format!("TRACE {} 0 seg={} dur={:.0} rules=release", release, r, _dur::RELEASE_MS));
					out.trace(&format!("TRACE {} 0 seg={} dur={:.0} rules=pause", PAU, p, pause));
				}
				self.play(out, release, _dur::RELEASE_MS, false, r, true).await?;
				self.play(out, PAU, pause, false, p, true).await?;
			}
		}
		Ok(())
	}

	// synthesize text, sentence by sentence; finish plays out the end
	pub async fn speak<O: Out>(&mut self, text: &str, out: &mut O) -> Result<(), O::Error> {
		let mut buf  = cbuf::new();				// text stream buffer
		let mut wbuf = cbuf::new();				// word buffer

		let mut dict_reader = dictreader::new();

		// read in text stream into input ring buffer
		buf.write(0x20);
		buf.write_text(text);
		buf.write(0x20);
		buf.write(0x00);

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
						for &b in word_text.as_bytes().iter().chain(core::iter::once(&0u8)) {
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
						self.sentence(&tokens, &texts, out).await?;
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
			self.sentence(&tokens, &texts, out).await?;
		}
		Ok(())
	}

	// speak text into writer, to the end
	pub async fn synthesize<W: WriteSample>(&mut self, text: &str, writer: W) -> Result<(), W::Error> {
		let mut out = Pcm(writer);
		self.speak(text, &mut out).await?;
		self.finish(&mut out).await
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::_vtm::m_sr;
	use core::{convert::Infallible, pin::{Pin, pin}, task::{Context, Poll, Waker}};
	use std::string::ToString;

	// run a future to its end, polling again whenever it waits
	fn block_on<F: Future>(f: F) -> F::Output {
		let mut f = pin!(f);
		let mut cx = Context::from_waker(Waker::noop());
		loop {
			if let Poll::Ready(r) = f.as_mut().poll(&mut cx) { return r; }
		}
	}

	// the samples and the playing segment of each
	struct Collect { x: Vec<f32>, segs: Vec<usize> }
	impl Out for Collect {
		type Error = Infallible;
		async fn sample(&mut self, x: f32, seg: usize) -> Result<(), Infallible> { self.x.push(x); self.segs.push(seg); Ok(()) }
	}

	// render one sentence of tokens into samples
	fn render(tokens: &[u8], texts: &[&str]) -> Collect {
		let texts: Vec<String> = texts.iter().map(|t| t.to_string()).collect();
		let mut out = Collect { x: Vec::new(), segs: Vec::new() };
		let mut sp = Speaker::new();
		block_on(async { sp.sentence(tokens, &texts, &mut out).await?; sp.finish(&mut out).await }).unwrap();
		out
	}

	// what an application's writer keeps
	struct Ints(Vec<i16>);
	impl WriteSample for Ints {
		type Error = Infallible;
		async fn write_sample(&mut self, sample: i16) -> Result<(), Infallible> { self.0.push(sample); Ok(()) }
	}

	// a writer that makes the synthesizer wait once before every sample, as one keeping pace with a device would
	struct Slow(Vec<i16>);
	struct WaitOnce(bool);
	impl Future for WaitOnce {
		type Output = ();
		fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<()> {
			if self.0 { return Poll::Ready(()); }
			self.0 = true;
			cx.waker().wake_by_ref();
			Poll::Pending
		}
	}
	impl WriteSample for Slow {
		type Error = Infallible;
		async fn write_sample(&mut self, sample: i16) -> Result<(), Infallible> { WaitOnce(false).await; self.0.push(sample); Ok(()) }
	}

	// a writer that fails after taking so many samples
	#[derive(Debug)]
	struct Full;
	impl core::fmt::Display for Full {
		fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result { f.write_str("full") }
	}
	impl core::error::Error for Full {}
	struct Small(usize);
	impl WriteSample for Small {
		type Error = Full;
		async fn write_sample(&mut self, _sample: i16) -> Result<(), Full> {
			if self.0 == 0 { return Err(Full); }
			self.0 -= 1;
			Ok(())
		}
	}

	#[test]
	fn synthesize_writes_the_float_output_rounded() {
		let text = "It is.";
		let mut floats = Collect { x: Vec::new(), segs: Vec::new() };
		let mut sp = Speaker::new();
		block_on(async { sp.speak(text, &mut floats).await?; sp.finish(&mut floats).await }).unwrap();
		let mut ints = Ints(Vec::new());
		block_on(synthesize(text.to_string(), &mut ints)).unwrap();
		assert!(ints.0.iter().any(|&x| x != 0), "16-bit output is silent");
		assert_eq!(ints.0, floats.x.iter().map(|&x| to_i16(x)).collect::<Vec<i16>>());
	}

	#[test]
	fn synthesize_speaks_with_voice_0_in_full() {
		let text = "Hello there, all of you.";
		let mut plain = Ints(Vec::new());
		block_on(synthesize(text.to_string(), &mut plain)).unwrap();
		let mut sp = Speaker::new();
		sp.synth.set_voice(_vtm::VOICES[0]);
		let mut voiced = Ints(Vec::new());
		block_on(sp.synthesize(text, &mut voiced)).unwrap();
		assert_eq!(plain.0, voiced.0);
	}

	#[test]
	fn a_writer_that_waits_gets_the_same_samples() {
		let (mut fast, mut slow) = (Ints(Vec::new()), Slow(Vec::new()));
		block_on(synthesize("It is.".to_string(), &mut fast)).unwrap();
		block_on(synthesize("It is.".to_string(), &mut slow)).unwrap();
		assert_eq!(fast.0, slow.0);
	}

	#[test]
	fn a_writer_error_stops_the_synthesis() {
		let mut w = Small(1000);
		assert!(block_on(synthesize("It is a long sentence.".to_string(), &mut w)).is_err());
		assert_eq!(w.0, 0);
	}

	#[test]
	fn every_spoken_phone_has_a_token_of_its_own() {
		for p in 1..=44 {
			let (a, b, s, _) = _utils::PHNIDToPhoneme(phone_token(p).unwrap());
			assert_eq!((a, b), (p, 0));
			assert!(!_utt::is_vowel(p) || s == 2 || matches!(p, 6 | 7 | 20), "phone {} stress {}", p, s);
		}
		assert!(phone_token(0).is_none() && phone_token(48).is_none());
	}

	#[test]
	fn playback_follows_the_schedule() {
		// it is , cat . : every segment plays in turn, starting where the durations before it add up to, less the
		// transition the phone before it carries in
		let (tokens, texts) = ([95, 53, 76, 95, 53, 87, 91, 90, 56, 16, 76, 92], ["it", "is", "cat"]);
		let segs: Vec<u32> = render(&tokens, &texts).segs.iter().map(|&s| s as u32).collect();
		let texts: Vec<String> = texts.iter().map(|t| t.to_string()).collect();

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
		let out = render(&[95, 53, 76, 95, 53, 87, 92], &["it", "is"]).x;
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
