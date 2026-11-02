/*

#########################################
# VOCAL TRACT MODEL FOR TTS SYNTHESIZER #
#########################################

  author: anthony c. bartman (@rommix0)
  date:   4/6/2026 - 11/01/2026

*/

// use core's PI and reference hertz for pitch track stuff
use core::f32::consts::PI;
use crate::_f0::REF_HZ;

// master sample rate
pub const m_sr: u32 = 22050;

// soft clip default
const CLIP_LIMIT: f32 = 0.85;

// lip radiation coefficient, tuned at LIP_TUNED_SR; a per-sample filter tilts brighter at a lower rate, and
// LIP_MATCH is the share of that extra tilt taken back (0: none, 1: the tuned response)
const LIP_D: f32 = 0.5;
const LIP_TUNED_SR: f32 = 32000.0;
const LIP_MATCH: f32 = 0.5;

// jitter and shimmer are drawn uniformly up to this many times their average cycle-to-cycle change (2/3 of the
// largest draw), so the voice profile's numbers are that average
const PERTURB_SPREAD: f32 = 1.5;

// output trim, keeps peaks under the soft clip
const OUT_DB: f32 = -2.0;

// makeup gain for the cascade branch (unity DC gain per resonator)
const CASC_GAIN: f32 = 2328.0;

// high frequency emphasis for the cascade branch
const CASC_EMPH: f32 = 0.75;

// frication noise: rolled off above this, so consonants are not too bright
const FRIC_LP_HZ: f32 = 4500.0;

// narrowest F1/F2 bandwidth (Hz) the cascade uses
const CASC_MIN_BW: f32 = 60.0;

// amplitudes in dB are relative to this full scale
const G_MAX: f32 = 87.0;

// gain for each whole dB from 0 to G_MAX: 0.5^((G_MAX - dB) / 6), every 6 dB halving it
pub const DB_GAIN: [f32; 88] = [
	4.3158372e-05, 4.8443635e-05, 5.437614e-05, 6.1035156e-05, 6.8509646e-05, 7.689948e-05, 8.6316744e-05, 9.688727e-05,
	0.00010875228, 0.00012207031, 0.00013701929, 0.00015379896, 0.00017263349, 0.00019377454, 0.00021750457, 0.00024414062,
	0.00027403858, 0.00030759792, 0.00034526698, 0.00038754908, 0.00043500913, 0.00048828125, 0.00054807717, 0.00061519584,
	0.00069053395, 0.00077509816, 0.00087001827, 0.0009765625, 0.0010961543, 0.0012303917, 0.0013810679, 0.0015501963,
	0.0017400365, 0.001953125, 0.0021923087, 0.0024607833, 0.0027621358, 0.0031003926, 0.003480073, 0.00390625,
	0.0043846173, 0.0049215667, 0.0055242716, 0.0062007853, 0.006960146, 0.0078125, 0.008769235, 0.009843133,
	0.011048543, 0.012401571, 0.013920292, 0.015625, 0.01753847, 0.019686267, 0.022097087, 0.024803141,
	0.027840585, 0.03125, 0.03507694, 0.039372534, 0.044194173, 0.049606282, 0.05568117, 0.0625,
	0.07015388, 0.07874507, 0.088388346, 0.099212565, 0.11136234, 0.125, 0.14030775, 0.15749013,
	0.17677669, 0.19842513, 0.22272468, 0.25, 0.2806155, 0.31498027, 0.35355338, 0.39685026,
	0.44544935, 0.5, 0.561231, 0.62996054, 0.70710677, 0.7937005, 0.8908987, 1.0,
];

// formants closer than MIN_D (Hz) get their bandwidths widened, by up to 1 + MUL_B times when they meet
const MIN_D: f32 = 350.0;
const MUL_B: f32 = 2.5;

// low shelf on the cascade branch: this much more level (dB) below about LOW_SHELF_HZ
const LOW_SHELF_DB: f32 = 4.0;
const LOW_SHELF_HZ: f32 = 250.0;

// how much voicing modulates breath noise and frication (1 is half-level noise in every other half period, 2 is none)
const BREATH_MOD: f32 = 0.3;
const FRIC_MOD: f32 = 1.0;

// makeup gain for the parallel branch (frication)
const PARR_GAIN: f32 = 2.5;

// init constants and nasal murmur
const RA_REF:   f32 = 3.8;
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
	Voice { f0: 100.0, range: 100.0, f1: 100.0, f2: 100.0, f3: 100.0, fhi: 100.0, rd: 100.0, br: 0.0, db:  0.0, fl:  75.0, jit: 0.2, shim: 0.2 },	// 0 table voice
	Voice { f0:  85.0, range:  95.0, f1:  90.0, f2:  91.0, f3:  92.0, fhi:  92.0, rd: 100.0, br: 0.0, db:  0.5, fl:  75.0, jit: 0.2, shim: 0.2 },	// 1 large head
	Voice { f0: 190.0, range: 115.0, f1: 122.0, f2: 117.0, f3: 115.0, fhi: 112.0, rd: 140.0, br: 4.0, db: -2.5, fl:  75.0, jit: 0.2, shim: 0.2 },	// 2 female
	Voice { f0: 235.0, range: 120.0, f1: 135.0, f2: 130.0, f3: 127.0, fhi: 124.0, rd: 125.0, br: 2.0, db: -5.0, fl:  75.0, jit: 0.2, shim: 0.2 },	// 3 child
	Voice { f0:  65.0, range:  80.0, f1:  80.0, f2:  82.0, f3:  83.0, fhi:  84.0, rd:  95.0, br: 0.0, db:  2.0, fl:  75.0, jit: 0.6, shim: 0.4 },	// 4 giant
	Voice { f0:  70.0, range: 100.0, f1: 135.0, f2: 132.0, f3: 130.0, fhi: 125.0, rd: 100.0, br: 0.0, db:  1.5, fl:  75.0, jit: 0.2, shim: 0.2 },	// 5 deep voice, tiny head
	Voice { f0: 100.0, range: 160.0, f1:  70.0, f2: 130.0, f3:  85.0, fhi: 100.0, rd: 100.0, br: 0.0, db:  1.0, fl:  75.0, jit: 0.2, shim: 0.2 },	// 6 pulled apart
	Voice { f0:  95.0, range: 110.0, f1: 100.0, f2: 100.0, f3: 100.0, fhi: 100.0, rd: 110.0, br: 2.0, db:  0.5, fl: 250.0, jit: 1.5, shim: 1.0 },	// 7 shaky
];

struct noise { alpha: f32, nxt: u32, xp1: f32 }
impl noise {
	fn new() -> noise {
		// the aspiration source's smoothing was tuned as 0.75 per sample at 32 kHz; keep that corner (~1.5 kHz)
		noise { alpha: libm::powf(0.75, 32000.0 / m_sr as f32), nxt: 0x2545F491, xp1: 0.0 }
	}
	fn xorshift(&mut self) {
		// 32-bit state: repeats after 2^32 - 1 samples instead of 3 s
		self.nxt ^= self.nxt << 13;
		self.nxt ^= self.nxt >> 17;
		self.nxt ^= self.nxt << 5;
	}
	fn step(&mut self) -> (f32, f32) {
		// white noise and the smoothed aspiration noise
		self.xorshift();
		let n: f32 = ((self.nxt >> 16) as u16 as i16) as f32 / 32767.0;
		let f: f32 = (1.0 - self.alpha) * n + self.alpha * self.xp1;
		self.xp1 = f;
		(n, f)
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

// cos(2 pi hz / m_sr) from COS_TABLE, a straight line between entries: the resonator lands within 0.4 Hz of hz from
// 150 Hz to F_MAX_SR, about 1 Hz off at 50 Hz (the line strays most where the cosine is flattest)
fn cos_hz(hz: f32) -> f32 {
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

// a resonator's pole radius for bandwidth bw, from EXP_TABLE with a straight line between entries: within 0.01 Hz of bw;
// wider than the table reaches (3.5 kHz at 22050 Hz, only the fixed upper resonators) it is worked out exactly
fn pole_radius(bw: f32) -> f32 {
	let x = PI * bw.max(0.0) / m_sr as f32;
	if x >= EXP_TOP { return libm::expf(-x); }
	let t = x * (EXP_STEPS as f32 / EXP_TOP);
	let i = (t as usize).min(EXP_STEPS - 1);
	EXP_TABLE[i] + (EXP_TABLE[i + 1] - EXP_TABLE[i]) * (t - i as f32)
}

// klatt style resonator
struct resonator {
	a:     f32,
	b:     f32,
	c:     f32,
	xp1:   f32,
	xp2:   f32,
	anti:  bool,
	bf:    f32,
	bw:    f32,
	pfreq: f32,
}
impl resonator {
	fn new(bf: f32, bw: f32, anti: bool) -> resonator {
		let mut r = resonator { a: 0.0, b: 0.0, c: 0.0, xp1: 0.0, xp2: 0.0, anti, bf, bw, pfreq: 0.0 };
		r.set_coeffs(bf, bw);
		r
	}
	fn set_coeffs(&mut self, bf: f32, bw: f32) {
		let r: f32 = pole_radius(bw);
		self.c = -(r * r);
		self.b = 2.0 * r * cos_hz(bf);
		self.a = 1.0 - self.b - self.c;

		// anti-resonator switch for special
		// resonators like nasal pairs
		if self.anti {
			self.a = 1.0 / self.a;
			self.b = -self.a * self.b;
			self.c = -self.a * self.c;
		}

		// Fujisaki-style gain compensation (KLSYN88-Qlatt)
		// replaces old DC gain equation as this works much better
		if self.pfreq != 0.0 && bf < self.pfreq {
			let g = bf / self.pfreq;
			self.xp1 *= g;
			self.xp2 *= g;
		}
		self.pfreq = bf;
		self.bf = bf;
		self.bw = bw;
	}
	fn step(&mut self, x: f32, bf: f32, bw: f32) -> f32 {
		let mut bff: f32 = bf;
		let mut bww: f32 = bw;
		if bff == 0.0 { bff = self.bf; }
		if bww == 0.0 { bww = self.bw; }
		// a fixed resonator (stepped with 0, 0) keeps the coefficients it was made with
		if bf > 0.0 || bw > 0.0 { self.set_coeffs(bff, bww); }
		let ot: f32 = self.a * x + self.b * self.xp1 + self.c * self.xp2;
		self.xp2 = self.xp1;
		self.xp1 = if self.anti { x } else { ot };
		ot
	}
}

// glottal model (refactoring of Qlatt's version of the Liljencrants-Fant glottal model using filters)
struct Biquad {
	b0: f32,
	b1: f32,
	b2: f32,
	a1: f32,
	a2: f32,
	x1: f32,
	x2: f32,
	y1: f32,
	y2: f32,
}
impl Biquad {
	fn new() -> Biquad {
		Biquad {
			b0: 0.0,
			b1: 0.0,
			b2: 0.0,
			a1: 0.0,
			a2: 0.0,
			x1: 0.0,
			x2: 0.0,
			y1: 0.0,
			y2: 0.0,
		}
	}
	fn set(&mut self, a: f32, b: f32, c: f32, d: f32, e: f32) {
		self.b0 = a;
		self.b1 = b;
		self.b2 = c;
		self.a1 = d;
		self.a2 = e;
	}
	fn step(&mut self, x: f32) -> f32 {
		let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2 - self.a1 * self.y1 - self.a2 * self.y2;
		self.x2 = self.x1;
		self.x1 = x;
		self.y2 = self.y1;
		self.y1 = y;
		y
	}
}

struct LpPole { a: f32, y1: f32, }
impl LpPole {
	fn new() -> LpPole { LpPole { a: 0.0, y1: 0.0, } }
	fn set(&mut self, a: f32) { self.a = a; }
	fn step(&mut self, x: f32) -> f32 {
		let y = (1.0 - self.a) * x + self.a * self.y1;
		self.y1 = y;
		y
	}
}

// a glottal pulse is spread over this many samples either side of its exact time, so the voice plays this many samples late
const PULSE_HALF: usize = 4;
const PULSE_RING: usize = 16;

struct LfSource {
	pos_in_period: f32,
	last_inc: f32,				// period fraction the last sample advanced by
	exc: [f32; PULSE_RING],		// excitation still to play, by absolute sample index
	n: usize,					// absolute sample index
	glottal: Biquad,
	tilt: LpPole,
	counter: u32,
	flut_from: f32,				// the flutter sines at the start of this millisecond
	flut_to:   f32,				// and at its end
	ra: f32,
	jitter: f32,				// average period change between pulses, in %
	shimmer: f32,				// average level change between pulses, in dB
	rng: u32,					// state for jitter and shimmer
	jit_k: f32,					// this period's rate factor
}
impl LfSource {
	fn new() -> LfSource {
		LfSource {
			pos_in_period: 0.0,
			last_inc: 0.0,
			exc: [0.0; PULSE_RING],
			n: 0,
			glottal: Biquad::new(),
			tilt: LpPole::new(),
			counter: 0,
			flut_from: 0.0,
			flut_to: 0.0,
			ra: 0.0,
			jitter: 0.0,
			shimmer: 0.0,
			rng: 0x6C8E9CF5,
			jit_k: 1.0,
		}
	}
	fn rand(&mut self) -> f32 {
		// uniform in -1..1
		self.rng ^= self.rng << 13;
		self.rng ^= self.rng >> 17;
		self.rng ^= self.rng << 5;
		self.rng as f32 / u32::MAX as f32 * 2.0 - 1.0
	}
	fn add_pulse(&mut self, ago: f32, amp: f32) {
		// a band-limited unit impulse at (n - ago), written PULSE_HALF samples ahead of what plays now
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
			self.exc[(first + k) % PULSE_RING] += wk / sum * amp;
		}
	}
	fn calc_fant(&mut self, f0: f32, rd: f32) {
		// get Fant 1997 parameters
		let t0 = 1.0 / f0;
		self.ra = (-1.0 + 4.8  * rd) / 100.0;
		let rk = (22.4 + 11.8 * rd) / 100.0;
		let rg_denom = 0.44 * rd - 4.0 * self.ra * (0.5 + 1.2 * rk);
		let rg = rk * (0.5 + 1.2 * rk) / rg_denom;

		// open quotient (pulse width of glottal basically)
		let oq = (1.0 + rk) / (2.0 * rg);

		// glottal frequency and bandwidth calculations
		let fg = 1.0 / (2.0 * oq * t0);
		let alpha_m = 1.0 / (1.0 + rk);
		let bg = 1.0 / (oq  * t0 * libm::tanf(PI * (1.0 - alpha_m)));

		// calculate coefficients for glottal biquad filter
		let a1 = -2.0 * libm::expf(-PI * bg / m_sr as f32) * libm::cosf(2.0 * PI * fg / m_sr as f32);
		let a2 = libm::expf(-2.0 * PI * bg / m_sr as f32);
		let ag = 1.0 / libm::sinf(PI * (1.0 - alpha_m));
		let b0 = 0.0;
		let b1 = -ag;
		let b2 = ag;
		self.glottal.set(b0, b1, b2, a1, a2);

		// calculate coefficients for tilt filter
		let fa = 1.0 / (2.0 * PI * self.ra * t0);
		self.tilt.set(libm::expf(-2.0 * PI * fa / m_sr as f32));
	}
	fn flutter_sines(c: u32) -> f32 {
		let w = 2.0 * PI * (c as f32 / m_sr as f32);
		libm::sinf(12.7 * w) + libm::sinf(7.1 * w) + libm::sinf(4.7 * w)
	}
	fn calc_flutter(&mut self, f0: f32, flutter: f32) -> f32 {
		// KLGLOTT88 pitch flutter algorithm
		if flutter <= 0.0 { return 0.0; }
		// the sines barely move in a millisecond: worked out at each end of one, a straight line between
		let step = m_sr / 1000;
		let n = self.counter % step;
		if n == 0 { (self.flut_from, self.flut_to) = (Self::flutter_sines(self.counter), Self::flutter_sines(self.counter + step)); }
		let a = self.flut_from + (self.flut_to - self.flut_from) * (n as f32 / step as f32);
		// 10 s holds a whole number of cycles of all three, so the wrap is seamless
		self.counter = (self.counter + 1) % (10 * m_sr);
		a * (flutter / 50.0) * (f0 / 100.0)
	}
	fn step(&mut self, f0: f32, rd: f32, flutter: f32) -> (f32, f32) {
		// slow F0 flutter on top of the asked pitch
		let f0_final = f0 + self.calc_flutter(f0, flutter);

		// determine noise modulation amount
		let nmod = if self.pos_in_period >= 0.5 { 0.5 } else { 1.0 };

		// start a glottal pulse at the exact time the period ended, somewhere since the last sample
		if self.pos_in_period == 0.0 || self.pos_in_period >= 1.0 {
			self.calc_fant(f0, rd);
			let ago = if self.last_inc > 0.0 { ((self.pos_in_period - 1.0) / self.last_inc).clamp(0.0, 0.999) } else { 0.0 };
			self.pos_in_period %= 1.0;
			// jitter stretches or shrinks this period, shimmer scales this pulse
			self.jit_k = 1.0 / (1.0 + PERTURB_SPREAD * self.jitter / 100.0 * self.rand());
			let amp = libm::powf(10.0, PERTURB_SPREAD * self.shimmer * self.rand() / 20.0);
			self.add_pulse(ago, amp);
		}
		let slot = self.n % PULSE_RING;
		let impulse = self.exc[slot];
		self.exc[slot] = 0.0;
		self.n += 1;

		let sample = self.tilt.step(self.glottal.step(impulse));

		self.last_inc = f0_final * self.jit_k / m_sr as f32;
		self.pos_in_period += self.last_inc;
		(sample, nmod)
	}
}

// vocal tract model - the main event
// AV, AH, AF, A2-A10 and AB are linear gains (1 is full scale, 87 dB), as _synth.rs keeps them; BR is in dB
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
	VR1:      resonator,
	VR2:      resonator,
	VR3:      resonator,
	VR4:      resonator,
	VR5:      resonator,
	VR6:      resonator,
	VR7:      resonator,
	VR8:      resonator,
	NPR:      resonator,
	NZR:      resonator,
	FR2:      resonator,
	FR3:      resonator,
	FR4:      resonator,
	FR5:      resonator,
	FR6:      resonator,
	FR7:      resonator,
	FR8:      resonator,
	FR9:      resonator,
	FR10:     resonator,
	xp1:      f32,
	ep1:      f32,
	shelf_y1: f32,
	fric_y1:  f32,		// frication noise lowpass state
	voice:    Voice,	// voice profile
	vgain:    f32,		// the voice profile's output level, linear
	fric_a:   f32,		// frication lowpass coefficient
	shelf_a:  f32,		// low shelf lowpass coefficient
	shelf_g:  f32,		// low shelf: the lowpassed copy's gain
	lip_d:    f32,		// lip radiation coefficient at this rate
	out_g:    f32,		// output trim, linear
}
impl vtm {
	pub fn new() -> vtm {
		let mut tract = vtm {
			F0     : 125.0,				// fundamental frequency (Hz)
			AV     : DB_GAIN[60],		// Amplitude of voicing (60 dB)
			RD     : 0.0,				// RD fant tilt param (for glottal pulse model)
			AH     : 0.0,				// Amplitude of aspiration, in dB
			AF     : 0.0,				// Amplitude of frication, in dB
			BR     : 6.0,				// breathiness amp (dB)
			F1     : F1_INIT,			// Frequency of 1st formant, in Hz
			B1     : B1_INIT,			// Bandwidth of 1st formant, in Hz
			F2     : F2_INIT,			// Frequency of 2nd formant, in Hz
			B2     : B2_INIT,			// Bandwidth of 2nd formant, in Hz
			F3     : F3_INIT,			// Frequency of 3rd formant, in Hz
			B3     : B3_INIT,			// Bandwidth of 3rd formant, in Hz
			NC     : 0.0,				// nasal coupling
			A2     : 0.0,				// Amp of parallel 2nd formant, in dB
			A3     : 0.0,				// Amp of parallel 3rd formant, in dB
			A4     : 0.0,				// Amp of parallel 4th formant, in dB
			A5     : 0.0,				// Amp of parallel 5th formant, in dB
			A6     : 0.0,				// Amp of parallel 6th formant, in dB
			A7     : 0.0,				// Amp of parallel 7th formant, in dB
			A8     : 0.0,				// Amp of parallel 8th formant, in dB
			A9     : 0.0,				// Amp of parallel 9th formant, in dB
			A10    : 0.0,				// Amp of parallel 10th formant, in dB
			AB     : 0.0,				// Amp of parallel bypass path, in dB
			B1_ADJ : 1.0,				// B1 adjustment factor
			B2_ADJ : 1.0,				// B2 adjustment factor
			B3_ADJ : 1.0,				// B3 adjustment factor
			noisegen : noise::new(),
			glottal  : LfSource::new(),
			VR1      : resonator::new(F1_INIT,  B1_INIT,  false),
			VR2      : resonator::new(F2_INIT,  B2_INIT,  false),
			VR3      : resonator::new(F3_INIT,  B3_INIT,  false),
			VR4      : resonator::new(F4_INIT,  B4_INIT,  false),
			VR5      : resonator::new(F5_INIT,  B5_INIT,  false),
			VR6      : resonator::new(F6_INIT,  B6_INIT,  false),
			VR7      : resonator::new(F7_INIT,  B7_INIT,  false),
			VR8      : resonator::new(F8_INIT,  B8_INIT,  false),
			NPR      : resonator::new(FN_INIT,  BN_INIT,  false),
			NZR      : resonator::new(FN_INIT,  BN_INIT,  true),
			FR2      : resonator::new(F2_INIT,  B2_INIT,  false),
			FR3      : resonator::new(F3_INIT,  B3_INIT,  false),
			FR4      : resonator::new(F4_INIT,  B4_INIT,  false),
			FR5      : resonator::new(F5_INIT,  B5_INIT,  false),
			FR6      : resonator::new(F6_INIT,  B6_INIT,  false),
			FR7      : resonator::new(F7_INIT,  B7_INIT,  false),
			FR8      : resonator::new(F8_INIT,  B8_INIT,  false),
			FR9      : resonator::new(F9_INIT,  B9_INIT,  false),
			FR10     : resonator::new(F10_INIT, B10_INIT, false),

			// past sample for lip radiation filter
			xp1: 0.0,

			// past sample for cascade emphasis filter
			ep1: 0.0,
			shelf_y1: 0.0,
			fric_y1: 0.0,
			voice: VOICES[0],
			vgain: 1.0,
			fric_a:  libm::expf(-2.0 * PI * FRIC_LP_HZ / m_sr as f32),
			shelf_a: libm::expf(-2.0 * PI * LOW_SHELF_HZ / m_sr as f32),
			shelf_g: libm::powf(10.0, LOW_SHELF_DB / 20.0) - 1.0,
			lip_d:   libm::powf(LIP_D, libm::powf(LIP_TUNED_SR / m_sr as f32, LIP_MATCH)),
			out_g:   libm::powf(10.0, OUT_DB / 20.0),
		};
		// voice 0 in full: its jitter and shimmer live in the glottal source, which only set_voice reaches
		tract.set_voice(VOICES[0]);
		tract
	}
	pub fn set_voice(&mut self, v: Voice) {
		// rebuild the fixed upper formants at the voice's scale (the lower ones are scaled every sample)
		self.voice = v;
		self.vgain = libm::powf(10.0, v.db / 20.0);
		self.glottal.jitter = v.jit;
		self.glottal.shimmer = v.shim;
		let f = |hz: f32| (hz * (v.fhi / 100.0)).min(F_MAX_SR * m_sr as f32);
		self.VR4  = resonator::new(f(F4_INIT),  B4_INIT,  false);
		self.VR5  = resonator::new(f(F5_INIT),  B5_INIT,  false);
		self.VR6  = resonator::new(f(F6_INIT),  B6_INIT,  false);
		self.VR7  = resonator::new(f(F7_INIT),  B7_INIT,  false);
		self.VR8  = resonator::new(f(F8_INIT),  B8_INIT,  false);
		self.FR4  = resonator::new(f(F4_INIT),  B4_INIT,  false);
		self.FR5  = resonator::new(f(F5_INIT),  B5_INIT,  false);
		self.FR6  = resonator::new(f(F6_INIT),  B6_INIT,  false);
		self.FR7  = resonator::new(f(F7_INIT),  B7_INIT,  false);
		self.FR8  = resonator::new(f(F8_INIT),  B8_INIT,  false);
		self.FR9  = resonator::new(f(F9_INIT),  B9_INIT,  false);
		self.FR10 = resonator::new(f(F10_INIT), B10_INIT, false);
	}
	fn apply_voice(&mut self) {
		// scale this sample's pitch, formants and source by the voice profile
		// like fix_bw, this relies on _synth setting them again every sample
		let v = self.voice;
		let top = F_MAX_SR * m_sr as f32;
		if v.range != 100.0 {
			self.F0 = REF_HZ * libm::powf(self.F0 / REF_HZ, v.range / 100.0);
		}
		self.F0 *= v.f0 / 100.0;
		self.F1 = (self.F1 * (v.f1 / 100.0)).min(top);
		self.F2 = (self.F2 * (v.f2 / 100.0)).min(top);
		self.F3 = (self.F3 * (v.f3 / 100.0)).min(top);
		self.RD *= v.rd / 100.0;
		self.BR += v.br;
	}
	fn db2lin(&mut self, gain: f32) -> f32 {
		if gain <= 0.0 { return 0.0; }
		if gain >= G_MAX { return 1.0; }
		// between whole dB, a straight line: within 0.01 dB of the curve
		let i = gain as usize;
		DB_GAIN[i] + (DB_GAIN[i + 1] - DB_GAIN[i]) * (gain - i as f32)
	}
	fn fix_bw(&mut self) {
		// adjust bandwidths to improve sound quality

		// F0-proximity bandwidth adjust
		self.B1 += 200.0 * libm::powf(self.F0 / self.F1 - 0.5, 2.0);

		// glottal leakage adjust (B1, B2)
		let DB1 = 250.0 * libm::powf(self.F1 / 500.0, 2.0) * (self.glottal.ra * 100.0 - RA_REF) / 12.0;
		let DB2 = DB1 * self.F1 / (2.0 * self.F2);
		self.B1 += DB1;
		self.B2 += DB2;

		// B1 adjust
		self.B1_ADJ = (self.F1 - self.F2).abs();
		if self.B1_ADJ < MIN_D {
			self.B1_ADJ = MUL_B * ((MIN_D - self.B1_ADJ) / MIN_D);
		} else {
			self.B1_ADJ = 0.0;
		}
		self.B1_ADJ += 1.0;

		// B2 adjust
		self.B2_ADJ = (self.F2 - self.F3).abs();
		if self.B2_ADJ < MIN_D {
			self.B2_ADJ = MUL_B * ((MIN_D - self.B2_ADJ) / MIN_D);
		} else {
			self.B2_ADJ = 0.0;
		}
		self.B2_ADJ += 1.0;

		// B3 adjust
		self.B3_ADJ = (self.F3 - self.VR4.bf).abs();
		if self.B3_ADJ < MIN_D {
			self.B3_ADJ = MUL_B * ((MIN_D - self.B3_ADJ) / MIN_D);
		} else {
			self.B3_ADJ = 0.0;
		}
		self.B3_ADJ += 1.0;
	}
	fn nasal_zero(&mut self, s: f32, k: f32) -> f32 {
		// nasal zero for a nasal coupling k (0 oral, 1 nasal consonant) and the current F1: on the nasal pole for an oral
		// sound (they cancel), on F1 for a nasal (the pole is then its murmur), and in between at a vowel's nasalized edge
		FN_INIT + k * ((self.F1 / s) - FN_INIT)
	}
	fn parallel(&mut self, x: f32) -> f32 {
		let mut yy: f32 = 0.0;
		let mut bb2 = self.B2 * self.B2_ADJ;
		let mut bb3 = self.B3 * self.B3_ADJ;
		if bb2 < 40.0 { bb2 = 40.0; }
		if bb3 < 40.0 { bb3 = 40.0; }

		yy -= self.FR2.step(x, self.F2, bb2)  * self.A2.min(1.0);
		yy += self.FR3.step(x, self.F3, bb3)  * self.A3.min(1.0);
		yy -= self.FR4.step(x, 0.0, 0.0)      * self.A4.min(1.0);
		yy += self.FR5.step(x, 0.0, 0.0)      * self.A5.min(1.0);
		yy -= self.FR6.step(x, 0.0, 0.0)      * self.A6.min(1.0);
		yy += self.FR7.step(x, 0.0, 0.0)      * self.A7.min(1.0);
		yy -= self.FR8.step(x, 0.0, 0.0)      * self.A8.min(1.0);
		yy += self.FR9.step(x, 0.0, 0.0)      * self.A9.min(1.0);
		yy -= self.FR10.step(x, 0.0, 0.0)     * self.A10.min(1.0);
		yy += self.AB.min(1.0) * x;
		yy
	}
	fn cascade(&mut self, x: f32) -> f32 {
		let mut bb1 = self.B1 * self.B1_ADJ;
		let mut bb2 = self.B2 * self.B2_ADJ;
		let mut bb3 = self.B3 * self.B3_ADJ;
		if bb1 < CASC_MIN_BW { bb1 = CASC_MIN_BW; }
		if bb2 < CASC_MIN_BW { bb2 = CASC_MIN_BW; }
		if bb3 < 40.0 { bb3 = 40.0; }

		// nasal coupling (Hawkins & Stevens 1985)
		let k = self.NC.clamp(0.0, 1.0);
		let s = self.voice.fhi / 100.0;
		let fnz = self.nasal_zero(s, k) * s;
		let bnz = BN_INIT + k * (bb1 - BN_INIT);
		let bnp = BN_INIT + k * (BN_MURMUR - BN_INIT);
		let fnp = FN_INIT + k * (FN_MURMUR - FN_INIT);

		let mut yy = self.NPR.step(self.NZR.step(x, fnz, bnz), fnp * s, bnp);		// NASAL TRACT
		yy = self.VR1.step(yy, self.F1, bb1);												// F1 - tongue height
		yy = self.VR2.step(yy, self.F2, bb2);												// F2 - tongue fronting
		yy = self.VR3.step(yy, self.F3, bb3);												// F3 - R-coloring
		yy = self.VR4.step(yy, 0.0, 0.0);
		yy = self.VR5.step(yy, 0.0, 0.0);
		yy = self.VR6.step(yy, 0.0, 0.0);
		yy = self.VR7.step(yy, 0.0, 0.0);
		yy = self.VR8.step(yy, 0.0, 0.0);
		yy
	}
	fn softclip(&mut self, x: f32) -> f32 {
		let a = x.abs();
		if a <= CLIP_LIMIT { return x; }
		let excess: f32 = a - CLIP_LIMIT;
		x.signum() * (CLIP_LIMIT + (1.0 - CLIP_LIMIT) * excess / (excess + 1.0))
	}
	fn low_shelf(&mut self, x: f32) -> f32 {
		// add a low-passed copy back on top, so only the bottom of the spectrum gains level
		let a = self.shelf_a;
		self.shelf_y1 = (1.0 - a) * x + a * self.shelf_y1;
		x + self.shelf_g * self.shelf_y1
	}
	fn emph(&mut self, x: f32) -> f32 {
		// brighten the cascade output
		let xx = x - CASC_EMPH * self.ep1;
		self.ep1 = x;
		xx
	}
	fn radiate(&mut self, x: f32) -> f32 {
		// lip radiation with its coefficient moved toward the tuned rate's response, at the same low-frequency gain
		let d = self.lip_d;
		self.lip(x, d) * (1.0 - LIP_D) / (1.0 - d)
	}
	fn lip(&mut self, x: f32, d: f32) -> f32 {
		let xx = x - d * self.xp1;
		self.xp1 = x;
		xx
	}
	pub fn step(&mut self) -> f32 {
		self.apply_voice();
		let (n, mut f) = self.noisegen.step();
		let (mut voice, nmod) = self.glottal.step(self.F0, self.RD, self.voice.fl);
		voice *= 0.01;
		f *= 0.6;

		// adjust bandwidths if formants are close together
		self.fix_bw();

		// noise modulation
		let modulate = |m: f32| if self.AV > 0.0 { 1.0 - m * (1.0 - nmod) } else { 1.0 };
		let (br_mod, af_mod) = (modulate(BREATH_MOD), modulate(FRIC_MOD));

		// route audio sources to a source fed in by cascade filter
		voice   = voice + (n * self.db2lin(self.BR) * br_mod);
		let casc_in: f32 = voice * self.AV.min(1.0) + f * self.AH.min(1.0);

		// route noise source to source fed in by parallel filter
		// frication is broadband noise, rolled off above FRIC_LP_HZ
		let a = self.fric_a;
		self.fric_y1 = (1.0 - a) * n + a * self.fric_y1;
		let parr_in: f32 = self.fric_y1 * self.AF.min(1.0) * af_mod;

		let casc_out: f32 = self.cascade(casc_in);
		let casc_out: f32 = self.emph(casc_out) * CASC_GAIN;
		let casc_out: f32 = self.low_shelf(casc_out);
		let parr_out: f32 = self.parallel(parr_in) * PARR_GAIN;

		let out = self.radiate(casc_out + parr_out) * self.out_g * self.vgain;
		self.softclip(out)
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

	#[test]
	fn low_shelf_lifts_only_the_bottom() {
		// steady DC comes out LOW_SHELF_DB louder; an alternating (Nyquist) signal is nearly unchanged
		let mut v = vtm::new();
		let mut y = 0.0;
		for _ in 0..20000 { y = v.low_shelf(1.0); }
		assert!((20.0 * y.log10() - LOW_SHELF_DB).abs() < 0.1, "DC gain {} dB", 20.0 * y.log10());
		let mut v = vtm::new();
		let mut peak = 0.0f32;
		for i in 0..20000 {
			let x = if i % 2 == 0 { 1.0 } else { -1.0 };
			let y = v.low_shelf(x);
			if i > 10000 { peak = peak.max(y.abs()); }
		}
		assert!((20.0 * peak.log10()).abs() < 0.3, "Nyquist gain {} dB", 20.0 * peak.log10());
	}

	// 4000 samples of a steady vowel with these F1, F2, B1, B2 in voice
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
	fn cascade_bandwidths_have_a_floor() {
		// a B1 or B2 asked below the floor renders exactly as the floor itself
		let render = |b1: f32, b2: f32| steady(VOICES[0], 600.0, 1500.0, b1, b2);
		let floor = render(CASC_MIN_BW, CASC_MIN_BW);
		assert_eq!(render(20.0, 20.0), floor);
		assert_ne!(render(CASC_MIN_BW + 200.0, CASC_MIN_BW), floor);
	}

	#[test]
	fn lip_radiation_tilts_less_than_its_raw_coefficient() {
		// at this rate the raw coefficient lifts the highs more than it did where it was tuned; radiate() trims that
		// part of the way back and leaves the low end where it was
		let gain = |hz: f32, f: &dyn Fn(&mut vtm, f32) -> f32| {
			let mut v = vtm::new();
			let mut peak = 0.0f32;
			for i in 0..20000 {
				let y = f(&mut v, (2.0 * PI * hz * i as f32 / m_sr as f32).sin());
				if i > 10000 { peak = peak.max(y.abs()); }
			}
			20.0 * peak.log10()
		};
		let raw = |v: &mut vtm, x: f32| v.lip(x, LIP_D);
		let ours = |v: &mut vtm, x: f32| v.radiate(x);
		assert!((gain(100.0, &ours) - gain(100.0, &raw)).abs() < 0.2, "low end moved");
		let cut = gain(4000.0, &raw) - gain(4000.0, &ours);
		assert!(cut > 0.5 && cut < 2.5, "4 kHz trimmed {} dB", cut);
	}

	// harmonic energy over the rest, in dB, between lo and hi Hz, of x at fundamental f0 (Hann-windowed DFT)
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

	#[test]
	fn glottal_pulses_keep_exact_time() {
		// at a period that is no whole number of samples, the voice's highs are still harmonic
		let mut g = LfSource::new();
		let x: Vec<f32> = (0..m_sr as usize / 2 + 8192).map(|_| g.step(120.0, 0.6, 0.0).0).collect();
		let tail = &x[x.len() - 8192..];
		let h = hnr(tail, 120.0, 2000.0, 4000.0);
		assert!(h > 40.0, "2-4 kHz harmonics only {} dB over the rest", h);
	}

	#[test]
	fn pulse_is_centred_on_its_time() {
		let mut g = LfSource::new();
		g.n = 100;
		g.add_pulse(0.3, 1.0);
		let (mut sum, mut at) = (0.0f32, 0.0f32);
		for j in 96..96 + PULSE_RING {
			let w = g.exc[j % PULSE_RING];
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
		let x: Vec<f32> = (0..m_sr as usize / 2 + 8192).map(|_| g.step(120.0, 0.6, 0.0).0).collect();
		let h = hnr(&x[x.len() - 8192..], 120.0, 2000.0, 4000.0);
		assert!(h < 30.0, "2-4 kHz harmonics {} dB over the rest with jitter", h);
	}

	#[test]
	fn flutter_is_centred_and_seamless() {
		let mut g = LfSource::new();
		let f: Vec<f32> = (0..12 * m_sr).map(|_| g.calc_flutter(120.0, 75.0)).collect();
		let mean = f.iter().sum::<f32>() / f.len() as f32;
		assert!(mean.abs() < 0.1, "flutter offsets F0 by {} Hz", mean);
		let jump = f.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0, f32::max);
		assert!(jump < 0.05, "flutter jumps {} Hz", jump);
		let mut g = LfSource::new();
		let hi: Vec<f32> = (0..2 * m_sr).map(|_| g.calc_flutter(240.0, 75.0)).collect();
		let rms = |v: &[f32]| (v.iter().map(|x| x * x).sum::<f32>() / v.len() as f32).sqrt();
		let ratio = rms(&hi) / rms(&f[..2 * m_sr as usize]);
		assert!((ratio - 2.0).abs() < 0.05, "flutter at 240 Hz is {}x that at 120", ratio);
		// the straight lines between milliseconds stay on the sines, across the 10 s wrap too
		let exact = |c: u32| LfSource::flutter_sines(c % (10 * m_sr)) * (75.0 / 50.0) * (120.0 / 100.0);
		let worst = f.iter().enumerate().map(|(c, &x)| (x - exact(c as u32)).abs()).fold(0.0, f32::max);
		assert!(worst < 0.01, "flutter strays {} Hz from its sines", worst);
	}

	#[test]
	fn noise_does_not_repeat_within_seconds() {
		let mut n = noise::new();
		let x: Vec<f32> = (0..4 * m_sr).map(|_| n.step().0).collect();
		let head = &x[..64];
		assert!(!x[1..].windows(64).any(|w| w == head), "noise repeats within 4 s");
	}

	#[test]
	fn voices_scale_the_formants_they_are_given() {
		// a voice at 150% renders the table's formants as the table voice renders them 1.5 times higher
		let render = |voice: Voice, f1: f32, f2: f32| steady(voice, f1, f2, 80.0, 90.0);
		let big = Voice { f1: 150.0, f2: 150.0, ..VOICES[0] };
		assert_eq!(render(big, 400.0, 1000.0), render(VOICES[0], 600.0, 1500.0));
		assert_ne!(render(big, 400.0, 1000.0), render(VOICES[0], 400.0, 1000.0));
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
	fn aspiration_noise_corner_is_in_hz() {
		// smoothed as tuned at 32 kHz: corner near 1.47 kHz whatever the rate
		let n = noise::new();
		let fc = -n.alpha.ln() * m_sr as f32 / (2.0 * PI);
		assert!((fc - 1465.0).abs() < 30.0, "aspiration noise corner {} Hz", fc);
	}

	#[test]
	fn db_gain_table_follows_the_curve() {
		let curve = |db: f32| 0.5f64.powf((G_MAX as f64 - db as f64) / 6.0);
		for (i, &g) in DB_GAIN.iter().enumerate() {
			assert!(((g as f64) / curve(i as f32) - 1.0).abs() < 1e-6, "{} dB: {} in the table", i, g);
		}
		let mut v = vtm::new();
		let worst = (1..8700).map(|k| k as f32 / 100.0).map(|db| (20.0 * (v.db2lin(db) as f64 / curve(db)).log10()).abs()).fold(0.0, f64::max);
		assert!(worst < 0.02, "the interpolated gain is off by {} dB", worst);
	}

	#[test]
	fn cos_table_puts_resonators_where_they_belong() {
		// the frequency the table's cosine stands for, against the asked one, up to the highest a formant may reach
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
		// the bandwidth the table's radius stands for, against the asked one, across all a resonator can be given
		let worst = (0..=6000).map(|bw| {
			let got = -(pole_radius(bw as f32) as f64).ln() * m_sr as f64 / core::f64::consts::PI;
			(got - bw as f64).abs()
		}).fold(0.0, f64::max);
		assert!(worst < 0.01, "a resonator's bandwidth is {} Hz off", worst);
	}
}
