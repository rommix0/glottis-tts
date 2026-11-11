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

the contour is evaluated per sample from the synthesizer's position; between two targets the
pitch moves evenly in time.

*/
use crate::_utt::{Utterance, Kind, Boundary, is_vowel, is_voiced, is_obstruent, is_sonorant};
use crate::_phone::*;
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
const VOICED_DIP_ST:        f32 = 0.6;		// ... and lowered through a voiced one, deepest at its end,
const VOICED_DIP_UP_MS:     f32 = 90.0;		// back up over this long
const BUMP_SPAN_MS:         f32 = 200.0;	// a bump is drawn over this long at most
const T_DIP_ST:             f32 = 1.5;		// fall into a pre-glottalized word-final t, over T_DIP_MS before it
const T_DIP_MS:             f32 = 40.0;
// glottal onset: a stressed word-initial vowel starts this low, back by ONSET_DIP_MS
const ONSET_DIP_ST:         f32 = 1.5;
const ONSET_DIP_MS:         f32 = 60.0;
// after a voiced sound the vowel's slot opens on that sound's tail: fall over this first
const ONSET_LEAD_MS:        f32 = 40.0;
// accents this far apart dip to the baseline between them, closer ones sag less
const TROUGH_FULL_MS:       f32 = 500.0;
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
	time: f32,				// ms on the contour's clock
}

// a short pitch offset, drawn on each segment it covers; age is the ms since the offset began
struct Bump { seg: usize, age0: f32, dur: f32, shape: Shape }

#[derive(Clone, Copy)]
enum Shape {
	Decay { st: f32, ms: f32 },				// st at age 0, decaying with time constant ms
	Fall  { st: f32, ms: f32 },				// 0 at age 0, falling to -st at ms, then held
	Vee   { st: f32, down: f32, up: f32 },	// 0 at age 0, -st at down, back to 0 up later (down 0: starts at -st)
	Hump  { st: f32, ms: f32 },				// 0 at age 0, st halfway through ms, 0 again at ms
}

impl Shape {
	fn at(self, age: f32) -> f32 {
		if age < 0.0 { return 0.0; }
		match self {
			Shape::Decay { st, ms } => if age < BUMP_SPAN_MS { st * libm::expf(-age / ms) } else { 0.0 },
			Shape::Fall  { st, ms } => -st * (age / ms).min(1.0),
			Shape::Vee   { st, down, up } => if age < down {
				-st * age / down
			} else {
				-st * (1.0 - (age - down) / up).max(0.0)
			},
			Shape::Hump  { st, ms } => if age < ms { st * libm::sinf(PI * age / ms) } else { 0.0 },
		}
	}
}

// ms of speech after which the contour's clock is moved back (an f32 then still counts in steps of about 0.06 ms)
const CLOCK_LIMIT_MS: f32 = 600_000.0;

pub struct Contour {
	targets: Vec<Target>,
	bumps:   Vec<Bump>,
	starts:  Vec<(usize, f32, f32)>,	// (stream index, start ms, ms) of every utterance segment, on the clock
	clock:   f32,			// ms of utterance time added so far (pauses between utterances left out)
	prev_seg: usize,		// stream index and clock time where the previous utterance starts
	prev_ms:  f32,
	target_hint: usize,		// where the last hz_at found itself in targets
	bump_hint:   usize,		// and in bumps
	start_hint:  usize,		// and in starts
}

// the first item of v that before() says is not before the place sought, as partition_point gives; samples come in
// order, so the answer for the last one usually still holds and is checked before searching
fn find<T>(v: &[T], last: &mut usize, before: impl Fn(&T) -> bool) -> usize {
	let k = (*last).min(v.len());
	let k = if (k == 0 || before(&v[k - 1])) && (k == v.len() || !before(&v[k])) {
		k
	} else {
		v.partition_point(before)
	};
	*last = k;
	k
}

impl Contour {
	pub fn new() -> Contour {
		Contour {
			targets: Vec::new(), bumps: Vec::new(), starts: Vec::new(), clock: 0.0, prev_seg: 0, prev_ms: 0.0,
			target_hint: 0, bump_hint: 0, start_hint: 0,
		}
	}

	pub fn targets_of(&self, seg: usize) -> impl Iterator<Item = &Target> {
		self.targets.iter().filter(move |t| t.seg == seg)
	}

	pub fn add_utterance(&mut self, utt: &Utterance, stream: &[usize]) {
		// stream[i] is the stream index of utt.segs[i]
		if utt.segs.is_empty() { return; }
		self.drop_old(stream[0]);
		let mut lay = Layout::new(utt, stream, self.clock);
		lay.place_phrases();
		let kept = merge_close(core::mem::take(&mut lay.placed));
		// time order is stream order, and a later utterance has later stream indices, so targets and starts stay sorted
		self.targets.extend(kept.into_iter().map(|(_, _, tg)| tg));
		self.starts.extend(utt.segs.iter().enumerate().map(|(i, s)| (stream[i], lay.clock + lay.t0[i], s.dur_ms)));
		self.clock += lay.t0[utt.segs.len()];
		lay.add_offsets();
		self.bumps.extend(lay.bumps);
	}

	// the synthesizer is at most a phone or two behind, so nothing before the previous utterance is asked for
	// again: drop it; past CLOCK_LIMIT_MS move the clock back to where that utterance starts, so an f32 still
	// counts it finely
	fn drop_old(&mut self, first_seg: usize) {
		let keep = self.prev_seg;
		let back = if self.clock > CLOCK_LIMIT_MS { self.prev_ms } else { 0.0 };
		self.targets.drain(..self.targets.partition_point(|t| t.seg < keep));
		self.bumps.drain(..self.bumps.partition_point(|b| b.seg < keep));
		self.starts.drain(..self.starts.partition_point(|s| s.0 < keep));
		for t in &mut self.targets { t.time -= back; }
		for s in &mut self.starts { s.1 -= back; }
		self.clock -= back;
		(self.target_hint, self.bump_hint, self.start_hint) = (0, 0, 0);
		(self.prev_seg, self.prev_ms) = (first_seg, self.clock);
	}

	// the short offsets at a position, in semitones
	fn bump_at(&mut self, seg: usize, frac: f32) -> f32 {
		let k = find(&self.bumps, &mut self.bump_hint, |b| b.seg < seg);
		self.bumps[k..].iter().take_while(|b| b.seg == seg).map(|b| b.shape.at(b.age0 + frac * b.dur)).sum()
	}

	pub fn hz_at(&mut self, seg: usize, frac: f32) -> f32 {
		// order by segment first, so the end of one segment comes before the start of the next
		let j = find(&self.targets, &mut self.target_hint, |t| (t.seg, t.frac) <= (seg, frac));
		let st = match j {
			_ if self.targets.is_empty() => 0.0,
			_ if j == self.targets.len() => self.targets[j - 1].st,
			0 => self.targets[0].st,
			j => {
				let (a, b) = (&self.targets[j - 1], &self.targets[j]);
				if a.kind == TargetKind::End {
					a.st
				} else {
					// between two targets inside a phrase every segment is an utterance segment, so it has a start time
					let k = find(&self.starts, &mut self.start_hint, |s| s.0 < seg);
					let x = match self.starts.get(k) {
						Some(&(s, start, dur)) if s == seg && b.time > a.time =>
							((start + frac * dur - a.time) / (b.time - a.time)).clamp(0.0, 1.0),
						_ => 1.0,
					};
					a.st + (b.st - a.st) * (1.0 - libm::cosf(PI * x)) / 2.0
				}
			},
		};
		(REF_HZ * libm::exp2f((st + self.bump_at(seg, frac)) / 12.0)).clamp(MIN_HZ, MAX_HZ)
	}
}

// targets closer than MERGE_MS within a phrase: keep the later one
// (phrases are apart by their pause, which utterance time leaves out)
fn merge_close(mut placed: Vec<(usize, f32, Target)>) -> Vec<(usize, f32, Target)> {
	placed.sort_by(|a, b| (a.0, a.1).partial_cmp(&(b.0, b.1)).unwrap());
	let mut kept: Vec<(usize, f32, Target)> = Vec::new();
	for (ph, t, tg) in placed {
		if let Some(&(pph, pt, _)) = kept.last() {
			if pph == ph && t - pt < MERGE_MS { kept.pop(); }
		}
		kept.push((ph, t, tg));
	}
	kept
}

// an utterance being added to the contour: its segment times, and the targets and offsets placed on them
struct Layout<'a> {
	utt:    &'a Utterance,
	stream: &'a [usize],				// stream[i] is the stream index of utt.segs[i]
	t0:     Vec<f32>,					// start time of each segment (ms) within the utterance, then its end
	clock:  f32,						// the contour's clock where the utterance starts
	placed: Vec<(usize, f32, Target)>,	// (phrase, time, target)
	bumps:  Vec<Bump>,
}

impl<'a> Layout<'a> {
	fn new(utt: &'a Utterance, stream: &'a [usize], clock: f32) -> Layout<'a> {
		// start time of each segment (ms) within the utterance
		let mut t0: Vec<f32> = Vec::with_capacity(utt.segs.len() + 1);
		let mut acc = 0.0;
		for s in &utt.segs {
			t0.push(acc);
			acc += s.dur_ms;
		}
		t0.push(acc);
		Layout { utt, stream, t0, clock, placed: Vec::new(), bumps: Vec::new() }
	}

	fn time(&self, i: usize, frac: f32) -> f32 { self.t0[i] + frac * self.utt.segs[i].dur_ms }

	// the accented syllable nucleus of each content word
	fn accent_of(&self, w: usize) -> Option<usize> {
		let word = &self.utt.words[w];
		if word.function { return None; }
		let syls = &self.utt.syls[word.syls.clone()];
		syls.iter().find(|s| s.stress == 1).or_else(|| syls.iter().find(|s| s.stress == 2)).and_then(|s| s.nucleus)
	}

	fn push(&mut self, i: usize, frac: f32, st: f32, base: f32, kind: TargetKind) {
		self.placed.push((
			self.utt.phrase_of_seg(i),
			self.time(i, frac),
			Target { seg: self.stream[i], frac, st, base, kind, time: self.clock + self.time(i, frac) },
		));
	}

	fn phrase_segs(&self, k: usize) -> core::ops::Range<usize> {
		let ph = &self.utt.phrases[k];
		self.utt.words[ph.words.start].segs.start..self.utt.words[ph.words.end - 1].segs.end
	}

	// phrase k's baseline at utterance time t
	fn base(&self, k: usize, t: f32) -> f32 {
		let segs = self.phrase_segs(k);
		let b0 = (-PHRASE_RESET_ST * k as f32).max(-PHRASE_RESET_MAX_ST);
		let (ts, te) = (self.t0[segs.start], self.t0[segs.end]);
		b0 - BASELINE_FALL_ST * if te > ts { (t - ts) / (te - ts) } else { 0.0 }
	}

	fn place_phrases(&mut self) {
		let mut first_height = ACCENT_ST;
		let last_accent_seg = (0..self.utt.words.len()).rev().find_map(|w| self.accent_of(w));
		for k in 0..self.utt.phrases.len() {
			self.place_phrase(k, &mut first_height, last_accent_seg);
		}
	}

	// phrase k's start, accents and end
	fn place_phrase(&mut self, k: usize, first_height: &mut f32, last_accent_seg: Option<usize>) {
		let utt = self.utt;
		let ph = &utt.phrases[k];
		let segs = self.phrase_segs(k);
		let voiced = |i: &usize| is_voiced(utt.segs[*i].phone);
		let (Some(first_v), Some(last_v)) = (segs.clone().find(voiced), segs.clone().rfind(voiced)) else {
			return;
		};

		let b = self.base(k, self.time(first_v, 0.0));
		self.push(first_v, 0.0, b, b, TargetKind::Start);

		// accents, stepping down within the phrase
		if k > 0 { *first_height = (*first_height * PHRASE_ACCENT_RATIO).max(ACCENT_FLOOR_ST); }
		self.place_accents(k, *first_height, last_accent_seg);

		// phrase end
		let be = self.base(k, self.time(last_v, 1.0));
		let end_st = match (ph.end, utt.kind) {
			(Boundary::Comma, _)   => CONTINUATION_RISE_ST,
			(Boundary::Minor, _)   => MINOR_RISE_ST,
			(_, Kind::YesNo)       => QUESTION_RISE_ST,
			(_, Kind::Exclamation) => FINAL_FALL_ST * EXCLAIM_RANGE,
			_                      => FINAL_FALL_ST,
		};
		self.push(last_v, 1.0, be + end_st, be, TargetKind::End);
	}

	fn place_accents(&mut self, k: usize, first_height: f32, last_accent_seg: Option<usize>) {
		let utt = self.utt;
		let ph = &utt.phrases[k];
		let last_phrase = utt.phrases.len() - 1;
		let mut h = first_height;
		let mut n_acc = 0;
		let mut prev_peak: Option<(usize, f32, f32)> = None;
		let last_content = ph.words.clone().rev().find(|&w| self.accent_of(w).is_some());
		let mut accented: Option<usize> = None;
		for w in ph.words.clone() {
			let Some(n) = self.accent_of(w) else { continue; };
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
			let tp = self.time(n, frac);
			let height = match kind {
				TargetKind::Low => LOW_ACCENT_ST,
				_ if utt.kind != Kind::Exclamation => h,
				_ if is_last => (h + EXCLAIM_BOOST_ST) * EXCLAIM_RANGE,
				_ => h * EXCLAIM_RANGE,
			};
			if let Some(prev) = prev_peak {
				if tp - prev.1 > TROUGH_GAP_MS { self.place_trough(k, prev, (n, tp, height)); }
			}
			self.push(n, frac, self.base(k, tp) + height, self.base(k, tp), kind);
			prev_peak = Some((n, tp, height));
		}
	}

	// trough on the baseline halfway between the peaks, each (segment, time, height)
	fn place_trough(&mut self, k: usize, prev: (usize, f32, f32), next: (usize, f32, f32)) {
		let ((prev_i, prev_t, prev_h), (n, tp, height)) = (prev, next);
		let tm = (prev_t + tp) / 2.0;
		let i = (prev_i..=n).find(|&i| self.t0[i + 1] >= tm).unwrap_or(n);
		let fr = if self.utt.segs[i].dur_ms > 0.0 {
			((tm - self.t0[i]) / self.utt.segs[i].dur_ms).clamp(0.0, 1.0)
		} else {
			0.0
		};
		let apart = ((tp - prev_t - TROUGH_GAP_MS) / (TROUGH_FULL_MS - TROUGH_GAP_MS)).min(1.0);
		let sag = (1.0 - apart) * prev_h.min(height).max(0.0);
		self.push(i, fr, self.base(k, tm) + sag, self.base(k, tm), TargetKind::Trough);
	}

	fn phone(&self, i: usize) -> u8 { self.utt.segs[i].phone }
	fn word_start(&self, i: usize) -> bool { self.utt.words[self.utt.segs[i].word].segs.start == i }
	fn word_end(&self, i: usize) -> bool { self.utt.words[self.utt.segs[i].word].segs.end == i + 1 }
	fn same_phrase(&self, i: usize, j: usize) -> bool { self.utt.phrase_of_seg(i) == self.utt.phrase_of_seg(j) }
	fn after_pause(&self, i: usize) -> bool { i == 0 || !self.same_phrase(i - 1, i) }
	// the phone before segment i, PAU at a phrase start
	fn prev_phone(&self, i: usize) -> u8 { if self.after_pause(i) { PAU } else { self.phone(i - 1) } }

	fn draw(&mut self, segs: core::ops::Range<usize>, start: f32, shape: Shape) {
		for j in segs {
			let bump = Bump { seg: self.stream[j], age0: self.t0[j] - start, dur: self.utt.segs[j].dur_ms, shape };
			self.bumps.push(bump);
		}
	}

	// short offsets: microprosody and glottal dips
	fn add_offsets(&mut self) {
		let utt = self.utt;
		for i in 0..utt.segs.len() {
			self.add_microprosody(i);
			self.add_t_dip(i);
			self.add_glottal_onset(i);
			// ih iy uh uw: intrinsically a little higher
			let s = &utt.segs[i];
			if matches!(s.phone, IH | IY | UH | UW) {
				self.draw(i..i + 1, self.t0[i], Shape::Hump { st: HIGH_VOWEL_ST, ms: s.dur_ms });
			}
		}
		self.bumps.sort_by_key(|b| b.seg);
	}

	// a sonorant or vowel after an obstruent (not a flap), until the next obstruent or the phrase end: after a
	// voiceless one raised and decaying; a voiced one lowers F0 through itself, back up in the next sound
	fn add_microprosody(&mut self, i: usize) {
		let n = self.utt.segs.len();
		let (p, prev) = (self.phone(i), self.prev_phone(i));
		if !self.after_pause(i) && is_sonorant(p) && is_obstruent(prev) && prev != DX {
			let voiced = is_voiced(prev);
			let span = if voiced { VOICED_DIP_UP_MS } else { BUMP_SPAN_MS };
			let t0 = &self.t0;
			let end = (i..n)
				.find(|&j| is_obstruent(self.phone(j)) || !self.same_phrase(i, j) || t0[j] - t0[i] >= span)
				.unwrap_or(n);
			if voiced {
				self.draw(i - 1..end, self.t0[i - 1],
					Shape::Vee { st: VOICED_DIP_ST, down: self.t0[i] - self.t0[i - 1], up: VOICED_DIP_UP_MS });
			} else {
				self.draw(i..end, self.t0[i], Shape::Decay { st: VOICELESS_BUMP_ST, ms: VOICELESS_BUMP_MS });
			}
		}
	}

	// a word-final t before a consonant in the same phrase, after a sonorant: F0 falls into it and stays
	// low through it
	fn add_t_dip(&mut self, i: usize) {
		let n = self.utt.segs.len();
		let (p, prev) = (self.phone(i), self.prev_phone(i));
		if p == T && !self.after_pause(i) && is_sonorant(prev) && self.word_end(i) && i + 1 < n
			&& self.same_phrase(i, i + 1) && !is_vowel(self.phone(i + 1)) {
			let start = self.t0[i] - T_DIP_MS;
			let first = (0..i).rev()
				.take_while(|&j| self.t0[j + 1] > start && self.same_phrase(j, i))
				.last()
				.unwrap_or(i);
			self.draw(first..i + 1, start, Shape::Fall { st: T_DIP_ST, ms: T_DIP_MS });
		}
	}

	// a stressed vowel starting a content word, after a sonorant or a pause: a glottal onset
	fn add_glottal_onset(&mut self, i: usize) {
		let utt = self.utt;
		let n = utt.segs.len();
		let after_pause = self.after_pause(i);
		let (p, prev) = (self.phone(i), self.prev_phone(i));
		let s = &utt.segs[i];
		if is_vowel(p) && s.stress > 0 && self.word_start(i) && !utt.words[s.word].function
			&& (after_pause || is_sonorant(prev)) {
			let (span, shape) = if !after_pause {
				(
					ONSET_LEAD_MS + ONSET_DIP_MS,
					Shape::Vee { st: ONSET_DIP_ST, down: ONSET_LEAD_MS, up: ONSET_DIP_MS },
				)
			} else {
				(ONSET_DIP_MS, Shape::Vee { st: ONSET_DIP_ST, down: 0.0, up: ONSET_DIP_MS })
			};
			let end = (i..n).find(|&j| self.t0[j] - self.t0[i] >= span || !self.same_phrase(i, j)).unwrap_or(n);
			self.draw(i..end, self.t0[i], shape);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	// boundary tokens, as the front end writes them
	use crate::_phone::{TOKEN_CW as CW, TOKEN_FW as FW, TOKEN_COMMA as COMMA};
	use crate::_phone::{TOKEN_PERIOD as PERIOD, TOKEN_QMARK as QMARK, TOKEN_BANG as BANG};

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

	#[test]
	fn contour_stays_small_over_a_long_stream() {
		// sentence after sentence, only the last two are kept and the clock is moved back past CLOCK_LIMIT_MS; the
		// newest sentence's pitch is the same as on a fresh contour
		let (u, mut fresh) = utt(&[BAA, DEE, GO, MA, NEE], &[(4, PERIOD)], "w");
		let n = u.segs.len();
		let ms: f32 = u.segs.iter().map(|s| s.dur_ms).sum();
		let mut c = Contour::new();
		let mut k = 0;
		while (k as f32) * ms < 3.0 * CLOCK_LIMIT_MS {
			let stream: Vec<usize> = (k * n..(k + 1) * n).collect();
			c.add_utterance(&u, &stream);
			assert!(c.starts.len() <= 2 * n && c.targets.len() <= 2 * fresh.targets.len(),
				"{} starts after {} sentences", c.starts.len(), k + 1);
			k += 1;
		}
		assert!(c.clock < CLOCK_LIMIT_MS + 3.0 * ms, "clock at {} ms after {} ms of speech", c.clock, k as f32 * ms);
		for i in 0..n {
			let (a, b) = (fresh.hz_at(i, 0.5), c.hz_at((k - 1) * n + i, 0.5));
			assert!((a - b).abs() < 0.01, "segment {}: {} Hz on a fresh contour, {} after {} sentences", i, a, b, k);
		}
	}

	#[test]
	fn pitch_moves_evenly_in_time() {
		// halfway in time between two targets the contour is halfway between them, whatever segments lie in between
		let (u, mut c) = utt(&[BAA, DEE, GO, MA, NEE], &[(4, PERIOD)], "w");
		let n = c.targets.len();
		let (a, b) = (0..n - 1)
			.map(|k| (&c.targets[k], &c.targets[k + 1]))
			.find(|(a, b)| a.kind != TargetKind::End && b.seg > a.seg + 1)
			.unwrap();
		let (mid, half) = ((a.time + b.time) / 2.0, (a.st + b.st) / 2.0);
		let mut t = 0.0;
		let i = (0..u.segs.len()).find(|&i| { t += u.segs[i].dur_ms; t > mid }).unwrap();
		let frac = 1.0 - (t - mid) / u.segs[i].dur_ms;
		let got = st(c.hz_at(i, frac)) - c.bump_at(i, frac);
		assert!((got - half).abs() < 0.05, "{} st halfway in time, want {}", got, half);
	}

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
		let rise = |c: &Contour, kind: TargetKind| c.targets.iter()
			.find(|t| t.kind == kind)
			.map(|t| t.st - t.base)
			.unwrap();
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
				assert!((tr.st - tr.base).abs() < 0.01,
					"{} ms apart: trough {} st over the baseline", gap, tr.st - tr.base);
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
		// a voiced obstruent lowers it, from nothing at its start (no step in the voice) to deepest at its end
		let (_, mut c) = offsets(&[CW, D, OW1, PERIOD]);
		assert_eq!(c.bump_at(0, 0.0), 0.0);
		assert!((c.bump_at(0, 0.5) + VOICED_DIP_ST / 2.0).abs() < 0.01);
		assert!((c.bump_at(1, 0.0) + VOICED_DIP_ST).abs() < 0.01);
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
		// "mow it" with "it" a content word: after the vowel F0 falls into the catch over ONSET_LEAD_MS, then
		// comes back
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
