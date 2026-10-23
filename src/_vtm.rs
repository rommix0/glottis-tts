/*

#########################################
# VOCAL TRACT MODEL FOR TTS SYNTHESIZER #
#########################################

  author: anthony c. bartman (@rommix0)
  date:   4/6/2026 - 8/31/2026

*/
use core::f32::consts::PI;

// master sample rate
pub const m_sr: u32 = 22050;

// soft clip default
const CLIP_LIMIT: f32 = 0.85;

// lip radiation coefficient, tuned at LIP_TUNED_SR; a per-sample filter tilts brighter at a lower rate, and
// LIP_MATCH is the share of that extra tilt taken back (0: none, 1: the tuned response)
const LIP_D: f32 = 0.5;
const LIP_TUNED_SR: f32 = 32000.0;
const LIP_MATCH: f32 = 0.5;

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

// low shelf on the cascade branch: this much more level (dB) below about LOW_SHELF_HZ
const LOW_SHELF_DB: f32 = 4.0;
const LOW_SHELF_HZ: f32 = 250.0;

// how much voicing modulates frication (1 is half-level noise in every other half period)
const FRIC_MOD: f32 = 0.3;

// makeup gain for the parallel branch (frication)
const PARR_GAIN: f32 = 2.5;

// formant parameter inits
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
pub const FN_INIT: f32 = 250.0;
const BN_INIT: f32 = 100.0;
// bandwidth of the nasal pole when it is a nasal consonant's murmur
const BN_MURMUR: f32 = 200.0;

// nasal zero for a nasal coupling k (0 oral, 1 nasal consonant) and the current F1: on the nasal pole for an oral
// sound (they cancel), on F1 for a nasal (the pole is then its murmur), and in between at a vowel's nasalized edge
pub fn nasal_zero(f1: f32, k: f32) -> f32 {
	let k = k.clamp(0.0, 1.0);
	FN_INIT + k * (f1 - FN_INIT)
}

// noise generator
struct noise { alpha: f32, nxt: u32, xp1: f32 }
impl noise {
	pub fn new() -> noise {
		// the aspiration source's smoothing was tuned as 0.75 per sample at 32 kHz; keep that corner (~1.5 kHz)
		noise { alpha: 0.75_f32.powf(32000.0 / m_sr as f32), nxt: 0x2545F491, xp1: 0.0 }
	}
	pub fn reset(&mut self) {
		self.nxt  = 0x2545F491;
		self.xp1  = 0.0;
	}
	fn xorshift(&mut self) {
		// 32-bit state: repeats after 2^32 - 1 samples instead of 3 s
		self.nxt ^= self.nxt << 13;
		self.nxt ^= self.nxt >> 17;
		self.nxt ^= self.nxt << 5;
	}
	pub fn step(&mut self) -> (f32, f32) {
		// generate both white and fricative noise
		self.xorshift();
		let n: f32 = ((self.nxt >> 16) as u16 as i16) as f32 / 32767.0;
		let mut f: f32 = (1.0 - self.alpha) * n + self.alpha * self.xp1;
		self.xp1 = f;
		(n, f)
	}
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
	anorm: f32,
}
impl resonator {
	pub fn new(bf: f32, bw: f32, anti: bool) -> resonator {
		resonator {
			a:     0.0,
			b:     0.0,
			c:     0.0,
			xp1:   0.0,
			xp2:   0.0,
			anti:  anti,
			bf:    bf,
			bw:    bw,
			pfreq: 0.0,
			anorm: 0.0,
		}
	}
	pub fn init(&mut self) { self.set_coeffs(self.bf, self.bw); }
	pub fn reset(&mut self) {
		self.xp1 = 0.0;
		self.xp2 = 0.0;
	}
	fn set_coeffs(&mut self, bf: f32, bw: f32) {
		// calculate the main coefficients here
		let r: f32 = (-PI * bw / m_sr as f32).exp();
		self.c = -(r * r);
		self.b = 2.0 * r * (2.0 * PI * bf / m_sr as f32).cos();
		self.a = 1.0 - self.b - self.c;

		// anti-resonator switch for special
		// resonators like nasal pairs
		if self.anti {
			if self.a == 0.0 {
				self.a = 0.0;
				self.b = 0.0;
				self.c = 0.0;
			}
			self.a = 1.0 / self.a;
			self.b = -self.a * self.b;
			self.c = -self.a * self.c;
		}

		// Fujisaki-style gain compensation (KLSYN88-Qlatt)
		// replaces old DC gain equation as this works much better
		if self.pfreq != 0.0 && bf < self.pfreq {
			self.anorm = bf / self.anorm;
			self.xp1 *= self.anorm;
			self.xp2 *= self.anorm;
		}
		self.pfreq = bf;
		self.anorm = bf;

		// set internal bf and bw to new values
		self.bf = bf;
		self.bw = bw;
	}
	pub fn step(&mut self, x: f32, bf: f32, bw: f32) -> f32 {
		// filter input x with resonator
		let mut bff: f32 = bf;
		let mut bww: f32 = bw;
		if bff == 0.0 { bff = self.bf; }
		if bww == 0.0 { bww = self.bw; }
		if bff > 0.0 || bww > 0.0 { self.set_coeffs(bff, bww); }
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
	pub fn new() -> Biquad {
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
	pub fn reset(&mut self) {
		self.x1 = 0.0;
		self.x2 = 0.0;
		self.y1 = 0.0;
		self.y2 = 0.0;
	}
	pub fn set(&mut self, a: f32, b: f32, c: f32, d: f32, e: f32) {
		self.b0 = a;
		self.b1 = b;
		self.b2 = c;
		self.a1 = d;
		self.a2 = e;
	}
	pub fn step(&mut self, x: f32) -> f32 {
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
	pub fn new() -> LpPole { LpPole { a: 0.0, y1: 0.0, } }
	pub fn reset(&mut self) { self.y1 = 0.0; }
	pub fn set(&mut self, a: f32) { self.a = a; }
	pub fn step(&mut self, x: f32) -> f32 {
		let y = (1.0 - self.a) * x + self.a * self.y1;
		self.y1 = y;
		y
	}
}

// a glottal pulse is spread over this many samples either side of its exact time, so the voice plays this many
// samples late
const PULSE_HALF: usize = 4;
const PULSE_RING: usize = 16;

struct LfSource {
	pos_in_period: f32,
	last_inc: f32,				// period fraction the last sample advanced by
	exc: [f32; PULSE_RING],		// excitation still to play, by absolute sample index
	n: usize,					// absolute sample index
	glottal: Biquad,
	tilt: LpPole,
	rng_state: u32,
	counter: u32,
	ra: f32,
}
impl LfSource {
	pub fn new() -> LfSource {
		LfSource {
			pos_in_period: 0.0,
			last_inc: 0.0,
			exc: [0.0; PULSE_RING],
			n: 0,
			glottal: Biquad::new(),
			tilt: LpPole::new(),
			rng_state: 0x12345678,
			counter: 0,
			ra: 0.0,
		}
	}
	pub fn reset(&mut self) {
		self.glottal.reset();
		self.tilt.reset();
		self.exc = [0.0; PULSE_RING];
	}
	fn add_pulse(&mut self, ago: f32) {
		// a band-limited unit impulse at (n - ago), written PULSE_HALF samples ahead of what plays now
		let h = PULSE_HALF as f32;
		let t = (self.n + PULSE_HALF) as f32 - ago;
		let first = t.floor() as usize + 1 - PULSE_HALF;
		let mut w = [0.0f32; 2 * PULSE_HALF];
		for (k, wk) in w.iter_mut().enumerate() {
			let x = (first + k) as f32 - t;
			let sinc = if x.abs() < 1e-6 { 1.0 } else { (PI * x).sin() / (PI * x) };
			*wk = sinc * (0.5 + 0.5 * (PI * x / h).cos());
		}
		let sum: f32 = w.iter().sum();
		for (k, wk) in w.iter().enumerate() {
			self.exc[(first + k) % PULSE_RING] += wk / sum;
		}
	}
	fn xorshift(&mut self) -> f32 {
		self.rng_state ^= self.rng_state << 13;
		self.rng_state ^= self.rng_state >> 17;
		self.rng_state ^= self.rng_state << 5;
		if self.rng_state & 0x1 == 0 { return 1.0; }
		-1.0
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
		let bg = 1.0 / (oq  * t0 * (PI * (1.0 - alpha_m)).tan());

		// calculate coefficients for glottal biquad filter
		let a1 = -2.0 * (-PI * bg / m_sr as f32).exp() * (2.0 * PI * fg / m_sr as f32).cos();
		let a2 = (-2.0 * PI * bg / m_sr as f32).exp();
		let ag = 1.0 / (PI * (1.0 - alpha_m)).sin();
		let b0 = 0.0;
		let b1 = -ag;
		let b2 = ag;
		self.glottal.set(b0, b1, b2, a1, a2);

		// calculate coefficients for tilt filter
		let fa = 1.0 / (2.0 * PI * self.ra * t0);
		self.tilt.set((-2.0 * PI * fa / m_sr as f32).exp());
	}
	fn calc_flutter(&mut self, f0: f32, flutter: f32) -> f32 {
		// KLGLOTT88 pitch flutter algorithm
		if flutter <= 0.0 { return 0.0; }
		let w = 2.0 * PI * (self.counter as f32 / m_sr as f32);
		let a = (12.7 * w).sin() + (7.1 * w).sin() + (4.7 * w).sin();
		// 10 s holds a whole number of cycles of all three, so the wrap is seamless
		self.counter = (self.counter + 1) % (10 * m_sr);
		a * (flutter / 50.0) * (f0 / 100.0)
	}
	pub fn step(&mut self, f0: f32, rd: f32, flutter: f32, jitter: f32, di: f32) -> (f32, f32, f32) {
		// calculate variations in F0
		let f0_with_flutter = f0 + self.calc_flutter(f0, flutter);
		let mut f0_final = f0_with_flutter;
		if jitter > 0.0 {
			let b = jitter / 100.0 * 4.5;
			f0_final += b * self.xorshift() * f0_with_flutter * (f0_with_flutter / m_sr as f32).sqrt();
		}

		// determine noise modulation amount
		let nmod = if self.pos_in_period >= 0.5 { 0.5 } else { 1.0 };

		// start a glottal pulse at the exact time the period ended, somewhere since the last sample
		if self.pos_in_period == 0.0 || self.pos_in_period >= 1.0 {
			self.calc_fant(f0, rd);
			let ago = if self.last_inc > 0.0 { ((self.pos_in_period - 1.0) / self.last_inc).clamp(0.0, 0.999) } else { 0.0 };
			self.pos_in_period %= 1.0;
			self.add_pulse(ago);
		}
		let slot = self.n % PULSE_RING;
		let impulse = self.exc[slot];
		self.exc[slot] = 0.0;
		self.n += 1;

		// construct glottal pulse
		// and add diplophonic pulsing if need be
		let mut sample = self.tilt.step(self.glottal.step(impulse));
		if di > 0.0 && self.counter % 2 == 1 {
			sample *= 1.0 - di / 100.0;
		}

		// advance position and return sample
		self.last_inc = f0_final / m_sr as f32;
		self.pos_in_period += self.last_inc;
		(sample, nmod, self.ra)
	}
}

// vocal tract model - the main event
pub struct vtm {
	G_MAX:    f32,
	pub F0:   f32,		// this can be accessed by _synth.rs
	pub AV:   f32,		// this can be accessed by _synth.rs
	pub RD:   f32,		// this can be accessed by _synth.rs
	pub AH:   f32,		// this can be accessed by _synth.rs
	pub AF:   f32,		// this can be accessed by _synth.rs
	pub BR:   f32,		// this can be accessed by _synth.rs
	pub F1:   f32,		// this can be accessed by _synth.rs
	pub B1:   f32,		// this can be accessed by _synth.rs
	pub F2:   f32,		// this can be accessed by _synth.rs
	pub B2:   f32,		// this can be accessed by _synth.rs
	pub F3:   f32,		// this can be accessed by _synth.rs
	pub B3:   f32,		// this can be accessed by _synth.rs
	pub NC:   f32,		// nasal coupling, 0 (oral) to 1 (nasal consonant); set by _synth.rs
	FNZ:      f32,
	pub A2:   f32,		// this can be accessed by _synth.rs
	pub A3:   f32,		// this can be accessed by _synth.rs
	pub A4:   f32,		// this can be accessed by _synth.rs
	pub A5:   f32,		// this can be accessed by _synth.rs
	pub A6:   f32,		// this can be accessed by _synth.rs
	pub A7:   f32,		// this can be accessed by _synth.rs
	pub A8:   f32,		// this can be accessed by _synth.rs
	pub A9:   f32,		// this can be accessed by _synth.rs
	pub A10:  f32,		// this can be accessed by _synth.rs
	pub AB:   f32,		// this can be accessed by _synth.rs
	RA:       f32,
	MIN_D:    f32,
	MUL_B:    f32,
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
}
impl vtm {
	pub fn new() -> vtm {
		vtm {
			/*##############################
			# vocal tract model parameters #
			##############################*/
			G_MAX  : 87.0,				// default max gain setting
			F0     : 125.0,				// fundamental frequency (Hz)
			AV     : 60.0,				// Amplitude of voicing, in dB
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
			FNZ    : FN_INIT,			// Frequency of nasal zero, in Hz
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
			RA     : 0.0,
			MIN_D  : 350.0,				// min distance between formants
			MUL_B  : 2.5,				// bandwidth multiplier adjust
			B1_ADJ : 1.0,				// B1 adjustment factor
			B2_ADJ : 1.0,				// B2 adjustment factor
			B3_ADJ : 1.0,				// B3 adjustment factor

			/*##############################
			# sound generators and filters #
			##############################*/
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
		}
	}
	pub fn init(&mut self) {
		// resonator initialization (absolutely needed for them to work correctly)
		self.VR1.init();
		self.VR2.init();
		self.VR3.init();
		self.VR4.init();
		self.VR5.init();
		self.VR6.init();
		self.VR7.init();
		self.VR8.init();
		self.NPR.init();
		self.NZR.init();
		self.FR2.init();
		self.FR3.init();
		self.FR4.init();
		self.FR5.init();
		self.FR6.init();
		self.FR7.init();
		self.FR8.init();
		self.FR9.init();
		self.FR10.init();
	}
	pub fn reset(&mut self) {
		// reset the VTM
		// prevents the weird DC offset bug
		// when past samples have not been cleared
		self.noisegen.reset();
		self.glottal.reset();
		self.VR1.reset();
		self.VR2.reset();
		self.VR3.reset();
		self.VR4.reset();
		self.VR5.reset();
		self.VR6.reset();
		self.VR7.reset();
		self.VR8.reset();
		self.NPR.reset();
		self.NZR.reset();
		self.FR2.reset();
		self.FR3.reset();
		self.FR4.reset();
		self.FR5.reset();
		self.FR6.reset();
		self.FR7.reset();
		self.FR8.reset();
		self.FR9.reset();
		self.FR10.reset();
		self.xp1 = 0.0;
		self.ep1 = 0.0;
		self.shelf_y1 = 0.0;
		self.fric_y1 = 0.0;
	}
	fn db2lin(&mut self, gain: f32) -> f32 {
		if gain <= 0.0 { return 0.0; }
		if gain >= self.G_MAX { return 1.0; }
		0.5_f32.powf((self.G_MAX.floor() - gain) / 6.0)
	}
	fn fix_bw(&mut self) {
		// adjust bandwidths to improve sound quality

		// F0-proximity bandwidth adjust
		self.B1 += 200.0 * (self.F0 / self.F1 - 0.5).powf(2.0);

		// glottal leakage adjust (B1, B2)
		let DB1 = 250.0 * (self.F1 / 500.0).powf(2.0) * (self.RA * 100.0 - RA_REF) / 12.0;
		let DB2 = DB1 * self.F1 / (2.0 * self.F2);
		self.B1 += DB1;
		self.B2 += DB2;

		// B1 adjust
		self.B1_ADJ = (self.F1 - self.F2).abs();
		if self.B1_ADJ < self.MIN_D {
			self.B1_ADJ = self.MUL_B * ((self.MIN_D - self.B1_ADJ) / self.MIN_D);
		} else {
			self.B1_ADJ = 0.0;
		}
		self.B1_ADJ += 1.0;

		// B2 adjust
		self.B2_ADJ = (self.F2 - self.F3).abs();
		if self.B2_ADJ < self.MIN_D {
			self.B2_ADJ = self.MUL_B * ((self.MIN_D - self.B2_ADJ) / self.MIN_D);
		} else {
			self.B2_ADJ = 0.0;
		}
		self.B2_ADJ += 1.0;

		// B3 adjust
		self.B3_ADJ = (self.F3 - F4_INIT).abs();
		if self.B3_ADJ < self.MIN_D {
			self.B3_ADJ = self.MUL_B * ((self.MIN_D - self.B3_ADJ) / self.MIN_D);
		} else {
			self.B3_ADJ = 0.0;
		}
		self.B3_ADJ += 1.0;
	}
	fn parallel(&mut self, x: f32) -> f32 {
		/*#########################
		# PARALLEL FILTER NETWORK #
		#########################*/
		let mut yy: f32 = 0.0;
		let mut bb2 = self.B2 * self.B2_ADJ;
		let mut bb3 = self.B3 * self.B3_ADJ;
		if bb2 < 40.0 { bb2 = 40.0; }
		if bb3 < 40.0 { bb3 = 40.0; }

		yy -= self.FR2.step(x, self.F2, bb2)  * self.db2lin(self.A2);
		yy += self.FR3.step(x, self.F3, bb3)  * self.db2lin(self.A3);
		yy -= self.FR4.step(x, 0.0, 0.0)      * self.db2lin(self.A4);
		yy += self.FR5.step(x, 0.0, 0.0)      * self.db2lin(self.A5);
		yy -= self.FR6.step(x, 0.0, 0.0)      * self.db2lin(self.A6);
		yy += self.FR7.step(x, 0.0, 0.0)      * self.db2lin(self.A7);
		yy -= self.FR8.step(x, 0.0, 0.0)      * self.db2lin(self.A8);
		yy += self.FR9.step(x, 0.0, 0.0)      * self.db2lin(self.A9);
		yy -= self.FR10.step(x, 0.0, 0.0)     * self.db2lin(self.A10);
		yy += self.db2lin(self.AB) * x;
		yy
	}
	fn cascade(&mut self, x: f32) -> f32 {
		/*########################
		# CASCADE FILTER NETWORK #
		########################*/
		let mut yy: f32 = 0.0;

		// adjust first three bandwidths
		let mut bb1 = self.B1 * self.B1_ADJ;
		let mut bb2 = self.B2 * self.B2_ADJ;
		let mut bb3 = self.B3 * self.B3_ADJ;
		if bb1 < CASC_MIN_BW { bb1 = CASC_MIN_BW; }
		if bb2 < CASC_MIN_BW { bb2 = CASC_MIN_BW; }
		if bb3 < 40.0 { bb3 = 40.0; }

		// nasal coupling (Hawkins & Stevens 1985)
		let k = self.NC.clamp(0.0, 1.0);
		self.FNZ = nasal_zero(self.F1, k);
		let bnz = BN_INIT + k * (bb1 - BN_INIT);
		let bnp = BN_INIT + k * (BN_MURMUR - BN_INIT);

		yy = self.NPR.step(self.NZR.step(x, self.FNZ, bnz), FN_INIT, bnp);	// NASAL TRACT
		yy = self.VR1.step(yy, self.F1, bb1);								// F1 - tongue height
		yy = self.VR2.step(yy, self.F2, bb2);								// F2 - tongue fronting
		yy = self.VR3.step(yy, self.F3, bb3);								// F3 - R-coloring
		yy = self.VR4.step(yy, 0.0, 0.0);
		yy = self.VR5.step(yy, 0.0, 0.0);
		yy = self.VR6.step(yy, 0.0, 0.0);
		yy = self.VR7.step(yy, 0.0, 0.0);
		yy = self.VR8.step(yy, 0.0, 0.0);
		yy
	}
	fn softclip(&mut self, x: f32) -> f32 {
		// use soft clipping to prevent overloading the audio
		let a: f32 = if x < 0.0 { -x } else { x };
		if a <= CLIP_LIMIT { return x; }
		let sign: f32 = if x < 0.0 { -1.0 } else { 1.0 };
		let excess: f32 = a - CLIP_LIMIT;
		sign * (CLIP_LIMIT + (1.0 - CLIP_LIMIT) * excess / (excess + 1.0))
	}
	fn low_shelf(&mut self, x: f32) -> f32 {
		// add a low-passed copy back on top, so only the bottom of the spectrum gains level
		let a = (-2.0 * PI * LOW_SHELF_HZ / m_sr as f32).exp();
		self.shelf_y1 = (1.0 - a) * x + a * self.shelf_y1;
		x + (10.0_f32.powf(LOW_SHELF_DB / 20.0) - 1.0) * self.shelf_y1
	}
	fn emph(&mut self, x: f32) -> f32 {
		// brighten the cascade output
		let xx = x - CASC_EMPH * self.ep1;
		self.ep1 = x;
		xx
	}
	fn radiate(&mut self, x: f32) -> f32 {
		// lip radiation with its coefficient moved toward the tuned rate's response, at the same low-frequency gain
		let d = LIP_D.powf((LIP_TUNED_SR / m_sr as f32).powf(LIP_MATCH));
		self.lip(x, d) * (1.0 - LIP_D) / (1.0 - d)
	}
	fn lip(&mut self, x: f32, d: f32) -> f32 {
		// apply lip radiation filter (start with 0.97 for d)
		//let xx = (1.0 - d) * x - d * self.xp1;
		let xx = x - d * self.xp1;
		self.xp1 = x;
		xx
	}
	pub fn step(&mut self) -> f32 {
		/*#################
		# MAIN OPERATIONS #
		#################*/

		// get noise and glottal sound sources
		let mut n:     f32 = 0.0;
		let mut f:     f32 = 0.0;
		let mut voice: f32 = 0.0;
		let mut nmod:  f32 = 0.0;
		(n, f) = self.noisegen.step();
		(voice, nmod, self.RA) = self.glottal.step(self.F0, self.RD, 75.0, 0.0, 0.0);
		voice *= 0.01;
		f *= 0.6;

		// adjust bandwidths if formants are close together
		self.fix_bw();

		// noise modulation
		let af_mod: f32 = if self.AV > 0.0 { 1.0 - FRIC_MOD * (1.0 - nmod) } else { 1.0 };

		// route audio sources to a source fed in by cascade filter
		voice   = voice + (n * self.db2lin(self.BR) * af_mod);
		let casc_in: f32 = voice * self.db2lin(self.AV) + f * self.db2lin(self.AH);

		// route noise source to source fed in by parallel filter
		// frication is broadband noise, rolled off above FRIC_LP_HZ
		let a = (-2.0 * PI * FRIC_LP_HZ / m_sr as f32).exp();
		self.fric_y1 = (1.0 - a) * n + a * self.fric_y1;
		let parr_in: f32 = self.fric_y1 * self.db2lin(self.AF) * af_mod;

		// pass them through the filters
		let casc_out: f32 = self.cascade(casc_in);
		let casc_out: f32 = self.emph(casc_out) * CASC_GAIN;
		let casc_out: f32 = self.low_shelf(casc_out);
		let parr_out: f32 = self.parallel(parr_in) * PARR_GAIN;

		// add them together and return
		let out = self.radiate(casc_out + parr_out) * 10.0_f32.powf(OUT_DB / 20.0);
		self.softclip(out)
	}
}



#[cfg(test)]
mod tests {
	use super::*;

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

	#[test]
	fn cascade_bandwidths_have_a_floor() {
		// a B1 or B2 asked below the floor renders exactly as the floor itself
		let render = |b1: f32, b2: f32| {
			let mut v = vtm::new();
			let mut out = Vec::new();
			for _ in 0..4000 {
				v.F0 = 120.0; v.AV = 60.0; v.RD = 0.6;
				v.F1 = 600.0; v.F2 = 1500.0; v.F3 = 2500.0;
				v.B1 = b1; v.B2 = b2; v.B3 = 150.0;
				out.push(v.step());
			}
			out
		};
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
		let x: Vec<f32> = (0..m_sr as usize / 2 + 8192).map(|_| g.step(120.0, 0.6, 0.0, 0.0, 0.0).0).collect();
		let tail = &x[x.len() - 8192..];
		let h = hnr(tail, 120.0, 2000.0, 4000.0);
		assert!(h > 40.0, "2-4 kHz harmonics only {} dB over the rest", h);
	}

	#[test]
	fn pulse_is_centred_on_its_time() {
		let mut g = LfSource::new();
		g.n = 100;
		g.add_pulse(0.3);
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
	}

	#[test]
	fn noise_does_not_repeat_within_seconds() {
		let mut n = noise::new();
		let x: Vec<f32> = (0..4 * m_sr).map(|_| n.step().0).collect();
		let head = &x[..64];
		assert!(!x[1..].windows(64).any(|w| w == head), "noise repeats within 4 s");
	}

	#[test]
	fn aspiration_noise_corner_is_in_hz() {
		// smoothed as tuned at 32 kHz: corner near 1.47 kHz whatever the rate
		let n = noise::new();
		let fc = -n.alpha.ln() * m_sr as f32 / (2.0 * PI);
		assert!((fc - 1465.0).abs() < 30.0, "aspiration noise corner {} Hz", fc);
	}
}
