/*

DURATION RULES

each segment's duration is

	dur = min + (inherent - min) * product_of_rule_factors / rate

so only the stretchable part above the minimum is scaled.

*/
use crate::_utt::{Utterance, Boundary, is_vowel, is_voiced, is_obstruent, offglide};

pub struct PhoneDur { pub inherent: f32, pub min: f32 }

pub const START_PAUSE_MS: f32 = 50.0;		// silence before the first phone of the input
pub const RELEASE_MS:     f32 = 80.0;		// phrase-end release phone (AXP/IXP)
const COMMA_PAUSE_MS:     f32 = 220.0;
const SENTENCE_PAUSE_MS:  f32 = 450.0;
const MINOR_PAUSE_MS:     f32 = 100.0;		// a minor break inside a long phrase

// rule factors, tuned by ear
const PHRASE_FINAL_VOWEL:     f32 = 1.4;
const PHRASE_FINAL_CODA:      f32 = 1.3;
const SENTENCE_FINAL:         f32 = 1.1;
const UNSTRESSED_VOWEL:       f32 = 0.7;
const SECONDARY_VOWEL:        f32 = 0.85;
const FUNCTION_WORD:          f32 = 0.85;
const POLYSYLLABIC:           f32 = 0.9;
const POLYSYLLABIC_FLOOR:     f32 = 0.7;
const BEFORE_VOICED_OBS:      f32 = 1.15;
const BEFORE_VOICELESS_OBS:   f32 = 0.9;
const CLUSTER_CONSONANT:      f32 = 0.8;
const WORD_INITIAL_CONSONANT: f32 = 1.1;
const TENSE_VOWEL:            f32 = 1.4;		// long (tense) vowels and diphthongs: aa ae ao aw ay er ey iy ow oy uw
const GLIDE_THROUGH:          f32 = 0.3;		// a w or y a vowel glides through into the next one ("follower")

pub fn phone_dur(p: u8) -> PhoneDur {
	match p {
		// phone => inherent, min (ms); tuned by ear
		1  => PhoneDur { inherent: 129.0, min:  58.0 },	// aa
		2  => PhoneDur { inherent: 110.0, min:  50.0 },	// ae
		3  => PhoneDur { inherent: 114.0, min:  51.0 },	// ah
		4  => PhoneDur { inherent: 120.0, min:  54.0 },	// ao
		5  => PhoneDur { inherent: 124.0, min:  56.0 },	// aw
		6  => PhoneDur { inherent:  90.0, min:  40.0 },	// ax
		7  => PhoneDur { inherent: 123.0, min:  55.0 },	// axr
		8  => PhoneDur { inherent: 152.0, min:  68.0 },	// ay
		9  => PhoneDur { inherent:  78.0, min:  45.0 },	// b
		10 => PhoneDur { inherent:  95.0, min:  43.0 },	// ch
		11 => PhoneDur { inherent:  75.0, min:  53.0 },	// d
		12 => PhoneDur { inherent:  66.0, min:  30.0 },	// dh
		13 => PhoneDur { inherent: 108.0, min:  49.0 },	// eh
		14 => PhoneDur { inherent: 121.0, min:  54.0 },	// er
		15 => PhoneDur { inherent: 119.0, min:  54.0 },	// ey
		16 => PhoneDur { inherent:  97.0, min:  44.0 },	// f
		17 => PhoneDur { inherent:  78.0, min:  70.0 },	// g
		18 => PhoneDur { inherent:  81.0, min:  36.0 },	// hh
		19 => PhoneDur { inherent: 105.0, min:  47.0 },	// ih
		20 => PhoneDur { inherent:  89.0, min:  40.0 },	// ix
		21 => PhoneDur { inherent: 112.0, min:  50.0 },	// iy
		22 => PhoneDur { inherent:  85.0, min:  38.0 },	// jh
		23 => PhoneDur { inherent: 103.0, min: 103.0 },	// k
		24 => PhoneDur { inherent:  94.0, min:  42.0 },	// l
		25 => PhoneDur { inherent: 104.0, min:  47.0 },	// lx
		26 => PhoneDur { inherent:  77.0, min:  35.0 },	// m
		27 => PhoneDur { inherent:  83.0, min:  37.0 },	// n
		28 => PhoneDur { inherent:  89.0, min:  40.0 },	// ng
		29 => PhoneDur { inherent: 119.0, min:  54.0 },	// ow
		30 => PhoneDur { inherent: 156.0, min:  70.0 },	// oy
		31 => PhoneDur { inherent:  88.0, min:  88.0 },	// p
		32 => PhoneDur { inherent:  86.0, min:  39.0 },	// r
		33 => PhoneDur { inherent:  99.0, min:  45.0 },	// rx
		34 => PhoneDur { inherent:  94.0, min:  42.0 },	// s
		35 => PhoneDur { inherent:  91.0, min:  41.0 },	// sh
		36 => PhoneDur { inherent: 101.0, min: 101.0 },	// t
		37 => PhoneDur { inherent:  90.0, min:  40.0 },	// th
		38 => PhoneDur { inherent: 110.0, min:  50.0 },	// uh
		39 => PhoneDur { inherent: 118.0, min:  53.0 },	// uw
		40 => PhoneDur { inherent:  81.0, min:  36.0 },	// v
		41 => PhoneDur { inherent:  87.0, min:  39.0 },	// w
		42 => PhoneDur { inherent:  85.0, min:  38.0 },	// y
		43 => PhoneDur { inherent:  88.0, min:  40.0 },	// z
		44 => PhoneDur { inherent:  94.0, min:  42.0 },	// zh
		47 => PhoneDur { inherent:  30.0, min:  20.0 },	// dx: a single quick tap
		_  => PhoneDur { inherent:  80.0, min:  40.0 },
	}
}

pub fn pause_after(end: Boundary, rate: f32) -> f32 {
	let ms = match end { Boundary::Comma => COMMA_PAUSE_MS, Boundary::Minor => MINOR_PAUSE_MS, _ => SENTENCE_PAUSE_MS };
	ms / rate
}

pub fn assign(utt: &mut Utterance, rate: f32) {
	for i in 0..utt.segs.len() {
		let p = utt.segs[i].phone;
		let vowel = is_vowel(p);
		let w = utt.segs[i].word;
		let syl = utt.segs[i].syl;
		let word = &utt.words[w];
		let same_word = |j: usize| j < utt.segs.len() && utt.segs[j].word == w;
		let mut f: f32 = 1.0;
		let mut rules: Vec<&'static str> = Vec::new();
		let mut apply = |name: &'static str, x: f32| { f *= x; rules.push(name); };

		// phrase-final lengthening: the nucleus and coda of the last syllable of a phrase
		if utt.is_last_syl_of_phrase(syl) {
			let nucleus = utt.syls[syl].nucleus;
			let is_nucleus = nucleus == Some(i);
			let is_coda = !vowel && nucleus.is_some_and(|n| i > n);
			if is_nucleus || is_coda {
				if is_nucleus { apply("phrase-final", PHRASE_FINAL_VOWEL); } else { apply("phrase-final", PHRASE_FINAL_CODA); }
				if !matches!(utt.phrases[word.phrase].end, Boundary::Comma | Boundary::Minor) { apply("sentence-final", SENTENCE_FINAL); }
			}
		}

		if vowel {
			match utt.segs[i].stress {
				0 => apply("unstressed", UNSTRESSED_VOWEL),
				2 => apply("secondary", SECONDARY_VOWEL),
				_ => {},
			}
			if matches!(p, 1 | 2 | 4 | 5 | 8 | 14 | 15 | 21 | 29 | 30 | 39) { apply("tense", TENSE_VOWEL); }
			let n = word.syls.len();
			if n >= 2 { apply("polysyllabic", libm::powf(POLYSYLLABIC, (n - 1) as f32).max(POLYSYLLABIC_FLOOR)); }
			if same_word(i + 1) && is_obstruent(utt.segs[i + 1].phone) {
				if is_voiced(utt.segs[i + 1].phone) { apply("before-voiced", BEFORE_VOICED_OBS); } else { apply("before-voiceless", BEFORE_VOICELESS_OBS); }
			}
		} else {
			let cons = |j: usize| same_word(j) && !is_vowel(utt.segs[j].phone);
			if cons(i + 1) || (i > 0 && cons(i - 1)) { apply("cluster", CLUSTER_CONSONANT); }
			if i == word.segs.start { apply("word-initial", WORD_INITIAL_CONSONANT); }
			if i > 0 && same_word(i - 1) && offglide(utt.segs[i - 1].phone) == Some(p) && same_word(i + 1) && is_vowel(utt.segs[i + 1].phone) {
				apply("glide-through", GLIDE_THROUGH);
			}
		}

		if word.function { apply("function-word", FUNCTION_WORD); }

		let pd = phone_dur(p);
		utt.segs[i].dur_ms = pd.min + (pd.inherent - pd.min) * f / rate;
		utt.segs[i].rules = rules;
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::_utt::Utterance;

	const CW: u8 = 90;
	const COMMA: u8 = 91;
	const PERIOD: u8 = 92;

	fn build(tokens: &[u8]) -> Utterance {
		let n = tokens.iter().filter(|&&t| t == 90 || t == 95).count();
		Utterance::build(tokens, &vec![String::from("w"); n])
	}
	fn stretch(u: &Utterance, i: usize) -> f32 {
		let pd = phone_dur(u.segs[i].phone);
		u.segs[i].dur_ms - pd.min
	}

	#[test]
	fn rate_shortens_but_respects_min() {
		// k 'ae t  s 'ae t .
		let toks = [CW, 56, 16, 76, CW, 72, 16, 76, PERIOD];
		let mut a = build(&toks);
		let mut b = build(&toks);
		assign(&mut a, 1.0);
		assign(&mut b, 1.5);
		// the stretchable part above min shrinks by the rate, the min stays
		for i in 0..a.segs.len() {
			let pd = phone_dur(a.segs[i].phone);
			assert!(b.segs[i].dur_ms >= pd.min);
			assert!((stretch(&b, i) * 1.5 - stretch(&a, i)).abs() < 0.01, "{}: {} ms at rate 1.5, {} at 1", i, b.segs[i].dur_ms, a.segs[i].dur_ms);
		}
		// k and t are all min: the rate leaves them alone
		assert_eq!(b.segs[0].dur_ms, a.segs[0].dur_ms);
		assert!(b.segs[1].dur_ms < a.segs[1].dur_ms);
		// pauses shrink by the rate too
		assert_eq!(pause_after(Boundary::Comma, 2.0), COMMA_PAUSE_MS / 2.0);
	}

	#[test]
	fn phrase_final_vowel_longer() {
		// k 'ae t  k 'ae t .
		let mut u = build(&[CW, 56, 16, 76, CW, 56, 16, 76, PERIOD]);
		assign(&mut u, 1.0);
		let ratio = stretch(&u, 4) / stretch(&u, 1);
		assert!((ratio - 1.4 * 1.1).abs() < 0.01, "ratio {}", ratio);
	}

	#[test]
	fn minor_break_lengthens_without_sentence_final() {
		// k 'ae t , k 'ae t . with the comma taken as a minor break
		let mut u = build(&[CW, 56, 16, 76, COMMA, CW, 56, 16, 76, PERIOD]);
		u.phrases[0].end = Boundary::Minor;
		assign(&mut u, 1.0);
		assert!(u.segs[1].rules.contains(&"phrase-final"));
		assert!(!u.segs[1].rules.contains(&"sentence-final"));
		assert_eq!(pause_after(Boundary::Minor, 1.0), MINOR_PAUSE_MS);
	}

	#[test]
	fn unstressed_shorter_than_stressed() {
		// p 'aa p  p aa p  b 'iy .
		let mut u = build(&[CW, 68, 15, 68, CW, 68, 32, 68, CW, 39, 26, PERIOD]);
		assign(&mut u, 1.0);
		let ratio = stretch(&u, 4) / stretch(&u, 1);
		assert!((ratio - 0.7).abs() < 0.01, "ratio {}", ratio);
	}

	#[test]
	fn tense_vowel_longer_than_lax() {
		// b 'iy t  b 'ih t  b 'iy .
		let mut u = build(&[CW, 39, 26, 76, CW, 39, 25, 76, CW, 39, 26, PERIOD]);
		assign(&mut u, 1.0);
		// each against its own stretchable range, so only the rule factors are compared
		let k = |i: usize| { let pd = phone_dur(u.segs[i].phone); stretch(&u, i) / (pd.inherent - pd.min) };
		let ratio = k(1) / k(4);
		assert!((ratio - TENSE_VOWEL).abs() < 0.01, "ratio {}", ratio);
	}

	#[test]
	fn before_voiced_longer() {
		// b 'ah z  b 'ah s  b 'iy .
		let mut u = build(&[CW, 39, 17, 87, CW, 39, 17, 72, CW, 39, 26, PERIOD]);
		assign(&mut u, 1.0);
		let ratio = stretch(&u, 1) / stretch(&u, 4);
		assert!((ratio - 1.15 / 0.9).abs() < 0.01, "ratio {}", ratio);
	}

	#[test]
	fn pause_values() {
		assert_eq!(pause_after(Boundary::Comma, 1.0), 220.0);
		assert_eq!(pause_after(Boundary::Period, 2.0), 225.0);
	}
}
