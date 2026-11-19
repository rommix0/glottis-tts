/*

#########################################
# VOCAL TRACT MODEL FOR TTS SYNTHESIZER #
#########################################

  author: anthony c. bartman (@rommix0)
  date:   4/6/2026 - 11/17/2026

*/

use core::f32::consts::PI;
use crate::_f0::REF_HZ;
use crate::_phone::IXP;
use crate::_vtbl::{formant_tbl, ROW_LEN};

// master sample rate: 22050 Hz, or 44100 Hz with the rate-44100 feature
pub const m_sr: u32 = if cfg!(feature = "rate-44100") { 44100 } else { 22050 };

// soft clip default
const CLIP_LIMIT: f32 = 0.85;

// lip radiation coefficient, tuned at LIP_TUNED_SR. at a lower rate the filter tilts brighter;
// LIP_MATCH is how much of that extra tilt to take back (0 none, 1 back to the tuned response)
const LIP_D: f32 = 0.5;
const LIP_TUNED_SR: f32 = 32000.0;
const LIP_MATCH: f32 = 0.5;
// at 44100 Hz the coefficient whose tilt best fits the 22050 Hz filter's up to 10 kHz (within 0.71 dB)
const LIP_D_44100: f32 = 0.6;

// jitter and shimmer draw uniformly up to this many times their average pulse-to-pulse change,
// so the voice profile's numbers are averages (the average is 2/3 of the largest draw)
const PERTURB_SPREAD: f32 = 1.5;

// output trim, keeps peaks under the soft clip
const OUT_DB: f32 = -11.5;

// makeup gain for the cascade branch (unity DC gain per resonator)
const CASC_GAIN: f32 = 2328.0;

// frication noise: rolled off above this, so consonants are not too bright
const FRIC_LP_HZ: f32 = 4500.0;
// at 44100 Hz the corner whose roll-off best fits the 22050 Hz one up to 10 kHz (within 0.5 dB)
const FRIC_LP_HZ_44100: f32 = 5700.0;

// narrowest F1/F2 bandwidth (Hz) the cascade uses
const CASC_MIN_BW: f32 = 60.0;
// the narrowest any other formant gets (Hz)
const MIN_BW: f32 = 40.0;

// a tuned F1 stays at least this far under F2
const F1_TUNE_BELOW_F2: f32 = 200.0;
// a raised F1 lifts every harmonic above it by about the square of its rise: give back this power of the rise,
// so the note comes out fuller, not louder
const F1_TUNE_GAIN_POW: f32 = 2.0;

// dB to an amplitude ratio
fn db_amp(db: f32) -> f32 { libm::powf(10.0, db / 20.0) }

// amplitudes in dB are relative to this full scale, and every DB_PER_HALF dB down halves the gain
pub const G_MAX: f32 = 87.0;
pub const DB_PER_HALF: f32 = 6.0;

// gain for each whole dB from 0 to G_MAX: 0.5^((G_MAX - dB) / 6)
pub const DB_GAIN: [f32; 88] = [
	4.3158372e-05, 4.8443635e-05, 5.437614e-05, 6.1035156e-05,
	6.8509646e-05, 7.689948e-05, 8.6316744e-05, 9.688727e-05,
	0.00010875228, 0.00012207031, 0.00013701929, 0.00015379896,
	0.00017263349, 0.00019377454, 0.00021750457, 0.00024414062,
	0.00027403858, 0.00030759792, 0.00034526698, 0.00038754908,
	0.00043500913, 0.00048828125, 0.00054807717, 0.00061519584,
	0.00069053395, 0.00077509816, 0.00087001827, 0.0009765625,
	0.0010961543, 0.0012303917, 0.0013810679, 0.0015501963,
	0.0017400365, 0.001953125, 0.0021923087, 0.0024607833,
	0.0027621358, 0.0031003926, 0.003480073, 0.00390625,
	0.0043846173, 0.0049215667, 0.0055242716, 0.0062007853,
	0.006960146, 0.0078125, 0.008769235, 0.009843133,
	0.011048543, 0.012401571, 0.013920292, 0.015625,
	0.01753847, 0.019686267, 0.022097087, 0.024803141,
	0.027840585, 0.03125, 0.03507694, 0.039372534,
	0.044194173, 0.049606282, 0.05568117, 0.0625,
	0.07015388, 0.07874507, 0.088388346, 0.099212565,
	0.11136234, 0.125, 0.14030775, 0.15749013,
	0.17677669, 0.19842513, 0.22272468, 0.25,
	0.2806155, 0.31498027, 0.35355338, 0.39685026,
	0.44544935, 0.5, 0.561231, 0.62996054,
	0.70710677, 0.7937005, 0.8908987, 1.0,
];

// formants closer than MIN_D (Hz) get their bandwidths widened, by up to 1 + MUL_B times when they meet
const MIN_D: f32 = 350.0;
const MUL_B: f32 = 2.5;
fn bw_widen(gap: f32) -> f32 { if gap < MIN_D { MUL_B * ((MIN_D - gap) / MIN_D) + 1.0 } else { 1.0 } }
// widening for B1-B3, each formant by its nearest neighbour (F1 only has F2, F3 has F2 and F4)
fn bw_widen3(f1: f32, f2: f32, f3: f32, f4: f32) -> [f32; 3] {
	let b2 = bw_widen((f2 - f3).abs());
	[bw_widen((f1 - f2).abs()), b2, bw_widen((f3 - f4).abs()).max(b2)]
}
// a fixed upper formant (F4 and up) at the voice's scale: its frequency and bandwidth
fn upper_formant(v: &Voice, hz: f32, bw: f32) -> (f32, f32) {
	((hz * (v.fhi / 100.0)).min(F_MAX_SR * RES_SR), bw * (v.fhi / 100.0))
}

// breath noise follows the glottal flow, whose power over a period is oq/2 of steady noise's:
// this times 1/sqrt(oq) keeps the average breath level
const BREATH_FLOW_GAIN: f32 = core::f32::consts::SQRT_2;

// makeup gain for the parallel branch (frication)
const PARR_GAIN: f32 = 7.5;

// init constants and nasal murmur
const RA_REF:   f32 = 3.8;
const RD_MIN:   f32 = 0.3;
const F1_INIT:  f32 = 500.0;
const B1_INIT:  f32 = 60.0;
const F2_INIT:  f32 = 1500.0;
const B2_INIT:  f32 = 90.0;
const F3_INIT:  f32 = 2500.0;
const B3_INIT:  f32 = 150.0;
const F4_INIT:  f32 = 3750.0;
const B4_INIT:  f32 = 250.0;
const F5_INIT:  f32 = 4500.0;
const B5_INIT:  f32 = 350.0;
const F6_INIT:  f32 = 5200.0;
const B6_INIT:  f32 = 550.0;
const F7_INIT:  f32 = 6500.0;
const B7_INIT:  f32 = 722.0;
const F8_INIT:  f32 = 7500.0;
const B8_INIT:  f32 = 1250.0;
const F9_INIT:  f32 = 8500.0;
const B9_INIT:  f32 = 2125.0;
const F10_INIT: f32 = 9500.0;
const B10_INIT: f32 = 4750.0;
const FN_INIT: f32 = 250.0;
const BN_INIT: f32 = 100.0;
const BN_MURMUR: f32 = 200.0;
const FN_MURMUR: f32 = 320.0;

// highest frequency a scaled formant may reach, as a share of the sample rate
const F_MAX_SR: f32 = 0.45;

// the resonators and the glottal filters at 44100 Hz feed back every second sample, so below 11 kHz they answer
// exactly as at 22050 Hz, where they lift the top of the band (a brightness the voice was tuned with); a low-pass at
// OUT_LP_HZ on the output takes away the mirror image this makes above 11 kHz, and any hiss the voice never had.
// the noise goes through the same low-pass before the resonators, which would fold its top into 11-14 kHz
const RES_STRIDE: usize = (m_sr / 22050) as usize;
const RES_SR: f32 = 22050.0;
const OUT_LP_HZ: f32 = 11000.0;
// the low-pass is 6th order: three sections at these Q
const OUT_LP_Q: [f32; 3] = [0.5176, 0.7071, 1.9319];

// a voice profile: scales in percent of the formant table's voice (100 leaves it alone), plus source changes
#[derive(Clone, Copy)]
pub struct Voice {
	pub f0:    f32,		// pitch
	pub range: f32,		// pitch excursion around the contour's reference pitch
	pub f1:    f32,		// F1
	pub f2:    f32,		// F2
	pub f3:    f32,		// F3
	pub fhi:   f32,		// F4 and up, and the nasal pole
	pub rd:    f32,		// glottal tilt (Rd), higher is softer
	pub br:    f32,		// breath noise added, in dB
	pub db:    f32,		// output level change, in dB, to match the table voice's loudness
	pub fl:    f32,		// slow pitch flutter, on the KLGLOTT88 flutter scale (100 is strong)
	pub jit:   f32,		// jitter: average change of the period from one pulse to the next, in %
	pub shim:  f32,		// shimmer: average change of the level from one pulse to the next, in dB
}

// voice profiles, picked by number; 0-4 follow head size, 5 and up are silly
pub const VOICES: [Voice; 8] = [
	// 0 table voice
	Voice { f0: 100.0, range: 100.0, f1: 100.0, f2: 100.0, f3: 100.0, fhi: 100.0,
		rd: 100.0, br: 0.0, db:  0.8, fl:  25.0, jit: 0.2, shim: 0.2 },
	// 1 large head
	Voice { f0:  85.0, range:  95.0, f1:  90.0, f2:  91.0, f3:  92.0, fhi:  92.0,
		rd: 100.0, br: 0.0, db:  0.9, fl:  25.0, jit: 0.2, shim: 0.2 },
	// 2 female
	Voice { f0: 150.0, range:  85.0, f1: 136.0, f2: 113.0, f3: 117.0, fhi: 111.0,
		rd: 240.0, br: 8.0, db:  2.2, fl:  25.0, jit: 0.2, shim: 0.2 },
	// 3 child
	Voice { f0: 235.0, range: 120.0, f1: 135.0, f2: 130.0, f3: 127.0, fhi: 124.0,
		rd: 125.0, br: 2.0, db: -3.1, fl:  25.0, jit: 0.2, shim: 0.2 },
	// 4 giant
	Voice { f0:  65.0, range:  80.0, f1:  80.0, f2:  82.0, f3:  83.0, fhi:  84.0,
		rd:  95.0, br: 0.0, db:  2.1, fl:  25.0, jit: 0.6, shim: 0.4 },
	// 5 deep voice, tiny head
	Voice { f0:  70.0, range: 100.0, f1: 135.0, f2: 132.0, f3: 130.0, fhi: 125.0,
		rd: 100.0, br: 0.0, db:  2.6, fl:  25.0, jit: 0.2, shim: 0.2 },
	// 6 pulled apart
	Voice { f0: 100.0, range: 160.0, f1:  70.0, f2: 130.0, f3:  85.0, fhi: 100.0,
		rd: 100.0, br: 0.0, db:  0.6, fl:  25.0, jit: 0.2, shim: 0.2 },
	// 7 shaky
	Voice { f0:  95.0, range: 110.0, f1: 100.0, f2: 100.0, f3: 100.0, fhi: 100.0,
		rd: 110.0, br: 2.0, db:  1.1, fl: 250.0, jit: 1.5, shim: 1.0 },
];

// measure aspiration's loudness at a voice's scale: white noise through the cascade and lips, times the output level.
// a bigger scale lets more noise through, but the voice's db only evens out its vowels
fn asp_level(v: &Voice) -> f32 {
	let top = F_MAX_SR * m_sr as f32;
	let (s1, s2, s3) = (v.f1 / 100.0, v.f2 / 100.0, v.f3 / 100.0);
	// F1-F3 and their widths as in a release (the table's row), widened like fix_bw;
	// upper formants as set_voice builds them
	let row = |k: usize| formant_tbl[IXP as usize * ROW_LEN + k] as f32;
	let (f1, f2, f3) = (row(5) * s1, row(6) * s2, row(7) * s3);
	let [a1, a2, a3] = bw_widen3(f1, f2, f3, F4_INIT * (v.fhi / 100.0));
	let upper = |hz: f32, bw: f32| {
		let (f, b) = upper_formant(v, hz, bw);
		resonator::new(f, b)
	};
	let mut rs = [
		resonator::new(f1.min(top), row(8) * s1 * a1),
		resonator::new(f2.min(top), row(9) * s2 * a2),
		resonator::new(f3.min(top), row(10) * s3 * a3),
		upper(F4_INIT, B4_INIT), upper(F5_INIT, B5_INIT), upper(F6_INIT, B6_INIT), upper(F7_INIT, B7_INIT),
		upper(F8_INIT, B8_INIT),
	];
	let (mut e, mut prev) = (0.0f32, 0.0f32);
	for n in 0..4096 {
		let mut y = if n == 0 { 1.0 } else { 0.0 };
		for r in rs.iter_mut() { y = r.step(y); }
		e += (y - prev) * (y - prev);
		prev = y;
	}
	libm::sqrtf(e) * db_amp(v.db)
}

struct noise { nxt: u32 }
impl noise {
	fn new() -> noise { noise { nxt: 0x2545F491 } }
	fn xorshift(&mut self) {
		// 32-bit state so it repeats after 2^32 - 1 samples, not within seconds
		self.nxt ^= self.nxt << 13;
		self.nxt ^= self.nxt >> 17;
		self.nxt ^= self.nxt << 5;
	}
	fn step(&mut self) -> i32 {
		// white noise, Q27 (-1 to 1)
		self.xorshift();
		(((self.nxt >> 16) as u16 as i16) as i32) << (SIG_BITS - 15)
	}
}

// cos over 0 to pi (0 Hz to half the sample rate) in COS_STEPS steps, for the resonators' coefficients
const COS_STEPS: usize = 512;
const COS_TABLE: [f32; COS_STEPS + 1] = [
	1.0, 0.99998116, 0.9999247, 0.9998306, 0.9996988, 0.9995294, 0.99932235, 0.99907774,
	0.99879545, 0.99847555, 0.9981181, 0.99772304, 0.99729043, 0.9968203, 0.9963126, 0.9957674,
	0.9951847, 0.9945646, 0.993907, 0.9932119, 0.99247956, 0.99170977, 0.99090266, 0.9900582,
	0.9891765, 0.9882576, 0.9873014, 0.9863081, 0.98527765, 0.9842101, 0.9831055, 0.9819639,
	0.98078525, 0.9795698, 0.9783174, 0.97702813, 0.9757021, 0.97433937, 0.97293997, 0.9715039,
	0.97003126, 0.9685221, 0.96697646, 0.96539444, 0.96377605, 0.9621214, 0.9604305, 0.95870346,
	0.95694035, 0.9551412, 0.953306, 0.951435, 0.94952816, 0.9475856, 0.9456073, 0.94359344,
	0.94154406, 0.9394592, 0.937339, 0.9351835, 0.9329928, 0.93076694, 0.9285061, 0.9262102,
	0.9238795, 0.92151403, 0.9191139, 0.9166791, 0.9142098, 0.91170603, 0.909168, 0.9065957,
	0.9039893, 0.9013488, 0.8986745, 0.89596623, 0.8932243, 0.89044875, 0.88763964, 0.8847971,
	0.8819213, 0.8790122, 0.8760701, 0.873095, 0.87008697, 0.86704624, 0.86397284, 0.86086696,
	0.8577286, 0.854558, 0.8513552, 0.84812033, 0.8448536, 0.841555, 0.8382247, 0.8348629,
	0.8314696, 0.82804507, 0.8245893, 0.8211025, 0.8175848, 0.8140363, 0.81045717, 0.8068476,
	0.8032075, 0.79953724, 0.7958369, 0.79210657, 0.7883464, 0.78455657, 0.7807372, 0.7768885,
	0.77301043, 0.76910335, 0.76516724, 0.7612024, 0.7572088, 0.7531868, 0.7491364, 0.74505776,
	0.7409511, 0.7368166, 0.7326543, 0.72846437, 0.7242471, 0.72000253, 0.71573085, 0.7114322,
	0.70710677, 0.70275474, 0.69837624, 0.69397146, 0.68954057, 0.6850837, 0.680601, 0.6760927,
	0.671559, 0.66699994, 0.6624158, 0.6578067, 0.65317285, 0.6485144, 0.64383155, 0.63912445,
	0.6343933, 0.62963825, 0.6248595, 0.6200572, 0.6152316, 0.6103828, 0.60551107, 0.60061646,
	0.5956993, 0.5907597, 0.58579785, 0.58081394, 0.57580817, 0.57078075, 0.5657318, 0.56066155,
	0.55557024, 0.55045795, 0.545325, 0.54017144, 0.53499764, 0.52980363, 0.52458966, 0.519356,
	0.51410276, 0.50883013, 0.50353837, 0.49822766, 0.4928982, 0.48755017, 0.48218378, 0.47679922,
	0.47139674, 0.4659765, 0.46053872, 0.45508358, 0.44961134, 0.44412214, 0.43861625, 0.43309382,
	0.42755508, 0.42200026, 0.41642955, 0.41084316, 0.4052413, 0.3996242, 0.39399204, 0.38834503,
	0.38268343, 0.37700742, 0.3713172, 0.36561298, 0.35989505, 0.35416353, 0.34841868, 0.34266073,
	0.33688986, 0.3311063, 0.3253103, 0.31950203, 0.31368175, 0.30784965, 0.30200595, 0.2961509,
	0.29028466, 0.28440753, 0.2785197, 0.27262136, 0.26671275, 0.2607941, 0.25486565, 0.24892761,
	0.24298018, 0.2370236, 0.2310581, 0.22508392, 0.21910124, 0.21311031, 0.20711137, 0.20110464,
	0.19509032, 0.18906866, 0.18303989, 0.17700422, 0.17096189, 0.16491312, 0.15885815, 0.15279719,
	0.14673047, 0.14065824, 0.1345807, 0.1284981, 0.12241068, 0.11631863, 0.110222206, 0.10412163,
	0.09801714, 0.091908954, 0.08579731, 0.07968244, 0.07356457, 0.06744392, 0.061320737, 0.055195246,
	0.049067676, 0.04293826, 0.036807224, 0.030674804, 0.024541229, 0.01840673, 0.012271538, 0.0061358847,
	0.0, -0.0061358847, -0.012271538, -0.01840673, -0.024541229, -0.030674804, -0.036807224, -0.04293826,
	-0.049067676, -0.055195246, -0.061320737, -0.06744392, -0.07356457, -0.07968244, -0.08579731, -0.091908954,
	-0.09801714, -0.10412163, -0.110222206, -0.11631863, -0.12241068, -0.1284981, -0.1345807, -0.14065824,
	-0.14673047, -0.15279719, -0.15885815, -0.16491312, -0.17096189, -0.17700422, -0.18303989, -0.18906866,
	-0.19509032, -0.20110464, -0.20711137, -0.21311031, -0.21910124, -0.22508392, -0.2310581, -0.2370236,
	-0.24298018, -0.24892761, -0.25486565, -0.2607941, -0.26671275, -0.27262136, -0.2785197, -0.28440753,
	-0.29028466, -0.2961509, -0.30200595, -0.30784965, -0.31368175, -0.31950203, -0.3253103, -0.3311063,
	-0.33688986, -0.34266073, -0.34841868, -0.35416353, -0.35989505, -0.36561298, -0.3713172, -0.37700742,
	-0.38268343, -0.38834503, -0.39399204, -0.3996242, -0.4052413, -0.41084316, -0.41642955, -0.42200026,
	-0.42755508, -0.43309382, -0.43861625, -0.44412214, -0.44961134, -0.45508358, -0.46053872, -0.4659765,
	-0.47139674, -0.47679922, -0.48218378, -0.48755017, -0.4928982, -0.49822766, -0.50353837, -0.50883013,
	-0.51410276, -0.519356, -0.52458966, -0.52980363, -0.53499764, -0.54017144, -0.545325, -0.55045795,
	-0.55557024, -0.56066155, -0.5657318, -0.57078075, -0.57580817, -0.58081394, -0.58579785, -0.5907597,
	-0.5956993, -0.60061646, -0.60551107, -0.6103828, -0.6152316, -0.6200572, -0.6248595, -0.62963825,
	-0.6343933, -0.63912445, -0.64383155, -0.6485144, -0.65317285, -0.6578067, -0.6624158, -0.66699994,
	-0.671559, -0.6760927, -0.680601, -0.6850837, -0.68954057, -0.69397146, -0.69837624, -0.70275474,
	-0.70710677, -0.7114322, -0.71573085, -0.72000253, -0.7242471, -0.72846437, -0.7326543, -0.7368166,
	-0.7409511, -0.74505776, -0.7491364, -0.7531868, -0.7572088, -0.7612024, -0.76516724, -0.76910335,
	-0.77301043, -0.7768885, -0.7807372, -0.78455657, -0.7883464, -0.79210657, -0.7958369, -0.79953724,
	-0.8032075, -0.8068476, -0.81045717, -0.8140363, -0.8175848, -0.8211025, -0.8245893, -0.82804507,
	-0.8314696, -0.8348629, -0.8382247, -0.841555, -0.8448536, -0.84812033, -0.8513552, -0.854558,
	-0.8577286, -0.86086696, -0.86397284, -0.86704624, -0.87008697, -0.873095, -0.8760701, -0.8790122,
	-0.8819213, -0.8847971, -0.88763964, -0.89044875, -0.8932243, -0.89596623, -0.8986745, -0.9013488,
	-0.9039893, -0.9065957, -0.909168, -0.91170603, -0.9142098, -0.9166791, -0.9191139, -0.92151403,
	-0.9238795, -0.9262102, -0.9285061, -0.93076694, -0.9329928, -0.9351835, -0.937339, -0.9394592,
	-0.94154406, -0.94359344, -0.9456073, -0.9475856, -0.94952816, -0.951435, -0.953306, -0.9551412,
	-0.95694035, -0.95870346, -0.9604305, -0.9621214, -0.96377605, -0.96539444, -0.96697646, -0.9685221,
	-0.97003126, -0.9715039, -0.97293997, -0.97433937, -0.9757021, -0.97702813, -0.9783174, -0.9795698,
	-0.98078525, -0.9819639, -0.9831055, -0.9842101, -0.98527765, -0.9863081, -0.9873014, -0.9882576,
	-0.9891765, -0.9900582, -0.99090266, -0.99170977, -0.99247956, -0.9932119, -0.993907, -0.9945646,
	-0.9951847, -0.9957674, -0.9963126, -0.9968203, -0.99729043, -0.99772304, -0.9981181, -0.99847555,
	-0.99879545, -0.99907774, -0.99932235, -0.9995294, -0.9996988, -0.9998306, -0.9999247, -0.99998116,
	-1.0,
];

// cos(2 pi hz / m_sr) from COS_TABLE, interpolated linearly.
// within 0.4 Hz from 150 Hz up to F_MAX_SR, about 1 Hz off at 50 Hz where the cosine is flattest
fn cos_hz(hz: f32) -> f32 {
	// at 44100 Hz the table's steps are twice as wide in Hz and F1 lands off: a desktop build works it out exactly
	if cfg!(feature = "rate-44100") { return libm::cosf(2.0 * PI * hz / m_sr as f32); }
	let n = COS_STEPS as f32;
	let mut t = (hz * (2.0 * n / m_sr as f32)).max(0.0);
	// above half the sample rate the cosine comes back down
	if t > n { t = (2.0 * n - t).max(0.0); }
	let i = (t as usize).min(COS_STEPS - 1);
	COS_TABLE[i] + (COS_TABLE[i + 1] - COS_TABLE[i]) * (t - i as f32)
}

// exp(-x) for x from 0 to EXP_TOP in EXP_STEPS steps, for the resonators' pole radius exp(-pi bw / m_sr)
const EXP_STEPS: usize = 256;
const EXP_TOP: f32 = 0.5;
const EXP_TABLE: [f32; EXP_STEPS + 1] = [
	1.0, 0.9980488, 0.9961014, 0.99415773, 0.99221796, 0.9902819, 0.9883497, 0.98642117,
	0.9844964, 0.9825755, 0.98065823, 0.97874475, 0.976835, 0.97492903, 0.9730267, 0.9711281,
	0.9692332, 0.967342, 0.9654546, 0.9635707, 0.9616906, 0.95981413, 0.95794135, 0.95607215,
	0.95420665, 0.9523448, 0.95048654, 0.94863194, 0.946781, 0.9449336, 0.94308984, 0.94124967,
	0.9394131, 0.93758005, 0.93575066, 0.9339248, 0.9321025, 0.9302838, 0.9284686, 0.9266569,
	0.9248488, 0.9230442, 0.9212432, 0.91944563, 0.9176516, 0.91586107, 0.914074, 0.91229045,
	0.91051036, 0.9087338, 0.9069606, 0.90519094, 0.90342474, 0.90166193, 0.8999026, 0.8981467,
	0.8963942, 0.89464515, 0.8928995, 0.89115727, 0.8894184, 0.887683, 0.8859509, 0.8842222,
	0.8824969, 0.880775, 0.8790564, 0.87734115, 0.87562925, 0.87392074, 0.8722155, 0.8705136,
	0.86881506, 0.8671198, 0.86542785, 0.86373925, 0.8620539, 0.8603718, 0.85869306, 0.8570176,
	0.8553453, 0.8536764, 0.85201067, 0.8503482, 0.84868896, 0.847033, 0.84538025, 0.84373075,
	0.8420844, 0.84044135, 0.83880144, 0.83716476, 0.8355313, 0.833901, 0.83227384, 0.8306499,
	0.82902914, 0.8274115, 0.825797, 0.8241857, 0.82257754, 0.82097256, 0.8193706, 0.81777185,
	0.81617624, 0.81458366, 0.81299424, 0.8114079, 0.8098247, 0.8082445, 0.80666745, 0.80509347,
	0.8035226, 0.80195475, 0.80038995, 0.7988282, 0.7972695, 0.79571384, 0.79416126, 0.79261166,
	0.7910651, 0.7895216, 0.78798103, 0.78644353, 0.784909, 0.78337747, 0.7818489, 0.7803234,
	0.7788008, 0.77728117, 0.7757645, 0.77425086, 0.7727401, 0.7712323, 0.76972747, 0.76822555,
	0.7667266, 0.76523054, 0.76373744, 0.7622472, 0.7607599, 0.7592755, 0.75779396, 0.75631535,
	0.7548396, 0.75336677, 0.75189674, 0.75042963, 0.7489654, 0.747504, 0.7460455, 0.74458975,
	0.7431369, 0.7416869, 0.7402397, 0.73879534, 0.73735374, 0.735915, 0.73447907, 0.73304594,
	0.7316156, 0.7301881, 0.72876334, 0.72734135, 0.72592217, 0.7245057, 0.723092, 0.7216811,
	0.72027296, 0.71886754, 0.7174649, 0.716065, 0.7146678, 0.7132733, 0.7118815, 0.7104925,
	0.7091062, 0.70772254, 0.7063416, 0.7049634, 0.7035879, 0.702215, 0.7008448, 0.6994773,
	0.6981125, 0.69675034, 0.6953908, 0.694034, 0.69267976, 0.69132817, 0.68997926, 0.68863297,
	0.6872893, 0.68594825, 0.6846098, 0.683274, 0.68194073, 0.6806101, 0.6792821, 0.6779567,
	0.67663383, 0.6753136, 0.6739959, 0.6726808, 0.67136824, 0.67005825, 0.6687508, 0.66744596,
	0.6661436, 0.6648438, 0.66354656, 0.66225183, 0.66095966, 0.65966994, 0.6583828, 0.6570982,
	0.655816, 0.65453637, 0.6532592, 0.6519846, 0.65071243, 0.64944273, 0.64817554, 0.6469108,
	0.64564854, 0.64438874, 0.6431314, 0.6418765, 0.64062405, 0.6393741, 0.6381265, 0.63688135,
	0.63563865, 0.6343984, 0.63316053, 0.6319251, 0.63069206, 0.62946147, 0.62823325, 0.6270074,
	0.625784, 0.624563, 0.6233443, 0.622128, 0.6209141, 0.6197026, 0.6184934, 0.61728656,
	0.61608213, 0.61488, 0.61368024, 0.61248285, 0.6112877, 0.61009496, 0.60890454, 0.60771644,
	0.60653067,
];

// pole radius for bandwidth bw from EXP_TABLE, interpolated linearly (within 0.01 Hz).
// past the table's end (3.5 kHz at 22050 Hz, only the fixed upper resonators) work it out exactly
fn pole_radius(bw: f32) -> f32 {
	let x = PI * bw.max(0.0) / m_sr as f32;
	if x >= EXP_TOP { return libm::expf(-x); }
	let t = x * (EXP_STEPS as f32 / EXP_TOP);
	let i = (t as usize).min(EXP_STEPS - 1);
	EXP_TABLE[i] + (EXP_TABLE[i + 1] - EXP_TABLE[i]) * (t - i as f32)
}

// klatt style resonator, in float: designs the coefficients, and checks the aspiration's level
fn res_coeffs(bf: f32, bw: f32) -> [f32; 3] {
	let r: f32 = pole_radius(bw);
	let c = -(r * r);
	let b = 2.0 * r * cos_hz(bf);
	[1.0 - b - c, b, c]
}
struct resonator {
	a:     f32,
	b:     f32,
	c:     f32,
	xp1:   f32,
	xp2:   f32,
}
impl resonator {
	fn new(bf: f32, bw: f32) -> resonator {
		let [a, b, c] = res_coeffs(bf, bw);
		resonator { a, b, c, xp1: 0.0, xp2: 0.0 }
	}
	fn step(&mut self, x: f32) -> f32 {
		let ot: f32 = self.a * x + self.b * self.xp1 + self.c * self.xp2;
		self.xp2 = self.xp1;
		self.xp1 = ot;
		ot
	}
}

// FIXED POINT
// the DSP runs on 32-bit integers, so a chip without a floating-point unit keeps up. every FRAME samples the
// inputs are read and turned into coefficients and gains (in float), and the DSP glides to them sample by sample
// 32 samples at 22050 Hz; it keeps that time (1.45 ms) at a higher rate
pub const FRAME: u32 = 32 * m_sr / 22050;
// coefficients and gains are Q28 (-8 to 8); the source, the parallel branch and the output are Q27 (-16 to 16);
// the cascade's signals are only a few thousandths, so they carry CASC_BITS fraction bits
const COEF_BITS: u32 = 28;
const SIG_BITS:  u32 = 27;
const CASC_BITS: u32 = 35;
const ONE: i32 = 1 << COEF_BITS;
const OUT_SCALE: f32 = 1.0 / (1u32 << SIG_BITS) as f32;

// x with bits fraction bits, saturating
fn to_q(x: f32, bits: u32) -> i32 { libm::roundf(x * (1u64 << bits) as f32) as i32 }
fn sat(x: i64) -> i32 { x.clamp(i32::MIN as i64, i32::MAX as i64) as i32 }
// a Q28 coefficient or gain times a signal
fn mul(c: i32, x: i32) -> i32 { sat((c as i64 * x as i64) >> COEF_BITS) }

// a coefficient or gain gliding to the value set at a frame, over the frame
#[derive(Default, Clone, Copy)]
struct Glide {
	now:    i32,
	step:   i32,
	target: i32,
}
impl Glide {
	fn set(&mut self, v: i32) {
		self.now = v;
		self.step = 0;
		self.target = v;
	}
	fn to(&mut self, target: i32) {
		// start from where the last glide was headed, so a value held (or 0) is held exactly
		self.now = self.target;
		self.target = target;
		self.step = ((target as i64 - self.now as i64) / FRAME as i64) as i32;
	}
	fn next(&mut self) -> i32 {
		self.now = self.now.saturating_add(self.step);
		self.now
	}
}

// a falling frequency would raise a ring's level: the scale that takes it back, a little each sample of the frame
fn shrink_per_sample(to: f32, from: f32) -> i32 {
	if to >= from || from <= 0.0 { return ONE; }
	// the FRAME-th root of to / from, to first order: a frame's step is small
	to_q(1.0 - (1.0 - to / from) / FRAME as f32, COEF_BITS)
}

// a resonator in fixed point, gliding to the coefficients set once a frame
struct fx_res {
	a:      Glide,
	b:      Glide,
	c:      Glide,
	y1:     i32,
	y2:     i32,
	y3:     i32,		// further back, for a stride of 2
	y4:     i32,
	stride: usize,		// samples between the outputs it feeds back: RES_STRIDE
	shrink: i32,		// per-sample scale of the ring while the frequency falls (Q28)
	bf:     f32,		// frequency it glides to, Hz
}
impl fx_res {
	fn new(bf: f32, bw: f32, stride: usize) -> fx_res {
		let s = stride as f32;
		let [a, b, c] = res_coeffs(bf * s, bw * s);
		let g = Glide::default();
		let mut r = fx_res { a: g, b: g, c: g, y1: 0, y2: 0, y3: 0, y4: 0, stride, shrink: ONE, bf };
		r.a.set(to_q(a, COEF_BITS));
		r.b.set(to_q(b, COEF_BITS));
		r.c.set(to_q(c, COEF_BITS));
		r
	}
	fn clear(&mut self) {
		(self.y1, self.y2, self.y3, self.y4) = (0, 0, 0, 0);
	}
	fn glide_to(&mut self, bf: f32, bw: f32) {
		self.shrink = shrink_per_sample(bf, self.bf);
		let s = self.stride as f32;
		let [a, b, c] = res_coeffs(bf * s, bw * s);
		self.a.to(to_q(a, COEF_BITS));
		self.b.to(to_q(b, COEF_BITS));
		self.c.to(to_q(c, COEF_BITS));
		self.bf = bf;
	}
	fn step(&mut self, x: i32) -> i32 {
		if self.shrink != ONE {
			self.y1 = mul(self.shrink, self.y1);
			self.y2 = mul(self.shrink, self.y2);
			if self.stride > 1 {
				self.y3 = mul(self.shrink, self.y3);
				self.y4 = mul(self.shrink, self.y4);
			}
		}
		let (a, b, c) = (self.a.next() as i64, self.b.next() as i64, self.c.next() as i64);
		let (back1, back2) = if self.stride > 1 { (self.y2, self.y4) } else { (self.y1, self.y2) };
		let y = sat((a * x as i64 + b * back1 as i64 + c * back2 as i64) >> COEF_BITS);
		if self.stride > 1 {
			self.y4 = self.y3;
			self.y3 = self.y2;
		}
		self.y2 = self.y1;
		self.y1 = y;
		y
	}
}

// a fixed upper formant in fixed point: set once for the voice
struct fixed_res {
	a:   i32,
	b:   i32,
	c:   i32,
	y1:  i32,
	y2:  i32,
	y3:  i32,		// further back, for a stride of 2
	y4:  i32,
	stride: usize,	// samples between the outputs it feeds back, as in fx_res
	bf:  f32,		// its frequency, Hz
}
impl fixed_res {
	fn new(bf: f32, bw: f32, stride: usize) -> fixed_res {
		let s = stride as f32;
		let [a, b, c] = res_coeffs(bf * s, bw * s).map(|v| to_q(v, COEF_BITS));
		fixed_res { a, b, c, y1: 0, y2: 0, y3: 0, y4: 0, stride, bf }
	}
	fn clear(&mut self) {
		(self.y1, self.y2, self.y3, self.y4) = (0, 0, 0, 0);
	}
	fn step(&mut self, x: i32) -> i32 {
		let (back1, back2) = if self.stride > 1 { (self.y2, self.y4) } else { (self.y1, self.y2) };
		let acc = self.a as i64 * x as i64 + self.b as i64 * back1 as i64 + self.c as i64 * back2 as i64;
		let y = sat(acc >> COEF_BITS);
		if self.stride > 1 {
			self.y4 = self.y3;
			self.y3 = self.y2;
		}
		self.y2 = self.y1;
		self.y1 = y;
		y
	}
}

// the nasal pair in fixed point: the zero and the pole as one filter. the zero alone (an inverted resonator)
// gains up to 60 dB on the way to the pole, too much range for fixed point
struct fx_nasal {
	n0:  Glide,		// the zero, times the pole's gain
	n1:  Glide,
	n2:  Glide,
	b:   Glide,		// the pole
	c:   Glide,
	x1:  i32,
	x2:  i32,
	x3:  i32,		// further back, for a stride of 2
	x4:  i32,
	y1:  i32,
	y2:  i32,
	y3:  i32,
	y4:  i32,
	stride: usize,	// samples between the inputs and outputs it reaches back to, as in fx_res
	shrink_z: i32,	// per-sample scales of the history while a frequency falls (Q28)
	shrink_p: i32,
	fz:  f32,		// the zero's frequency, Hz
	fp:  f32,		// the pole's
}
impl fx_nasal {
	fn new(f: f32, bw: f32, stride: usize) -> fx_nasal {
		let g = Glide::default();
		let mut n = fx_nasal {
			n0: g, n1: g, n2: g, b: g, c: g, x1: 0, x2: 0, x3: 0, x4: 0, y1: 0, y2: 0, y3: 0, y4: 0, stride,
			shrink_z: ONE, shrink_p: ONE, fz: f, fp: f,
		};
		n.set(f, bw, f, bw);
		n
	}
	// the coefficients for a zero at fz and a pole at fp, at this stride: n0, n1, n2, b, c
	fn coeffs(fz: f32, bz: f32, fp: f32, bp: f32, stride: usize) -> [i32; 5] {
		let s = stride as f32;
		let [za, zb, zc] = res_coeffs(fz * s, bz * s);
		let [pa, pb, pc] = res_coeffs(fp * s, bp * s);
		// the zero is a resonator inverted: (1 - zb z^-1 - zc z^-2) / za
		let k = pa / za;
		[k, -zb * k, -zc * k, pb, pc].map(|v| to_q(v, COEF_BITS))
	}
	fn set(&mut self, fz: f32, bz: f32, fp: f32, bp: f32) {
		let [n0, n1, n2, b, c] = Self::coeffs(fz, bz, fp, bp, self.stride);
		self.n0.set(n0);
		self.n1.set(n1);
		self.n2.set(n2);
		self.b.set(b);
		self.c.set(c);
	}
	fn glide_to(&mut self, fz: f32, bz: f32, fp: f32, bp: f32) {
		// as with a resonator, a falling frequency scales its history down
		self.shrink_z = shrink_per_sample(fz, self.fz);
		self.shrink_p = shrink_per_sample(fp, self.fp);
		let [n0, n1, n2, b, c] = Self::coeffs(fz, bz, fp, bp, self.stride);
		self.n0.to(n0);
		self.n1.to(n1);
		self.n2.to(n2);
		self.b.to(b);
		self.c.to(c);
		self.fz = fz;
		self.fp = fp;
	}
	fn step(&mut self, x: i32) -> i32 {
		let wide = self.stride > 1;
		if self.shrink_z != ONE {
			self.x1 = mul(self.shrink_z, self.x1);
			self.x2 = mul(self.shrink_z, self.x2);
			if wide {
				self.x3 = mul(self.shrink_z, self.x3);
				self.x4 = mul(self.shrink_z, self.x4);
			}
		}
		if self.shrink_p != ONE {
			self.y1 = mul(self.shrink_p, self.y1);
			self.y2 = mul(self.shrink_p, self.y2);
			if wide {
				self.y3 = mul(self.shrink_p, self.y3);
				self.y4 = mul(self.shrink_p, self.y4);
			}
		}
		let (xb1, xb2) = if wide { (self.x2, self.x4) } else { (self.x1, self.x2) };
		let (yb1, yb2) = if wide { (self.y2, self.y4) } else { (self.y1, self.y2) };
		let zeros = self.n0.next() as i64 * x as i64 + self.n1.next() as i64 * xb1 as i64
			+ self.n2.next() as i64 * xb2 as i64;
		let poles = self.b.next() as i64 * yb1 as i64 + self.c.next() as i64 * yb2 as i64;
		let y = sat((zeros + poles) >> COEF_BITS);
		if wide {
			self.x4 = self.x3;
			self.x3 = self.x2;
			self.y4 = self.y3;
			self.y3 = self.y2;
		}
		self.x2 = self.x1;
		self.x1 = x;
		self.y2 = self.y1;
		self.y1 = y;
		y
	}
}

// glottal model (refactoring of Qlatt's version of the Liljencrants-Fant glottal model using filters)
#[derive(Default)]
struct Biquad {
	b0: i32,
	b1: i32,
	b2: i32,
	a1: i32,
	a2: i32,
	x1: i32,
	x2: i32,
	x3: i32,		// further back, for a stride of 2
	x4: i32,
	y1: i32,
	y2: i32,
	y3: i32,
	y4: i32,
	stride: usize,	// samples between the values it reaches back to (0 or 1: the last two), as in fx_res
}
impl Biquad {
	// a low-pass section at hz with this Q
	fn lowpass(hz: f32, q: f32) -> Biquad {
		let w = 2.0 * PI * hz / m_sr as f32;
		let alpha = libm::sinf(w) / (2.0 * q);
		let cw = libm::cosf(w);
		let a0 = 1.0 + alpha;
		let mut f = Biquad::default();
		f.set((1.0 - cw) / 2.0 / a0, (1.0 - cw) / a0, (1.0 - cw) / 2.0 / a0, -2.0 * cw / a0, (1.0 - alpha) / a0);
		f
	}
	fn set(&mut self, b0: f32, b1: f32, b2: f32, a1: f32, a2: f32) {
		self.b0 = to_q(b0, COEF_BITS);
		self.b1 = to_q(b1, COEF_BITS);
		self.b2 = to_q(b2, COEF_BITS);
		self.a1 = to_q(a1, COEF_BITS);
		self.a2 = to_q(a2, COEF_BITS);
	}
	fn step(&mut self, x: i32) -> i32 {
		let wide = self.stride > 1;
		let (xb1, xb2) = if wide { (self.x2, self.x4) } else { (self.x1, self.x2) };
		let (yb1, yb2) = if wide { (self.y2, self.y4) } else { (self.y1, self.y2) };
		let acc = self.b0 as i64 * x as i64 + self.b1 as i64 * xb1 as i64 + self.b2 as i64 * xb2 as i64
			- self.a1 as i64 * yb1 as i64 - self.a2 as i64 * yb2 as i64;
		let y = sat(acc >> COEF_BITS);
		if wide {
			self.x4 = self.x3;
			self.x3 = self.x2;
			self.y4 = self.y3;
			self.y3 = self.y2;
		}
		self.x2 = self.x1;
		self.x1 = x;
		self.y2 = self.y1;
		self.y1 = y;
		y
	}
}

#[derive(Default)]
struct LpPole {
	a:  i32,
	y1: i32,
	y2: i32,			// for a stride of 2
	stride: usize,		// as in Biquad
}
impl LpPole {
	fn step(&mut self, x: i32) -> i32 {
		let back = if self.stride > 1 { self.y2 } else { self.y1 };
		let y = mul(ONE - self.a, x) + mul(self.a, back);
		self.y2 = self.y1;
		self.y1 = y;
		y
	}
}

// the first half of the cosine table in Q14, for the breath's flow shape
const FLOW_STEPS: usize = COS_STEPS / 2;
const FLOW_Q14: [i32; FLOW_STEPS + 1] = flow_table();
const fn flow_table() -> [i32; FLOW_STEPS + 1] {
	let mut t = [0; FLOW_STEPS + 1];
	let mut i = 0;
	while i <= FLOW_STEPS {
		t[i] = (COS_TABLE[i] * 16384.0 + 0.5) as i32;
		i += 1;
	}
	t
}

// phase step per sample for a pitch: 2^32 is a whole period
fn phase_step(hz: f32) -> i32 { (hz.max(0.0) / m_sr as f32 * 4294967296.0) as i32 }

// a glottal pulse spreads this many samples either side of its exact time, so the voice plays that late
const PULSE_HALF: usize = 4;
const PULSE_RING: usize = 16;

struct LfSource {
	phase:      u32,				// how far through the period: 2^32 is a whole period
	due:        bool,				// the period has run out: a pulse is due
	inc:        u32,				// phase step the last sample took
	exc:        [i32; PULSE_RING],	// excitation still to play, by sample index (Q27)
	n:          usize,				// sample index, modulo PULSE_RING
	glottal:    Biquad,
	tilt:       LpPole,
	flut_clock: u32,
	ra:         f32,
	oq:         f32,				// open quotient: share of the period the glottis is open
	open_end:   u32,				// phase where the glottis closes
	open_k:     u32,				// phase to flow-table steps: FLOW_STEPS * 2 over the open phase
	breath_k:   i32,				// breath gain for this oq (Q14) that keeps its average level
	jitter:     f32,				// average period change between pulses, in %
	shimmer:    f32,				// average level change between pulses, in dB
	rng:        noise,				// for jitter and shimmer
	jit_q:      u32,				// this period's rate factor (Q30)
}
impl LfSource {
	fn new() -> LfSource {
		let mut g = LfSource {
			phase: 0,
			due: true,
			inc: 0,
			exc: [0; PULSE_RING],
			n: 0,
			glottal: Biquad { stride: RES_STRIDE, ..Biquad::default() },
			tilt: LpPole { stride: RES_STRIDE, ..LpPole::default() },
			flut_clock: 0,
			ra: 0.0,
			oq: 0.5,
			open_end: 0,
			open_k: 0,
			breath_k: 0,
			jitter: 0.0,
			shimmer: 0.0,
			rng: noise { nxt: 0x6C8E9CF5 },
			jit_q: 1 << 30,
		};
		g.set_oq(0.5);
		g
	}
	fn rand(&mut self) -> f32 {
		// uniform in -1..1
		self.rng.xorshift();
		self.rng.nxt as f32 / u32::MAX as f32 * 2.0 - 1.0
	}
	fn add_pulse(&mut self, ago: f32, amp: f32) {
		// write a band-limited unit impulse at (n - ago), PULSE_HALF samples ahead of what plays now
		let h = PULSE_HALF as f32;
		let t = (self.n + PULSE_HALF) as f32 - ago;
		let first = libm::floorf(t) as usize + 1 - PULSE_HALF;
		let mut w = [0.0f32; 2 * PULSE_HALF];
		for (k, wk) in w.iter_mut().enumerate() {
			let x = (first + k) as f32 - t;
			let sinc = if x.abs() < 1e-6 { 1.0 } else { libm::sinf(PI * x) / (PI * x) };
			*wk = sinc * (0.5 + 0.5 * libm::cosf(PI * x / h));
		}
		let sum: f32 = w.iter().sum();
		for (k, wk) in w.iter().enumerate() {
			self.exc[(first + k) % PULSE_RING] += to_q(wk / sum * amp, SIG_BITS);
		}
	}
	fn calc_fant(&mut self, f0: f32, rd: f32) {
		// get Fant 1997 parameters
		let t0 = 1.0 / f0.max(1.0);
		// under Rd 0.21 the return phase Ra goes negative and the tilt filter blows up
		let rd = rd.max(RD_MIN);
		self.ra = (-1.0 + 4.8  * rd) / 100.0;
		let rk = (22.4 + 11.8 * rd) / 100.0;
		let rg_denom = 0.44 * rd - 4.0 * self.ra * (0.5 + 1.2 * rk);
		let rg = rk * (0.5 + 1.2 * rk) / rg_denom;

		// open quotient (pulse width of glottal basically)
		let oq = (1.0 + rk) / (2.0 * rg);
		self.set_oq(oq);

		// glottal frequency and bandwidth calculations
		let fg = 1.0 / (2.0 * oq * t0);
		let alpha_m = 1.0 / (1.0 + rk);
		let bg = 1.0 / (oq  * t0 * libm::tanf(PI * (1.0 - alpha_m)));

		// calculate coefficients for glottal biquad filter, at the rate it answers as (RES_SR)
		let a1 = -2.0 * libm::expf(-PI * bg / RES_SR) * libm::cosf(2.0 * PI * fg / RES_SR);
		let a2 = libm::expf(-2.0 * PI * bg / RES_SR);
		// at a stride of 2 a pulse fills every other sample, half the level: the gain makes it up
		let ag = RES_STRIDE as f32 / libm::sinf(PI * (1.0 - alpha_m));
		self.glottal.set(0.0, -ag, ag, a1, a2);

		// calculate coefficients for tilt filter
		let fa = 1.0 / (2.0 * PI * self.ra * t0);
		self.tilt.a = to_q(libm::expf(-2.0 * PI * fa / RES_SR), COEF_BITS);
	}
	fn set_oq(&mut self, oq: f32) {
		self.oq = oq.clamp(0.05, 1.0);
		self.open_end = (self.oq * 4294967296.0) as u32;
		self.open_k = (2.0 * FLOW_STEPS as f32 / self.oq) as u32;
		self.breath_k = to_q(BREATH_FLOW_GAIN / libm::sqrtf(self.oq), 14);
	}
	fn flutter_sines(c: u32) -> f32 {
		let w = 2.0 * PI * (c as f32 / m_sr as f32);
		libm::sinf(12.7 * w) + libm::sinf(7.1 * w) + libm::sinf(4.7 * w)
	}
	fn flutter(&mut self, f0: f32, flutter: f32) -> f32 {
		// KLGLOTT88 pitch flutter algorithm, once a frame; the pitch glides between frames
		if flutter <= 0.0 { return 0.0; }
		let a = Self::flutter_sines(self.flut_clock);
		// 10 s holds a whole number of cycles of all three, so the wrap is seamless
		self.flut_clock = (self.flut_clock + FRAME) % (10 * m_sr);
		a * (flutter / 50.0) * (f0 / 100.0)
	}
	fn restart(&mut self) {
		self.phase = 0;
		self.due = true;
	}
	fn openness(&self) -> i32 {
		// glottal flow shape (Q14): a half sine over the open phase, closed after it
		if self.due || self.phase >= self.open_end { return 0; }
		// sin(pi t) = cos(pi (t - 1/2)), from the table's first half
		let u = ((self.phase as u64 * self.open_k as u64) >> 32) as i32;
		FLOW_Q14[((u - FLOW_STEPS as i32).unsigned_abs() as usize).min(FLOW_STEPS)]
	}
	fn step(&mut self, base_inc: u32, f0: f32, rd: f32) -> (i32, bool) {
		// the first half of the period, for the frication's modulation
		let first_half = !self.due && self.phase < 1 << 31;

		// start a glottal pulse at the exact time the period ended, somewhere since the last sample
		if self.due {
			self.calc_fant(f0, rd);
			let ago = if self.inc > 0 { (self.phase as f32 / self.inc as f32).min(0.999) } else { 0.0 };
			self.due = false;
			// jitter stretches or shrinks this period, shimmer scales this pulse
			let jit = 1.0 / (1.0 + PERTURB_SPREAD * self.jitter / 100.0 * self.rand());
			self.jit_q = to_q(jit, 30) as u32;
			let amp = db_amp(PERTURB_SPREAD * self.shimmer * self.rand());
			self.add_pulse(ago, amp);
		}
		let slot = self.n % PULSE_RING;
		let impulse = self.exc[slot];
		self.exc[slot] = 0;
		self.n = (self.n + 1) % PULSE_RING;

		let sample = self.tilt.step(self.glottal.step(impulse));

		self.inc = ((base_inc as u64 * self.jit_q as u64) >> 30) as u32;
		let (phase, ended) = self.phase.overflowing_add(self.inc);
		self.phase = phase;
		self.due |= ended;
		(sample, first_half)
	}
}

// the parallel branch rests after this long without input: its narrowest ring has died away by 100 dB
const PARR_REST: u32 = m_sr / 10;

// white noise at 44100 Hz spreads over twice the band: sqrt 2 keeps its level per Hz (Q28)
const NOISE_44100_K: i32 = (core::f32::consts::SQRT_2 * ONE as f32) as i32;

// fixed-point constants: the voicing's share of the source, and the branches' makeup gains
const VOICING_K: i32 = (0.01 * ONE as f32) as i32;
const PARR_K: i32 = (PARR_GAIN * ONE as f32) as i32;
// the cascade's scale down to the output's, times CASC_GAIN, as a Q16 factor
const CASC_K: i64 = (CASC_GAIN as i64) << (16 - (CASC_BITS - SIG_BITS));
const CLIP_Q: i64 = (CLIP_LIMIT * (1u32 << SIG_BITS) as f32) as i64;

// vocal tract model - the main event
// AV, AH, AF, A2-A10 and AB are linear gains (1 is full scale, 87 dB), as _synth.rs keeps them; BR is in dB.
// they and the formants are read once a frame, and the fixed-point DSP glides to them
pub struct vtm {
	pub F0:   f32,
	pub AV:   f32,
	pub RD:   f32,
	pub AH:   f32,
	pub AF:   f32,
	pub BR:   f32,
	pub F1:   f32,
	pub B1:   f32,
	pub F2:   f32,
	pub B2:   f32,
	pub F3:   f32,
	pub B3:   f32,
	pub NC:   f32,		// nasal coupling, 0 (oral) to 1 (nasal consonant); set by _synth.rs
	pub A2:   f32,
	pub A3:   f32,
	pub A4:   f32,
	pub A5:   f32,
	pub A6:   f32,
	pub A7:   f32,
	pub A8:   f32,
	pub A9:   f32,
	pub A10:  f32,
	pub AB:   f32,
	B1_ADJ:   f32,
	B2_ADJ:   f32,
	B3_ADJ:   f32,
	noisegen: noise,
	glottal:  LfSource,
	NASAL:    fx_nasal,	// the nasal pair: NZR's zero and NPR's pole in one filter
	VR1:      fx_res,
	VR2:      fx_res,
	VR3:      fx_res,
	VR4:      fixed_res,
	VR5:      fixed_res,
	VR6:      fixed_res,
	VR7:      fixed_res,
	VR8:      fixed_res,
	out_lp:   [Biquad; 3],	// keeps the output under 11 kHz at 44100 Hz, as at 22050 Hz
	noise_lp: [Biquad; 3],	// and the noise, before the resonators
	FR2:      fx_res,
	FR3:      fx_res,
	FR4:      fixed_res,
	FR5:      fixed_res,
	FR6:      fixed_res,
	FR7:      fixed_res,
	FR8:      fixed_res,
	FR9:      fixed_res,
	FR10:     fixed_res,
	av:       Glide,		// the gains as the DSP plays them (Q28)
	ah:       Glide,		// aspiration, with its level match
	af:       Glide,
	amps:     [Glide; 9],	// A2 to A10
	ab:       Glide,
	br:       Glide,		// breath noise
	inc:      Glide,		// pitch, as the phase step per sample
	pulse_f0: f32,		// the frame's pitch and Rd, for shaping the pulses
	pulse_rd: f32,
	frame_left: u32,	// samples until the next frame
	xp1:      i32,		// lip radiation's past sample
	fric_y1:  i32,		// frication noise lowpass state
	voice:    Voice,	// voice profile
	asp_gain: f32,		// aspiration gain, matches the table voice's loudness
	vgain:    f32,		// the voice profile's output level, linear
	// cascade B1/B2 floor, wider for a higher voice (its rings have less time to die)
	min_bw:   f32,
	pub f1_over_f0: f32,	// F1 at least the pitch times this, as a singer opens up on a high note (0: off)
	fric_a:   i32,		// frication lowpass coefficient (Q28)
	lip_d:    i32,		// lip radiation coefficient at this rate (Q28)
	lip_k:    i32,		// lip radiation's makeup gain (Q28)
	out_k:    i32,		// output trim times the voice's level (Q28)
	parr_quiet: u32,	// samples since the parallel branch last had input
	voiced:   bool,		// the voicing gain was above 0 last sample
}
impl vtm {
	pub fn new() -> vtm {
		let fric_lp_hz = if cfg!(feature = "rate-44100") { FRIC_LP_HZ_44100 } else { FRIC_LP_HZ };
		let lip_d = if cfg!(feature = "rate-44100") {
			LIP_D_44100
		} else {
			libm::powf(LIP_D, libm::powf(LIP_TUNED_SR / m_sr as f32, LIP_MATCH))
		};
		let g = Glide::default();
		let mut tract = vtm {
			F0     : 125.0,											// fundamental frequency (Hz)
			AV     : DB_GAIN[60],									// Amplitude of voicing (60 dB)
			RD     : 0.0,											// RD fant tilt param (for glottal pulse model)
			AH     : 0.0,
			AF     : 0.0,
			BR     : 6.0,											// breathiness amp (dB)
			F1     : F1_INIT,										// Frequency of 1st formant, in Hz
			B1     : B1_INIT,										// Bandwidth of 1st formant, in Hz
			F2     : F2_INIT,										// Frequency of 2nd formant, in Hz
			B2     : B2_INIT,										// Bandwidth of 2nd formant, in Hz
			F3     : F3_INIT,										// Frequency of 3rd formant, in Hz
			B3     : B3_INIT,										// Bandwidth of 3rd formant, in Hz
			NC     : 0.0,											// nasal coupling
			A2     : 0.0,
			A3     : 0.0,
			A4     : 0.0,
			A5     : 0.0,
			A6     : 0.0,
			A7     : 0.0,
			A8     : 0.0,
			A9     : 0.0,
			A10    : 0.0,
			AB     : 0.0,
			B1_ADJ : 1.0,
			B2_ADJ : 1.0,
			B3_ADJ : 1.0,
			noisegen : noise::new(),								// noise generator
			glottal  : LfSource::new(),								// glottal pulse generator
			NASAL    : fx_nasal::new(FN_INIT, BN_INIT, RES_STRIDE),		// nasal pair
			VR1      : fx_res::new(F1_INIT,  B1_INIT, RES_STRIDE),				// cascade resonators
			VR2      : fx_res::new(F2_INIT,  B2_INIT, RES_STRIDE),				//    |
			VR3      : fx_res::new(F3_INIT,  B3_INIT, RES_STRIDE),				//    |
			VR4      : fixed_res::new(F4_INIT,  B4_INIT, RES_STRIDE),				//    |
			VR5      : fixed_res::new(F5_INIT,  B5_INIT, RES_STRIDE),				//    |
			VR6      : fixed_res::new(F6_INIT,  B6_INIT, RES_STRIDE),				//    |
			VR7      : fixed_res::new(F7_INIT,  B7_INIT, RES_STRIDE),				//    |
			VR8      : fixed_res::new(F8_INIT,  B8_INIT, RES_STRIDE),				//    V
			out_lp  : OUT_LP_Q.map(|q| Biquad::lowpass(OUT_LP_HZ, q)),
			noise_lp: OUT_LP_Q.map(|q| Biquad::lowpass(OUT_LP_HZ, q)),
			FR2      : fx_res::new(F2_INIT,  B2_INIT, RES_STRIDE),				// parallel resonators
			FR3      : fx_res::new(F3_INIT,  B3_INIT, RES_STRIDE),				//    |
			FR4      : fixed_res::new(F4_INIT,  B4_INIT, RES_STRIDE),				//    |
			FR5      : fixed_res::new(F5_INIT,  B5_INIT, RES_STRIDE),				//    |
			FR6      : fixed_res::new(F6_INIT,  B6_INIT, RES_STRIDE),				//    |
			FR7      : fixed_res::new(F7_INIT,  B7_INIT, RES_STRIDE),				//    |
			FR8      : fixed_res::new(F8_INIT,  B8_INIT, RES_STRIDE),				//    |
			FR9      : fixed_res::new(F9_INIT,  B9_INIT, RES_STRIDE),				//    |
			FR10     : fixed_res::new(F10_INIT, B10_INIT, RES_STRIDE),				//    V
			av: g,
			ah: g,
			af: g,
			amps: [g; 9],
			ab: g,
			br: g,
			inc: g,
			pulse_f0: 125.0,
			pulse_rd: 0.0,
			frame_left: 0,

			// past sample for lip radiation filter
			xp1: 0,

			fric_y1: 0,
			voice: VOICES[0],
			asp_gain: 1.0,
			vgain: 1.0,
			min_bw: CASC_MIN_BW,
			f1_over_f0: 0.0,

			fric_a:  to_q(libm::expf(-2.0 * PI * fric_lp_hz / m_sr as f32), COEF_BITS),
			lip_d:   to_q(lip_d, COEF_BITS),
			lip_k:   to_q((1.0 - LIP_D) / (1.0 - lip_d), COEF_BITS),
			out_k:   0,
			parr_quiet: 0,
			voiced:  false,
		};
		// set voice 0 fully: jitter and shimmer live in the glottal source, which only set_voice reaches
		tract.set_voice(VOICES[0]);
		tract
	}
	pub fn voice(&self) -> Voice { self.voice }
	pub fn set_voice(&mut self, v: Voice) {
		// rebuild the fixed upper formants at the voice's scale (the lower ones follow the inputs every frame)
		self.voice = v;
		self.vgain = db_amp(v.db);
		self.out_k = to_q(db_amp(OUT_DB) * self.vgain, COEF_BITS);
		self.asp_gain = asp_level(&VOICES[0]) / asp_level(&v);
		self.min_bw = CASC_MIN_BW * libm::sqrtf((v.f0 / 100.0).max(1.0));
		self.glottal.jitter = v.jit;
		self.glottal.shimmer = v.shim;
		let upper = |hz: f32, bw: f32| {
			let (f, b) = upper_formant(&v, hz, bw);
			fixed_res::new(f, b, RES_STRIDE)
		};
		self.VR4  = upper(F4_INIT,  B4_INIT);
		self.VR5  = upper(F5_INIT,  B5_INIT);
		self.VR6  = upper(F6_INIT,  B6_INIT);
		self.VR7  = upper(F7_INIT,  B7_INIT);
		self.VR8  = upper(F8_INIT,  B8_INIT);
		self.FR4  = upper(F4_INIT,  B4_INIT);
		self.FR5  = upper(F5_INIT,  B5_INIT);
		self.FR6  = upper(F6_INIT,  B6_INIT);
		self.FR7  = upper(F7_INIT,  B7_INIT);
		self.FR8  = upper(F8_INIT,  B8_INIT);
		self.FR9  = upper(F9_INIT,  B9_INIT);
		self.FR10 = upper(F10_INIT, B10_INIT);
	}
	fn apply_voice(&mut self) {
		// scale this frame's pitch, formants and source by the voice profile
		// like fix_bw, this relies on _synth setting them again before every frame
		let v = self.voice;
		let top = F_MAX_SR * m_sr as f32;
		if v.range != 100.0 { self.F0 = REF_HZ * libm::powf(self.F0 / REF_HZ, v.range / 100.0); }

		self.F0 *= v.f0 / 100.0;
		self.F1 = (self.F1 * (v.f1 / 100.0)).min(top);
		self.F2 = (self.F2 * (v.f2 / 100.0)).min(top);
		self.F3 = (self.F3 * (v.f3 / 100.0)).min(top);
		// bandwidths follow the formants, so a smaller tract keeps the same sharpness (Q)
		self.B1 *= v.f1 / 100.0;
		self.B2 *= v.f2 / 100.0;
		self.B3 *= v.f3 / 100.0;
		self.RD *= v.rd / 100.0;
		self.BR += v.br;
		self.tune_f1();
	}
	fn tune_f1(&mut self) {
		// a pitch over F1 has no harmonic for F1 to lift, so the note comes out thin and its level jumps
		// from note to note: raise F1 over the pitch, keeping its sharpness (Q)
		let tuned = (self.F0 * self.f1_over_f0).min(self.F2 - F1_TUNE_BELOW_F2);
		if tuned > self.F1 {
			self.AV *= libm::powf(self.F1.max(1.0) / tuned, F1_TUNE_GAIN_POW);
			self.B1 *= tuned / self.F1.max(1.0);
			self.F1 = tuned;
		}
	}
	fn db2lin(gain: f32) -> f32 {
		if gain <= 0.0 { return 0.0; }
		if gain >= G_MAX { return 1.0; }
		// interpolate between whole dB: within 0.01 dB of the curve
		let i = gain as usize;
		DB_GAIN[i] + (DB_GAIN[i + 1] - DB_GAIN[i]) * (gain - i as f32)
	}
	fn fix_bw(&mut self) {
		// adjust bandwidths to improve sound quality

		// F0-proximity bandwidth adjust
		let near = (self.F0 / self.F1 - 0.5).max(0.0);
		self.B1 += 200.0 * near * near;

		// glottal leakage adjust (B1, B2): an open glottis only adds loss, so it never narrows a bandwidth
		let f1 = self.F1 / 500.0;
		let DB1 = (250.0 * f1 * f1 * (self.glottal.ra * 100.0 - RA_REF) / 12.0).max(0.0);
		let DB2 = DB1 * self.F1 / (2.0 * self.F2);
		self.B1 += DB1;
		self.B2 += DB2;

		// widen formants that are close together. F3 also widens for a close F2,
		// or the F2-F3 pair rings as one narrow peak (front vowels in voice 6)
		[self.B1_ADJ, self.B2_ADJ, self.B3_ADJ] = bw_widen3(self.F1, self.F2, self.F3, self.VR4.bf);
	}
	fn nasal_zero(&self, scale: f32, k: f32) -> f32 {
		// place the nasal zero for coupling k (0 oral, 1 nasal consonant):
		// oral sits on the nasal pole (they cancel), nasal sits on F1 (the pole is then the murmur),
		// a vowel's nasalized edge is in between
		FN_INIT + k * ((self.F1 / scale) - FN_INIT)
	}

	// FRAME: read the inputs and set what the DSP glides to
	fn frame(&mut self) {
		self.apply_voice();
		self.fix_bw();
		let flutter = self.glottal.flutter(self.F0, self.voice.fl);
		self.inc.to(phase_step(self.F0 + flutter));
		self.pulse_f0 = self.F0;
		self.pulse_rd = self.RD;
		self.set_cascade();
		self.set_parallel();
		self.set_gains();
	}
	fn set_cascade(&mut self) {
		let bb1 = (self.B1 * self.B1_ADJ).max(self.min_bw);
		let bb2 = (self.B2 * self.B2_ADJ).max(self.min_bw);
		let bb3 = (self.B3 * self.B3_ADJ).max(MIN_BW);

		// nasal coupling (Hawkins & Stevens 1985)
		let k = self.NC.clamp(0.0, 1.0);
		let scale = self.voice.fhi / 100.0;
		let fnz = self.nasal_zero(scale, k) * scale;
		let bnz = BN_INIT + k * (bb1 - BN_INIT);
		let bnp = BN_INIT + k * (BN_MURMUR - BN_INIT);
		let fnp = FN_INIT + k * (FN_MURMUR - FN_INIT);
		self.NASAL.glide_to(fnz, bnz, fnp * scale, bnp);

		// the formants keep the cap of the rate they answer as
		let top = F_MAX_SR * RES_SR;
		self.VR1.glide_to(self.F1.min(top), bb1);			// F1 - tongue height
		self.VR2.glide_to(self.F2.min(top), bb2);			// F2 - tongue fronting
		self.VR3.glide_to(self.F3.min(top), bb3);			// F3 - R-coloring
	}
	fn set_parallel(&mut self) {
		let top = F_MAX_SR * RES_SR;
		self.FR2.glide_to(self.F2.min(top), (self.B2 * self.B2_ADJ).max(MIN_BW));
		self.FR3.glide_to(self.F3.min(top), (self.B3 * self.B3_ADJ).max(MIN_BW));
	}
	fn set_gains(&mut self) {
		let gain = |g: f32| to_q(g.clamp(0.0, 1.0), COEF_BITS);
		self.av.to(gain(self.AV));
		self.ah.to(to_q(0.54 * self.asp_gain * self.AH.clamp(0.0, 1.0), COEF_BITS));
		self.af.to(gain(self.AF));
		let amps = [self.A2, self.A3, self.A4, self.A5, self.A6, self.A7, self.A8, self.A9, self.A10];
		for (glide, a) in self.amps.iter_mut().zip(amps) { glide.to(gain(a)); }
		self.ab.to(gain(self.AB));
		self.br.to(to_q(Self::db2lin(self.BR), COEF_BITS));
	}

	// SAMPLE: the fixed-point DSP
	fn parallel(&mut self, x: i32) -> i32 {
		// with no input for PARR_REST samples its rings are inaudible: clear them and rest until input comes
		if x != 0 {
			self.parr_quiet = 0;
		} else if self.parr_quiet < PARR_REST {
			self.parr_quiet += 1;
			if self.parr_quiet == PARR_REST { self.clear_parallel(); }
		}
		if self.parr_quiet == PARR_REST {
			for a in self.amps.iter_mut() { a.next(); }
			self.ab.next();
			return 0;
		}
		// odd formants added, even ones taken away, so neighbouring peaks don't cancel
		let a = &mut self.amps;
		let mut yy: i64 = 0;
		yy -= mul(a[0].next(), self.FR2.step(x)) as i64;
		yy += mul(a[1].next(), self.FR3.step(x)) as i64;
		yy -= mul(a[2].next(), self.FR4.step(x)) as i64;
		yy += mul(a[3].next(), self.FR5.step(x)) as i64;
		yy -= mul(a[4].next(), self.FR6.step(x)) as i64;
		yy += mul(a[5].next(), self.FR7.step(x)) as i64;
		yy -= mul(a[6].next(), self.FR8.step(x)) as i64;
		yy += mul(a[7].next(), self.FR9.step(x)) as i64;
		yy -= mul(a[8].next(), self.FR10.step(x)) as i64;
		yy += mul(self.ab.next(), x) as i64;
		sat(yy)
	}
	fn clear_parallel(&mut self) {
		self.FR2.clear();
		self.FR3.clear();
		for r in [&mut self.FR4, &mut self.FR5, &mut self.FR6, &mut self.FR7, &mut self.FR8, &mut self.FR9,
			&mut self.FR10] {
			r.clear();
		}
	}
	fn cascade(&mut self, x: i32) -> i32 {
		let mut yy = self.NASAL.step(x);			// NASAL TRACT
		yy = self.VR1.step(yy);
		yy = self.VR2.step(yy);
		yy = self.VR3.step(yy);
		yy = self.VR4.step(yy);
		yy = self.VR5.step(yy);
		yy = self.VR6.step(yy);
		yy = self.VR7.step(yy);
		yy = self.VR8.step(yy);
		yy
	}
	fn softclip(x: i32) -> i32 {
		let a = (x as i64).abs();
		if a <= CLIP_Q { return x; }
		let one = 1i64 << SIG_BITS;
		let excess = a - CLIP_Q;
		let y = CLIP_Q + (one - CLIP_Q) * excess / (excess + one);
		if x < 0 { -y as i32 } else { y as i32 }
	}
	fn radiate(&mut self, x: i32) -> i32 {
		// lip radiation, coefficient pulled toward the tuned rate's response, same low-frequency gain
		let d = self.lip_d;
		mul(self.lip_k, self.lip(x, d))
	}
	fn lip(&mut self, x: i32, d: i32) -> i32 {
		let xx = x.saturating_sub(mul(d, self.xp1));
		self.xp1 = x;
		xx
	}
	// the next step reads the inputs
	pub fn wants_frame(&self) -> bool { self.frame_left == 0 }
	pub fn step(&mut self) -> f32 {
		if self.frame_left == 0 {
			self.frame();
			self.frame_left = FRAME;
		}
		self.frame_left -= 1;
		let av = self.av.next();
		let white = self.noisegen.step();
		let mut white = if cfg!(feature = "rate-44100") { mul(NOISE_44100_K, white) } else { white };
		if RES_STRIDE > 1 {
			for f in self.noise_lp.iter_mut() { white = f.step(white); }
		}

		// voicing starts with a pulse, not wherever the free-running period happens to be
		if av > 0 && !self.voiced { self.glottal.restart(); }
		self.voiced = av > 0;
		let open = self.glottal.openness();
		let (pulse, first_half) = self.glottal.step(self.inc.next() as u32, self.pulse_f0, self.pulse_rd);

		// breath follows the flow while voicing, at the same average power as unmodulated noise
		let br = self.br.next();
		let br = if av > 0 { mul(br, open * self.glottal.breath_k) } else { br };
		let voicing = mul(VOICING_K, pulse) + mul(br, white);

		// cascade input: voicing with breath, and aspiration, up to the cascade's scale
		let ah = self.ah.next();
		let to_casc = COEF_BITS - (CASC_BITS - SIG_BITS);
		let casc_in = sat((voicing as i64 * av as i64 + white as i64 * ah as i64) >> to_casc);

		// parallel input: frication, broadband noise rolled off above FRIC_LP_HZ, at half level in the second
		// half of each voiced period
		self.fric_y1 = mul(ONE - self.fric_a, white) + mul(self.fric_a, self.fric_y1);
		let mut parr_in = mul(self.af.next(), self.fric_y1);
		if av > 0 && !first_half { parr_in /= 2; }

		let casc_out = sat((self.cascade(casc_in) as i64 * CASC_K) >> 16);
		let parr_out = mul(PARR_K, self.parallel(parr_in));

		let mut out = mul(self.out_k, self.radiate(casc_out.saturating_add(parr_out)));
		if RES_STRIDE > 1 {
			for f in self.out_lp.iter_mut() { out = f.step(out); }
		}
		Self::softclip(out) as f32 * OUT_SCALE
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn nasal_zero_moves_smoothly() {
		// the zero sits on the nasal pole for oral sounds, on F1 for a nasal, and moves without jumps in between
		let mut v = vtm::new();
		let mut zero = |f1: f32, k: f32| { v.F1 = f1; v.nasal_zero(1.0, k) };
		assert_eq!(zero(700.0, 0.0), FN_INIT);
		assert_eq!(zero(250.0, 1.0), 250.0);
		assert_eq!(zero(400.0, 1.0), 400.0);
		let mut last = zero(500.0, 0.0);
		for i in 1..=100 {
			let z = zero(500.0, i as f32 / 100.0);
			assert!((z - last).abs() < 10.0, "zero jumps {} -> {}", last, z);
			last = z;
		}
	}

	// render 4000 samples of a steady vowel with these F1, F2, B1, B2 in voice
	fn steady(voice: Voice, f1: f32, f2: f32, b1: f32, b2: f32) -> Vec<f32> {
		let mut v = vtm::new();
		v.set_voice(voice);
		(0..4000).map(|_| {
			v.F0 = 120.0; v.AV = DB_GAIN[60]; v.RD = 0.6;
			v.F1 = f1; v.F2 = f2; v.F3 = 2500.0;
			v.B1 = b1; v.B2 = b2; v.B3 = 150.0;
			v.step()
		}).collect()
	}

	#[test]
	fn aspiration_is_as_loud_in_every_voice() {
		// the breath after a final stop: white noise through wide formants. a bigger voice scale lets more through
		let level = |voice: Voice| {
			let mut v = vtm::new();
			v.set_voice(voice);
			let row = |k: usize| formant_tbl[IXP as usize * ROW_LEN + k] as f32;
			let x: Vec<f32> = (0..12000).map(|_| {
				v.F0 = 120.0;
				v.AV = 0.0;
				v.AH = DB_GAIN[50];
				v.RD = 0.6;
				(v.F1, v.F2, v.F3) = (row(5), row(6), row(7));
				(v.B1, v.B2, v.B3) = (row(8), row(9), row(10));
				v.step()
			}).collect();
			10.0 * libm::log10f(x[2000..].iter().map(|s| s * s).sum::<f32>() / 10000.0)
		};
		// within a couple of dB (the matching uses fixed reference formants)
		let table = level(VOICES[0]);
		for (i, &voice) in VOICES.iter().enumerate() {
			assert!((level(voice) - table).abs() < 2.5, "voice {}: {:.1} dB against {:.1}", i, level(voice), table);
		}
	}

	#[test]
	fn close_f2_and_f3_both_widen() {
		// voice 6's ix: F2 and F3 194 Hz apart. widening only B2 leaves them ringing as one narrow peak
		let mut v = vtm::new();
		v.F1 = 280.0; v.F2 = 2080.0; v.F3 = 2274.0;
		v.fix_bw();
		assert!(v.B2_ADJ > 2.0, "B2 x{}", v.B2_ADJ);
		assert_eq!(v.B3_ADJ, v.B2_ADJ);
	}

	#[test]
	fn cascade_bandwidths_have_a_floor() {
		// a B1 or B2 below the floor renders the same as the floor (both of these end up under it)
		let render = |b1: f32, b2: f32| steady(VOICES[0], 600.0, 1500.0, b1, b2);
		let floor = render(10.0, 10.0);
		assert_eq!(render(20.0, 20.0), floor);
		assert_ne!(render(CASC_MIN_BW + 200.0, 10.0), floor);
	}

	#[test]
	fn high_voices_have_a_higher_bandwidth_floor() {
		// a high voice's rings have less time to die between pulses, so it floors a B1 the table voice keeps
		let hi = VOICES[3];
		assert_eq!(steady(hi, 600.0, 1500.0, 65.0, 10.0), steady(hi, 600.0, 1500.0, 10.0, 10.0));
		assert_ne!(steady(VOICES[0], 600.0, 1500.0, 65.0, 10.0), steady(VOICES[0], 600.0, 1500.0, 10.0, 10.0));
	}

	#[test]
	fn a_tuned_f1_rises_over_a_high_pitch_only() {
		// (F1, B1, AV) the tract gets for an ee-like vowel at pitch f0, tuned by k
		let f1 = |f0: f32, k: f32| {
			let mut v = vtm::new();
			v.f1_over_f0 = k;
			v.F0 = f0; v.AV = DB_GAIN[60]; v.RD = 0.6; v.F1 = 300.0; v.F2 = 2300.0; v.B1 = 60.0; v.B2 = 100.0;
			v.step();
			(v.F1, v.B1, v.AV)
		};
		assert_eq!(f1(800.0, 0.0).0, 300.0);
		assert_eq!(f1(150.0, 1.1).0, 300.0);
		let (tuned, b1, av) = f1(800.0, 1.1);
		assert!((tuned - 880.0).abs() < 0.1, "F1 {}", tuned);
		assert!(b1 > 60.0 * 880.0 / 300.0 - 1.0, "B1 {} for F1 {}", b1, tuned);
		// and gives back some of the level the raise adds
		let untuned_av = f1(800.0, 0.0).2;
		assert!((av / untuned_av - libm::powf(300.0 / 880.0, F1_TUNE_GAIN_POW)).abs() < 0.001, "AV {}", av);
		// never up into F2
		assert!(f1(2500.0, 1.1).0 <= 2300.0 - F1_TUNE_BELOW_F2);
	}

	#[test]
	fn pitch_far_below_f1_leaves_b1_alone() {
		// B1 widens as F0 comes up toward F1, not as it moves away from it
		let b1 = |f0: f32| {
			let mut v = vtm::new();
			v.F0 = f0; v.AV = DB_GAIN[60]; v.RD = 0.6; v.F1 = 750.0; v.F2 = 1250.0; v.B1 = 100.0; v.B2 = 88.0;
			v.step();
			v.B1
		};
		assert_eq!(b1(100.0), 100.0);
		assert!(b1(500.0) > 100.0);
	}

	#[test]
	fn glottal_leakage_never_narrows() {
		// whatever the source, the bandwidths a vowel asks for only get wider
		for &rd in &[0.3f32, 0.6, 1.0, 2.0] {
			let mut v = vtm::new();
			for _ in 0..50 {
				v.F0 = 110.0;
				v.AV = DB_GAIN[60];
				v.RD = rd;
				v.F1 = 750.0;
				v.F2 = 1250.0;
				v.B1 = 100.0;
				v.B2 = 88.0;
				v.step();
			}
			assert!(v.B1 >= 100.0 && v.B2 >= 88.0, "Rd {}: B1 {} B2 {}", rd, v.B1, v.B2);
		}
	}

	// noise power per Hz between lo and hi Hz, in dB (Hann-windowed DFT, so any rate and length compare)
	fn density_db(x: &[f32], lo: f32, hi: f32) -> f32 {
		let n = x.len();
		let w: Vec<f64> = (0..n).map(|i| 0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / n as f64).cos()).collect();
		let w2: f64 = w.iter().map(|v| v * v).sum();
		let (k0, k1) = ((lo * n as f32 / m_sr as f32) as usize, (hi * n as f32 / m_sr as f32) as usize);
		let mut sum = 0.0f64;
		for k in k0..k1 {
			let (mut re, mut im) = (0.0f64, 0.0f64);
			for (i, &v) in x.iter().enumerate() {
				let ph = 2.0 * std::f64::consts::PI * (k * i) as f64 / n as f64;
				re += w[i] * v as f64 * ph.cos();
				im -= w[i] * v as f64 * ph.sin();
			}
			sum += re * re + im * im;
		}
		10.0 * (sum / (k1 - k0) as f64 / (m_sr as f64 * w2)).log10() as f32
	}

	#[test]
	fn vowel_spectrum_keeps_to_the_rate() {
		// a held ah has the same level per 1 kHz band up to 6 kHz at either rate. the levels are the 22050 Hz
		// build's, measured over 0.74 s
		let mut v = vtm::new();
		let span = 16384 * (m_sr / 22050) as usize;
		let x: Vec<f32> = (0..m_sr as usize / 2 + span).map(|_| {
			v.F0 = 120.0; v.AV = DB_GAIN[60]; v.RD = 0.6;
			v.F1 = 700.0; v.F2 = 1220.0; v.F3 = 2600.0; v.B1 = 90.0; v.B2 = 110.0; v.B3 = 160.0;
			v.step()
		}).collect();
		let tail = &x[x.len() - span..];
		let want = [-50.71f32, -59.72, -79.12, -85.29, -89.62, -103.08];
		let got: Vec<f32> = (0..6)
			.map(|k| density_db(tail, (k as f32 * 1000.0).max(50.0), (k + 1) as f32 * 1000.0))
			.collect();
		for k in 0..6 {
			assert!((got[k] - want[k]).abs() < 1.0, "{}-{} kHz: {:?} dB per Hz, not {:?}", k, k + 1, got, want);
		}
	}

	#[test]
	fn noise_level_per_hz_keeps_to_the_rate() {
		// frication straight through the parallel bypass is as loud per Hz at either rate. the levels are the 22050 Hz
		// build's; over 0.74 s the estimate holds to about 0.1 dB
		let mut v = vtm::new();
		let span = 16384 * (m_sr / 22050) as usize;
		let x: Vec<f32> = (0..m_sr as usize / 4 + span).map(|_| {
			v.AV = 0.0; v.AH = 0.0; v.AF = DB_GAIN[60]; v.AB = DB_GAIN[60];
			v.step()
		}).collect();
		let tail = &x[x.len() - span..];
		for (lo, hi, want) in [(2000.0, 4000.0, -100.30), (6000.0, 8000.0, -98.73)] {
			let got = density_db(tail, lo, hi);
			assert!((got - want).abs() < 0.5, "{}-{} Hz: {} dB per Hz, not {}", lo, hi, got, want);
		}
	}

	#[test]
	#[cfg(feature = "rate-44100")]
	fn hiss_stays_under_the_reference_band() {
		// at 44100 Hz frication adds no hiss the 22050 Hz voice never had: 12-16 kHz over 20 dB under 2-4 kHz
		let mut v = vtm::new();
		let span = 16384 * (m_sr / 22050) as usize;
		let x: Vec<f32> = (0..m_sr as usize / 4 + span).map(|_| {
			v.AV = 0.0; v.AH = 0.0; v.AF = DB_GAIN[60]; v.AB = DB_GAIN[60];
			v.step()
		}).collect();
		let tail = &x[x.len() - span..];
		let gap = density_db(tail, 12000.0, 16000.0) - density_db(tail, 2000.0, 4000.0);
		assert!(gap < -20.0, "12-16 kHz only {} dB under 2-4 kHz", gap);
	}

	#[test]
	fn lip_radiation_matches_the_reference_rate() {
		// radiate()'s tilt, relative to 100 Hz, follows the 22050 Hz filter's at either rate (one coefficient fits it
		// within 0.75 dB up to 10 kHz)
		let gain = |hz: f32| {
			let mut v = vtm::new();
			let mut peak = 0.0f32;
			for i in 0..4 * m_sr as usize {
				let x = (2.0 * PI * hz * i as f32 / m_sr as f32).sin();
				let y = v.radiate(to_q(x, SIG_BITS)) as f32 * OUT_SCALE;
				if i > 2 * m_sr as usize { peak = peak.max(y.abs()); }
			}
			20.0 * peak.log10()
		};
		let d = libm::powf(LIP_D, libm::powf(LIP_TUNED_SR / 22050.0, LIP_MATCH));
		let reference = |hz: f32| {
			let w = 2.0 * PI * hz / 22050.0;
			10.0 * (1.0 + d * d - 2.0 * d * w.cos()).log10()
		};
		for hz in [1000.0, 4000.0, 8000.0] {
			let got = gain(hz) - gain(100.0);
			let want = reference(hz) - reference(100.0);
			assert!((got - want).abs() < 0.75, "{} Hz: {} dB over 100 Hz, the reference {}", hz, got, want);
		}
	}

	#[test]
	#[cfg(not(feature = "rate-44100"))]
	fn lip_radiation_tilts_less_than_its_raw_coefficient() {
		// at this rate the raw coefficient lifts the highs more than at its tuned rate.
		// radiate() takes part of that back and leaves the low end alone
		let gain = |hz: f32, f: &dyn Fn(&mut vtm, f32) -> f32| {
			let mut v = vtm::new();
			let mut peak = 0.0f32;
			for i in 0..20000 {
				let y = f(&mut v, (2.0 * PI * hz * i as f32 / m_sr as f32).sin());
				if i > 10000 { peak = peak.max(y.abs()); }
			}
			20.0 * peak.log10()
		};
		let raw = |v: &mut vtm, x: f32| v.lip(to_q(x, SIG_BITS), to_q(LIP_D, COEF_BITS)) as f32 * OUT_SCALE;
		let ours = |v: &mut vtm, x: f32| v.radiate(to_q(x, SIG_BITS)) as f32 * OUT_SCALE;
		assert!((gain(100.0, &ours) - gain(100.0, &raw)).abs() < 0.2, "low end moved");
		let cut = gain(4000.0, &raw) - gain(4000.0, &ours);
		assert!(cut > 0.5 && cut < 2.5, "4 kHz trimmed {} dB", cut);
	}

	// harmonic-to-rest energy in dB between lo and hi Hz, for x at fundamental f0 (Hann-windowed DFT)
	fn hnr(x: &[f32], f0: f32, lo: f32, hi: f32) -> f32 {
		let n = x.len();
		let (mut h, mut r) = (0.0f64, 0.0f64);
		let k0 = (lo * n as f32 / m_sr as f32) as usize;
		let k1 = (hi * n as f32 / m_sr as f32) as usize;
		for k in k0..k1 {
			let (mut re, mut im) = (0.0f64, 0.0f64);
			for (i, &v) in x.iter().enumerate() {
				let w = 0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / n as f64).cos();
				let ph = 2.0 * std::f64::consts::PI * (k * i) as f64 / n as f64;
				re += w * v as f64 * ph.cos();
				im -= w * v as f64 * ph.sin();
			}
			let hz = k as f32 * m_sr as f32 / n as f32;
			let near = ((hz / f0) - (hz / f0).round()).abs() < 0.08;
			if near { h += re * re + im * im } else { r += re * re + im * im }
		}
		10.0 * (h / r).log10() as f32
	}

	// one sample of the source at a steady pitch, as a float
	fn source(g: &mut LfSource, f0: f32, rd: f32) -> f32 { g.step(phase_step(f0) as u32, f0, rd).0 as f32 * OUT_SCALE }

	// hnr() of the source after half a second, over 0.37 s at any rate so the harmonics stay as sharp in Hz
	const HNR_SPAN: usize = 8192 * (m_sr / 22050) as usize;
	fn source_hnr(g: &mut LfSource, f0: f32, lo: f32, hi: f32) -> f32 {
		let x: Vec<f32> = (0..m_sr as usize / 2 + HNR_SPAN).map(|_| source(g, f0, 0.6)).collect();
		hnr(&x[x.len() - HNR_SPAN..], f0, lo, hi)
	}

	#[test]
	fn glottal_pulses_keep_exact_time() {
		// at a period that is no whole number of samples, the voice's highs are still harmonic
		let h = source_hnr(&mut LfSource::new(), 120.0, 2000.0, 4000.0);
		assert!(h > 40.0, "2-4 kHz harmonics only {} dB over the rest", h);
		// and on a sung high C, where the pulses come every 21 samples at 22050 Hz
		let h = source_hnr(&mut LfSource::new(), 1046.0, 2000.0, 8000.0);
		assert!(h > 40.0, "at 1046 Hz the 2-8 kHz harmonics only {} dB over the rest", h);
	}

	#[test]
	fn voicing_starts_with_a_pulse() {
		// after a silence of any length, a pulse fires on the first voiced sample
		for silent in [100, 160, 230, 300] {
			let mut v = vtm::new();
			for i in 0..silent + 2 * FRAME as usize {
				v.F0 = 120.0;
				v.AV = if i < silent { 0.0 } else { DB_GAIN[60] };
				v.step();
				if v.voiced { break; }
			}
			assert!(v.voiced, "after {} silent samples the voice never started", silent);
			let since_pulse = v.glottal.phase as f32 / v.glottal.inc as f32;
			assert!(since_pulse < 1.01, "after {} silent samples the last pulse was {} samples ago", silent, since_pulse);
		}
	}

	#[test]
	fn breath_follows_the_glottal_flow() {
		// silent while the glottis is closed, and oq/2 of steady noise's power over a period (what the gain undoes)
		let mut g = LfSource::new();
		let (mut closed, mut power, n) = (0, 0.0f32, 20000);
		for _ in 0..n {
			let open = g.openness() as f32 / 16384.0;
			if open == 0.0 { closed += 1; }
			power += open * open;
			source(&mut g, 120.0, 0.9);
		}
		let closed_share = closed as f32 / n as f32;
		assert!((closed_share - (1.0 - g.oq)).abs() < 0.02, "closed {} of the time, oq {}", closed_share, g.oq);
		let ratio = power / n as f32 / (g.oq / 2.0);
		assert!((ratio - 1.0).abs() < 0.05, "breath power {} of the expected", ratio);
	}

	#[test]
	fn pulse_is_centred_on_its_time() {
		let mut g = LfSource::new();
		g.n = 100;
		g.add_pulse(0.3, 1.0);
		let (mut sum, mut at) = (0.0f32, 0.0f32);
		for j in 96..96 + PULSE_RING {
			let w = g.exc[j % PULSE_RING] as f32 * OUT_SCALE;
			sum += w;
			at += w * j as f32;
		}
		assert!((sum - 1.0).abs() < 1e-4, "pulse sums to {}", sum);
		let want = (100 + PULSE_HALF) as f32 - 0.3;
		assert!((at / sum - want).abs() < 0.02, "pulse at {} not {}", at / sum, want);
	}

	#[test]
	fn jitter_and_shimmer_average_what_they_are_set_to() {
		// the period and level change between pulses by the set amount on average
		let mut g = LfSource::new();
		let (mut k, mut a) = (Vec::new(), Vec::new());
		g.jitter = 1.0;
		g.shimmer = 0.5;
		for _ in 0..20000 {
			let r = PERTURB_SPREAD * g.rand();
			k.push(1.0 + g.jitter / 100.0 * r);
			a.push(g.shimmer * PERTURB_SPREAD * g.rand());
		}
		let avg = |v: &[f32]| v.windows(2).map(|w| (w[1] - w[0]).abs()).sum::<f32>() / (v.len() - 1) as f32;
		assert!((avg(&k) * 100.0 - 1.0).abs() < 0.05, "jitter {} %", avg(&k) * 100.0);
		assert!((avg(&a) - 0.5).abs() < 0.03, "shimmer {} dB", avg(&a));
		// and the pulses feel it: 1% jitter blurs the 2-4 kHz harmonics that stay clean without it
		let mut g = LfSource::new();
		g.jitter = 1.0;
		let h = source_hnr(&mut g, 120.0, 2000.0, 4000.0);
		assert!(h < 30.0, "2-4 kHz harmonics {} dB over the rest with jitter", h);
	}

	#[test]
	fn flutter_is_centred_and_seamless() {
		// one value a frame for 12 s, across the clock's 10 s wrap
		let frames = (12 * m_sr / FRAME) as usize;
		let mut g = LfSource::new();
		let f: Vec<f32> = (0..frames).map(|_| g.flutter(120.0, 75.0)).collect();
		let mean = f.iter().sum::<f32>() / f.len() as f32;
		assert!(mean.abs() < 0.1, "flutter offsets F0 by {} Hz", mean);
		// the pitch glides from one frame's value to the next, so what a sample hears is a frame's change spread
		// over FRAME samples
		let jump = f.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0, f32::max) / FRAME as f32;
		assert!(jump < 0.05, "flutter jumps {} Hz in a sample", jump);
		let mut g = LfSource::new();
		let hi: Vec<f32> = (0..frames / 6).map(|_| g.flutter(240.0, 75.0)).collect();
		let rms = |v: &[f32]| (v.iter().map(|x| x * x).sum::<f32>() / v.len() as f32).sqrt();
		let ratio = rms(&hi) / rms(&f[..frames / 6]);
		assert!((ratio - 2.0).abs() < 0.05, "flutter at 240 Hz is {}x that at 120", ratio);
	}

	#[test]
	fn noise_does_not_repeat_within_seconds() {
		let mut n = noise::new();
		let x: Vec<i32> = (0..4 * m_sr).map(|_| n.step()).collect();
		let head = &x[..64];
		assert!(!x[1..].windows(64).any(|w| w == head), "noise repeats within 4 s");
	}

	#[test]
	fn voices_scale_the_formants_they_are_given() {
		// a 150% voice sounds like the table voice with formants 1.5 times higher and wider
		let big = Voice { f1: 150.0, f2: 150.0, ..VOICES[0] };
		assert_eq!(steady(big, 400.0, 1000.0, 80.0, 90.0), steady(VOICES[0], 600.0, 1500.0, 120.0, 135.0));
		assert_ne!(steady(big, 400.0, 1000.0, 80.0, 90.0), steady(VOICES[0], 600.0, 1500.0, 80.0, 90.0));
	}

	#[test]
	fn voices_keep_formants_below_nyquist() {
		for (i, voice) in VOICES.iter().enumerate() {
			let mut v = vtm::new();
			v.set_voice(*voice);
			for r in [&v.VR8, &v.FR9, &v.FR10] {
				assert!(r.bf <= F_MAX_SR * m_sr as f32, "voice {} puts a formant at {} Hz", i, r.bf);
			}
		}
	}

	#[test]
	fn db_gain_table_follows_the_curve() {
		let curve = |db: f32| 0.5f64.powf((G_MAX as f64 - db as f64) / 6.0);
		for (i, &g) in DB_GAIN.iter().enumerate() {
			assert!(((g as f64) / curve(i as f32) - 1.0).abs() < 1e-6, "{} dB: {} in the table", i, g);
		}
		let worst = (1..8700)
			.map(|k| k as f32 / 100.0)
			.map(|db| (20.0 * (vtm::db2lin(db) as f64 / curve(db)).log10()).abs())
			.fold(0.0, f64::max);
		assert!(worst < 0.02, "the interpolated gain is off by {} dB", worst);
	}

	#[test]
	fn cos_table_puts_resonators_where_they_belong() {
		// compare the frequency the table's cosine gives with the asked one, up to F_MAX_SR
		let off = |hz: u32| {
			let got = (cos_hz(hz as f32) as f64).clamp(-1.0, 1.0).acos() * m_sr as f64 / (2.0 * core::f64::consts::PI);
			(got - hz as f64).abs()
		};
		let top = (F_MAX_SR * m_sr as f32) as u32;
		for (from, most) in [(50, 1.1), (150, 0.4)] {
			let worst = (from..=top).map(off).fold(0.0, f64::max);
			assert!(worst < most, "from {} Hz a resonator lands {} Hz off", from, worst);
		}
		// above half the rate, the mirror image
		assert!((cos_hz(m_sr as f32 * 0.6) - cos_hz(m_sr as f32 * 0.4)).abs() < 1e-6);
	}

	#[test]
	fn exp_table_gives_resonators_their_bandwidth() {
		// compare the bandwidth the table's radius gives with the asked one, over every bandwidth a resonator gets
		let worst = (0..=6000).map(|bw| {
			let got = -(pole_radius(bw as f32) as f64).ln() * m_sr as f64 / core::f64::consts::PI;
			(got - bw as f64).abs()
		}).fold(0.0, f64::max);
		assert!(worst < 0.01, "a resonator's bandwidth is {} Hz off", worst);
	}
}
