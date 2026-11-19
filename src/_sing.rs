/*

SINGING

notes in, sung samples out. a note has a MIDI key, a length and a lyric:

	+ a word (or a few) starts a new word on its first syllable
	+ "-" takes the next syllable of the word being sung, or holds its vowel
	  on a new pitch once the word has none left
	+ "" is a rest

a word with more syllables than notes splits its last note among the rest.

timing: a syllable's vowel starts on its note. the consonants before the
vowel come in ahead of the beat, out of the note or rest before; the ones
after it, and the release before a rest, close its last note. consonants
take at most MAX_CONSONANT_SHARE of the time they come out of.

pitch, in layers on the notes:

	+ a glide into each note that ends on its start, with a small dip the
	  other way before it and an overshoot after it
	+ a scoop up into a note after a rest, a fall at the end of one before a rest
	+ vibrato that sets in a while into a note and fades before it ends

each sing() call is a passage of its own: start it with a rest to give the
first consonants their time ahead of the beat.

*/
use core::f32::consts::PI;
use core::ops::Range;
use alloc::{format, string::String, vec::Vec};
use crate::{SpeechSink, Speaker, normalize, release_phone, run_synth};
use crate::{_synth, _vtm::m_sr};
use crate::_vtm::Voice;
use crate::_dread::dictreader;
use crate::_utils::cbuf;
use crate::_utt::Utterance;
use crate::_dur::{phone_dur, RELEASE_MS};
use crate::_f0::REF_HZ;
use crate::_phone::*;

// a lyric that carries on the word before
const HOLD: &str = "-";

// timing
const CONSONANT_SCALE:     f32 = 0.85;		// a sung consonant against its inherent speech length
const MAX_CONSONANT_SHARE: f32 = 0.85;		// of the note or rest the consonants come out of
const DIPHTHONG_GLIDE_MS:  f32 = 150.0;		// a held vowel keeps its first target until this long before its end
const VOWEL_BW_SCALE:      f32 = 1.3;		// a held vowel's still formants ring on a harmonic unless wider
const VOWEL_MIN_B2:        f32 = 130.0;		// ... and a narrower F2 rings on its harmonic all the same (ao)
const FLUSH_MS:            f32 = 50.0;		// pause and silence that play out the end in finish()
const SAMPLES_PER_MS:      f64 = m_sr as f64 / 1000.0;

// pitch, tuned by ear
const GLIDE_MS:         f32 = 60.0;			// a glide into the next note, plus GLIDE_MS_PER_ST a semitone
const GLIDE_MS_PER_ST:  f32 = 8.0;
const GLIDE_MAX_MS:     f32 = 160.0;
const GLIDE_MAX_SHARE:  f32 = 0.4;			// of the note it leaves
const BEND_SHARE:       f32 = 0.15;			// the dip before a glide and the overshoot after it, of the interval
const BEND_MAX_ST:      f32 = 0.5;
const PREP_MS:          f32 = 80.0;			// the dip, just before the glide
const OVERSHOOT_MS:     f32 = 120.0;		// the overshoot, just after the note starts
const SCOOP_ST:         f32 = 1.0;			// a note after a rest starts this low ...
const SCOOP_MS:         f32 = 100.0;		// ... and rises over this long
const FALL_ST:          f32 = 1.5;			// a note before a rest falls this far over its last FALL_MS
const FALL_MS:          f32 = 150.0;
const VIBRATO_HZ:       f32 = 5.5;
const VIBRATO_ST:       f32 = 0.35;			// peak deviation
const VIBRATO_DELAY_MS: f32 = 300.0;		// into the note
const VIBRATO_RISE_MS:  f32 = 400.0;
const VIBRATO_FADE_MS:  f32 = 150.0;		// before the note, or the glide out of it, ends
const MIN_HZ:           f32 = 40.0;
const MAX_HZ:           f32 = 1600.0;

pub struct Note<'a> {
	pub key:   u8,			// MIDI key: 60 is middle C, 69 is A4 at 440 Hz
	pub ms:    f32,			// length
	pub lyric: &'a str,		// a word, "-" for the next syllable or a held vowel, "" for a rest
}

// one phone to sing
struct Sung {
	phone:      u8,
	stressed:   bool,
	word_start: bool,
	ms:         f32,
}

// a syllable's phones and which of them is its vowel
struct Syllable {
	phones:  Vec<Sung>,
	nucleus: usize,
}

impl Syllable {
	fn onset(&mut self) -> &mut [Sung] { &mut self.phones[..self.nucleus] }
	fn coda(&mut self)  -> &mut [Sung] { &mut self.phones[self.nucleus + 1..] }
}

fn total_ms(p: &[Sung]) -> f32 { p.iter().map(|s| s.ms).sum() }

fn scale(p: &mut [Sung], k: f32) {
	for s in p { s.ms *= k; }
}

// what a note, or a piece of a split one, sings
enum Part { Rest, Hold, Sing(Syllable) }

struct Piece {
	key:  f32,
	ms:   f32,
	part: Part,
}

// a note on the song clock
struct Tone {
	start: f64,		// ms
	end:   f64,
	key:   f32,
	sung:  bool,	// false for a rest
}

// a syllable, or a rest, over one or more tones
struct Span {
	tones: Range<usize>,
	syl:   Option<Syllable>,	// None for a rest
	ms:    f32,					// the vowel's length, or the pause's
}

// the lyric's syllables, from the dictionary or letter to sound
fn syllables(lyric: &str, dict: &mut dictreader) -> Vec<Syllable> {
	let mut tokens: Vec<u8> = Vec::new();
	let mut texts: Vec<String> = Vec::new();
	let mut wbuf = cbuf::new();
	for word in normalize(lyric).split_whitespace() {
		wbuf.write_text(word);
		Speaker::word_tokens(&mut wbuf, dict, &mut tokens, &mut texts);
	}
	let utt = Utterance::build(&tokens, &texts);
	let mut out = Vec::new();
	for syl in &utt.syls {
		let mut phones = Vec::new();
		for i in syl.segs.clone() {
			let s = &utt.segs[i];
			phones.push(Sung {
				phone:      s.phone,
				stressed:   s.stress != 0,
				word_start: utt.words[s.word].segs.start == i,
				ms:         phone_dur(s.phone).inherent * CONSONANT_SCALE,
			});
		}
		// a syllable with no vowel ("hmm") holds its last sound
		let nucleus = syl.nucleus.map_or(phones.len() - 1, |n| n - syl.segs.start);
		out.push(Syllable { phones, nucleus });
	}
	out
}

// what each note sings: a lyric's syllables go to its note and the "-" notes after it
fn pieces(notes: &[Note], dict: &mut dictreader) -> Vec<Piece> {
	let mut out = Vec::new();
	let mut i = 0;
	while i < notes.len() {
		let lyric = notes[i].lyric.trim();
		let run = 1 + notes[i + 1..].iter().take_while(|n| n.lyric.trim() == HOLD).count();
		let mut syls = if lyric.is_empty() || lyric == HOLD { Vec::new() } else { syllables(lyric, dict) };
		// syllables left over when the notes run out split the last note
		let last_split = syls.len().saturating_sub(run) + 1;
		syls.reverse();
		for (k, n) in notes[i..i + run].iter().enumerate() {
			let split = if k + 1 == run { last_split } else { 1 };
			for _ in 0..split {
				let part = match syls.pop() {
					Some(s) => Part::Sing(s),
					None if k == 0 && lyric.is_empty() => Part::Rest,
					None => Part::Hold,
				};
				out.push(Piece { key: n.key as f32, ms: n.ms.max(0.0) / split as f32, part });
			}
		}
		i += run;
	}
	out
}

// lay the pieces out from start: a tone each, gathered into spans
fn lay_out(pieces: Vec<Piece>, start: f64) -> (Vec<Tone>, Vec<Span>) {
	let mut tones: Vec<Tone> = Vec::new();
	let mut spans: Vec<Span> = Vec::new();
	let mut t = start;
	for p in pieces {
		let k = tones.len();
		tones.push(Tone { start: t, end: t + p.ms as f64, key: p.key, sung: false });
		t += p.ms as f64;
		match (p.part, spans.last_mut()) {
			(Part::Sing(s), _) => spans.push(Span { tones: k..k + 1, syl: Some(s), ms: 0.0 }),
			// a hold carries on whatever came before, and rests run together
			(Part::Hold, Some(last)) => last.tones.end = k + 1,
			(Part::Rest, Some(last)) if last.syl.is_none() => last.tones.end = k + 1,
			_ => spans.push(Span { tones: k..k + 1, syl: None, ms: 0.0 }),
		}
	}
	for s in &spans {
		for k in s.tones.clone() { tones[k].sung = s.syl.is_some(); }
	}
	(tones, spans)
}

// a sung span before a rest, or at the end, closes with the release
fn add_releases(spans: &mut [Span]) {
	for j in 0..spans.len() {
		let before_rest = spans.get(j + 1).is_none_or(|s| s.syl.is_none());
		if let Some(s) = spans[j].syl.as_mut() && before_rest {
			let last = s.phones[s.phones.len() - 1].phone;
			s.phones.push(Sung { phone: release_phone(last), stressed: false, word_start: false, ms: RELEASE_MS });
		}
	}
}

// fit the consonants that come out of each span's time, and give its vowel or pause the rest
fn fit(spans: &mut [Span], tones: &[Tone]) {
	for j in 0..spans.len() {
		let own = (tones[spans[j].tones.end - 1].end - tones[spans[j].tones.start].start) as f32;
		// its coda and the next onset; the very first onset has no time before it to come out of
		let mut taken = 0.0;
		if let Some(s) = spans[j].syl.as_mut() {
			taken += total_ms(s.coda());
			if j == 0 { taken += total_ms(s.onset()); }
		}
		if let Some(s) = spans.get_mut(j + 1).and_then(|s| s.syl.as_mut()) {
			taken += total_ms(s.onset());
		}
		let budget = MAX_CONSONANT_SHARE * own;
		let k = if taken > budget { budget / taken } else { 1.0 };
		if let Some(s) = spans[j].syl.as_mut() {
			scale(s.coda(), k);
			if j == 0 { scale(s.onset(), k); }
		}
		if let Some(s) = spans.get_mut(j + 1).and_then(|s| s.syl.as_mut()) {
			scale(s.onset(), k);
		}
		spans[j].ms = own - taken * k;
	}
}

fn ease(x: f32) -> f32 { (1.0 - libm::cosf(PI * x.clamp(0.0, 1.0))) / 2.0 }

// 0 at both ends, 1 in the middle, flat at the ends
fn bump(x: f32) -> f32 {
	if !(0.0..=1.0).contains(&x) { return 0.0; }
	let s = libm::sinf(PI * x);
	s * s
}

fn glide_ms(interval: f32, note_ms: f32) -> f32 {
	(GLIDE_MS + GLIDE_MS_PER_ST * interval.abs()).min(GLIDE_MAX_MS).min(GLIDE_MAX_SHARE * note_ms)
}

// the dip before a glide and the overshoot after it, in the glide's direction
fn bend(interval: f32) -> f32 { (BEND_SHARE * interval).clamp(-BEND_MAX_ST, BEND_MAX_ST) }

fn key_hz(key: f32) -> f32 { 440.0 * libm::exp2f((key - 69.0) / 12.0) }

// the pitch to give the synthesizer so the voice's pitch scaling lands on hz
fn voice_input_hz(hz: f32, v: &Voice) -> f32 {
	REF_HZ * libm::powf(hz / (REF_HZ * v.f0 / 100.0), 100.0 / v.range)
}

// the notes and the segments played, on the song clock
struct Melody {
	tones: Vec<Tone>,
	segs:  Vec<(usize, f64, f32)>,		// (stream index, start ms, ms) of every phone played
}

impl Melody {
	fn hz_at(&self, seg: usize, frac: f32, v: &Voice) -> f32 {
		let k = self.segs.partition_point(|s| s.0 < seg);
		let t = match self.segs.get(k) {
			Some(&(s, start, ms)) if s == seg => start + (frac * ms) as f64,
			_ => return REF_HZ,
		};
		match self.key_at(t) {
			Some(key) => voice_input_hz(key_hz(key).clamp(MIN_HZ, MAX_HZ), v),
			None => REF_HZ,
		}
	}

	// the tone whose pitch sounds at t: in a rest, the note after it (its consonants come ahead of the beat),
	// else the note before it
	fn tone_at(&self, t: f64) -> Option<usize> {
		let k = self.tones.partition_point(|n| n.start <= t).saturating_sub(1);
		let sung = |i: usize| self.tones.get(i).is_some_and(|n| n.sung);
		if sung(k) { return Some(k); }
		if sung(k + 1) { return Some(k + 1); }
		k.checked_sub(1).filter(|&i| sung(i))
	}

	// the pitch at t, as a MIDI key
	fn key_at(&self, t: f64) -> Option<f32> {
		let k = self.tone_at(t)?;
		let tone = &self.tones[k];
		let prev = k.checked_sub(1).map(|i| &self.tones[i]).filter(|p| p.sung);
		let next = self.tones.get(k + 1).filter(|n| n.sung);
		let age = (t - tone.start) as f32;		// below 0 for the consonants ahead of the beat
		let left = (tone.end - t) as f32;
		let mut key = tone.key;

		// INTO THE NOTE
		match prev {
			Some(p) => key += bend(tone.key - p.key) * bump(age / OVERSHOOT_MS),
			None => key -= SCOOP_ST * (1.0 - ease(age / SCOOP_MS)),
		}

		// OUT OF IT
		let mut sustain_left = left;
		match next {
			Some(n) => {
				let interval = n.key - tone.key;
				let g = glide_ms(interval, (tone.end - tone.start) as f32);
				key += interval * ease(1.0 - left / g.max(1.0));
				key -= bend(interval) * bump((left - g) / PREP_MS);
				sustain_left = left - g;
			},
			None => key -= FALL_ST * ease(1.0 - left / FALL_MS),
		}

		// VIBRATO
		let vib_age = age - VIBRATO_DELAY_MS;
		if vib_age > 0.0 && sustain_left > 0.0 {
			let depth = VIBRATO_ST * (vib_age / VIBRATO_RISE_MS).min(1.0) * (sustain_left / VIBRATO_FADE_MS).min(1.0);
			key += depth * libm::sinf(2.0 * PI * VIBRATO_HZ * vib_age / 1000.0);
		}
		Some(key)
	}

	// drop what is older than the previous passage: the synth is never more than a phone or two behind
	fn drop_before(&mut self, seg: usize, t: f64) {
		self.segs.drain(..self.segs.partition_point(|s| s.0 < seg));
		self.tones.drain(..self.tones.partition_point(|n| n.end < t));
	}
}

pub struct Singer {
	pub synth: _synth::engine,
	pub trace: bool,		// send each phone's plan to SpeechSink::trace
	melody:    Melody,
	dict:      dictreader,
	clock:     f64,			// ms of song sung so far
	played:    i64,			// samples given to the synthesizer, less what it left out
	next_seg:  usize,		// stream index for the next phone
	prev_seg:  usize,		// stream index and clock where the previous passage starts
	prev_ms:   f64,
	f0:        f32,			// the pitch the synthesizer read at its last frame
}

impl Singer {
	pub fn new() -> Singer {
		let mut synth = _synth::engine::new();
		synth.vowel_glide_ms = Some(DIPHTHONG_GLIDE_MS);
		synth.vowel_bw_scale = VOWEL_BW_SCALE;
		synth.vowel_min_b2 = VOWEL_MIN_B2;
		// a sung line runs legato from vowel to vowel
		synth.word_catch = false;
		Singer {
			synth,
			trace:    false,
			melody:   Melody { tones: Vec::new(), segs: Vec::new() },
			dict:     dictreader::new(),
			clock:    0.0,
			played:   0,
			next_seg: 0,
			prev_seg: 0,
			prev_ms:  0.0,
			f0:       REF_HZ,
		}
	}

	// sing a passage of notes; finish plays out the end
	pub async fn sing<O: SpeechSink>(&mut self, notes: &[Note<'_>], out: &mut O) -> Result<(), O::Error> {
		self.melody.drop_before(self.prev_seg, self.prev_ms);
		(self.prev_seg, self.prev_ms) = (self.next_seg, self.clock);

		let pieces = pieces(notes, &mut self.dict);
		let (tones, mut spans) = lay_out(pieces, self.clock);
		add_releases(&mut spans);
		fit(&mut spans, &tones);
		// a passage ends in a rest, so the next one starts fresh
		let end = tones.last().map_or(self.clock, |n| n.end);
		self.melody.tones.extend(tones);
		self.melody.tones.push(Tone { start: end, end, key: 0.0, sung: false });

		// a vowel or pause ends on the clock; the consonants keep their own length
		for span in spans {
			let Some(syl) = span.syl else {
				self.clock += span.ms as f64;
				let dur_ms = self.ms_until_clock();
				self.play(out, PAU, dur_ms, false, true).await?;
				continue;
			};
			for (i, p) in syl.phones.iter().enumerate() {
				let ms = if i == syl.nucleus { span.ms } else { p.ms };
				self.clock += ms as f64;
				let dur_ms = if i == syl.nucleus { self.ms_until_clock() } else { libm::roundf(ms) as i32 };
				// every sung vowel carries a note, so none is unstressed ("i" is the pronoun to the dictionary)
				let stressed = p.stressed || i == syl.nucleus;
				self.play(out, p.phone, dur_ms, stressed, p.word_start).await?;
			}
		}
		Ok(())
	}

	// play out what is still in the synthesizer
	pub async fn finish<O: SpeechSink>(&mut self, out: &mut O) -> Result<(), O::Error> {
		let mut first = true;
		while self.synth.is_not_last() {
			self.clock += FLUSH_MS as f64;
			self.play(out, if first { PAU } else { SIL }, FLUSH_MS as i32, false, true).await?;
			first = false;
		}
		Ok(())
	}

	// whole ms that bring the playback up to the clock: what rounding and the synthesizer's left-out samples cost
	fn ms_until_clock(&mut self) -> i32 {
		self.played -= self.synth.take_lost_samples() as i64;
		let behind = self.clock * SAMPLES_PER_MS - self.played as f64;
		libm::round(behind / SAMPLES_PER_MS) as i32
	}

	async fn play<O: SpeechSink>(
		&mut self, out: &mut O, phone: u8, dur_ms: i32, stressed: bool, word_start: bool
	) -> Result<(), O::Error> {
		if dur_ms <= 0 { return Ok(()); }
		let seg = self.next_seg;
		self.next_seg += 1;
		let start = self.played as f64 / SAMPLES_PER_MS;
		self.melody.segs.push((seg, start, dur_ms as f32));
		if self.trace {
			out.trace(&format!("TRACE {} {} seg={} dur={} at={:.0}", phone, stressed as u8, seg, dur_ms, start));
		}
		// the synthesizer's own ms to samples
		self.played += (dur_ms as f32 * m_sr as f32 / 1000.0) as i64;
		if self.synth.load(phone, dur_ms, stressed, seg, word_start) {
			let v = self.synth.voice();
			let melody = &self.melody;
			run_synth(&mut self.synth, out, &mut self.f0, |s, f| melody.hz_at(s, f, &v)).await?;
		}
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn note(key: u8, ms: f32, lyric: &str) -> Note<'_> { Note { key, ms, lyric } }

	#[test]
	fn a_word_with_more_syllables_than_notes_splits_the_last_note() {
		let p = pieces(&[note(60, 600.0, "happy")], &mut dictreader::new());
		assert_eq!(p.len(), 2);
		for x in &p {
			assert!(matches!(x.part, Part::Sing(_)) && x.ms == 300.0);
		}
	}

	#[test]
	fn a_hold_past_the_last_syllable_carries_the_vowel() {
		let p = pieces(&[note(60, 300.0, "go"), note(62, 300.0, "-"), note(64, 300.0, "")], &mut dictreader::new());
		assert!(matches!(p[0].part, Part::Sing(_)));
		assert!(matches!(p[1].part, Part::Hold));
		assert!(matches!(p[2].part, Part::Rest));
		let (tones, spans) = lay_out(p, 0.0);
		assert_eq!(spans[0].tones, 0..2, "the held note is part of the syllable");
		assert!(tones[1].sung && !tones[2].sung);
	}

	#[test]
	fn consonants_never_take_the_whole_note() {
		let p = pieces(&[note(60, 100.0, "strengths"), note(60, 100.0, "")], &mut dictreader::new());
		let (tones, mut spans) = lay_out(p, 0.0);
		add_releases(&mut spans);
		fit(&mut spans, &tones);
		assert!(spans[0].ms >= 100.0 * (1.0 - MAX_CONSONANT_SHARE) - 0.01, "vowel {} ms", spans[0].ms);
	}
}
