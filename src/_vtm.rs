/*

#########################################
# VOCAL TRACT MODEL FOR TTS SYNTHESIZER #
#########################################

  author: anthony c. bartman (@rommix0)
  date:   4/6/2026 - 8/31/2026

*/
use core::f32::consts::PI;

// master sample rate
pub const m_sr: u32 = 32000;

// soft clip default
const CLIP_LIMIT: f32 = 0.85;

// makeup gain for the cascade branch (unity DC gain per resonator)
const CASC_GAIN: f32 = 2328.0;

// high frequency emphasis for the cascade branch
const CASC_EMPH: f32 = 0.75;

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

// noise generator
struct noise { alpha: f32, nxt: u16, xp1: f32 }
impl noise {
	pub fn new() -> noise {
		noise { alpha: 0.75, nxt: 5612, xp1: 0.0 }
	}
	pub fn reset(&mut self) {
		self.nxt  = 5612;
		self.xp1  = 0.0;
	}
	fn xorshift(&mut self) {
		self.nxt ^= self.nxt << 7;
		self.nxt ^= self.nxt >> 9;
		self.nxt ^= self.nxt << 8;
	}
	pub fn step(&mut self) -> (f32, f32) {
		// generate both white and fricative noise
		self.xorshift();
		let n: f32 = (self.nxt as i16) as f32 / 32767.0;
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

struct LfSource {
	pos_in_period: f32,
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
		self.counter = (self.counter + 1) % m_sr;
		1.0 + a * flutter / 50.0
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

		// create impulse if start of glottal pulse
		let mut impulse = 0.0;
		if self.pos_in_period == 0.0 || self.pos_in_period >= 1.0 {
			self.calc_fant(f0, rd);
			self.pos_in_period %= 1.0;
			impulse = 1.0;
		}

		// construct glottal pulse
		// and add diplophonic pulsing if need be
		let mut sample = self.tilt.step(self.glottal.step(impulse));
		if di > 0.0 && self.counter % 2 == 1 {
			sample *= 1.0 - di / 100.0;
		}

		// advance position and return sample
		self.pos_in_period += f0_final / m_sr as f32;
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
	pub FNP:  f32,		// this can be accessed by _synth.rs
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
	VR9:      resonator,
	VR10:     resonator,
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
			FNP    : FN_INIT,			// Frequency of nasal pole, in Hz
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
			VR9      : resonator::new(F9_INIT,  B9_INIT,  false),
			VR10     : resonator::new(F10_INIT, B10_INIT, false),
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
		self.VR9.init();
		self.VR10.init();
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
		self.VR9.reset();
		self.VR10.reset();
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
		if bb1 < 40.0 { bb1 = 40.0; }
		if bb2 < 40.0 { bb2 = 40.0; }
		if bb3 < 40.0 { bb3 = 40.0; }

		// nasal coupling (Hawkins & Stevens 1985)
		if self.FNP < self.F1 {
			self.FNZ = FN_INIT;
		} else {
			self.FNZ = (self.FNP + self.F1) / 2.0;
		}

		yy = self.NPR.step(self.NZR.step(x, self.FNZ, 0.0), 0.0, 0.0);		// NASAL TRACT
		yy = self.VR1.step(yy, self.F1, bb1);								// F1 - tongue height
		yy = self.VR2.step(yy, self.F2, bb2);								// F2 - tongue fronting
		yy = self.VR3.step(yy, self.F3, bb3);								// F3 - R-coloring
		yy = self.VR4.step(yy, 0.0, 0.0);
		yy = self.VR5.step(yy, 0.0, 0.0);
		yy = self.VR6.step(yy, 0.0, 0.0);
		yy = self.VR7.step(yy, 0.0, 0.0);
		yy = self.VR8.step(yy, 0.0, 0.0);
		yy = self.VR9.step(yy, 0.0, 0.0);
		yy = self.VR10.step(yy, 0.0, 0.0);
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
	fn emph(&mut self, x: f32) -> f32 {
		// brighten the cascade output
		let xx = x - CASC_EMPH * self.ep1;
		self.ep1 = x;
		xx
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
		let af_mod: f32 = if self.AV > 0.0 { nmod } else { 1.0 };

		// route audio sources to a source fed in by cascade filter
		voice   = voice + (n * self.db2lin(self.BR) * af_mod);
		let casc_in: f32 = voice * self.db2lin(self.AV) + f * self.db2lin(self.AH);

		// route noise source to source fed in by parallel filter
		let parr_in: f32 = f * self.db2lin(self.AF) * af_mod;

		// pass them through the filters
		let casc_out: f32 = self.cascade(casc_in);
		let casc_out: f32 = self.emph(casc_out) * CASC_GAIN;
		let parr_out: f32 = self.parallel(parr_in);

		// add them together and return
		let out = self.lip(casc_out + parr_out, 0.5);
		self.softclip(out)
	}
}
