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
use _utils::cbuf;
use _dread::dictreader;
use _utt::{Utterance, Boundary, PAU};

// where the speech goes
pub trait Out {
	// one sample, and the stream index of the segment playing
	fn sample(&mut self, x: f32, seg: usize);
	// one line of the trace
	fn trace(&mut self, _line: &str) {}
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

	fn play(&mut self, out: &mut dyn Out, phone: u8, dur_ms: f32, stressed: bool, seg: usize, word_start: bool) {
		// feed one phone into the synthesizer and send out what it plays
		self.last = phone;
		if self.synth.load(phone, libm::roundf(dur_ms) as i32, stressed, seg, word_start) {
			while self.synth.is_running() {
				let (s, f) = self.synth.position();
				let f0 = self.contour.hz_at(s, f);
				out.sample(self.synth.sample(f0), s);
			}
		}
	}

	// play out what is still in the synthesizer
	pub fn finish(&mut self, out: &mut dyn Out) {
		let mut finish_off = true;
		while self.synth.is_not_last() {
			let seg = self.take_seg();
			if finish_off {
				finish_off = false;
				if self.last == PAU {
					// the sentence already ended with its release and pause
					self.play(out, PAU, 50.0, false, seg, true);
				} else {
					// unpunctuated end of input: this is its release
					self.play(out, 48, 100.0, false, seg, true);
				}
			} else {
				self.play(out, 0, 50.0, false, seg, true);
			}
		}
	}

	fn take_seg(&mut self) -> usize {
		let s = self.next_seg;
		self.next_seg += 1;
		s
	}

	// synthesize one sentence of phone tokens
	pub fn sentence(&mut self, tokens: &[u8], texts: &[String], out: &mut dyn Out) {
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
			self.play(out, PAU, _dur::START_PAUSE_MS, false, seg, true);
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
				self.play(out, s.phone, s.dur_ms, s.stress != 0, stream[i], utt.words[s.word].segs.start == i);
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
				self.play(out, release, _dur::RELEASE_MS, false, r, true);
				self.play(out, PAU, pause, false, p, true);
			}
		}
	}

	// synthesize text, sentence by sentence; finish plays out the end
	pub fn speak(&mut self, text: &str, out: &mut dyn Out) {
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
						self.sentence(&tokens, &texts, out);
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
			self.sentence(&tokens, &texts, out);
		}
	}
}
