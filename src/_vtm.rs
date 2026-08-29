/*

#########################################
# VOCAL TRACT MODEL FOR TTS SYNTHESIZER #
#########################################

  author: anthony c. bartman (@rommix0)
  date:   4/6/2026 - 8/26/2026

*/

pub const m_sr: u16 = 22050;

pub const M_EU: f32 = 2.7182818284590452;
pub const M_PI: f32 = 3.1415927410125732;

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
const FNP_INIT: f32 = 250.0;
const BNP_INIT: f32 = 100.0;
const FNZ_INIT: f32 = 250.0;
const BNZ_INIT: f32 = 100.0;

// noise generators
struct noise {
	alpha: f32,
	//nxt:   i32,
	nxt:   u16,
	xp1:   f32,
}
impl noise {
	pub fn new() -> noise {
		noise {
			alpha: 0.75,
			nxt:   5612,
			xp1:   0.0,
		}
	}
	pub fn reset(&mut self) {
		self.nxt = 5612;
		self.xp1 = 0.0;
	}
	fn xorshift(x: u16) -> u16 {
		let mut xx = x;
		xx ^= xx << 7;
		xx ^= xx >> 9;
		xx ^= xx << 8;
		xx
	}
	pub fn step(&mut self) -> (f32, f32) {
		self.nxt = noise::xorshift(self.nxt);
		let n: f32 = (self.nxt as f32 / 32767.0) - 1.0;
		let f: f32 = (1.0 - self.alpha) * n + self.alpha * self.xp1;
		self.xp1 = f;
		(n, f)
	}
}

// klatt resonator
struct resonator {
	a:    f32,
	b:    f32,
	c:    f32,
	xp1:  f32,
	xp2:  f32,
	anti: bool,
	bf:   f32,
	bw:   f32,
}
impl resonator {
	pub fn new(bf: f32, bw: f32, anti: bool) -> resonator {
		resonator {
			a:    0.0,
			b:    0.0,
			c:    0.0,
			xp1:  0.0,
			xp2:  0.0,
			anti: anti,
			bf:   bf,
			bw:   bw,
		}
	}
	pub fn init(&mut self) { self.set_coeffs(self.bf, self.bw); }
	pub fn reset(&mut self) {
		self.xp1 = 0.0;
		self.xp2 = 0.0;
	}
	fn set_coeffs(&mut self, bf: f32, bw: f32) {
		// calculate the main coefficients here
		let r: f32 = M_EU.powf(-M_PI * bw / m_sr as f32);
		self.c = -(r * r);
		let w: f32 = 2.0 * M_PI * bf / m_sr as f32;
		self.b = 2.0 * r * w.cos();
		self.a = 1.0 - self.b - self.c;

		// anti-resonator switch for special
		// resonators like nasal pairs
		if self.anti {
			if self.a == 0.0 {
				self.a = 0.0;
				self.b = 0.0;
				self.c = 0.0;
			}
			self.a = -self.a;
			self.b /= self.a;
			self.c /= self.a;
		}

		// normalize filter gain so it's not too loud
		self.a *= (1.0 - self.b - self.c) / self.a;
		
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

// structs for both tilter and peaker filters
struct tilter {
	xp1: f32,
	xp2: f32,
	yp1: f32,
	yp2: f32,
	y:   f32,
	b0:  f32,
	b1:  f32,
	b2:  f32,
	a0:  f32,
	a1:  f32,
	a2:  f32,
}
struct peaker {
	xp1: f32,
	xp2: f32,
	yp1: f32,
	yp2: f32,
	y:   f32,
	b0:  f32,
	b1:  f32,
	b2:  f32,
	a0:  f32,
	a1:  f32,
	a2:  f32,
}

// tilter for glottal model
impl tilter {
	pub fn new() -> tilter {
		tilter {
			xp1: 0.0,
			xp2: 0.0,
			yp1: 0.0,
			yp2: 0.0,
			y:   0.0,
			b0:  0.0,
			b1:  0.0,
			b2:  0.0,
			a0:  0.0,
			a1:  0.0,
			a2:  0.0,
		}
	}
	pub fn reset(&mut self) {
		self.xp1 = 0.0;
		self.xp2 = 0.0;
		self.yp1 = 0.0;
		self.yp2 = 0.0;
		self.y   = 0.0;
	}
	fn design(&mut self, Q: f32) {
		let mut QQ: f32 = Q;
		if QQ < 0.0 { QQ = 0.0; }
		let alpha: f32 = 1.0 / (2.0 * QQ + 0.0000001);
		self.b0 = alpha;
		self.a0 = 1.0 / (1.0 + alpha);
		self.a1 = -2.0;
		self.a2 = 1.0 - alpha;
	}
	pub fn filter(&mut self, x: f32, q: f32) -> f32 {
		self.design(q);
		self.yp2 = self.yp1;
		self.yp1 = self.y;
		self.y   = self.a0 * (self.b0 * x - self.b0 * self.xp2 - self.a1 * self.yp1 - self.a2 * self.yp2);
		self.xp2 = self.xp1;
		self.xp1 = x;
		self.y
	}
}

// peaker for glottal model
impl peaker {
	pub fn new() -> peaker {
		peaker {
			xp1: 0.0,
			xp2: 0.0,
			yp1: 0.0,
			yp2: 0.0,
			y:   0.0,
			b0:  0.0,
			b1:  0.0,
			b2:  0.0,
			a0:  0.0,
			a1:  0.0,
			a2:  0.0,
		}
	}
	pub fn reset(&mut self) {
		self.xp1 = 0.0;
		self.xp2 = 0.0;
		self.yp1 = 0.0;
		self.yp2 = 0.0;
		self.y   = 0.0;
	}
	fn design(&mut self, f: f32, g: f32) {
		let Wc: f32 = 2.0 * M_PI * f / m_sr as f32;
		let alpha: f32 = 0.5 * Wc.sin();
		let A: f32 = (10.0_f32.powf(g / 20.0)).sqrt();
		let dd: f32 = alpha * A;
		let ee: f32 = alpha / A;
		let ff: f32 = -2.0 * Wc.cos();
		self.b0 = 1.0 + dd;
		self.b1 = ff;
		self.b2 = 1.0 - dd;
		self.a0 = 1.0 / (1.0 + ee);
		self.a2 = 1.0 - ee;
	}
	pub fn filter(&mut self, x: f32, f: f32, g: f32) -> f32 {
		self.design(f, g);
		self.yp2 = self.yp1;
		self.yp1 = self.y;
		self.y   = self.a0 * (self.b0 * x + self.b1 * self.xp1 + self.b2 * self.xp2 - self.b1 * self.yp1 - self.a2 * self.yp2);
		self.xp2 = self.xp1;
		self.xp1 = x;
		self.y
	}
}

// glottal pulse model itself
struct glottal {
	count: f32,
	tilt: tilter,
	peak: peaker,
	glot_tbl: [f32; 4],
	x: f32,
	c: usize,
}
impl glottal {
	pub fn new() -> glottal {
		glottal {
			count: 0.0,
			tilt: tilter::new(),
			peak: peaker::new(),
			glot_tbl: [0.0, 0.65, 1.0, 0.0],
			x: 0.0,
			c: 0,
		}
	}
	pub fn reset(&mut self) {
		self.tilt.reset();
		self.peak.reset();
	}
	pub fn step(&mut self, freq: f32, peak_db: f32, tilt_q: f32) -> (f32 ,f32) {
		let per: f32 = m_sr as f32 / freq;
		let hlf: f32 = per / 2.0;
		
		// create pulse
		if self.c < 4 {
			self.x = self.glot_tbl[self.c];
			self.c += 1;
		}
		
		// noise modulation
		let mut nmod: f32 = 1.0;
		if self.count >= hlf { nmod = 0.5; }
		
		// reset counter if glottal length has been past
		if self.count == 0.0 || self.count > per {
			self.count = 0.0;
			self.c = 0;
		}
		
		// increment counter
		self.count += 1.0;
		
		// return the filtered glottal pulse
		let f1: f32 = self.tilt.filter(self.x, tilt_q);
		let f2: f32 = self.peak.filter(f1, freq, peak_db);
		(f2, nmod)
	}
}

// vocal tract model - the main event
pub struct vtm {
        G_MAX:    f32,
        pub F0:   f32,		// this can be accessed by _synth.rs
        pub AV:   f32,		// this can be accessed by _synth.rs
        pub TL:   f32,		// this can be accessed by _synth.rs
        pub PK:   f32,		// this can be accessed by _synth.rs
        pub AH:   f32,		// this can be accessed by _synth.rs
        pub AF:   f32,		// this can be accessed by _synth.rs
        pub BR:   f32,		// this can be accessed by _synth.rs
        pub F1:   f32,		// this can be accessed by _synth.rs
        pub B1:   f32,		// this can be accessed by _synth.rs
        pub F2:   f32,		// this can be accessed by _synth.rs
        pub B2:   f32,		// this can be accessed by _synth.rs
        pub F3:   f32,		// this can be accessed by _synth.rs
        pub B3:   f32,		// this can be accessed by _synth.rs
        pub F4:   f32,		// this can be accessed by _synth.rs
        pub B4:   f32,		// this can be accessed by _synth.rs
        pub F5:   f32,		// this can be accessed by _synth.rs
        pub B5:   f32,		// this can be accessed by _synth.rs
        pub F6:   f32,		// this can be accessed by _synth.rs
        pub B6:   f32,		// this can be accessed by _synth.rs
        pub F7:   f32,		// this can be accessed by _synth.rs
        pub B7:   f32,		// this can be accessed by _synth.rs
        pub FNP:  f32,		// this can be accessed by _synth.rs
        pub BNP:  f32,		// this can be accessed by _synth.rs
        pub FNZ:  f32,		// this can be accessed by _synth.rs
        pub BNZ:  f32,		// this can be accessed by _synth.rs
        pub A2:   f32,		// this can be accessed by _synth.rs
        pub A3:   f32,		// this can be accessed by _synth.rs
        pub A4:   f32,		// this can be accessed by _synth.rs
        pub A5:   f32,		// this can be accessed by _synth.rs
        pub A6:   f32,		// this can be accessed by _synth.rs
        pub A7:   f32,		// this can be accessed by _synth.rs
        pub AB:   f32,		// this can be accessed by _synth.rs
        MIN_D:    f32,
        MUL_B:    f32,
        B1_ADJ:   f32,
        B2_ADJ:   f32,
        B3_ADJ:   f32,
        noisegen: noise,
        glottal:  glottal,
        VR1:      resonator,
        VR2:      resonator,
        VR3:      resonator,
        VR4:      resonator,
        VR5:      resonator,
        VR6:      resonator,
        NPR:      resonator,
        NZR:      resonator,
        FR2:      resonator,
        FR3:      resonator,
        FR4:      resonator,
        FR5:      resonator,
        FR6:      resonator,
        FR7:      resonator,
}
impl vtm {
	pub fn new() -> vtm {
		vtm {
			/*##############################
			# vocal tract model parameters #
			##############################*/
			G_MAX  : 87.0,			// default max gain setting
			F0     : 125.0,			// fundamental frequency (Hz)
			AV     : 60.0,			// Amplitude of voicing, in dB
			TL     : 0.0,			// tilt (for glottal pulse model)
			PK     : 0.0,			// peak (for glottal pulse model)
			AH     : 0.0,			// Amplitude of aspiration, in dB
			AF     : 0.0,			// Amplitude of frication, in dB
			BR     : 50.0,			// breathiness amp (dB)
			F1     : F1_INIT,		// Frequency of 1st formant, in Hz
			B1     : B1_INIT,		// Bandwidth of 1st formant, in Hz
			F2     : F2_INIT,		// Frequency of 2nd formant, in Hz
			B2     : B2_INIT,		// Bandwidth of 2nd formant, in Hz
			F3     : F3_INIT,		// Frequency of 3rd formant, in Hz
			B3     : B3_INIT,		// Bandwidth of 3rd formant, in Hz
			F4     : F4_INIT,		// Frequency of 4th formant, in Hz
			B4     : B4_INIT,		// Bandwidth of 4th formant, in Hz
			F5     : F5_INIT,		// Frequency of 5th formant, in Hz
			B5     : B5_INIT,		// Bandwidth of 5th formant, in Hz
			F6     : F6_INIT,		// Frequency of 6th formant, in Hz
			B6     : B6_INIT,		// Bandwidth of 6th formant, in Hz
			F7     : F7_INIT,		// Frequency of 7th formant, in Hz
			B7     : B7_INIT,		// Bandwidth of 7th formant, in Hz
			FNP    : FNP_INIT,		// Frequency of nasal pole, in Hz
			BNP    : BNP_INIT,		// Bandwidth of nasal pole, in Hz
			FNZ    : FNZ_INIT,		// Frequency of nasal zero, in Hz
			BNZ    : BNZ_INIT,		// Bandwidth of nasal zero, in Hz
			A2     : 0.0,			// Amp of parallel 2nd formant, in dB
			A3     : 0.0,			// Amp of parallel 3rd formant, in dB
			A4     : 0.0,			// Amp of parallel 4th formant, in dB
			A5     : 0.0,			// Amp of parallel 5th formant, in dB
			A6     : 0.0,			// Amp of parallel 6th formant, in dB
			A7     : 0.0,			// Amp of parallel 7th formant, in dB
			AB     : 0.0,			// Amp of parallel bypass path, in dB
			MIN_D  : 300.0,			// min distance between formants
			MUL_B  : 2.5,			// bandwidth multiplier adjust
			B1_ADJ : 1.0,			// B1 adjustment factor
			B2_ADJ : 1.0,			// B2 adjustment factor
			B3_ADJ : 1.0,			// B3 adjustment factor

			/*##############################
			# sound generators and filters #
			##############################*/
			noisegen : noise::new(),
			glottal  : glottal::new(),
			VR1      : resonator::new(F1_INIT,  B1_INIT,  false),
			VR2      : resonator::new(F2_INIT,  B2_INIT,  false),
			VR3      : resonator::new(F3_INIT,  B3_INIT,  false),
			VR4      : resonator::new(F4_INIT,  B4_INIT,  false),
			VR5      : resonator::new(F5_INIT,  B5_INIT,  false),
			VR6      : resonator::new(F6_INIT,  B6_INIT,  false),
			NPR      : resonator::new(FNP_INIT, BNP_INIT, false),
			NZR      : resonator::new(FNZ_INIT, BNZ_INIT, true),
			FR2      : resonator::new(F2_INIT,  B2_INIT,  false),
			FR3      : resonator::new(F3_INIT,  B3_INIT,  false),
			FR4      : resonator::new(F4_INIT,  B4_INIT,  false),
			FR5      : resonator::new(F5_INIT,  B5_INIT,  false),
			FR6      : resonator::new(F6_INIT,  B6_INIT,  false),
			FR7      : resonator::new(F7_INIT,  B7_INIT,  false),
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
		self.NPR.init();
		self.NZR.init();
		self.FR2.init();
		self.FR3.init();
		self.FR4.init();
		self.FR5.init();
		self.FR6.init();
		self.FR7.init();
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
		self.NPR.reset();
		self.NZR.reset();
		self.FR2.reset();
		self.FR3.reset();
		self.FR4.reset();
		self.FR5.reset();
		self.FR6.reset();
		self.FR7.reset();
	}
	fn db2lin(&mut self, gain: f32) -> f32 {
		if gain <= 0.0 { return 0.0; }
		if gain >= self.G_MAX { return 1.0; }
		0.5_f32.powf((self.G_MAX.floor() - gain) / 6.0)
	}
	fn max_adj(&mut self) -> f32 {
		let mut result: f32 = self.B3_ADJ;
		if self.B1_ADJ >= self.B2_ADJ && self.B1_ADJ >= self.B3_ADJ {
			result = self.B1_ADJ;
		} else if self.B2_ADJ >= self.B1_ADJ && self.B2_ADJ >= self.B3_ADJ {
			result = self.B2_ADJ;
		}
		result
	}
	fn fix_bw(&mut self) {
		// widen bandwidths if first three formants are extremely close together
		// this improves sound quality by a lot, and makes it less whistly and robotic.
		self.B1_ADJ = (self.F1 - self.F2).abs();
		if self.B1_ADJ < self.MIN_D {
			self.B1_ADJ = self.MUL_B * ((self.MIN_D - self.B1_ADJ) / self.MIN_D);
		} else {
			self.B1_ADJ = 0.0;
		}
		self.B1_ADJ += 1.0;

		self.B2_ADJ = (self.F2 - self.F3).abs();
		if self.B2_ADJ < self.MIN_D {
			self.B2_ADJ = self.MUL_B * ((self.MIN_D - self.B2_ADJ) / self.MIN_D);
		} else {
			self.B2_ADJ = 0.0;
		}
		self.B2_ADJ += 1.0;

		self.B3_ADJ = (self.F3 - self.F4).abs();
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
		yy += self.FR2.step(x, self.F2, self.B2 * self.B2_ADJ) * self.db2lin(self.A2);
		yy -= self.FR3.step(x, self.F3, self.B3 * self.B3_ADJ) * self.db2lin(self.A3);
		yy += self.FR4.step(x, self.F4, self.B4)               * self.db2lin(self.A4);
		yy -= self.FR5.step(x, self.F5, self.B5)               * self.db2lin(self.A5);
		yy += self.FR6.step(x, self.F6, self.B6)               * self.db2lin(self.A6);
		yy -= self.FR7.step(x, self.F7, self.B7)               * self.db2lin(self.A7);
		yy += self.db2lin(self.AB) * x;
		yy
	}
	fn cascade(&mut self, x: f32) -> f32 {
		/*########################
		# CASCADE FILTER NETWORK #
		########################*/
		let mut yy: f32 = 0.0;
		yy += self.NPR.step(self.NZR.step(x, self.FNZ, 0.0), 0.0, 0.0);		// NASAL TRACT
		yy += self.VR1.step(yy, self.F1, self.B1 * self.B1_ADJ);			// F1 - tongue height
		yy += self.VR2.step(yy, self.F2, self.B2 * self.B2_ADJ);			// F2 - tongue fronting
		yy += self.VR3.step(yy, self.F3, self.B3 * self.B3_ADJ);			// F3 - R-coloring
		yy += self.VR4.step(yy, 0.0, 0.0);
		yy += self.VR5.step(yy, 0.0, 0.0);
		yy += self.VR6.step(yy, 0.0, 0.0);
		yy
	}
	pub fn step(&mut self) -> f32 {
		/*#################
		# MAIN OPERATIONS #
		#################*/

		// adjust bandwidths if formants are close together
		self.fix_bw();
		
		// adjust tilt (might improve sound quality)
		self.TL *= self.max_adj();

		// get noise and glottal sound sources
		let mut n:     f32 = 0.0;
		let mut f:     f32 = 0.0;
		let mut voice: f32 = 0.0;
		let mut nmod:  f32 = 0.0;
		(n, f) = self.noisegen.step();
		(voice, nmod) = self.glottal.step(self.F0, self.PK, self.TL);
		f *= 2.00;

		// noise modulation
		let af_mod: f32 = if self.AV > 0.0 { nmod } else { 1.0 };

		// route audio sources to a source fed in by cascade filter
		voice   = voice + (n * self.db2lin(self.BR) * af_mod);
		let casc_in: f32 = voice * self.db2lin(self.AV) + f * self.db2lin(self.AH);

		// route noise source to source fed in by parallel filter
		let parr_in: f32 = f * self.db2lin(self.AF) * af_mod;

		// pass them through the filters
		let casc_out: f32 = self.cascade(casc_in);
		let parr_out: f32 = self.parallel(parr_in);

		// add them together and return
		casc_out + parr_out
	}
}
