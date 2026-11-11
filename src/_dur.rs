/*

DURATION RULES

each segment's duration is

	dur = min + (inherent - min) * product_of_rule_factors / (rate * TEMPO)

so only the stretchable part above the minimum is scaled.

*/
use crate::_utt::{Utterance, Boundary, is_vowel, is_voiced, is_obstruent};
use crate::_phone::*;
use alloc::vec::Vec;

pub struct PhoneDur { pub inherent: f32, pub min: f32 }

const TEMPO: f32 = 1.12;					// the pace at rate 1
pub const START_PAUSE_MS: f32 = 50.0;		// silence before the first phone of the input
pub const RELEASE_MS:     f32 = 80.0;		// phrase-end release phone (AXP/IXP)
const COMMA_PAUSE_MS:     f32 = 160.0;
const SENTENCE_PAUSE_MS:  f32 = 450.0;
const MINOR_PAUSE_MS:     f32 = 100.0;		// a minor break inside a long phrase

// rule factors, tuned by ear
const PHRASE_FINAL_VOWEL:     f32 = 1.4;
const PHRASE_FINAL_CODA:      f32 = 1.3;
const SENTENCE_FINAL:         f32 = 1.7;
const UNSTRESSED_VOWEL:       f32 = 0.7;
const SECONDARY_VOWEL:        f32 = 0.85;
const FUNCTION_WORD:          f32 = 0.75;
const POLYSYLLABIC:           f32 = 0.9;
const POLYSYLLABIC_FLOOR:     f32 = 0.7;
const BEFORE_VOICED_OBS:      f32 = 1.15;
const BEFORE_VOICELESS_OBS:   f32 = 0.9;
const CLUSTER_CONSONANT:      f32 = 0.8;
const WORD_INITIAL_CONSONANT: f32 = 1.1;
const TENSE_VOWEL:            f32 = 1.4;		// long (tense) vowels and diphthongs: aa ae ao aw ay er ey iy ow oy uw
const SYLLABIC_L_SCHWA:       f32 = 0.5;		// the schwa of a syllabic (dark) l ("example") is mostly the l
const GLIDE_THROUGH:          f32 = 0.3;		// a w or y a vowel glides through into the next one ("follower")
const BEFORE_DENTAL:          f32 = 0.5;		// an s, z, f or v runs into a th or dh after it ("sixths", "twelfth")

pub fn phone_dur(p: u8) -> PhoneDur {
	match p {
		// phone => inherent, min (ms); tuned by ear
		AA  => PhoneDur { inherent: 129.0, min:  58.0 },
		AE  => PhoneDur { inherent: 110.0, min:  50.0 },
		AH  => PhoneDur { inherent: 114.0, min:  51.0 },
		AO  => PhoneDur { inherent: 120.0, min:  54.0 },
		AW  => PhoneDur { inherent: 149.0, min:  67.0 },
		AX  => PhoneDur { inherent:  90.0, min:  40.0 },
		AXR => PhoneDur { inherent: 123.0, min:  55.0 },
		AY  => PhoneDur { inherent: 117.0, min:  52.0 },
		B   => PhoneDur { inherent:  78.0, min:  45.0 },
		CH  => PhoneDur { inherent:  95.0, min:  43.0 },
		D   => PhoneDur { inherent:  75.0, min:  34.0 },
		DH  => PhoneDur { inherent:  66.0, min:  30.0 },
		EH  => PhoneDur { inherent: 108.0, min:  49.0 },
		ER  => PhoneDur { inherent: 121.0, min:  54.0 },
		EY  => PhoneDur { inherent: 119.0, min:  54.0 },
		F   => PhoneDur { inherent:  97.0, min:  44.0 },
		G   => PhoneDur { inherent:  78.0, min:  35.0 },
		HH  => PhoneDur { inherent:  81.0, min:  36.0 },
		IH  => PhoneDur { inherent: 105.0, min:  47.0 },
		IX  => PhoneDur { inherent:  89.0, min:  40.0 },
		IY  => PhoneDur { inherent: 112.0, min:  50.0 },
		JH  => PhoneDur { inherent:  85.0, min:  38.0 },
		K   => PhoneDur { inherent: 103.0, min:  46.0 },
		L   => PhoneDur { inherent:  94.0, min:  42.0 },
		LX  => PhoneDur { inherent: 104.0, min:  47.0 },
		M   => PhoneDur { inherent:  77.0, min:  35.0 },
		N   => PhoneDur { inherent:  83.0, min:  37.0 },
		NG  => PhoneDur { inherent:  89.0, min:  40.0 },
		OW  => PhoneDur { inherent: 119.0, min:  54.0 },
		OY  => PhoneDur { inherent: 156.0, min:  70.0 },
		P   => PhoneDur { inherent:  88.0, min:  40.0 },
		R   => PhoneDur { inherent:  86.0, min:  39.0 },
		RX  => PhoneDur { inherent:  99.0, min:  45.0 },
		S   => PhoneDur { inherent:  94.0, min:  42.0 },
		SH  => PhoneDur { inherent:  91.0, min:  41.0 },
		T   => PhoneDur { inherent: 101.0, min:  45.0 },
		TH  => PhoneDur { inherent:  90.0, min:  40.0 },
		UH  => PhoneDur { inherent: 110.0, min:  50.0 },
		UW  => PhoneDur { inherent: 118.0, min:  53.0 },
		V   => PhoneDur { inherent:  81.0, min:  36.0 },
		W   => PhoneDur { inherent:  87.0, min:  39.0 },
		Y   => PhoneDur { inherent:  85.0, min:  38.0 },
		Z   => PhoneDur { inherent:  88.0, min:  40.0 },
		ZH  => PhoneDur { inherent:  94.0, min:  42.0 },
		DX  => PhoneDur { inherent:  45.0, min:  30.0 },	// a single tap, long enough for its dip to be heard
		_   => PhoneDur { inherent:  80.0, min:  40.0 },
	}
}

pub fn pause_after(end: Boundary, rate: f32) -> f32 {
	let ms = match end {
		Boundary::Comma => COMMA_PAUSE_MS,
		Boundary::Minor => MINOR_PAUSE_MS,
		_ => SENTENCE_PAUSE_MS
	};
	ms / tempo(rate)
}

// the speaking rate times TEMPO, the rate held to 0.5..2 (a NaN is the normal rate)
fn tempo(rate: f32) -> f32 { if rate.is_nan() { TEMPO } else { rate.clamp(0.5, 2.0) * TEMPO } }

pub fn assign(utt: &mut Utterance, rate: f32) {
	for i in 0..utt.segs.len() {
		let p = utt.segs[i].phone;
		let vowel = is_vowel(p);
		let w = utt.segs[i].word;
		let syl = utt.segs[i].syl;
		let word = &utt.words[w];
		let same_word = |j: usize| j < utt.segs.len() && utt.segs[j].word == w;
		let mut factor: f32 = 1.0;
		let mut rules: Vec<&'static str> = Vec::new();
		let mut apply = |name: &'static str, x: f32| { factor *= x; rules.push(name); };

		// phrase-final lengthening: the nucleus and coda of the last syllable of a phrase
		if utt.is_last_syl_of_phrase(syl) {
			let nucleus = utt.syls[syl].nucleus;
			let is_nucleus = nucleus == Some(i);
			let is_coda = !vowel && nucleus.is_some_and(|n| i > n);
			if is_nucleus || is_coda {
				apply("phrase-final", if is_nucleus { PHRASE_FINAL_VOWEL } else { PHRASE_FINAL_CODA });
				if !matches!(utt.phrases[word.phrase].end, Boundary::Comma | Boundary::Minor) {
					apply("sentence-final", SENTENCE_FINAL);
				}
			}
		}

		if vowel {
			match utt.segs[i].stress {
				0 => apply("unstressed", UNSTRESSED_VOWEL),
				2 => apply("secondary", SECONDARY_VOWEL),
				_ => {},
			}
			if matches!(p, AA | AE | AO | AW | AY | ER | EY | IY | OW | OY | UW) { apply("tense", TENSE_VOWEL); }
			let n = word.syls.len();
			if n >= 2 { apply("polysyllabic", libm::powf(POLYSYLLABIC, (n - 1) as f32).max(POLYSYLLABIC_FLOOR)); }
			if same_word(i + 1) && utt.segs[i + 1].phone == LX && matches!(p, AX | IX) {
				apply("syllabic-l", SYLLABIC_L_SCHWA);
			}
			if same_word(i + 1) && is_obstruent(utt.segs[i + 1].phone) {
				if is_voiced(utt.segs[i + 1].phone) {
					apply("before-voiced", BEFORE_VOICED_OBS);
				} else {
					apply("before-voiceless", BEFORE_VOICELESS_OBS);
				}
			}
		} else {
			let cons = |j: usize| same_word(j) && !is_vowel(utt.segs[j].phone);
			if cons(i + 1) || (i > 0 && cons(i - 1)) { apply("cluster", CLUSTER_CONSONANT); }
			if i == word.segs.start { apply("word-initial", WORD_INITIAL_CONSONANT); }
			if matches!(p, S | Z | F | V) && same_word(i + 1) && matches!(utt.segs[i + 1].phone, TH | DH) {
				apply("before-dental", BEFORE_DENTAL);
			}
			if utt.segs[i].glide { apply("glide-through", GLIDE_THROUGH); }
		}

		if word.function { apply("function-word", FUNCTION_WORD); }

		let pd = phone_dur(p);
		utt.segs[i].dur_ms = pd.min + (pd.inherent - pd.min) * factor / tempo(rate);
		utt.segs[i].rules = rules;
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	// boundary tokens, as the front end writes them
	use crate::_phone::{TOKEN_CW as CW, TOKEN_FW as FW, TOKEN_COMMA as COMMA, TOKEN_PERIOD as PERIOD};

	fn build(tokens: &[u8]) -> Utterance {
		let n = tokens.iter().filter(|&&t| t == CW || t == FW).count();
		Utterance::build(tokens, &vec!["w"; n])
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
			assert!((stretch(&b, i) * 1.5 - stretch(&a, i)).abs() < 0.01,
				"{}: {} ms at rate 1.5, {} at 1", i, b.segs[i].dur_ms, a.segs[i].dur_ms);
		}
		// stops shrink with the rate like everything else
		assert!(b.segs[0].dur_ms < a.segs[0].dur_ms);
		assert!(b.segs[1].dur_ms < a.segs[1].dur_ms);
		// pauses shrink by the rate too
		assert_eq!(pause_after(Boundary::Comma, 2.0), COMMA_PAUSE_MS / (2.0 * TEMPO));
	}

	#[test]
	fn s_and_f_before_th_are_short() {
		// s 'ih k s th s .  against  s 'ih k s t s ., then the same with f for the s
		for c in [72, 46] {
			let mut th = build(&[CW, 72, 25, 56, c, 77, 72, PERIOD]);
			let mut t = build(&[CW, 72, 25, 56, c, 76, 72, PERIOD]);
			assign(&mut th, 1.0);
			assign(&mut t, 1.0);
			let ratio = stretch(&th, 3) / stretch(&t, 3);
			assert!((ratio - BEFORE_DENTAL).abs() < 0.01, "{}: ratio {}", c, ratio);
		}
	}

	#[test]
	fn phrase_final_vowel_longer() {
		// k 'ae t  k 'ae t .
		let mut u = build(&[CW, 56, 16, 76, CW, 56, 16, 76, PERIOD]);
		assign(&mut u, 1.0);
		let ratio = stretch(&u, 4) / stretch(&u, 1);
		assert!((ratio - PHRASE_FINAL_VOWEL * SENTENCE_FINAL).abs() < 0.01, "ratio {}", ratio);
	}

	#[test]
	fn minor_break_lengthens_without_sentence_final() {
		// k 'ae t , k 'ae t . with the comma taken as a minor break
		let mut u = build(&[CW, 56, 16, 76, COMMA, CW, 56, 16, 76, PERIOD]);
		u.phrases[0].end = Boundary::Minor;
		assign(&mut u, 1.0);
		assert!(u.segs[1].rules.contains(&"phrase-final"));
		assert!(!u.segs[1].rules.contains(&"sentence-final"));
		assert_eq!(pause_after(Boundary::Minor, 1.0), MINOR_PAUSE_MS / TEMPO);
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
		assert_eq!(pause_after(Boundary::Comma, 1.0), 160.0 / TEMPO);
		assert_eq!(pause_after(Boundary::Period, 2.0), 450.0 / (2.0 * TEMPO));
	}
}
