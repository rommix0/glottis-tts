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

rapped (Style::Rap), the timing is the same but the pitch is spoken: the
speech contour's, laid over the sung lengths, a phrase between rests.

humanized (Singer::humanize, 0 for none), each syllable lands a little early
or late and each note a little sharp or flat, the pitch wanders slowly and
the vibrato varies; the same seed always gives the same take.

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
use crate::_utt::{Utterance, Seg, Syl, Word, Phrase, Boundary, Kind, is_vowel};
use crate::_dur::{phone_dur, RELEASE_MS};
use crate::_f0::{self, REF_HZ};
use crate::_phone::*;

// a lyric that carries on the word before
const HOLD: &str = "-";

// timing
const CONSONANT_SCALE:     f32 = 0.85;		// a sung consonant against its inherent speech length
const MAX_CONSONANT_SHARE: f32 = 0.85;		// of the note or rest the consonants come out of
const DIPHTHONG_GLIDE_MS:  f32 = 150.0;		// a held vowel keeps its first target until this long before its end
const VOWEL_BW_SCALE:      f32 = 1.3;		// a held vowel's still formants ring on a harmonic unless wider
const VOWEL_MIN_B2:        f32 = 130.0;		// ... and a narrower F2 rings on its harmonic all the same (ao)
const F1_OVER_F0:          f32 = 1.1;		// F1 kept over the pitch on a high note, as a singer opens up
const EASE_FROM_HZ:        f32 = 262.0;		// above middle C the voicing eases off this much an octave ...
const EASE_DB_PER_OCT:     f32 = 5.0;		// ... or high notes, louder with their pitch, run into the soft clip
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
const FALL_MAX_SHARE:   f32 = 0.3;			// of a short note: its fall shorter and shallower, or it droops
const VIBRATO_HZ:       f32 = 5.5;
const VIBRATO_ST:       f32 = 0.35;			// peak deviation
const VIBRATO_DELAY_MS: f32 = 300.0;		// into the note
const VIBRATO_RISE_MS:  f32 = 400.0;
const VIBRATO_FADE_MS:  f32 = 150.0;		// before the note, or the glide out of it, ends
const RAP_ACCENT_MS:    f32 = 100.0;		// rapped, an accent peaks this soon into its vowel at the latest, as spoken
// humanizing, at humanize 1
const HUMAN_ONSET_MS:   f32 = 20.0;			// a syllable lands up to this early or late ...
const HUMAN_ONSET_ROOM: f32 = 0.25;			// ... but takes at most this share of the note or rest beside it
const HUMAN_DETUNE_ST:  f32 = 0.1;			// a note lands up to this sharp or flat
const HUMAN_WANDER_ST:  f32 = 0.08;			// the pitch wanders up to this far ...
const HUMAN_WANDER_MS:  f32 = 300.0;		// ... in a new direction this often
const HUMAN_VIBRATO:    f32 = 0.2;			// a note's vibrato is up to this share deeper or shallower, a third of it faster
// what each kind of variation draws from the seed
const ONSET_DRAW:   u32 = 1;
const DETUNE_DRAW:  u32 = 2;
const VIBRATO_DRAW: u32 = 3;
const WANDER_DRAW:  u32 = 4;
const MIN_HZ:           f32 = 40.0;
const MAX_HZ:           f32 = 1600.0;

#[derive(Clone, Copy, PartialEq)]
pub enum Style {
	Sing,		// each vowel held for its note, on the note's pitch
	Rap,		// the same timing, with the pitch of speech
}

pub struct Note<'a> {
	pub key:   u8,			// MIDI key: 60 is middle C, 69 is A4 at 440 Hz
	pub ms:    f32,			// length
	pub lyric: &'a str,		// a word, "-" for the next syllable or a held vowel, "" for a rest
}

// one phone to sing
struct Sung {
	phone:      u8,
	stress:     u8,			// 0, 1 primary, 2 secondary
	word_start: bool,
	ms:         f32,
}

// a syllable's phones and which of them is its vowel
struct Syllable {
	phones:   Vec<Sung>,
	nucleus:  usize,
	function: bool,		// of a function word, which takes no accent
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
	key:   f32,		// with its humanized detuning
	sung:  bool,	// false for a rest
	vib:   f32,		// humanized vibrato: depth this share more, rate a third of it
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
				stress:     s.stress,
				word_start: utt.words[s.word].segs.start == i,
				ms:         phone_dur(s.phone).inherent * CONSONANT_SCALE,
			});
		}
		// a syllable with no vowel ("hmm") holds its last sound
		let nucleus = syl.nucleus.map_or(phones.len() - 1, |n| n - syl.segs.start);
		out.push(Syllable { phones, nucleus, function: utt.words[syl.word].function });
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
		tones.push(Tone { start: t, end: t + p.ms as f64, key: p.key, sung: false, vib: 0.0 });
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
			s.phones.push(Sung { phone: release_phone(last), stress: 0, word_start: false, ms: RELEASE_MS });
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

// how far a note before a rest has fallen with left ms to go
fn fall(left: f32, note_ms: f32) -> f32 {
	let ms = FALL_MS.min(FALL_MAX_SHARE * note_ms).max(1.0);
	FALL_ST * ms / FALL_MS * ease(1.0 - left / ms)
}

fn key_hz(key: f32) -> f32 { 440.0 * libm::exp2f((key - 69.0) / 12.0) }

// the pitch to give the synthesizer so the voice's pitch scaling lands on hz
fn voice_input_hz(hz: f32, v: &Voice) -> f32 {
	REF_HZ * libm::powf(hz / (REF_HZ * v.f0 / 100.0), 100.0 / v.range)
}

// a repeatable random number in -1..1 for draw i of a kind of variation
fn noise(seed: u32, kind: u32, i: u64) -> f32 {
	let mut x = i ^ ((seed as u64) << 32) ^ ((kind as u64) << 56);
	x = x.wrapping_mul(0x9E37_79B9_7F4A_7C15);
	x ^= x >> 29;
	x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
	x ^= x >> 32;
	(x >> 40) as f32 / (1u64 << 23) as f32 - 1.0
}

// each sung note's detuning and vibrato, drawn by where it falls in the song
fn humanize_tones(tones: &mut [Tone], amount: f32, seed: u32) {
	for t in tones.iter_mut().filter(|t| t.sung) {
		let at = libm::round(t.start) as u64;
		t.key += HUMAN_DETUNE_ST * amount * noise(seed, DETUNE_DRAW, at);
		t.vib = HUMAN_VIBRATO * amount * noise(seed, VIBRATO_DRAW, at);
	}
}

// each syllable after the first lands a little early or late, out of the note or rest beside it;
// the passage's start and end stay on the beat, so a long song never drifts
fn humanize_onsets(tones: &mut [Tone], spans: &[Span], amount: f32, seed: u32) {
	for j in 1..spans.len() {
		if spans[j].syl.is_none() { continue; }
		let (before, k) = (&spans[j - 1].tones, spans[j].tones.start);
		let len = |r: &Range<usize>| (tones[r.end - 1].end - tones[r.start].start) as f32;
		let room = HUMAN_ONSET_ROOM * len(before).min(len(&spans[j].tones));
		let shift = (HUMAN_ONSET_MS * amount * noise(seed, ONSET_DRAW, libm::round(tones[k].start) as u64))
			.clamp(-room, room) as f64;
		tones[k].start += shift;
		tones[k - 1].end += shift;
	}
}

// rapped, the passage as an utterance for the speech contour: its words, syllables and stress at the sung
// lengths, a phrase between rests; and the stream index each segment plays as, counting from first_seg
fn rap_utterance(spans: &[Span], first_seg: usize) -> (Utterance, Vec<usize>) {
	let mut utt = Utterance {
		segs: Vec::new(), syls: Vec::new(), words: Vec::new(), phrases: Vec::new(), kind: Kind::Statement,
	};
	let mut stream: Vec<usize> = Vec::new();
	let mut seg = first_seg;
	let mut phrase_start = 0;		// the first word of the phrase being built
	for span in spans {
		let Some(syl) = &span.syl else {
			// the rest's pause plays as a segment of its own, outside the utterance
			seg += 1;
			end_phrase(&mut utt, &mut phrase_start, Boundary::Comma);
			continue;
		};
		let new_word = syl.phones[0].word_start || utt.words.len() == phrase_start;
		if new_word {
			let (n, s) = (utt.segs.len(), utt.syls.len());
			let phrase = utt.phrases.len();
			utt.words.push(Word { segs: n..n, syls: s..s, function: syl.function, text: String::new(), phrase });
		}
		let w = utt.words.len() - 1;
		let first = utt.segs.len();
		for (i, p) in syl.phones.iter().enumerate() {
			seg += 1;
			// the release is the voice dying away after the phrase, as in speech
			if matches!(p.phone, AXP | IXP) { continue; }
			let dur_ms = if i == syl.nucleus { span.ms } else { p.ms };
			let s = utt.syls.len();
			utt.segs.push(Seg { phone: p.phone, stress: p.stress, word: w, syl: s, dur_ms, rules: Vec::new(), glide: false });
			stream.push(seg - 1);
		}
		let nucleus = Some(first + syl.nucleus).filter(|&n| is_vowel(utt.segs[n].phone));
		let stress = syl.phones[syl.nucleus].stress;
		utt.syls.push(Syl { segs: first..utt.segs.len(), nucleus, stress, word: w });
		utt.words[w].segs.end = utt.segs.len();
		utt.words[w].syls.end = utt.syls.len();
	}
	end_phrase(&mut utt, &mut phrase_start, Boundary::Period);
	// the passage's last phrase ends it like a period, after a rest too
	if let Some(last) = utt.phrases.last_mut() {
		last.end = Boundary::Period;
	}
	(utt, stream)
}

// close the phrase begun at word phrase_start, if it has any words
fn end_phrase(utt: &mut Utterance, phrase_start: &mut usize, end: Boundary) {
	if utt.words.len() > *phrase_start {
		utt.phrases.push(Phrase { words: *phrase_start..utt.words.len(), end });
		*phrase_start = utt.words.len();
	}
}

// the notes and the segments played, on the song clock
struct Melody {
	tones: Vec<Tone>,
	segs:  Vec<(usize, f64, f32)>,		// (stream index, start ms, ms) of every phone played
	wander_st: f32,						// how far the pitch wanders (0: not at all)
	seed:      u32,
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
		let mut key = tone.key + self.wander(t);

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
			None => key -= fall(left, (tone.end - tone.start) as f32),
		}

		// VIBRATO
		let vib_age = age - VIBRATO_DELAY_MS;
		if vib_age > 0.0 && sustain_left > 0.0 {
			let shape = (vib_age / VIBRATO_RISE_MS).min(1.0) * (sustain_left / VIBRATO_FADE_MS).min(1.0);
			let (depth, hz) = (VIBRATO_ST * (1.0 + tone.vib) * shape, VIBRATO_HZ * (1.0 + tone.vib / 3.0));
			key += depth * libm::sinf(2.0 * PI * hz * vib_age / 1000.0);
		}
		Some(key)
	}

	// the slow wander at t: a new random height every HUMAN_WANDER_MS, eased between
	fn wander(&self, t: f64) -> f32 {
		if self.wander_st == 0.0 { return 0.0; }
		let x = t.max(0.0) / HUMAN_WANDER_MS as f64;
		let k = x as u64;
		let (a, b) = (noise(self.seed, WANDER_DRAW, k), noise(self.seed, WANDER_DRAW, k + 1));
		self.wander_st * (a + (b - a) * ease((x - k as f64) as f32))
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
	pub style: Style,
	pub humanize: f32,		// how human the timing and pitch drift: 0 exact as written, 1 natural
	pub seed:  u32,			// the take: the same seed always drifts the same way
	melody:    Melody,
	contour:   _f0::Contour,	// the pitch when rapped
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
		synth.set_f1_over_f0(F1_OVER_F0);
		synth.set_high_note_ease(EASE_FROM_HZ, EASE_DB_PER_OCT);
		// a sung line runs legato from vowel to vowel
		synth.word_catch = false;
		let mut contour = _f0::Contour::new();
		contour.accent_max_ms = RAP_ACCENT_MS;
		Singer {
			synth,
			trace:    false,
			style:    Style::Sing,
			humanize: 1.0,
			seed:     1,
			melody:   Melody { tones: Vec::new(), segs: Vec::new(), wander_st: 0.0, seed: 1 },
			contour,
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
		let (mut tones, mut spans) = lay_out(pieces, self.clock);
		humanize_tones(&mut tones, self.humanize, self.seed);
		humanize_onsets(&mut tones, &spans, self.humanize, self.seed);
		(self.melody.wander_st, self.melody.seed) = (HUMAN_WANDER_ST * self.humanize, self.seed);
		add_releases(&mut spans);
		fit(&mut spans, &tones);
		if self.style == Style::Rap {
			let (utt, stream) = rap_utterance(&spans, self.next_seg);
			self.contour.add_utterance(&utt, &stream);
		}
		// a passage ends in a rest, so the next one starts fresh
		let end = tones.last().map_or(self.clock, |n| n.end);
		self.melody.tones.extend(tones);
		self.melody.tones.push(Tone { start: end, end, key: 0.0, sung: false, vib: 0.0 });

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
				let stressed = p.stress != 0 || i == syl.nucleus;
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
		// every phone takes a stream index, played or not, so rap_utterance can count them out ahead
		let seg = self.next_seg;
		self.next_seg += 1;
		if dur_ms <= 0 { return Ok(()); }
		let start = self.played as f64 / SAMPLES_PER_MS;
		self.melody.segs.push((seg, start, dur_ms as f32));
		if self.trace {
			out.trace(&format!("TRACE {} {} seg={} dur={} at={:.0}", phone, stressed as u8, seg, dur_ms, start));
		}
		// the synthesizer's own ms to samples
		self.played += (dur_ms as f32 * m_sr as f32 / 1000.0) as i64;
		if self.synth.load(phone, dur_ms, stressed, seg, word_start) {
			let v = self.synth.voice();
			let (melody, contour) = (&self.melody, &mut self.contour);
			match self.style {
				Style::Sing => run_synth(&mut self.synth, out, &mut self.f0, |s, f| melody.hz_at(s, f, &v)).await?,
				Style::Rap  => run_synth(&mut self.synth, out, &mut self.f0, |s, f| contour.hz_at(s, f)).await?,
			}
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
	fn a_rapped_passage_is_spoken_with_early_accents() {
		// rest, farmer (over two notes), dog, rest: one phrase of two content words, ending in a fall,
		// each accent peaking within RAP_ACCENT_MS of its long vowel's start
		let notes = [note(0, 300.0, ""), note(55, 1000.0, "farmer"), note(57, 500.0, "-"), note(60, 1000.0, "dog"),
			note(0, 300.0, "")];
		let (tones, mut spans) = lay_out(pieces(&notes, &mut dictreader::new()), 0.0);
		add_releases(&mut spans);
		fit(&mut spans, &tones);
		let (utt, stream) = rap_utterance(&spans, 0);
		assert_eq!(utt.words.len(), 2);
		assert_eq!(utt.phrases.len(), 1);
		assert!(utt.phrases[0].end == Boundary::Period);
		assert!(stream.len() == utt.segs.len() && stream.windows(2).all(|w| w[0] < w[1]));
		let mut contour = _f0::Contour::new();
		contour.accent_max_ms = RAP_ACCENT_MS;
		contour.add_utterance(&utt, &stream);
		let mut accents = 0;
		for (i, seg) in utt.segs.iter().enumerate() {
			for t in contour.targets_of(stream[i]).filter(|t| t.kind == _f0::TargetKind::Accent) {
				assert!(t.frac * seg.dur_ms <= RAP_ACCENT_MS + 0.01, "peak {} ms in", t.frac * seg.dur_ms);
				accents += 1;
			}
		}
		assert_eq!(accents, 2);
	}

	#[test]
	fn a_short_note_falls_less_before_a_rest() {
		// a long note falls FALL_ST over its last FALL_MS; one of 250 ms over its last 30%, as far in proportion
		let at = |note_ms: f64, left: f64| {
			let tones = vec![Tone { start: 0.0, end: note_ms, key: 60.0, sung: true, vib: 0.0 },
				Tone { start: note_ms, end: note_ms + 300.0, key: 60.0, sung: false, vib: 0.0 }];
			60.0 - (Melody { tones, segs: Vec::new(), wander_st: 0.0, seed: 0 }).key_at(note_ms - left).unwrap()
		};
		assert!((at(1000.0, 0.0) - FALL_ST).abs() < 0.01, "falls {} st", at(1000.0, 0.0));
		assert!((at(250.0, 0.0) - FALL_ST * 0.5).abs() < 0.01, "falls {} st", at(250.0, 0.0));
		assert!(at(250.0, 76.0).abs() < 0.01, "already {} st down", at(250.0, 76.0));
	}

	// a passage laid out: rest, sat, on, mats, a long hum, rest
	fn laid_out() -> (Vec<Tone>, Vec<Span>) {
		let notes = [note(0, 300.0, ""), note(55, 600.0, "sat"), note(57, 400.0, "on"), note(59, 800.0, "mats"),
			note(60, 900.0, "hum"), note(0, 300.0, "")];
		lay_out(pieces(&notes, &mut dictreader::new()), 0.0)
	}

	#[test]
	fn humanized_syllables_land_near_their_beat() {
		let (mut tones, spans) = laid_out();
		let grid: Vec<(f64, f64)> = tones.iter().map(|t| (t.start, t.end)).collect();
		humanize_onsets(&mut tones, &spans, 1.0, 7);
		let mut moved = 0;
		for (t, &(start, end)) in tones.iter().zip(&grid) {
			assert!((t.start - start).abs() <= HUMAN_ONSET_MS as f64 && t.end > t.start, "{} for {}", t.start, start);
			assert!((t.end - end).abs() <= HUMAN_ONSET_MS as f64);
			if t.start != start { moved += 1; }
		}
		assert!(moved > 0, "nothing moved");
		// the passage starts and ends on the beat
		assert_eq!((tones[0].start, tones[tones.len() - 1].end), (grid[0].0, grid[grid.len() - 1].1));
	}

	#[test]
	fn a_humanized_note_lands_near_its_key() {
		let (mut tones, _) = laid_out();
		let keys: Vec<f32> = tones.iter().map(|t| t.key).collect();
		humanize_tones(&mut tones, 1.0, 7);
		let off: Vec<f32> = tones.iter().zip(&keys).map(|(t, k)| t.key - k).collect();
		assert!(off.iter().all(|d| d.abs() <= HUMAN_DETUNE_ST) && off.iter().any(|d| *d != 0.0), "{:?}", off);
		// and wanders no further than HUMAN_WANDER_ST
		let m = Melody { tones, segs: Vec::new(), wander_st: HUMAN_WANDER_ST, seed: 7 };
		let w: Vec<f32> = (0..300).map(|i| m.wander(i as f64 * 10.0)).collect();
		assert!(w.iter().all(|x| x.abs() <= HUMAN_WANDER_ST) && w.iter().any(|x| x.abs() > HUMAN_WANDER_ST / 4.0));
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
