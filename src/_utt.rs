/*

UTTERANCE STRUCTURE

one sentence of phone tokens from the front end, turned into
segments with their syllable, word, phrase and sentence context

*/
use std::ops::Range;

use crate::_utils::PHNIDToPhoneme;

// silent pause phone
pub const PAU: u8 = 56;

// phone classes (phone ids as in phone_to_id.txt)
pub fn is_vowel(p: u8) -> bool {
	matches!(p, 1..=8 | 13..=15 | 19..=21 | 29 | 30 | 38 | 39 | 48 | 49)
}
pub fn is_stop(p: u8) -> bool {
	matches!(p, 9 | 11 | 17 | 23 | 31 | 36 | 47)
}
pub fn is_fricative(p: u8) -> bool {
	// affricates count as fricatives here
	matches!(p, 10 | 12 | 16 | 18 | 22 | 34 | 35 | 37 | 40 | 43 | 44)
}
pub fn is_voiced(p: u8) -> bool {
	is_vowel(p) || matches!(p, 9 | 11 | 12 | 17 | 22 | 24..=28 | 32 | 33 | 40..=44 | 47)
}
pub fn is_obstruent(p: u8) -> bool {
	is_stop(p) || is_fricative(p)
}

// legal word onsets, used to split consonants between syllables
fn is_legal_onset(c: &[u8]) -> bool {
	const P: u8 = 31; const B: u8 = 9; const T: u8 = 36; const D: u8 = 11; const K: u8 = 23; const G: u8 = 17;
	const F: u8 = 16; const V: u8 = 40; const TH: u8 = 37; const S: u8 = 34; const SH: u8 = 35; const HH: u8 = 18;
	const M: u8 = 26; const N: u8 = 27; const L: u8 = 24; const R: u8 = 32; const W: u8 = 41; const Y: u8 = 42;
	match c {
		[] => true,
		[x] => !is_vowel(*x) && !matches!(*x, 25 | 28 | 33),		// not lx, ng, rx
		[S, P | T | K | M | N | L | W | F] => true,
		[P | B | K | G | F, L | R] => true,
		[T | D | TH | SH, R] => true,
		[T | D | K | G | TH, W] => true,
		[P | B | K | G | F | V | M | HH, Y] => true,
		[S, P, L | R | Y] => true,
		[S, T, R] => true,
		[S, K, L | R | W | Y] => true,
		_ => false,
	}
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Boundary { Comma, Period, Question, Exclaim, None }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind { Statement, Exclamation, YesNo, Wh }

pub struct Seg {
	pub phone:  u8,			// phone id
	pub stress: u8,			// 0, 1 primary, 2 secondary
	pub word:   usize,		// index into Utterance::words
	pub syl:    usize,		// index into Utterance::syls
	pub dur_ms: f32,		// set by the duration rules
	pub rules:  Vec<&'static str>,	// duration rules that fired (for --trace)
}
pub struct Syl {
	pub segs:    Range<usize>,
	pub nucleus: Option<usize>,		// None for a word with no vowel
	pub stress:  u8,
	pub word:    usize,
}
pub struct Word {
	pub segs:     Range<usize>,
	pub syls:     Range<usize>,
	pub function: bool,
	pub text:     String,
	pub phrase:   usize,
}
pub struct Phrase {
	pub words: Range<usize>,
	pub end:   Boundary,
}
pub struct Utterance {
	pub segs:    Vec<Seg>,
	pub syls:    Vec<Syl>,
	pub words:   Vec<Word>,
	pub phrases: Vec<Phrase>,
	pub kind:    Kind,
}

const WH_WORDS: [&str; 9] = ["who", "what", "when", "where", "why", "how", "which", "whose", "whom"];

impl Utterance {
	pub fn build(tokens: &[u8], texts: &[String]) -> Utterance {
		// texts[i] is the text of the i-th word (one per 90/95 token)
		let mut u = Utterance { segs: Vec::new(), syls: Vec::new(), words: Vec::new(), phrases: Vec::new(), kind: Kind::Statement };
		let mut phrase_start: usize = 0;
		let mut text_i: usize = 0;
		for &tk in tokens {
			let (pa, pb, sa, sb) = PHNIDToPhoneme(tk);
			match pa {
				50 | 55 => {
					u.close_word();
					u.drop_empty_word();
					let text = texts.get(text_i).cloned().unwrap_or_default();
					text_i += 1;
					let n = u.segs.len();
					let ns = u.syls.len();
					u.words.push(Word { segs: n..n, syls: ns..ns, function: pa == 55, text, phrase: u.phrases.len() });
				},
				51..=54 => {
					u.close_word();
					u.drop_empty_word();
					let end = match pa { 51 => Boundary::Comma, 52 => Boundary::Period, 53 => Boundary::Question, _ => Boundary::Exclaim };
					if u.words.len() > phrase_start {
						u.phrases.push(Phrase { words: phrase_start..u.words.len(), end });
						phrase_start = u.words.len();
					} else if let Some(last) = u.phrases.last_mut() {
						// punctuation with no words since the last phrase end replaces that end
						last.end = end;
					}
				},
				0 => {},
				_ => {
					if u.words.is_empty() || u.words.last().unwrap().phrase != u.phrases.len() || u.words.last().unwrap().segs.end != u.segs.len() {
						// phones before any word boundary, or after punctuation: start a content word
						let n = u.segs.len();
						let ns = u.syls.len();
						u.words.push(Word { segs: n..n, syls: ns..ns, function: false, text: String::new(), phrase: u.phrases.len() });
					}
					let w = u.words.len() - 1;
					for (p, s) in [(pa, sa), (pb, sb)] {
						if p == 0 { continue; }
						u.segs.push(Seg { phone: p, stress: if is_vowel(p) { s } else { 0 }, word: w, syl: 0, dur_ms: 0.0, rules: Vec::new() });
						u.words[w].segs.end = u.segs.len();
					}
				},
			}
		}
		u.close_word();
		u.drop_empty_word();
		if u.words.len() > phrase_start {
			u.phrases.push(Phrase { words: phrase_start..u.words.len(), end: Boundary::None });
		}
		u.kind = match u.phrases.last().map(|p| p.end) {
			Some(Boundary::Exclaim) => Kind::Exclamation,
			Some(Boundary::Question) => {
				let first = u.words.iter().find(|w| !w.text.is_empty()).map(|w| w.text.to_lowercase()).unwrap_or_default();
				if WH_WORDS.contains(&first.as_str()) { Kind::Wh } else { Kind::YesNo }
			},
			_ => Kind::Statement,
		};
		u
	}
	fn drop_empty_word(&mut self) {
		// a word boundary with no phones after it is not a word
		if self.words.last().is_some_and(|w| w.segs.is_empty()) { self.words.pop(); }
	}
	fn close_word(&mut self) {
		// split the last word into syllables, once it has all its phones
		let Some(w) = self.words.len().checked_sub(1) else { return; };
		if self.words[w].syls.end > self.words[w].syls.start || self.words[w].segs.is_empty() { return; }
		let segs = self.words[w].segs.clone();
		let nuclei: Vec<usize> = segs.clone().filter(|&i| is_vowel(self.segs[i].phone)).collect();
		let first_syl = self.syls.len();
		if nuclei.is_empty() {
			self.syls.push(Syl { segs: segs.clone(), nucleus: None, stress: 0, word: w });
		} else {
			let mut start = segs.start;
			for (k, &n) in nuclei.iter().enumerate() {
				let end = match nuclei.get(k + 1) {
					Some(&next) => {
						// longest legal onset before the next nucleus goes with it
						let phones: Vec<u8> = (n + 1..next).map(|i| self.segs[i].phone).collect();
						let split = (0..=phones.len()).find(|&j| is_legal_onset(&phones[j..])).unwrap();
						n + 1 + split
					},
					None => segs.end,
				};
				self.syls.push(Syl { segs: start..end, nucleus: Some(n), stress: self.segs[n].stress, word: w });
				start = end;
			}
		}
		for s in first_syl..self.syls.len() {
			for i in self.syls[s].segs.clone() { self.segs[i].syl = s; }
		}
		self.words[w].syls = first_syl..self.syls.len();
	}
	pub fn is_last_syl_of_phrase(&self, syl: usize) -> bool {
		let w = self.syls[syl].word;
		syl + 1 == self.words[w].syls.end && w + 1 == self.phrases[self.words[w].phrase].words.end
	}
	pub fn phrase_of_seg(&self, seg: usize) -> usize {
		self.words[self.segs[seg].word].phrase
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	// token ids from PHNIDToPhoneme
	const CW: u8 = 90;		// content word boundary
	const FW: u8 = 95;		// function word boundary
	const COMMA: u8 = 91;
	const PERIOD: u8 = 92;
	const QMARK: u8 = 93;
	const BANG: u8 = 94;

	fn texts(words: &[&str]) -> Vec<String> {
		words.iter().map(|w| w.to_string()).collect()
	}
	fn syl_phones(u: &Utterance, i: usize) -> Vec<u8> {
		u.segs[u.syls[i].segs.clone()].iter().map(|s| s.phone).collect()
	}

	#[test]
	fn syllabify_single_consonant_goes_right() {
		// 'ae p ax l
		let u = Utterance::build(&[CW, 16, 68, 36, 59, PERIOD], &texts(&["apple"]));
		assert_eq!(u.syls.len(), 2);
		assert_eq!(syl_phones(&u, 0), vec![2]);
		assert_eq!(syl_phones(&u, 1), vec![31, 6, 24]);
	}

	#[test]
	fn syllabify_legal_cluster() {
		// 'eh k s t r ax
		let u = Utterance::build(&[CW, 21, 56, 72, 76, 70, 36, PERIOD], &texts(&["extra"]));
		assert_eq!(syl_phones(&u, 0), vec![13, 23]);
		assert_eq!(syl_phones(&u, 1), vec![34, 36, 32, 6]);
	}

	#[test]
	fn syllabify_illegal_cluster_split() {
		// ix n s t r 'ah k t
		let u = Utterance::build(&[CW, 53, 64, 72, 76, 70, 17, 56, 76, PERIOD], &texts(&["instruct"]));
		assert_eq!(syl_phones(&u, 0), vec![20, 27]);
		assert_eq!(syl_phones(&u, 1), vec![34, 36, 32, 3, 23, 36]);
		assert_eq!(u.syls[1].stress, 1);
		assert_eq!(u.syls[0].stress, 0);
	}

	#[test]
	fn word_without_vowel() {
		// hh m
		let u = Utterance::build(&[CW, 51, 61, PERIOD], &texts(&["hmm"]));
		assert_eq!(u.syls.len(), 1);
		assert_eq!(u.syls[0].nucleus, None);
		assert_eq!(syl_phones(&u, 0), vec![18, 26]);
	}

	#[test]
	fn phrases_split_at_comma() {
		// 'ey , b 'iy .
		let u = Utterance::build(&[CW, 24, COMMA, CW, 39, 26, PERIOD], &texts(&["a", "b"]));
		assert_eq!(u.phrases.len(), 2);
		assert_eq!(u.phrases[0].end, Boundary::Comma);
		assert_eq!(u.phrases[1].end, Boundary::Period);
		assert_eq!(u.words[1].phrase, 1);
		assert!(u.is_last_syl_of_phrase(0));
		assert_eq!(u.phrase_of_seg(2), 1);
	}

	#[test]
	fn kind_detection() {
		let w = texts(&["go"]);
		let go = [CW, 48, 27];		// g 'ow
		let with = |p: u8| { let mut t = go.to_vec(); t.push(p); t };
		assert_eq!(Utterance::build(&with(PERIOD), &w).kind, Kind::Statement);
		assert_eq!(Utterance::build(&with(BANG), &w).kind, Kind::Exclamation);
		assert_eq!(Utterance::build(&with(QMARK), &w).kind, Kind::YesNo);
		assert_eq!(Utterance::build(&with(QMARK), &texts(&["Where"])).kind, Kind::Wh);
	}

	#[test]
	fn no_terminal_punctuation() {
		let u = Utterance::build(&[CW, 51, 15], &texts(&["ha"]));
		assert_eq!(u.phrases.len(), 1);
		assert_eq!(u.phrases[0].end, Boundary::None);
		assert_eq!(u.kind, Kind::Statement);
	}

	#[test]
	fn empty_trailing_word_dropped() {
		// a boundary token with no phones after the sentence end adds nothing
		let u = Utterance::build(&[CW, 56, 18, 76, PERIOD, CW], &texts(&["cat"]));
		assert_eq!(u.words.len(), 1);
		assert_eq!(u.phrases.len(), 1);
		assert_eq!(u.kind, Kind::Statement);
	}

	#[test]
	fn function_flag() {
		let u = Utterance::build(&[FW, 43, 36, CW, 56, 18, 76, PERIOD], &texts(&["the", "cat"]));
		assert!(u.words[0].function);
		assert!(!u.words[1].function);
		assert_eq!(u.words[1].text, "cat");
	}
}
