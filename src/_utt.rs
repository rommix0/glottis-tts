/*

UTTERANCE STRUCTURE

one sentence of phone tokens from the front end, turned into
segments with their syllable, word, phrase and sentence context

*/
use core::ops::Range;

use crate::_utils::PHNIDToPhoneme;
use alloc::{string::String, vec, vec::Vec};

// silent pause phone
pub const PAU: u8 = 56;

// phone classes
pub fn is_vowel(p: u8)     -> bool { matches!(p, 1..=8 | 13..=15 | 19..=21 | 29 | 30 | 38 | 39 | 48 | 49) }
pub fn is_stop(p: u8)      -> bool { matches!(p, 9 | 11 | 17 | 23 | 31 | 36 | 47) }
pub fn is_fricative(p: u8) -> bool { matches!(p, 10 | 12 | 16 | 18 | 22 | 34 | 35 | 37 | 40 | 43 | 44) }
pub fn is_voiced(p: u8)    -> bool { is_vowel(p) || matches!(p, 9 | 11 | 12 | 17 | 22 | 24..=28 | 32 | 33 | 40..=44 | 47) }
pub fn is_obstruent(p: u8) -> bool { is_stop(p) || is_fricative(p) }
pub fn is_sonorant(p: u8)  -> bool { is_vowel(p) || matches!(p, 24..=28 | 32 | 33 | 41 | 42) }

// the glide a vowel ends on: w after ow uw aw, y after iy ey ay oy
pub fn offglide(p: u8) -> Option<u8> {
	match p { 29 | 39 | 5 => Some(41), 21 | 15 | 8 | 30 => Some(42), _ => None }
}

// legal word onsets, used to split consonants between syllables
fn is_legal_onset(cluster: &[u8]) -> bool {
	const P: u8 = 31; const B: u8 = 9;  const T: u8 = 36;  const D: u8 = 11; const K: u8 = 23;  const G: u8 = 17;
	const F: u8 = 16; const V: u8 = 40; const TH: u8 = 37; const S: u8 = 34; const SH: u8 = 35; const HH: u8 = 18;
	const M: u8 = 26; const N: u8 = 27; const L: u8 = 24;  const R: u8 = 32; const W: u8 = 41;  const Y: u8 = 42;
	match cluster {
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
pub enum Boundary { Comma, Period, Question, Exclaim, Minor, Unmarked }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind { Statement, Exclamation, YesNo, Wh }

pub struct Seg {
	pub phone:  u8,					// phone id
	pub stress: u8,					// 0, 1 primary, 2 secondary
	pub word:   usize,				// index into Utterance::words
	pub syl:    usize,				// index into Utterance::syls
	pub dur_ms: f32,				// set by the duration rules
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

// a phrase longer than this many syllables gets a minor break, leaving pieces of at least MIN_PIECE_SYL
const MAX_PHRASE_SYL: usize = 14;
const MIN_PIECE_SYL:  usize = 5;
// the break goes before one of these words best ...
const BREAK_BEFORE: [&str; 23] = [
	"and", "but", "or", "nor", "so", "yet", "because", "while",
	"although", "though", "unless", "whereas", "until", "if", "when", "whether",
	"since", "that", "which", "who", "whom", "whose", "where"
];
// ... else before a preposition ("to" is too often an infinitive's or a compound's, "text to speech")
const PREPOSITIONS: [&str; 31] = [
	"in", "on", "at", "by", "near", "over", "under", "after",
	"before", "with", "without", "from", "into", "onto", "through", "during",
	"across", "about", "against", "along", "among", "around", "behind", "below",
	"beside", "between", "beyond", "toward", "towards", "upon", "within"
];
const WH_WORDS: [&str; 9] = [
	"who", "what", "when",
	"where", "why", "how",
	"which", "whose", "whom"
];

impl Utterance {
	pub fn build(tokens: &[u8], texts: &[impl AsRef<str>]) -> Utterance {
		// texts[i] is the text of the i-th word (one per 90/95 token)
		let mut u = Utterance { segs: Vec::new(), syls: Vec::new(), words: Vec::new(), phrases: Vec::new(), kind: Kind::Statement };
		let mut phrase_start: usize = 0;
		let mut text_i: usize = 0;
		for &tk in tokens {
			let (pa, pb, sa, sb) = PHNIDToPhoneme(tk);
			match pa {
				50 | 55 => {
					u.close_word();
					let text = String::from(texts.get(text_i).map_or("", AsRef::as_ref));
					text_i += 1;
					let n  = u.segs.len();
					let ns = u.syls.len();
					u.words.push(Word { segs: n..n, syls: ns..ns, function: pa == 55, text, phrase: u.phrases.len() });
				},
				51..=54 => {
					u.close_word();
					let end = match pa {
						51 => Boundary::Comma,
						52 => Boundary::Period,
						53 => Boundary::Question,
						_  => Boundary::Exclaim
					};
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
					if u.words.last().is_none_or(|w| w.phrase != u.phrases.len()) {
						// phones before any word boundary, or after punctuation: start a content word
						let n = u.segs.len();
						let ns = u.syls.len();
						u.words.push(Word { segs: n..n, syls: ns..ns, function: false, text: String::new(), phrase: u.phrases.len() });
					}
					let w = u.words.len() - 1;
					for (p, s) in [(pa, sa), (pb, sb)] {
						if p == 0 { continue; }
						// a vowel ending on a w or y glides through it into the next vowel of its word ("follower", "being")
						let prev = u.segs.last().filter(|x| x.word == w).map(|x| x.phone);
						if is_vowel(p) && let Some(g) = prev.and_then(offglide) {
							u.segs.push(Seg { phone: g, stress: 0, word: w, syl: 0, dur_ms: 0.0, rules: Vec::new() });
						}
						u.segs.push(Seg { phone: p, stress: if is_vowel(p) { s } else { 0 }, word: w, syl: 0, dur_ms: 0.0, rules: Vec::new() });
						u.words[w].segs.end = u.segs.len();
					}
				},
			}
		}
		u.close_word();
		if u.words.len() > phrase_start {
			u.phrases.push(Phrase { words: phrase_start..u.words.len(), end: Boundary::Unmarked });
		}
		// a sentence does not end on a comma: one at the end of the input ends it
		if let Some(last) = u.phrases.last_mut() {
			if last.end == Boundary::Comma { last.end = Boundary::Period; }
		}
		u.break_long_phrases();
		u.flap();
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
	fn break_long_phrases(&mut self) {
		let old = core::mem::take(&mut self.phrases);
		for ph in old {
			let pieces = self.pieces(ph.words, ph.end);
			self.phrases.extend(pieces);
		}
		for (k, ph) in self.phrases.iter().enumerate() {
			for w in ph.words.clone() { self.words[w].phrase = k; }
		}
	}
	fn pieces(&self, words: Range<usize>, end: Boundary) -> Vec<Phrase> {
		// a phrase with too many syllables to say in one go breaks after a content word, before its best next word:
		// a conjunction or relative word, else a preposition, the nearer the middle the better; never before another
		// function word, which would split a verb from its object ("using | the") or a noun from "of"
		let syls = |r: Range<usize>| -> usize { r.map(|w| self.words[w].syls.len()).sum() };
		let total = syls(words.clone());
		if total <= MAX_PHRASE_SYL { return vec![Phrase { words, end }]; }
		let mut best: Option<(f32, usize)> = None;
		for j in words.start + 1..words.end {
			let left = syls(words.start..j);
			if left < MIN_PIECE_SYL || total - left < MIN_PIECE_SYL || self.words[j - 1].function { continue; }
			let text = self.words[j].text.to_lowercase();
			let score = if BREAK_BEFORE.contains(&text.as_str()) { 3.0 } else if PREPOSITIONS.contains(&text.as_str()) { 2.0 }
				else { continue };
			let value = score - 2.0 * (left as f32 / total as f32 - 0.5).abs();
			if best.is_none_or(|(v, _)| value > v) { best = Some((value, j)); }
		}
		let Some((_, j)) = best else { return vec![Phrase { words, end }] };
		let mut out = self.pieces(words.start..j, Boundary::Minor);
		out.extend(self.pieces(j..words.end, end));
		out
	}
	fn flap(&mut self) {
		// a t or d after a vowel or r is said as a flap before an unstressed vowel in its word ("city", "rider"),
		// or ending its word before a vowel in the same phrase ("got it"); a word's first stop, a stop before a
		// stressed vowel and a t before a word-final syllabic n ("button") keep their closure
		const T: u8 = 36; const D: u8 = 11; const DX: u8 = 47; const N: u8 = 27; const RX: u8 = 33;
		fn vowel(p: u8) -> bool { is_vowel(p) && !matches!(p, 48 | 49) }
		for i in 1..self.segs.len().saturating_sub(1) {
			let (prev, cur, next) = (&self.segs[i - 1], &self.segs[i], &self.segs[i + 1]);
			if !matches!(cur.phone, T | D) || prev.word != cur.word || !(vowel(prev.phone) || prev.phone == RX) { continue; }
			let w = &self.words[cur.word];
			let flaps = if next.word == cur.word {
				let syllabic_n = matches!(next.phone, 6 | 20) && i + 3 == w.segs.end && self.segs[i + 2].phone == N;
				vowel(next.phone) && next.stress == 0 && !syllabic_n
			} else {
				vowel(next.phone) && self.words[next.word].phrase == w.phrase && self.words[next.word].segs.start == i + 1
			};
			if flaps { self.segs[i].phone = DX; }
		}
	}
	fn close_word(&mut self) {
		// split the last word into syllables, once it has all its phones; a word boundary with no phones after it
		// is not a word
		let Some(w) = self.words.len().checked_sub(1) else { return; };
		if self.words[w].segs.is_empty() { self.words.pop(); return; }
		if self.words[w].syls.end > self.words[w].syls.start { return; }
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

	fn syl_phones(u: &Utterance, i: usize) -> Vec<u8> {
		u.segs[u.syls[i].segs.clone()].iter().map(|s| s.phone).collect()
	}

	#[test]
	fn a_w_glides_between_vowels() {
		// f 'aa lx ow axr: ow glides through a w into axr, which starts the last syllable
		let u = Utterance::build(&[CW, 46, 15, 60, 66, 37, PERIOD], &["follower"]);
		assert_eq!(u.segs.iter().map(|s| s.phone).collect::<Vec<_>>(), vec![16, 1, 25, 29, 41, 7]);
		assert_eq!(syl_phones(&u, 2), vec![41, 7]);
		// not across words
		let u = Utterance::build(&[CW, 66, CW, 32, PERIOD], &["owe", "aa"]);
		assert_eq!(u.segs.len(), 2);
	}

	#[test]
	fn syllabify_single_consonant_goes_right() {
		// 'ae p ax l
		let u = Utterance::build(&[CW, 16, 68, 36, 59, PERIOD], &["apple"]);
		assert_eq!(u.syls.len(), 2);
		assert_eq!(syl_phones(&u, 0), vec![2]);
		assert_eq!(syl_phones(&u, 1), vec![31, 6, 24]);
	}

	#[test]
	fn syllabify_legal_cluster() {
		// 'eh k s t r ax
		let u = Utterance::build(&[CW, 21, 56, 72, 76, 70, 36, PERIOD], &["extra"]);
		assert_eq!(syl_phones(&u, 0), vec![13, 23]);
		assert_eq!(syl_phones(&u, 1), vec![34, 36, 32, 6]);
	}

	#[test]
	fn syllabify_illegal_cluster_split() {
		// ix n s t r 'ah k t
		let u = Utterance::build(&[CW, 53, 64, 72, 76, 70, 17, 56, 76, PERIOD], &["instruct"]);
		assert_eq!(syl_phones(&u, 0), vec![20, 27]);
		assert_eq!(syl_phones(&u, 1), vec![34, 36, 32, 3, 23, 36]);
		assert_eq!(u.syls[1].stress, 1);
		assert_eq!(u.syls[0].stress, 0);
	}

	#[test]
	fn word_without_vowel() {
		// hh m
		let u = Utterance::build(&[CW, 51, 61, PERIOD], &["hmm"]);
		assert_eq!(u.syls.len(), 1);
		assert_eq!(u.syls[0].nucleus, None);
		assert_eq!(syl_phones(&u, 0), vec![18, 26]);
	}

	#[test]
	fn phrases_split_at_comma() {
		// 'ey , b 'iy .
		let u = Utterance::build(&[CW, 24, COMMA, CW, 39, 26, PERIOD], &["a", "b"]);
		assert_eq!(u.phrases.len(), 2);
		assert_eq!(u.phrases[0].end, Boundary::Comma);
		assert_eq!(u.phrases[1].end, Boundary::Period);
		assert_eq!(u.words[1].phrase, 1);
		assert!(u.is_last_syl_of_phrase(0));
		assert_eq!(u.phrase_of_seg(2), 1);
	}

	#[test]
	fn a_trailing_comma_ends_the_sentence() {
		// 'ey ,  at the end of the input
		let u = Utterance::build(&[CW, 24, COMMA], &["a"]);
		assert_eq!(u.phrases.len(), 1);
		assert_eq!(u.phrases[0].end, Boundary::Period);
		// a comma inside the sentence stays
		let u = Utterance::build(&[CW, 24, COMMA, CW, 39, 26, PERIOD], &["a", "b"]);
		assert_eq!(u.phrases[0].end, Boundary::Comma);
	}

	#[test]
	fn kind_detection() {
		let w = ["go"];
		let go = [CW, 48, 27];		// g 'ow
		let with = |p: u8| { let mut t = go.to_vec(); t.push(p); t };
		assert_eq!(Utterance::build(&with(PERIOD), &w).kind, Kind::Statement);
		assert_eq!(Utterance::build(&with(BANG), &w).kind, Kind::Exclamation);
		assert_eq!(Utterance::build(&with(QMARK), &w).kind, Kind::YesNo);
		assert_eq!(Utterance::build(&with(QMARK), &["Where"]).kind, Kind::Wh);
	}

	#[test]
	fn no_terminal_punctuation() {
		let u = Utterance::build(&[CW, 51, 15], &["ha"]);
		assert_eq!(u.phrases.len(), 1);
		assert_eq!(u.phrases[0].end, Boundary::Unmarked);
		assert_eq!(u.kind, Kind::Statement);
	}

	#[test]
	fn empty_trailing_word_dropped() {
		// a boundary token with no phones after the sentence end adds nothing
		let u = Utterance::build(&[CW, 56, 18, 76, PERIOD, CW], &["cat"]);
		assert_eq!(u.words.len(), 1);
		assert_eq!(u.phrases.len(), 1);
		assert_eq!(u.kind, Kind::Statement);
	}

	#[test]
	fn function_flag() {
		let u = Utterance::build(&[FW, 43, 36, CW, 56, 18, 76, PERIOD], &["the", "cat"]);
		assert!(u.words[0].function);
		assert!(!u.words[1].function);
		assert_eq!(u.words[1].text, "cat");
	}

	fn phones(u: &Utterance) -> Vec<u8> { u.segs.iter().map(|s| s.phone).collect() }

	// one-syllable words (function?, text), each b 'aa, then a period
	fn sentence(words: &[(bool, &str)]) -> Utterance {
		let mut toks = Vec::new();
		for &(f, _) in words { toks.extend([if f { FW } else { CW }, 39, 15]); }
		toks.push(PERIOD);
		Utterance::build(&toks, &words.iter().map(|w| w.1).collect::<Vec<_>>())
	}

	#[test]
	fn a_long_phrase_breaks_before_a_conjunction() {
		// 16 syllables: the break goes before "and", not before "the" at the middle or the preposition "of"
		let (c, f) = ((false, "w"), |t| (true, t));
		let u = sentence(&[c, c, f("the"), c, c, c, f("and"), c, f("the"), c, c, f("of"), c, c, c, c]);
		assert_eq!(u.phrases.len(), 2);
		assert_eq!(u.phrases[0].words, 0..6);
		assert_eq!(u.phrases[0].end, Boundary::Minor);
		assert_eq!(u.phrases[1].end, Boundary::Period);
		assert_eq!(u.words[6].phrase, 1);
		assert!(u.is_last_syl_of_phrase(5));
	}

	#[test]
	fn a_long_phrase_keeps_a_verb_with_its_object_and_to_and_of_with_their_words() {
		// "... using | the final version | of the text | to speech ...": no break before an article, "of" or "to"
		let (c, f) = ((false, "w"), |t| (true, t));
		let u = sentence(&[c, c, c, c, c, f("the"), c, c, c, f("of"), f("the"), c, f("to"), c, c, c, c]);
		assert_eq!(u.phrases.len(), 1);
	}

	#[test]
	fn a_phrase_breaks_only_where_it_can() {
		let (c, f) = ((false, "w"), |t| (true, t));
		// short enough
		let mut short = vec![c; 13];
		short.insert(6, f("and"));
		assert_eq!(sentence(&short).phrases.len(), 1);
		// no word between content words to break before
		assert_eq!(sentence(&[c; 16]).phrases.len(), 1);
		// the only place leaves a piece too short
		let mut edge = vec![c; 15];
		edge.insert(2, f("and"));
		assert_eq!(sentence(&edge).phrases.len(), 1);
		// a very long phrase breaks more than once, each piece short enough
		let mut long = Vec::new();
		for _ in 0..6 { long.extend([c, c, c, c, c, f("and")]); }
		long.pop();
		let u = sentence(&long);
		assert!(u.phrases.len() >= 3, "{} phrases", u.phrases.len());
		for ph in &u.phrases { assert!(ph.words.len() <= MAX_PHRASE_SYL, "{:?}", ph.words); }
	}

	#[test]
	fn flaps_between_a_vowel_and_an_unstressed_vowel() {
		// s 'ih t iy: the t flaps
		let u = Utterance::build(&[CW, 72, 25, 76, 54, PERIOD], &["city"]);
		assert_eq!(phones(&u), vec![34, 19, 47, 21]);
	}

	#[test]
	fn keeps_the_stop_before_a_stressed_vowel() {
		// ax t 'ae k
		let u = Utterance::build(&[CW, 36, 76, 16, 56, PERIOD], &["attack"]);
		assert_eq!(phones(&u)[1], 36);
	}

	#[test]
	fn keeps_a_words_first_stop() {
		// ax | t 'aa p
		let u = Utterance::build(&[CW, 36, CW, 76, 15, 68, PERIOD], &["a", "top"]);
		assert_eq!(phones(&u)[1], 36);
	}

	#[test]
	fn flaps_a_word_final_stop_before_a_vowel() {
		// g 'aa t | 'ih t: the first t flaps, the phrase-final one does not
		let u = Utterance::build(&[CW, 48, 15, 76, CW, 25, 76, PERIOD], &["got", "it"]);
		assert_eq!(phones(&u), vec![17, 1, 47, 19, 36]);
	}

	#[test]
	fn keeps_the_stop_before_a_syllabic_n() {
		// b 'ah t ax n
		let u = Utterance::build(&[CW, 39, 17, 76, 36, 64, PERIOD], &["button"]);
		assert_eq!(phones(&u)[2], 36);
	}

	#[test]
	fn no_flap_across_a_phrase_break() {
		// g 'aa t , 'ih t
		let u = Utterance::build(&[CW, 48, 15, 76, COMMA, CW, 25, 76, PERIOD], &["got", "it"]);
		assert_eq!(phones(&u)[2], 36);
	}
}
