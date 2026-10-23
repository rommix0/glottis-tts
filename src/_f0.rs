/*

PITCH CONTOUR

pitch targets placed on segments (a stream index and a position inside
the segment), in semitones from REF_HZ:

	+ a baseline per phrase that falls over the phrase, each later phrase
	  starting a little lower
	+ an accent peak on the stressed vowel of every content word, stepping
	  down within the phrase and restarting at each phrase
	+ a trough on the baseline between accents that are far apart
	+ phrase ends: a fall for statements, a rise for yes/no questions,
	  a small rise at commas and a smaller one at minor breaks

the contour is evaluated per sample from the synthesizer's position.

*/
use crate::_utt::{Utterance, Kind, Boundary, is_vowel, is_voiced};
use core::f32::consts::PI;

pub const REF_HZ: f32 = 110.0;
pub const LOW_RISE_QUESTION: bool = true;
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
const QUESTION_RISE_ST:     f32 = 8.0;
const LOW_ACCENT_ST:        f32 = -1.0;
const LOW_ACCENT_FRAC:      f32 = 0.3;
const CONTINUATION_RISE_ST: f32 = 3.0;
const MINOR_RISE_ST:        f32 = 1.5;	// at a minor break inside a long phrase
const ONSET_BUMP_ST:        f32 = 0.5;
const ONSET_BUMP_MS:        f32 = 40.0;
const MERGE_MS:             f32 = 20.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tk { Start, Accent, Low, Trough, End }

#[derive(Debug, Clone, Copy)]
pub struct Target {
	pub seg:  usize,		// stream index of the segment
	pub frac: f32,			// position inside the segment (0..1)
	pub st:   f32,			// semitones from REF_HZ
	pub base: f32,			// the baseline here (for tracing and tests)
	pub kind: Tk,
	pub hold: bool,			// hold this value until the next target
}

// a short pitch offset at a vowel onset
struct Bump { seg: usize, st: f32, frac_end: f32 }

pub struct Contour {
	targets: Vec<Target>,
	bumps:   Vec<Bump>,
}

impl Contour {
	pub fn new() -> Contour { Contour { targets: Vec::new(), bumps: Vec::new() } }

	pub fn targets_of(&self, seg: usize) -> Vec<Target> {
		self.targets.iter().filter(|t| t.seg == seg).copied().collect()
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
		let push = |new: &mut Vec<(usize, f32, Target)>, i: usize, frac: f32, st: f32, base: f32, kind: Tk, hold: bool| {
			new.push((utt.phrase_of_seg(i), time(i, frac), Target { seg: stream[i], frac, st, base, kind, hold }));
		};

		let last_phrase = utt.phrases.len() - 1;
		let mut first_height = ACCENT_ST;
		let last_accent_seg = (0..utt.words.len()).rev().find_map(|w| accent_of(w));
		for (k, ph) in utt.phrases.iter().enumerate() {
			let segs = utt.words[ph.words.start].segs.start..utt.words[ph.words.end - 1].segs.end;
			let voiced: Vec<usize> = segs.clone().filter(|&i| is_voiced(utt.segs[i].phone)).collect();
			let (Some(&first_v), Some(&last_v)) = (voiced.first(), voiced.last()) else { continue; };

			let b0 = (-PHRASE_RESET_ST * k as f32).max(-PHRASE_RESET_MAX_ST);
			let (ts, te) = (t0[segs.start], t0[segs.end]);
			let base = |t: f32| b0 - BASELINE_FALL_ST * if te > ts { (t - ts) / (te - ts) } else { 0.0 };

			push(&mut new, first_v, 0.0, base(time(first_v, 0.0)), base(time(first_v, 0.0)), Tk::Start, false);

			// accents, stepping down within the phrase
			if k > 0 { first_height = (first_height * PHRASE_ACCENT_RATIO).max(ACCENT_FLOOR_ST); }
			let mut h = first_height;
			let mut n_acc = 0;
			let mut prev_peak: Option<(usize, f32)> = None;
			for w in ph.words.clone() {
				let Some(n) = accent_of(w) else { continue; };
				if n_acc > 0 { h = (h * DOWNSTEP).max(ACCENT_FLOOR_ST); }
				n_acc += 1;
				let is_last = Some(n) == last_accent_seg && k == last_phrase;
				let (frac, kind) = if is_last && utt.kind == Kind::YesNo && LOW_RISE_QUESTION { (LOW_ACCENT_FRAC, Tk::Low) } else { (ACCENT_FRAC, Tk::Accent) };
				let tp = time(n, frac);
				if let Some((pi, pt)) = prev_peak {
					if tp - pt > TROUGH_GAP_MS {
						// trough on the baseline halfway between the peaks
						let tm = (pt + tp) / 2.0;
						let i = (pi..=n).find(|&i| t0[i + 1] >= tm).unwrap_or(n);
						let fr = if utt.segs[i].dur_ms > 0.0 { ((tm - t0[i]) / utt.segs[i].dur_ms).clamp(0.0, 1.0) } else { 0.0 };
						push(&mut new, i, fr, base(tm), base(tm), Tk::Trough, false);
					}
				}
				let height = match kind {
					Tk::Low => LOW_ACCENT_ST,
					_ if is_last && utt.kind == Kind::Exclamation => h + EXCLAIM_BOOST_ST,
					_ => h,
				};
				push(&mut new, n, frac, base(tp) + height, base(tp), kind, false);
				prev_peak = Some((n, tp));
			}

			// phrase end
			let be = base(time(last_v, 1.0));
			let end_st = if ph.end == Boundary::Comma {
				CONTINUATION_RISE_ST
			} else if ph.end == Boundary::Minor {
				MINOR_RISE_ST
			} else if utt.kind == Kind::YesNo {
				QUESTION_RISE_ST
			} else {
				FINAL_FALL_ST
			};
			push(&mut new, last_v, 1.0, be + end_st, be, Tk::End, true);
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

		// small pitch offsets at vowel onsets after a consonant
		for i in 1..utt.segs.len() {
			let (p, q) = (utt.segs[i].phone, utt.segs[i - 1].phone);
			if !is_vowel(p) || is_vowel(q) { continue; }
			let st = if is_voiced(q) { -ONSET_BUMP_ST } else { ONSET_BUMP_ST };
			let frac_end = if utt.segs[i].dur_ms > 0.0 { (ONSET_BUMP_MS / utt.segs[i].dur_ms).min(1.0) } else { 0.0 };
			self.bumps.push(Bump { seg: stream[i], st, frac_end });
		}
	}

	pub fn hz_at(&self, seg: usize, frac: f32) -> f32 {
		let pos = seg as f32 + frac;
		// order by segment first, so the end of one segment comes before the start of the next
		let j = self.targets.partition_point(|t| (t.seg, t.frac) <= (seg, frac));
		let st = match j {
			_ if self.targets.is_empty() => 0.0,
			_ if j == self.targets.len() => self.targets[j - 1].st,
			0 => self.targets[0].st,
			j => {
				let (a, b) = (&self.targets[j - 1], &self.targets[j]);
				if a.hold {
					a.st
				} else {
					let (pa, pb) = (a.seg as f32 + a.frac, b.seg as f32 + b.frac);
					let x = if pb > pa { (pos - pa) / (pb - pa) } else { 1.0 };
					a.st + (b.st - a.st) * (1.0 - (PI * x).cos()) / 2.0
				}
			},
		};
		let k = self.bumps.partition_point(|b| b.seg < seg);
		let bump = self.bumps.get(k).filter(|b| b.seg == seg && frac < b.frac_end).map(|b| b.st * (1.0 - frac / b.frac_end)).unwrap_or(0.0);
		(REF_HZ * 2.0_f32.powf((st + bump) / 12.0)).clamp(MIN_HZ, MAX_HZ)
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
		let mut texts: Vec<String> = Vec::new();
		for (i, &(func, c, v)) in words.iter().enumerate() {
			toks.push(if func { FW } else { CW });
			toks.push(c);
			toks.push(v);
			texts.push(if i == 0 { first_text.to_string() } else { "w".to_string() });
			for &(after, p) in ends { if after == i { toks.push(p); } }
		}
		let mut u = Utterance::build(&toks, &texts);
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

	#[test]
	fn question_ends_higher_than_statement() {
		let (us, cs) = utt(&[BAA, DEE], &[(1, PERIOD)], "w");
		let (uq, cq) = utt(&[BAA, DEE], &[(1, QMARK)], "w");
		let s = st(cs.hz_at(last_voiced(&us), 1.0));
		let q = st(cq.hz_at(last_voiced(&uq), 1.0));
		assert!(q - s >= 11.0, "question {} vs statement {}", q, s);
	}

	#[test]
	fn comma_phrase_ends_above_baseline() {
		let (u, c) = utt(&[BAA, DEE, GO, MA], &[(1, COMMA), (3, PERIOD)], "w");
		let end = u.words[1].segs.end - 1;
		let t = c.targets.iter().find(|t| t.kind == Tk::End && t.seg == end).unwrap();
		assert!((t.st - (t.base + CONTINUATION_RISE_ST)).abs() < 0.01);
		assert!((t.base - (0.0 - BASELINE_FALL_ST)).abs() < 0.01);
		assert!((st(c.hz_at(end, 1.0)) - 1.0).abs() < 0.5);
	}

	#[test]
	fn downstep_within_phrase_and_restart() {
		let (_, c) = utt(&[BAA, DEE, GO, MA, NEE], &[(2, COMMA), (4, PERIOD)], "w");
		let h: Vec<f32> = c.targets.iter().filter(|t| t.kind == Tk::Accent).map(|t| t.st - t.base).collect();
		assert_eq!(h.len(), 5);
		assert!((h[0] - 6.0).abs() < 0.01);
		assert!((h[1] - 4.8).abs() < 0.01);
		assert!((h[2] - 3.84).abs() < 0.01);
		assert!((h[3] - 5.4).abs() < 0.01);
		assert!((h[4] - 4.32).abs() < 0.01);
	}

	#[test]
	fn downstep_floor() {
		let (_, c) = utt(&[BAA, DEE, GO, MA, NEE, BAA, DEE, GO], &[(7, PERIOD)], "w");
		let h: Vec<f32> = c.targets.iter().filter(|t| t.kind == Tk::Accent).map(|t| t.st - t.base).collect();
		assert!(h.iter().all(|&x| x >= ACCENT_FLOOR_ST - 0.01));
		assert!((h[7] - ACCENT_FLOOR_ST).abs() < 0.01);
	}

	#[test]
	fn long_list_keeps_pitch_up() {
		// twelve comma phrases: resets and accents must not sink without end
		let w = [BAA, DEE, GO, MA, NEE, BAA, DEE, GO, MA, NEE, BAA, DEE];
		let ends: Vec<(usize, u8)> = (0..12).map(|i| (i, if i == 11 { PERIOD } else { COMMA })).collect();
		let (_, c) = utt(&w, &ends, "w");
		for t in c.targets.iter().filter(|t| t.kind == Tk::Accent) {
			assert!(t.st - t.base >= ACCENT_FLOOR_ST - 0.01, "accent {} above baseline", t.st - t.base);
		}
		for t in c.targets.iter().filter(|t| t.kind == Tk::Start) {
			assert!(t.base >= -PHRASE_RESET_MAX_ST - 0.01, "phrase starts at {}", t.base);
		}
	}

	#[test]
	fn minor_break_ends_with_a_small_rise() {
		let mut u = Utterance::build(&[CW, 39, 15, CW, 42, 26, COMMA, CW, 48, 27, PERIOD], &["w".to_string(), "w".to_string(), "w".to_string()]);
		u.phrases[0].end = Boundary::Minor;
		crate::_dur::assign(&mut u, 1.0);
		let stream: Vec<usize> = (0..u.segs.len()).collect();
		let mut c = Contour::new();
		c.add_utterance(&u, &stream);
		let end = u.words[1].segs.end - 1;
		let t = c.targets.iter().find(|t| t.kind == Tk::End && t.seg == end).unwrap();
		assert!((t.st - (t.base + MINOR_RISE_ST)).abs() < 0.01, "{} over {}", t.st, t.base);
	}

	#[test]
	fn wh_question_falls() {
		let (u, c) = utt(&[(true, 81, 26), BAA, DEE], &[(2, QMARK)], "where");
		assert_eq!(u.kind, crate::_utt::Kind::Wh);
		let t = c.targets.iter().find(|t| t.kind == Tk::End).unwrap();
		assert!((t.st - (t.base + FINAL_FALL_ST)).abs() < 0.01);
	}

	#[test]
	fn low_rise_question() {
		let (u, c) = utt(&[BAA, DEE], &[(1, QMARK)], "w");
		let low = c.targets.iter().find(|t| t.kind == Tk::Low).unwrap();
		assert_eq!(low.seg, u.syls[u.words[1].syls.start].nucleus.unwrap());
		assert!((low.frac - LOW_ACCENT_FRAC).abs() < 0.01);
		assert!((low.st - (low.base + LOW_ACCENT_ST)).abs() < 0.01);
		assert_eq!(c.targets.iter().filter(|t| t.kind == Tk::Accent).count(), 1);
	}

	#[test]
	fn exclamation_boosts_last_accent() {
		let (_, c) = utt(&[BAA, DEE], &[(1, BANG)], "w");
		let h: Vec<f32> = c.targets.iter().filter(|t| t.kind == Tk::Accent).map(|t| t.st - t.base).collect();
		assert!((h[1] - (4.8 + EXCLAIM_BOOST_ST)).abs() < 0.01);
	}

	#[test]
	fn no_content_words() {
		// it is .
		let (_, c) = utt(&[(true, 76, 25), (true, 87, 25)], &[(1, PERIOD)], "it");
		assert_eq!(c.targets.iter().filter(|t| t.kind == Tk::Accent).count(), 0);
		assert!(c.targets.iter().any(|t| t.kind == Tk::Start));
		assert!(c.targets.iter().any(|t| t.kind == Tk::End));
	}

	#[test]
	fn clamped() {
		let w = [BAA, DEE, GO, MA, NEE, BAA, DEE, GO, MA, NEE];
		let (u, c) = utt(&w, &[(9, BANG)], "w");
		for s in 0..u.segs.len() {
			for k in 0..=10 {
				let hz = c.hz_at(s, k as f32 / 10.0);
				assert!((70.0..=250.0).contains(&hz), "{} Hz", hz);
			}
		}
	}
}
