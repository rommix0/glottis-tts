// duration CART model
pub struct model {
	mn:        f32,
	sd:        f32,
	DUR_ADD:   u16,
	DUR_SUB:   u16,
	L2_CLASS:  u8,
	L1_CLASS:  u8,
	L1_PHONE:  u8,
	CU_CLASS:  u8,
	CU_STRESS: u8,
	CU_PHONE:  u8,
	CU_DURAT:  f32,
	R1_CLASS:  u8,
	R1_STRESS: u8,
	R1_PHONE:  u8,
	R2_CLASS:  u8,
	R2_STRESS: u8,
	R2_PHONE:  u8,
	LAST_SYLL: bool,
	R1_LSYLL:  bool,
	R2_LSYLL:  bool,
}
impl model {
	pub fn new() -> model {
		model {
			mn:        90.0,
			sd:        75.0,
			DUR_ADD:   0,
			DUR_SUB:   0,
			L2_CLASS:  0,
			L1_CLASS:  0,
			L1_PHONE:  0,
			CU_CLASS:  0,
			CU_STRESS: 0,
			CU_PHONE:  0,
			CU_DURAT:  0.0,
			R1_CLASS:  0,
			R1_STRESS: 0,
			R1_PHONE:  0,
			R2_CLASS:  0,
			R2_STRESS: 0,
			R2_PHONE:  0,
			LAST_SYLL: false,
			R1_LSYLL:  false,
			R2_LSYLL:  false,
		}
	}
	pub fn reset(&mut self) {
		self.L2_CLASS  = 0;
		self.L1_CLASS  = 0;
		self.L1_PHONE  = 0;
		self.CU_CLASS  = 0;
		self.CU_STRESS = 0;
		self.CU_PHONE  = 0;
		self.CU_DURAT  = 0.0;
		self.R1_CLASS  = 0;
		self.R1_STRESS = 0;
		self.R1_PHONE  = 0;
		self.R2_CLASS  = 0;
		self.R2_STRESS = 0;
		self.R2_PHONE  = 0;
		self.LAST_SYLL = false;
		self.R1_LSYLL  = false;
		self.R2_LSYLL  = false;
	}
	fn phon2id(&mut self, x: u8) -> u8 {
		match x {
			50|55 => {
				// // %
				6
			},
			0|51 => {
				// _ ,
				2
			},
			52|53|54 => {
				// . ? !
				5
			},
			31|36|23 => {
				// p t k
				7
			},
			9|11|17|47 => {
				// b d g dx
				8
			},
			16|34|10|18|35|37 => {
				// f s ch hh sh th
				9
			},
			40|43|12|22|44 => {
				// v z dh jh zh
				10
			},
			26|27|28 => {
				// m n ng
				11
			},
			24|32|25|33 => {
				// l r lx rx
				12
			},
			41|42 => {
				// w y
				13
			},
			2|13|15|19|20 => {
				// ae eh ey ih ix
				14
			},
			38|39 => {
				// uh uw
				16
			},
			_ => {
				// otherwise
				15
			}
		}
	}
	fn phon2ch(&mut self) {
		// phoneme to Crystal and House (1988) [mostly] mean and standard deviations for duration modeling
		match self.CU_CLASS {
			7|8 => {
				// plosives
				self.mn = 92.0;
				self.sd = 32.0;
			},
			9|10 => {
				// fricative
				self.mn = 74.0;
				self.sd = 39.0;
			},
			11|12|13 => {
				// non-vowel sonorant
				self.mn = 67.0;
				self.sd = 28.0;
			},
			_ => {
				// otherwise
				self.mn = 90.0;
				self.sd = 60.0;
			},
		}
		if self.CU_PHONE == 10 {
			// CH
			self.mn = 126.0;
			self.sd = 36.0;
		} else if self.CU_PHONE == 22 {
			// JH
			self.mn = 105.0;
			self.sd = 31.0;
		}
		self.mn += 10.0;
		self.sd += 15.0;
	}
	fn getmin(&mut self) -> u16 {
		// get minimum durations of phoneme
		// taken from Klatt table
		match self.CU_PHONE {
			1  => 100,		// aa
			2  => 80,		// ae
			3  => 60,		// ah
			4  => 100,		// ao
			5  => 100,		// aw
			6  => 60,		// ax
			7  => 120,		// axr
			8  => 150,		// ay
			9  => 60,		// b
			10 => 50,		// ch
			11 => 50,		// d
			12 => 30,		// dh
			13 => 70,		// eh
			14 => 80,		// er
			15 => 100,		// ey
			16 => 80,		// f
			17 => 60,		// g
			19 => 40,		// ih
			20 => 60,		// ix
			21 => 55,		// iy
			22 => 50,		// jh
			23 => 60,		// k
			24 => 40,		// l
			25 => 70,		// lx
			26 => 60,		// m
			27 => 50,		// n
			28 => 60,		// ng
			29 => 80,		// ow
			30 => 150,		// oy
			31 => 50,		// p
			32 => 30,		// r
			33 => 70,		// rx
			34 => 60,		// s
			35 => 80,		// sh
			36 => 50,		// t
			37 => 60,		// th
			38 => 60,		// uh
			39 => 70,		// uw
			40 => 40,		// v
			41 => 60,		// w
			42 => 40,		// y
			43 => 40,		// z
			44 => 40,		// zh
			48 => 70,		// axp
			49 => 70,		// ixp
			_  => 20,		// otherwise
		}
	}
	fn adjust(&mut self) {
		// do some Klatt-esque duration adjustments after running the tree
		// this seems to improve articulation of the speech
		self.DUR_ADD = 0;
		self.DUR_SUB = 0;

		if self.CU_CLASS == 7 || self.CU_CLASS == 8 {
			if self.R1_CLASS == 7 || self.R1_CLASS == 8 || self.R1_CLASS == 11 {
				// PLOSIVE -> { plosive, nasal }
				// shorten plosive if next to another plosive or nasal
				self.DUR_SUB += 30;
			}
		}

		if self.L1_CLASS == 7 {
			if self.CU_CLASS == 11 || self.CU_CLASS == 12 || self.CU_CLASS == 13 || self.CU_CLASS == 14 || self.CU_CLASS == 16 {
				// Plosive aspiration lengthening
				self.DUR_ADD += 20;
				if self.CU_CLASS == 14 || self.CU_CLASS == 16 {
					// plosive -> VOWEL
					self.DUR_ADD += 10;
				}
			}
		}

		if self.L1_CLASS == 13 {
			if self.CU_CLASS == 14 || self.CU_CLASS == 16 {
				// Lengthening due to glide on left
				self.DUR_ADD = 20;
			}
		}

		if self.R1_CLASS == 13 {
			if self.CU_CLASS == 14 || self.CU_CLASS == 16 {
				// Lengthening due to glide on right
				self.DUR_ADD += 10;
			}
		}

		if self.L1_CLASS == 6 && self.CU_STRESS != 0 {
			if self.CU_CLASS == 14 || self.CU_CLASS == 16 {
				// stressed word-initial vowel
				self.DUR_ADD += 25;
			}
		}
		
		if self.R1_CLASS == 14 || self.R1_CLASS == 16 {
			if self.CU_CLASS == 14 || self.CU_CLASS == 16 {
				// VOWEL -> vowel
				self.DUR_ADD += 30;
			}
		}
		
		if self.L1_CLASS == 9 && self.R1_CLASS == 9 {
			if self.CU_CLASS == 14 || self.CU_CLASS == 16 {
				// vowel inbetween fricatives
				self.DUR_ADD += 15;
			}
		}
		
		if self.R1_CLASS == 7 || self.R1_CLASS == 8 {
			if self.CU_CLASS == 14 || self.CU_CLASS == 16 {
				// vowel before plosives
				self.DUR_ADD += 25;
			}
		}
		
		if self.CU_CLASS == 9 || self.CU_CLASS == 10 {
			if self.R1_CLASS == 6 {
				// word-final fricatives
				self.DUR_ADD += 20;
			}
		}
		
		if self.R1_CLASS == 12 {
			if self.CU_CLASS == 14 || self.CU_CLASS == 16 {
				// VOWEL -> liquid
				self.DUR_ADD += 10;
			}
		}
		
		if self.LAST_SYLL {
			if self.CU_CLASS == 10 {
				self.DUR_ADD += 20;
			} else {
				self.DUR_ADD += 40;
			}
			if self.R1_CLASS == 11 || self.R1_CLASS == 12 || self.R1_CLASS == 13 {
				self.DUR_SUB += 20;
			}
		}
	}
	fn adv(&mut self, phn: u8, strs: u8, is_last_syl: bool) {
		self.L2_CLASS  = self.L1_CLASS;
		self.L1_CLASS  = self.CU_CLASS;
		self.CU_CLASS  = self.R1_CLASS;
		self.R1_CLASS  = self.R2_CLASS;
		self.R2_CLASS  = self.phon2id(phn);

		self.L1_PHONE  = self.CU_PHONE;
		self.CU_PHONE  = self.R1_PHONE;
		self.R1_PHONE  = self.R2_PHONE;
		self.R2_PHONE  = phn;

		self.CU_STRESS = self.R1_STRESS;
		self.R1_STRESS = self.R2_STRESS;
		self.R2_STRESS = strs;

		self.LAST_SYLL = self.R1_LSYLL;
		self.R1_LSYLL  = self.R2_LSYLL;
		self.R2_LSYLL  = is_last_syl;
	}
	pub fn fetch(&mut self) -> (u8, u8, u8, u16, bool) {
		self.phon2ch();
		let mut pdur: u16 = (self.mn + self.CU_DURAT as f32 * self.sd) as u16 + self.DUR_ADD - self.DUR_SUB;
		let mindur: u16 = self.getmin();
		if pdur < mindur { pdur = mindur; }
		(self.L1_PHONE, self.CU_PHONE, self.CU_STRESS, pdur, self.LAST_SYLL)
	}
	pub fn is_ready(&mut self) -> bool {
		if self.CU_PHONE != 0 { return true; }
		false
	}
	pub fn is_not_last(&mut self) -> bool {
		if self.CU_PHONE != 0 && self.R1_PHONE == 0 && self.R2_PHONE == 0 { return false; }
		true
	}
	pub fn step(&mut self, phn: u8, strs: u8, is_last_syl: bool) {
		// update the model
		self.adv(phn, strs, is_last_syl);

		// start with worst case scenario first
		// before proceeding with tree
		self.CU_DURAT = 0.0;

		// the CART tree itself (some modifications)
		if self.CU_CLASS <= 5 {
			// current phoneme is phrase boundary
			if self.CU_CLASS <= 2 {
				// is comma or silent token
				if self.R1_CLASS <= 12 {
					if self.R1_CLASS <= 8 {
						// plosives and boundaries on right
						self.CU_DURAT = 3.187;
					} else {
						// fricatives on right
						self.CU_DURAT = 2.377;
					}
				} else {
					// vowels and glides on right
					self.CU_DURAT = 3.181;
				}
			} else {
				// is terminal punctuation ending sentences.
				if self.R1_CLASS <= 14 {
					// consonants, boundaries, glides, front vowels on right
					if self.L2_CLASS <= 6 {
						// phrase/word boundary on second left context
						self.CU_DURAT = -0.874;
					} else {
						// otherwise
						self.CU_DURAT = 5.391;
					}
				} else {
					// mid and back vowel on right
					if self.R2_CLASS <= 11 {
						// plosives, fricatives, boundaries on second right context
						self.CU_DURAT = 7.412;
					} else {
						// liquid, glides, vowels on second right context
						self.CU_DURAT = 16.006;
					}
				}
			}
		} else {
			// is not a phrase boundary
			if self.CU_CLASS == 6 {
				// is word boundary
				self.CU_DURAT = -0.874;
			} else {
				if self.CU_STRESS == 0 {
					// unstressed phoneme
					if self.R1_CLASS <= 5 {
						// phrase boundary on right
						if self.CU_CLASS <= 8 {
							// plosives
							self.CU_DURAT = 0.192;
						} else {
							if self.CU_CLASS <= 10 {
								// fricatives
								self.CU_DURAT = 1.344;
							} else {
								if self.CU_CLASS == 11 {
									// nasals
									self.CU_DURAT = 0.521;
								} else {
									// otherwise
									self.CU_DURAT = 1.308;
								}
							}
						}
					} else {
						if self.CU_CLASS <= 8 {
							// plosives
							if self.CU_CLASS <= 7 {
								// unvoiced plosive
								if self.L1_CLASS == 6 {
									// word boundary is left
									self.CU_DURAT = 0.137;
								} else {
									// otherwise
									self.CU_DURAT = -0.23;
								}
							} else {
								// voiced plosive
								self.CU_DURAT = -0.364;
							}
						} else {
							if self.CU_CLASS == 9 {
								// unvoiced fricative
								self.CU_DURAT = 0.317;
							} else {
								// otherwise
								if self.R1_CLASS <= 10 {
									// plosive, fricative on right
									if self.LAST_SYLL {
										// is in final syllable
										self.CU_DURAT = 0.126;
									} else {
										// otherwise
										self.CU_DURAT = -0.192;
									}
								} else {
									// otherwise
									self.CU_DURAT = -0.111;
								}
							}
						}
					}
				} else {
					// stressed phoneme
					if self.R1_CLASS <= 4 {
						// phrase boundaries except exclaimation point on right
						self.CU_DURAT = 2.48;
					} else {
						if self.R2_CLASS <= 4 {
							// phrase boundaries except exclaimation point on second right context
							self.CU_DURAT = 1.283;
						} else {
							if self.R1_CLASS <= 11 {
								// nasal, fricative, plosive, word boundary on right
								self.CU_DURAT = 0.549;
							} else {
								// otherwise
								self.CU_DURAT = 0.265;
							}
						}
					}
				}
			}
		}
		self.adjust();
	}
}
