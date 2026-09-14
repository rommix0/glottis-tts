// duration CART model
pub struct model {
	mn:        f32,
	sd:        f32,
	ACCENT_SWITCH: bool,
	DUR_ADD:   f32,
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
	CU_ACCENT: u8,
	R1_ACCENT: u8,
	R2_ACCENT: u8,
	CU_PITCH: f32,
	CU_BEGIN: f32,
	R1_BEGIN: f32,
	R2_BEGIN: f32,
	CU_END: f32,
	R1_END: f32,
	R2_END: f32,
}
impl model {
	pub fn new() -> model {
		model {
			ACCENT_SWITCH: false,
			mn:        100.0,
			sd:        85.0,
			DUR_ADD:   0.0,
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
			CU_ACCENT: 0,
			R1_ACCENT: 0,
			R2_ACCENT: 0,
			CU_PITCH: 0.0,
			CU_BEGIN: 0.0,
			R1_BEGIN: 0.0,
			R2_BEGIN: 0.0,
			CU_END: 0.0,
			R1_END: 0.0,
			R2_END: 0.0,
		}
	}
	pub fn reset(&mut self) {
		self.ACCENT_SWITCH = false;
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
		self.CU_ACCENT = 0;
		self.R1_ACCENT = 0;
		self.R2_ACCENT = 0;
		self.CU_PITCH  = 0.0;
		self.CU_BEGIN  = 0.0;
		self.R1_BEGIN  = 0.0;
		self.R2_BEGIN  = 0.0;
		self.CU_END    = 0.0;
		self.R1_END    = 0.0;
		self.R2_END    = 0.0;
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
	fn getmin(&mut self) -> f32 {
		// get minimum durations of phoneme
		// taken from Klatt table
		match self.CU_PHONE {
			1  => 100.0,	// aa
			2  => 80.0,		// ae
			3  => 60.0,		// ah
			4  => 100.0,	// ao
			5  => 100.0,	// aw
			6  => 60.0,		// ax
			7  => 120.0,	// axr
			8  => 150.0,	// ay
			9  => 60.0,		// b
			10 => 50.0,		// ch
			11 => 50.0,		// d
			12 => 30.0,		// dh
			13 => 70.0,		// eh
			14 => 80.0,		// er
			15 => 100.0,	// ey
			16 => 80.0,		// f
			17 => 60.0,		// g
			19 => 40.0,		// ih
			20 => 60.0,		// ix
			21 => 55.0,		// iy
			22 => 50.0,		// jh
			23 => 60.0,		// k
			24 => 40.0,		// l
			25 => 70.0,		// lx
			26 => 60.0,		// m
			27 => 50.0,		// n
			28 => 60.0,		// ng
			29 => 80.0,		// ow
			30 => 150.0,	// oy
			31 => 50.0,		// p
			32 => 30.0,		// r
			33 => 70.0,		// rx
			34 => 60.0,		// s
			35 => 80.0,		// sh
			36 => 50.0,		// t
			37 => 60.0,		// th
			38 => 60.0,		// uh
			39 => 70.0,		// uw
			40 => 40.0,		// v
			41 => 60.0,		// w
			42 => 40.0,		// y
			43 => 40.0,		// z
			44 => 40.0,		// zh
			48 => 70.0,		// axp
			49 => 70.0,		// ixp
			_  => 20.0,		// otherwise
		}
	}
	fn adjust(&mut self) {
		// do some Klatt-esque duration adjustments after running the tree
		// this seems to improve articulation of the speech
		self.DUR_ADD = 0.0;

		if self.CU_CLASS == 7 || self.CU_CLASS == 8 {
			if self.R1_CLASS == 7 || self.R1_CLASS == 8 || self.R1_CLASS == 11 {
				// PLOSIVE -> { plosive, nasal }
				// shorten plosive if next to another plosive or nasal
				self.DUR_ADD -= 30.0;
			}
		}

		if self.L1_CLASS == 7 {
			if self.CU_CLASS == 11 || self.CU_CLASS == 12 || self.CU_CLASS == 13 || self.CU_CLASS == 14 || self.CU_CLASS == 16 {
				// Plosive aspiration lengthening
				self.DUR_ADD += 20.0;
				if self.CU_CLASS == 14 || self.CU_CLASS == 16 {
					// plosive -> VOWEL
					self.DUR_ADD += 10.0;
				}
			}
		}

		if self.L1_CLASS == 13 {
			if self.CU_CLASS == 14 || self.CU_CLASS == 16 {
				// Lengthening due to glide on left
				self.DUR_ADD = 20.0;
			}
		}

		if self.R1_CLASS == 13 {
			if self.CU_CLASS == 14 || self.CU_CLASS == 16 {
				// Lengthening due to glide on right
				self.DUR_ADD += 10.0;
			}
		}

		if self.L1_CLASS == 6 && self.CU_STRESS != 0 {
			if self.CU_CLASS == 14 || self.CU_CLASS == 16 {
				// stressed word-initial vowel
				self.DUR_ADD += 25.0;
			}
		}

		if self.R1_CLASS == 14 || self.R1_CLASS == 16 {
			if self.CU_CLASS == 14 || self.CU_CLASS == 16 {
				// VOWEL -> vowel
				self.DUR_ADD += 30.0;
			}
		}

		if self.L1_CLASS == 9 && self.R1_CLASS == 9 {
			if self.CU_CLASS == 14 || self.CU_CLASS == 16 {
				// vowel inbetween fricatives
				self.DUR_ADD += 15.0;
			}
		}

		if self.R1_CLASS == 7 || self.R1_CLASS == 8 {
			if self.CU_CLASS == 14 || self.CU_CLASS == 16 {
				// vowel before plosives
				self.DUR_ADD += 25.0;
			}
		}

		if self.CU_CLASS == 9 || self.CU_CLASS == 10 {
			if self.R1_CLASS == 6 {
				// word-final fricatives
				self.DUR_ADD += 20.0;
			}
		}

		if self.R1_CLASS == 12 {
			if self.CU_CLASS == 14 || self.CU_CLASS == 16 {
				// VOWEL -> liquid
				self.DUR_ADD += 10.0;
			}
		}

		if self.LAST_SYLL {
			if self.CU_CLASS == 10 {
				self.DUR_ADD += 20.0;
			} else {
				self.DUR_ADD += 40.0;
			}
			if self.R1_CLASS == 11 || self.R1_CLASS == 12 || self.R1_CLASS == 13 {
				self.DUR_ADD -= 20.0;
			}
		}
	}
	fn adv(&mut self, phn: u8, strs: u8, is_last_syl: bool, p_id: usize, p_total: usize) {
		if phn == 55 {
			self.ACCENT_SWITCH = true;
		} else if phn == 50 {
			self.ACCENT_SWITCH = false;
		}
		let begin_per: f32 = (p_id as f32 / p_total as f32) * 100.0;
		let end_per:   f32 = ((p_total - p_id) as f32 / p_total as f32) * 100.0;

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

		self.CU_ACCENT = self.R1_ACCENT;
		self.R1_ACCENT = self.R2_ACCENT;
		self.R2_ACCENT = if self.ACCENT_SWITCH { 2 } else { 0 };

		self.CU_BEGIN = self.R1_BEGIN;
		self.R1_BEGIN = self.R2_BEGIN;
		self.R2_BEGIN = begin_per;

		self.CU_END = self.R1_END;
		self.R1_END = self.R2_END;
		self.R2_END = end_per;
	}
	pub fn fetch(&mut self) -> (u8, u8, u8, i32, bool, f32) {
		self.phon2ch();
		let mut pdur: f32 = (self.mn + self.CU_DURAT * self.sd) + self.DUR_ADD;

		let mindur: f32 = self.getmin();
		if pdur < mindur { pdur = mindur; }
		if self.CU_ACCENT == 0 { pdur *= 0.85; }
		
		self.get_pitch();
		//println!("pitch: {} hz", self.CU_PITCH);

		(self.L1_PHONE, self.CU_PHONE, self.CU_STRESS, pdur as i32, self.LAST_SYLL, self.CU_PITCH)
	}
	pub fn is_ready(&mut self) -> bool {
		if self.CU_PHONE != 0 { return true; }
		false
	}
	pub fn is_not_last(&mut self) -> bool {
		if self.CU_PHONE != 0 && self.R1_PHONE == 0 && self.R2_PHONE == 0 { return false; }
		true
	}
	fn get_pitch(&mut self) {
	  let pitch_mn: f32 = 110.0;
	  let pitch_sd: f32 = 90.0;
	  let mut x = 0.0;
	  if self.CU_CLASS <= 5 {
		if self.CU_BEGIN <= 98.53 {
		  x = -2.58;
		}
		if self.CU_BEGIN > 98.53 {
		  if self.L2_CLASS <= 15 {
			if self.L2_CLASS <= 8 {
			  x = -1.26;
			}
			if self.L2_CLASS > 8 {
			  if self.L1_CLASS <= 7 {
				x = -2.11;
			  }
			  if self.L1_CLASS > 7 {
				if self.CU_DURAT <= 4.14 {
				  if self.CU_DURAT <= 3.59 {
					if self.L2_CLASS <= 11 {
					  x = -1.98;
					}
					if self.L2_CLASS > 11 {
					  x = -1.38;
					}
				  }
				  if self.CU_DURAT > 3.59 {
					if self.L1_CLASS <= 13 {
					  x = -2.43;
					}
					if self.L1_CLASS > 13 {
					  x = -1.51;
					}
				  }
				}
				if self.CU_DURAT > 4.14 {
				  if self.L2_CLASS <= 11 {
					if self.L1_CLASS <= 12 {
					  if self.CU_DURAT <= 4.86 {
						x = 1.89;
					  }
					  if self.CU_DURAT > 4.86 {
						x = -0.52;
					  }
					}
					if self.L1_CLASS > 12 {
					  x = -1.44;
					}
				  }
				  if self.L2_CLASS > 11 {
					if self.L1_CLASS <= 9 {
					  x = -2.19;
					}
					if self.L1_CLASS > 9 {
					  if self.CU_DURAT <= 5.83 {
						x = -0.97;
					  }
					  if self.CU_DURAT > 5.83 {
						x = -1.76;
					  }
					}
				  }
				}
			  }
			}
		  }
		  if self.L2_CLASS > 15 {
			x = -0.92;
		  }
		}
	  }
	  if self.CU_CLASS > 5 {
		if self.R1_CLASS <= 3 {
		  if self.R1_CLASS <= 2 {
			if self.CU_END <= 92.27 {
			  if self.CU_DURAT <= 2.34 {
				if self.CU_END <= 83.14 {
				  if self.CU_ACCENT <= 0 {
					x = -0.75;
				  }
				  if self.CU_ACCENT > 0 {
					if self.CU_DURAT <= -0.32 {
					  if self.CU_DURAT <= -0.36 {
						x = -0.69;
					  }
					  if self.CU_DURAT > -0.36 {
						x = 1.38;
					  }
					}
					if self.CU_DURAT > -0.32 {
					  if self.CU_DURAT <= 1.05 {
						if self.L2_CLASS <= 7 {
						  if self.CU_DURAT <= 0.86 {
							x = -1.53;
						  }
						  if self.CU_DURAT > 0.86 {
							x = -0.73;
						  }
						}
						if self.L2_CLASS > 7 {
						  x = -1.1;
						}
					  }
					  if self.CU_DURAT > 1.05 {
						if self.CU_CLASS <= 13 {
						  if self.CU_DURAT <= 2.07 {
							x = -0.79;
						  }
						  if self.CU_DURAT > 2.07 {
							x = -1.68;
						  }
						}
						if self.CU_CLASS > 13 {
						  if self.CU_DURAT <= 1.55 {
							x = -1.35;
						  }
						  if self.CU_DURAT > 1.55 {
							x = -0.92;
						  }
						}
					  }
					}
				  }
				}
				if self.CU_END > 83.14 {
				  if self.CU_END <= 83.24 {
					x = 1.57;
				  }
				  if self.CU_END > 83.24 {
					if self.R2_CLASS <= 8 {
					  x = -1.61;
					}
					if self.R2_CLASS > 8 {
					  x = -0.66;
					}
				  }
				}
			  }
			  if self.CU_DURAT > 2.34 {
				x = -0.35;
			  }
			}
			if self.CU_END > 92.27 {
			  if self.CU_BEGIN <= 2.48 {
				x = 2.23;
			  }
			  if self.CU_BEGIN > 2.48 {
				if self.CU_CLASS <= 7 {
				  x = 1.47;
				}
				if self.CU_CLASS > 7 {
				  x = -0.22;
				}
			  }
			}
		  }
		  if self.R1_CLASS > 2 {
			if self.CU_STRESS <= 0 {
			  if self.L1_CLASS <= 15 {
				if self.CU_DURAT <= 0.35 {
				  if self.L2_CLASS <= 6 {
					x = -2.5;
				  }
				  if self.L2_CLASS > 6 {
					if self.L1_CLASS <= 14 {
					  if self.CU_DURAT <= -0.02 {
						x = -1.51;
					  }
					  if self.CU_DURAT > -0.02 {
						if self.L1_CLASS <= 10 {
						  x = 0.56;
						}
						if self.L1_CLASS > 10 {
						  x = -1.02;
						}
					  }
					}
					if self.L1_CLASS > 14 {
					  x = -1.59;
					}
				  }
				}
				if self.CU_DURAT > 0.35 {
				  if self.L2_CLASS <= 16 {
					x = -1.71;
				  }
				  if self.L2_CLASS > 16 {
					x = -0.91;
				  }
				}
			  }
			  if self.L1_CLASS > 15 {
				x = -1.05;
			  }
			}
			if self.CU_STRESS > 0 {
			  x = -0.92;
			}
		  }
		}
		if self.R1_CLASS > 3 {
		  if self.CU_BEGIN <= 10.18 {
			if self.CU_ACCENT <= 0 {
			  if self.CU_DURAT <= 0.34 {
				if self.R1_CLASS <= 6 {
				  if self.CU_CLASS <= 10 {
					if self.CU_DURAT <= 0.16 {
					  if self.CU_DURAT <= -0.59 {
						if self.CU_BEGIN <= 2.33 {
						  x = -0.24;
						}
						if self.CU_BEGIN > 2.33 {
						  x = 0.75;
						}
					  }
					  if self.CU_DURAT > -0.59 {
						if self.CU_CLASS <= 7 {
						  x = -0.73;
						}
						if self.CU_CLASS > 7 {
						  x = -0.16;
						}
					  }
					}
					if self.CU_DURAT > 0.16 {
					  if self.CU_BEGIN <= 4.56 {
						if self.CU_CLASS <= 9 {
						  x = 1.79;
						}
						if self.CU_CLASS > 9 {
						  x = 0.39;
						}
					  }
					  if self.CU_BEGIN > 4.56 {
						x = 0.19;
					  }
					}
				  }
				  if self.CU_CLASS > 10 {
					if self.L2_CLASS <= 14 {
					  if self.CU_END <= 90.2 {
						x = -0.54;
					  }
					  if self.CU_END > 90.2 {
						if self.L2_CLASS <= 4 {
						  if self.CU_CLASS <= 14 {
							x = 0.27;
						  }
						  if self.CU_CLASS > 14 {
							x = -0.26;
						  }
						}
						if self.L2_CLASS > 4 {
						  if self.CU_DURAT <= 0.16 {
							x = 0.52;
						  }
						  if self.CU_DURAT > 0.16 {
							x = 0.1;
						  }
						}
					  }
					}
					if self.L2_CLASS > 14 {
					  if self.CU_BEGIN <= 4.36 {
						x = 2.26;
					  }
					  if self.CU_BEGIN > 4.36 {
						x = 0.76;
					  }
					}
				  }
				}
				if self.R1_CLASS > 6 {
				  if self.L1_CLASS <= 15 {
					if self.CU_BEGIN <= 7.88 {
					  if self.CU_BEGIN <= 3.1 {
						if self.CU_DURAT <= -0.72 {
						  if self.L1_CLASS <= 8 {
							x = -0.27;
						  }
						  if self.L1_CLASS > 8 {
							x = 0.29;
						  }
						}
						if self.CU_DURAT > -0.72 {
						  if self.CU_CLASS <= 9 {
							x = 0.9;
						  }
						  if self.CU_CLASS > 9 {
							x = 0.24;
						  }
						}
					  }
					  if self.CU_BEGIN > 3.1 {
						if self.L1_CLASS <= 10 {
						  if self.CU_DURAT <= -0.83 {
							x = 0.2;
						  }
						  if self.CU_DURAT > -0.83 {
							x = 0.61;
						  }
						}
						if self.L1_CLASS > 10 {
						  if self.CU_DURAT <= -0.42 {
							x = 0.79;
						  }
						  if self.CU_DURAT > -0.42 {
							x = 0.32;
						  }
						}
					  }
					}
					if self.CU_BEGIN > 7.88 {
					  if self.L1_CLASS <= 13 {
						if self.L1_CLASS <= 9 {
						  if self.R2_CLASS <= 14 {
							x = 0.12;
						  }
						  if self.R2_CLASS > 14 {
							x = 0.66;
						  }
						}
						if self.L1_CLASS > 9 {
						  x = -0.14;
						}
					  }
					  if self.L1_CLASS > 13 {
						if self.L2_CLASS <= 6 {
						  x = -0.29;
						}
						if self.L2_CLASS > 6 {
						  x = 0.59;
						}
					  }
					}
				  }
				  if self.L1_CLASS > 15 {
					if self.R1_CLASS <= 14 {
					  x = 1.23;
					}
					if self.R1_CLASS > 14 {
					  if self.L2_CLASS <= 13 {
						x = 1.64;
					  }
					  if self.L2_CLASS > 13 {
						x = -0.35;
					  }
					}
				  }
				}
			  }
			  if self.CU_DURAT > 0.34 {
				if self.CU_END <= 91.87 {
				  if self.R1_CLASS <= 9 {
					if self.L1_CLASS <= 9 {
					  x = 2.13;
					}
					if self.L1_CLASS > 9 {
					  x = 0.31;
					}
				  }
				  if self.R1_CLASS > 9 {
					x = -0.26;
				  }
				}
				if self.CU_END > 91.87 {
				  if self.CU_CLASS <= 8 {
					x = -1.2;
				  }
				  if self.CU_CLASS > 8 {
					if self.R1_CLASS <= 6 {
					  if self.R2_CLASS <= 13 {
						if self.L1_CLASS <= 7 {
						  x = 0.11;
						}
						if self.L1_CLASS > 7 {
						  x = 1.6;
						}
					  }
					  if self.R2_CLASS > 13 {
						if self.L2_CLASS <= 11 {
						  x = -0.6;
						}
						if self.L2_CLASS > 11 {
						  x = 2.49;
						}
					  }
					}
					if self.R1_CLASS > 6 {
					  if self.CU_DURAT <= 1.06 {
						if self.CU_CLASS <= 14 {
						  if self.CU_DURAT <= 0.68 {
							x = 0.76;
						  }
						  if self.CU_DURAT > 0.68 {
							x = 1.44;
						  }
						}
						if self.CU_CLASS > 14 {
						  x = 0.33;
						}
					  }
					  if self.CU_DURAT > 1.06 {
						x = 0.08;
					  }
					}
				  }
				}
			  }
			}
			if self.CU_ACCENT > 0 {
			  if self.R2_CLASS <= 4 {
				if self.CU_END <= 93.32 {
				  if self.L1_CLASS <= 10 {
					x = -0.11;
				  }
				  if self.L1_CLASS > 10 {
					x = -0.94;
				  }
				}
				if self.CU_END > 93.32 {
				  if self.L1_CLASS <= 8 {
					x = 2.27;
				  }
				  if self.L1_CLASS > 8 {
					if self.CU_CLASS <= 13 {
					  x = 1.01;
					}
					if self.CU_CLASS > 13 {
					  if self.L2_CLASS <= 7 {
						x = 0.8;
					  }
					  if self.L2_CLASS > 7 {
						if self.CU_BEGIN <= 6.49 {
						  x = -0.84;
						}
						if self.CU_BEGIN > 6.49 {
						  x = 1.56;
						}
					  }
					}
				  }
				}
			  }
			  if self.R2_CLASS > 4 {
				if self.L1_CLASS <= 6 {
				  if self.CU_CLASS <= 9 {
					if self.CU_ACCENT <= 2 {
					  if self.R1_CLASS <= 14 {
						if self.CU_DURAT <= 0.14 {
						  if self.CU_ACCENT <= 1 {
							x = 0.39;
						  }
						  if self.CU_ACCENT > 1 {
							x = 1.53;
						  }
						}
						if self.CU_DURAT > 0.14 {
						  if self.CU_END <= 98.74 {
							x = 1.08;
						  }
						  if self.CU_END > 98.74 {
							x = 3.74;
						  }
						}
					  }
					  if self.R1_CLASS > 14 {
						if self.CU_DURAT <= -0.69 {
						  x = 3.54;
						}
						if self.CU_DURAT > -0.69 {
						  if self.CU_END <= 99.44 {
							x = 1.33;
						  }
						  if self.CU_END > 99.44 {
							x = -0.81;
						  }
						}
					  }
					}
					if self.CU_ACCENT > 2 {
					  if self.CU_CLASS <= 8 {
						x = -0.26;
					  }
					  if self.CU_CLASS > 8 {
						x = 0.75;
					  }
					}
				  }
				  if self.CU_CLASS > 9 {
					if self.R1_CLASS <= 10 {
					  x = 0.64;
					}
					if self.R1_CLASS > 10 {
					  if self.L2_CLASS <= 15 {
						if self.CU_DURAT <= 0.15 {
						  if self.CU_END <= 92.18 {
							x = -0.08;
						  }
						  if self.CU_END > 92.18 {
							x = 0.45;
						  }
						}
						if self.CU_DURAT > 0.15 {
						  if self.R1_CLASS <= 16 {
							x = -0.17;
						  }
						  if self.R1_CLASS > 16 {
							x = 1.05;
						  }
						}
					  }
					  if self.L2_CLASS > 15 {
						if self.L2_CLASS <= 16 {
						  x = 1.35;
						}
						if self.L2_CLASS > 16 {
						  x = 0.17;
						}
					  }
					}
				  }
				}
				if self.L1_CLASS > 6 {
				  if self.L1_CLASS <= 10 {
					if self.R1_CLASS <= 11 {
					  if self.CU_END <= 96.45 {
						if self.CU_DURAT <= 0.94 {
						  if self.CU_STRESS <= 0 {
							x = 0.79;
						  }
						  if self.CU_STRESS > 0 {
							x = 1.38;
						  }
						}
						if self.CU_DURAT > 0.94 {
						  x = 0.44;
						}
					  }
					  if self.CU_END > 96.45 {
						if self.CU_END <= 96.74 {
						  x = 2.24;
						}
						if self.CU_END > 96.74 {
						  x = 1.24;
						}
					  }
					}
					if self.R1_CLASS > 11 {
					  if self.CU_CLASS <= 7 {
						if self.L2_CLASS <= 6 {
						  x = 2.34;
						}
						if self.L2_CLASS > 6 {
						  x = 1.45;
						}
					  }
					  if self.CU_CLASS > 7 {
						if self.CU_END <= 95.83 {
						  if self.L1_CLASS <= 7 {
							x = 1.51;
						  }
						  if self.L1_CLASS > 7 {
							x = 0.93;
						  }
						}
						if self.CU_END > 95.83 {
						  if self.L2_CLASS <= 3 {
							x = 0.02;
						  }
						  if self.L2_CLASS > 3 {
							x = 1.85;
						  }
						}
					  }
					}
				  }
				  if self.L1_CLASS > 10 {
					if self.CU_END <= 93.42 {
					  if self.R1_CLASS <= 7 {
						if self.CU_CLASS <= 11 {
						  if self.CU_DURAT <= -0.42 {
							x = 0.85;
						  }
						  if self.CU_DURAT > -0.42 {
							x = -0.05;
						  }
						}
						if self.CU_CLASS > 11 {
						  x = 0.6;
						}
					  }
					  if self.R1_CLASS > 7 {
						if self.CU_ACCENT <= 1 {
						  if self.CU_DURAT <= 0.46 {
							x = 0.49;
						  }
						  if self.CU_DURAT > 0.46 {
							x = 1.04;
						  }
						}
						if self.CU_ACCENT > 1 {
						  x = 1.06;
						}
					  }
					}
					if self.CU_END > 93.42 {
					  if self.CU_DURAT <= 0.8 {
						if self.CU_BEGIN <= 3.62 {
						  if self.R2_CLASS <= 10 {
							x = 0.88;
						  }
						  if self.R2_CLASS > 10 {
							x = 1.26;
						  }
						}
						if self.CU_BEGIN > 3.62 {
						  if self.CU_CLASS <= 15 {
							x = 0.78;
						  }
						  if self.CU_CLASS > 15 {
							x = 1.44;
						  }
						}
					  }
					  if self.CU_DURAT > 0.8 {
						x = -0.1;
					  }
					}
				  }
				}
			  }
			}
		  }
		  if self.CU_BEGIN > 10.18 {
			if self.CU_ACCENT <= 0 {
			  if self.R1_CLASS <= 10 {
				if self.CU_CLASS <= 15 {
				  if self.L1_CLASS <= 15 {
					if self.L1_CLASS <= 9 {
					  if self.L2_CLASS <= 9 {
						if self.CU_DURAT <= -0.47 {
						  if self.L1_CLASS <= 7 {
							x = 0.59;
						  }
						  if self.L1_CLASS > 7 {
							x = -0.03;
						  }
						}
						if self.CU_DURAT > -0.47 {
						  if self.R2_CLASS <= 4 {
							x = -1.05;
						  }
						  if self.R2_CLASS > 4 {
							x = -0.04;
						  }
						}
					  }
					  if self.L2_CLASS > 9 {
						if self.L1_CLASS <= 8 {
						  if self.CU_END <= 29.95 {
							x = -0.45;
						  }
						  if self.CU_END > 29.95 {
							x = -0.26;
						  }
						}
						if self.L1_CLASS > 8 {
						  if self.R1_CLASS <= 6 {
							x = 0.84;
						  }
						  if self.R1_CLASS > 6 {
							x = -0.02;
						  }
						}
					  }
					}
					if self.L1_CLASS > 9 {
					  if self.R1_CLASS <= 6 {
						if self.CU_CLASS <= 10 {
						  if self.CU_DURAT <= 0.21 {
							x = -0.46;
						  }
						  if self.CU_DURAT > 0.21 {
							x = -0.11;
						  }
						}
						if self.CU_CLASS > 10 {
						  if self.L2_CLASS <= 13 {
							x = -0.25;
						  }
						  if self.L2_CLASS > 13 {
							x = 0.26;
						  }
						}
					  }
					  if self.R1_CLASS > 6 {
						if self.L1_CLASS <= 10 {
						  if self.CU_DURAT <= 0.06 {
							x = -0.39;
						  }
						  if self.CU_DURAT > 0.06 {
							x = -0.01;
						  }
						}
						if self.L1_CLASS > 10 {
						  if self.L2_CLASS <= 6 {
							x = -0.27;
						  }
						  if self.L2_CLASS > 6 {
							x = -0.06;
						  }
						}
					  }
					}
				  }
				  if self.L1_CLASS > 15 {
					if self.R2_CLASS <= 11 {
					  x = -0.42;
					}
					if self.R2_CLASS > 11 {
					  if self.CU_END <= 14.34 {
						x = -0.09;
					  }
					  if self.CU_END > 14.34 {
						if self.CU_END <= 86.12 {
						  if self.CU_END <= 83.68 {
							x = 0.28;
						  }
						  if self.CU_END > 83.68 {
							x = 1.41;
						  }
						}
						if self.CU_END > 86.12 {
						  x = -0.43;
						}
					  }
					}
				  }
				}
				if self.CU_CLASS > 15 {
				  if self.L2_CLASS <= 9 {
					if self.CU_DURAT <= 0.25 {
					  x = 0.2;
					}
					if self.CU_DURAT > 0.25 {
					  if self.CU_END <= 59.5 {
						x = 0.3;
					  }
					  if self.CU_END > 59.5 {
						if self.CU_DURAT <= 1.02 {
						  if self.CU_END <= 60.84 {
							x = 3.55;
						  }
						  if self.CU_END > 60.84 {
							x = 1.28;
						  }
						}
						if self.CU_DURAT > 1.02 {
						  x = -0.62;
						}
					  }
					}
				  }
				  if self.L2_CLASS > 9 {
					if self.L1_CLASS <= 14 {
					  x = -0.23;
					}
					if self.L1_CLASS > 14 {
					  if self.CU_DURAT <= -0.31 {
						x = 1.41;
					  }
					  if self.CU_DURAT > -0.31 {
						x = 0.13;
					  }
					}
				  }
				}
			  }
			  if self.R1_CLASS > 10 {
				if self.L2_CLASS <= 9 {
				  if self.L1_CLASS <= 9 {
					if self.L1_CLASS <= 6 {
					  if self.CU_CLASS <= 10 {
						if self.L2_CLASS <= 8 {
						  if self.L2_CLASS <= 7 {
							x = 0.43;
						  }
						  if self.L2_CLASS > 7 {
							x = 0.01;
						  }
						}
						if self.L2_CLASS > 8 {
						  if self.CU_CLASS <= 7 {
							x = 1.28;
						  }
						  if self.CU_CLASS > 7 {
							x = 0.55;
						  }
						}
					  }
					  if self.CU_CLASS > 10 {
						if self.CU_DURAT <= -0.53 {
						  x = 0.54;
						}
						if self.CU_DURAT > -0.53 {
						  if self.CU_DURAT <= -0.05 {
							x = 0.01;
						  }
						  if self.CU_DURAT > -0.05 {
							x = -0.25;
						  }
						}
					  }
					}
					if self.L1_CLASS > 6 {
					  if self.CU_BEGIN <= 97.4 {
						if self.L2_CLASS <= 8 {
						  if self.R2_CLASS <= 9 {
							x = 0.58;
						  }
						  if self.R2_CLASS > 9 {
							x = 0.25;
						  }
						}
						if self.L2_CLASS > 8 {
						  if self.R2_CLASS <= 8 {
							x = 1.38;
						  }
						  if self.R2_CLASS > 8 {
							x = 0.66;
						  }
						}
					  }
					  if self.CU_BEGIN > 97.4 {
						x = -1.0;
					  }
					}
				  }
				  if self.L1_CLASS > 9 {
					if self.R2_CLASS <= 4 {
					  if self.CU_DURAT <= -0.13 {
						x = -2.49;
					  }
					  if self.CU_DURAT > -0.13 {
						x = -0.73;
					  }
					}
					if self.R2_CLASS > 4 {
					  if self.L1_CLASS <= 15 {
						if self.CU_END <= 7.79 {
						  x = -0.35;
						}
						if self.CU_END > 7.79 {
						  x = 0.04;
						}
					  }
					  if self.L1_CLASS > 15 {
						x = 0.42;
					  }
					}
				  }
				}
				if self.L2_CLASS > 9 {
				  if self.CU_DURAT <= -0.6 {
					if self.L1_CLASS <= 10 {
					  if self.L1_CLASS <= 9 {
						if self.L1_CLASS <= 8 {
						  if self.CU_CLASS <= 7 {
							x = -0.34;
						  }
						  if self.CU_CLASS > 7 {
							x = -0.1;
						  }
						}
						if self.L1_CLASS > 8 {
						  if self.CU_BEGIN <= 46.77 {
							x = 0.32;
						  }
						  if self.CU_BEGIN > 46.77 {
							x = 0.01;
						  }
						}
					  }
					  if self.L1_CLASS > 9 {
						if self.R2_CLASS <= 16 {
						  x = -0.43;
						}
						if self.R2_CLASS > 16 {
						  if self.CU_BEGIN <= 73.99 {
							x = -0.6;
						  }
						  if self.CU_BEGIN > 73.99 {
							x = -1.48;
						  }
						}
					  }
					}
					if self.L1_CLASS > 10 {
					  if self.CU_BEGIN <= 92.69 {
						if self.R1_CLASS <= 13 {
						  if self.L1_CLASS <= 11 {
							x = -0.01;
						  }
						  if self.L1_CLASS > 11 {
							x = 0.24;
						  }
						}
						if self.R1_CLASS > 13 {
						  if self.CU_END <= 8.14 {
							x = 0.92;
						  }
						  if self.CU_END > 8.14 {
							x = -0.03;
						  }
						}
					  }
					  if self.CU_BEGIN > 92.69 {
						x = -0.26;
					  }
					}
				  }
				  if self.CU_DURAT > -0.6 {
					if self.CU_CLASS <= 9 {
					  if self.L1_CLASS <= 4 {
						if self.CU_DURAT <= -0.28 {
						  x = 0.33;
						}
						if self.CU_DURAT > -0.28 {
						  x = 0.96;
						}
					  }
					  if self.L1_CLASS > 4 {
						if self.CU_DURAT <= 0.35 {
						  if self.CU_CLASS <= 7 {
							x = 0.51;
						  }
						  if self.CU_CLASS > 7 {
							x = -0.01;
						  }
						}
						if self.CU_DURAT > 0.35 {
						  if self.L2_CLASS <= 16 {
							x = 0.58;
						  }
						  if self.L2_CLASS > 16 {
							x = 1.59;
						  }
						}
					  }
					}
					if self.CU_CLASS > 9 {
					  if self.L1_CLASS <= 8 {
						if self.CU_DURAT <= -0.11 {
						  if self.R2_CLASS <= 16 {
							x = -0.11;
						  }
						  if self.R2_CLASS > 16 {
							x = 0.35;
						  }
						}
						if self.CU_DURAT > -0.11 {
						  x = -0.27;
						}
					  }
					  if self.L1_CLASS > 8 {
						if self.CU_CLASS <= 10 {
						  x = -0.4;
						}
						if self.CU_CLASS > 10 {
						  if self.CU_CLASS <= 14 {
							x = 0.67;
						  }
						  if self.CU_CLASS > 14 {
							x = 0.09;
						  }
						}
					  }
					}
				  }
				}
			  }
			}
			if self.CU_ACCENT > 0 {
			  if self.R2_CLASS <= 3 {
				if self.R1_CLASS <= 10 {
				  if self.CU_STRESS <= 0 {
					if self.R2_CLASS <= 2 {
					  if self.CU_DURAT <= 0.65 {
						if self.R1_CLASS <= 9 {
						  if self.CU_ACCENT <= 2 {
							x = -1.3;
						  }
						  if self.CU_ACCENT > 2 {
							x = -0.7;
						  }
						}
						if self.R1_CLASS > 9 {
						  if self.CU_DURAT <= 0.42 {
							x = -0.35;
						  }
						  if self.CU_DURAT > 0.42 {
							x = -1.49;
						  }
						}
					  }
					  if self.CU_DURAT > 0.65 {
						if self.R1_CLASS <= 9 {
						  x = -0.32;
						}
						if self.R1_CLASS > 9 {
						  if self.L2_CLASS <= 7 {
							x = -1.83;
						  }
						  if self.L2_CLASS > 7 {
							x = -0.92;
						  }
						}
					  }
					}
					if self.R2_CLASS > 2 {
					  if self.CU_DURAT <= -0.54 {
						x = -0.2;
					  }
					  if self.CU_DURAT > -0.54 {
						if self.CU_CLASS <= 16 {
						  if self.CU_BEGIN <= 99.37 {
							x = -1.84;
						  }
						  if self.CU_BEGIN > 99.37 {
							x = -0.84;
						  }
						}
						if self.CU_CLASS > 16 {
						  x = -1.21;
						}
					  }
					}
				  }
				  if self.CU_STRESS > 0 {
					if self.R2_CLASS <= 2 {
					  if self.CU_STRESS <= 1 {
						if self.CU_BEGIN <= 12.98 {
						  x = 0.95;
						}
						if self.CU_BEGIN > 12.98 {
						  x = -0.26;
						}
					  }
					  if self.CU_STRESS > 1 {
						x = -0.97;
					  }
					}
					if self.R2_CLASS > 2 {
					  x = -0.96;
					}
				  }
				}
				if self.R1_CLASS > 10 {
				  if self.R1_CLASS <= 15 {
					if self.L1_CLASS <= 6 {
					  x = 0.5;
					}
					if self.L1_CLASS > 6 {
					  if self.R2_CLASS <= 2 {
						if self.CU_DURAT <= 1.76 {
						  x = -0.56;
						}
						if self.CU_DURAT > 1.76 {
						  x = 0.07;
						}
					  }
					  if self.R2_CLASS > 2 {
						if self.CU_CLASS <= 14 {
						  x = -0.79;
						}
						if self.CU_CLASS > 14 {
						  if self.CU_DURAT <= 0.07 {
							x = -1.64;
						  }
						  if self.CU_DURAT > 0.07 {
							x = -0.76;
						  }
						}
					  }
					}
				  }
				  if self.R1_CLASS > 15 {
					if self.R2_CLASS <= 2 {
					  x = 0.81;
					}
					if self.R2_CLASS > 2 {
					  x = -0.22;
					}
				  }
				}
			  }
			  if self.R2_CLASS > 3 {
				if self.CU_END <= 7.33 {
				  if self.L1_CLASS <= 9 {
					if self.L2_CLASS <= 7 {
					  if self.CU_CLASS <= 13 {
						if self.CU_BEGIN <= 98.76 {
						  x = 0.74;
						}
						if self.CU_BEGIN > 98.76 {
						  x = -0.88;
						}
					  }
					  if self.CU_CLASS > 13 {
						if self.CU_BEGIN <= 95.53 {
						  if self.L1_CLASS <= 8 {
							x = 0.02;
						  }
						  if self.L1_CLASS > 8 {
							x = 0.81;
						  }
						}
						if self.CU_BEGIN > 95.53 {
						  x = -0.14;
						}
					  }
					}
					if self.L2_CLASS > 7 {
					  if self.CU_CLASS <= 9 {
						if self.CU_DURAT <= 0.49 {
						  if self.CU_BEGIN <= 97.2 {
							x = 0.09;
						  }
						  if self.CU_BEGIN > 97.2 {
							x = -0.42;
						  }
						}
						if self.CU_DURAT > 0.49 {
						  x = 0.42;
						}
					  }
					  if self.CU_CLASS > 9 {
						if self.L1_CLASS <= 6 {
						  x = -0.42;
						}
						if self.L1_CLASS > 6 {
						  if self.CU_BEGIN <= 97.55 {
							x = 0.06;
						  }
						  if self.CU_BEGIN > 97.55 {
							x = -0.49;
						  }
						}
					  }
					}
				  }
				  if self.L1_CLASS > 9 {
					if self.CU_BEGIN <= 96.32 {
					  if self.CU_DURAT <= 0.16 {
						if self.R1_CLASS <= 9 {
						  if self.CU_DURAT <= -0.8 {
							x = -2.58;
						  }
						  if self.CU_DURAT > -0.8 {
							x = -0.41;
						  }
						}
						if self.R1_CLASS > 9 {
						  x = -0.19;
						}
					  }
					  if self.CU_DURAT > 0.16 {
						x = -0.06;
					  }
					}
					if self.CU_BEGIN > 96.32 {
					  if self.CU_DURAT <= -0.17 {
						if self.CU_BEGIN <= 97.15 {
						  x = -0.4;
						}
						if self.CU_BEGIN > 97.15 {
						  if self.R2_CLASS <= 10 {
							x = -1.31;
						  }
						  if self.R2_CLASS > 10 {
							x = -0.8;
						  }
						}
					  }
					  if self.CU_DURAT > -0.17 {
						x = -0.57;
					  }
					}
				  }
				}
				if self.CU_END > 7.33 {
				  if self.L1_CLASS <= 9 {
					if self.L1_CLASS <= 6 {
					  if self.CU_CLASS <= 9 {
						if self.CU_CLASS <= 8 {
						  if self.CU_CLASS <= 7 {
							x = 0.52;
						  }
						  if self.CU_CLASS > 7 {
							x = -0.25;
						  }
						}
						if self.CU_CLASS > 8 {
						  if self.L2_CLASS <= 7 {
							x = 1.05;
						  }
						  if self.L2_CLASS > 7 {
							x = 0.55;
						  }
						}
					  }
					  if self.CU_CLASS > 9 {
						if self.L2_CLASS <= 7 {
						  if self.CU_DURAT <= -0.06 {
							x = 0.65;
						  }
						  if self.CU_DURAT > -0.06 {
							x = -0.02;
						  }
						}
						if self.L2_CLASS > 7 {
						  if self.R2_CLASS <= 7 {
							x = 0.06;
						  }
						  if self.R2_CLASS > 7 {
							x = -0.14;
						  }
						}
					  }
					}
					if self.L1_CLASS > 6 {
					  if self.R1_CLASS <= 11 {
						if self.L1_CLASS <= 8 {
						  if self.L1_CLASS <= 7 {
							x = 0.46;
						  }
						  if self.L1_CLASS > 7 {
							x = 0.14;
						  }
						}
						if self.L1_CLASS > 8 {
						  if self.CU_DURAT <= -0.39 {
							x = 0.92;
						  }
						  if self.CU_DURAT > -0.39 {
							x = 0.56;
						  }
						}
					  }
					  if self.R1_CLASS > 11 {
						if self.CU_CLASS <= 7 {
						  if self.CU_DURAT <= -0.57 {
							x = 0.44;
						  }
						  if self.CU_DURAT > -0.57 {
							x = 1.46;
						  }
						}
						if self.CU_CLASS > 7 {
						  if self.L1_CLASS <= 7 {
							x = 0.97;
						  }
						  if self.L1_CLASS > 7 {
							x = 0.54;
						  }
						}
					  }
					}
				  }
				  if self.L1_CLASS > 9 {
					if self.R1_CLASS <= 6 {
					  if self.CU_CLASS <= 11 {
						if self.CU_DURAT <= 0.91 {
						  if self.CU_ACCENT <= 2 {
							x = -0.13;
						  }
						  if self.CU_ACCENT > 2 {
							x = 0.1;
						  }
						}
						if self.CU_DURAT > 0.91 {
						  if self.CU_END <= 82.24 {
							x = -0.71;
						  }
						  if self.CU_END > 82.24 {
							x = 1.44;
						  }
						}
					  }
					  if self.CU_CLASS > 11 {
						if self.CU_DURAT <= 0.56 {
						  if self.CU_END <= 87.56 {
							x = 0.38;
						  }
						  if self.CU_END > 87.56 {
							x = 1.22;
						  }
						}
						if self.CU_DURAT > 0.56 {
						  if self.CU_STRESS <= 0 {
							x = -0.28;
						  }
						  if self.CU_STRESS > 0 {
							x = 0.29;
						  }
						}
					  }
					}
					if self.R1_CLASS > 6 {
					  if self.CU_CLASS <= 15 {
						if self.CU_DURAT <= -0.01 {
						  if self.CU_CLASS <= 10 {
							x = 0.08;
						  }
						  if self.CU_CLASS > 10 {
							x = 0.2;
						  }
						}
						if self.CU_DURAT > -0.01 {
						  if self.CU_CLASS <= 9 {
							x = 0.44;
						  }
						  if self.CU_CLASS > 9 {
							x = 0.19;
						  }
						}
					  }
					  if self.CU_CLASS > 15 {
						if self.CU_BEGIN <= 63.22 {
						  if self.CU_END <= 85.49 {
							x = 0.55;
						  }
						  if self.CU_END > 85.49 {
							x = 1.02;
						  }
						}
						if self.CU_BEGIN > 63.22 {
						  x = 0.29;
						}
					  }
					}
				  }
				}
			  }
			}
		  }
		}
	  }
	  self.CU_PITCH = pitch_mn + x * pitch_sd;
	}
	pub fn step(&mut self, phn: u8, strs: u8, is_last_syl: bool, p_id: usize, p_total: usize) {
		// update the model
		self.adv(phn, strs, is_last_syl, p_id, p_total);

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
