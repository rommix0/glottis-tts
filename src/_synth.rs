/*---------------------------------------
| KLATT-STYLE FORMANT SYNTHESIZER       |
|                                       |
| AUTHOR: ANTHONY C. BARTMAN (@ROMMIX0) |
| DATE:   8/11/2026-11/10/2026          |
---------------------------------------*/

use crate::_vtm::{vtm, Voice, DB_GAIN, DB_PER_HALF, G_MAX};
use crate::_vtm::m_sr;
use crate::_vtbl::formant_tbl;
use crate::_utt::PAU;

// pre-calculated millisecond constants for slopes
const two_ms:     i32 = (m_sr as f32 * 0.002) as i32;
const five_ms:    i32 = (m_sr as f32 * 0.005) as i32;
const ten_ms:     i32 = (m_sr as f32 * 0.010) as i32;
const twenty_ms:  i32 = (m_sr as f32 * 0.020) as i32;
const thirty_ms:  i32 = (m_sr as f32 * 0.030) as i32;
const forty_ms:   i32 = (m_sr as f32 * 0.040) as i32;
const fifty_ms:   i32 = (m_sr as f32 * 0.050) as i32;
const sixty_ms:   i32 = (m_sr as f32 * 0.060) as i32;
const fifteen_ms: i32 = (m_sr as f32 * 0.015) as i32;
const eighty_ms:  i32 = (m_sr as f32 * 0.080) as i32;

// per-sample fader steps
const optime:    f32 = 1.0 / two_ms   as f32;		// master fader
const glot_time: f32 = 1.0 / ten_ms   as f32;		// glottalizer
const end_time:  f32 = 1.0 / fifty_ms as f32;		// phrase-final

// shortest plosive closure before the burst
const min_closure: i32 = thirty_ms;

// plosive release by place (labial, alveolar, velar)
// the burst falls BURST_DECAY_DB over its length
const BURST_MS: [[f32; 3]; 2] = [[10.0, 15.0, 25.0], [10.0, 10.0, 20.0]];		// voiceless, voiced
const ASP_MS: [f32; 3] = [50.0, 55.0, 50.0];
const BURST_DECAY_DB: f32 = 12.0;

// affricate frication after the closure (voiceless, voiced), and its fall
const AFFRIC_MS: [f32; 2] = [55.0, 40.0];
const AFFRIC_DECAY_DB: f32 = 6.0;

// plosive burst trims (not affricates): overall, before an unstressed vowel, next to a sibilant
const BURST_GAIN_DB: f32 = -4.0;
const BURST_UNSTRESSED_DB: f32 = -6.0;
const BURST_SIBILANT_DB: f32 = 0.0;

// share of each formant move spent easing in and out: one starting or stopping at full speed clicks
const TRACK_EASE: f32 = 0.1;

// a move cut off by the next one hands its speed on, fading over this long, instead of stopping dead
const CARRY_MS: f32 = 10.0;

// a burst into a vowel or sonorant duller than an open vowel is cut by this share of 40 log10(F1 * F2 / RELEASE_REF)
const RELEASE_FOLLOW: f32 = 0.5;
const RELEASE_REF: f32 = 600.0 * 1500.0;

// aspiration takes the larger of the front-vowel cut and this share of the same difference
const ASP_FOLLOW: f32 = 0.5;

// lip release (AH for p, b): noise in front of the tract, so F2 and F3 stay under these, eased over LIP_RAMP_MS
const LABIAL_BURST_AH: [f32; 2] = [27.0, 28.0];
const LABIAL_BURST_F2: f32 = 1100.0;
const LABIAL_BURST_F3: f32 = 2300.0;
const LABIAL_BURST_WIDEN: f32 = 800.0;		// B2 and B3 added, so the release has no compact peak (a k cue)
const LIP_RAMP_MS: f32 = 5.0;

// an unreleased stop plays this share of its duration, as closure merging with the next one
const UNRELEASED_SHARE: f32 = 0.5;

// fricative crossfades with a vowel or sonorant, centred on the boundary: out of the fricative, then into it
const SIB_RAMP_MS: f32 = 40.0;
const FRIC_RAMP_MS: f32 = 40.0;
const SIB_ENTRY_MS: f32 = 60.0;
const FRIC_ENTRY_MS: f32 = 70.0;

// fricative noise rises and falls over this long next to silence or the phrase-final release
const FRIC_SILENCE_MS: f32 = 70.0;

// two different fricatives in a row dip at the switch, so they don't run together into one hiss
const FRIC_DIP_DB: f32 = 10.0;
const FRIC_DIP_MS: f32 = 30.0;

// a word-initial vowel after a vowel starts with a glottal catch: a voice dip centred on the join, lighter when
// unstressed ("Nina and"); an unstressed one after an r-coloured vowel is linked by the r instead ("letter and")
const GLOTTAL_DIP_DB: f32 = 24.0;
const GLOTTAL_DIP_UNSTRESSED_DB: f32 = 12.0;
const GLOTTAL_DIP_MS: f32 = 40.0;

// two different nasals in a row dip at the switch, so they don't run together into one hum ("thin men")
const NASAL_DIP_DB: f32 = 4.0;
const NASAL_DIP_MS: f32 = 30.0;

// nasal into a voiceless fricative: the velum closes before the tongue lets go, a silent gap before the noise, which
// then rises over NASAL_GAP_RISE_MS
const NASAL_GAP_MS: f32 = 25.0;
const NASAL_GAP_RISE_MS: f32 = 20.0;

// a nasal after an unstressed vowel is quieter, so it stays under that weaker vowel ("-ing", "-en"); not one that
// starts a word ("the moon")
const NASAL_AFTER_UNSTRESSED_DB: f32 = 6.0;

// a nasal's voicing and nasal coupling switch within this long at its closure and release
const NASAL_EDGE_MS: f32 = 15.0;

// tongue consonant into a back vowel: the vowel reaches its formants this long after the release
const BACK_TRANSITION_MS: f32 = 80.0;

// a vowel's F3 drops this much at its end before an r
const RHOTIC_F3: f32 = 200.0;

// where a vowel sits before an r: phone, then F1 and F2 at its start and at its end
const BEFORE_R: [(u8, f32, f32, f32, f32); 6] = [
	(13, 420.0, 2060.0, 470.0, 1900.0),	// eh
	(19, 300.0, 2400.0, 330.0, 2250.0),	// ih: near ee ("near", "here")
	(1,  706.0, 1250.0, 716.0, 1300.0),	// aa: only a little closer than its own place
	(4,  548.0,  808.0, 545.0,  808.0),	// ao
	(38, 310.0,  930.0, 370.0,  890.0),	// uh
	(39, 310.0,  930.0, 370.0,  890.0),	// uw
];

// breath noise, and the extra breath and Rd as the voice dies at a phrase end, reached BREATH_RAMP_MS into the release
const BREATH_DB: f32 = 6.0;
const BREATH_END_DB: f32 = 12.0;
const BREATH_END_RD: f32 = 0.3;
const BREATH_RAMP_MS: f32 = 60.0;

// longest move out of a vowel into the next phone
const VOWEL_EXIT_MS: f32 = 50.0;

// out of a vowel an r reaches its formants within this long
const R_ONSET_MS: f32 = 50.0;

// while F2 glides, F1 still opens into a back vowel within this long
const BACK_F1_MS: f32 = 40.0;

// a back vowel whose own targets move this much (F1 or F2) starts that glide on time, not after the slow F2
const GLIDE_HZ: f32 = 100.0;

// a consonant's F1 follows the next vowel by this share of its F1 off F1_FOLLOW_REF, never under F1_FOLLOW_MIN
const F1_FOLLOW: f32 = 0.5;
const F1_FOLLOW_REF: f32 = 500.0;
const F1_FOLLOW_MIN: f32 = 180.0;

// front vowels pull an alveolar's F2 locus up but back vowels don't pull it down: a lower vowel F2 counts as this
const ALVEOLAR_F2_FLOOR: f32 = 1500.0;

// a fricative's voicing switches within this long at a vowel or sonorant, though its noise crossfades longer
const VOICE_SWITCH_MS: f32 = 10.0;

// the vowel after a nasal reaches its formants this long after the release
const NASAL_RELEASE_MS: f32 = 30.0;

// a vowel's nasal coupling next to a nasal: up to NASAL_VOWEL_K, over NASAL_ANTIC_MS before it and NASAL_CARRY_MS after
const NASAL_VOWEL_K: f32 = 0.2;
const NASAL_ANTIC_MS: f32 = 80.0;
const NASAL_CARRY_MS: f32 = 40.0;

// aspiration (AH) and its share before an unstressed vowel or a sonorant (none after a same-word sibilant)
const ASP_DB: f32 = 15.5;
const ASP_UNSTRESSED: f32 = 0.6;
const ASP_BEFORE_SONORANT: f32 = 0.8;

// a word-internal t before an unstressed schwa ("button", "Clinton") is softer still: this share of that aspiration,
// this much quieter
const SCHWA_T_ASP: f32 = 0.5;
const SCHWA_T_ASP_DB: f32 = 6.0;

// after aspiration the voicing builds up over VOICE_ONSET_MS while the aspiration fades out under it
const VOICE_ONSET_MS: f32 = 10.0;
const ASP_OVERLAP_MS: f32 = 20.0;

// an l or r after an aspirated p, t or k in its word starts voiceless:
// its voice comes in, and the aspiration fades, over this share of it
const LIQUID_DEVOICE: f32 = 0.8;

// aspiration is loud against a close front vowel: cut per kHz of the vowel's F2-F1 gap above ASP_FRONT_FROM_HZ
const ASP_FRONT_DB_PER_KHZ: f32 = 6.5;
const ASP_FRONT_FROM_HZ: f32 = 700.0;
fn asp_front_cut(f1: f32, f2: f32) -> f32 { ASP_FRONT_DB_PER_KHZ * ((f2 - f1) - ASP_FRONT_FROM_HZ).max(0.0) / 1000.0 }

// h: breath level (AH) before a vowel, cut like aspiration before front vowels, crossfading with the vowel's voicing
const H_DB: f32 = 12.0;
const H_EDGE_MS: f32 = 20.0;

// the open glottis damps F1 and F2 during aspiration: bandwidth added, fading out with it
const ASP_B1_WIDEN: f32 = 400.0;
const ASP_B2_WIDEN: f32 = 70.0;

// voice bar in a voiced closure: it fades over the closure, muffled by Rd rising over its first 10 ms
const VOICE_BAR_FADE_DB: f32 = 6.0;
const VOICE_BAR_RD: f32 = 1.0;

// longest share of a vowel the transition into it may take
const ramp_share: f32 = 0.6;

// vowel reduction: how far a vowel of d ms gets from neutral toward its target (0..1)
const tau_vwl: f32 = 60.0;
fn reduce_k(d_ms: f32) -> f32 { 1.0 - libm::expf(-d_ms / tau_vwl) }

// velar F3 next to a vowel: (slope, offset) of F3 = offset + slope * the vowel's F2 (near F2 by front vowels)
const VELAR_F3: (f32, f32) = (0.45, 1700.0);
// k and g next to a high front vowel: F2 at least this far above the vowel's
const VELAR_F2_ABOVE: f32 = 150.0;
// ng's F3 comes down to just above its F2 (the velar pinch that sets it apart from n)
const NG_PINCH_HZ: f32 = 300.0;

// consonant F2 next to a vowel, by place: (slope, offset, floor) of F2 = offset + slope * max(the vowel's F2, floor)
fn locus(x: u8) -> Option<(f32, f32, f32)> {
	match x {
		9 | 31 | 26                 => Some((0.9, 0.0, 0.0)),					// bilabial: b p m
		16 | 40                     => Some((0.8, 300.0, 0.0)),					// labiodental: f v
		37 | 12                     => Some((0.5, 900.0, 0.0)),					// dental: th dh
		11 | 36 | 27 | 34 | 43 | 47 => Some((0.6, 800.0, ALVEOLAR_F2_FLOOR)),	// alveolar: d t n s z dx
		35 | 44 | 10 | 22           => Some((0.3, 1500.0, 0.0)),				// postalveolar: sh zh ch jh
		17 | 23                     => Some((0.5, 1100.0, 0.0)),				// velar: g k
		28                          => Some((0.9, 400.0, 0.0)),					// ng: near the vowel's own F2
		_                           => None,
	}
}

fn db_gain(db: f32) -> f32 {
	// an amplitude (dB, 0 is off) as linear gain: G_MAX is full scale, every DB_PER_HALF dB halves it
	if db <= 0.0 { 0.0 } else { libm::powf(0.5, (G_MAX - db) / DB_PER_HALF) }
}

fn audible(g: f32) -> f32 {
	// a gain at or under that of 0 dB is off
	if g <= DB_GAIN[0] { 0.0 } else { g }
}

fn fade(g: f32, m: f32) -> f32 {
	// scale an amplitude's gain by a fader m
	if m <= 0.0 { 0.0 } else { audible(g * m) }
}

fn lerp_gain(a: f32, b: f32, y: f32) -> f32 {
	audible(a * (1.0 - y) + b * y)
}

fn lin2smooth(x: f32) -> f32 {
	// ease from 0 to 1 as x goes from 0 to 1, slow at both ends
	x * x * (3.0 - 2.0 * x)
}

fn share_of(d: i32, pct: u8) -> i32 {
	(d as f32 * (pct as f32 / 100.0)) as i32
}

fn feats(x: u8) -> u16 {
	/*

	phnfeat bitmap (little endian, so 15 (MSB) to 0 (LSB) is the ordering):
		1  -> consonant/vowel
		2  -> unvoiced/voiced
		3  -> front
		4  -> back
		5  -> nasal
		6  -> alveolar
		7  -> stop
		8  -> bilabial
		9  -> velar
		10 -> sibilant
		11 -> lateral
		12 -> rhotic
		13 -> glide
		14 -> diphthong
		15 -> dental
		16 -> affricate

	*/
	match x {
		47                     => 0x0022,		// dx (alveolar flap)
		40                     => 0x0002,		// v
		1|2|3|4|5|6|7|14|29    => 0x0003,		// aa, ae, ah, ao, aw, ax, axr, er, ow
		13|15|19|20|21         => 0x0007,		// eh, ey, ih, ix, iy
		38|39                  => 0x000B,		// uh, uw
		27                     => 0x0032,		// n
		36                     => 0x0060,		// t
		11                     => 0x0062,		// d
		26                     => 0x0092,		// m
		31                     => 0x00C0,		// p
		9                      => 0x00C2,		// b
		28                     => 0x0112,		// ng
		23                     => 0x0140,		// k
		17                     => 0x0142,		// g
		34|35                  => 0x0220,		// s, sh
		43|44                  => 0x0222,		// z, zh
		24|25                  => 0x0422,		// l, lx
		32|33                  => 0x0822,		// r, rx
		42                     => 0x1002,		// y
		41                     => 0x1182,		// w
		8|30                   => 0x2003,		// ay, oy
		37                     => 0x4000,		// th
		12                     => 0x4002,		// dh
		10                     => 0x8260,		// ch
		22                     => 0x8262,		// jh
		_                      => 0x0000,		// otherwise
	}
}

fn samples(ms: f32) -> i32 { (ms * m_sr as f32 / 1000.0) as i32 }

fn fricative(p: u8) -> bool { matches!(p, 16 | 37 | 40 | 12 | 34 | 43 | 35 | 44) }
fn sibilant(f: u16) -> bool { f & 0x8200 == 0x200 }
// aw, ay, ow, oy (feats' 0x2000 marks only ay and oy)
fn diphthong(p: u8) -> bool { matches!(p, 5 | 8 | 29 | 30) }

// a word-internal t before an unstressed schwa ("button"): softened by SCHWA_T_ASP and SCHWA_T_ASP_DB
fn schwa_t(phone: u8, word_start: bool, next: u8, next_stressed: bool) -> bool {
	phone == 36 && !word_start && matches!(next, 6 | 20) && !next_stressed
}

fn stop_place(x: u8) -> usize { match x { 31 | 9 => 0, 36 | 11 => 1, _ => 2 } }

// a plosive's parts in samples: the closure and burst (an affricate's frication) play in its own segment,
// the aspiration in the start of the next phone
#[derive(Debug)]
struct Stop { closure: i32, burst: i32, asp: i32, }

fn stop_timeline(phone: u8, pdr: i32, prev: u8, next: u8, next_stressed: bool, word_start: bool, next_word_start: bool) -> Stop {
	let pf = feats(phone);
	if pf & 0x8000 != 0 {
		// affricate: always released, its frication plays where a stop's burst would
		let fric = samples(AFFRIC_MS[(pf & 0x2 != 0) as usize]).min((pdr - min_closure).max(0));
		return Stop { closure: pdr - fric, burst: fric, asp: 0 };
	}
	let nf = feats(next);

	// unreleased before another stop or a nasal, a t or d before a word starting with th or dh ("get the"), and a
	// word-final t after a vowel, r, n or l before a word starting with a consonant ("not that"); a k or g before a word
	// starting with a nasal, th or dh keeps a burst, or it is not heard ("unique New York", "thick thistle")
	let final_t = phone == 36 && !word_start && (feats(prev) & 0x1 != 0 || matches!(prev, 33 | 27 | 24 | 25))
		&& next_word_start && nf & 0x1 == 0 && !matches!(next, 0 | 48 | 49 | PAU);
	let before_dental = next_word_start && matches!(next, 37 | 12);
	let place = stop_place(phone);
	let velar_kept = place == 2 && next_word_start && (nf & 0x10 != 0 || before_dental);
	if !velar_kept && (nf & 0x50 != 0 || final_t || (before_dental && place != 0)) {
		return Stop { closure: (pdr as f32 * UNRELEASED_SHARE) as i32, burst: 0, asp: 0 };
	}
	let voiced = pf & 0x2 != 0;
	let mut burst = samples(BURST_MS[voiced as usize][place]);
	// a lip stop before th or dh keeps its release, without aspiration ("keep the")
	let mut asp = if voiced || before_dental { 0 } else { samples(ASP_MS[place]) };

	// too short for the whole release: burst and aspiration shrink together
	let room = (pdr - min_closure).max(0);
	if burst + asp > room {
		let k = room as f32 / (burst + asp) as f32;
		burst = (burst as f32 * k) as i32;
		asp = (asp as f32 * k) as i32;
	}

	// context only shortens the aspiration; the closure takes the time it gives up
	if nf & 0x1 != 0 {
		if !next_stressed { asp = (asp as f32 * ASP_UNSTRESSED) as i32; }
		if schwa_t(phone, word_start, next, next_stressed) { asp = (asp as f32 * SCHWA_T_ASP) as i32; }
	} else if nf & 0x1C00 != 0 {
		asp = (asp as f32 * ASP_BEFORE_SONORANT) as i32;
	} else {
		// obstruent, phrase-final release or pause: burst only
		asp = 0;
	}

	if feats(prev) & 0x200 != 0 && !word_start { asp = 0; }
	Stop { closure: pdr - burst - asp, burst, asp }
}

pub struct engine {
	pub tbl: &'static [u16; 1550],	// the formant table, swapped by --table
	vtm_eng: vtm,
	marker_A: i32,
	amp_len: i32,			// samples the amplitudes take to reach this phone's values
	amp_start: i32,			// counter value where that ramp starts
	voice_len: i32,			// samples the voicing takes to switch at a fricative edge (0: it follows amp_len)
	voice_start: i32,		// counter value where that switch starts
	asp_fade: i32,			// samples the aspiration takes to fade out under the voice
	dip_mid: i32,			// counter value at the middle of a dip (between two fricatives, or a glottal catch)
	dip_half: i32,			// samples either side of it the dip takes (0: no dip)
	dip_db: f32,			// and how deep it goes (dB)
	devoice_len: i32,		// samples over which a word-final voiced fricative loses its voice (0: it keeps it)
	gap: i32,				// samples of closure at the start of a fricative after a nasal (0: none)
	ab_at: i32,				// counter value where the formants start moving from the A to the B targets
	marker_B: i32,
	marker_BA: i32,
	is_first: bool,
	counter: i32,
	phn_l1: u8,
	phn_c:  u8,
	phn_r1: u8,
	pft_l1: u16,
	pft_c:  u16,
	pft_r1: u16,
	pdr_c:  i32,
	pdr_r1: i32,
	pst_l1: bool,
	seg_c: usize,			// stream index of the playing segment
	seg_r1: usize,
	seg_start: i32,			// counter value when the playing segment started
	pwi_c:  bool,			// phone starts a word
	pwi_r1: bool,
	stop_on: bool,			// playing a plosive's closure and burst
	stop_bar: bool,			// voiced plosive after a voiced sound: a voice bar in the closure
	stop_affric: bool,
	stop_labial: bool,
	lip_hold: f32,			// how far F2 and F3 are held down for a lip release (0..1)
	burst_len: i32,
	burst_decay: f32,
	burst_gain: f32,		// linear gain on the burst
	after_stop: bool,		// this phone follows a released plosive
	f1_len: i32,			// samples F1 takes into this phone when it moves faster than the other formants (0: same)
	f1_speed: f32,			// how much faster F1 moves than the shared interpolator
	carry: [f32; 7],		// RD, F1-F3, B1-B3 change per sample when the last move started (a cut-off move's speed)
	carry_n: i32,			// samples since then
	last: [f32; 7],			// those registers one sample ago
	after_stop_next: bool,
	asp_db: f32,			// aspiration level carried over from a voiceless stop (0: none)
	asp_start: i32,
	asp_seg: usize,
	asp_db_next: f32,
	asp_seg_next: usize,
	pst_c: bool,
	pst_r1: bool,
	GL_V: f32,
	RD_reg: f32,
	NZP_reg: f32,
	F1_reg: f32,
	F2_reg: f32,
	F3_reg: f32,
	B1_reg: f32,
	B2_reg: f32,
	B3_reg: f32,
	A2_reg: f32,
	A3_reg: f32,
	A4_reg: f32,
	A5_reg: f32,
	A6_reg: f32,
	A7_reg: f32,
	A8_reg: f32,
	A9_reg: f32,
	A10_reg: f32,
	AB_reg: f32,
	AF_reg: f32,
	AH_reg: f32,
	AV_reg: f32,
	RD_reg_src: f32,
	NZP_reg_src: f32,
	F1_reg_src: f32,
	F2_reg_src: f32,
	F3_reg_src: f32,
	B1_reg_src: f32,
	B2_reg_src: f32,
	B3_reg_src: f32,
	A2_reg_src: f32,
	A3_reg_src: f32,
	A4_reg_src: f32,
	A5_reg_src: f32,
	A6_reg_src: f32,
	A7_reg_src: f32,
	A8_reg_src: f32,
	A9_reg_src: f32,
	A10_reg_src: f32,
	AB_reg_src: f32,
	AF_reg_src: f32,
	AH_reg_src: f32,
	AV_reg_src: f32,
	RD_reg_trg: f32,
	NZP_reg_trg: f32,
	F1_reg_trg: f32,
	F2_reg_trg: f32,
	F3_reg_trg: f32,
	B1_reg_trg: f32,
	B2_reg_trg: f32,
	B3_reg_trg: f32,
	A2_reg_trg: f32,
	A3_reg_trg: f32,
	A4_reg_trg: f32,
	A5_reg_trg: f32,
	A6_reg_trg: f32,
	A7_reg_trg: f32,
	A8_reg_trg: f32,
	A9_reg_trg: f32,
	A10_reg_trg: f32,
	AB_reg_trg: f32,
	AF_reg_trg: f32,
	AH_reg_trg: f32,
	AV_reg_trg: f32,
	slope: f32,
	interp: f32,
	master_fade: f32,
	C_A_src: u8,
	C_B_src: u8,
	RD_A_src: f32,
	RD_B_src: f32,
	NZP_A_src: f32,
	F1_A_src: f32,
	F2_A_src: f32,
	F3_A_src: f32,
	B1_A_src: f32,
	B2_A_src: f32,
	B3_A_src: f32,
	A2_src: f32,
	A3_src: f32,
	A4_src: f32,
	A5_src: f32,
	A6_src: f32,
	A7_src: f32,
	A8_src: f32,
	A9_src: f32,
	A10_src: f32,
	AB_src: f32,
	AF_src: f32,
	AH_src: f32,
	AV_src: f32,
	F1_B_src: f32,
	F2_B_src: f32,
	F3_B_src: f32,
	B1_B_src: f32,
	B2_B_src: f32,
	B3_B_src: f32,
}
impl engine {
	pub fn new() -> engine {
		engine {
			tbl: &formant_tbl,
			vtm_eng: vtm::new(),
			marker_A:  0,
			amp_len:   0,
			ab_at: 0,
			amp_start: 0,
			voice_len: 0,
			voice_start: 0,
			asp_fade: 0,
			dip_mid: 0,
			dip_half: 0,
			dip_db: 0.0,
			devoice_len: 0,
			gap: 0,
			marker_B:  0,
			marker_BA: 0,
			is_first:  true,
			counter:   0,
			phn_l1: 0,
			phn_c:  0,
			phn_r1: 0,
			pft_l1: 0,
			pft_c:  0,
			pft_r1: 0,
			pdr_c:  0,
			pdr_r1: 0,
			pst_l1: false,
			pst_c:  false,
			pst_r1: false,
			seg_c: 0,
			seg_r1: 0,
			seg_start: 0,
			pwi_c:  true,
			pwi_r1: true,
			stop_on: false,
			stop_bar: false,
			stop_affric: false,
			stop_labial: false,
			lip_hold: 0.0,
			burst_len: 0,
			burst_decay: 0.0,
			burst_gain: 1.0,
			after_stop: false,
			f1_len: 0,
			f1_speed: 1.0,
			carry: [0.0; 7],
			carry_n: 0,
			last: [0.0; 7],
			after_stop_next: false,
			asp_db: 0.0,
			asp_start: 0,
			asp_seg: 0,
			asp_db_next: 0.0,
			asp_seg_next: 0,
			// traversal registers
			GL_V:    1.0,				// glottalization AV level
			RD_reg:  0.0,
			NZP_reg: 0.0,
			F1_reg:  0.0,
			F2_reg:  0.0,
			F3_reg:  0.0,
			B1_reg:  0.0,
			B2_reg:  0.0,
			B3_reg:  0.0,
			A2_reg:  0.0,
			A3_reg:  0.0,
			A4_reg:  0.0,
			A5_reg:  0.0,
			A6_reg:  0.0,
			A7_reg:  0.0,
			A8_reg:  0.0,
			A9_reg:  0.0,
			A10_reg:  0.0,
			AB_reg:  0.0,
			AF_reg:  0.0,
			AH_reg:  0.0,
			AV_reg:  0.0,
			// source registers
			RD_reg_src:  0.0,
			NZP_reg_src: 0.0,
			F1_reg_src:  0.0,
			F2_reg_src:  0.0,
			F3_reg_src:  0.0,
			B1_reg_src:  0.0,
			B2_reg_src:  0.0,
			B3_reg_src:  0.0,
			A2_reg_src:  0.0,
			A3_reg_src:  0.0,
			A4_reg_src:  0.0,
			A5_reg_src:  0.0,
			A6_reg_src:  0.0,
			A7_reg_src:  0.0,
			A8_reg_src:  0.0,
			A9_reg_src:  0.0,
			A10_reg_src:  0.0,
			AB_reg_src:  0.0,
			AF_reg_src:  0.0,
			AH_reg_src:  0.0,
			AV_reg_src:  0.0,
			// target registers
			RD_reg_trg:  0.0,
			NZP_reg_trg: 0.0,
			F1_reg_trg:  0.0,
			F2_reg_trg:  0.0,
			F3_reg_trg:  0.0,
			B1_reg_trg:  0.0,
			B2_reg_trg:  0.0,
			B3_reg_trg:  0.0,
			A2_reg_trg:  0.0,
			A3_reg_trg:  0.0,
			A4_reg_trg:  0.0,
			A5_reg_trg:  0.0,
			A6_reg_trg:  0.0,
			A7_reg_trg:  0.0,
			A8_reg_trg:  0.0,
			A9_reg_trg:  0.0,
			A10_reg_trg:  0.0,
			AB_reg_trg:  0.0,
			AF_reg_trg:  0.0,
			AH_reg_trg:  0.0,
			AV_reg_trg:  0.0,
			slope:        0.0,
			interp:       0.0,
			master_fade:  1.0,					// master fader for phrase-final trigger
			C_A_src:   0,
			C_B_src:   0,
			RD_A_src:  0.0,
			RD_B_src:  0.0,
			NZP_A_src: 0.0,
			F1_A_src:  0.0,
			F2_A_src:  0.0,
			F3_A_src:  0.0,
			B1_A_src:  0.0,
			B2_A_src:  0.0,
			B3_A_src:  0.0,
			A2_src:    0.0,
			A3_src:    0.0,
			A4_src:    0.0,
			A5_src:    0.0,
			A6_src:    0.0,
			A7_src:    0.0,
			A8_src:    0.0,
			A9_src:    0.0,
			A10_src:   0.0,
			AB_src:    0.0,
			AF_src:    0.0,
			AH_src:    0.0,
			AV_src:    0.0,
			F1_B_src:  0.0,
			F2_B_src:  0.0,
			F3_B_src:  0.0,
			B1_B_src:  0.0,
			B2_B_src:  0.0,
			B3_B_src:  0.0,
		}
	}
	pub fn set_voice(&mut self, v: Voice) { self.vtm_eng.set_voice(v); }
	fn advance(&mut self, phone: u8, dur_ms: i32, stressed: bool, seg: usize, word_start: bool) {
		self.phn_l1 = self.phn_c;
		self.phn_c  = self.phn_r1;
		self.phn_r1 = phone;
		self.pft_l1 = self.pft_c;
		self.pft_c  = self.pft_r1;
		self.pft_r1 = feats(phone);
		self.pdr_c  = self.pdr_r1;
		self.pdr_r1 = samples(dur_ms as f32);
		self.pst_l1 = self.pst_c;
		self.pst_c  = self.pst_r1;
		self.pst_r1 = stressed;
		self.seg_c  = self.seg_r1;
		self.seg_r1 = seg;
		self.pwi_c  = self.pwi_r1;
		self.pwi_r1 = word_start;
	}
	pub fn is_running(&self)  -> bool { self.counter < self.marker_B }
	pub fn is_ready(&self)    -> bool { self.phn_c != 0 }
	pub fn is_not_last(&self) -> bool { self.phn_c == 0 || self.phn_r1 != 0 }
	fn fetch(&mut self) {
		// the pause uses the silent row
		let row: u8 = if self.phn_c == PAU { 0 } else { self.phn_c };
		// the phrase-end release after a nasal is that nasal dying away
		let row = if matches!(row, 48 | 49) && self.pft_l1 & 0x10 != 0 { self.phn_l1 } else { row };
		let offset_src: usize = row as usize * 31;
		//---------------source---------------------
		self.C_A_src   = self.tbl[offset_src+0] as u8;						// also target during transition time
		self.C_B_src   = self.tbl[offset_src+1] as u8;
		self.RD_A_src  = self.tbl[offset_src+2] as f32 / 100.0;
		self.RD_B_src  = self.tbl[offset_src+3] as f32 / 100.0;
		self.NZP_A_src = if feats(row) & 0x10 != 0 { 1.0 } else { 0.0 };	// nasal coupling: 1 for a nasal consonant
		self.F1_A_src  = self.tbl[offset_src+5] as f32;
		self.F2_A_src  = self.tbl[offset_src+6] as f32;
		self.F3_A_src  = self.tbl[offset_src+7] as f32;
		self.B1_A_src  = self.tbl[offset_src+8] as f32;
		self.B2_A_src  = self.tbl[offset_src+9] as f32;
		self.B3_A_src  = self.tbl[offset_src+10] as f32;
		self.A2_src    = self.tbl[offset_src+11] as f32;
		self.A3_src    = self.tbl[offset_src+12] as f32;
		self.A4_src    = self.tbl[offset_src+13] as f32;
		self.A5_src    = self.tbl[offset_src+14] as f32;
		self.A6_src    = self.tbl[offset_src+15] as f32;
		self.A7_src    = self.tbl[offset_src+16] as f32;
		self.A8_src    = self.tbl[offset_src+17] as f32;
		self.A9_src    = self.tbl[offset_src+18] as f32;
		self.A10_src   = self.tbl[offset_src+19] as f32;
		self.AB_src    = self.tbl[offset_src+20] as f32;
		self.AF_src    = self.tbl[offset_src+21] as f32;
		self.AH_src    = self.tbl[offset_src+22] as f32;
		self.AV_src    = self.tbl[offset_src+23] as f32;
		self.F1_B_src  = self.tbl[offset_src+25] as f32;
		self.F2_B_src  = self.tbl[offset_src+26] as f32;
		self.F3_B_src  = self.tbl[offset_src+27] as f32;
		self.B1_B_src  = self.tbl[offset_src+28] as f32;
		self.B2_B_src  = self.tbl[offset_src+29] as f32;
		self.B3_B_src  = self.tbl[offset_src+30] as f32;

		if self.F1_B_src == 0.0 { self.F1_B_src = self.F1_A_src; }
		if self.F2_B_src == 0.0 { self.F2_B_src = self.F2_A_src; }
		if self.F3_B_src == 0.0 { self.F3_B_src = self.F3_A_src; }
		if self.B1_B_src == 0.0 { self.B1_B_src = self.B1_A_src; }
		if self.B2_B_src == 0.0 { self.B2_B_src = self.B2_A_src; }
		if self.B3_B_src == 0.0 { self.B3_B_src = self.B3_A_src; }

		if self.phn_c == PAU && !self.is_first {
			// hold the formants where they are during a pause
			// (a first pause has nothing to hold and keeps the silent row's values)
			self.F1_A_src = self.F1_reg;
			self.F2_A_src = self.F2_reg;
			self.F3_A_src = self.F3_reg;
			self.B1_A_src = self.B1_reg;
			self.B2_A_src = self.B2_reg;
			self.B3_A_src = self.B3_reg;
			self.F1_B_src = self.F1_reg;
			self.F2_B_src = self.F2_reg;
			self.F3_B_src = self.F3_reg;
			self.B1_B_src = self.B1_reg;
			self.B2_B_src = self.B2_reg;
			self.B3_B_src = self.B3_reg;
		}
	}
	fn ftar(&mut self) {
		if self.pft_c & 0x1 == 1 {
			// STRESS COLORING FOR VOWEL
			let factor: f32 = if self.pst_c { 0.75 } else { 1.25 };

			// VOWEL BEFORE R
			// sits higher, and for the back vowels further back ("air", "four", "tour")
			if self.pft_r1 & 0x800 != 0 {
				if let Some(&(_, f1a, f2a, f1b, f2b)) = BEFORE_R.iter().find(|v| v.0 == self.phn_c) {
					(self.F1_A_src, self.F2_A_src, self.F1_B_src, self.F2_B_src) = (f1a, f2a, f1b, f2b);
				}
			}

			// VELAR PINCHING RULE (more on a front vowel's F2)
			if self.pft_l1 & 0x100 != 0 && self.phn_l1 != 41 {
				self.F2_A_src += if self.pft_c & 0x4 != 0 { 130.0 } else { 80.0 } * factor;
				self.F3_A_src -= 100.0 * factor;
			}
			if self.pft_r1 & 0x100 != 0 && self.phn_r1 != 41 {
				// a back vowel's F2 stays put before a velar
				if self.F2_B_src > 1500.0 { self.F2_B_src += 80.0 * factor; }
				self.F3_B_src -= 100.0 * factor;
			}

			// BILABIAL ROUNDING RULE
			if self.pft_l1 & 0x80 != 0 {
				self.F2_A_src -= 50.0 * factor;
			}
			if self.pft_r1 & 0x80 != 0 {
				self.F2_B_src -= 50.0 * factor;
			}

			// SONORANT ADJUSTMENT
			// after r and w; an l and a y keep their F3 high
			if matches!(self.phn_l1, 32 | 33 | 41) {
				self.F3_A_src -= 150.0 * factor;
			}
			if self.phn_r1 == 41 {
				// before w; an l and a y keep their F3 high, an r has its own rule
				self.F3_B_src -= 150.0 * factor;
			}

			// NASALIZATION
			if self.pft_r1 & 0x10 != 0 {
				self.F2_A_src += 110.0 * factor;
				self.F2_B_src += 110.0 * factor;
			}

			// WIDEN BANDWIDTH AFTER A NASAL
			if self.pft_l1 & 0x10 != 0 {
				self.B1_A_src += 70.0 * factor;
				self.B1_B_src += 70.0 * factor;
			}

			// LATERAL ADJUSTMENT
			if self.pft_r1 & 0x400 != 0 {
				if self.pft_c & 0x4 != 0 {
					self.F2_A_src -= 150.0 * factor;
					self.F2_B_src -= 150.0 * factor;
				} else if self.pft_c & 0x2000 != 0 {
					// diphthong next to lateral
					self.F2_B_src -= 250.0 * factor;
				}
			}

			// VOWEL RHOTICIZATION
			if self.pft_r1 & 0x800 != 0 {
				// the vowel takes on r-coloring as it moves into the r, not from its start, by the same amount
				// stressed or not; its F2 does not rise away from the r ("tour")
				self.F3_B_src -= RHOTIC_F3;
				self.F2_B_src = self.F2_B_src.min(self.F2_A_src);
			}

			// GLIDE TOWARD THE NEXT CONSONANT
			// a vowel's own F2 glide does not head away from where the next consonant takes F2 ("cat", "pick");
			// the diphthongs aw, ay, ow and oy keep theirs
			if !diphthong(self.phn_c) && self.pft_r1 & 0x1 == 0 && !matches!(self.phn_r1, 0 | 18 | 48 | 49 | PAU) {
				let next_f2 = match locus(self.phn_r1) {
					Some((k, o, floor)) => o + k * self.F2_B_src.max(floor),
					None => self.tbl[self.phn_r1 as usize * 31 + 6] as f32,
				};
				if (self.F2_B_src - self.F2_A_src) * (next_f2 - self.F2_B_src) < 0.0 {
					self.F2_B_src = self.F2_A_src;
				}
			}

			// BACK VOWEL ALVEOLARIZATION
			// not before an r, where the vowel has its own place
			if self.pft_c == 0x000B && self.pft_r1 & 0x800 == 0 {
				if self.pft_l1 & 0x20 != 0 {
					self.F2_A_src += 250.0 * factor;
				}
				// not into a dark l, which pulls F2 down: the vowel went up and back down ("full")
				if self.pft_r1 & 0x20 != 0 && self.phn_r1 != 25 {
					self.F2_B_src += 250.0 * factor;
				}
			}

		} else {
			// F2 and the velars' F3 are left to loci(), which places them from the neighbouring vowels
			// BEFORE A FRONT VOWEL: w moves up toward it; a sibilant's F3 (which A3 follows) rises
			if self.pft_r1 & 0x5 == 0x5 {
				if self.phn_c == 41 {
					self.F2_A_src += 150.0;
					self.F2_B_src += 150.0;
				}
				if self.phn_c == 41 || self.pft_c & 0x200 != 0 {
					self.F3_A_src += 175.0;
					self.F3_B_src += 175.0;
				}
			}
			// NASALS (m, n, ng)
			if self.pft_c & 0x10 != 0 {
				// after a front vowel, a lower F3 (ng's F3 is its pinch)
				if self.phn_c != 28 && self.pft_l1 & 0x5 == 0x5 {
					self.F3_A_src -= 100.0;
					self.F3_B_src -= 100.0;
				}
				// a wider B1
				self.B1_A_src += 107.0;
				self.B1_B_src += 107.0;
				// quieter after an unstressed vowel, unless the nasal starts the word
				if self.pft_l1 & 0x1 != 0 && !self.pst_l1 && !self.pwi_c {
					self.AV_src -= NASAL_AFTER_UNSTRESSED_DB;
				}
			}
		}

		// a sibilant next to another fricative keeps its level, so the other doesn't drown it ("thieves fished")
		let sib_cluster = sibilant(self.pft_c) && (fricative(self.phn_l1) || fricative(self.phn_r1));
		// frication level: not for vowels, stops or affricates (which keep the table's)
		if self.pft_c & 0x41 == 0 {
			if self.pdr_c < eighty_ms && self.pft_l1 & 0x40 == 0 && !sib_cluster {
				// short fricatives don't reach AF target (less than 80 ms), lower AF
				// after a stop the noise starts at full level
				self.AF_src -= 3.0;
			}
			if self.pft_l1 & 0x1 == 1 && self.pft_r1 & 0x1 == 1 {
				// fricatives are stronger between two vowels
				self.AF_src += 3.0;
			}
		}

		if self.pft_r1 & 0x4 != 0 {
			// lower AF if front vowel is next
			self.AF_src -= 3.0;
		}

		if !self.pst_l1 && !self.pst_r1 && self.pft_c & 0x1 == 0 && !sib_cluster {
			// unstressed consonant has lower AF
			self.AF_src -= 3.0;
		}
	}
	fn reduce(&mut self) {
		// move vowel targets toward the neutral vowel (AX) as the vowel gets shorter
		if self.pft_c & 0x1 == 0 || self.phn_c == 6 || self.phn_c == 20 { return; }
		[self.F1_A_src, self.F2_A_src, self.F3_A_src] = self.reduced([self.F1_A_src, self.F2_A_src, self.F3_A_src], self.pdr_c);
		[self.F1_B_src, self.F2_B_src, self.F3_B_src] = self.reduced([self.F1_B_src, self.F2_B_src, self.F3_B_src], self.pdr_c);
	}
	fn reduced(&self, f: [f32; 3], dur: i32) -> [f32; 3] {
		// F1-F3 of a vowel lasting dur samples, moved toward the neutral vowel's
		let ax_row = 6 * 31;
		let k = reduce_k(dur as f32 * 1000.0 / m_sr as f32);
		[0, 1, 2].map(|i| { let n = self.tbl[ax_row + 5 + i] as f32; n + (f[i] - n) * k })
	}
	fn quiet_next_db(&self) -> f32 {
		// how much quieter above F2 the next vowel or sonorant is than an open vowel (dB; 0 if none or louder)
		if self.pft_r1 & 0x1 == 0 && self.pft_r1 & 0x1C00 == 0 { return 0.0; }
		let [f1, f2, _] = self.next_vowel_formants();
		(-40.0 * libm::log10f(f1 * f2 / RELEASE_REF)).max(0.0)
	}
	fn next_vowel_formants(&self) -> [f32; 3] {
		// the next vowel's F1-F3 at its start, reduced for its length as reduce() will do
		let row = self.phn_r1 as usize * 31;
		let target = [self.tbl[row + 5] as f32, self.tbl[row + 6] as f32, self.tbl[row + 7] as f32];
		if self.phn_r1 == 6 || self.phn_r1 == 20 { return target; }
		self.reduced(target, self.pdr_r1)
	}
	fn loci(&mut self, prev_f2: f32) {
		let prev_vowel = self.pft_l1 & 0x1 != 0;
		let next_vowel = self.pft_r1 & 0x1 != 0;
		if self.phn_c == 18 {
			// h is the next vowel's formants, breathed
			if next_vowel {
				let [f1, f2, f3] = self.next_vowel_formants();
				(self.F1_A_src, self.F2_A_src, self.F3_A_src) = (f1, f2, f3);
				(self.F1_B_src, self.F2_B_src, self.F3_B_src) = (f1, f2, f3);
				self.AH_src = H_DB - asp_front_cut(f1, f2);
			}
			return;
		}
		let Some((f2_slope, f2_offset, floor)) = locus(self.phn_c) else { return };
		let [next_f1, next_f2, next_f3] = self.next_vowel_formants();
		if next_vowel {
			let f1 = (self.F1_A_src + F1_FOLLOW * (next_f1 - F1_FOLLOW_REF)).max(F1_FOLLOW_MIN);
			(self.F1_A_src, self.F1_B_src) = (f1, f1);
		}
		// f and v: the tongue is free, so it is already where the next vowel wants it (no F3 fall into "first")
		if next_vowel && (self.phn_c == 16 || self.phn_c == 40) { (self.F3_A_src, self.F3_B_src) = (next_f3, next_f3); }
		let left  = if prev_vowel { Some(prev_f2) } else { None };
		let right = if next_vowel { Some(next_f2) } else { None };
		// F2 (and F3 for k and g) at an edge whose vowel has F2 v; k and g sit above a front vowel's F2, with F3 coming
		// down toward it
		let phn = self.phn_c;
		let edge = |v: f32, f3: f32| {
			let f2 = f2_offset + f2_slope * v.max(floor);
			if matches!(phn, 17 | 23) { (f2.max(v + VELAR_F2_ABOVE), VELAR_F3.1 + VELAR_F3.0 * v) } else { (f2, f3) }
		};
		// a side with no vowel holds the other side's value
		if let Some(v) = left.or(right) { (self.F2_A_src, self.F3_A_src) = edge(v, self.F3_A_src); }
		if let Some(v) = right.or(left) { (self.F2_B_src, self.F3_B_src) = edge(v, self.F3_B_src); }
		// ng's F3 pinches to just above its F2
		if phn == 28 { (self.F3_A_src, self.F3_B_src) = (self.F2_A_src + NG_PINCH_HZ, self.F2_B_src + NG_PINCH_HZ); }
	}
	fn load_registers(&mut self) {
		// load for the first time
		self.counter = 0;
		// source
		self.RD_reg_src  = self.RD_A_src;
		self.NZP_reg_src = self.NZP_A_src;
		self.F1_reg_src  = self.F1_A_src;
		self.F2_reg_src  = self.F2_A_src;
		self.F3_reg_src  = self.F3_A_src;
		self.B1_reg_src  = self.B1_A_src;
		self.B2_reg_src  = self.B2_A_src;
		self.B3_reg_src  = self.B3_A_src;
		self.A2_reg_src  = 0.0;
		self.A3_reg_src  = 0.0;
		self.A4_reg_src  = 0.0;
		self.A5_reg_src  = 0.0;
		self.A6_reg_src  = 0.0;
		self.A7_reg_src  = 0.0;
		self.A8_reg_src  = 0.0;
		self.A9_reg_src  = 0.0;
		self.A10_reg_src = 0.0;
		self.AB_reg_src  = 0.0;
		self.AF_reg_src  = 0.0;
		self.AH_reg_src  = 0.0;
		self.AV_reg_src  = 0.0;
		// target
		self.RD_reg_trg  = self.RD_reg_src;
		self.NZP_reg_trg = self.NZP_reg_src;
		self.F1_reg_trg  = self.F1_reg_src;
		self.F2_reg_trg  = self.F2_reg_src;
		self.F3_reg_trg  = self.F3_reg_src;
		self.B1_reg_trg  = self.B1_reg_src;
		self.B2_reg_trg  = self.B2_reg_src;
		self.B3_reg_trg  = self.B3_reg_src;
		self.A2_reg_trg  = db_gain(self.A2_src);
		self.A3_reg_trg  = db_gain(self.A3_src);
		self.A4_reg_trg  = db_gain(self.A4_src);
		self.A5_reg_trg  = db_gain(self.A5_src);
		self.A6_reg_trg  = db_gain(self.A6_src);
		self.A7_reg_trg  = db_gain(self.A7_src);
		self.A8_reg_trg  = db_gain(self.A8_src);
		self.A9_reg_trg  = db_gain(self.A9_src);
		self.A10_reg_trg = db_gain(self.A10_src);
		self.AB_reg_trg  = db_gain(self.AB_src);
		self.AF_reg_trg  = db_gain(self.AF_src);
		self.AH_reg_trg  = db_gain(self.AH_src);
		self.AV_reg_trg  = db_gain(self.AV_src);
	}
	fn regs(&self) -> [f32; 7] { [self.RD_reg, self.F1_reg, self.F2_reg, self.F3_reg, self.B1_reg, self.B2_reg, self.B3_reg] }
	fn start_move(&mut self) {
		// keep the registers' speed going into the new move
		let now = self.regs();
		for k in 0..7 { self.carry[k] = now[k] - self.last[k]; }
		self.carry_n = 0;
		self.interp = 0.0;
	}
	fn update_registers(&mut self) {
		if self.interp < 1.0 { self.interp = (self.interp + self.slope).min(1.0); }

		// the carried speed fades to nothing over CARRY_MS: n (1 - n/N)^2 starts at that speed and ends at rest
		let len = samples(CARRY_MS);
		self.carry_n = (self.carry_n + 1).min(len);
		let n = self.carry_n as f32 / len as f32;
		let carry_k = self.carry_n as f32 * (1.0 - n) * (1.0 - n);
		self.last = self.regs();

		// formants move in straight lines with rounded ends (the amps, done elsewhere, ease in and out)
		let track = |x: f32| {
			let (e, v) = (TRACK_EASE, 1.0 / (1.0 - TRACK_EASE));
			if x < e { v * x * x / (2.0 * e) } else if x > 1.0 - e { 1.0 - v * (1.0 - x) * (1.0 - x) / (2.0 * e) } else { v * (x - e / 2.0) }
		};
		let y  = track(self.interp);
		let y1 = track((self.interp * self.f1_speed).min(1.0));

		self.RD_reg  = self.RD_reg_src  * (1.0 - y) + self.RD_reg_trg  * y;
		self.F1_reg  = self.F1_reg_src  * (1.0 - y1) + self.F1_reg_trg  * y1;
		self.F2_reg  = self.F2_reg_src  * (1.0 - y) + self.F2_reg_trg  * y;
		self.F3_reg  = self.F3_reg_src  * (1.0 - y) + self.F3_reg_trg  * y;
		self.B1_reg  = self.B1_reg_src  * (1.0 - y) + self.B1_reg_trg  * y;
		self.B2_reg  = self.B2_reg_src  * (1.0 - y) + self.B2_reg_trg  * y;
		self.B3_reg  = self.B3_reg_src  * (1.0 - y) + self.B3_reg_trg  * y;

		self.RD_reg += self.carry[0] * carry_k;
		self.F1_reg += self.carry[1] * carry_k;
		self.F2_reg += self.carry[2] * carry_k;
		self.F3_reg += self.carry[3] * carry_k;
		self.B1_reg += self.carry[4] * carry_k;
		self.B2_reg += self.carry[5] * carry_k;
		self.B3_reg += self.carry[6] * carry_k;
	}
	fn update_amp_registers(&mut self) {
		let now = self.counter;
		let y = if self.amp_len > 0 { lin2smooth(((now - self.amp_start) as f32 / self.amp_len as f32).clamp(0.0, 1.0)) } else { 0.0 };

		// the nasal coupling switches with the amplitudes
		self.NZP_reg = self.NZP_reg_src * (1.0 - y) + self.NZP_reg_trg * y;
		// the velum lowers during a vowel before a nasal and is still rising after one, so the vowel is partly nasal;
		// not the diphthongs aw, ay, ow and oy, which sound too nasal with it ("joined")
		if self.pft_c & 0x1 != 0 && !diphthong(self.phn_c) {
			let near = |nasal: bool, d: i32, len: f32| if nasal { NASAL_VOWEL_K * (1.0 - d as f32 / samples(len) as f32).clamp(0.0, 1.0) } else { 0.0 };
			let k = near(self.pft_r1 & 0x10 != 0, self.marker_B - now, NASAL_ANTIC_MS).max(near(self.pft_l1 & 0x10 != 0, now, NASAL_CARRY_MS));
			self.NZP_reg = self.NZP_reg.max(k);
		}

		// update the interpolator for the parallel amps
		self.A2_reg  = lerp_gain(self.A2_reg_src, self.A2_reg_trg, y);
		self.A3_reg  = lerp_gain(self.A3_reg_src, self.A3_reg_trg, y);
		self.A4_reg  = lerp_gain(self.A4_reg_src, self.A4_reg_trg, y);
		self.A5_reg  = lerp_gain(self.A5_reg_src, self.A5_reg_trg, y);
		self.A6_reg  = lerp_gain(self.A6_reg_src, self.A6_reg_trg, y);
		self.A7_reg  = lerp_gain(self.A7_reg_src, self.A7_reg_trg, y);
		self.A8_reg  = lerp_gain(self.A8_reg_src, self.A8_reg_trg, y);
		self.A9_reg  = lerp_gain(self.A9_reg_src, self.A9_reg_trg, y);
		self.A10_reg = lerp_gain(self.A10_reg_src, self.A10_reg_trg, y);

		// update the interpolator for main amps
		self.AB_reg = lerp_gain(self.AB_reg_src, self.AB_reg_trg, y);
		let (mut g_af, mut g_ah, mut g_av) = (1.0, 1.0, 1.0);
		let t = (now - self.dip_mid) as f32 / self.dip_half.max(1) as f32;
		if self.dip_half > 0 && t.abs() < 1.0 {
			let g = libm::exp2f(-self.dip_db / (2.0 * DB_PER_HALF) * (1.0 + libm::cosf(core::f32::consts::PI * t)));
			(g_af, g_ah, g_av) = (g, g, g);
		}
		if self.gap > 0 && now >= 0 {
			let k = ((now - self.gap) as f32 / samples(NASAL_GAP_RISE_MS) as f32).clamp(0.0, 1.0);
			(g_af, g_ah) = (k, k);
		}
		if self.asp_db > 0.0 && now < 0 {
			// aspiration rises over 2 ms after the burst
			let rise = ((now - self.asp_start) as f32 / two_ms as f32).min(1.0);
			(g_af, g_ah, g_av) = (0.0, rise, 0.0);
		}
		self.AF_reg = fade(lerp_gain(self.AF_reg_src, self.AF_reg_trg, y), self.master_fade * g_af);
		self.AH_reg = fade(lerp_gain(self.AH_reg_src, self.AH_reg_trg, y), self.master_fade * g_ah);

		if self.devoice_len > 0 && now >= 0 { g_av *= (1.0 - now as f32 / self.devoice_len as f32).clamp(0.0, 1.0); }
		let yv = if self.voice_len > 0 { lin2smooth(((now - self.voice_start) as f32 / self.voice_len as f32).clamp(0.0, 1.0)) } else { y };
		self.AV_reg = fade(lerp_gain(self.AV_reg_src, self.AV_reg_trg, yv), self.master_fade * g_av);

		if self.stop_on && now >= 0 {
			// the voice bar fades over the closure and holds through the burst
			let bar = if self.stop_bar {
				let t = (now.min(self.marker_A) as f32 / self.marker_A.max(1) as f32).min(1.0);
				fade(self.AV_reg, libm::exp2f(-VOICE_BAR_FADE_DB / DB_PER_HALF * t))
			} else { 0.0 };
			if now < self.marker_A || self.burst_len == 0 {
				// closure (all of an unreleased stop): the previous phone's sound dies away over 5 ms
				let g = self.master_fade * (1.0 - now as f32 / five_ms as f32).max(0.0);
				self.AF_reg = fade(self.AF_reg_src, g);
				self.AH_reg = fade(self.AH_reg_src, g);
				self.AV_reg = if self.stop_bar { bar } else { fade(self.AV_reg_src, g) };
			} else {
				// burst: a sharp onset, then an exponential decay over its length
				let t = (now - self.marker_A) as f32 / self.burst_len.max(1) as f32;
				let g = self.burst_gain * libm::exp2f(-self.burst_decay / DB_PER_HALF * t);
				if self.stop_labial {
					// a lip release: weak noise through the vocal tract, low and falling
					self.AF_reg = 0.0;
					self.AH_reg = fade(db_gain(LABIAL_BURST_AH[(self.pft_c & 0x2 != 0) as usize]), self.master_fade * g);
				} else {
					self.AF_reg = fade(self.AF_reg, g);
					self.AH_reg = 0.0;
				}
				// a voiced affricate's frication is voiced; a stop's burst keeps the voice bar
				if !self.stop_affric { self.AV_reg = bar; }
			}
		}
		if self.asp_db > 0.0 && now >= 0 {
			// aspiration fades out under the first voicing pulses
			let k = 1.0 - now as f32 / self.asp_fade.max(1) as f32;
			if k > 0.0 { self.AH_reg = self.AH_reg.max(fade(db_gain(self.asp_db), self.master_fade * k)); }
		}
	}
	fn update_A_to_B(&mut self) {
		let dist = (self.marker_B - self.ab_at).max(thirty_ms);
		self.slope = 1.0 / dist as f32;
		self.start_move();
		self.f1_speed = 1.0;
		// source (A)
		self.RD_reg_src  = self.RD_reg;
		self.F1_reg_src  = self.F1_reg;
		self.F2_reg_src  = self.F2_reg;
		self.F3_reg_src  = self.F3_reg;
		self.B1_reg_src  = self.B1_reg;
		self.B2_reg_src  = self.B2_reg;
		self.B3_reg_src  = self.B3_reg;
		// target (B)
		self.RD_reg_trg  = self.RD_B_src;
		self.F1_reg_trg  = self.F1_B_src;
		self.F2_reg_trg  = self.F2_B_src;
		self.F3_reg_trg  = self.F3_B_src;
		self.B1_reg_trg  = self.B1_B_src;
		self.B2_reg_trg  = self.B2_B_src;
		self.B3_reg_trg  = self.B3_B_src;
	}
	fn update_B_to_Next(&mut self) {
		let mut dist = if self.stop_on {
			// the formants reach a stop by its closure, where the sound stops
			self.marker_BA
		} else {
			self.marker_A + self.marker_BA
		};
		if self.pft_c & 0x800 != 0 && self.pft_l1 & 0x1 != 0 {
			// out of a vowel the tongue moves into an r quickly
			dist = dist.min(samples(R_ONSET_MS));
		}
		let dist = dist.max(thirty_ms);
		self.slope = 1.0 / dist as f32;
		self.start_move();
		self.f1_speed = if self.f1_len > 0 { (dist as f32 / (self.f1_len + self.marker_BA).max(1) as f32).max(1.0) } else { 1.0 };

		// negative: the move into this phone plays out in the previous phone's tail
		self.counter = -(self.marker_BA);

		// source (A)
		self.RD_reg_src  = self.RD_reg;
		self.NZP_reg_src = self.NZP_reg;
		self.F1_reg_src  = self.F1_reg;
		self.F2_reg_src  = self.F2_reg;
		self.F3_reg_src  = self.F3_reg;
		self.B1_reg_src  = self.B1_reg;
		self.B2_reg_src  = self.B2_reg;
		self.B3_reg_src  = self.B3_reg;
		self.A2_reg_src  = self.A2_reg;
		self.A3_reg_src  = self.A3_reg;
		self.A4_reg_src  = self.A4_reg;
		self.A5_reg_src  = self.A5_reg;
		self.A6_reg_src  = self.A6_reg;
		self.A7_reg_src  = self.A7_reg;
		self.A8_reg_src  = self.A8_reg;
		self.A9_reg_src  = self.A9_reg;
		self.A10_reg_src = self.A10_reg;
		self.AB_reg_src  = self.AB_reg;
		self.AF_reg_src  = self.AF_reg;
		self.AH_reg_src  = self.AH_reg;
		self.AV_reg_src  = self.AV_reg;
		// target (B)
		self.RD_reg_trg  = self.RD_A_src;
		self.NZP_reg_trg = self.NZP_A_src;
		self.F1_reg_trg  = self.F1_A_src;
		self.F2_reg_trg  = self.F2_A_src;
		self.F3_reg_trg  = self.F3_A_src;
		self.B1_reg_trg  = self.B1_A_src;
		self.B2_reg_trg  = self.B2_A_src;
		self.B3_reg_trg  = self.B3_A_src;
		self.A2_reg_trg  = db_gain(self.A2_src);
		self.A3_reg_trg  = db_gain(self.A3_src);
		self.A4_reg_trg  = db_gain(self.A4_src);
		self.A5_reg_trg  = db_gain(self.A5_src);
		self.A6_reg_trg  = db_gain(self.A6_src);
		self.A7_reg_trg  = db_gain(self.A7_src);
		self.A8_reg_trg  = db_gain(self.A8_src);
		self.A9_reg_trg  = db_gain(self.A9_src);
		self.A10_reg_trg = db_gain(self.A10_src);
		self.AB_reg_trg  = db_gain(self.AB_src);
		self.AF_reg_trg  = db_gain(self.AF_src);
		self.AH_reg_trg  = db_gain(self.AH_src);
		self.AV_reg_trg  = db_gain(self.AV_src);
	}
	pub fn load(&mut self, phone: u8, dur_ms: i32, stressed: bool, seg: usize, word_start: bool) -> bool {
		// seg: the segment's stream index, reported back by position()
		self.advance(phone, dur_ms, stressed, seg, word_start);
		if self.is_ready() {
			// aspiration left by a voiceless stop plays in this phone's carried-over region
			self.asp_db = self.asp_db_next;
			self.asp_seg = self.asp_seg_next;
			self.asp_db_next = 0.0;
			self.after_stop = self.after_stop_next;
			self.after_stop_next = false;
			self.stop_on = false;
			self.stop_labial = false;
			self.f1_len = 0;
			let mut stop_asp = 0;

			// the previous phone's formant targets at its end, before fetch replaces them
			let prev_f = [self.F1_B_src, self.F2_B_src, self.F3_B_src];
			let prev_rd = self.RD_B_src;
			let (prev_b, prev_av) = ([self.B1_B_src, self.B2_B_src, self.B3_B_src], self.AV_src);

			self.fetch();

			// do formant target re-adjustments before doing anything else
			self.ftar();

			// short vowels fall short of their targets
			self.reduce();

			// consonants take their formants at each edge from the neighbouring vowel
			self.loci(prev_f[1]);

			// the phrase-end release is the last phone's voice dying away, so it keeps that voice's Rd
			if matches!(self.phn_c, 48 | 49) {
				(self.RD_A_src, self.RD_B_src) = (prev_rd, prev_rd);
				// at the end of a phrase an r or a nasal keeps its tongue in place while the voice dies away
				if matches!(self.phn_l1, 32 | 33 | 14 | 7) || self.pft_l1 & 0x10 != 0 {
					[self.F1_A_src, self.F2_A_src, self.F3_A_src] = prev_f;
					[self.F1_B_src, self.F2_B_src, self.F3_B_src] = prev_f;
				}
				// and a nasal its velum, damping and level
				if self.pft_l1 & 0x10 != 0 {
					[self.B1_A_src, self.B2_A_src, self.B3_A_src] = prev_b;
					[self.B1_B_src, self.B2_B_src, self.B3_B_src] = prev_b;
					self.AV_src = prev_av;
				}
			}

			// get duration markers so slopes can be calculated
			if self.phn_c == PAU {
				// pause plays out its whole duration
				self.marker_A = self.pdr_c.min(twenty_ms);
				self.marker_B = self.pdr_c;
			} else if self.pft_c & 0x8040 == 0 {
				// not plosive nor affricate
				if self.pft_c & 0x10 != 0 {
					// nasal: the formants arrive by the closure, the murmur holds
					self.marker_A = 0;
				} else if self.pft_c & 0x1C01 == 0 {
					// fricative transition
					if self.pft_r1 & 0x1C10 != 0 {
						// shorter before a sonorant consonant
						self.marker_A = twenty_ms;
					} else if self.pft_c & 0x200 != 0 {
						// length sibilant a bit
						self.marker_A = sixty_ms;
					} else {
						self.marker_A = forty_ms;
					}
				} else if self.pft_c & 0x1 != 0 {
					// vowel transition: quicker after a nasal's release, slower out of a tongue consonant into a back vowel
					let tongue = matches!(self.phn_l1, 11 | 36 | 27 | 34 | 43 | 47 | 35 | 44 | 10 | 22 | 17 | 23 | 28);
					let back = self.pft_l1 & 0x10 == 0 && tongue && self.F2_A_src < 1400.0;
					self.marker_A = if self.pft_l1 & 0x10 != 0 { samples(NASAL_RELEASE_MS) }
						else if back { samples(BACK_TRANSITION_MS) }
						else { fifty_ms };
					if back { self.f1_len = samples(BACK_F1_MS); }
				} else {
					// l, r, w and y: get transition from formant table
					self.marker_A = share_of(self.pdr_c, self.C_A_src);
				}
				self.marker_B = share_of(self.pdr_c, 100 - self.C_B_src);
				if self.pft_c & 0x1 != 0 {
					// short vowels get a shorter transition so they still reach their target
					self.marker_A = self.marker_A.min((ramp_share * self.marker_B as f32) as i32);
				}
			} else {
				// plosive or affricate: closure, then burst or frication;
				// a voiceless stop's aspiration is carried into the next phone
				let stop = stop_timeline(self.phn_c, self.pdr_c, self.phn_l1, self.phn_r1, self.pst_r1, self.pwi_c, self.pwi_r1);
				self.marker_A = stop.closure;
				self.marker_B = stop.closure + stop.burst;
				self.burst_len = stop.burst;
				self.stop_on = true;
				self.stop_bar = self.pft_c & 0x2 != 0 && self.pft_l1 & 0x2 != 0;
				self.stop_affric = self.pft_c & 0x8000 != 0;
				self.stop_labial = !self.stop_affric && stop_place(self.phn_c) == 0;
				self.burst_decay = if self.stop_affric { AFFRIC_DECAY_DB } else { BURST_DECAY_DB };
				let unstressed = self.pft_r1 & 0x1 != 0 && !self.pst_r1;
				let sibilant = (self.pft_l1 | self.pft_r1) & 0x200 != 0;
				// a voiced stop or a t before a liquid or glide keeps its release ("gloves", "bridge", "tree")
				let follow = if (self.pft_c & 0x2 != 0 || self.phn_c == 36) && self.pft_r1 & 0x1 == 0 { 0.0 } else { -RELEASE_FOLLOW * self.quiet_next_db() };
				let gain_db = if self.stop_affric { 0.0 } else {
					BURST_GAIN_DB + if unstressed { BURST_UNSTRESSED_DB } else { 0.0 } + if sibilant { BURST_SIBILANT_DB } else { 0.0 } + follow
				};
				self.burst_gain = libm::exp2f(gain_db / DB_PER_HALF);
				self.after_stop_next = stop.burst > 0;
				stop_asp = stop.asp;
			}
			if self.is_first {
				self.marker_BA = share_of(self.pdr_c, self.C_B_src);
				self.is_first = false;
				self.load_registers();
			} else {
				self.update_B_to_Next();
				self.marker_BA = share_of(self.pdr_c, self.C_B_src);
			}
			if self.pft_c & 0x1 != 0 && self.marker_BA > samples(VOWEL_EXIT_MS) {
				// a long vowel keeps its own formants longer, the move into the next phone takes the same time
				self.marker_B += self.marker_BA - samples(VOWEL_EXIT_MS);
				self.marker_BA = samples(VOWEL_EXIT_MS);
			}
			if self.phn_c == PAU {
				// no transition carried into the next phone, so the pause keeps its length
				self.marker_BA = 0;
			}
			if self.stop_on {
				self.marker_BA = stop_asp;
				if stop_asp > 0 {
					let mut level = ASP_DB;
					// softer before a front, close vowel, or one quiet above F2
					let front = if self.pft_r1 & 0x1 != 0 {
						let [f1, f2, _] = self.next_vowel_formants();
						asp_front_cut(f1, f2)
					} else { 0.0 };
					level -= front.max(if self.phn_c == 36 && self.pft_r1 & 0x1 == 0 { 0.0 } else { ASP_FOLLOW * self.quiet_next_db() });
					if schwa_t(self.phn_c, self.pwi_c, self.phn_r1, self.pst_r1) { level -= SCHWA_T_ASP_DB; }
					self.asp_db_next = level.max(1.0);
					self.asp_seg_next = self.seg_c;
				}
			}
			// a consonant's formants move to their B (right-side) targets early enough to arrive by its end
			self.ab_at = if self.pft_c & 0x1 == 0 && self.phn_c != PAU { self.marker_A.min(self.marker_B - thirty_ms).max(0) } else { self.marker_A };
			let glides = (self.F1_B_src - self.F1_A_src).abs() >= GLIDE_HZ || (self.F2_B_src - self.F2_A_src).abs() >= GLIDE_HZ;
			if self.f1_len > 0 && glides {
				self.ab_at = self.marker_A.min(fifty_ms);
			}
			// step() counts before it checks, so a move due at the phone's first sample starts at its second; with A and B the
			// same (a nasal after silence) there is nothing to move, and starting one would cut short its glide into A
			if self.ab_at <= self.counter {
				let a = [self.F1_A_src, self.F2_A_src, self.F3_A_src, self.B1_A_src, self.B2_A_src, self.B3_A_src, self.RD_A_src];
				let b = [self.F1_B_src, self.F2_B_src, self.F3_B_src, self.B1_B_src, self.B2_B_src, self.B3_B_src, self.RD_B_src];
				self.ab_at = if a == b { i32::MIN } else { self.counter + 1 };
			}
			// sources switch quickly at obstruent edges; formants keep their own transition
			fn obstruent(f: u16) -> bool { f != 0 && f & 0x1 == 0 && f & 0x1C10 == 0 }
			self.amp_len = if obstruent(self.pft_c) || obstruent(self.pft_l1) { self.marker_A.min(fifteen_ms) } else { self.marker_A };
			self.seg_start = self.counter;
			if self.after_stop {
				// voicing builds up quickly after a release
				self.amp_len = self.amp_len.min(samples(VOICE_ONSET_MS));
			}
			fn nasal(f: u16) -> bool { f & 0x10 != 0 }
			if nasal(self.pft_c) || nasal(self.pft_l1) {
				// a nasal's closure and release are quick
				self.amp_len = samples(NASAL_EDGE_MS);
			}
			if self.phn_c == 18 || self.phn_l1 == 18 {
				self.amp_len = samples(H_EDGE_MS);
			}
			self.amp_start = 0;
			self.voice_len = 0;
			fn sonorant(f: u16) -> bool { f & 0x1 != 0 || f & 0x1C10 != 0 }
			fn quiet(p: u8) -> bool { p == PAU || p == 0 || p == 48 || p == 49 }
			self.dip_half = 0;
			// h is breath noise too: without the dip a fricative runs into it as one hiss
			fn hiss(p: u8) -> bool { fricative(p) || p == 18 }
			if hiss(self.phn_c) && fricative(self.phn_l1) && self.phn_c != self.phn_l1 {
				self.dip_mid = self.amp_len / 2;
				self.dip_half = samples(FRIC_DIP_MS / 2.0).min(self.dip_mid - self.counter);
				self.dip_db = FRIC_DIP_DB;
			}
			if nasal(self.pft_c) && nasal(self.pft_l1) && self.phn_c != self.phn_l1 {
				// centred on the join, or starting at this phone's first sample when less of the last one carries over
				let half = samples(NASAL_DIP_MS / 2.0);
				self.dip_mid = (self.counter + half).max(0);
				self.dip_half = half;
				self.dip_db = NASAL_DIP_DB;
			}
			if self.pft_c & 0x1 != 0 && self.pwi_c && self.pft_l1 & 0x1 != 0 && !quiet(self.phn_l1)
				&& (self.pst_c || !matches!(self.phn_l1, 7 | 14)) {
				self.dip_mid = 0;
				self.dip_half = samples(GLOTTAL_DIP_MS / 2.0).min(-self.counter);
				self.dip_db = if self.pst_c { GLOTTAL_DIP_DB } else { GLOTTAL_DIP_UNSTRESSED_DB };
			}
			// a word-final voiced fricative before a voiceless obstruent devoices ("thieves fished")
			self.devoice_len = if fricative(self.phn_c) && self.pft_c & 0x2 != 0 && self.pwi_r1 && self.pft_r1 & 0x2 == 0
				&& (fricative(self.phn_r1) || self.pft_r1 & 0x40 != 0) { self.pdr_c } else { 0 };
			self.gap = if nasal(self.pft_l1) && fricative(self.phn_c) && self.pft_c & 0x2 == 0 { samples(NASAL_GAP_MS) } else { 0 };
			if (fricative(self.phn_c) && sonorant(self.pft_l1)) || (fricative(self.phn_l1) && sonorant(self.pft_c)) {
				// a fricative fades in and out across its boundary, as far as the carried-over region allows
				let sib = sibilant(self.pft_c) || sibilant(self.pft_l1);
				let entry = fricative(self.phn_c);
				let len = match (sib, entry) {
					(true, true) => SIB_ENTRY_MS,
					(true, false) => SIB_RAMP_MS,
					(false, true) => FRIC_ENTRY_MS,
					(false, false) => FRIC_RAMP_MS,
				};
				let half = samples(len / 2.0);
				self.amp_start = (-half).max(self.counter);
				self.amp_len = half - self.amp_start;
				let hv = samples(VOICE_SWITCH_MS / 2.0);
				self.voice_start = (-hv).max(self.counter);
				self.voice_len = hv - self.voice_start;
			} else if (fricative(self.phn_c) && quiet(self.phn_l1)) || (fricative(self.phn_l1) && quiet(self.phn_c)) {
				// out of and into silence, fricative noise rises and falls slowly
				self.amp_len = samples(FRIC_SILENCE_MS);
			}
			if self.asp_db > 0.0 {
				// aspiration: no voicing or frication until this phone's own part starts
				self.AF_reg_src = 0.0;
				self.AV_reg_src = 0.0;
				self.AH_reg_src = db_gain(self.asp_db);
				self.asp_start = self.counter;
				self.seg_start = 0;
			}
			self.asp_fade = samples(ASP_OVERLAP_MS);
			if self.asp_db > 0.0 && matches!(self.phn_c, 24 | 32) && !self.pwi_c {
				let span = (LIQUID_DEVOICE * self.marker_B as f32) as i32;
				if span > 0 {
					self.voice_start = 0;
					self.voice_len = span;
					self.asp_fade = span.max(self.asp_fade);
				}
			}
		}
		self.is_ready()
	}
	pub fn position(&self) -> (usize, f32) {
		// the playing segment and how far through it we are (0..1)
		// the aspiration still belongs to the stop
		if self.asp_db > 0.0 && self.counter < 0 { return (self.asp_seg, 1.0); }
		let len = self.marker_B - self.seg_start;
		let frac = if len > 0 { (self.counter - self.seg_start) as f32 / len as f32 } else { 1.0 };
		(self.seg_c, frac.clamp(0.0, 1.0))
	}
	fn step(&mut self) {
		self.counter += 1;
		self.update_amp_registers();
		if self.counter == self.ab_at {
			// the move to the B targets: at marker A, or earlier for a consonant
			self.update_A_to_B();
		}
		if self.counter >= 0 && self.counter < self.marker_A {
			// the glottalizer lowers the voicing into a voiceless phone
			if self.pft_c & 0x2 == 0 && self.GL_V > 0.75 {
				self.GL_V -= glot_time;
			}
		} else if self.counter > self.marker_A && self.counter < self.marker_B {
			// and brings it back in a voiced one
			if self.pft_c & 0x2 != 0 && self.GL_V < 1.0 {
				self.GL_V = (self.GL_V + glot_time).min(1.0);
			}

			// phrase-final fade out in AXP or IXP
			if self.phn_c == 48 || self.phn_c == 49 {
				if self.master_fade > 0.0 {
					self.master_fade = (self.master_fade - end_time).max(0.0);
				}
			} else if self.master_fade < 1.0 {
				self.master_fade = (self.master_fade + optime).min(1.0);
			}
		}
		self.update_registers()
	}
	pub fn sample(&mut self, f0: f32) -> f32 {
		self.step();

		// aspiration: the open glottis damps F1 and F2, less so as voicing takes over
		let damp = if self.asp_db <= 0.0 { 0.0 } else if self.counter < 0 { 1.0 } else {
			(1.0 - self.counter as f32 / self.asp_fade.max(1) as f32).max(0.0)
		};

		// a lip release: low and falling whatever the vowel
		let ramp = samples(LIP_RAMP_MS);
		let lips = self.stop_on && self.stop_labial && self.burst_len > 0 && self.counter >= self.marker_A - ramp;
		let dir = if lips { 1.0 } else { -1.0 };
		self.lip_hold = (self.lip_hold + dir / ramp as f32).clamp(0.0, 1.0);

		// as the voice dies away at a phrase end it gets breathier and laxer
		let breath = if matches!(self.phn_c, 48 | 49) {
			((self.counter - self.seg_start) as f32 / samples(BREATH_RAMP_MS) as f32).clamp(0.0, 1.0)
		} else { 0.0 };
		let bar = if self.stop_on && self.stop_bar && self.counter >= 0 && self.counter < self.marker_A {
			(self.counter as f32 / ten_ms as f32).min(1.0)
		} else { 0.0 };

		self.vtm_eng.F0  = f0;
		self.vtm_eng.F1  = self.F1_reg;
		self.vtm_eng.B1  = self.B1_reg + damp * ASP_B1_WIDEN;
		self.vtm_eng.B2  = self.B2_reg + damp * ASP_B2_WIDEN + self.lip_hold * LABIAL_BURST_WIDEN;
		self.vtm_eng.B3  = self.B3_reg + self.lip_hold * LABIAL_BURST_WIDEN;
		self.vtm_eng.F2  = self.F2_reg + self.lip_hold * (self.F2_reg.min(LABIAL_BURST_F2) - self.F2_reg);
		self.vtm_eng.F3  = self.F3_reg + self.lip_hold * (self.F3_reg.min(LABIAL_BURST_F3) - self.F3_reg);
		self.vtm_eng.RD  = self.RD_reg + BREATH_END_RD * breath + VOICE_BAR_RD * bar;
		self.vtm_eng.BR  = BREATH_DB + BREATH_END_DB * breath;
		self.vtm_eng.NC  = self.NZP_reg;
		self.vtm_eng.A2  = self.A2_reg;
		self.vtm_eng.A3  = self.A3_reg;
		self.vtm_eng.A4  = self.A4_reg;
		self.vtm_eng.A5  = self.A5_reg;
		self.vtm_eng.A6  = self.A6_reg;
		self.vtm_eng.A7  = self.A7_reg;
		self.vtm_eng.A8  = self.A8_reg;
		self.vtm_eng.A9  = self.A9_reg;
		self.vtm_eng.A10 = self.A10_reg;
		self.vtm_eng.AB  = self.AB_reg;
		self.vtm_eng.AF  = self.AF_reg;
		self.vtm_eng.AH  = self.AH_reg;
		self.vtm_eng.AV  = fade(self.AV_reg, self.GL_V);
		self.vtm_eng.step()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use super::samples as ms;

	// a register's gain back in dB, for the checks written in dB
	fn db(g: f32) -> f32 { if g <= 0.0 { 0.0 } else { G_MAX + DB_PER_HALF * g.log2() } }

	// render phones (id, ms, stressed) as segments 0, 1, ..., handing each the engine and the sample it made
	fn drive(phones: &[(u8, i32, bool)], mut each: impl FnMut(&engine, f32)) {
		let mut e = engine::new();
		for (seg, &(p, d, st)) in phones.iter().enumerate() {
			if e.load(p, d, st, seg, true) {
				while e.is_running() { let y = e.sample(120.0); each(&e, y); }
			}
		}
	}

	// the engine as phone `at` loads, after rendering the phones (id, ms, stressed) before it
	fn loaded(phones: &[(u8, i32, bool)], at: u8) -> engine {
		let mut e = engine::new();
		for &(p, d, st) in phones {
			if e.load(p, d, st, 0, true) {
				if e.phn_c == at { break; }
				while e.is_running() { e.sample(120.0); }
			}
		}
		e
	}

	// render phones (id, ms) the way lib.rs drives the engine
	fn render(phones: &[(u8, i32)]) -> Vec<f32> {
		let mut e = engine::new();
		let mut out: Vec<f32> = Vec::new();
		for &(p, d) in phones {
			if e.load(p, d, true, 0, true) {
				while e.is_running() { out.push(e.sample(120.0)); }
			}
		}
		let mut first = true;
		while e.is_not_last() {
			let ready = if first { first = false; e.load(48, 100, false, 0, true) } else { e.load(0, 50, false, 0, true) };
			if ready {
				while e.is_running() { out.push(e.sample(120.0)); }
			}
		}
		out
	}

	// formant targets (F1-F3 at A) of phone p lasting d ms, between two AA's
	fn targets_for(p: u8, d: i32) -> [f32; 3] {
		let e = loaded(&[(1, 150, true), (p, d, true), (1, 150, true)], p);
		[e.F1_A_src, e.F2_A_src, e.F3_A_src]
	}

	#[test]
	fn reduce_k_grows() {
		assert!((reduce_k(60.0) - 0.632).abs() < 0.001);
		assert!((reduce_k(90.0) - 0.777).abs() < 0.001);
		assert!((reduce_k(195.0) - 0.961).abs() < 0.001);
		assert!(reduce_k(50.0) < reduce_k(51.0));
	}

	#[test]
	fn long_vowel_keeps_target() {
		let long = targets_for(1, 1000);
		let norm = targets_for(1, 150);
		for k in 0..3 { assert!((norm[k] - long[k]).abs() <= 0.05 * long[k], "F{} {} vs {}", k + 1, norm[k], long[k]); }
	}

	#[test]
	fn short_vowel_moves_to_neutral() {
		let long = targets_for(21, 1000);
		let short = targets_for(21, 50);
		let n_f2 = formant_tbl[6 * 31 + 6] as f32;
		assert!(short[1] < long[1] && short[1] > n_f2, "IY F2 {} not between {} and {}", short[1], n_f2, long[1]);
	}

	#[test]
	fn position_advances() {
		// the reported segment follows the loaded order, and each runs to its end
		let mut e = engine::new();
		let mut seen: Vec<(usize, f32)> = Vec::new();
		for &(p, d, seg) in &[(1u8, 150, 7usize), (21, 150, 8), (1, 150, 9), (48, 100, 10), (0, 50, 11)] {
			if e.load(p, d, false, seg, true) {
				while e.is_running() {
					e.sample(120.0);
					seen.push(e.position());
				}
			}
		}
		let mut order: Vec<usize> = seen.iter().map(|x| x.0).collect();
		order.dedup();
		assert_eq!(order, vec![7, 8, 9, 10]);
		for w in seen.windows(2) {
			if w[0].0 == w[1].0 { assert!(w[1].1 >= w[0].1); } else { assert!(w[0].1 >= 0.95, "seg {} ended at {}", w[0].0, w[0].1); }
		}
	}

	#[test]
	fn faders_scale_linear_gain() {
		// a fader of 0.5 halves the gain (-6 dB), it does not halve the dB value
		let fade_db = |a, m| db(fade(db_gain(a), m));
		assert!((fade_db(60.0, 0.5) - 54.0).abs() < 0.01);
		assert!((fade_db(60.0, 1.0) - 60.0).abs() < 0.01);
		assert_eq!(fade_db(60.0, 0.0), 0.0);
		assert_eq!(fade_db(0.0, 0.5), 0.0);
		assert_eq!(fade_db(3.0, 0.01), 0.0);
	}

	#[test]
	fn amp_ramp_is_linear_in_gain() {
		// halfway from silence to 60 dB is half the gain (54 dB), not 30 dB
		let lerp_db_gain = |a, b, y| db(lerp_gain(db_gain(a), db_gain(b), y));
		assert!((lerp_db_gain(0.0, 60.0, 0.5) - 54.0).abs() < 0.01);
		assert_eq!(lerp_db_gain(0.0, 60.0, 0.0), 0.0);
		assert!((lerp_db_gain(0.0, 60.0, 1.0) - 60.0).abs() < 0.01);
		assert!((lerp_db_gain(50.0, 50.0, 0.3) - 50.0).abs() < 0.01);
	}

	#[test]
	fn ramp_scales() {
		// a short vowel still reaches its target before its own segment ends
		let e = loaded(&[(1, 150, false), (6, 60, false), (1, 150, false)], 6);
		assert_eq!(e.phn_c, 6);
		assert!(e.marker_A < e.marker_B, "ramp {} ends after the vowel {}", e.marker_A, e.marker_B);
	}

	#[test]
	fn pause_first_is_finite() {
		// a pause as the very first phone has no formants to hold yet
		let out = render(&[(PAU, 50), (1, 150)]);
		assert!(out.iter().all(|x| x.is_finite()));
	}

	#[test]
	fn pause_plays_its_duration() {
		let plain = render(&[(1, 150), (1, 150)]);
		let paused = render(&[(1, 150), (PAU, 300), (1, 150)]);
		let diff = paused.len() as i64 - plain.len() as i64;
		assert!((diff - ms(300.0) as i64).abs() <= ms(25.0) as i64, "pause added {} samples", diff);

		// the pause is a run of silence
		let mut run = 0;
		let mut best = 0;
		for x in &paused {
			if x.abs() < 1e-3 { run += 1; if run > best { best = run; } } else { run = 0; }
		}
		assert!(best >= ms(250.0), "longest silent run {} samples", best);
	}


	#[test]
	fn stop_timeline_fits() {
		for &p in &[31u8, 36, 23] {
			for &d in &[60.0, 88.0, 101.0, 103.0, 150.0] {
				let pdr = ms(d);
				let t = stop_timeline(p, pdr, 1, 1, true, true, false);
				assert!((t.closure + t.burst + t.asp - pdr).abs() <= 1, "{} at {} ms: {:?}", p, d, t);
				assert!(t.closure >= min_closure, "{} at {} ms: closure {}", p, d, t.closure);
			}
		}
		// too short for the full release: burst and aspiration shrink together
		let short = stop_timeline(23, ms(60.0), 1, 1, true, true, false);
		let full = stop_timeline(23, ms(150.0), 1, 1, true, true, false);
		assert!(short.burst < full.burst && short.asp < full.asp);
		assert!((short.burst as f32 / short.asp as f32 - full.burst as f32 / full.asp as f32).abs() < 0.02);
	}

	#[test]
	fn aspiration_by_context() {
		let pdr = ms(101.0);
		let t = |prev: u8, next: u8, stressed: bool, w: bool| stop_timeline(36, pdr, prev, next, stressed, w, false);
		let base = t(1, 1, true, true);
		assert!((base.asp - ms(55.0)).abs() <= 1);
		assert_eq!(t(34, 1, true, false).asp, 0, "after s in the word");
		assert_eq!(t(35, 1, true, false).asp, 0, "after sh in the word");
		assert!((t(34, 1, true, true).asp - ms(55.0)).abs() <= 1, "after s across a word start");
		assert!((t(1, 1, false, true).asp - ms(33.0)).abs() <= 1, "before an unstressed vowel");
		assert!((t(3, 6, false, false).asp - ms(33.0 * SCHWA_T_ASP)).abs() <= 1, "inside a word before a schwa");
		assert!((t(1, 6, false, true).asp - ms(33.0)).abs() <= 1, "starting a word before a schwa");
		assert!((t(1, 24, true, true).asp - ms(55.0 * ASP_BEFORE_SONORANT)).abs() <= 1, "before l");
		assert_eq!(t(1, 34, true, true).asp, 0, "before s");
		assert_eq!(t(1, 48, false, true).asp, 0, "phrase-final");
		for &n in &[36u8, 27] {
			let s = t(1, n, true, true);
			// unreleased, and its closure merges with the next one
			assert_eq!((s.closure, s.burst, s.asp), ((pdr as f32 * UNRELEASED_SHARE) as i32, 0, 0), "unreleased before {}", n);
		}
		// a word-final t before a consonant is unreleased ("not that"), but not before a vowel or at a phrase end
		let unreleased = (pdr as f32 * UNRELEASED_SHARE) as i32;
		assert_eq!(stop_timeline(36, pdr, 1, 12, true, false, true).burst, 0, "not that");
		assert_eq!(stop_timeline(36, pdr, 1, 12, true, false, true).closure, unreleased);
		assert!(stop_timeline(36, pdr, 1, 1, true, false, true).burst > 0, "before a vowel");
		assert!(stop_timeline(36, pdr, 1, 49, false, false, true).burst > 0, "phrase-final");
		// a lip stop, k or g before a word starting with th or dh is released without aspiration; t and d are unreleased
		let keep_the = stop_timeline(31, pdr, 21, 12, true, false, true);
		assert!(keep_the.burst > 0 && keep_the.asp == 0, "keep the: {:?}", keep_the);
		let take_thick = stop_timeline(23, pdr, 15, 37, true, false, true);
		assert!(take_thick.burst > 0 && take_thick.asp == 0, "take thick: {:?}", take_thick);
		assert_eq!(stop_timeline(11, pdr, 1, 37, true, false, true).burst, 0, "had thick");
		// k and g keep that burst before a word starting with a nasal, not inside a word or before another stop
		let unique_new = stop_timeline(23, pdr, 21, 27, true, false, true);
		assert!(unique_new.burst > 0 && unique_new.asp == 0, "unique New: {:?}", unique_new);
		assert_eq!(stop_timeline(23, pdr, 2, 27, false, false, false).burst, 0, "acne");
		assert_eq!(stop_timeline(23, pdr, 21, 36, true, false, true).burst, 0, "unique town");
		assert!(stop_timeline(31, pdr, 21, 34, true, false, true).burst > 0, "keep sitting");
		assert!(stop_timeline(31, pdr, 21, 12, true, false, false).burst > 0, "inside a word");
		// a shorter aspiration gives its time to the closure: the stop keeps its length
		for &(prev, next, st, w) in &[(34u8, 1u8, true, false), (35, 1, true, false), (1, 1, false, true), (1, 24, true, true), (1, 34, true, true), (1, 48, false, true)] {
			let s = t(prev, next, st, w);
			assert_eq!(s.closure, base.closure + base.asp - s.asp);
		}
	}

	#[test]
	fn back_vowels_keep_their_f2_before_a_velar() {
		let f2 = |v: u8, c: u8| loaded(&[(PAU, 50, false), (18, 80, false), (v, 200, true), (c, 80, false), (6, 100, false)], v).F2_B_src;
		assert!((f2(1, 23) - f2(1, 18)).abs() < 1.0, "aa ends at F2 {} before k, {} before h", f2(1, 23), f2(1, 18));
		assert!(f2(21, 23) > f2(21, 18) + 10.0, "iy ends at F2 {} before k, {} before h", f2(21, 23), f2(21, 18));
	}

	#[test]
	fn a_voiced_stop_keeps_its_release_before_a_liquid() {
		let gain = |s: u8, n: u8| loaded(&[(PAU, 50, false), (6, 100, false), (s, 80, false), (n, 200, true), (6, 100, false)], s).burst_gain;
		assert!((gain(17, 24) - gain(17, 1)).abs() < 1e-4, "g: {} before l, {} before aa", gain(17, 24), gain(17, 1));
		assert!(gain(23, 24) < gain(23, 1) * 0.9, "k: {} before l, {} before aa", gain(23, 24), gain(23, 1));
	}

	#[test]
	fn f2_falls_toward_a_lip_consonant() {
		for p in [9u8, 31, 26] {
			let (slope, offset, _) = locus(p).unwrap();
			for v in [900.0, 1250.0, 2300.0] { assert!(offset + slope * v < v, "phone {} next to F2 {}", p, v); }
		}
	}

	// per-sample (phone, counter, AF, AH, AV, reported position) while rendering (phone, ms, seg)
	fn trace_regs(phones: &[(u8, i32, usize)]) -> (Vec<(u8, i32, f32, f32, f32, (usize, f32))>, Vec<(i32, i32, f32)>) {
		trace_regs_st(phones, &[])
	}

	// as trace_regs, with the segments listed in `unstressed` loaded unstressed
	fn trace_regs_st(phones: &[(u8, i32, usize)], unstressed: &[usize]) -> (Vec<(u8, i32, f32, f32, f32, (usize, f32))>, Vec<(i32, i32, f32)>) {
		let mut e = engine::new();
		let mut out = Vec::new();
		let mut marks = Vec::new();		// (marker_A, marker_B, AF_src) of each phone
		for &(p, d, seg) in phones {
			if e.load(p, d, !unstressed.contains(&seg), seg, true) {
				marks.push((e.marker_A, e.marker_B, e.AF_src));
				while e.is_running() {
					let pos = e.position();
					e.sample(120.0);
					out.push((e.phn_c, e.counter, db(e.AF_reg), db(e.AH_reg), db(e.AV_reg), pos));
				}
			}
		}
		(out, marks)
	}

	#[test]
	fn burst_onset_is_sharp() {
		let (out, marks) = trace_regs(&[(PAU, 50, 0), (1, 150, 1), (36, 101, 2), (1, 150, 3), (48, 100, 4), (0, 50, 5)]);
		let (a, b, level) = marks[2];
		let t: Vec<_> = out.iter().filter(|x| x.0 == 36 && x.1 >= 0).collect();
		for x in t.iter().filter(|x| x.1 < a) { assert_eq!(x.2, 0.0, "AF in the closure at {}", x.1); }
		let first = t.iter().find(|x| x.1 == a).unwrap();
		let level = level + BURST_GAIN_DB;
		assert!((first.2 - level).abs() < 0.5, "burst starts at {} not {}", first.2, level);
		let last = t.iter().find(|x| x.1 == b).unwrap();
		assert!((last.2 - (level - BURST_DECAY_DB)).abs() < 0.5, "burst ends at {} not {}", last.2, level - BURST_DECAY_DB);
	}

	#[test]
	fn aspiration_reports_stop_segment() {
		let (out, _) = trace_regs(&[(PAU, 50, 0), (1, 150, 1), (36, 101, 2), (1, 150, 3), (48, 100, 4), (0, 50, 5)]);
		let asp: Vec<_> = out.iter().filter(|x| x.0 == 1 && x.1 < 0 && x.5.0 != 1).collect();
		assert!(asp.len() as i32 >= ms(50.0), "aspiration {} samples", asp.len());
		for x in &asp {
			assert_eq!(x.5, (2, 1.0), "aspiration reports {:?}", x.5);
			assert_eq!(x.4, 0.0, "voicing during aspiration");
		}
		assert!(asp.iter().skip(ms(2.0) as usize).all(|x| x.3 > 0.0), "aspiration is silent");
		let first = out.iter().find(|x| x.5.0 == 3).unwrap();
		assert_eq!(first.5.1, 0.0, "vowel starts at frac {}", first.5.1);
	}

	#[test]
	fn voicing_onset_overlaps_aspiration() {
		// aspiration fades out under the first voicing pulses, and voicing builds up over them
		let (out, _) = trace_regs(&[(PAU, 50, 0), (1, 150, 1), (36, 101, 2), (1, 150, 3), (48, 100, 4), (0, 50, 5)]);
		let v: Vec<_> = out.iter().filter(|x| x.5.0 == 3).collect();
		let at = |m: f32| v.iter().find(|x| x.1 == ms(m)).unwrap();
		assert!(at(10.0).3 > 0.0, "aspiration gone 10 ms into voicing");
		assert!(at(VOICE_ONSET_MS + 2.0).4 > at(2.0).4 + 3.0, "voicing at full level within 2 ms");
	}

	#[test]
	fn voiced_closure_is_quiet() {
		for &(prev, bar) in &[(1u8, true), (34u8, false)] {
			let (out, marks) = trace_regs(&[(PAU, 50, 0), (prev, 150, 1), (11, 80, 2), (1, 150, 3), (48, 100, 4), (0, 50, 5)]);
			let (a, b, _) = marks[2];
			let cl: Vec<_> = out.iter().filter(|x| x.0 == 11 && x.1 >= ms(5.0) && x.1 < a).collect();
			assert!(cl.len() as i32 > ms(40.0), "closure {} samples", cl.len());
			for x in &cl { assert_eq!(x.2, 0.0, "AF in the closure after {}", prev); }
			let (first, last) = (cl.first().unwrap().4, cl.last().unwrap().4);
			if bar {
				// a voice bar after a vowel, fading over the closure
				assert!(first > 0.0 && last > 0.0 && first - last > 4.0, "voice bar {} -> {}", first, last);
			} else {
				assert!(cl.iter().all(|x| x.4 == 0.0), "voicing in the closure after {}", prev);
			}
			// a burst, then voicing builds up in the vowel
			assert!(out.iter().any(|x| x.0 == 11 && x.1 >= a && x.1 <= b && x.2 > 0.0), "no burst after {}", prev);
			let v: Vec<_> = out.iter().filter(|x| x.5.0 == 3).collect();
			let full = v.iter().find(|x| x.1 == ms(40.0)).unwrap().4;
			let at = v.iter().find(|x| x.1 == ms(VOICE_ONSET_MS + 2.0)).unwrap().4;
			assert!(full - at < 3.0, "voicing {} at onset+12 ms vs {} later", at, full);
		}
	}

	#[test]
	fn formants_start_and_stop_moving_gently() {
		// a formant that starts or stops moving at full speed in one sample clicks, most on the big F3 sweeps into and
		// out of an r, so its speed changes by only a little from one sample to the next
		for seq in [[(4u8, 132), (33, 122)], [(1, 150), (32, 90)], [(14, 140), (36, 100)]] {
			let (mut last, mut speed, mut worst) = (None::<[f32; 3]>, [0.0f32; 3], 0.0f32);
			let phones: Vec<_> = [(PAU, 50)].iter().chain(seq.iter()).chain([(1, 150), (48, 100), (0, 50)].iter()).map(|&(p, d)| (p, d, true)).collect();
			drive(&phones, |e, _| {
				let f = [e.F1_reg, e.F2_reg, e.F3_reg];
				if let Some(l) = last {
					for k in 0..3 {
						let s = f[k] - l[k];
						worst = worst.max((s - speed[k]).abs());
						speed[k] = s;
					}
				}
				last = Some(f);
			});
			assert!(worst < 0.1, "{:?}: a formant's speed changes {} Hz per sample in one sample", seq, worst);
		}
	}

	#[test]
	fn formants_do_not_jump_at_a_lip_release() {
		// a formant stepped in one sample makes a resonator click, so the lip release's low F2 and F3 come and go
		// smoothly, even where they are far from the stop's own (a b after ay and before r)
		for &(prev, stop, next) in &[(8u8, 9u8, 32u8), (21, 9, 21), (8, 31, 32), (26, 9, 24)] {
			let (mut f2, mut f3, mut worst) = (None::<f32>, 0.0f32, 0.0f32);
			drive(&[(PAU, 50, true), (prev, 120, true), (stop, 90, true), (next, 150, true), (48, 100, true), (0, 50, true)], |e, _| {
				let (a, b) = (e.vtm_eng.F2, e.vtm_eng.F3);
				if let Some(last) = f2 { worst = worst.max((a - last).abs()).max((b - f3).abs()); }
				(f2, f3) = (Some(a), b);
			});
			assert!(worst < 50.0, "{}-{}-{}: F2/F3 jump {} Hz in one sample", prev, stop, next, worst);
		}
	}

	#[test]
	fn a_lip_release_has_no_compact_peak() {
		let mut widest = 0.0f32;
		drive(&[(PAU, 50, true), (21, 120, true), (31, 90, true), (19, 150, true), (48, 100, true), (0, 50, true)], |e, _| {
			widest = widest.max(e.vtm_eng.B3 - e.B3_reg);
		});
		assert!((widest - LABIAL_BURST_WIDEN).abs() < 1.0, "B3 widened by {} Hz at most", widest);
	}

	#[test]
	fn voice_bar_is_muffled() {
		// the voice's Rd rises by VOICE_BAR_RD in a voiced closure and is back to the stop's own at the burst
		let mut rd = Vec::new();
		drive(&[(PAU, 50, true), (1, 150, true), (11, 80, true), (1, 150, true), (48, 100, true), (0, 50, true)], |e, _| {
			if e.phn_c == 11 && e.counter >= 0 { rd.push((e.counter, e.marker_A, e.vtm_eng.RD - e.RD_reg)); }
		});
		let at = |c: i32| rd.iter().find(|x| x.0 == c).unwrap();
		assert!((at(ms(15.0)).2 - VOICE_BAR_RD).abs() < 1e-4, "Rd +{} 15 ms into the closure", at(ms(15.0)).2);
		let burst = at(ms(15.0)).1;
		assert_eq!(at(burst).2, 0.0, "Rd still raised at the burst");
	}

	#[test]
	fn affricate_has_closure_then_frication() {
		for &(p, d, fric) in &[(10u8, 100, 55.0), (22u8, 90, 40.0)] {
			// released even before a nasal, with no aspiration
			for &next in &[1u8, 27] {
				let t = stop_timeline(p, ms(d as f32), 1, next, true, true, false);
				assert!((t.burst - ms(fric)).abs() <= 1 && t.asp == 0 && t.closure + t.burst == ms(d as f32), "{} before {}: {:?}", p, next, t);
			}
			let (out, marks) = trace_regs(&[(PAU, 50, 0), (1, 150, 1), (p, d, 2), (1, 150, 3), (48, 100, 4), (0, 50, 5)]);
			let (a, b, _) = marks[2];
			let seg: Vec<_> = out.iter().filter(|x| x.0 == p && x.1 >= 0).collect();
			assert!(seg.iter().filter(|x| x.1 < a).all(|x| x.2 == 0.0), "frication in the closure of {}", p);
			assert!(seg.iter().filter(|x| x.1 >= a && x.1 <= b).all(|x| x.2 > 0.0), "frication gaps in {}", p);
		}
	}

	#[test]
	fn nasal_is_softer_after_unstressed_vowel() {
		// AA-N-AA: the N's voicing is NASAL_AFTER_UNSTRESSED_DB lower when the AA before it is unstressed, unless the
		// N starts a word ("the moon")
		let av = |stressed: bool, word_start: bool| {
			let mut e = engine::new();
			let mut v = 0.0;
			for &(p, d, st, w) in &[(PAU, 50, true, true), (1, 150, stressed, true), (27, 100, true, word_start), (1, 150, true, false), (48, 100, true, true), (0, 50, true, true)] {
				if e.load(p, d, st, 0, w) {
					while e.is_running() { e.sample(120.0); if e.phn_c == 27 && e.counter == ms(50.0) { v = db(e.AV_reg); } }
				}
			}
			v
		};
		let (st, un) = (av(true, false), av(false, false));
		assert!((st - un - NASAL_AFTER_UNSTRESSED_DB).abs() < 0.5, "after stressed {} unstressed {}", st, un);
		let (st, un) = (av(true, true), av(false, true));
		assert!((st - un).abs() < 0.5, "a word's first nasal: after stressed {} unstressed {}", st, un);
	}

	#[test]
	fn burst_is_softer_before_unstressed_vowel() {
		let onset = |unstressed: &[usize]| {
			let (out, marks) = trace_regs_st(&[(PAU, 50, 0), (1, 150, 1), (36, 101, 2), (6, 80, 3), (48, 100, 4), (0, 50, 5)], unstressed);
			let a = marks[2].0;
			out.iter().find(|x| x.0 == 36 && x.1 == a).unwrap().2
		};
		let (st, un) = (onset(&[]), onset(&[3]));
		assert!((st - un + BURST_UNSTRESSED_DB).abs() < 0.5, "stressed {} unstressed {}", st, un);
	}

	// AF at the burst onset of the T in phones (T at segment 2)
	fn t_burst_af(phones: &[(u8, i32, usize)]) -> f32 {
		let (out, marks) = trace_regs(phones);
		let a = marks[2].0;
		out.iter().find(|x| x.0 == 36 && x.1 == a).unwrap().2
	}

	#[test]
	fn burst_is_softer_next_to_sibilant() {
		let tail = [(48u8, 100, 4usize), (0, 50, 5)];
		let with = |a: u8, b: u8| -> Vec<(u8, i32, usize)> { [(PAU, 50, 0), (a, 150, 1), (36, 101, 2), (b, 150, 3)].iter().chain(tail.iter()).cloned().collect() };
		let plain = t_burst_af(&with(1, 1));
		let after = t_burst_af(&with(34, 1));
		let before = t_burst_af(&with(1, 34));
		assert!((plain - after + BURST_SIBILANT_DB).abs() < 0.5, "after s: {} vs {}", after, plain);
		// before s: no aspiration either way, the burst only drops by the sibilant cut
		assert!((plain - before + BURST_SIBILANT_DB).abs() < 0.5, "before s: {} vs {}", before, plain);
	}

	#[test]
	fn fricative_ramps_are_centered() {
		// frication fades in across the vowel-to-fricative boundary over the entry crossfade,
		// and out across the fricative-to-vowel one over the shorter exit crossfade
		for &(f, entry, exit) in &[(34u8, SIB_ENTRY_MS, SIB_RAMP_MS), (16, FRIC_ENTRY_MS, FRIC_RAMP_MS)] {
			let (out, _) = trace_regs(&[(PAU, 50, 0), (1, 150, 1), (f, 150, 2), (1, 150, 3), (48, 100, 4), (0, 50, 5)]);
			let (half, half_out) = (ms(entry / 2.0), ms(exit / 2.0));
			let af = |p: u8, c: i32| out.iter().find(|x| x.0 == p && x.1 == c).unwrap().2;
			let full = af(f, half + ms(10.0));
			assert_eq!(af(f, -half - ms(2.0)), 0.0, "{}: frication before the ramp", f);
			// the level is in dB, so a quiet fricative only clears 0 dB a few ms into the smooth ramp
			assert!(af(f, -half + ms(5.0)) > 0.0, "{}: frication not started {} ms before the boundary", f, entry / 2.0 - 5.0);
			let mid = af(f, 0);
			assert!(mid > 0.0 && mid < full - 3.0, "{}: frication at the boundary {} (full {})", f, mid, full);
			assert!((af(f, half) - full).abs() < 0.5, "{}: frication not full {} ms after the boundary", f, entry / 2.0);
			assert!((af(1, -half_out - ms(2.0)) - full).abs() < 0.5, "{}: frication fades before the exit ramp", f);
			let out_mid = af(1, 0);
			assert!(out_mid > 0.0 && out_mid < full - 3.0, "{}: frication at the exit boundary {}", f, out_mid);
			// the voicing holds until just before each boundary and switches within VOICE_SWITCH_MS
			let av = |p: u8, c: i32| out.iter().filter(|x| x.0 == p && x.1 == c).last().unwrap().4;
			let sw = ms(VOICE_SWITCH_MS);
			assert_eq!(av(f, -sw), av(f, -half - ms(2.0)), "{}: voicing fades before the switch", f);
			assert_eq!(av(f, sw), 0.0, "{}: voicing {} ms into the fricative", f, VOICE_SWITCH_MS);
			assert_eq!(av(1, -sw), 0.0, "{}: voicing before the switch into the vowel", f);
			assert_eq!(af(1, half_out + ms(1.0)), 0.0, "{}: frication after the exit ramp", f);
		}
	}

	#[test]
	fn different_fricatives_dip_between() {
		// S-SH dips at the switch so the two don't run together into one hiss; S-S runs on as one. The dip is
		// measured against the frication either side of it, which glides from one fricative's level to the other's
		for &(b, dips) in &[(35u8, true), (34, false)] {
			let (out, _) = trace_regs(&[(PAU, 50, 0), (1, 150, 1), (34, 120, 2), (b, 120, 3), (1, 150, 4), (48, 100, 5), (0, 50, 6)]);
			let k = (0..out.len()).filter(|&i| out[i].5.0 == 3 && out[i].1 < ms(30.0))
				.min_by(|&i, &j| out[i].2.partial_cmp(&out[j].2).unwrap()).unwrap();
			let w = ms(20.0) as usize;
			let (dip, before, after) = (out[k].2, out[k - w].2, out[k + w].2);
			let around = (before + after) / 2.0;
			if dips {
				// FRIC_DIP_DB below the glide, and a dip: well below both sides
				assert!(dip < around - FRIC_DIP_DB + 1.0 && dip < before.min(after) - FRIC_DIP_DB / 2.0,
					"S-SH dips to {} ({} before, {} after)", dip, before, after);
			} else {
				assert!(dip > around - 0.5, "S-S dips to {} ({})", dip, around);
			}
		}
	}

	#[test]
	fn sibilant_next_to_a_fricative_keeps_its_level() {
		// a short z after an unstressed vowel takes the short and unstressed cuts before m, not before f ("thieves fished")
		let af = |r: u8| loaded(&[(PAU, 50, false), (6, 100, false), (43, 70, false), (r, 70, false), (1, 150, true)], 43).AF_src;
		assert_eq!(af(16) - af(26), 6.0, "z before f {} dB, before m {} dB", af(16), af(26));
	}

	#[test]
	fn word_final_voiced_fricative_devoices_before_a_voiceless_one() {
		// IY-Z | F-AA ("these fish"): the z's voice fades out across it; before a vowel, or inside a word, it keeps it
		let av = |next: u8, next_word: bool| {
			let mut e = engine::new();
			let mut v = 0.0;
			for &(p, d, w) in &[(PAU, 50, true), (21, 150, true), (43, 90, false), (next, 100, next_word), (1, 150, false), (48, 100, true), (0, 50, true)] {
				if e.load(p, d, true, 0, w) {
					while e.is_running() { e.sample(120.0); if e.phn_c == 43 && e.counter == ms(55.0) { v = db(e.AV_reg); } }
				}
			}
			v
		};
		let (devoiced, in_word, vowel) = (av(16, true), av(16, false), av(1, true));
		assert!(devoiced < in_word - 6.0, "z before a word's f: voice {} dB, inside a word {} dB", devoiced, in_word);
		assert!((vowel - in_word).abs() < 3.0, "z before a vowel {} dB, inside a word {} dB", vowel, in_word);
	}

	#[test]
	fn different_nasals_dip_between() {
		// N-M dips NASAL_DIP_DB at the switch so the two don't run together into one hum; N-N runs on as one
		for &(b, dips) in &[(26u8, true), (27, false)] {
			let (out, _) = trace_regs(&[(PAU, 50, 0), (1, 150, 1), (27, 100, 2), (b, 100, 3), (1, 150, 4), (48, 100, 5), (0, 50, 6)]);
			let low = out.iter().filter(|x| x.5.0 == 3 && x.1 < ms(40.0)).map(|x| x.4).fold(f32::MAX, f32::min);
			let full = out.iter().find(|x| x.5.0 == 3 && x.1 == ms(60.0)).unwrap().4;
			if dips {
				assert!((full - low - NASAL_DIP_DB).abs() < 1.0, "N-M dips to {} from {}", low, full);
			} else {
				assert!(full - low < 0.5, "N-N dips to {} from {}", low, full);
			}
		}
	}

	#[test]
	fn voiceless_fricative_after_nasal_starts_closed() {
		// AA-N-S-AA: no noise for the first NASAL_GAP_MS of the S, then it comes in; AA-N-Z-AA runs straight on
		for &(f, closed) in &[(34u8, true), (43, false)] {
			let (out, _) = trace_regs(&[(PAU, 50, 0), (1, 150, 1), (27, 100, 2), (f, 150, 3), (1, 150, 4), (48, 100, 5), (0, 50, 6)]);
			let af = |c: i32| out.iter().find(|x| x.0 == f && x.1 == c).unwrap().2;
			let gap = ms(NASAL_GAP_MS);
			if closed {
				for c in 0..gap { assert_eq!(af(c), 0.0, "noise {} ms into the closure", c as f32 / ms(1.0) as f32); }
				assert!(af(gap + ms(3.0)) > 0.0, "no noise after the closure");
				// and it rises over NASAL_GAP_RISE_MS, not at once: 5 ms in it is still well under full
				let (early, full) = (af(gap + ms(5.0)), af(gap + ms(NASAL_GAP_RISE_MS + 5.0)));
				assert!(early < full - 6.0, "noise at {} 5 ms after the closure, {} once risen", early, full);
			} else {
				assert!(af(gap / 2) > 0.0, "N-Z has a closure");
			}
		}
	}

	#[test]
	fn unreleased_stop_stays_silent() {
		// AA-K-T-AA: no frication anywhere in the K or in the T's closure
		let (out, marks) = trace_regs(&[(PAU, 50, 0), (1, 150, 1), (23, 103, 2), (36, 101, 3), (1, 150, 4), (48, 100, 5), (0, 50, 6)]);
		let t_closure = marks[3].0;
		for x in out.iter().filter(|x| (x.0 == 23 && x.1 >= 0) || (x.0 == 36 && x.1 >= 0 && x.1 < t_closure)) {
			assert_eq!(x.2, 0.0, "AF {} in phone {} at {}", x.2, x.0, x.1);
		}
	}

	// mean power (dB) of x around f (+-100 Hz), Hann-windowed
	fn band_db(x: &[f32], f: f32) -> f32 {
		let n = x.len();
		let mut sum = 0.0f64;
		let mut k = 0;
		let mut g = f - 100.0;
		while g <= f + 100.0 {
			let (mut re, mut im) = (0.0f64, 0.0f64);
			for (i, v) in x.iter().enumerate() {
				let w = 0.5 - 0.5 * (2.0 * PI64 * i as f64 / n as f64).cos();
				let ph = 2.0 * PI64 * g as f64 * i as f64 / m_sr as f64;
				re += *v as f64 * w * ph.cos();
				im -= *v as f64 * w * ph.sin();
			}
			sum += re * re + im * im;
			k += 1;
			g += 10.0;
		}
		(10.0 * (sum / k as f64).log10()) as f32
	}
	const PI64: f64 = std::f64::consts::PI;

	#[test]
	fn nasal_murmur_keeps_its_upper_formants() {
		// AA-M-AA: the murmur's F2 region is not buried under a doubled low resonance
		let mut m: Vec<f32> = Vec::new();
		drive(&[(PAU, 50, true), (1, 150, true), (26, 120, true), (1, 150, true), (48, 100, true), (0, 50, true)], |e, y| {
			if e.phn_c == 26 && e.counter >= ms(40.0) && e.counter < ms(100.0) { m.push(y); }
		});
		let low = band_db(&m, 250.0);
		let f2 = band_db(&m, 1100.0);
		assert!(f2 - low > -35.0, "murmur F2 region {} dB under its low peak", low - f2);
	}

	#[test]
	fn a_nasal_after_a_pause_keeps_its_entry_glide() {
		// nothing carries over from a pause, so the n starts at counter 0 with its A and B targets the same: no A-to-B move
		// starts, which would restart its formants' glide from the silence's over the whole n instead of 30 ms
		let e = loaded(&[(PAU, 100, true), (27, 100, true), (2, 150, true)], 27);
		assert_eq!(e.ab_at, i32::MIN);
		let mut at_35ms = None;
		drive(&[(PAU, 100, true), (27, 100, true), (2, 150, true), (48, 100, true), (0, 50, true)], |e, _| {
			if e.phn_c == 27 && e.counter == samples(35.0) { at_35ms = Some((e.F2_reg, e.F2_A_src)); }
		});
		let (f2, target) = at_35ms.unwrap();
		assert!((f2 - target).abs() < 1.0, "n's F2 {} by 35 ms, target {}", f2, target);
	}

	#[test]
	fn diphthongs_stay_oral_next_to_a_nasal() {
		// the most nasal coupling a vowel gets while it plays before an N: some for AA, none for OY ("joined")
		let most = |v: u8| {
			let mut k = 0.0f32;
			drive(&[(PAU, 50, true), (22, 80, true), (v, 250, true), (27, 100, true), (48, 100, true), (0, 50, true)], |e, _| {
				if e.phn_c == v { k = k.max(e.vtm_eng.NC); }
			});
			k
		};
		assert!(most(1) > NASAL_VOWEL_K / 2.0, "AA before N: {}", most(1));
		assert_eq!(most(30), 0.0, "OY before N");
	}

	#[test]
	fn nasal_edges_are_fast() {
		// AA-M-AA: the nasal coupling and the voicing level switch within NASAL_EDGE_MS of each boundary
		let mut tr: Vec<(u8, i32, f32, f32)> = Vec::new();		// (phone, counter, coupling, AV)
		let mut f1: Vec<(u8, i32, f32, f32)> = Vec::new();		// (phone, counter, F1, the phone's F1 target)
		drive(&[(PAU, 50, true), (1, 150, true), (26, 120, true), (1, 150, true), (48, 100, true), (0, 50, true)], |e, _| {
			tr.push((e.phn_c, e.counter, e.vtm_eng.NC, db(e.AV_reg)));
			f1.push((e.phn_c, e.counter, e.F1_reg, e.F1_A_src));
		});
		let at = |p: u8, c: i32| *tr.iter().find(|x| x.0 == p && x.1 == c).unwrap();
		let edge = ms(NASAL_EDGE_MS);
		assert!(at(26, edge).2 > 0.99, "coupling {} at {} ms into the nasal", at(26, edge).2, NASAL_EDGE_MS);
		// the vowel after the nasal: the later of the two AA samples at each counter value
		let at_v = |c: i32| *tr.iter().filter(|x| x.0 == 1 && x.1 == c).last().unwrap();
		assert!(at_v(edge).2 <= NASAL_VOWEL_K, "coupling {} at {} ms into the vowel", at_v(edge).2, NASAL_EDGE_MS);
		assert!(at_v(ms(NASAL_CARRY_MS)).2 < 0.01, "coupling {} at {} ms into the vowel", at_v(ms(NASAL_CARRY_MS)).2, NASAL_CARRY_MS);
		assert!((at_v(ms(40.0)).3 - at_v(edge).3).abs() < 1.0, "voicing {} at {} ms vs {} later", at_v(edge).3, NASAL_EDGE_MS, at_v(ms(40.0)).3);
		// the formants reach the nasal by its closure, and the vowel's after its release
		let m0 = f1.iter().find(|x| x.0 == 26 && x.1 == 1).unwrap();
		assert!((m0.2 - m0.3).abs() < 10.0, "F1 {} at the closure, nasal's {}", m0.2, m0.3);
		let v = f1.iter().filter(|x| x.0 == 1 && x.1 == ms(NASAL_RELEASE_MS)).last().unwrap();
		assert!((v.2 - v.3).abs() < 10.0, "F1 {} {} ms after the release, vowel's {}", v.2, NASAL_RELEASE_MS, v.3);
	}

	// (F2 at A, F2 at B, F1 at A) of phone c between vowels v1 and v2, all long
	fn cons_targets(v1: u8, c: u8, v2: u8) -> (f32, f32, f32) {
		let e = loaded(&[(PAU, 50, true), (v1, 1000, true), (c, 90, true), (v2, 1000, true)], c);
		(e.F2_A_src, e.F2_B_src, e.F1_A_src)
	}

	#[test]
	fn consonant_f2_follows_the_vowel() {
		// the consonant's F2 on each side moves with the neighbouring vowel's F2, by its place's slope; an alveolar
		// takes a back vowel's F2 as ALVEOLAR_F2_FLOOR
		for &c in &[9u8, 11, 17, 26, 27, 34] {
			let alveolar = matches!(c, 11 | 27 | 34);
			let f2 = |v: u8| { let x = formant_tbl[v as usize * 31 + 6] as f32; if alveolar { x.max(ALVEOLAR_F2_FLOOR) } else { x } };
			let (slope, offset, _) = locus(c).unwrap();
			let on = |v: f32| (offset + slope * v).max(if c == 17 { v + VELAR_F2_ABOVE } else { 0.0 });
			let (_, b_iy, _) = cons_targets(6, c, 21);
			let (_, b_uw, _) = cons_targets(6, c, 39);
			let want = on(f2(21)) - on(f2(39));
			assert!(((b_iy - b_uw) - want).abs() < 0.1 * want, "phone {}: F2 {} before iy, {} before uw, want a {} gap", c, b_iy, b_uw, want);
			let (a_iy, _, _) = cons_targets(21, c, 6);
			let (a_uw, _, _) = cons_targets(39, c, 6);
			assert!((a_iy - a_uw) > 0.5 * want, "phone {}: F2 {} after iy, {} after uw", c, a_iy, a_uw);
		}
	}

	#[test]
	fn h_takes_the_next_vowels_formants() {
		for &v in &[21u8, 1, 39] {
			let (a, b, f1) = cons_targets(6, 18, v);
			let (tf1, tf2) = (formant_tbl[v as usize * 31 + 5] as f32, formant_tbl[v as usize * 31 + 6] as f32);
			assert!((a - tf2).abs() < 0.03 * tf2 && (b - tf2).abs() < 0.03 * tf2, "h before {}: F2 {} {} vs {}", v, a, b, tf2);
			assert!((f1 - tf1).abs() < 0.05 * tf1, "h before {}: F1 {} vs {}", v, f1, tf1);
		}
	}

	#[test]
	fn consonant_reaches_its_right_side() {
		// the vowel after a consonant starts from the consonant's right-side F2
		for &c in &[9u8, 11, 17, 43, 40, 12, 31, 26] {
			let (mut last, mut target) = (0.0f32, 0.0f32);
			drive(&[(PAU, 50, true), (6, 120, true), (c, 90, true), (21, 200, true), (48, 100, true), (0, 50, true)], |e, _| {
				if e.phn_c == c { last = e.F2_reg; target = e.F2_B_src; }
			});
			assert!((last - target).abs() < 30.0, "phone {} ends at F2 {} not {}", c, last, target);
		}
	}

	// the burst samples (and AF/AH during them) of stop c, sentence-initially before vowel v
	fn burst_of(c: u8, v: u8) -> (Vec<f32>, Vec<(f32, f32)>) {
		let (mut x, mut amps) = (Vec::new(), Vec::new());
		drive(&[(PAU, 50, true), (c, 100, true), (v, 200, true), (48, 100, true), (0, 50, true)], |e, y| {
			if e.phn_c == c && e.counter >= e.marker_A { x.push(y); amps.push((db(e.AF_reg), db(e.AH_reg))); }
		});
		(x, amps)
	}

	#[test]
	fn th_hiss_is_flat_like_f() {
		// th's noise is duller than f's (about 12 dB more 1.7 kHz against 5.5 kHz, so fin and thin differ) but has no
		// strong low resonance of its own: within 15 dB of f (a 1.7 kHz resonance of its own put it 20 dB over)
		let mid = |c: u8| {
			let mut x = Vec::new();
			drive(&[(PAU, 50, true), (1, 150, true), (c, 150, true), (1, 150, true), (48, 100, true), (0, 50, true)], |e, y| {
				if e.phn_c == c && e.counter > ms(50.0) && e.counter < ms(100.0) { x.push(y); }
			});
			x
		};
		let tilt = |x: &[f32]| band_db(x, 1700.0) - band_db(x, 5500.0);
		let (th, f) = (tilt(&mid(37)), tilt(&mid(16)));
		assert!(th < f + 15.0, "th 1.7 kHz over 5.5 kHz by {} dB, f by {}", th, f);
	}

	#[test]
	fn er_f3_falls_after_th_not_after_f() {
		// "thirst" against "first": the tongue tip leaves the teeth, so er's F3 starts high after th; during f the
		// tongue is free and already r-shaped
		let fall = |c: u8| {
			let (mut start, mut mid) = (0.0, 0.0);
			drive(&[(PAU, 50, true), (c, 100, true), (14, 180, true), (34, 110, true), (48, 100, true), (0, 50, true)], |e, _| {
				if e.phn_c == 14 && e.counter == 0 { start = e.F3_reg; }
				if e.phn_c == 14 && e.counter == ms(90.0) { mid = e.F3_reg; }
			});
			start - mid
		};
		let (th, f) = (fall(37), fall(16));
		assert!(th > 400.0, "er's F3 starts only {} Hz over its middle after th", th);
		assert!(f.abs() < 150.0, "er's F3 starts {} Hz off its middle after f", f);
	}

	#[test]
	fn labial_burst_is_vocal_tract_noise() {
		for &c in &[31u8, 9] {
			let (x, amps) = burst_of(c, 1);
			assert!(!x.is_empty());
			assert!(amps.iter().all(|a| a.0 == 0.0), "frication noise in the labial burst of {}", c);
			assert!(amps.iter().any(|a| a.1 > 0.0), "no burst noise for {}", c);
			// low and falling: more energy near 1 kHz than near 3 kHz
			assert!(band_db(&x, 1000.0) > band_db(&x, 3000.0), "burst of {} is not falling", c);
		}
	}

	#[test]
	fn velar_burst_follows_the_vowel() {
		// the burst peaks near the velar F2: high before iy, mid before aa
		let peak = |x: &[f32]| (20..=70).map(|k| k as f32 * 50.0).max_by(|&a, &b| band_db(x, a).partial_cmp(&band_db(x, b)).unwrap()).unwrap();
		for &c in &[23u8, 17] {
			let (front, back) = (peak(&burst_of(c, 21).0), peak(&burst_of(c, 1).0));
			assert!(front > 2200.0 && back < 2000.0 && front > back + 500.0, "{} peaks at {} Hz before iy, {} before aa", c, front, back);
		}
	}

	#[test]
	fn velar_f3_follows_the_vowel() {
		// velar F3 sits near F2 before a front vowel and lower before a back one
		let f2 = |v: u8| formant_tbl[v as usize * 31 + 6] as f32;
		for &c in &[17u8, 23] {
			let mut f3 = Vec::new();
			for &v in &[21u8, 1] {
				let got = loaded(&[(PAU, 50, true), (6, 1000, true), (c, 90, true), (v, 1000, true)], c).F3_B_src;
				f3.push(got);
				let want = VELAR_F3.1 + VELAR_F3.0 * f2(v);
				assert!((got - want).abs() < 0.03 * want, "{} before {}: F3 {} not {}", c, v, got, want);
			}
			assert!(f3[0] > f3[1] + 400.0, "{}: F3 {} before iy, {} before aa", c, f3[0], f3[1]);
		}
	}

	#[test]
	fn ng_stays_apart_from_n() {
		// between back vowels ng's F2 sits well under n's, and its F3 just above its F2 (the velar pinch)
		let at = |c: u8| loaded(&[(PAU, 50, true), (3, 1000, true), (c, 90, true), (3, 1000, true)], c);
		let (ng, n) = (at(28), at(27));
		assert!(n.F2_A_src - ng.F2_A_src > 90.0, "ng F2 {} vs n {}", ng.F2_A_src, n.F2_A_src);
		assert!((ng.F3_A_src - ng.F2_A_src - NG_PINCH_HZ).abs() < 1.0, "ng F3 {} over F2 {}", ng.F3_A_src, ng.F2_A_src);
		assert!(n.F3_A_src - n.F2_A_src > 2.0 * NG_PINCH_HZ, "n F3 {} over F2 {}", n.F3_A_src, n.F2_A_src);
	}

	#[test]
	fn labial_burst_leaves_no_transient() {
		// the release and the vowel's first 40 ms peak no higher after the lips than after the tongue
		let peak = |c: u8| {
			let mut m = 0.0f32;
			drive(&[(PAU, 50, true), (c, 90, true), (8, 250, true), (48, 100, true), (0, 50, true)], |e, y| {
				if e.phn_c == c || (e.phn_c == 8 && e.counter < ms(40.0)) { m = m.max(y.abs()); }
			});
			m
		};
		let (b, d) = (peak(9), peak(11));
		assert!(b < 1.15 * d, "peak {} after b, {} after d", b, d);
	}

	#[test]
	fn b_release_is_no_louder_than_p() {
		// b's lip-release noise stays level with p's (within 1 dB), not well over it
		let noise = |c: u8| {
			let mut m = 0.0f32;
			drive(&[(PAU, 50, true), (1, 150, true), (c, 90, true), (8, 250, true), (48, 100, true), (0, 50, true)], |e, _| {
				if e.phn_c == c && e.counter >= e.marker_A { m = m.max(e.AH_reg); }
			});
			m
		};
		let (b, p) = (noise(9), noise(31));
		assert!(b > 0.0 && db(b) <= db(p) + 1.01, "release noise {} dB for b, {} dB for p", db(b), db(p));
	}

	#[test]
	fn aspiration_damps_f1() {
		// the open glottis damps F1 (and a little F2) during aspiration; the vowel's own part is back to normal
		let (mut asp, mut voiced) = (Vec::new(), Vec::new());
		drive(&[(PAU, 50, true), (23, 100, true), (1, 200, true), (48, 100, true), (0, 50, true)], |e, _| {
			let wide = (e.vtm_eng.B1 - e.B1_reg, e.vtm_eng.B2 - e.B2_reg);
			if e.phn_c == 1 && e.counter < 0 { asp.push(wide); }
			if e.phn_c == 1 && e.counter > ms(ASP_OVERLAP_MS + 5.0) { voiced.push(wide); }
		});
		assert!(!asp.is_empty());
		assert!(asp.iter().all(|w| w.0 >= ASP_B1_WIDEN - 30.0 && w.1 >= ASP_B2_WIDEN - 20.0), "aspiration not damped: {:?}", asp[0]);
		assert!(voiced.iter().all(|w| w.0 < ASP_B1_WIDEN / 2.0), "voiced vowel still damped: {:?}", voiced[0]);
	}

	// per sample (phone, counter, AF gain, AH dB) through phones
	fn noise_track(phones: &[(u8, i32)]) -> Vec<(u8, i32, f32, f32)> {
		let phones: Vec<_> = phones.iter().chain([(48u8, 100), (0, 50)].iter()).map(|&(p, d)| (p, d, true)).collect();
		let mut out = Vec::new();
		drive(&phones, |e, _| out.push((e.phn_c, e.counter, e.AF_reg, db(e.AH_reg))));
		out
	}

	// 10-90 % rise time (ms) of the AF gain into phone c, against its peak
	fn af_rise_ms(t: &[(u8, i32, f32, f32)], c: u8) -> f32 {
		let idx: Vec<usize> = t.iter().enumerate().filter(|x| x.1.0 == c).map(|x| x.0).collect();
		let hi = idx.iter().map(|&i| t[i].2).fold(0.0, f32::max);
		let top = idx.iter().cloned().find(|&i| t[i].2 >= 0.9 * hi).unwrap();
		let v: Vec<f32> = t[..=top].iter().map(|x| x.2).collect();
		let a = v.iter().rposition(|x| *x < 0.1 * hi).unwrap_or(0);
		(top - a) as f32 * 1000.0 / m_sr as f32
	}

	#[test]
	fn fricative_edges_are_gradual() {
		let th = noise_track(&[(PAU, 50), (1, 150), (37, 100), (1, 150)]);
		assert!(af_rise_ms(&th, 37) >= 18.0, "th rises in {} ms", af_rise_ms(&th, 37));
		let s = noise_track(&[(PAU, 100), (34, 110), (1, 150)]);
		assert!(af_rise_ms(&s, 34) >= 30.0, "s after silence rises in {} ms", af_rise_ms(&s, 34));
	}

	#[test]
	fn voiceless_fricative_breathes_into_its_vowels() {
		// th: f's own breath is too faint to show since f got brighter than th
		let t = noise_track(&[(PAU, 50), (1, 150), (37, 110), (1, 150)]);
		// the TH's carried-over region is the first vowel's tail; the second vowel starts breathy
		assert!(t.iter().any(|x| x.0 == 37 && x.1 < 0 && x.3 > 0.0), "no breath at the vowel's end");
		let after: Vec<_> = t.iter().filter(|x| x.0 == 1).collect();
		let start = after.iter().filter(|x| x.1 == ms(5.0)).last().unwrap();
		assert!(start.3 > 0.0, "no breath at the next vowel's start");
	}

	#[test]
	fn tongue_consonants_glide_slowly_into_back_vowels() {
		// out of d, F2 still moves 50 ms into a back vowel but has arrived in a front one (aa: ao and the other back
		// vowels now glide toward their B targets)
		let f2_at = |v: u8, at: f32| {
			let mut f = (0.0, 0.0);
			drive(&[(PAU, 50, true), (6, 100, true), (11, 80, true), (v, 250, true), (48, 100, true), (0, 50, true)], |e, _| {
				if e.phn_c == v && e.counter == ms(at) { f = (e.F2_reg, e.F2_A_src); }
			});
			f
		};
		let (back, back_t) = f2_at(1, 50.0);
		assert!((back - back_t).abs() > 30.0, "F2 {} already at {} 50 ms into aa", back, back_t);
		let (back, back_t) = f2_at(1, BACK_TRANSITION_MS + 1.0);
		assert!((back - back_t).abs() < 5.0, "F2 {} not at {} by {} ms", back, back_t, BACK_TRANSITION_MS);
		let (front, front_t) = f2_at(21, 51.0);
		assert!((front - front_t).abs() < 5.0, "F2 {} not at {} 50 ms into iy", front, front_t);
	}

	#[test]
	fn aspiration_is_softer_before_front_vowels() {
		// the aspiration a stop leaves for the next vowel drops with how front that vowel is
		let asp_before = |v: u8| loaded(&[(PAU, 50, true), (6, 100, true), (36, 101, true), (v, 1000, true)], 36).asp_db_next;
		let gap = |v: u8| formant_tbl[v as usize * 31 + 6] as f32 - formant_tbl[v as usize * 31 + 5] as f32;
		let want = ASP_FRONT_DB_PER_KHZ * (gap(21) - ASP_FRONT_FROM_HZ) / 1000.0;
		assert!(want > 3.0);
		assert!(((asp_before(1) - asp_before(21)) - want).abs() < 0.3, "cut {} before iy, want {}", asp_before(1) - asp_before(21), want);
		assert_eq!(asp_before(1), ASP_DB, "aspiration before aa changed");
		// and before a vowel quiet above F2, by ASP_FOLLOW of how much quieter it is than an open vowel
		let f = |v: u8, i: usize| formant_tbl[v as usize * 31 + i] as f32;
		let want = ASP_FOLLOW * -40.0 * (f(39, 5) * f(39, 6) / RELEASE_REF).log10();
		assert!(want > 3.0);
		assert!(((asp_before(1) - asp_before(39)) - want).abs() < 0.3, "cut {} before uw, want {}", asp_before(1) - asp_before(39), want);
	}

	// (F1 at A, F2 at A, F2 at B) of stressed vowel v between consonants c1 and c2
	fn vowel_targets(c1: u8, v: u8, c2: u8) -> (f32, f32, f32) {
		let e = loaded(&[(PAU, 50, false), (6, 100, false), (c1, 80, false), (v, 200, true), (c2, 80, false), (6, 100, false)], v);
		(e.F1_A_src, e.F2_A_src, e.F2_B_src)
	}

	#[test]
	fn velars_do_not_lower_front_vowels() {
		// eh keeps its height after a velar, so it stays apart from ae
		let (f1_b, _, _) = vowel_targets(9, 13, 9);
		let (f1_g, _, _) = vowel_targets(17, 13, 17);
		assert!((f1_g - f1_b).abs() < 5.0, "eh F1 {} after g, {} after b", f1_g, f1_b);
	}

	#[test]
	fn alveolars_front_uw_and_uh() {
		// d, l and an r before the vowel front uw and uh; b does not, and neither does a dark l after the vowel, which
		// would make F2 rise and fall back into it ("full")
		for v in [39u8, 38] {
			let (_, a_b, b_b) = vowel_targets(9, v, 9);
			for &(l, r) in &[(11u8, 11u8), (24, 24), (32, 11)] {
				let (_, a, b) = vowel_targets(l, v, r);
				assert!(a > a_b + 100.0 && a + b > a_b + b_b + 200.0, "{} not fronted next to {}/{}: {} {} vs {} {}", v, l, r, a, b, a_b, b_b);
			}
			let (_, a, b) = vowel_targets(16, v, 25);
			assert!(b <= a, "{} rises into lx: {} -> {}", v, a, b);
		}
	}

	#[test]
	fn f1_opens_into_a_back_vowel_before_f2_arrives() {
		// out of d, ao's F1 is at its target while F2 is still gliding
		let mut at = None;
		drive(&[(PAU, 50, true), (6, 100, true), (11, 80, true), (4, 250, true), (48, 100, true), (0, 50, true)], |e, _| {
			if e.phn_c == 4 && e.counter == ms(BACK_F1_MS + 5.0) { at = Some((e.F1_reg, e.F1_A_src, e.F2_reg, e.F2_A_src)); }
		});
		let (f1, f1_t, f2, f2_t) = at.unwrap();
		assert!((f1 - f1_t).abs() < 0.03 * f1_t, "F1 {} not at {}", f1, f1_t);
		assert!(f2 - f2_t > 50.0, "F2 {} already at {}", f2, f2_t);
	}

	#[test]
	fn h_breathes_like_aspiration() {
		// h's breath is no louder than a stop's aspiration, and softer before a front vowel
		let mid = |v: u8| {
			let t = noise_track(&[(PAU, 50), (18, 90), (v, 150)]);
			t.iter().filter(|x| x.0 == 18 && x.1 == ms(60.0)).last().unwrap().3
		};
		let (iy, uw) = (mid(21), mid(39));
		assert!(uw <= ASP_DB + 1.0, "h before uw at {} dB", uw);
		assert!(uw - iy > 5.0, "h before iy {} vs uw {}", iy, uw);
	}

	#[test]
	fn h_breath_ends_early_in_its_vowel() {
		let t = noise_track(&[(PAU, 50), (18, 90), (2, 150)]);
		let ah = t.iter().filter(|x| x.0 == 2 && x.1 == ms(H_EDGE_MS + 5.0)).last().unwrap().3;
		assert!(ah <= 0.0, "breath {} dB at {} ms into the vowel", ah, H_EDGE_MS + 5.0);
	}

	#[test]
	fn s_has_no_vowel_coloured_breath() {
		// breath through the cascade takes the vowels' formants and smears s; sh keeps it
		let ah = |c: u8| {
			let t = noise_track(&[(PAU, 50), (1, 150), (c, 110), (1, 150)]);
			t.iter().filter(|x| x.0 == c && x.1 == ms(50.0)).last().unwrap().3
		};
		assert!(ah(34) <= 0.0, "s breath {} dB", ah(34));
		assert!(ah(35) > 0.0, "sh lost its breath");
	}

	#[test]
	fn gliding_back_vowel_starts_its_glide_on_time() {
		// ow's own glide out of d keeps the timing it has out of b
		let f1 = |c: u8| {
			let mut f = 0.0;
			drive(&[(PAU, 50, true), (6, 100, true), (c, 80, true), (29, 250, true), (48, 100, true), (0, 50, true)], |e, _| {
				if e.phn_c == 29 && e.counter == ms(90.0) { f = e.F1_reg; }
			});
			f
		};
		let (b, d) = (f1(9), f1(11));
		assert!((b - d).abs() < 15.0, "ow F1 at 90 ms: {} after b, {} after d", b, d);
	}

	#[test]
	fn a_liquid_after_an_aspirated_stop_starts_voiceless() {
		// p l ey: in one word the l's voice comes in late; "stop | lay" keeps a voiced l
		let av = |word_start: bool| {
			let mut e = engine::new();
			let mut out = Vec::new();
			for &(p, d, w) in &[(PAU, 50, true), (31, 90, true), (24, 80, word_start), (15, 150, false), (PAU, 50, true)] {
				if e.load(p, d, true, 0, w) {
					while e.is_running() {
						e.sample(120.0);
						if e.phn_c == 24 && e.counter >= 0 { out.push(db(e.AV_reg)); }
					}
				}
			}
			(out[out.len() / 5], out[out.len() - 1])
		};
		let (early, late) = av(false);
		assert!(early < late - 10.0, "l after p in its word: AV {} early, {} late", early, late);
		let (early, late) = av(true);
		assert!(early > late - 3.0, "l starting a word: AV {} early, {} late", early, late);
	}

	#[test]
	fn vowels_are_not_pressed() {
		// vowel Rd was set by ear to about 0.6 (2026-10-31): tense enough for clear vowel colour, no more
		for &v in &[1u8, 2, 3, 4, 5, 6, 7, 8, 13, 14, 15, 19, 20, 21, 29, 30, 39] {
			let o = v as usize * 31;
			let (a, b) = (formant_tbl[o + 2], formant_tbl[o + 3]);
			assert!(a >= 55 && b >= 55, "vowel {} Rd {} / {}", v, a, b);
		}
	}

	#[test]
	fn r_colors_only_the_end_of_a_vowel() {
		// before r a vowel starts as before t and its F3 is RHOTIC_F3 lower only at its end, stressed or not
		let f3 = |v: u8, c2: u8, st: bool| {
			let e = loaded(&[(PAU, 50, false), (6, 100, false), (23, 80, false), (v, 200, st), (c2, 80, false), (6, 100, false)], v);
			(e.F3_A_src, e.F3_B_src)
		};
		for &v in &[1u8, 13, 21, 4] {
			for &st in &[true, false] {
				let ((ra, rb), (ta, tb)) = (f3(v, 33, st), f3(v, 36, st));
				assert!((ra - ta).abs() < 1.0, "vowel {} starts at F3 {} before r, {} before t", v, ra, ta);
				// reduction toward the neutral vowel scales the shift a little
				assert!((tb - rb - RHOTIC_F3 * reduce_k(200.0)).abs() < 1.0, "vowel {} ends at F3 {} before r, {} before t", v, rb, tb);
			}
		}
	}

	#[test]
	fn vowel_glide_does_not_turn_away_from_the_next_consonant() {
		// ae glides down in F2, an alveolar is higher: ae holds its F2 before t, ay keeps its glide
		let f2 = |v: u8| { let e = loaded(&[(PAU, 50, false), (26, 100, false), (v, 150, true), (36, 80, false), (6, 100, false)], v); (e.F2_A_src, e.F2_B_src) };
		let (a, b) = f2(2);
		assert_eq!(a, b, "ae before t glides from {} to {}", a, b);
		let (a, b) = f2(8);
		assert!(b - a > 500.0, "ay before t glides from {} to {}", a, b);
	}

	#[test]
	fn vowel_f3_does_not_dip_before_l() {
		// the vowel ends at the same F3 before l as before t ("feel")
		let f3 = |c2: u8| loaded(&[(PAU, 50, false), (16, 100, false), (21, 150, true), (c2, 80, false), (6, 100, false)], 21).F3_B_src;
		for l in [24u8, 25] { assert!((f3(l) - f3(36)).abs() < 1.0, "iy ends at F3 {} before {}, {} before t", f3(l), l, f3(36)); }
	}

	#[test]
	fn only_low_f3_neighbours_lower_a_vowels_f3() {
		// l and y have a high F3, so a vowel next to them keeps its own ("lee", "you"); after r and w it starts lower
		let start = |c: u8| loaded(&[(PAU, 50, false), (6, 100, false), (c, 80, false), (21, 150, true), (36, 80, false), (6, 100, false)], 21);
		let f3 = |c: u8| start(c).F3_A_src;
		for c in [24u8, 42] { assert!((f3(c) - f3(11)).abs() < 1.0, "iy starts at F3 {} after {}, {} after d", f3(c), c, f3(11)); }
		for c in [32u8, 41] { assert!(f3(c) < f3(11) - 100.0, "iy starts at F3 {} after {}, {} after d", f3(c), c, f3(11)); }
		let end = |c: u8| loaded(&[(PAU, 50, false), (16, 100, false), (21, 150, true), (c, 80, false), (6, 100, false)], 21).F3_B_src;
		assert!((end(42) - end(36)).abs() < 1.0, "iy ends at F3 {} before y, {} before t", end(42), end(36));
		// w is labial-velar, but no velar: it gets the lip rounding and no velar pinch
		assert!((start(41).F2_A_src - start(9).F2_A_src).abs() < 1.0, "iy F2 {} after w, {} after b", start(41).F2_A_src, start(9).F2_A_src);
	}

	#[test]
	fn a_vowel_after_a_voiceless_stop_is_damped_only_by_the_aspiration() {
		// the open glottis is modelled while it is open (aspiration); the vowel's own targets are those after d
		let b = |c: u8| { let e = loaded(&[(PAU, 50, false), (6, 100, false), (c, 80, false), (1, 150, true), (36, 80, false), (6, 100, false)], 1); (e.B1_A_src, e.B2_A_src) };
		assert_eq!(b(36), b(11));
	}

	#[test]
	fn vowels_have_their_own_place_before_r() {
		// eh higher and further front, the back vowels higher and further back; uw is not fronted by the t or the r
		let (eh_r, eh_t) = (vowel_targets(9, 13, 33), vowel_targets(9, 13, 36));
		assert!(eh_r.0 < eh_t.0 - 80.0 && eh_r.1 > eh_t.1 + 150.0, "eh before r {:?}, before t {:?}", eh_r, eh_t);
		let (ao_r, ao_t) = (vowel_targets(9, 4, 33), vowel_targets(9, 4, 36));
		assert!(ao_r.0 < ao_t.0 - 80.0 && ao_r.1 < ao_t.1 - 100.0, "ao before r {:?}, before t {:?}", ao_r, ao_t);
		let (_, a, b) = vowel_targets(36, 38, 33);
		assert!(a < 1100.0 && b < 1100.0, "uh in tour glides {} -> {}", a, b);
		// aa barely moves ("march" is not "morch")
		let (aa_r, aa_t) = (vowel_targets(9, 1, 33), vowel_targets(9, 1, 36));
		assert!(aa_r.0 > aa_t.0 - 80.0 && (aa_r.1 - aa_t.1).abs() < 100.0, "aa before r {:?}, before t {:?}", aa_r, aa_t);
		// ih closer and further front, toward ee ("near" is not "nit-r")
		let (ih_r, ih_t) = (vowel_targets(27, 19, 33), vowel_targets(27, 19, 36));
		assert!(ih_r.0 < ih_t.0 - 60.0 && ih_r.1 > ih_t.1 + 200.0, "ih before r {:?}, before t {:?}", ih_r, ih_t);
	}

	#[test]
	fn vowel_f2_does_not_rise_before_r() {
		// uh glides up in F2 before t, not before r
		let f2 = |c2: u8| {
			let e = loaded(&[(PAU, 50, false), (36, 100, false), (38, 150, true), (c2, 80, false), (6, 100, false)], 38);
			(e.F2_A_src, e.F2_B_src)
		};
		let ((ra, rb), (ta, tb)) = (f2(33), f2(36));
		assert!(tb > ta, "uh before t ends at F2 {} from {}", tb, ta);
		assert!(rb <= ra, "uh before r ends at F2 {} from {}", rb, ra);
	}

	// per sample (phone, F3, AV, the phone's own F3 A target) while rendering (phone, ms, stressed)
	fn trace_f3(phones: &[(u8, i32, bool)]) -> Vec<(u8, f32, f32, f32)> {
		let mut out = Vec::new();
		drive(phones, |e, _| out.push((e.phn_c, e.F3_reg, db(e.AV_reg), e.F3_A_src)));
		out
	}

	#[test]
	fn r_arrives_quickly_after_a_vowel() {
		// out of a vowel, an r (final or not) reaches its F3 within R_ONSET_MS of its start
		for &(v, r, next) in &[(4u8, 33u8, 48u8), (1, 33, 20), (1, 32, 21)] {
			let out = trace_f3(&[(PAU, 50, false), (35, 90, false), (v, 140, true), (r, 110, false), (next, 80, false), (0, 50, false)]);
			let rr: Vec<_> = out.iter().filter(|x| x.0 == r).collect();
			let n = rr.iter().position(|x| x.1 <= x.3 + 10.0).unwrap();
			assert!(n as i32 <= samples(R_ONSET_MS) + 1, "r {} after vowel {} reaches F3 {} after {} ms", r, v, rr[0].3, n as f32 * 1000.0 / m_sr as f32);
		}
	}

	#[test]
	fn r_holds_its_formants_through_the_final_release() {
		// at the end of a phrase the voice dies on the r's F3, not on the release's
		for &(r, d) in &[(33u8, 120), (14, 200)] {
			let mut seq = vec![(PAU, 50, false), (23, 90, false)];
			if r == 33 { seq.push((4, 150, true)); }
			seq.extend([(r, d, r == 14), (48, 80, false), (0, 50, false)]);
			let out = trace_f3(&seq);
			let end = out.iter().filter(|x| x.0 == r).last().unwrap().1;
			let top = out.iter().filter(|x| x.0 == 48 && x.2 > 0.0).map(|x| x.1).fold(0.0f32, f32::max);
			assert!(top - end < 50.0, "r {} ends at F3 {}, its release rises to {}", r, end, top);
		}
		// other phones still release toward the neutral vowel
		let out = trace_f3(&[(PAU, 50, false), (23, 90, false), (21, 150, true), (48, 80, false), (0, 50, false)]);
		let end = out.iter().filter(|x| x.0 == 21).last().unwrap().1;
		let low = out.iter().filter(|x| x.0 == 48 && x.2 > 0.0).map(|x| x.1).fold(9999.0f32, f32::min);
		assert!(end - low > 200.0, "iy's release stays at F3 {} from {}", low, end);
	}

	#[test]
	fn a_word_after_a_vowel_starts_with_a_catch() {
		// V-AA (the AA starting a word): the voice dips GLOTTAL_DIP_DB at the join, GLOTTAL_DIP_UNSTRESSED_DB when the AA
		// is unstressed; after AXR an unstressed AA runs on, linked by the r
		let av = |first: u8, unstressed: &[usize]| {
			let (out, _) = trace_regs_st(&[(PAU, 50, 0), (first, 150, 1), (1, 150, 2), (48, 100, 3), (0, 50, 4)], unstressed);
			let at = |c: i32| out.iter().find(|x| x.5.0 == 2 && x.1 == c).unwrap().4;
			(at(0), at(ms(40.0)))
		};
		let (dip, full) = av(1, &[]);
		assert!((full - dip - GLOTTAL_DIP_DB).abs() < 1.0, "catch dips to {} from {}", dip, full);
		let (dip, full) = av(1, &[2]);
		assert!((full - dip - GLOTTAL_DIP_UNSTRESSED_DB).abs() < 1.0, "unstressed catch dips to {} from {}", dip, full);
		let (run_on, full) = av(7, &[2]);
		assert!(run_on > full - 3.0, "unstressed AA after AXR dips to {} from {}", run_on, full);
	}

	#[test]
	fn a_final_nasal_hums_away() {
		// after an N the phrase-end release is the N's voice dying away, with no breath of an open vowel; after AA it breathes
		let breath = |p: u8| {
			let (out, _) = trace_regs(&[(PAU, 50, 0), (1, 150, 1), (p, 100, 2), (48, 100, 3), (0, 50, 4)]);
			let r: Vec<_> = out.iter().filter(|x| x.0 == 48 && x.1 >= 0).collect();
			(r.iter().map(|x| x.3).fold(0.0f32, f32::max), r[0].4)
		};
		let ((n_ah, n_av), (a_ah, _)) = (breath(27), breath(1));
		assert_eq!(n_ah, 0.0, "breath in the release after N");
		assert!(a_ah > 0.0, "no breath in the release after AA");
		assert!(n_av > 0.0, "the N's voice stops at the release");
	}

	#[test]
	fn a_final_nasal_stays_nasal_as_it_dies_away() {
		// the release keeps the nasal's coupling, formants and bandwidths ("come in.", "mine.", "sing.")
		for n in [27u8, 26, 28] {
			let mut at: Vec<(u8, [f32; 5])> = Vec::new();
			drive(&[(PAU, 50, false), (1, 150, true), (n, 100, false), (48, 150, false), (0, 50, false)],
				|e, _| at.push((e.phn_c, [e.NZP_reg, e.F2_reg, e.F3_reg, e.B1_reg, e.B2_reg])));
			let end_of = |p: u8| at.iter().filter(|x| x.0 == p).last().unwrap().1;
			let (nasal, release) = (end_of(n), end_of(48));
			for k in 0..5 { assert!((nasal[k] - release[k]).abs() <= 0.01 * nasal[k].abs() + 1e-3, "{}: {:?} -> {:?}", n, nasal, release); }
		}
	}

	// per sample (phone, counter, F1, F2, the F2 being moved toward) while rendering (phone, ms, stressed)
	fn trace_moves(phones: &[(u8, i32, bool)]) -> Vec<(u8, i32, f32, f32, f32)> {
		let mut out = Vec::new();
		drive(phones, |e, _| out.push((e.phn_c, e.counter, e.F1_reg, e.F2_reg, e.F2_reg_trg)));
		out
	}

	#[test]
	fn formants_reach_a_stop_by_its_closure() {
		// the move out of a vowel is heard only until the closure, so it is over by then
		for &v in &[1u8, 29, 21] {
			for &c in &[31u8, 36, 23, 9, 11, 17] {
				let out = trace_moves(&[(PAU, 50, false), (35, 90, false), (v, 140, true), (c, 90, false), (6, 100, false), (48, 80, false), (0, 50, false)]);
				let x = out.iter().find(|x| x.0 == c && x.1 == 0).unwrap();
				assert!((x.3 - x.4).abs() < 5.0, "vowel {} into {}: F2 {} at the closure, moving to {}", v, c, x.3, x.4);
			}
		}
	}

	#[test]
	fn a_long_vowel_moves_into_the_next_phone_in_set_time() {
		// ah carries over half its length into the next phone; a long one holds that to VOWEL_EXIT_MS
		let carried = |d: i32| {
			let out = trace_moves(&[(PAU, 50, false), (23, 90, false), (3, d, true), (36, 90, false), (6, 100, false), (48, 80, false), (0, 50, false)]);
			// the counter has stepped once by the first sample
			(1 - out.iter().find(|x| x.0 == 36).unwrap().1, out.len())
		};
		let (long, n_long) = carried(200);
		assert_eq!(long, samples(VOWEL_EXIT_MS), "carried {} ms", long as f32 * 1000.0 / m_sr as f32);
		let (short, n_short) = carried(60);
		assert!(short < samples(VOWEL_EXIT_MS) && short > samples(25.0), "short ah carries {} samples", short);
		// the vowel's length does not change
		assert!((n_long as i32 - n_short as i32 - samples(140.0)).abs() <= 1, "{} samples apart", n_long - n_short);
	}

	// per sample (phone, breath noise level, Rd) as the tract gets them, rendering (phone, ms, stressed)
	fn trace_breath(phones: &[(u8, i32, bool)]) -> Vec<(u8, f32, f32)> {
		let mut out = Vec::new();
		drive(phones, |e, _| out.push((e.phn_c, e.vtm_eng.BR, e.vtm_eng.RD)));
		out
	}

	#[test]
	fn the_voice_gets_breathy_as_a_phrase_ends() {
		let out = trace_breath(&[(PAU, 50, false), (23, 90, false), (1, 200, true), (48, 80, false), (0, 50, false)]);
		let vowel: Vec<_> = out.iter().filter(|x| x.0 == 1).collect();
		let release: Vec<_> = out.iter().filter(|x| x.0 == 48).collect();
		let rd = vowel.last().unwrap().2;
		// the vowel itself as ever
		assert!(vowel.iter().all(|x| x.1 == BREATH_DB), "breath in the vowel");
		// rising through the start of the release, then held
		let k = ms(BREATH_RAMP_MS) as usize;
		assert!(release.windows(2).take(k).all(|w| w[1].1 >= w[0].1), "breath rises");
		let held = release[k + 1];
		assert!((held.1 - BREATH_DB - BREATH_END_DB).abs() < 0.01, "breath {} at the release", held.1);
		assert!((held.2 - rd - BREATH_END_RD).abs() < 0.02, "Rd {} at the release, {} at the vowel's end", held.2, rd);
		// no breathier voice inside a phrase
		let out = trace_breath(&[(PAU, 50, false), (23, 90, false), (1, 200, true), (36, 90, false), (1, 150, false), (48, 80, false), (0, 50, false)]);
		assert!(out.iter().filter(|x| x.0 != 48 && x.0 != 0).all(|x| x.1 == BREATH_DB), "breath inside the phrase");
	}

	#[test]
	fn the_release_keeps_the_voice_rd() {
		// the phrase-end release is the vowel's voice dying away, not a tenser one
		let mut out = Vec::new();
		drive(&[(PAU, 50, false), (23, 90, false), (1, 200, true), (48, 80, false), (0, 50, false)], |e, _| out.push((e.phn_c, e.RD_reg)));
		let end = out.iter().filter(|x| x.0 == 1).last().unwrap().1;
		assert!(out.iter().filter(|x| x.0 == 48).all(|x| (x.1 - end).abs() < 0.02), "Rd falls from {} in the release", end);
	}

	#[test]
	fn a_short_liquid_or_glide_arrives_early() {
		// l, r, w and y are heard by where their formants go, so even a short one is mostly there by its middle
		for &c in &[24u8, 32, 41, 42] {
			let out = trace_moves(&[(PAU, 50, false), (2, 140, true), (c, 40, false), (2, 140, false), (48, 80, false), (0, 50, false)]);
			let start = out.iter().find(|x| x.0 == c).unwrap().3;
			let own: Vec<_> = out.iter().filter(|x| x.0 == c && x.1 >= 0).collect();
			let (mid, end) = (own[own.len() / 2].3, own[own.len() - 1].3);
			assert!((mid - end).abs() <= 0.1 * (start - end).abs(), "phone {}: F2 {} -> {} at its middle, {} at its end", c, start, mid, end);
		}
	}

	#[test]
	fn formants_do_not_stop_dead() {
		// "follower": the move into its short w was cut off at full speed, a click when that landed on a voicing pulse
		let mut e = engine::new();
		let mut f: Vec<[f32; 3]> = Vec::new();
		for (seg, &(p, d)) in [(PAU, 50), (16, 102), (1, 139), (25, 104), (29, 106), (41, 53), (7, 114), (48, 80), (PAU, 50)].iter().enumerate() {
			if e.load(p, d, p == 1, seg, seg == 1) {
				while e.is_running() { e.sample(120.0); f.push([e.F1_reg, e.F2_reg, e.F3_reg]); }
			}
		}
		let jump = |w: &[[f32; 3]], k: usize| (w[2][k] - 2.0 * w[1][k] + w[0][k]).abs();
		let worst = f.windows(3).map(|w| jump(w, 0).max(jump(w, 1)).max(jump(w, 2))).fold(0.0, f32::max);
		assert!(worst < 0.1, "a formant changed speed by {} Hz per sample in one sample", worst);
	}
}
