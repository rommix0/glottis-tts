/*

###################################
#  STEPS FOR RUNNING PITCH MODEL  #
###################################

1) reset the model to start

2) start at baseline
	- or what Dennis Klatt calls the brim of the hat

3a) when first stressed vowel is hit, do an initial H* accent pitch rise
	- HAT_RISE per Klatt

3b) fall to L* after the rise when stress pulse counter goes to 0.

4) if content word comes after a content word, multiply stress factor by 0.45
	- adheres to compound stress rule

5) if function word comes after a content word, do a downstep.
	- this adheres to the downstepping phenomenon described by Pierrehumbert (1980)
	- also adheres to phonological boundary setting based on this paper:
		"Prosodic Sentence Analysis Without Exhaustive Parsing"
		Hugo Quene, Rene Kager (1993)

		_  -->	+PhiBound  /  <content_word> ___ <function_word>

			example sentences taken from the paper:
				she prunes # the red roses # in her garden.
				he came # by car.
				his car # was blue.

	k = 0.6 (chosen by Pierrehumbert, but we'll be using a different one here)

	y = x * k^n

	(0.20 < k^n < 1.00)

	where:
		y is result
		x is value to downstep
		k is downstep ratio
		n is downstep counter (start at 0, resets when terminal punctuations . ! ? are hit)

6) if token is a comma, reverse a downstep (decrement the downstep counter).

7) when phrase-final, do the Klatt-style terminal falls (or rises if a question)

8) reset the model when terminal punctuation is hit. could just leave out since Step 1 is reset.

*/

// Bartman-Klatt-Pierrehumbert (BKP) Hat-Style Pitch Model
pub struct model {
	baseline: u16,			// baseline pitch
	hatrise: u16,			// initial hat rise pitch
	prange: u16,			// pitch range
	alpha: f32,				// factor for smoothing
	Hs: u16,				// H*
	Ls: u16,				// L*
	k: f32,					// downstep ratio
	kC: u8,					// downstep counter
	stress_factor: f32,		// stress factor (for compound stress)
	stress_ps: u16,			// stress pulse length
	pulse: f32,				// pulse (start at baseline)
	phrase: u8,				// phrase mode
	count: u16,				// stress pulse timer
	yp1: f32,				// past Y sample for filter. init with baseline
}
impl model {
	pub fn new() -> model {
		let bln:  u16 = 60;
		let prng: u16 = 95;
		let hss:  u16 = (prng as f32 * 0.85) as u16;
		let lss:  u16 = (prng as f32 * 0.15) as u16;
		model {
			baseline : bln,
			hatrise : 100,
			prange : prng,
			alpha : 0.9993,
			Hs : hss,
			Ls : lss,
			k : 0.83,
			kC : 0,
			stress_factor : 1.0,
			stress_ps : 725,
			pulse : bln as f32,
			phrase : 0,
			count : 0,
			yp1 : bln as f32,
		}
	}
	pub fn reset(&mut self) {
		self.stress_factor = 1.0;
		self.pulse         = self.baseline as f32;
		self.yp1           = self.baseline as f32;
		self.phrase        = 0;
		self.count         = 0;
		self.kC            = 0;
	}
	pub fn set(&mut self, stress: u8) {
		if stress != 0 {
			self.count = self.stress_ps;
			if self.phrase == 0 { self.phrase = 1; }
		}
	}
	pub fn set_final(&mut self, ftype: u8) {
		match ftype {
			1 => { self.phrase = 3 },		// exclamation point emphasis
			2 => { self.phrase = 2 },		// question mark final rise
			_ => { self.phrase = 0 },		// default hat fall
		}
	}
	pub fn set_compound(&mut self, x: bool) {
		self.stress_factor = 1.0;
		if x { self.stress_factor = 0.45; }
	}
	pub fn downstep(&mut self) {
		let cur_ratio: f32 = self.k.powf(self.kC as f32);
		if self.kC < 255 && cur_ratio > 0.2 { self.kC += 1; }
	}
	pub fn upstep(&mut self) {
		if self.kC > 0 { self.kC -= 1; }
	}
	pub fn step(&mut self) -> f32 {
		// hat pulse movement generation
		let mut factor: f32 = 1.0;
		let mut movement: f32 = 0.0;
		if self.count > 0 {
			if self.phrase == 3 {
				factor *= 1.25;
			} else {
				movement += self.Hs as f32;
			}
			self.count -= 1;
		} else if self.phrase == 2 {
			factor *= 1.15;
		} else if self.phrase == 3 {
			self.phrase = 0;
		}
		movement *= self.stress_factor;

		// construct pulse
		self.pulse = if self.phrase == 1 { self.hatrise as f32 } else { self.baseline as f32 };
		if self.phrase == 2 {
			self.pulse += self.Hs as f32;
		} else if self.phrase == 0 {
			self.pulse = self.baseline as f32;
		} else {
			let dn: f32 = self.k.powf(self.kC as f32);
			self.pulse += self.Ls as f32 * dn;
			self.pulse += movement * factor * dn;
		}
		
		// smooth the pulse
		let y: f32 = (1.0 - self.alpha) * self.pulse + self.alpha * self.yp1;
		self.yp1 = y;
		y
	}
}
