/*---------------------------------------
| KLATT-STYLE FORMANT SYNTHESIZER       |
|                                       |
| AUTHOR: ANTHONY C. BARTMAN (@ROMMIX0) |
| DATE:   8/11/2026-10/23/2026          |
---------------------------------------*/


use crate::_vtm::{vtm, Voice};
use crate::_vtm::m_sr;
use crate::_vtbl::formant_tbl;
use crate::_utt::PAU;

const two_ms:    i32 = (m_sr as f32 * 0.002) as i32;
const five_ms:   i32 = (m_sr as f32 * 0.005) as i32;
const ten_ms:    i32 = (m_sr as f32 * 0.010) as i32;
const twenty_ms: i32 = (m_sr as f32 * 0.020) as i32;
const thirty_ms: i32 = (m_sr as f32 * 0.030) as i32;
const forty_ms:  i32 = (m_sr as f32 * 0.040) as i32;
const fifty_ms:  i32 = (m_sr as f32 * 0.050) as i32;
const sixty_ms:  i32 = (m_sr as f32 * 0.060) as i32;
const fifteen_ms: i32 = (m_sr as f32 * 0.015) as i32;
const eighty_ms: i32 = (m_sr as f32 * 0.080) as i32;

// shortest plosive closure before the burst
const min_closure: i32 = thirty_ms;

// plosive release: burst and aspiration lengths (ms) by place (labial, alveolar, velar)
const BURST_MS: [[f32; 3]; 2] = [[5.0, 15.0, 25.0], [5.0, 10.0, 20.0]];		// voiceless, voiced
const ASP_MS: [f32; 3] = [50.0, 55.0, 50.0];
// the burst falls this much (dB) over its length
const BURST_DECAY_DB: f32 = 12.0;
// affricates: frication length (ms; voiceless, voiced) after the closure, and its fall (dB) over that length
const AFFRIC_MS: [f32; 2] = [55.0, 40.0];
const AFFRIC_DECAY_DB: f32 = 6.0;
// plosive bursts (not affricates): overall trim, and the extra cut before an unstressed vowel (dB)
const BURST_GAIN_DB: f32 = -4.0;
const BURST_UNSTRESSED_DB: f32 = -6.0;
// a lip release is weak noise through the vocal tract: its level (AH, dB) for p and b
const LABIAL_BURST_AH: [f32; 2] = [24.0, 30.0];
// the noise starts at the lips, in front of the tract, so the tract hardly shapes it:
// during the burst F2 and F3 are held at most at these (Hz), whatever the vowel
const LABIAL_BURST_F2: f32 = 1100.0;
const LABIAL_BURST_F3: f32 = 2300.0;
// and the cut next to a sibilant (dB)
const BURST_SIBILANT_DB: f32 = -6.0;
// an unreleased stop plays this share of its duration, as closure merging with the next one
const UNRELEASED_SHARE: f32 = 0.5;
// sibilant fricatives fade out into a vowel or sonorant over this long (ms), centred on the boundary
const SIB_RAMP_MS: f32 = 40.0;
// other fricatives crossfade into a vowel or sonorant over this long (ms), centred on the boundary
const FRIC_RAMP_MS: f32 = 40.0;
// out of a vowel or sonorant into a fricative, the crossfade takes this long (ms), sibilant and not
const SIB_ENTRY_MS: f32 = 60.0;
const FRIC_ENTRY_MS: f32 = 70.0;
// fricative noise rises and falls over this long (ms) next to silence or the phrase-final release
const FRIC_SILENCE_MS: f32 = 70.0;
// a nasal's voicing level and nasal coupling switch within this long (ms) at its closure and release
const NASAL_EDGE_MS: f32 = 15.0;
// out of a tongue consonant into a back vowel, the vowel reaches its formants this long (ms) after the release
const BACK_TRANSITION_MS: f32 = 80.0;
// a vowel's F3 is this much (Hz) lower at its end before an r
const RHOTIC_F3: f32 = 200.0;
// breath noise in the voice (dB), and how much more of it and how much laxer the voice (Rd) as it dies away at a
// phrase end, reached this long (ms) into the phrase-end release
const BREATH_DB: f32 = 6.0;
const BREATH_END_DB: f32 = 12.0;
const BREATH_END_RD: f32 = 0.3;
const BREATH_RAMP_MS: f32 = 60.0;
// a vowel's move into the next phone takes at most this long (ms)
const VOWEL_EXIT_MS: f32 = 50.0;
// out of a vowel an r reaches its formants within this long (ms)
const R_ONSET_MS: f32 = 50.0;
// while F2 glides, F1 still opens into the back vowel within this long (ms)
const BACK_F1_MS: f32 = 40.0;
// a back vowel whose own targets move this much (Hz, F1 or F2) starts that glide on time, not after the slow F2
const GLIDE_HZ: f32 = 100.0;
// the vowel after a nasal reaches its formants this long (ms) after the release
const NASAL_RELEASE_MS: f32 = 30.0;
// a vowel's nasal coupling next to a nasal: up to NASAL_VOWEL_K, over its last NASAL_ANTIC_MS before one and its first
// NASAL_CARRY_MS after one
const NASAL_VOWEL_K: f32 = 0.3;
const NASAL_ANTIC_MS: f32 = 80.0;
const NASAL_CARRY_MS: f32 = 40.0;
// aspiration level (AH, dB) by place
const ASP_DB: [f32; 3] = [17.5, 15.5, 15.5];
// aspiration's share before an unstressed vowel or a sonorant (none after a sibilant in the same word)
const ASP_UNSTRESSED: f32 = 0.6;
const ASP_BEFORE_SONORANT: f32 = 0.5;
// after aspiration, voicing builds up over VOICE_ONSET_MS while the aspiration fades out under it (ms)
const VOICE_ONSET_MS: f32 = 10.0;
const ASP_OVERLAP_MS: f32 = 20.0;
// aspiration through a front, close vowel's tract comes out loud against that quiet vowel: it is cut by this
// much (dB) per kHz of the next vowel's F2-F1 gap above ASP_FRONT_FROM_HZ
const ASP_FRONT_DB_PER_KHZ: f32 = 6.5;
const ASP_FRONT_FROM_HZ: f32 = 700.0;
// h's breath level (AH dB) before a vowel, cut like aspiration before front vowels
const H_DB: f32 = 18.0;
// h's breath and the neighbouring vowel's voicing crossfade over this long (ms)
const H_EDGE_MS: f32 = 20.0;
// the open glottis damps F1 and F2 during aspiration: bandwidth added (Hz), fading out with the aspiration
const ASP_B1_WIDEN: f32 = 400.0;
const ASP_B2_WIDEN: f32 = 70.0;
// voiced stop closure after a voiced sound: the voice bar fades this much (dB) over the closure
const VOICE_BAR_FADE_DB: f32 = 6.0;

// longest share of a vowel the transition into it may take
const ramp_share: f32 = 0.6;

// vowel reduction: how far a vowel of d ms gets from neutral toward its target (0..1)
const tau_f1: f32 = 40.0;
const tau_f2: f32 = 40.0;
const tau_f3: f32 = 40.0;

fn reduce_k(d_ms: f32, tau: f32) -> f32 {
	1.0 - (-d_ms / tau).exp()
}

// velar F3 next to a vowel: slope and offset of F3 = offset + slope * the vowel's F2 (near F2 by front vowels)
const VELAR_F3: (f32, f32) = (0.45, 1700.0);

// consonant F2 next to a vowel, by place: slope and offset of F2 = offset + slope * the vowel's F2
fn locus(x: u8) -> Option<(f32, f32)> {
	match x {
		9 | 31 | 26 | 16 | 40      => Some((0.8, 300.0)),		// labial: b p m f v
		37 | 12                    => Some((0.5, 900.0)),		// dental: th dh
		11 | 36 | 27 | 34 | 43 | 47 => Some((0.4, 1200.0)),	// alveolar: d t n s z dx
		35 | 44 | 10 | 22          => Some((0.3, 1500.0)),		// postalveolar: sh zh ch jh
		17 | 23 | 28               => Some((0.5, 1100.0)),		// velar: g k ng
		_                          => None,
	}
}

fn fade_db(db: f32, m: f32) -> f32 {
	// scale an amplitude (dB, 0 is off) by a fader m in linear gain
	if db <= 0.0 || m <= 0.0 { return 0.0; }
	(db + 6.0 * m.log2()).max(0.0)
}

fn lerp_db_gain(a: f32, b: f32, y: f32) -> f32 {
	// interpolate two amplitudes (dB, 0 is off) in linear gain, the same scale the VTM uses
	let gain = |db: f32| if db <= 0.0 { 0.0 } else { 0.5_f32.powf((87.0 - db) / 6.0) };
	let g = gain(a) * (1.0 - y) + gain(b) * y;
	if g <= 0.0 { 0.0 } else { 87.0 + 6.0 * g.log2() }
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
		1|2|3|4|5|6|7|14|29|38 => 0x0003,		// aa, ae, ah, ao, aw, ax, axr, er, ow
		13|15|19|20|21         => 0x0007,		// eh, ey, ih, ix, iy
		39                     => 0x000B,		// uh, uw
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
		24|25|45               => 0x0422,		// l, lx
		32|33|46               => 0x0822,		// r, rx
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

fn samples(ms: f32) -> i32 {
	(ms * m_sr as f32 / 1000.0) as i32
}

fn stop_place(x: u8) -> usize {
	match x { 31 | 9 => 0, 36 | 11 => 1, _ => 2 }
}

// a plosive's parts in samples: the closure and burst (an affricate's frication) play in its own segment,
// the aspiration in the start of the next phone
#[derive(Debug, Clone, Copy, PartialEq)]
struct Stop {
	closure: i32,
	burst:   i32,
	asp:     i32,
}

fn stop_timeline(phone: u8, pdr: i32, prev: u8, next: u8, next_stressed: bool, word_start: bool) -> Stop {
	let pf = feats(phone);
	if pf & 0x8000 != 0 {
		// affricate: always released, its frication plays where a stop's burst would
		let fric = samples(AFFRIC_MS[(pf & 0x2 != 0) as usize]).min((pdr - min_closure).max(0));
		return Stop { closure: pdr - fric, burst: fric, asp: 0 };
	}
	let nf = feats(next);
	// unreleased before another stop or a nasal
	if nf & 0x50 != 0 { return Stop { closure: (pdr as f32 * UNRELEASED_SHARE) as i32, burst: 0, asp: 0 }; }
	let place = stop_place(phone);
	let voiced = pf & 0x2 != 0;
	let mut burst = samples(BURST_MS[voiced as usize][place]);
	let mut asp = if voiced { 0 } else { samples(ASP_MS[place]) };
	// too short for the whole release: burst and aspiration shrink together
	let room = (pdr - min_closure).max(0);
	if burst + asp > room {
		let k = room as f32 / (burst + asp) as f32;
		burst = (burst as f32 * k) as i32;
		asp = (asp as f32 * k) as i32;
	}
	let closure = pdr - burst - asp;
	// context only shortens the aspiration, and with it the stop
	if nf & 0x1 != 0 {
		if !next_stressed { asp = (asp as f32 * ASP_UNSTRESSED) as i32; }
	} else if nf & 0x1C00 != 0 {
		asp = (asp as f32 * ASP_BEFORE_SONORANT) as i32;
	} else {
		// obstruent, phrase-final release or pause: burst only
		asp = 0;
	}
	if feats(prev) & 0x200 != 0 && !word_start { asp = 0; }
	Stop { closure, burst, asp }
}

pub struct engine {
	vtm_eng: vtm,
	marker_A: i32,
	amp_len: i32,		// samples the amplitudes take to reach this phone's values
	amp_start: i32,		// counter value where that ramp starts
	ab_at: i32,			// counter value where the formants start moving from the A to the B targets
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
	seg_c: usize,		// stream index of the playing segment
	seg_r1: usize,
	seg_start: i32,	// counter value when the playing segment started
	pwi_c:  bool,		// phone starts a word
	pwi_r1: bool,
	stop_on: bool,		// playing a plosive's closure and burst
	stop_bar: bool,		// voiced plosive after a voiced sound: a voice bar in the closure
	stop_affric: bool,
	stop_labial: bool,
	burst_len: i32,
	burst_decay: f32,
	burst_gain: f32,	// linear gain on the burst
	after_stop: bool,	// this phone follows a released plosive
	f1_len: i32,		// samples F1 takes into this phone when it moves faster than the other formants (0: same)
	f1_speed: f32,		// how much faster F1 moves than the shared interpolator
	after_stop_next: bool,
	asp_db: f32,		// aspiration level carried over from a voiceless stop (0: none)
	asp_start: i32,
	asp_seg: usize,
	asp_db_next: f32,
	asp_seg_next: usize,
	pub pst_c: bool,
	pst_r1: bool,
	GL: f32,
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
	optime: f32,
	glot_time: f32,
	end_time: f32,
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
	NZP_B_src: f32,
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
			/////////////////////////////
			//          VTM            //
			/////////////////////////////
			vtm_eng: vtm::new(),
			/////////////////////////////
			//          MAIN           //
			/////////////////////////////
			marker_A:  0,
			amp_len:   0,
			ab_at: 0,
			amp_start: 0,
			marker_B:  0,
			marker_BA: 0,
			is_first:  true,
			counter:   0,
			/////////////////////////////
			//     CONTEXT WINDOW      //
			/////////////////////////////
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
			burst_len: 0,
			burst_decay: 0.0,
			burst_gain: 1.0,
			after_stop: false,
			f1_len: 0,
			f1_speed: 1.0,
			after_stop_next: false,
			asp_db: 0.0,
			asp_start: 0,
			asp_seg: 0,
			asp_db_next: 0.0,
			asp_seg_next: 0,
			/////////////////////////////
			//        REGISTERS        //
			/////////////////////////////
			// traversal registers
			GL:      1.0,				// glottalization F0 level
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
			/////////////////////////////
			//         SLOPE           //
			/////////////////////////////
			slope:        0.0,
			interp:       0.0,
			master_fade:  1.0,							// master fader for phrase-final trigger
			optime:       1.0 / two_ms   as f32,		// time the master fader takes to come back
			glot_time:    1.0 / ten_ms   as f32,		// glottalization timing
			end_time:     1.0 / fifty_ms as f32,		// phrase-final fade time
			/////////////////////////////
			//         SOURCE          //
			/////////////////////////////
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
			NZP_B_src: 0.0,
			F1_B_src:  0.0,
			F2_B_src:  0.0,
			F3_B_src:  0.0,
			B1_B_src:  0.0,
			B2_B_src:  0.0,
			B3_B_src:  0.0,
		}
	}
	pub fn set_voice(&mut self, v: Voice) {
		self.vtm_eng.set_voice(v);
	}
	fn ms_to_samples(&mut self, x: i32) -> i32 {
		// convert milliseconds to samples
		((x as f32 / 1000.0) * m_sr as f32) as i32
	}
	fn advance(&mut self, x: u8, d: i32, s: bool, seg: usize, w: bool) {
		// x is phone_id
		// d is duration
		// s is stress
		/////////////////////////////
		//        PHONEME          //
		/////////////////////////////
		self.phn_l1 = self.phn_c;
		self.phn_c  = self.phn_r1;
		self.phn_r1 = x;
		/////////////////////////////
		//        FEATURES         //
		/////////////////////////////
		self.pft_l1 = self.pft_c;
		self.pft_c  = self.pft_r1;
		self.pft_r1 = feats(x);
		/////////////////////////////
		//        DURATION         //
		/////////////////////////////
		self.pdr_c  = self.pdr_r1;
		self.pdr_r1 = self.ms_to_samples(d);
		/////////////////////////////
		//         STRESS          //
		/////////////////////////////
		self.pst_l1 = self.pst_c;
		self.pst_c  = self.pst_r1;
		self.pst_r1 = s;
		/////////////////////////////
		//     STREAM POSITION     //
		/////////////////////////////
		self.seg_c  = self.seg_r1;
		self.seg_r1 = seg;
		self.pwi_c  = self.pwi_r1;
		self.pwi_r1 = w;
	}
	pub fn is_running(&mut self) -> bool {
		if self.counter >= self.marker_B { return false; }
		true
	}
	pub fn is_ready(&mut self) -> bool {
		if self.phn_c != 0 { return true; }
		false
	}
	pub fn is_not_last(&mut self) -> bool {
		if self.phn_c != 0 && self.phn_r1 == 0 { return false; }
		true
	}
	fn fetch(&mut self) {
		// the pause uses the silent row
		let row: u8 = if self.phn_c == PAU { 0 } else { self.phn_c };
		let offset_src: usize = row as usize * 31;
		//---------------source---------------------
		self.C_A_src   = formant_tbl[offset_src+0] as u8;		// also target during transition time
		self.C_B_src   = formant_tbl[offset_src+1] as u8;
		self.RD_A_src  = formant_tbl[offset_src+2] as f32 / 100.0;
		self.RD_B_src  = formant_tbl[offset_src+3] as f32 / 100.0;
		self.NZP_A_src = self.fetch_nsl();
		self.F1_A_src  = formant_tbl[offset_src+5] as f32;
		self.F2_A_src  = formant_tbl[offset_src+6] as f32;
		self.F3_A_src  = formant_tbl[offset_src+7] as f32;
		self.B1_A_src  = formant_tbl[offset_src+8] as f32;
		self.B2_A_src  = formant_tbl[offset_src+9] as f32;
		self.B3_A_src  = formant_tbl[offset_src+10] as f32;
		self.A2_src    = formant_tbl[offset_src+11] as f32;
		self.A3_src    = formant_tbl[offset_src+12] as f32;
		self.A4_src    = formant_tbl[offset_src+13] as f32;
		self.A5_src    = formant_tbl[offset_src+14] as f32;
		self.A6_src    = formant_tbl[offset_src+15] as f32;
		self.A7_src    = formant_tbl[offset_src+16] as f32;
		self.A8_src    = formant_tbl[offset_src+17] as f32;
		self.A9_src    = formant_tbl[offset_src+18] as f32;
		self.A10_src   = formant_tbl[offset_src+19] as f32;
		self.AB_src    = formant_tbl[offset_src+20] as f32;
		self.AF_src    = formant_tbl[offset_src+21] as f32;
		self.AH_src    = formant_tbl[offset_src+22] as f32;
		self.AV_src    = formant_tbl[offset_src+23] as f32;
		self.NZP_B_src = self.NZP_A_src;
		self.F1_B_src  = formant_tbl[offset_src+25] as f32;
		self.F2_B_src  = formant_tbl[offset_src+26] as f32;
		self.F3_B_src  = formant_tbl[offset_src+27] as f32;
		self.B1_B_src  = formant_tbl[offset_src+28] as f32;
		self.B2_B_src  = formant_tbl[offset_src+29] as f32;
		self.B3_B_src  = formant_tbl[offset_src+30] as f32;

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
	fn fetch_nsl(&mut self) -> f32 {
		// nasal coupling: 1 for a nasal consonant, 0 otherwise
		if self.pft_c & 0x10 != 0 { 1.0 } else { 0.0 }
	}
	fn ptar(&mut self) {
		if self.phn_c == 24 && self.pft_r1 & 0x42 == 0x40 {
			self.phn_c = 45;
		} else if self.phn_c == 32 && self.pft_r1 & 0x42 == 0x40 {
			self.phn_c = 46;
		}
	}
	fn ftar(&mut self) {
		if self.pft_c & 0x1 == 1 {
			// STRESS COLORING FOR VOWEL
			let factor: f32 = if self.pst_c { 0.75 } else { 1.25 };

			// VELAR PINCHING RULE
			if self.pft_l1 & 0x100 != 0 {
				self.F2_A_src += 80.0 * factor;
				self.F3_A_src -= 100.0 * factor;
			}
			if self.pft_r1 & 0x100 != 0 {
				self.F2_B_src += 80.0 * factor;
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
			if self.pft_l1 & 0x1C00 != 0 {
				self.F3_A_src -= 150.0 * factor;
			}
			if self.pft_r1 & 0x1C00 != 0 && self.pft_r1 & 0x800 == 0 {
				self.F3_B_src -= 150.0 * factor;
			}

			// NASALIZATION
			if self.pft_r1 & 0x10 != 0 {
				self.F2_A_src += 110.0 * factor;
				self.F2_B_src += 110.0 * factor;
			}

			// WIDEN BANDWIDTH
			if self.pft_r1 & 0x3 == 0x3 {
				self.B1_B_src *= 2.5 * factor;
			}
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

			// FRONT VOWEL ADJUSTMENT
			if self.pft_c & 0x4 != 0 {
				if self.pft_l1 & 0x100 != 0 {
					self.F2_A_src += 50.0 * factor;
				}
			}

			// VOWEL RHOTICIZATION
			if self.pft_r1 & 0x800 != 0 {
				// the vowel takes on r-coloring as it moves into the r, not from its start, by the same amount
				// stressed or not
				self.F3_B_src -= RHOTIC_F3;
			}

			// WIDEN BANDWIDTH AFTER UNVOICED PLOSIVE
			if self.pft_l1 & 0x42 == 0x40 {
				self.B1_A_src += 125.0 * factor;
				self.B2_A_src += 50.0 * factor;
			}

			// BACK VOWEL ALVEOLARIZATION (next to alveolar stops, nasals and fricatives, not l or r)
			if self.pft_c == 0x000B {
				let alveolar = |f: u16| f & 0x20 != 0 && f & 0x0C00 == 0;
				if alveolar(self.pft_l1) {
					self.F2_A_src += 250.0 * factor;
				}
				if alveolar(self.pft_r1) {
					self.F2_B_src += 250.0 * factor;
				}
			}

		} else if self.pft_c & 0x1 == 0 {
			// NASAL ADJUSTMENT AFTER FRONT VOWEL
			if self.pft_c & 0x10 != 0 && self.pft_l1 & 0x5 == 0x5 {
				self.F2_A_src -= 125.0;
				self.F2_B_src -= 125.0;
				self.F3_A_src -= 100.0;
				self.F3_B_src -= 100.0;
			}
			// VELAR FRONTING/LABIALIZATION RULE
			if self.pft_c & 0x300 != 0 && self.pft_r1 & 0x5 == 0x5 {
				self.F2_A_src += 150.0;
				self.F2_B_src += 150.0;
				self.F3_A_src += 175.0;
				self.F3_B_src += 175.0;
			}
			// NASAL BANDWIDTH ADJUSTMENT (CHEN 1997)
			if self.pft_c & 0x10 != 0 {
				self.B1_A_src += 107.0;
				self.B1_B_src += 107.0;
			}
		}

		// deal with fricative amp params below
		if self.pft_c & 0x8000 == 0 {
			if self.pft_c & 0x41 == 0 && self.pdr_c < eighty_ms {
				// short fricatives don't reach AF target (less than 80 ms), lower AF
				self.AF_src -= 3.0;
			}
			if self.pft_l1 & 0x1 == 1 && self.pft_c & 0x41 == 0 && self.pft_r1 & 0x1 == 1 {
				// fricatives are stronger between two vowels
				self.AF_src += 3.0;
			}
		}

		if self.pft_r1 & 0x4 != 0 {
			// lower AF if front vowel is next
			self.AF_src -= 3.0;
		}

		if self.pst_l1 == false && self.pst_r1 == false && self.pft_c & 0x1 == 0 {
			// unstressed consonant has lower AF
			self.AF_src -= 3.0;
		}
	}
	fn reduce(&mut self) {
		// move vowel targets toward the neutral vowel (AX) as the vowel gets shorter
		if self.pft_c & 0x1 == 0 || self.phn_c == 6 || self.phn_c == 20 { return; }
		let d_ms = self.pdr_c as f32 * 1000.0 / m_sr as f32;
		let n = 6 * 31;
		let (n1, n2, n3) = (formant_tbl[n + 5] as f32, formant_tbl[n + 6] as f32, formant_tbl[n + 7] as f32);
		let (k1, k2, k3) = (reduce_k(d_ms, tau_f1), reduce_k(d_ms, tau_f2), reduce_k(d_ms, tau_f3));
		self.F1_A_src = n1 + (self.F1_A_src - n1) * k1;
		self.F2_A_src = n2 + (self.F2_A_src - n2) * k2;
		self.F3_A_src = n3 + (self.F3_A_src - n3) * k3;
		self.F1_B_src = n1 + (self.F1_B_src - n1) * k1;
		self.F2_B_src = n2 + (self.F2_B_src - n2) * k2;
		self.F3_B_src = n3 + (self.F3_B_src - n3) * k3;
	}
	fn next_vowel_formants(&self) -> [f32; 3] {
		// the next vowel's F1-F3 at its start, reduced for its length as reduce() will do
		let o = self.phn_r1 as usize * 31;
		let t = [formant_tbl[o + 5] as f32, formant_tbl[o + 6] as f32, formant_tbl[o + 7] as f32];
		if self.phn_r1 == 6 || self.phn_r1 == 20 { return t; }
		let d_ms = self.pdr_r1 as f32 * 1000.0 / m_sr as f32;
		let n = 6 * 31;
		let nv = [formant_tbl[n + 5] as f32, formant_tbl[n + 6] as f32, formant_tbl[n + 7] as f32];
		let k = [reduce_k(d_ms, tau_f1), reduce_k(d_ms, tau_f2), reduce_k(d_ms, tau_f3)];
		[0, 1, 2].map(|i| nv[i] + (t[i] - nv[i]) * k[i])
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
				self.AH_src = H_DB - ASP_FRONT_DB_PER_KHZ * ((f2 - f1) - ASP_FRONT_FROM_HZ).max(0.0) / 1000.0;
			}
			return;
		}
		let Some((k, c)) = locus(self.phn_c) else { return };
		let next_f2 = self.next_vowel_formants()[1];
		let left = if prev_vowel { Some(prev_f2) } else { None };
		let right = if next_vowel { Some(next_f2) } else { None };
		// a side with no vowel holds the other side's value
		if let Some(v) = left.or(right) { self.F2_A_src = c + k * v; }
		if let Some(v) = right.or(left) { self.F2_B_src = c + k * v; }
		if stop_place(self.phn_c) == 2 && self.pft_c & 0x100 != 0 {
			// velars: F3 comes down toward F2 by front vowels
			let (k3, c3) = VELAR_F3;
			if let Some(v) = left.or(right) { self.F3_A_src = c3 + k3 * v; }
			if let Some(v) = right.or(left) { self.F3_B_src = c3 + k3 * v; }
		}
	}
	fn get_control_A_duration(&mut self, x: u8, d: i32) -> i32 {
		(d as f32 * (x as f32 / 100.0)) as i32
	}
	fn get_control_B_duration(&mut self, x: u8, d: i32) -> i32 {
		let xx = 100 - x;
		self.get_control_A_duration(xx, d)
	}
	fn load_registers(&mut self) {
		// load for the first time
		// reset counter
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
		self.A10_reg_src  = 0.0;
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
		self.A2_reg_trg  = self.A2_src;
		self.A3_reg_trg  = self.A3_src;
		self.A4_reg_trg  = self.A4_src;
		self.A5_reg_trg  = self.A5_src;
		self.A6_reg_trg  = self.A6_src;
		self.A7_reg_trg  = self.A7_src;
		self.A8_reg_trg  = self.A8_src;
		self.A9_reg_trg  = self.A9_src;
		self.A10_reg_trg = self.A10_src;
		self.AB_reg_trg  = self.AB_src;
		self.AF_reg_trg  = self.AF_src;
		self.AH_reg_trg  = self.AH_src;
		self.AV_reg_trg  = self.AV_src;
	}
	fn lin2smooth(&mut self, x: f32) -> f32 {
		// linear to smooth mapping
		// used by interpolator for smoothing
		x * x * (3.0 - 2.0 * x)
	}
	fn update_registers(&mut self) {
		// x is interpolation factor (0 < x < 1)
		if self.interp < 1.0 {
			self.interp += self.slope;
			if self.interp > 1.0 { self.interp = 1.0; }
		}

		// assign self.interp to y. this will give us and opportunity
		// to use smooth mappings (such as linear to cosine)
		let y = self.lin2smooth(self.interp);
		let y1 = self.lin2smooth((self.interp * self.f1_speed).min(1.0));

		// update the interpolator (except amps, those will be done elsewhere)
		self.RD_reg  = self.RD_reg_src  * (1.0 - y) + self.RD_reg_trg  * y;
		self.F1_reg  = self.F1_reg_src  * (1.0 - y1) + self.F1_reg_trg  * y1;
		self.F2_reg  = self.F2_reg_src  * (1.0 - y) + self.F2_reg_trg  * y;
		self.F3_reg  = self.F3_reg_src  * (1.0 - y) + self.F3_reg_trg  * y;
		self.B1_reg  = self.B1_reg_src  * (1.0 - y) + self.B1_reg_trg  * y;
		self.B2_reg  = self.B2_reg_src  * (1.0 - y) + self.B2_reg_trg  * y;
		self.B3_reg  = self.B3_reg_src  * (1.0 - y) + self.B3_reg_trg  * y;
	}
	fn update_amp_registers(&mut self, c: i32, l: i32) {
		// interpolation smooth mapping algorithm
		let mut y: f32 = 0.0;
		if l > 0 {
			let mut rr: f32 = (c - self.amp_start) as f32 / l as f32;
			if rr < 0.0 { rr = 0.0; }
			if rr > 1.0 { rr = 1.0; }
			y = self.lin2smooth(rr);
		}


		// the nasal coupling switches with the amplitudes
		self.NZP_reg = self.NZP_reg_src * (1.0 - y) + self.NZP_reg_trg * y;
		if self.pft_c & 0x1 != 0 {
			// the velum lowers during a vowel before a nasal and is still rising after one, so the vowel is partly nasal
			let near = |nasal: bool, d: i32, len: f32| if nasal { NASAL_VOWEL_K * (1.0 - d as f32 / samples(len) as f32).clamp(0.0, 1.0) } else { 0.0 };
			let k = near(self.pft_r1 & 0x10 != 0, self.marker_B - c, NASAL_ANTIC_MS).max(near(self.pft_l1 & 0x10 != 0, c, NASAL_CARRY_MS));
			self.NZP_reg = self.NZP_reg.max(k);
		}

		// update the interpolator for the parallel amps
		self.A2_reg  = lerp_db_gain(self.A2_reg_src, self.A2_reg_trg, y);
		self.A3_reg  = lerp_db_gain(self.A3_reg_src, self.A3_reg_trg, y);
		self.A4_reg  = lerp_db_gain(self.A4_reg_src, self.A4_reg_trg, y);
		self.A5_reg  = lerp_db_gain(self.A5_reg_src, self.A5_reg_trg, y);
		self.A6_reg  = lerp_db_gain(self.A6_reg_src, self.A6_reg_trg, y);
		self.A7_reg  = lerp_db_gain(self.A7_reg_src, self.A7_reg_trg, y);
		self.A8_reg  = lerp_db_gain(self.A8_reg_src, self.A8_reg_trg, y);
		self.A9_reg  = lerp_db_gain(self.A9_reg_src, self.A9_reg_trg, y);
		self.A10_reg = lerp_db_gain(self.A10_reg_src, self.A10_reg_trg, y);

		// update the interpolator for main amps
		self.AB_reg = lerp_db_gain(self.AB_reg_src, self.AB_reg_trg, y);
		let (mut g_af, mut g_ah, mut g_av) = (1.0, 1.0, 1.0);
		if self.asp_db > 0.0 && c < 0 {
			// aspiration rises over 2 ms after the burst
			let rise = ((c - self.asp_start) as f32 / two_ms as f32).min(1.0);
			(g_af, g_ah, g_av) = (0.0, rise, 0.0);
		}
		self.AF_reg = fade_db(lerp_db_gain(self.AF_reg_src, self.AF_reg_trg, y), self.master_fade * g_af);
		self.AH_reg = fade_db(lerp_db_gain(self.AH_reg_src, self.AH_reg_trg, y), self.master_fade * g_ah);
		self.AV_reg = fade_db(lerp_db_gain(self.AV_reg_src, self.AV_reg_trg, y), self.master_fade * g_av);
		if self.stop_on && c >= 0 {
			// the voice bar fades over the closure and holds through the burst
			let bar = if self.stop_bar {
				let t = (c.min(self.marker_A) as f32 / self.marker_A.max(1) as f32).min(1.0);
				fade_db(self.AV_reg, (-VOICE_BAR_FADE_DB / 6.0 * t).exp2())
			} else { 0.0 };
			if c < self.marker_A || self.burst_len == 0 {
				// closure (all of an unreleased stop): the previous phone's sound dies away over 5 ms
				let g = self.master_fade * (1.0 - c as f32 / five_ms as f32).max(0.0);
				self.AF_reg = fade_db(self.AF_reg_src, g);
				self.AH_reg = fade_db(self.AH_reg_src, g);
				self.AV_reg = if self.stop_bar { bar } else { fade_db(self.AV_reg_src, g) };
			} else {
				// burst: a sharp onset, then an exponential decay over its length
				let t = (c - self.marker_A) as f32 / self.burst_len.max(1) as f32;
				let g = self.burst_gain * (-self.burst_decay / 6.0 * t).exp2();
				if self.stop_labial {
					// a lip release: weak noise through the vocal tract, low and falling
					self.AF_reg = 0.0;
					self.AH_reg = fade_db(LABIAL_BURST_AH[(self.pft_c & 0x2 != 0) as usize], self.master_fade * g);
				} else {
					self.AF_reg = fade_db(self.AF_reg, g);
					self.AH_reg = 0.0;
				}
				// a voiced affricate's frication is voiced; a stop's burst keeps the voice bar
				if !self.stop_affric { self.AV_reg = bar; }
			}
		}
		if self.asp_db > 0.0 && c >= 0 {
			// aspiration fades out under the first voicing pulses
			let k = 1.0 - c as f32 / samples(ASP_OVERLAP_MS) as f32;
			if k > 0.0 { self.AH_reg = self.AH_reg.max(fade_db(self.asp_db, self.master_fade * k)); }
		}
	}
	fn update_A_to_B(&mut self) {
		let mut dist = self.marker_B - self.ab_at;
		if dist < thirty_ms { dist = thirty_ms; }
		self.slope = 1.0 / dist as f32;
		self.interp = 0.0;
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
		// calculate transition slope here
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
		if dist < thirty_ms { dist = thirty_ms; }
		self.slope = 1.0 / dist as f32;
		self.interp = 0.0;
		self.f1_speed = if self.f1_len > 0 { (dist as f32 / (self.f1_len + self.marker_BA).max(1) as f32).max(1.0) } else { 1.0 };

		// set negative count so locus transition can be completed
		// before we preceed to the next phoneme
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
		self.A2_reg_trg  = self.A2_src;
		self.A3_reg_trg  = self.A3_src;
		self.A4_reg_trg  = self.A4_src;
		self.A5_reg_trg  = self.A5_src;
		self.A6_reg_trg  = self.A6_src;
		self.A7_reg_trg  = self.A7_src;
		self.A8_reg_trg  = self.A8_src;
		self.A9_reg_trg  = self.A9_src;
		self.A10_reg_trg = self.A10_src;
		self.AB_reg_trg  = self.AB_src;
		self.AF_reg_trg  = self.AF_src;
		self.AH_reg_trg  = self.AH_src;
		self.AV_reg_trg  = self.AV_src;
	}
	pub fn load(&mut self, x: u8, d: i32, s: bool, seg: usize, w: bool) -> bool {
		// x is phoneme
		// d is duration (ms)
		// s is stress
		// seg is the segment's stream index (reported back by position)
		// w is true if x starts a word

		self.advance(x, d, s, seg, w);
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
			let mut stop_asp: Option<i32> = None;

			// do ptar before anything else
			self.ptar();
			
			// the previous phone's formant targets at its end, before fetch replaces them
			let prev_f2 = self.F2_B_src;
			let prev_f = [self.F1_B_src, self.F2_B_src, self.F3_B_src];
			let prev_rd = self.RD_B_src;

			// phoneme is ready and loaded up
			// load in formant data
			self.fetch();

			// do formant target re-adjustments before doing anything else
			self.ftar();

			// short vowels fall short of their targets
			self.reduce();

			// consonants take their formants at each edge from the neighbouring vowel
			self.loci(prev_f2);

			// the phrase-end release is the last phone's voice dying away, so it keeps that voice's Rd
			if self.phn_c == 48 || self.phn_c == 49 {
				(self.RD_A_src, self.RD_B_src) = (prev_rd, prev_rd);
			}
			// at the end of a phrase an r keeps its tongue in place while the voice dies away
			if (self.phn_c == 48 || self.phn_c == 49) && matches!(self.phn_l1, 32 | 33 | 46 | 14 | 7) {
				[self.F1_A_src, self.F2_A_src, self.F3_A_src] = prev_f;
				[self.F1_B_src, self.F2_B_src, self.F3_B_src] = prev_f;
			}

			// get duration markers so slopes can be calculated
			if self.phn_c == PAU {
				// pause plays out its whole duration
				self.marker_A = self.pdr_c.min(twenty_ms);
				self.marker_B = self.pdr_c;
			} else if self.pft_c & 0x8040 == 0 {
				// not plosive nor affricate

				// adjust phoneme duration and make sure they don't dip below a minimum threshold.
				//if self.pdr_c < thirty_ms: self.pdr_c = thirty_ms

				//==============================================
				// typical transition times for certain sounds:
				//
				//   + stop  -> 30 ms (~20-40ms)
				//   + fric  -> 40 ms (~30-50ms)
				//   + sonor -> 50 ms (~40-60ms) [default]
				//==============================================
				if self.pft_c & 0x10 != 0 {
					// nasal: the formants arrive by the closure, the murmur holds
					self.marker_A = 0;
				} else if self.pft_c & 0x1C01 == 0 {
					// fricative transition
					if self.pft_r1 & 0x1C10 != 0 {
						// shorten if a sonorant consonant follows (-20ms)
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
					self.marker_A = self.get_control_A_duration(self.C_A_src, self.pdr_c);
				}
				self.marker_B = self.get_control_B_duration(self.C_B_src, self.pdr_c);
				if self.pft_c & 0x1 != 0 {
					// short vowels get a shorter transition so they still reach their target
					self.marker_A = self.marker_A.min((ramp_share * self.marker_B as f32) as i32);
				}
			} else {
				// plosive or affricate: closure, then burst or frication;
				// a voiceless stop's aspiration is carried into the next phone
				let t = stop_timeline(self.phn_c, self.pdr_c, self.phn_l1, self.phn_r1, self.pst_r1, self.pwi_c);
				self.marker_A = t.closure;
				self.marker_B = t.closure + t.burst;
				self.burst_len = t.burst;
				self.stop_on = true;
				self.stop_bar = self.pft_c & 0x2 != 0 && self.pft_l1 & 0x2 != 0;
				self.stop_affric = self.pft_c & 0x8000 != 0;
				self.stop_labial = !self.stop_affric && stop_place(self.phn_c) == 0;
				self.burst_decay = if self.stop_affric { AFFRIC_DECAY_DB } else { BURST_DECAY_DB };
				let unstressed = self.pft_r1 & 0x1 != 0 && !self.pst_r1;
				let sibilant = (self.pft_l1 | self.pft_r1) & 0x200 != 0;
				let gain_db = if self.stop_affric { 0.0 } else {
					BURST_GAIN_DB + if unstressed { BURST_UNSTRESSED_DB } else { 0.0 } + if sibilant { BURST_SIBILANT_DB } else { 0.0 }
				};
				self.burst_gain = (gain_db / 6.0).exp2();
				self.after_stop_next = t.burst > 0;
				stop_asp = Some(t.asp);
			}
			if self.is_first {
				// load in registers if this is the first time
				// and reset counter
				self.marker_BA = self.get_control_A_duration(self.C_B_src, self.pdr_c);
				self.is_first = false;
				self.load_registers();
			} else {
				// do locus calculation here and set counter to negative number
				// so we can do a complete locus transition
				self.update_B_to_Next();
				self.marker_BA = self.get_control_A_duration(self.C_B_src, self.pdr_c);
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
			if let Some(a) = stop_asp {
				self.marker_BA = a;
				if a > 0 {
					let mut level = ASP_DB[stop_place(self.phn_c)];
					if self.pft_r1 & 0x1 != 0 {
						// softer before a front, close vowel
						let [f1, f2, _] = self.next_vowel_formants();
						level -= ASP_FRONT_DB_PER_KHZ * ((f2 - f1) - ASP_FRONT_FROM_HZ).max(0.0) / 1000.0;
					}
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
			// sources switch quickly at obstruent edges; formants keep their own transition
			let obstruent = |f: u16| f != 0 && f & 0x1 == 0 && f & 0x1C10 == 0;
			self.amp_len = if obstruent(self.pft_c) || obstruent(self.pft_l1) { self.marker_A.min(fifteen_ms) } else { self.marker_A };
			self.seg_start = self.counter;
			if self.after_stop {
				// voicing builds up quickly after a release
				self.amp_len = self.amp_len.min(samples(VOICE_ONSET_MS));
			}
			let nasal = |f: u16| f & 0x10 != 0;
			if nasal(self.pft_c) || nasal(self.pft_l1) {
				// a nasal's closure and release are quick
				self.amp_len = samples(NASAL_EDGE_MS);
			}
			if self.phn_c == 18 || self.phn_l1 == 18 {
				self.amp_len = samples(H_EDGE_MS);
			}
			self.amp_start = 0;
			let sibilant = |f: u16| f & 0x8200 == 0x200;
			let sonorant = |f: u16| f & 0x1 != 0 || f & 0x1C10 != 0;
			let fricative = |p: u8| matches!(p, 16 | 37 | 40 | 12 | 34 | 43 | 35 | 44);
			let quiet = |p: u8| p == PAU || p == 0 || p == 48 || p == 49;
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
			} else if (fricative(self.phn_c) && quiet(self.phn_l1)) || (fricative(self.phn_l1) && quiet(self.phn_c)) {
				// out of and into silence, fricative noise rises and falls slowly
				self.amp_len = samples(FRIC_SILENCE_MS);
			}
			if self.asp_db > 0.0 {
				// aspiration: no voicing or frication until this phone's own part starts
				self.AF_reg_src = 0.0;
				self.AV_reg_src = 0.0;
				self.AH_reg_src = self.asp_db;
				self.asp_start = self.counter;
				self.seg_start = 0;
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
		self.update_amp_registers(self.counter, self.amp_len);
		if self.counter == self.ab_at && self.ab_at != self.marker_A {
			// a consonant starting its move to the B targets early
			self.update_A_to_B();
		}
		if self.counter == self.marker_A {
			
			// update registers for diphthongnization (all phonemes)
			if self.ab_at == self.marker_A { self.update_A_to_B(); }
			
		} else if self.counter >= 0 && self.counter < self.marker_A {
			
			// update glottalizer when switching to unvoiced phoneme (for AV and F0)
			if self.pft_c & 0x2 == 0 && self.GL > 0.75 {
				self.GL   -= self.glot_time;
				self.GL_V -= self.glot_time;
			}
				
		} else if self.counter > self.marker_A && self.counter < self.marker_B {
			
			// return glottalizer to 1.0 if less than when switching back to voiced phoneme
			if self.pft_c & 0x2 != 0 && self.GL < 1.0 {
				self.GL   += self.glot_time;
				self.GL_V += self.glot_time;
				if self.GL   > 1.0 { self.GL   = 1.0; }
				if self.GL_V > 1.0 { self.GL_V = 1.0; }
			}

			// do phrase-final fade out if AXP or IXP is the phoneme
			if self.phn_c == 48 || self.phn_c == 49 {
				if self.master_fade > 0.0 {
					self.master_fade -= self.end_time;
					if self.master_fade < 0.0 { self.master_fade = 0.0; }
				}
			} else if self.master_fade < 1.0 {
				self.master_fade += self.optime;
				if self.master_fade > 1.0 { self.master_fade = 1.0; }
			}
			
		}
		// update the interpolator
		self.update_registers()
	}
	pub fn sample(&mut self, f0: f32) -> f32 {
		self.step();
		self.vtm_eng.F0  = f0;
		self.vtm_eng.F1  = self.F1_reg;
		self.vtm_eng.F2  = self.F2_reg;
		self.vtm_eng.F3  = self.F3_reg;
		// aspiration: the open glottis damps F1 and F2, less so as voicing takes over
		let damp = if self.asp_db <= 0.0 { 0.0 } else if self.counter < 0 { 1.0 } else {
			(1.0 - self.counter as f32 / samples(ASP_OVERLAP_MS) as f32).max(0.0)
		};
		self.vtm_eng.B1  = self.B1_reg + damp * ASP_B1_WIDEN;
		self.vtm_eng.B2  = self.B2_reg + damp * ASP_B2_WIDEN;
		self.vtm_eng.B3  = self.B3_reg;
		if self.stop_on && self.stop_labial && self.burst_len > 0 && self.counter >= self.marker_A {
			// a lip release: low and falling whatever the vowel
			self.vtm_eng.F2 = self.F2_reg.min(LABIAL_BURST_F2);
			self.vtm_eng.F3 = self.F3_reg.min(LABIAL_BURST_F3);
		}
		// as the voice dies away at a phrase end it gets breathier and laxer
		let breath = if matches!(self.phn_c, 48 | 49) {
			((self.counter - self.seg_start) as f32 / samples(BREATH_RAMP_MS) as f32).clamp(0.0, 1.0)
		} else { 0.0 };
		self.vtm_eng.RD  = self.RD_reg + BREATH_END_RD * breath;
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
		self.vtm_eng.AV  = fade_db(self.AV_reg, self.GL_V);
		self.vtm_eng.step()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::_utt::PAU;

	// render phones (id, ms) the way main.rs drives the engine
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
		let mut e = engine::new();
		for &(q, dd) in &[(1, 150), (p, d), (1, 150)] {
			if e.load(q, dd, true, 0, true) {
				if e.phn_c == p { break; }
				while e.is_running() { e.sample(120.0); }
			}
		}
		[e.F1_A_src, e.F2_A_src, e.F3_A_src]
	}

	#[test]
	fn reduce_k_grows() {
		assert!((reduce_k(40.0, 40.0) - 0.632).abs() < 0.001);
		assert!((reduce_k(60.0, 40.0) - 0.777).abs() < 0.001);
		assert!((reduce_k(130.0, 40.0) - 0.961).abs() < 0.001);
		assert!(reduce_k(50.0, 40.0) < reduce_k(51.0, 40.0));
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
		let order: Vec<usize> = seen.iter().map(|x| x.0).fold(Vec::new(), |mut v, s| { if v.last() != Some(&s) { v.push(s); } v });
		assert_eq!(order, vec![7, 8, 9, 10]);
		for w in seen.windows(2) {
			if w[0].0 == w[1].0 { assert!(w[1].1 >= w[0].1); } else { assert!(w[0].1 >= 0.95, "seg {} ended at {}", w[0].0, w[0].1); }
		}
	}

	#[test]
	fn faders_scale_linear_gain() {
		// a fader of 0.5 halves the gain (-6 dB), it does not halve the dB value
		assert!((fade_db(60.0, 0.5) - 54.0).abs() < 0.01);
		assert!((fade_db(60.0, 1.0) - 60.0).abs() < 0.01);
		assert_eq!(fade_db(60.0, 0.0), 0.0);
		assert_eq!(fade_db(0.0, 0.5), 0.0);
		assert_eq!(fade_db(3.0, 0.01), 0.0);
	}

	#[test]
	fn amp_ramp_is_linear_in_gain() {
		// halfway from silence to 60 dB is half the gain (54 dB), not 30 dB
		assert!((lerp_db_gain(0.0, 60.0, 0.5) - 54.0).abs() < 0.01);
		assert_eq!(lerp_db_gain(0.0, 60.0, 0.0), 0.0);
		assert!((lerp_db_gain(0.0, 60.0, 1.0) - 60.0).abs() < 0.01);
		assert!((lerp_db_gain(50.0, 50.0, 0.3) - 50.0).abs() < 0.01);
	}

	#[test]
	fn ramp_scales() {
		// a short vowel still reaches its target before its own segment ends
		let mut e = engine::new();
		for &(p, d) in &[(1, 150), (6, 60), (1, 150)] {
			if e.load(p, d, false, 0, true) {
				if e.phn_c == 6 { break; }
				while e.is_running() { e.sample(120.0); }
			}
		}
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

	fn ms(x: f32) -> i32 { (x * m_sr as f32 / 1000.0) as i32 }

	#[test]
	fn stop_timeline_fits() {
		for &p in &[31u8, 36, 23] {
			for &d in &[60.0, 88.0, 101.0, 103.0, 150.0] {
				let pdr = ms(d);
				let t = stop_timeline(p, pdr, 1, 1, true, true);
				assert!((t.closure + t.burst + t.asp - pdr).abs() <= 1, "{} at {} ms: {:?}", p, d, t);
				assert!(t.closure >= min_closure, "{} at {} ms: closure {}", p, d, t.closure);
			}
		}
		// too short for the full release: burst and aspiration shrink together
		let short = stop_timeline(23, ms(60.0), 1, 1, true, true);
		let full = stop_timeline(23, ms(150.0), 1, 1, true, true);
		assert!(short.burst < full.burst && short.asp < full.asp);
		assert!((short.burst as f32 / short.asp as f32 - full.burst as f32 / full.asp as f32).abs() < 0.02);
	}

	#[test]
	fn aspiration_by_context() {
		let pdr = ms(101.0);
		let t = |prev: u8, next: u8, stressed: bool, w: bool| stop_timeline(36, pdr, prev, next, stressed, w);
		let base = t(1, 1, true, true);
		assert!((base.asp - ms(55.0)).abs() <= 1);
		assert_eq!(t(34, 1, true, false).asp, 0, "after s in the word");
		assert_eq!(t(35, 1, true, false).asp, 0, "after sh in the word");
		assert!((t(34, 1, true, true).asp - ms(55.0)).abs() <= 1, "after s across a word start");
		assert!((t(1, 1, false, true).asp - ms(33.0)).abs() <= 1, "before an unstressed vowel");
		assert!((t(1, 24, true, true).asp - ms(27.5)).abs() <= 1, "before l");
		assert_eq!(t(1, 34, true, true).asp, 0, "before s");
		assert_eq!(t(1, 48, false, true).asp, 0, "phrase-final");
		for &n in &[36u8, 27] {
			let s = t(1, n, true, true);
			// unreleased, and its closure merges with the next one
			assert_eq!((s.closure, s.burst, s.asp), ((pdr as f32 * UNRELEASED_SHARE) as i32, 0, 0), "unreleased before {}", n);
		}
		// a shorter aspiration never stretches the closure
		for &(prev, next, st, w) in &[(34u8, 1u8, true, false), (35, 1, true, false), (1, 1, false, true), (1, 24, true, true), (1, 34, true, true), (1, 48, false, true)] {
			assert_eq!(t(prev, next, st, w).closure, base.closure);
		}
	}

	// per-sample (phone, counter, AF, AH, AV, reported position) while rendering (phone, ms, seg)
	fn trace_regs(phones: &[(u8, i32, usize)]) -> (engine, Vec<(u8, i32, f32, f32, f32, (usize, f32))>, Vec<(i32, i32, f32)>) {
		trace_regs_st(phones, &[])
	}

	// as trace_regs, with the segments listed in `unstressed` loaded unstressed
	fn trace_regs_st(phones: &[(u8, i32, usize)], unstressed: &[usize]) -> (engine, Vec<(u8, i32, f32, f32, f32, (usize, f32))>, Vec<(i32, i32, f32)>) {
		let mut e = engine::new();
		let mut out = Vec::new();
		let mut marks = Vec::new();		// (marker_A, marker_B, AF_src) of each phone
		for &(p, d, seg) in phones {
			if e.load(p, d, !unstressed.contains(&seg), seg, true) {
				marks.push((e.marker_A, e.marker_B, e.AF_src));
				while e.is_running() {
					let pos = e.position();
					e.sample(120.0);
					out.push((e.phn_c, e.counter, e.AF_reg, e.AH_reg, e.AV_reg, pos));
				}
			}
		}
		(e, out, marks)
	}

	#[test]
	fn burst_onset_is_sharp() {
		let (_, out, marks) = trace_regs(&[(PAU, 50, 0), (1, 150, 1), (36, 101, 2), (1, 150, 3), (48, 100, 4), (0, 50, 5)]);
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
		let (_, out, _) = trace_regs(&[(PAU, 50, 0), (1, 150, 1), (36, 101, 2), (1, 150, 3), (48, 100, 4), (0, 50, 5)]);
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
		let (e, out, _) = trace_regs(&[(PAU, 50, 0), (1, 150, 1), (36, 101, 2), (1, 150, 3), (48, 100, 4), (0, 50, 5)]);
		let v: Vec<_> = out.iter().filter(|x| x.5.0 == 3).collect();
		let at = |m: f32| v.iter().find(|x| x.1 == ms(m)).unwrap();
		assert!(at(10.0).3 > 0.0, "aspiration gone 10 ms into voicing");
		assert!(at(VOICE_ONSET_MS + 2.0).4 > at(2.0).4 + 3.0, "voicing at full level within 2 ms");
		let _ = e;
	}

	#[test]
	fn voiced_closure_is_quiet() {
		for &(prev, bar) in &[(1u8, true), (34u8, false)] {
			let (_, out, marks) = trace_regs(&[(PAU, 50, 0), (prev, 150, 1), (11, 80, 2), (1, 150, 3), (48, 100, 4), (0, 50, 5)]);
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
	fn affricate_has_closure_then_frication() {
		for &(p, d, fric) in &[(10u8, 100, 55.0), (22u8, 90, 40.0)] {
			// released even before a nasal, with no aspiration
			for &next in &[1u8, 27] {
				let t = stop_timeline(p, ms(d as f32), 1, next, true, true);
				assert!((t.burst - ms(fric)).abs() <= 1 && t.asp == 0 && t.closure + t.burst == ms(d as f32), "{} before {}: {:?}", p, next, t);
			}
			let (_, out, marks) = trace_regs(&[(PAU, 50, 0), (1, 150, 1), (p, d, 2), (1, 150, 3), (48, 100, 4), (0, 50, 5)]);
			let (a, b, _) = marks[2];
			let seg: Vec<_> = out.iter().filter(|x| x.0 == p && x.1 >= 0).collect();
			assert!(seg.iter().filter(|x| x.1 < a).all(|x| x.2 == 0.0), "frication in the closure of {}", p);
			assert!(seg.iter().filter(|x| x.1 >= a && x.1 <= b).all(|x| x.2 > 0.0), "frication gaps in {}", p);
		}
	}

	#[test]
	fn burst_is_softer_before_unstressed_vowel() {
		let onset = |unstressed: &[usize]| {
			let (_, out, marks) = trace_regs_st(&[(PAU, 50, 0), (1, 150, 1), (36, 101, 2), (6, 80, 3), (48, 100, 4), (0, 50, 5)], unstressed);
			let a = marks[2].0;
			out.iter().find(|x| x.0 == 36 && x.1 == a).unwrap().2
		};
		let (st, un) = (onset(&[]), onset(&[3]));
		assert!((st - un + BURST_UNSTRESSED_DB).abs() < 0.5, "stressed {} unstressed {}", st, un);
	}

	// AF at the burst onset of the T in phones (T at segment 2)
	fn t_burst_af(phones: &[(u8, i32, usize)]) -> f32 {
		let (_, out, marks) = trace_regs(phones);
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
			let (_, out, _) = trace_regs(&[(PAU, 50, 0), (1, 150, 1), (f, 150, 2), (1, 150, 3), (48, 100, 4), (0, 50, 5)]);
			let (half, half_out) = (ms(entry / 2.0), ms(exit / 2.0));
			let af = |p: u8, c: i32| out.iter().find(|x| x.0 == p && x.1 == c).unwrap().2;
			let full = af(f, half + ms(10.0));
			assert_eq!(af(f, -half - ms(2.0)), 0.0, "{}: frication before the ramp", f);
			assert!(af(f, -half + ms(2.0)) > 0.0, "{}: frication not started {} ms before the boundary", f, entry / 2.0 - 2.0);
			let mid = af(f, 0);
			assert!(mid > 0.0 && mid < full - 3.0, "{}: frication at the boundary {} (full {})", f, mid, full);
			assert!((af(f, half) - full).abs() < 0.5, "{}: frication not full {} ms after the boundary", f, entry / 2.0);
			assert!((af(1, -half_out - ms(2.0)) - full).abs() < 0.5, "{}: frication fades before the exit ramp", f);
			let out_mid = af(1, 0);
			assert!(out_mid > 0.0 && out_mid < full - 3.0, "{}: frication at the exit boundary {}", f, out_mid);
			assert_eq!(af(1, half_out + ms(1.0)), 0.0, "{}: frication after the exit ramp", f);
		}
	}

	#[test]
	fn unreleased_stop_stays_silent() {
		// AA-K-T-AA: no frication anywhere in the K or in the T's closure
		let (_, out, marks) = trace_regs(&[(PAU, 50, 0), (1, 150, 1), (23, 103, 2), (36, 101, 3), (1, 150, 4), (48, 100, 5), (0, 50, 6)]);
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
		let mut e = engine::new();
		let mut m: Vec<f32> = Vec::new();
		for &(p, d) in &[(PAU, 50), (1, 150), (26, 120), (1, 150), (48, 100), (0, 50)] {
			if e.load(p, d, true, 0, true) {
				while e.is_running() {
					let y = e.sample(120.0);
					if e.phn_c == 26 && e.counter >= ms(40.0) && e.counter < ms(100.0) { m.push(y); }
				}
			}
		}
		let low = band_db(&m, 250.0);
		let f2 = band_db(&m, 1100.0);
		assert!(f2 - low > -35.0, "murmur F2 region {} dB under its low peak", low - f2);
	}

	#[test]
	fn nasal_edges_are_fast() {
		// AA-M-AA: the nasal coupling and the voicing level switch within NASAL_EDGE_MS of each boundary
		let mut e = engine::new();
		let mut tr: Vec<(u8, i32, f32, f32)> = Vec::new();		// (phone, counter, coupling, AV)
		let mut f1: Vec<(u8, i32, f32, f32)> = Vec::new();		// (phone, counter, F1, the phone's F1 target)
		for &(p, d) in &[(PAU, 50), (1, 150), (26, 120), (1, 150), (48, 100), (0, 50)] {
			if e.load(p, d, true, 0, true) {
				while e.is_running() {
					e.sample(120.0);
					tr.push((e.phn_c, e.counter, e.vtm_eng.NC, e.AV_reg));
					f1.push((e.phn_c, e.counter, e.F1_reg, e.F1_A_src));
				}
			}
		}
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
		let mut e = engine::new();
		for &(q, d) in &[(PAU, 50), (v1, 1000), (c, 90), (v2, 1000)] {
			if e.load(q, d, true, 0, true) {
				if e.phn_c == c { break; }
				while e.is_running() { e.sample(120.0); }
			}
		}
		(e.F2_A_src, e.F2_B_src, e.F1_A_src)
	}

	#[test]
	fn consonant_f2_follows_the_vowel() {
		// the consonant's F2 on each side moves with the neighbouring vowel's F2, by its place's slope
		let f2 = |v: u8| formant_tbl[v as usize * 31 + 6] as f32;
		for &c in &[9u8, 11, 17, 26, 27, 34] {
			let slope = locus(c).unwrap().0;
			let (_, b_iy, _) = cons_targets(6, c, 21);
			let (_, b_uw, _) = cons_targets(6, c, 39);
			let want = slope * (f2(21) - f2(39));
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
			let mut e = engine::new();
			let (mut last, mut target) = (0.0f32, 0.0f32);
			for &(q, d) in &[(PAU, 50), (6, 120), (c, 90), (21, 200), (48, 100), (0, 50)] {
				if e.load(q, d, true, 0, true) {
					while e.is_running() {
						e.sample(120.0);
						if e.phn_c == c { last = e.F2_reg; target = e.F2_B_src; }
					}
				}
			}
			assert!((last - target).abs() < 30.0, "phone {} ends at F2 {} not {}", c, last, target);
		}
	}

	// the burst samples (and AF/AH during them) of stop c, sentence-initially before vowel v
	fn burst_of(c: u8, v: u8) -> (Vec<f32>, Vec<(f32, f32)>) {
		let mut e = engine::new();
		let (mut x, mut amps) = (Vec::new(), Vec::new());
		for &(p, d) in &[(PAU, 50), (c, 100), (v, 200), (48, 100), (0, 50)] {
			if e.load(p, d, true, 0, true) {
				while e.is_running() {
					let y = e.sample(120.0);
					if e.phn_c == c && e.counter >= e.marker_A { x.push(y); amps.push((e.AF_reg, e.AH_reg)); }
				}
			}
		}
		(x, amps)
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
		for &c in &[23u8, 17] {
			let (front, _) = burst_of(c, 21);
			let (back, _) = burst_of(c, 1);
			assert!(band_db(&front, 2400.0) > band_db(&front, 1400.0) + 3.0, "{} before iy: no high peak", c);
			assert!(band_db(&back, 1400.0) > band_db(&back, 2400.0) + 3.0, "{} before aa: no low peak", c);
		}
	}

	#[test]
	fn velar_f3_follows_the_vowel() {
		// velar F3 sits near F2 before a front vowel and lower before a back one
		let f2 = |v: u8| formant_tbl[v as usize * 31 + 6] as f32;
		for &c in &[17u8, 23] {
			let mut f3 = Vec::new();
			for &v in &[21u8, 1] {
				let mut e2 = engine::new();
				for &(q, d) in &[(PAU, 50), (6, 1000), (c, 90), (v, 1000)] {
					if e2.load(q, d, true, 0, true) {
						if e2.phn_c == c { break; }
						while e2.is_running() { e2.sample(120.0); }
					}
				}
				f3.push(e2.F3_B_src);
				let want = VELAR_F3.1 + VELAR_F3.0 * f2(v);
				assert!((e2.F3_B_src - want).abs() < 0.03 * want, "{} before {}: F3 {} not {}", c, v, e2.F3_B_src, want);
			}
			assert!(f3[0] > f3[1] + 400.0, "{}: F3 {} before iy, {} before aa", c, f3[0], f3[1]);
		}
	}

	#[test]
	fn labial_burst_leaves_no_transient() {
		// the vowel after a lip release peaks no higher than after a tongue release
		let peak = |c: u8| {
			let mut e = engine::new();
			let mut m = 0.0f32;
			for &(p, d) in &[(PAU, 50), (c, 90), (8, 250), (48, 100), (0, 50)] {
				if e.load(p, d, true, 0, true) {
					while e.is_running() { m = m.max(e.sample(120.0).abs()); }
				}
			}
			m
		};
		let (b, d) = (peak(9), peak(11));
		assert!(b < 1.15 * d, "peak {} after b, {} after d", b, d);
	}

	#[test]
	fn aspiration_damps_f1() {
		// the open glottis damps F1 (and a little F2) during aspiration; the vowel's own part is back to normal
		let mut e = engine::new();
		let (mut asp, mut voiced) = (Vec::new(), Vec::new());
		for &(p, d) in &[(PAU, 50), (23, 100), (1, 200), (48, 100), (0, 50)] {
			if e.load(p, d, true, 0, true) {
				while e.is_running() {
					e.sample(120.0);
					let wide = (e.vtm_eng.B1 - e.B1_reg, e.vtm_eng.B2 - e.B2_reg);
					if e.phn_c == 1 && e.counter < 0 { asp.push(wide); }
					if e.phn_c == 1 && e.counter > ms(ASP_OVERLAP_MS + 5.0) { voiced.push(wide); }
				}
			}
		}
		assert!(!asp.is_empty());
		assert!(asp.iter().all(|w| w.0 >= ASP_B1_WIDEN - 30.0 && w.1 >= ASP_B2_WIDEN - 20.0), "aspiration not damped: {:?}", asp[0]);
		assert!(voiced.iter().all(|w| w.0 < ASP_B1_WIDEN / 2.0), "voiced vowel still damped: {:?}", voiced[0]);
	}

	// per sample (phone, counter, AF gain, AH dB) through phones
	fn noise_track(phones: &[(u8, i32)]) -> Vec<(u8, i32, f32, f32)> {
		let mut e = engine::new();
		let g = |db: f32| if db <= 0.0 { 0.0 } else { 0.5_f32.powf((87.0 - db) / 6.0) };
		let mut out = Vec::new();
		for &(p, d) in phones.iter().chain([(48u8, 100), (0, 50)].iter()) {
			if e.load(p, d, true, 0, true) {
				while e.is_running() {
					e.sample(120.0);
					out.push((e.phn_c, e.counter, g(e.AF_reg), e.AH_reg));
				}
			}
		}
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
		let t = noise_track(&[(PAU, 50), (1, 150), (16, 110), (1, 150)]);
		// the F's carried-over region is the first vowel's tail; the second vowel starts breathy
		assert!(t.iter().any(|x| x.0 == 16 && x.1 < 0 && x.3 > 0.0), "no breath at the vowel's end");
		let after: Vec<_> = t.iter().filter(|x| x.0 == 1).collect();
		let start = after.iter().filter(|x| x.1 == ms(5.0)).last().unwrap();
		assert!(start.3 > 0.0, "no breath at the next vowel's start");
	}

	#[test]
	fn tongue_consonants_glide_slowly_into_back_vowels() {
		// out of d, F2 still moves 50 ms into a back vowel but has arrived in a front one
		let f2_at = |v: u8, at: f32| {
			let mut e = engine::new();
			let mut f = (0.0, 0.0);
			for &(p, d) in &[(PAU, 50), (6, 100), (11, 80), (v, 250), (48, 100), (0, 50)] {
				if e.load(p, d, true, 0, true) {
					while e.is_running() {
						e.sample(120.0);
						if e.phn_c == v && e.counter == ms(at) { f = (e.F2_reg, e.F2_A_src); }
					}
				}
			}
			f
		};
		let (back, back_t) = f2_at(4, 50.0);
		assert!((back - back_t).abs() > 30.0, "F2 {} already at {} 50 ms into ao", back, back_t);
		let (back, back_t) = f2_at(4, BACK_TRANSITION_MS + 1.0);
		assert!((back - back_t).abs() < 5.0, "F2 {} not at {} by {} ms", back, back_t, BACK_TRANSITION_MS);
		let (front, front_t) = f2_at(21, 51.0);
		assert!((front - front_t).abs() < 5.0, "F2 {} not at {} 50 ms into iy", front, front_t);
	}

	#[test]
	fn aspiration_is_softer_before_front_vowels() {
		// the aspiration a stop leaves for the next vowel drops with how front that vowel is
		let asp_before = |v: u8| {
			let mut e = engine::new();
			for &(q, d) in &[(PAU, 50), (6, 100), (36, 101), (v, 1000)] {
				if e.load(q, d, true, 0, true) {
					if e.phn_c == 36 { break; }
					while e.is_running() { e.sample(120.0); }
				}
			}
			e.asp_db_next
		};
		let gap = |v: u8| formant_tbl[v as usize * 31 + 6] as f32 - formant_tbl[v as usize * 31 + 5] as f32;
		let want = ASP_FRONT_DB_PER_KHZ * (gap(21) - ASP_FRONT_FROM_HZ) / 1000.0;
		assert!(want > 3.0);
		assert!(((asp_before(1) - asp_before(21)) - want).abs() < 0.3, "cut {} before iy, want {}", asp_before(1) - asp_before(21), want);
		assert_eq!(asp_before(1), ASP_DB[1], "aspiration before aa changed");
	}

	// (F1 at A, F2 at A, F2 at B) of stressed vowel v between consonants c1 and c2
	fn vowel_targets(c1: u8, v: u8, c2: u8) -> (f32, f32, f32) {
		let mut e = engine::new();
		for &(q, d, st) in &[(PAU, 50, false), (6, 100, false), (c1, 80, false), (v, 200, true), (c2, 80, false), (6, 100, false)] {
			if e.load(q, d, st, 0, true) {
				if e.phn_c == v { break; }
				while e.is_running() { e.sample(120.0); }
			}
		}
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
	fn only_alveolar_obstruents_front_uw() {
		let (_, a_b, b_b) = vowel_targets(9, 39, 9);
		let (_, a_d, b_d) = vowel_targets(11, 39, 11);
		assert!(a_d > a_b + 100.0 && b_d > b_b + 100.0, "uw not fronted next to d: {} {} vs {} {}", a_d, b_d, a_b, b_b);
		for &(l, r) in &[(24u8, 25u8), (32, 33)] {
			let (_, a, b) = vowel_targets(l, 39, r);
			assert!((a - a_b).abs() < 60.0 && (b - b_b).abs() < 60.0, "uw fronted next to {}/{}: {} {} vs {} {}", l, r, a, b, a_b, b_b);
		}
	}

	#[test]
	fn f1_opens_into_a_back_vowel_before_f2_arrives() {
		// out of d, ao's F1 is at its target while F2 is still gliding
		let mut e = engine::new();
		let mut at = None;
		for &(p, d) in &[(PAU, 50), (6, 100), (11, 80), (4, 250), (48, 100), (0, 50)] {
			if e.load(p, d, true, 0, true) {
				while e.is_running() {
					e.sample(120.0);
					if e.phn_c == 4 && e.counter == ms(BACK_F1_MS + 5.0) { at = Some((e.F1_reg, e.F1_A_src, e.F2_reg, e.F2_A_src)); }
				}
			}
		}
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
		assert!(uw <= ASP_DB[0] + 1.0, "h before uw at {} dB", uw);
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
			let mut e = engine::new();
			let mut f = 0.0;
			for &(p, d) in &[(PAU, 50), (6, 100), (c, 80), (29, 250), (48, 100), (0, 50)] {
				if e.load(p, d, true, 0, true) {
					while e.is_running() {
						e.sample(120.0);
						if e.phn_c == 29 && e.counter == ms(90.0) { f = e.F1_reg; }
					}
				}
			}
			f
		};
		let (b, d) = (f1(9), f1(11));
		assert!((b - d).abs() < 15.0, "ow F1 at 90 ms: {} after b, {} after d", b, d);
	}

	#[test]
	fn vowels_are_not_pressed() {
		for &v in &[1u8, 2, 3, 4, 5, 6, 7, 8, 13, 14, 15, 19, 20, 21, 29, 30, 39] {
			let o = v as usize * 31;
			let (a, b) = (formant_tbl[o + 2], formant_tbl[o + 3]);
			assert!(a >= 80 && b >= 75, "vowel {} Rd {} / {}", v, a, b);
		}
	}

	#[test]
	fn r_colors_only_the_end_of_a_vowel() {
		// before r a vowel starts as before t and its F3 is RHOTIC_F3 lower only at its end, stressed or not
		let f3 = |v: u8, c2: u8, st: bool| {
			let mut e = engine::new();
			for &(q, d, s) in &[(PAU, 50, false), (6, 100, false), (23, 80, false), (v, 200, st), (c2, 80, false), (6, 100, false)] {
				if e.load(q, d, s, 0, true) {
					if e.phn_c == v { break; }
					while e.is_running() { e.sample(120.0); }
				}
			}
			(e.F3_A_src, e.F3_B_src)
		};
		for &v in &[1u8, 13, 21, 4] {
			for &st in &[true, false] {
				let ((ra, rb), (ta, tb)) = (f3(v, 33, st), f3(v, 36, st));
				assert!((ra - ta).abs() < 1.0, "vowel {} starts at F3 {} before r, {} before t", v, ra, ta);
				// reduction toward the neutral vowel scales the shift a little
				assert!((tb - rb - RHOTIC_F3).abs() < 5.0, "vowel {} ends at F3 {} before r, {} before t", v, rb, tb);
			}
		}
	}

	// per sample (phone, F3, AV, the phone's own F3 A target) while rendering (phone, ms, stressed)
	fn trace_f3(phones: &[(u8, i32, bool)]) -> Vec<(u8, f32, f32, f32)> {
		let mut e = engine::new();
		let mut out = Vec::new();
		for &(p, d, st) in phones {
			if e.load(p, d, st, 0, true) {
				while e.is_running() {
					e.sample(120.0);
					out.push((e.phn_c, e.F3_reg, e.AV_reg, e.F3_A_src));
				}
			}
		}
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

	// per sample (phone, counter, F1, F2, the F2 being moved toward) while rendering (phone, ms, stressed)
	fn trace_moves(phones: &[(u8, i32, bool)]) -> Vec<(u8, i32, f32, f32, f32)> {
		let mut e = engine::new();
		let mut out = Vec::new();
		for &(p, d, st) in phones {
			if e.load(p, d, st, 0, true) {
				while e.is_running() {
					e.sample(120.0);
					out.push((e.phn_c, e.counter, e.F1_reg, e.F2_reg, e.F2_reg_trg));
				}
			}
		}
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
		let mut e = engine::new();
		let mut out = Vec::new();
		for &(p, d, st) in phones {
			if e.load(p, d, st, 0, true) {
				while e.is_running() {
					e.sample(120.0);
					out.push((e.phn_c, e.vtm_eng.BR, e.vtm_eng.RD));
				}
			}
		}
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
		let mut e = engine::new();
		let mut out = Vec::new();
		for &(p, d, st) in &[(PAU, 50, false), (23, 90, false), (1, 200, true), (48, 80, false), (0, 50, false)] {
			if e.load(p, d, st, 0, true) {
				while e.is_running() {
					e.sample(120.0);
					out.push((e.phn_c, e.RD_reg));
				}
			}
		}
		let end = out.iter().filter(|x| x.0 == 1).last().unwrap().1;
		assert!(out.iter().filter(|x| x.0 == 48).all(|x| (x.1 - end).abs() < 0.02), "Rd falls from {} in the release", end);
	}

	#[test]
	fn a_short_liquid_or_glide_arrives_early() {
		// l, r, w and y are heard by where their formants go, so even a short one is mostly there by its middle
		for &c in &[24u8, 32, 41, 42] {
			let out = trace_moves(&[(PAU, 50, false), (1, 140, true), (c, 40, false), (1, 140, false), (48, 80, false), (0, 50, false)]);
			let start = out.iter().find(|x| x.0 == c).unwrap().3;
			let own: Vec<_> = out.iter().filter(|x| x.0 == c && x.1 >= 0).collect();
			let (mid, end) = (own[own.len() / 2].3, own[own.len() - 1].3);
			assert!((mid - end).abs() <= 0.1 * (start - end).abs(), "phone {}: F2 {} -> {} at its middle, {} at its end", c, start, mid, end);
		}
	}
}
