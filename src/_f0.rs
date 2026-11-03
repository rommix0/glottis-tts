/*

PITCH CONTOUR

pitch targets placed on segments (a stream index and a position inside
the segment), in semitones from REF_HZ:

	+ a baseline per phrase that falls over the phrase, each later phrase
	  starting a little lower
	+ an accent peak on the stressed vowel of every content word, stepping
	  down within the phrase and restarting at each phrase; a content word
	  right after an accented one goes without, unless it ends the phrase;
	  the last accent peaks early in its vowel
	+ a trough between accents that are far apart: a sag when they are
	  fairly close, down to the baseline when further apart
	+ phrase ends: a fall for statements, a rise for yes/no questions,
	  a small rise at commas and a smaller one at minor breaks
	+ exclamations: every accent and the final fall widened

and short offsets on top of the targets:

	+ microprosody: a sonorant or vowel after an obstruent starts raised
	  after a voiceless one and lowered after a voiced one, and decays;
	  high vowels rise a little over their middle
	+ glottal dips: a fall into a word-final t before a consonant, and a
	  low start to a stressed vowel that begins a content word, reached
	  just after the vowel's start when a sonorant comes before it

the contour is evaluated per sample from the synthesizer's position.

*/
use crate::_utt::{Utterance, Kind, Boundary, PAU, is_vowel, is_voiced, is_obstruent, is_sonorant};
use core::f32::consts::PI;
use alloc::vec::Vec;

pub const REF_HZ: f32 = 110.0;
const MIN_HZ: f32 = 70.0;
const MAX_HZ: f32 = 250.0;

// targets, tuned by ear
const BASELINE_FALL_ST:     f32 = 2.0;
const PHRASE_RESET_ST:      f32 = 1.0;
const PHRASE_RESET_MAX_ST:  f32 = 2.0;
const ACCENT_ST:            f32 = 6.0;
const DOWNSTEP:             f32 = 0.8;
const ACCENT_FLOOR_ST:      f32 = 2.0;
const PHRASE_ACCENT_RATIO:  f32 = 0.9;
const ACCENT_FRAC:          f32 = 0.55;
const TROUGH_GAP_MS:        f32 = 250.0;
const FINAL_FALL_ST:        f32 = -4.0;
const EXCLAIM_BOOST_ST:     f32 = 2.0;
const EXCLAIM_RANGE:        f32 = 1.4;		// an exclamation's accents and final fall, widened
const QUESTION_RISE_ST:     f32 = 8.0;
const LOW_ACCENT_ST:        f32 = -1.0;
const LOW_ACCENT_FRAC:      f32 = 0.3;
const NUCLEAR_FRAC:         f32 = 0.2;		// the last accent peaks early, so the final fall runs through its vowel
const CONTINUATION_RISE_ST: f32 = 3.0;
const MINOR_RISE_ST:        f32 = 1.5;		// at a minor break inside a long phrase
const VOICELESS_BUMP_ST:    f32 = 0.6;		// microprosody after a voiceless obstruent, decaying over ..._MS
const VOICELESS_BUMP_MS:    f32 = 50.0;
const VOICED_BUMP_ST:       f32 = -0.6;		// ... and lowered after a voiced one
const VOICED_BUMP_MS:       f32 = 30.0;
const BUMP_SPAN_MS:         f32 = 200.0;	// a bump is drawn over this long at most
const T_DIP_ST:             f32 = 1.5;		// fall into a pre-glottalized word-final t, over T_DIP_MS before it
const T_DIP_MS:             f32 = 40.0;
const ONSET_DIP_ST:         f32 = 1.5;		// glottal onset: a stressed word-initial vowel starts this low, back by ONSET_DIP_MS
const ONSET_DIP_MS:         f32 = 60.0;
const ONSET_LEAD_MS:        f32 = 40.0;		// after a voiced sound the vowel's slot opens on that sound's tail: fall over this first
const TROUGH_FULL_MS:       f32 = 500.0;	// accents this far apart dip to the baseline between them, closer ones sag less
const MERGE_MS:             f32 = 20.0;
const HIGH_VOWEL_ST:        f32 = 0.7;		// high vowels rise this much over their middle

#[derive(Debug, PartialEq)]
pub enum TargetKind { Start, Accent, Low, Trough, End }

#[derive(Debug)]
pub struct Target {
	pub seg:  usize,		// stream index of the segment
	pub frac: f32,			// position inside the segment (0..1)
	pub st:   f32,			// semitones from REF_HZ
	pub base: f32,			// the baseline here (for tracing and tests)
	pub kind: TargetKind,	// an End target holds its value until the next one
}

// a short pitch offset, drawn on each segment it covers; age is the ms since the offset began
struct Bump { seg: usize, age0: f32, dur: f32, shape: Shape }

#[derive(Clone, Copy)]
enum Shape {
	Decay { st: f32, ms: f32 },		// st at age 0, decaying with time constant ms
	Fall  { st: f32, ms: f32 },		// 0 at age 0, falling to -st at ms, then held
	Rise  { st: f32, ms: f32 },		// -st at age 0, back to 0 at ms
	Vee   { st: f32, down: f32, up: f32 },	// 0 at age 0, -st at down, back to 0 up later
	Hump  { st: f32, ms: f32 },		// 0 at age 0, st halfway through ms, 0 again at ms
}

impl Shape {
	fn at(self, age: f32) -> f32 {
		if age < 0.0 { return 0.0; }
		match self {
			Shape::Decay { st, ms } => if age < BUMP_SPAN_MS { st * libm::expf(-age / ms) } else { 0.0 },
			Shape::Fall  { st, ms } => -st * (age / ms).min(1.0),
			Shape::Rise  { st, ms } => -st * (1.0 - age / ms).max(0.0),
			Shape::Vee   { st, down, up } => if age < down { -st * age / down } else { -st * (1.0 - (age - down) / up).max(0.0) },
			Shape::Hump  { st, ms } => if age < ms { st * libm::sinf(PI * age / ms) } else { 0.0 },
		}
	}
}

pub struct Contour {
	targets: Vec<Target>,
	bumps:   Vec<Bump>,
	target_at: usize,		// where the last hz_at found itself in targets
	bump_at:   usize,		// and in bumps
}

// the first item of v that before() says is not before the place sought, as partition_point gives; samples come in
// order, so the answer for the last one usually still holds and is checked before searching
fn find<T>(v: &[T], last: &mut usize, before: impl Fn(&T) -> bool) -> usize {
	let k = (*last).min(v.len());
	let k = if (k == 0 || before(&v[k - 1])) && (k == v.len() || !before(&v[k])) { k } else { v.partition_point(before) };
	*last = k;
	k
}

impl Contour {
	pub fn new() -> Contour { Contour { targets: Vec::new(), bumps: Vec::new(), target_at: 0, bump_at: 0 } }

	pub fn targets_of(&self, seg: usize) -> impl Iterator<Item = &Target> {
		self.targets.iter().filter(move |t| t.seg == seg)
	}

	pub fn add_utterance(&mut self, utt: &Utterance, stream: &[usize]) {
		// stream[i] is the stream index of utt.segs[i]
		if utt.segs.is_empty() { return; }

		// start time of each segment (ms) within the utterance
		let mut t0: Vec<f32> = Vec::with_capacity(utt.segs.len() + 1);
		let mut acc = 0.0;
		for s in &utt.segs { t0.push(acc); acc += s.dur_ms; }
		t0.push(acc);
		let time = |i: usize, frac: f32| t0[i] + frac * utt.segs[i].dur_ms;

		// the accented syllable nucleus of each content word
		let accent_of = |w: usize| -> Option<usize> {
			let word = &utt.words[w];
			if word.function { return None; }
			let syls = &utt.syls[word.syls.clone()];
			syls.iter().find(|s| s.stress == 1).or_else(|| syls.iter().find(|s| s.stress == 2)).and_then(|s| s.nucleus)
		};

		let mut new: Vec<(usize, f32, Target)> = Vec::new();		// (phrase, time, target)
		let push = |new: &mut Vec<(usize, f32, Target)>, i: usize, frac: f32, st: f32, base: f32, kind: TargetKind| {
			new.push((utt.phrase_of_seg(i), time(i, frac), Target { seg: stream[i], frac, st, base, kind }));
		};

		let last_phrase = utt.phrases.len() - 1;
		let mut first_height = ACCENT_ST;
		let last_accent_seg = (0..utt.words.len()).rev().find_map(accent_of);
		for (k, ph) in utt.phrases.iter().enumerate() {
			let segs = utt.words[ph.words.start].segs.start..utt.words[ph.words.end - 1].segs.end;
			let voiced: Vec<usize> = segs.clone().filter(|&i| is_voiced(utt.segs[i].phone)).collect();
			let (Some(&first_v), Some(&last_v)) = (voiced.first(), voiced.last()) else { continue; };

			let b0 = (-PHRASE_RESET_ST * k as f32).max(-PHRASE_RESET_MAX_ST);
			let (ts, te) = (t0[segs.start], t0[segs.end]);
			let base = |t: f32| b0 - BASELINE_FALL_ST * if te > ts { (t - ts) / (te - ts) } else { 0.0 };

			push(&mut new, first_v, 0.0, base(time(first_v, 0.0)), base(time(first_v, 0.0)), TargetKind::Start);

			// accents, stepping down within the phrase
			if k > 0 { first_height = (first_height * PHRASE_ACCENT_RATIO).max(ACCENT_FLOOR_ST); }
			let mut h = first_height;
			let mut n_acc = 0;
			let mut prev_peak: Option<(usize, f32, f32)> = None;
			let last_content = ph.words.clone().rev().find(|&w| accent_of(w).is_some());
			let mut accented: Option<usize> = None;
			for w in ph.words.clone() {
				let Some(n) = accent_of(w) else { continue; };
				if accented.is_some_and(|a| a + 1 == w) && Some(w) != last_content { continue; }
				accented = Some(w);
				if n_acc > 0 { h = (h * DOWNSTEP).max(ACCENT_FLOOR_ST); }
				n_acc += 1;
				let is_last = Some(n) == last_accent_seg && k == last_phrase;
				let (frac, kind) = match (is_last, utt.kind) {
					(true, Kind::YesNo) => (LOW_ACCENT_FRAC, TargetKind::Low),
					(true, _) => (NUCLEAR_FRAC, TargetKind::Accent),
					_ => (ACCENT_FRAC, TargetKind::Accent),
				};
				let tp = time(n, frac);
				let height = match kind {
					TargetKind::Low => LOW_ACCENT_ST,
					_ if utt.kind != Kind::Exclamation => h,
					_ if is_last => (h + EXCLAIM_BOOST_ST) * EXCLAIM_RANGE,
					_ => h * EXCLAIM_RANGE,
				};
				if let Some((pi, pt, ph)) = prev_peak {
					if tp - pt > TROUGH_GAP_MS {
						// trough on the baseline halfway between the peaks
						let tm = (pt + tp) / 2.0;
						let i = (pi..=n).find(|&i| t0[i + 1] >= tm).unwrap_or(n);
						let fr = if utt.segs[i].dur_ms > 0.0 { ((tm - t0[i]) / utt.segs[i].dur_ms).clamp(0.0, 1.0) } else { 0.0 };
						let k = ((tp - pt - TROUGH_GAP_MS) / (TROUGH_FULL_MS - TROUGH_GAP_MS)).min(1.0);
						let sag = (1.0 - k) * ph.min(height).max(0.0);
						push(&mut new, i, fr, base(tm) + sag, base(tm), TargetKind::Trough);
					}
				}
				push(&mut new, n, frac, base(tp) + height, base(tp), kind);
				prev_peak = Some((n, tp, height));
			}

			// phrase end
			let be = base(time(last_v, 1.0));
			let end_st = match (ph.end, utt.kind) {
				(Boundary::Comma, _)   => CONTINUATION_RISE_ST,
				(Boundary::Minor, _)   => MINOR_RISE_ST,
				(_, Kind::YesNo)       => QUESTION_RISE_ST,
				(_, Kind::Exclamation) => FINAL_FALL_ST * EXCLAIM_RANGE,
				_                      => FINAL_FALL_ST,
			};
			push(&mut new, last_v, 1.0, be + end_st, be, TargetKind::End);
		}

		// targets closer than MERGE_MS within a phrase: keep the later one
		// (phrases are apart by their pause, which utterance time leaves out)
		new.sort_by(|a, b| (a.0, a.1).partial_cmp(&(b.0, b.1)).unwrap());
		let mut kept: Vec<(usize, f32, Target)> = Vec::new();
		for (ph, t, tg) in new {
			if let Some(&(pph, pt, _)) = kept.last() {
				if pph == ph && t - pt < MERGE_MS { kept.pop(); }
			}
			kept.push((ph, t, tg));
		}
		self.targets.extend(kept.into_iter().map(|(_, _, tg)| tg));
		self.targets.sort_by(|a, b| (a.seg, a.frac).partial_cmp(&(b.seg, b.frac)).unwrap());

		// short offsets: microprosody and glottal dips
		const T: u8 = 36;
		const FLAP: u8 = 47;
		let n = utt.segs.len();
		let phone = |i: usize| utt.segs[i].phone;
		let word_start = |i: usize| utt.words[utt.segs[i].word].segs.start == i;
		let word_end = |i: usize| utt.words[utt.segs[i].word].segs.end == i + 1;
		let same_phrase = |i: usize, j: usize| utt.phrase_of_seg(i) == utt.phrase_of_seg(j);
		let mut bumps: Vec<Bump> = Vec::new();
		let draw = |bumps: &mut Vec<Bump>, segs: core::ops::Range<usize>, start: f32, shape: Shape| {
			for j in segs { bumps.push(Bump { seg: stream[j], age0: t0[j] - start, dur: utt.segs[j].dur_ms, shape }); }
		};
		for i in 0..n {
			let after_pause = i == 0 || !same_phrase(i - 1, i);
			let (p, q) = (phone(i), if after_pause { PAU } else { phone(i - 1) });

			// a sonorant or vowel after an obstruent (not a flap): raised, decaying until the next obstruent or the phrase end
			if !after_pause && is_sonorant(p) && is_obstruent(q) && q != FLAP {
				let shape = if is_voiced(q) {
					Shape::Decay { st: VOICED_BUMP_ST, ms: VOICED_BUMP_MS }
				} else {
					Shape::Decay { st: VOICELESS_BUMP_ST, ms: VOICELESS_BUMP_MS }
				};
				let end = (i..n).find(|&j| is_obstruent(phone(j)) || !same_phrase(i, j) || t0[j] - t0[i] >= BUMP_SPAN_MS).unwrap_or(n);
				draw(&mut bumps, i..end, t0[i], shape);
			}

			// a word-final t before a consonant in the same phrase, after a sonorant: F0 falls into it and stays low through it
			if p == T && !after_pause && is_sonorant(q) && word_end(i) && i + 1 < n && same_phrase(i, i + 1) && !is_vowel(phone(i + 1)) {
				let start = t0[i] - T_DIP_MS;
				let first = (0..i).rev().take_while(|&j| t0[j + 1] > start && same_phrase(j, i)).last().unwrap_or(i);
				draw(&mut bumps, first..i + 1, start, Shape::Fall { st: T_DIP_ST, ms: T_DIP_MS });
			}

			// a stressed vowel starting a content word, after a sonorant or a pause: a glottal onset
			let s = &utt.segs[i];
			if is_vowel(p) && s.stress > 0 && word_start(i) && !utt.words[s.word].function && (after_pause || is_sonorant(q)) {
				let (span, shape) = if !after_pause {
					(ONSET_LEAD_MS + ONSET_DIP_MS, Shape::Vee { st: ONSET_DIP_ST, down: ONSET_LEAD_MS, up: ONSET_DIP_MS })
				} else {
					(ONSET_DIP_MS, Shape::Rise { st: ONSET_DIP_ST, ms: ONSET_DIP_MS })
				};
				let end = (i..n).find(|&j| t0[j] - t0[i] >= span || !same_phrase(i, j)).unwrap_or(n);
				draw(&mut bumps, i..end, t0[i], shape);
			}

			// ih iy uh uw: intrinsically a little higher
			if matches!(p, 19 | 21 | 38 | 39) {
				draw(&mut bumps, i..i + 1, t0[i], Shape::Hump { st: HIGH_VOWEL_ST, ms: s.dur_ms });
			}
		}
		bumps.sort_by_key(|b| b.seg);
		self.bumps.extend(bumps);
	}

	// the short offsets at a position, in semitones
	fn bump_at(&mut self, seg: usize, frac: f32) -> f32 {
		let k = find(&self.bumps, &mut self.bump_at, |b| b.seg < seg);
		self.bumps[k..].iter().take_while(|b| b.seg == seg).map(|b| b.shape.at(b.age0 + frac * b.dur)).sum()
	}

	pub fn hz_at(&mut self, seg: usize, frac: f32) -> f32 {
		let pos = seg as f32 + frac;
		// order by segment first, so the end of one segment comes before the start of the next
		let j = find(&self.targets, &mut self.target_at, |t| (t.seg, t.frac) <= (seg, frac));
		let st = match j {
			_ if self.targets.is_empty() => 0.0,
			_ if j == self.targets.len() => self.targets[j - 1].st,
			0 => self.targets[0].st,
			j => {
				let (a, b) = (&self.targets[j - 1], &self.targets[j]);
				if a.kind == TargetKind::End {
					a.st
				} else {
					let (pa, pb) = (a.seg as f32 + a.frac, b.seg as f32 + b.frac);
					let x = if pb > pa { (pos - pa) / (pb - pa) } else { 1.0 };
					a.st + (b.st - a.st) * (1.0 - libm::cosf(PI * x)) / 2.0
				}
			},
		};
		(REF_HZ * libm::exp2f((st + self.bump_at(seg, frac)) / 12.0)).clamp(MIN_HZ, MAX_HZ)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::_utt::Utterance;

	const CW: u8 = 90;
	const FW: u8 = 95;
	const COMMA: u8 = 91;
	const PERIOD: u8 = 92;
	const QMARK: u8 = 93;
	const BANG: u8 = 94;

	// words are (function?, consonant token, stressed vowel token); tokens from PHNIDToPhoneme
	fn utt(words: &[(bool, u8, u8)], ends: &[(usize, u8)], first_text: &str) -> (Utterance, Contour) {
		let mut toks: Vec<u8> = Vec::new();
		let mut texts: Vec<&str> = Vec::new();
		for (i, &(func, c, v)) in words.iter().enumerate() {
			toks.push(if func { FW } else { CW });
			toks.push(c);
			toks.push(v);
			texts.push(if i == 0 { first_text } else { "w" });
			for &(after, p) in ends { if after == i { toks.push(p); } }
		}
		contour(Utterance::build(&toks, &texts))
	}

	// u timed at rate 1 and its pitch contour
	fn contour(mut u: Utterance) -> (Utterance, Contour) {
		crate::_dur::assign(&mut u, 1.0);
		let stream: Vec<usize> = (0..u.segs.len()).collect();
		let mut c = Contour::new();
		c.add_utterance(&u, &stream);
		(u, c)
	}
	fn st(hz: f32) -> f32 { 12.0 * (hz / REF_HZ).log2() }
	fn last_voiced(u: &Utterance) -> usize { u.segs.len() - 1 }

	// b 'aa, d 'iy, g 'ow, m 'aa, n 'iy
	const BAA: (bool, u8, u8) = (false, 39, 15);
	const DEE: (bool, u8, u8) = (false, 42, 26);
	const GO:  (bool, u8, u8) = (false, 48, 27);
	const MA:  (bool, u8, u8) = (false, 61, 15);
	const NEE: (bool, u8, u8) = (false, 64, 26);
	// ih t, a function word
	const IT:  (bool, u8, u8) = (true, 76, 25);

	#[test]
	fn question_ends_higher_than_statement() {
		let (us, mut cs) = utt(&[BAA, DEE], &[(1, PERIOD)], "w");
		let (uq, mut cq) = utt(&[BAA, DEE], &[(1, QMARK)], "w");
		let s = st(cs.hz_at(last_voiced(&us), 1.0));
		let q = st(cq.hz_at(last_voiced(&uq), 1.0));
		assert!(q - s >= 11.0, "question {} vs statement {}", q, s);
	}

	#[test]
	fn comma_phrase_ends_above_baseline() {
		let (u, mut c) = utt(&[BAA, DEE, GO, MA], &[(1, COMMA), (3, PERIOD)], "w");
		let end = u.words[1].segs.end - 1;
		let t = c.targets.iter().find(|t| t.kind == TargetKind::End && t.seg == end).unwrap();
		assert!((t.st - (t.base + CONTINUATION_RISE_ST)).abs() < 0.01);
		assert!((t.base - (0.0 - BASELINE_FALL_ST)).abs() < 0.01);
		assert!((st(c.hz_at(end, 1.0)) - 1.0).abs() < 0.5);
	}

	#[test]
	fn downstep_within_phrase_and_restart() {
		let (_, c) = utt(&[BAA, IT, DEE, IT, GO, MA, IT, NEE], &[(4, COMMA), (7, PERIOD)], "w");
		let h: Vec<f32> = c.targets.iter().filter(|t| t.kind == TargetKind::Accent).map(|t| t.st - t.base).collect();
		assert_eq!(h.len(), 5);
		assert!((h[0] - 6.0).abs() < 0.01);
		assert!((h[1] - 4.8).abs() < 0.01);
		assert!((h[2] - 3.84).abs() < 0.01);
		assert!((h[3] - 5.4).abs() < 0.01);
		assert!((h[4] - 4.32).abs() < 0.01);
	}

	#[test]
	fn downstep_floor() {
		// 13 syllables, one phrase
		let (_, c) = utt(&[BAA, IT, DEE, IT, GO, IT, MA, IT, NEE, IT, BAA, IT, DEE], &[(12, PERIOD)], "w");
		let h: Vec<f32> = c.targets.iter().filter(|t| t.kind == TargetKind::Accent).map(|t| t.st - t.base).collect();
		assert!(h.iter().all(|&x| x >= ACCENT_FLOOR_ST - 0.01));
		assert!((h[6] - ACCENT_FLOOR_ST).abs() < 0.01);
	}

	#[test]
	fn long_list_keeps_pitch_up() {
		// twelve comma phrases: resets and accents must not sink without end
		let w = [BAA, DEE, GO, MA, NEE, BAA, DEE, GO, MA, NEE, BAA, DEE];
		let ends: Vec<(usize, u8)> = (0..12).map(|i| (i, if i == 11 { PERIOD } else { COMMA })).collect();
		let (_, c) = utt(&w, &ends, "w");
		for t in c.targets.iter().filter(|t| t.kind == TargetKind::Accent) {
			assert!(t.st - t.base >= ACCENT_FLOOR_ST - 0.01, "accent {} above baseline", t.st - t.base);
		}
		for t in c.targets.iter().filter(|t| t.kind == TargetKind::Start) {
			assert!(t.base >= -PHRASE_RESET_MAX_ST - 0.01, "phrase starts at {}", t.base);
		}
	}

	#[test]
	fn the_last_accent_peaks_early() {
		let (_, c) = utt(&[BAA, IT, DEE], &[(2, PERIOD)], "w");
		let fracs: Vec<f32> = c.targets.iter().filter(|t| t.kind == TargetKind::Accent).map(|t| t.frac).collect();
		assert_eq!(fracs, [ACCENT_FRAC, NUCLEAR_FRAC]);
	}

	#[test]
	fn an_exclamation_widens_accents_and_the_fall() {
		let (_, cs) = utt(&[BAA, IT, DEE], &[(2, PERIOD)], "w");
		let (_, ce) = utt(&[BAA, IT, DEE], &[(2, BANG)], "w");
		let rise = |c: &Contour, kind: TargetKind| c.targets.iter().find(|t| t.kind == kind).map(|t| t.st - t.base).unwrap();
		assert!(rise(&ce, TargetKind::Accent) > rise(&cs, TargetKind::Accent) + 1.0);
		assert!(rise(&ce, TargetKind::End) < rise(&cs, TargetKind::End) - 1.0);
		assert!((rise(&ce, TargetKind::Accent) - EXCLAIM_RANGE * rise(&cs, TargetKind::Accent)).abs() < 0.01);
		assert!((rise(&ce, TargetKind::End) - EXCLAIM_RANGE * rise(&cs, TargetKind::End)).abs() < 0.01);
	}

	#[test]
	fn a_voiced_obstruent_lowers_the_next_onset_and_high_vowels_rise() {
		// b 'aa, then d 'iy: aa starts low after b; iy, also after a voiced d, is still up at its middle
		let (u, mut c) = utt(&[BAA, DEE], &[(1, PERIOD)], "w");
		let nucleus = |w: usize| u.syls[u.words[w].syls.start].nucleus.unwrap();
		assert!(c.bump_at(nucleus(0), 0.0) < -0.5);
		assert!(c.bump_at(nucleus(1), 0.5) > 0.5);
	}

	#[test]
	fn deaccents_a_content_word_after_an_accented_one() {
		// b 'aa d 'iy g 'ow ih t m 'aa .: d 'iy follows an accent and is not last, so only it loses its accent
		let (u, c) = utt(&[BAA, DEE, GO, IT, MA], &[(4, PERIOD)], "w");
		let peaks: Vec<usize> = c.targets.iter().filter(|t| t.kind == TargetKind::Accent).map(|t| t.seg).collect();
		let nucleus = |w: usize| u.syls[u.words[w].syls.start].nucleus.unwrap();
		assert_eq!(peaks, vec![nucleus(0), nucleus(2), nucleus(4)]);
		// the last content word keeps its accent after an accented one
		let (u, c) = utt(&[BAA, DEE], &[(1, PERIOD)], "w");
		assert_eq!(c.targets.iter().filter(|t| t.kind == TargetKind::Accent).count(), 2, "{}", u.words.len());
	}

	#[test]
	fn minor_break_ends_with_a_small_rise() {
		let mut u = Utterance::build(&[CW, 39, 15, CW, 42, 26, COMMA, CW, 48, 27, PERIOD], &["w", "w", "w"]);
		u.phrases[0].end = Boundary::Minor;
		let (u, c) = contour(u);
		let end = u.words[1].segs.end - 1;
		let t = c.targets.iter().find(|t| t.kind == TargetKind::End && t.seg == end).unwrap();
		assert!((t.st - (t.base + MINOR_RISE_ST)).abs() < 0.01, "{} over {}", t.st, t.base);
	}

	#[test]
	fn wh_question_falls() {
		let (u, c) = utt(&[(true, 81, 26), BAA, DEE], &[(2, QMARK)], "where");
		assert_eq!(u.kind, crate::_utt::Kind::Wh);
		let t = c.targets.iter().find(|t| t.kind == TargetKind::End).unwrap();
		assert!((t.st - (t.base + FINAL_FALL_ST)).abs() < 0.01);
	}

	#[test]
	fn low_rise_question() {
		let (u, c) = utt(&[BAA, DEE], &[(1, QMARK)], "w");
		let low = c.targets.iter().find(|t| t.kind == TargetKind::Low).unwrap();
		assert_eq!(low.seg, u.syls[u.words[1].syls.start].nucleus.unwrap());
		assert!((low.frac - LOW_ACCENT_FRAC).abs() < 0.01);
		assert!((low.st - (low.base + LOW_ACCENT_ST)).abs() < 0.01);
		assert_eq!(c.targets.iter().filter(|t| t.kind == TargetKind::Accent).count(), 1);
	}

	#[test]
	fn exclamation_boosts_last_accent() {
		let (_, c) = utt(&[BAA, DEE], &[(1, BANG)], "w");
		let h: Vec<f32> = c.targets.iter().filter(|t| t.kind == TargetKind::Accent).map(|t| t.st - t.base).collect();
		assert!((h[1] - (4.8 + EXCLAIM_BOOST_ST) * EXCLAIM_RANGE).abs() < 0.01);
	}

	#[test]
	fn no_content_words() {
		// it is .
		let (_, c) = utt(&[(true, 76, 25), (true, 87, 25)], &[(1, PERIOD)], "it");
		assert_eq!(c.targets.iter().filter(|t| t.kind == TargetKind::Accent).count(), 0);
		assert!(c.targets.iter().any(|t| t.kind == TargetKind::Start));
		assert!(c.targets.iter().any(|t| t.kind == TargetKind::End));
	}

	#[test]
	fn close_accents_sag_rather_than_dip() {
		// "ma baa ... ma": the trough between the accents sits above the baseline while they are close, on it once
		// TROUGH_FULL_MS apart
		let (mut close, mut far) = (false, false);
		for fill in 0..8 {
			let mut w = vec![MA];
			w.extend(std::iter::repeat((true, 39, 15)).take(fill));
			w.push(MA);
			let (u, c) = utt(&w, &[(w.len() - 1, PERIOD)], "w");
			let t0: Vec<f32> = u.segs.iter().scan(0.0, |a, s| { let t = *a; *a += s.dur_ms; Some(t) }).collect();
			let time = |t: &Target| t0[t.seg] + t.frac * u.segs[t.seg].dur_ms;
			let acc: Vec<f32> = c.targets.iter().filter(|t| t.kind == TargetKind::Accent).map(time).collect();
			let Some(tr) = c.targets.iter().find(|t| t.kind == TargetKind::Trough) else { continue };
			let gap = acc[1] - acc[0];
			if gap >= TROUGH_FULL_MS {
				far = true;
				assert!((tr.st - tr.base).abs() < 0.01, "{} ms apart: trough {} st over the baseline", gap, tr.st - tr.base);
			} else {
				close = true;
				assert!(tr.st - tr.base > 0.1, "{} ms apart: trough on the baseline", gap);
			}
		}
		assert!(close && far, "close {} far {}", close, far);
	}

	#[test]
	fn clamped() {
		let w = [BAA, DEE, GO, MA, NEE, BAA, DEE, GO, MA, NEE];
		let (u, mut c) = utt(&w, &[(9, BANG)], "w");
		for s in 0..u.segs.len() {
			for k in 0..=10 {
				let hz = c.hz_at(s, k as f32 / 10.0);
				assert!((70.0..=250.0).contains(&hz), "{} Hz", hz);
			}
		}
	}

	// tokens from PHNIDToPhoneme, a word boundary before each word
	const S: u8 = 72; const L: u8 = 59; const M: u8 = 61; const D: u8 = 42; const T: u8 = 76; const K: u8 = 56;
	const OW1: u8 = 27; const IH1: u8 = 25;

	fn offsets(toks: &[u8]) -> (Utterance, Contour) {
		let words = toks.iter().filter(|&&t| t == CW || t == FW).count();
		contour(Utterance::build(toks, &vec!["w"; words]))
	}

	#[test]
	fn obstruents_raise_the_following_sonorant() {
		// "slow": the l starts raised and the rise carries on, decaying, into the vowel
		let (_, mut c) = offsets(&[CW, S, L, OW1, PERIOD]);
		assert!((c.bump_at(1, 0.0) - VOICELESS_BUMP_ST).abs() < 0.01);
		assert!(c.bump_at(2, 0.0) > 0.0 && c.bump_at(2, 0.0) < c.bump_at(1, 0.0));
		// a voiced obstruent raises it less
		let (_, mut c) = offsets(&[CW, D, OW1, PERIOD]);
		assert!((c.bump_at(1, 0.0) - VOICED_BUMP_ST).abs() < 0.01);
		// a nasal does nothing
		let (u, mut c) = offsets(&[CW, M, OW1, PERIOD]);
		assert!((0..u.segs.len()).all(|i| c.bump_at(i, 0.0) == 0.0));
	}

	#[test]
	fn word_final_t_before_a_consonant_dips() {
		// "mitt cow": F0 falls over the end of the vowel and stays low through the t
		let (u, mut c) = offsets(&[CW, M, IH1, T, CW, K, OW1, PERIOD]);
		assert!(u.segs[1].dur_ms > T_DIP_MS);
		assert_eq!(c.bump_at(1, 0.0), 0.0);
		assert!((c.bump_at(1, 1.0) + T_DIP_ST).abs() < 0.01);
		assert!((c.bump_at(2, 0.5) + T_DIP_ST).abs() < 0.01);
		// "mitt owe": not before a vowel
		let (_, mut c) = offsets(&[CW, M, IH1, T, CW, OW1, PERIOD]);
		assert_eq!(c.bump_at(2, 0.5), 0.0);
	}

	#[test]
	fn stressed_vowel_starting_a_content_word_has_a_glottal_onset() {
		// "mow it" with "it" a content word: after the vowel F0 falls into the catch over ONSET_LEAD_MS, then comes back
		let (u, mut c) = offsets(&[CW, M, OW1, CW, IH1, T, PERIOD]);
		let d = u.segs[2].dur_ms;
		assert_eq!(c.bump_at(2, 0.0), 0.0);
		// ih is a high vowel, so its own rise rides on top
		let high = Shape::Hump { st: HIGH_VOWEL_ST, ms: d }.at(ONSET_LEAD_MS);
		assert!((c.bump_at(2, ONSET_LEAD_MS / d) - high + ONSET_DIP_ST).abs() < 0.01);
		assert!(d > ONSET_LEAD_MS + ONSET_DIP_MS && c.bump_at(2, 1.0) == 0.0);
		// "it" after a pause starts low
		let (_, mut c) = offsets(&[CW, IH1, T, PERIOD]);
		assert!((c.bump_at(0, 0.0) + ONSET_DIP_ST).abs() < 0.01);
		// not for a function word
		let (_, mut c) = offsets(&[CW, M, OW1, FW, IH1, T, PERIOD]);
		assert_eq!(c.bump_at(2, 0.0), 0.0);
	}
}
