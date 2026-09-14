pub struct model {
	dur_mn: f32,
	dur_sd: f32,
	pit_mn: f32,
	pit_sd: f32,
	CU_DURAT: f32,
	DUR_ADD: f32,

	CU_BEGIN: f32,
	R1_BEGIN: f32,			// only for loading
	R2_BEGIN: f32,			// only for loading

	CU_STRESS: u8,
	R1_STRESS: u8,			// only for loading
	R2_STRESS: u8,			// only for loading

	CU_PREV_ACCENT: u8,
	R1_PREV_ACCENT: u8,		// only for loading
	R2_PREV_ACCENT: u8,		// only for loading

	CU_CURR_ACCENT: u8,
	R1_CURR_ACCENT: u8,		// only for loading
	R2_CURR_ACCENT: u8,		// only for loading

	L1_F0: f32,
	F0: f32,

	L2_CLASS: u8,
	L1_CLASS: u8,
	CU_CLASS: u8,
	R1_CLASS: u8,
	R2_CLASS: u8,

	L2_PHONE: u8,
	L1_PHONE: u8,
	CU_PHONE: u8,
	R1_PHONE: u8,
	R2_PHONE: u8,
}
impl model {
	pub fn new() -> model {
		model {
			dur_mn: 95.0,
			dur_sd: 60.0,
			pit_mn: 110.0,
			pit_sd: 55.0,
			CU_DURAT: 0.0,
			DUR_ADD: 0.0,

			CU_BEGIN: 0.0,
			R1_BEGIN: 0.0,
			R2_BEGIN: 0.0,

			CU_STRESS: 0,
			R1_STRESS: 0,
			R2_STRESS: 0,

			CU_PREV_ACCENT: 0,
			R1_PREV_ACCENT: 0,
			R2_PREV_ACCENT: 0,

			CU_CURR_ACCENT: 0,
			R1_CURR_ACCENT: 0,
			R2_CURR_ACCENT: 0,

			L1_F0: 0.0,
			F0: 0.0,

			L2_CLASS: 0,
			L1_CLASS: 0,
			CU_CLASS: 0,
			R1_CLASS: 0,
			R2_CLASS: 0,

			L2_PHONE: 0,
			L1_PHONE: 0,
			CU_PHONE: 0,
			R1_PHONE: 0,
			R2_PHONE: 0,
		}
	}
	pub fn reset(&mut self) {
		self.CU_DURAT = 0.0;

		self.CU_BEGIN = 0.0;
		self.R1_BEGIN = 0.0;
		self.R2_BEGIN = 0.0;

		self.CU_STRESS = 0;
		self.R1_STRESS = 0;
		self.R2_STRESS = 0;

		self.CU_PREV_ACCENT = 0;
		self.R1_PREV_ACCENT = 0;
		self.R2_PREV_ACCENT = 0;

		self.CU_CURR_ACCENT = 0;
		self.R1_CURR_ACCENT = 0;
		self.R2_CURR_ACCENT = 0;

		self.L1_F0 = 0.0;
		self.F0    = 0.0;

		self.L2_CLASS = 0;
		self.L1_CLASS = 0;
		self.CU_CLASS = 0;
		self.R1_CLASS = 0;
		self.R2_CLASS = 0;

		self.L2_PHONE = 0;
		self.L1_PHONE = 0;
		self.CU_PHONE = 0;
		self.R1_PHONE = 0;
		self.R2_PHONE = 0;
	}
	pub fn is_ready(&mut self) -> bool {
		if self.CU_PHONE != 0 { return true; }
		false
	}
	pub fn is_not_last(&mut self) -> bool {
		if self.CU_PHONE != 0 && self.R1_PHONE == 0 && self.R2_PHONE == 0 { return false; }
		true
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
			24|32 => {
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
			25|33 => {
				// lx rx
				17
			},
			_ => {
				// otherwise
				15
			}
		}
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

	}
	fn adv(&mut self, phn: u8, strs: u8, prev_accent: u8, curr_accent: u8, p_id: usize, p_total: usize) {
		let begin_per: f32 = (p_id as f32 / p_total as f32) * 100.0;
		self.L1_F0 = self.F0;

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

		self.CU_PREV_ACCENT = self.R1_PREV_ACCENT;
		self.R1_PREV_ACCENT = self.R2_PREV_ACCENT;
		self.R2_PREV_ACCENT = prev_accent;

		self.CU_CURR_ACCENT = self.R1_CURR_ACCENT;
		self.R1_CURR_ACCENT = self.R2_CURR_ACCENT;
		self.R2_CURR_ACCENT = curr_accent;

		self.CU_BEGIN = self.R1_BEGIN;
		self.R1_BEGIN = self.R2_BEGIN;
		self.R2_BEGIN = begin_per;
	}
	pub fn fetch(&mut self) -> (u8, u8, u8, i32, f32) {
		let mut pdur: f32 = (self.dur_mn + self.CU_DURAT * self.dur_sd) + self.DUR_ADD;
		let mut pitch: f32 = self.pit_mn + self.F0 * self.pit_sd;
		if pitch < 50.0 { pitch = 50.0; }

		let mindur: f32 = self.getmin();
		if self.CU_CURR_ACCENT == 0 { pdur *= 0.85; }
		if pdur < mindur { pdur = mindur; }

		println!("Duration {}, F0: {}", pdur, pitch);

		(self.L1_PHONE, self.CU_PHONE, self.CU_STRESS, pdur as i32, pitch)
	}
	fn pitch_model(&mut self) {
		let mut x: f32 = 0.0;
		if self.L1_F0 <= 0.22 {
			if self.CU_CLASS <= 5 {
				if self.CU_BEGIN <= 98.0 {
					x = -2.12;
				}
				if self.CU_BEGIN > 98.0 {
					if self.L1_F0 <= -1.52 {
						if self.L1_F0 <= -2.01 {
							x = -2.18;
						}
						if self.L1_F0 > -2.01 {
							x = -1.86;
						}
					}
					if self.L1_F0 > -1.52 {
						if self.L1_F0 <= -0.66 {
							x = -1.29;
						}
						if self.L1_F0 > -0.66 {
							x = -0.21;
						}
					}
				}
			}
			if self.CU_CLASS > 5 {
				if self.L1_F0 <= -0.38 {
					if self.L1_CLASS <= 5 {
						if self.CU_CLASS <= 9 {
							if self.CU_DURAT <= -0.51 {
								if self.CU_CLASS <= 7 {
									if self.CU_BEGIN <= 35.5 {
										if self.CU_PREV_ACCENT == 0 {
											x = 2.03;
										}
										if self.CU_PREV_ACCENT == 1 {
											x = 6.69;
										}
									}
									if self.CU_BEGIN > 35.5 {
										x = -0.03;
									}
								}
								if self.CU_CLASS > 7 {
									if self.L1_CLASS <= 4 {
										if self.CU_DURAT <= -0.62 {
											if self.CU_DURAT <= -0.63 {
												x = 0.5;
											}
											if self.CU_DURAT > -0.63 {
												x = 1.9;
											}
										}
										if self.CU_DURAT > -0.62 {
											x = 0.24;
										}
									}
									if self.L1_CLASS > 4 {
										x = 2.6;
									}
								}
							}
							if self.CU_DURAT > -0.51 {
								if self.CU_BEGIN <= 1.5 {
									if self.CU_CLASS <= 7 {
										x = 2.31;
									}
									if self.CU_CLASS > 7 {
										if self.CU_CLASS <= 8 {
											x = 0.57;
										}
										if self.CU_CLASS > 8 {
											x = 1.59;
										}
									}
								}
								if self.CU_BEGIN > 1.5 {
									if self.CU_CLASS <= 8 {
										if self.CU_CLASS <= 7 {
											x = 1.0;
										}
										if self.CU_CLASS > 7 {
											if self.CU_DURAT <= -0.34 {
												if self.L1_CLASS <= 1 {
													x = -2.21;
												}
												if self.L1_CLASS > 1 {
													x = 0.56;
												}
											}
											if self.CU_DURAT > -0.34 {
												x = -0.73;
											}
										}
									}
									if self.CU_CLASS > 8 {
										if self.L1_CLASS <= 2 {
											x = 0.99;
										}
										if self.L1_CLASS > 2 {
											x = 1.44;
										}
									}
								}
							}
						}
						if self.CU_CLASS > 9 {
							if self.CU_CLASS <= 13 {
								if self.CU_CLASS <= 10 {
									if self.CU_DURAT <= -0.11 {
										if self.CU_BEGIN <= 9.0 {
											if self.CU_DURAT <= -0.3 {
												if self.CU_CURR_ACCENT == 0 {
													x = 0.32;
												}
												if self.CU_CURR_ACCENT == 1 {
													x = 4.06;
												}
											}
											if self.CU_DURAT > -0.3 {
												x = -0.01;
											}
										}
										if self.CU_BEGIN > 9.0 {
											x = -0.07;
										}
									}
									if self.CU_DURAT > -0.11 {
										x = -0.56;
									}
								}
								if self.CU_CLASS > 10 {
									x = 0.2;
								}
							}
							if self.CU_CLASS > 13 {
								if self.CU_CURR_ACCENT == 0 {
									if self.CU_CLASS <= 14 {
										if self.CU_BEGIN <= 1.5 {
											if self.CU_DURAT <= 0.61 {
												x = 0.76;
											}
											if self.CU_DURAT > 0.61 {
												x = 2.03;
											}
										}
										if self.CU_BEGIN > 1.5 {
											if self.L1_CLASS <= 2 {
												x = 0.26;
											}
											if self.L1_CLASS > 2 {
												x = 0.65;
											}
										}
									}
									if self.CU_CLASS > 14 {
										if self.CU_BEGIN <= 90.0 {
											if self.L1_CLASS <= 4 {
												if self.CU_STRESS == 0 {
													if self.CU_BEGIN <= 86.0 {
														x = 0.45;
													}
													if self.CU_BEGIN > 86.0 {
														x = 2.91;
													}
												}
												if self.CU_STRESS == 1 {
													x = 1.24;
												}
											}
											if self.L1_CLASS > 4 {
												x = 4.78;
											}
										}
										if self.CU_BEGIN > 90.0 {
											x = 6.66;
										}
									}
								}
								if self.CU_CURR_ACCENT == 1 {
									if self.CU_DURAT <= 0.4 {
										x = 0.22;
									}
									if self.CU_DURAT > 0.4 {
										if self.CU_CLASS <= 14 {
											x = 2.23;
										}
										if self.CU_CLASS > 14 {
											x = 1.2;
										}
									}
								}
							}
						}
					}
					if self.L1_CLASS > 5 {
						if self.L1_F0 <= -1.16 {
							if self.L1_CLASS <= 6 {
								if self.CU_DURAT <= 0.67 {
									if self.L1_F0 <= -1.69 {
										if self.CU_BEGIN <= 70.0 {
											if self.CU_BEGIN <= 67.0 {
												if self.CU_CLASS <= 7 {
													x = 1.05;
												}
												if self.CU_CLASS > 7 {
													x = -0.11;
												}
											}
											if self.CU_BEGIN > 67.0 {
												x = 2.97;
											}
										}
										if self.CU_BEGIN > 70.0 {
											x = -0.78;
										}
									}
									if self.L1_F0 > -1.69 {
										if self.CU_DURAT <= -0.03 {
											x = -0.75;
										}
										if self.CU_DURAT > -0.03 {
											x = -0.48;
										}
									}
								}
								if self.CU_DURAT > 0.67 {
									x = 0.02;
								}
							}
							if self.L1_CLASS > 6 {
								if self.CU_STRESS == 0 {
									if self.L1_F0 <= -1.5 {
										if self.CU_BEGIN <= 97.5 {
											if self.CU_CLASS <= 11 {
												if self.L1_F0 <= -1.84 {
													x = -1.84;
												}
												if self.L1_F0 > -1.84 {
													if self.CU_CLASS <= 10 {
														x = -1.57;
													}
													if self.CU_CLASS > 10 {
														if self.CU_CURR_ACCENT == 0 {
															if self.CU_DURAT <= -0.24 {
																x = 1.82;
															}
															if self.CU_DURAT > -0.24 {
																x = -1.85;
															}
														}
														if self.CU_CURR_ACCENT == 1 {
															x = -1.94;
														}
													}
												}
											}
											if self.CU_CLASS > 11 {
												if self.CU_BEGIN <= 2.5 {
													x = -0.24;
												}
												if self.CU_BEGIN > 2.5 {
													if self.L1_CLASS <= 10 {
														if self.CU_CLASS <= 13 {
															x = -0.82;
														}
														if self.CU_CLASS > 13 {
															x = -1.48;
														}
													}
													if self.L1_CLASS > 10 {
														x = -1.79;
													}
												}
											}
										}
										if self.CU_BEGIN > 97.5 {
											x = -2.03;
										}
									}
									if self.L1_F0 > -1.5 {
										if self.L1_CLASS <= 10 {
											if self.CU_DURAT <= -0.67 {
												x = -1.31;
											}
											if self.CU_DURAT > -0.67 {
												if self.CU_DURAT <= 0.52 {
													if self.CU_CURR_ACCENT == 0 {
														x = -0.52;
													}
													if self.CU_CURR_ACCENT == 1 {
														if self.CU_CLASS <= 14 {
															x = -0.83;
														}
														if self.CU_CLASS > 14 {
															x = -1.24;
														}
													}
												}
												if self.CU_DURAT > 0.52 {
													x = -1.46;
												}
											}
										}
										if self.L1_CLASS > 10 {
											if self.CU_BEGIN <= 97.5 {
												if self.CU_CLASS <= 10 {
													x = -1.34;
												}
												if self.CU_CLASS > 10 {
													x = -1.57;
												}
											}
											if self.CU_BEGIN > 97.5 {
												x = -1.94;
											}
										}
									}
								}
								if self.CU_STRESS == 1 {
									if self.L1_F0 <= -1.93 {
										if self.CU_BEGIN <= 90.5 {
											if self.L1_F0 <= -1.99 {
												x = 0.54;
											}
											if self.L1_F0 > -1.99 {
												x = 3.09;
											}
										}
										if self.CU_BEGIN > 90.5 {
											x = 3.09;
										}
									}
									if self.L1_F0 > -1.93 {
										if self.CU_STRESS <= 1 {
											if self.CU_BEGIN <= 97.5 {
												if self.L1_F0 <= -1.56 {
													if self.L1_CLASS <= 9 {
														x = -0.02;
													}
													if self.L1_CLASS > 9 {
														if self.CU_BEGIN <= 55.0 {
															x = 2.27;
														}
														if self.CU_BEGIN > 55.0 {
															x = -0.58;
														}
													}
												}
												if self.L1_F0 > -1.56 {
													x = -0.18;
												}
											}
											if self.CU_BEGIN > 97.5 {
												x = -1.12;
											}
										}
										if self.CU_STRESS > 1 {
											x = -1.16;
										}
									}
								}
							}
						}
						if self.L1_F0 > -1.16 {
							if self.CU_STRESS == 0 {
								if self.L1_CLASS <= 9 {
									if self.L1_F0 <= -0.75 {
										if self.CU_DURAT <= -0.74 {
											if self.L1_F0 <= -0.93 {
												x = -1.03;
											}
											if self.L1_F0 > -0.93 {
												x = -0.83;
											}
										}
										if self.CU_DURAT > -0.74 {
											if self.CU_CLASS <= 13 {
												if self.CU_DURAT <= 0.14 {
													if self.CU_CLASS <= 10 {
														if self.CU_CLASS <= 7 {
															x = -0.41;
														}
														if self.CU_CLASS > 7 {
															x = -0.69;
														}
													}
													if self.CU_CLASS > 10 {
														x = -0.39;
													}
												}
												if self.CU_DURAT > 0.14 {
													x = -0.27;
												}
											}
											if self.CU_CLASS > 13 {
												if self.CU_BEGIN <= 97.5 {
													x = -0.67;
												}
												if self.CU_BEGIN > 97.5 {
													x = -1.51;
												}
											}
										}
									}
									if self.L1_F0 > -0.75 {
										if self.CU_CLASS <= 6 {
											if self.L1_F0 <= -0.53 {
												x = -0.64;
											}
											if self.L1_F0 > -0.53 {
												x = -0.45;
											}
										}
										if self.CU_CLASS > 6 {
											if self.CU_CLASS <= 7 {
												x = -0.08;
											}
											if self.CU_CLASS > 7 {
												if self.CU_CLASS <= 8 {
													x = -0.73;
												}
												if self.CU_CLASS > 8 {
													if self.CU_CLASS <= 13 {
														if self.CU_DURAT <= 0.23 {
															if self.CU_CLASS <= 10 {
																x = -0.37;
															}
															if self.CU_CLASS > 10 {
																if self.L1_CLASS <= 7 {
																	x = -0.24;
																}
																if self.L1_CLASS > 7 {
																	x = 0.02;
																}
															}
														}
														if self.CU_DURAT > 0.23 {
															if self.CU_CLASS <= 9 {
																x = -0.01;
															}
															if self.CU_CLASS > 9 {
																if self.L1_CLASS <= 7 {
																	x = -0.31;
																}
																if self.L1_CLASS > 7 {
																	x = 0.26;
																}
															}
														}
													}
													if self.CU_CLASS > 13 {
														if self.CU_BEGIN <= 98.5 {
															if self.L1_CLASS <= 8 {
																x = -0.41;
															}
															if self.L1_CLASS > 8 {
																if self.CU_CURR_ACCENT == 0 {
																	x = -0.43;
																}
																if self.CU_CURR_ACCENT == 1 {
																	x = -0.87;
																}
															}
														}
														if self.CU_BEGIN > 98.5 {
															x = -1.33;
														}
													}
												}
											}
										}
									}
								}
								if self.L1_CLASS > 9 {
									if self.CU_BEGIN <= 98.5 {
										if self.L1_F0 <= -0.78 {
											if self.L1_F0 <= -0.96 {
												if self.CU_CURR_ACCENT == 0 {
													if self.CU_CLASS <= 12 {
														x = -1.04;
													}
													if self.CU_CLASS > 12 {
														x = -0.71;
													}
												}
												if self.CU_CURR_ACCENT == 1 {
													if self.CU_CLASS <= 10 {
														if self.CU_CLASS <= 8 {
															x = -1.11;
														}
														if self.CU_CLASS > 8 {
															x = -0.84;
														}
													}
													if self.CU_CLASS > 10 {
														if self.L1_CLASS <= 10 {
															x = -0.96;
														}
														if self.L1_CLASS > 10 {
															if self.CU_DURAT <= 1.02 {
																x = -1.24;
															}
															if self.CU_DURAT > 1.02 {
																if self.CU_CLASS <= 16 {
																	x = -1.37;
																}
																if self.CU_CLASS > 16 {
																	x = 4.87;
																}
															}
														}
													}
												}
											}
											if self.L1_F0 > -0.96 {
												if self.L1_CLASS <= 10 {
													x = -0.78;
												}
												if self.L1_CLASS > 10 {
													if self.CU_DURAT <= 0.55 {
														if self.CU_CLASS <= 9 {
															if self.CU_CLASS <= 8 {
																x = -0.92;
															}
															if self.CU_CLASS > 8 {
																x = -0.62;
															}
														}
														if self.CU_CLASS > 9 {
															x = -0.97;
														}
													}
													if self.CU_DURAT > 0.55 {
														x = -1.27;
													}
												}
											}
										}
										if self.L1_F0 > -0.78 {
											if self.CU_CURR_ACCENT == 0 {
												if self.L1_F0 <= -0.59 {
													x = -0.7;
												}
												if self.L1_F0 > -0.59 {
													if self.CU_DURAT <= -0.74 {
														x = -0.48;
													}
													if self.CU_DURAT > -0.74 {
														if self.CU_CLASS <= 10 {
															x = -0.72;
														}
														if self.CU_CLASS > 10 {
															x = -0.48;
														}
													}
												}
											}
											if self.CU_CURR_ACCENT == 1 {
												if self.CU_DURAT <= 1.15 {
													if self.L1_CLASS <= 10 {
														x = -0.45;
													}
													if self.L1_CLASS > 10 {
														if self.CU_DURAT <= -0.28 {
															x = -0.91;
														}
														if self.CU_DURAT > -0.28 {
															if self.CU_CLASS <= 14 {
																if self.L1_CLASS <= 11 {
																	x = -0.5;
																}
																if self.L1_CLASS > 11 {
																	x = -0.72;
																}
															}
															if self.CU_CLASS > 14 {
																x = -0.91;
															}
														}
													}
												}
												if self.CU_DURAT > 1.15 {
													x = -1.68;
												}
											}
										}
									}
									if self.CU_BEGIN > 98.5 {
										if self.CU_DURAT <= -0.47 {
											x = -1.22;
										}
										if self.CU_DURAT > -0.47 {
											x = -1.86;
										}
									}
								}
							}
							if self.CU_STRESS == 1 {
								if self.CU_STRESS <= 1 {
									if self.CU_BEGIN <= 90.5 {
										if self.CU_PREV_ACCENT == 0 {
											if self.L1_CLASS <= 9 {
												if self.CU_DURAT <= 1.69 {
													if self.CU_CURR_ACCENT == 0 {
														if self.CU_DURAT <= 0.25 {
															x = -0.19;
														}
														if self.CU_DURAT > 0.25 {
															x = 0.25;
														}
													}
													if self.CU_CURR_ACCENT == 1 {
														if self.CU_DURAT <= 0.32 {
															x = 0.51;
														}
														if self.CU_DURAT > 0.32 {
															if self.L1_F0 <= -0.9 {
																if self.L1_CLASS <= 8 {
																	x = -0.19;
																}
																if self.L1_CLASS > 8 {
																	x = 0.92;
																}
															}
															if self.L1_F0 > -0.9 {
																if self.CU_BEGIN <= 80.0 {
																	x = 0.29;
																}
																if self.CU_BEGIN > 80.0 {
																	x = 0.74;
																}
															}
														}
													}
												}
												if self.CU_DURAT > 1.69 {
													x = -0.47;
												}
											}
											if self.L1_CLASS > 9 {
												if self.CU_BEGIN <= 2.5 {
													if self.L1_F0 <= -0.96 {
														if self.L1_CLASS <= 10 {
															x = -0.07;
														}
														if self.L1_CLASS > 10 {
															x = 2.87;
														}
													}
													if self.L1_F0 > -0.96 {
														if self.CU_DURAT <= 0.27 {
															x = -0.41;
														}
														if self.CU_DURAT > 0.27 {
															if self.L1_F0 <= -0.78 {
																x = -0.15;
															}
															if self.L1_F0 > -0.78 {
																x = 1.2;
															}
														}
													}
												}
												if self.CU_BEGIN > 2.5 {
													if self.CU_CURR_ACCENT == 0 {
														if self.CU_DURAT <= 0.03 {
															x = -0.53;
														}
														if self.CU_DURAT > 0.03 {
															if self.L1_F0 <= -0.82 {
																x = -0.6;
															}
															if self.L1_F0 > -0.82 {
																x = -0.03;
															}
														}
													}
													if self.CU_CURR_ACCENT == 1 {
														if self.L1_CLASS <= 10 {
															if self.CU_DURAT <= 1.78 {
																x = 0.47;
															}
															if self.CU_DURAT > 1.78 {
																x = -0.45;
															}
														}
														if self.L1_CLASS > 10 {
															if self.L1_F0 <= -0.53 {
																x = -0.37;
															}
															if self.L1_F0 > -0.53 {
																if self.CU_DURAT <= 1.01 {
																	x = 0.08;
																}
																if self.CU_DURAT > 1.01 {
																	x = -0.49;
																}
															}
														}
													}
												}
											}
										}
										if self.CU_PREV_ACCENT == 1 {
											if self.L1_CLASS <= 11 {
												if self.CU_CURR_ACCENT == 0 {
													x = -0.16;
												}
												if self.CU_CURR_ACCENT == 1 {
													if self.CU_DURAT <= 1.42 {
														x = 0.09;
													}
													if self.CU_DURAT > 1.42 {
														x = -0.5;
													}
												}
											}
											if self.L1_CLASS > 11 {
												x = -0.39;
											}
										}
									}
									if self.CU_BEGIN > 90.5 {
										if self.CU_BEGIN <= 98.5 {
											if self.CU_CURR_ACCENT == 0 {
												x = -0.46;
											}
											if self.CU_CURR_ACCENT == 1 {
												if self.CU_DURAT <= 1.06 {
													x = -0.04;
												}
												if self.CU_DURAT > 1.06 {
													x = -0.56;
												}
											}
										}
										if self.CU_BEGIN > 98.5 {
											x = -0.84;
										}
									}
								}
								if self.CU_STRESS > 1 {
									if self.L1_CLASS <= 8 {
										if self.CU_BEGIN <= 2.0 {
											x = 1.69;
										}
										if self.CU_BEGIN > 2.0 {
											x = -0.42;
										}
									}
									if self.L1_CLASS > 8 {
										x = -0.93;
									}
								}
							}
						}
					}
				}
				if self.L1_F0 > -0.38 {
					if self.L1_CLASS <= 13 {
						if self.CU_STRESS == 0 {
							if self.L1_F0 <= -0.01 {
								if self.L1_F0 <= -0.22 {
									if self.CU_CLASS <= 13 {
										if self.CU_DURAT <= -0.08 {
											if self.CU_CURR_ACCENT == 0 {
												x = -0.29;
											}
											if self.CU_CURR_ACCENT == 1 {
												if self.CU_CLASS <= 7 {
													x = 0.11;
												}
												if self.CU_CLASS > 7 {
													if self.CU_CLASS <= 8 {
														x = -0.59;
													}
													if self.CU_CLASS > 8 {
														x = -0.13;
													}
												}
											}
										}
										if self.CU_DURAT > -0.08 {
											x = -0.02;
										}
									}
									if self.CU_CLASS > 13 {
										if self.CU_CURR_ACCENT == 0 {
											x = -0.22;
										}
										if self.CU_CURR_ACCENT == 1 {
											if self.CU_BEGIN <= 97.5 {
												x = -0.44;
											}
											if self.CU_BEGIN > 97.5 {
												x = -1.21;
											}
										}
									}
								}
								if self.L1_F0 > -0.22 {
									if self.L1_CLASS <= 10 {
										if self.CU_CLASS <= 13 {
											if self.CU_DURAT <= -0.74 {
												x = -0.12;
											}
											if self.CU_DURAT > -0.74 {
												if self.L1_CLASS <= 6 {
													if self.CU_CLASS <= 7 {
														x = 0.17;
													}
													if self.CU_CLASS > 7 {
														if self.CU_CLASS <= 8 {
															x = -0.5;
														}
														if self.CU_CLASS > 8 {
															if self.CU_CLASS <= 9 {
																x = 0.13;
															}
															if self.CU_CLASS > 9 {
																x = -0.08;
															}
														}
													}
												}
												if self.L1_CLASS > 6 {
													x = 0.24;
												}
											}
										}
										if self.CU_CLASS > 13 {
											if self.L1_CLASS <= 9 {
												if self.L1_CLASS <= 8 {
													x = -0.15;
												}
												if self.L1_CLASS > 8 {
													x = -0.38;
												}
											}
											if self.L1_CLASS > 9 {
												x = 0.1;
											}
										}
									}
									if self.L1_CLASS > 10 {
										x = -0.22;
									}
								}
							}
							if self.L1_F0 > -0.01 {
								if self.L1_CLASS <= 10 {
									if self.CU_CLASS <= 14 {
										if self.CU_CURR_ACCENT == 0 {
											if self.L1_F0 <= 0.14 {
												x = 0.07;
											}
											if self.L1_F0 > 0.14 {
												x = 0.22;
											}
										}
										if self.CU_CURR_ACCENT == 1 {
											if self.CU_CLASS <= 7 {
												x = 0.48;
											}
											if self.CU_CLASS > 7 {
												if self.CU_CLASS <= 8 {
													x = -0.31;
												}
												if self.CU_CLASS > 8 {
													if self.CU_CLASS <= 9 {
														x = 0.4;
													}
													if self.CU_CLASS > 9 {
														if self.CU_CLASS <= 10 {
															x = -0.33;
														}
														if self.CU_CLASS > 10 {
															if self.CU_DURAT <= 0.44 {
																x = 0.31;
															}
															if self.CU_DURAT > 0.44 {
																x = -0.12;
															}
														}
													}
												}
											}
										}
									}
									if self.CU_CLASS > 14 {
										if self.CU_BEGIN <= 97.5 {
											if self.CU_BEGIN <= 1.5 {
												x = 1.36;
											}
											if self.CU_BEGIN > 1.5 {
												x = 0.04;
											}
										}
										if self.CU_BEGIN > 97.5 {
											x = -1.05;
										}
									}
								}
								if self.L1_CLASS > 10 {
									x = -0.01;
								}
							}
						}
						if self.CU_STRESS == 1 {
							if self.CU_DURAT <= 0.9 {
								if self.CU_BEGIN <= 3.5 {
									if self.CU_DURAT <= 0.32 {
										x = 0.78;
									}
									if self.CU_DURAT > 0.32 {
										if self.CU_BEGIN <= 1.5 {
											x = 2.25;
										}
										if self.CU_BEGIN > 1.5 {
											x = 1.07;
										}
									}
								}
								if self.CU_BEGIN > 3.5 {
									if self.CU_CURR_ACCENT == 0 {
										if self.L1_F0 <= -0.2 {
											if self.CU_DURAT <= -0.02 {
												x = -0.2;
											}
											if self.CU_DURAT > -0.02 {
												x = 0.07;
											}
										}
										if self.L1_F0 > -0.2 {
											if self.CU_DURAT <= 0.17 {
												x = 0.14;
											}
											if self.CU_DURAT > 0.17 {
												if self.L1_CLASS <= 10 {
													x = 0.28;
												}
												if self.L1_CLASS > 10 {
													if self.CU_DURAT <= 0.43 {
														if self.CU_BEGIN <= 66.5 {
															if self.CU_BEGIN <= 57.5 {
																x = 0.95;
															}
															if self.CU_BEGIN > 57.5 {
																x = 2.48;
															}
														}
														if self.CU_BEGIN > 66.5 {
															x = 0.42;
														}
													}
													if self.CU_DURAT > 0.43 {
														if self.L1_CLASS <= 12 {
															x = 0.68;
														}
														if self.L1_CLASS > 12 {
															x = -0.27;
														}
													}
												}
											}
										}
									}
									if self.CU_CURR_ACCENT == 1 {
										if self.CU_STRESS <= 1 {
											if self.L1_CLASS <= 11 {
												if self.L1_F0 <= 0.04 {
													if self.CU_PREV_ACCENT == 0 {
														if self.L1_CLASS <= 8 {
															if self.L1_CLASS <= 7 {
																x = 0.59;
															}
															if self.L1_CLASS > 7 {
																x = 0.94;
															}
														}
														if self.L1_CLASS > 8 {
															x = 0.46;
														}
													}
													if self.CU_PREV_ACCENT == 1 {
														x = 0.34;
													}
												}
												if self.L1_F0 > 0.04 {
													x = 0.71;
												}
											}
											if self.L1_CLASS > 11 {
												if self.CU_BEGIN <= 97.5 {
													if self.CU_DURAT <= 0.43 {
														x = 0.35;
													}
													if self.CU_DURAT > 0.43 {
														if self.L1_F0 <= -0.01 {
															if self.CU_DURAT <= 0.7 {
																x = 0.06;
															}
															if self.CU_DURAT > 0.7 {
																if self.CU_DURAT <= 0.87 {
																	x = -0.67;
																}
																if self.CU_DURAT > 0.87 {
																	x = 0.5;
																}
															}
														}
														if self.L1_F0 > -0.01 {
															x = 0.33;
														}
													}
												}
												if self.CU_BEGIN > 97.5 {
													x = -0.44;
												}
											}
										}
										if self.CU_STRESS > 1 {
											x = -0.13;
										}
									}
								}
							}
							if self.CU_DURAT > 0.9 {
								if self.L1_CLASS <= 11 {
									if self.CU_STRESS <= 1 {
										if self.CU_BEGIN <= 1.5 {
											x = 1.25;
										}
										if self.CU_BEGIN > 1.5 {
											if self.CU_DURAT <= 1.68 {
												if self.CU_BEGIN <= 94.5 {
													if self.L1_F0 <= 0.11 {
														if self.CU_PREV_ACCENT == 0 {
															x = 0.27;
														}
														if self.CU_PREV_ACCENT == 1 {
															if self.L1_CLASS <= 9 {
																x = -0.03;
															}
															if self.L1_CLASS > 9 {
																if self.L1_CLASS <= 10 {
																	x = 1.07;
																}
																if self.L1_CLASS > 10 {
																	x = -0.03;
																}
															}
														}
													}
													if self.L1_F0 > 0.11 {
														if self.CU_DURAT <= 1.1 {
															if self.CU_BEGIN <= 35.0 {
																if self.CU_DURAT <= 1.08 {
																	x = 1.03;
																}
																if self.CU_DURAT > 1.08 {
																	x = 2.64;
																}
															}
															if self.CU_BEGIN > 35.0 {
																x = 0.52;
															}
														}
														if self.CU_DURAT > 1.1 {
															x = 0.21;
														}
													}
												}
												if self.CU_BEGIN > 94.5 {
													x = -0.23;
												}
											}
											if self.CU_DURAT > 1.68 {
												if self.L1_CLASS <= 6 {
													x = 0.62;
												}
												if self.L1_CLASS > 6 {
													x = -0.33;
												}
											}
										}
									}
									if self.CU_STRESS > 1 {
										x = -0.73;
									}
								}
								if self.L1_CLASS > 11 {
									if self.CU_BEGIN <= 95.5 {
										if self.CU_BEGIN <= 86.0 {
											if self.CU_DURAT <= 1.53 {
												if self.L1_F0 <= -0.22 {
													x = -0.69;
												}
												if self.L1_F0 > -0.22 {
													x = -0.09;
												}
											}
											if self.CU_DURAT > 1.53 {
												x = -0.81;
											}
										}
										if self.CU_BEGIN > 86.0 {
											if self.CU_CLASS <= 15 {
												x = 0.06;
											}
											if self.CU_CLASS > 15 {
												x = 2.17;
											}
										}
									}
									if self.CU_BEGIN > 95.5 {
										x = -0.88;
									}
								}
							}
						}
					}
					if self.L1_CLASS > 13 {
						if self.L1_F0 <= -0.14 {
							if self.CU_CLASS <= 6 {
								x = -0.25;
							}
							if self.CU_CLASS > 6 {
								if self.CU_BEGIN <= 98.5 {
									if self.CU_STRESS == 0 {
										if self.CU_CLASS <= 10 {
											if self.CU_DURAT <= -0.04 {
												x = -0.69;
											}
											if self.CU_DURAT > -0.04 {
												x = -0.42;
											}
										}
										if self.CU_CLASS > 10 {
											if self.CU_CURR_ACCENT == 0 {
												x = -0.31;
											}
											if self.CU_CURR_ACCENT == 1 {
												if self.CU_CLASS <= 14 {
													x = -0.51;
												}
												if self.CU_CLASS > 14 {
													if self.CU_DURAT <= 0.8 {
														x = -0.65;
													}
													if self.CU_DURAT > 0.8 {
														x = -1.44;
													}
												}
											}
										}
									}
									if self.CU_STRESS == 1 {
										x = 0.3;
									}
								}
								if self.CU_BEGIN > 98.5 {
									x = -1.35;
								}
							}
						}
						if self.L1_F0 > -0.14 {
							if self.CU_CLASS <= 6 {
								x = 0.04;
							}
							if self.CU_CLASS > 6 {
								if self.CU_CLASS <= 10 {
									if self.CU_DURAT <= -0.12 {
										x = -0.48;
									}
									if self.CU_DURAT > -0.12 {
										if self.CU_BEGIN <= 98.5 {
											x = -0.29;
										}
										if self.CU_BEGIN > 98.5 {
											x = -1.4;
										}
									}
								}
								if self.CU_CLASS > 10 {
									if self.CU_BEGIN <= 97.5 {
										if self.CU_CURR_ACCENT == 0 {
											if self.CU_DURAT <= 0.52 {
												x = 0.01;
											}
											if self.CU_DURAT > 0.52 {
												x = -0.83;
											}
										}
										if self.CU_CURR_ACCENT == 1 {
											if self.CU_STRESS == 0 {
												if self.CU_DURAT <= 0.44 {
													if self.L1_F0 <= -0.01 {
														x = -0.36;
													}
													if self.L1_F0 > -0.01 {
														x = -0.15;
													}
												}
												if self.CU_DURAT > 0.44 {
													x = -0.51;
												}
											}
											if self.CU_STRESS == 1 {
												x = 0.36;
											}
										}
									}
									if self.CU_BEGIN > 97.5 {
										if self.CU_DURAT <= 0.04 {
											x = -0.31;
										}
										if self.CU_DURAT > 0.04 {
											x = -1.6;
										}
									}
								}
							}
						}
					}
				}
			}
		}
		if self.L1_F0 > 0.22 {
			if self.L1_F0 <= 1.44 {
				if self.L1_F0 <= 0.78 {
					if self.L1_CLASS <= 13 {
						if self.CU_STRESS == 0 {
							if self.CU_CLASS <= 2 {
								x = -2.12;
							}
							if self.CU_CLASS > 2 {
								if self.L1_F0 <= 0.43 {
									if self.L1_CLASS <= 10 {
										if self.L1_CLASS <= 6 {
											if self.CU_CLASS <= 7 {
												x = 0.5;
											}
											if self.CU_CLASS > 7 {
												if self.CU_CLASS <= 8 {
													x = -0.11;
												}
												if self.CU_CLASS > 8 {
													x = 0.29;
												}
											}
										}
										if self.L1_CLASS > 6 {
											if self.CU_CURR_ACCENT == 0 {
												x = 0.34;
											}
											if self.CU_CURR_ACCENT == 1 {
												if self.CU_CLASS <= 13 {
													x = 0.75;
												}
												if self.CU_CLASS > 13 {
													if self.L1_CLASS <= 9 {
														if self.L1_CLASS <= 8 {
															x = 0.42;
														}
														if self.L1_CLASS > 8 {
															x = -0.02;
														}
													}
													if self.L1_CLASS > 9 {
														x = 0.73;
													}
												}
											}
										}
									}
									if self.L1_CLASS > 10 {
										if self.CU_CLASS <= 7 {
											x = 0.3;
										}
										if self.CU_CLASS > 7 {
											x = 0.07;
										}
									}
								}
								if self.L1_F0 > 0.43 {
									if self.CU_CLASS <= 7 {
										if self.CU_CURR_ACCENT == 0 {
											if self.CU_DURAT <= 3.79 {
												x = 0.59;
											}
											if self.CU_DURAT > 3.79 {
												x = -2.12;
											}
										}
										if self.CU_CURR_ACCENT == 1 {
											if self.L1_CLASS <= 9 {
												if self.L1_CLASS <= 6 {
													x = 0.66;
												}
												if self.L1_CLASS > 6 {
													x = 1.08;
												}
											}
											if self.L1_CLASS > 9 {
												x = 0.34;
											}
										}
									}
									if self.CU_CLASS > 7 {
										if self.CU_CLASS <= 8 {
											if self.CU_BEGIN <= 94.5 {
												x = 0.06;
											}
											if self.CU_BEGIN > 94.5 {
												if self.L1_CLASS <= 8 {
													x = 6.9;
												}
												if self.L1_CLASS > 8 {
													x = 0.24;
												}
											}
										}
										if self.CU_CLASS > 8 {
											if self.CU_DURAT <= -0.38 {
												x = 0.71;
											}
											if self.CU_DURAT > -0.38 {
												if self.CU_DURAT <= 1.53 {
													if self.L1_F0 <= 0.59 {
														x = 0.4;
													}
													if self.L1_F0 > 0.59 {
														if self.CU_CLASS <= 10 {
															x = 0.33;
														}
														if self.CU_CLASS > 10 {
															if self.CU_DURAT <= 0.34 {
																x = 0.65;
															}
															if self.CU_DURAT > 0.34 {
																x = 0.3;
															}
														}
													}
												}
												if self.CU_DURAT > 1.53 {
													if self.L1_CLASS <= 8 {
														x = 0.53;
													}
													if self.L1_CLASS > 8 {
														x = -1.87;
													}
												}
											}
										}
									}
								}
							}
						}
						if self.CU_STRESS == 1 {
							if self.CU_DURAT <= 0.86 {
								if self.CU_PREV_ACCENT == 0 {
									if self.CU_BEGIN <= 97.5 {
										if self.L1_F0 <= 0.43 {
											if self.CU_CURR_ACCENT == 0 {
												x = 0.52;
											}
											if self.CU_CURR_ACCENT == 1 {
												if self.CU_DURAT <= 0.69 {
													if self.CU_STRESS <= 1 {
														x = 0.96;
													}
													if self.CU_STRESS > 1 {
														x = 0.33;
													}
												}
												if self.CU_DURAT > 0.69 {
													x = 0.47;
												}
											}
										}
										if self.L1_F0 > 0.43 {
											if self.CU_BEGIN <= 3.5 {
												x = 1.5;
											}
											if self.CU_BEGIN > 3.5 {
												if self.L1_CLASS <= 11 {
													if self.CU_CURR_ACCENT == 0 {
														if self.CU_DURAT <= 0.23 {
															x = 0.65;
														}
														if self.CU_DURAT > 0.23 {
															x = 1.04;
														}
													}
													if self.CU_CURR_ACCENT == 1 {
														x = 1.05;
													}
												}
												if self.L1_CLASS > 11 {
													x = 0.77;
												}
											}
										}
									}
									if self.CU_BEGIN > 97.5 {
										x = -0.36;
									}
								}
								if self.CU_PREV_ACCENT == 1 {
									if self.L1_F0 <= 0.4 {
										if self.CU_CURR_ACCENT == 0 {
											x = 0.33;
										}
										if self.CU_CURR_ACCENT == 1 {
											x = 0.65;
										}
									}
									if self.L1_F0 > 0.4 {
										if self.CU_DURAT <= 0.55 {
											x = 0.76;
										}
										if self.CU_DURAT > 0.55 {
											if self.CU_BEGIN <= 95.5 {
												x = 0.53;
											}
											if self.CU_BEGIN > 95.5 {
												x = -0.55;
											}
										}
									}
								}
							}
							if self.CU_DURAT > 0.86 {
								if self.L1_CLASS <= 11 {
									if self.CU_PREV_ACCENT == 0 {
										if self.CU_BEGIN <= 3.5 {
											if self.CU_DURAT <= 1.2 {
												x = 1.0;
											}
											if self.CU_DURAT > 1.2 {
												x = 2.47;
											}
										}
										if self.CU_BEGIN > 3.5 {
											if self.CU_CLASS <= 14 {
												if self.CU_DURAT <= 1.35 {
													if self.L1_F0 <= 0.4 {
														x = 0.17;
													}
													if self.L1_F0 > 0.4 {
														x = 0.75;
													}
												}
												if self.CU_DURAT > 1.35 {
													x = -0.41;
												}
											}
											if self.CU_CLASS > 14 {
												if self.CU_DURAT <= 1.07 {
													x = 0.88;
												}
												if self.CU_DURAT > 1.07 {
													x = 0.49;
												}
											}
										}
									}
									if self.CU_PREV_ACCENT == 1 {
										if self.CU_DURAT <= 2.88 {
											if self.CU_CURR_ACCENT == 0 {
												if self.L1_CLASS <= 8 {
													x = 0.28;
												}
												if self.L1_CLASS > 8 {
													x = 1.11;
												}
											}
											if self.CU_CURR_ACCENT == 1 {
												if self.CU_BEGIN <= 86.5 {
													x = 0.04;
												}
												if self.CU_BEGIN > 86.5 {
													x = -0.54;
												}
											}
										}
										if self.CU_DURAT > 2.88 {
											x = 4.18;
										}
									}
								}
								if self.L1_CLASS > 11 {
									if self.CU_DURAT <= 2.36 {
										if self.L1_F0 <= 0.66 {
											x = -0.32;
										}
										if self.L1_F0 > 0.66 {
											x = 0.28;
										}
									}
									if self.CU_DURAT > 2.36 {
										x = 2.82;
									}
								}
							}
						}
					}
					if self.L1_CLASS > 13 {
						if self.CU_DURAT <= -0.74 {
							if self.L1_F0 <= 0.49 {
								x = 0.35;
							}
							if self.L1_F0 > 0.49 {
								x = 0.64;
							}
						}
						if self.CU_DURAT > -0.74 {
							if self.CU_CLASS <= 10 {
								if self.CU_CLASS <= 5 {
									x = -2.12;
								}
								if self.CU_CLASS > 5 {
									if self.L1_F0 <= 0.53 {
										if self.CU_BEGIN <= 96.5 {
											x = -0.18;
										}
										if self.CU_BEGIN > 96.5 {
											x = -0.74;
										}
									}
									if self.L1_F0 > 0.53 {
										if self.CU_DURAT <= 0.02 {
											x = -0.06;
										}
										if self.CU_DURAT > 0.02 {
											x = 0.16;
										}
									}
								}
							}
							if self.CU_CLASS > 10 {
								if self.CU_BEGIN <= 97.5 {
									if self.L1_F0 <= 0.53 {
										if self.CU_CURR_ACCENT == 0 {
											x = 0.32;
										}
										if self.CU_CURR_ACCENT == 1 {
											if self.CU_DURAT <= 1.26 {
												x = 0.07;
											}
											if self.CU_DURAT > 1.26 {
												if self.CU_PREV_ACCENT == 0 {
													x = -0.26;
												}
												if self.CU_PREV_ACCENT == 1 {
													x = -1.86;
												}
											}
										}
									}
									if self.L1_F0 > 0.53 {
										if self.CU_CURR_ACCENT == 0 {
											if self.CU_DURAT <= 0.45 {
												x = 0.69;
											}
											if self.CU_DURAT > 0.45 {
												if self.CU_CLASS <= 16 {
													x = 1.16;
												}
												if self.CU_CLASS > 16 {
													x = -0.86;
												}
											}
										}
										if self.CU_CURR_ACCENT == 1 {
											if self.CU_DURAT <= 0.62 {
												if self.CU_STRESS == 0 {
													x = 0.32;
												}
												if self.CU_STRESS == 1 {
													x = 1.15;
												}
											}
											if self.CU_DURAT > 0.62 {
												if self.L1_CLASS <= 15 {
													x = 0.05;
												}
												if self.L1_CLASS > 15 {
													x = -0.93;
												}
											}
										}
									}
								}
								if self.CU_BEGIN > 97.5 {
									if self.CU_DURAT <= 0.43 {
										x = -0.49;
									}
									if self.CU_DURAT > 0.43 {
										x = -1.92;
									}
								}
							}
						}
					}
				}
				if self.L1_F0 > 0.78 {
					if self.L1_CLASS <= 13 {
						if self.CU_DURAT <= 1.26 {
							if self.CU_STRESS == 0 {
								if self.L1_CLASS <= 6 {
									if self.CU_DURAT <= -0.46 {
										x = 1.05;
									}
									if self.CU_DURAT > -0.46 {
										if self.CU_CLASS <= 10 {
											if self.CU_BEGIN <= 3.5 {
												x = 1.04;
											}
											if self.CU_BEGIN > 3.5 {
												if self.CU_CLASS <= 7 {
													x = 0.7;
												}
												if self.CU_CLASS > 7 {
													if self.CU_CLASS <= 8 {
														x = 0.15;
													}
													if self.CU_CLASS > 8 {
														if self.CU_BEGIN <= 89.0 {
															x = 0.59;
														}
														if self.CU_BEGIN > 89.0 {
															if self.CU_BEGIN <= 97.5 {
																x = -0.15;
															}
															if self.CU_BEGIN > 97.5 {
																x = 2.72;
															}
														}
													}
												}
											}
										}
										if self.CU_CLASS > 10 {
											if self.CU_DURAT <= 0.16 {
												if self.CU_PREV_ACCENT == 0 {
													x = 1.06;
												}
												if self.CU_PREV_ACCENT == 1 {
													x = 0.75;
												}
											}
											if self.CU_DURAT > 0.16 {
												x = 0.59;
											}
										}
									}
								}
								if self.L1_CLASS > 6 {
									if self.L1_CLASS <= 10 {
										if self.L1_F0 <= 1.07 {
											if self.CU_CURR_ACCENT == 0 {
												x = 0.92;
											}
											if self.CU_CURR_ACCENT == 1 {
												if self.CU_CLASS <= 13 {
													if self.CU_DURAT <= 0.53 {
														x = 1.26;
													}
													if self.CU_DURAT > 0.53 {
														x = 4.27;
													}
												}
												if self.CU_CLASS > 13 {
													x = 0.9;
												}
											}
										}
										if self.L1_F0 > 1.07 {
											x = 1.32;
										}
									}
									if self.L1_CLASS > 10 {
										if self.CU_DURAT <= -0.69 {
											if self.L1_F0 <= 1.07 {
												x = 0.91;
											}
											if self.L1_F0 > 1.07 {
												x = 1.24;
											}
										}
										if self.CU_DURAT > -0.69 {
											if self.CU_CLASS <= 10 {
												x = 0.48;
											}
											if self.CU_CLASS > 10 {
												x = 0.82;
											}
										}
									}
								}
							}
							if self.CU_STRESS == 1 {
								if self.CU_DURAT <= 0.99 {
									if self.CU_BEGIN <= 4.5 {
										if self.CU_PREV_ACCENT == 0 {
											if self.L1_CLASS <= 9 {
												if self.CU_CURR_ACCENT == 0 {
													x = 1.37;
												}
												if self.CU_CURR_ACCENT == 1 {
													x = 2.0;
												}
											}
											if self.L1_CLASS > 9 {
												x = 2.24;
											}
										}
										if self.CU_PREV_ACCENT == 1 {
											x = 0.76;
										}
									}
									if self.CU_BEGIN > 4.5 {
										if self.CU_PREV_ACCENT == 0 {
											if self.CU_DURAT <= 0.95 {
												if self.CU_DURAT <= 0.88 {
													if self.L1_F0 <= 0.87 {
														x = 1.17;
													}
													if self.L1_F0 > 0.87 {
														if self.L1_CLASS <= 9 {
															if self.CU_CLASS <= 15 {
																if self.L1_CLASS <= 7 {
																	x = 1.27;
																}
																if self.L1_CLASS > 7 {
																	x = 1.57;
																}
															}
															if self.CU_CLASS > 15 {
																if self.L1_F0 <= 1.28 {
																	x = 1.87;
																}
																if self.L1_F0 > 1.28 {
																	x = 3.19;
																}
															}
														}
														if self.L1_CLASS > 9 {
															if self.CU_DURAT <= 0.3 {
																x = 1.47;
															}
															if self.CU_DURAT > 0.3 {
																x = 1.11;
															}
														}
													}
												}
												if self.CU_DURAT > 0.88 {
													if self.L1_F0 <= 1.0 {
														x = 0.34;
													}
													if self.L1_F0 > 1.0 {
														x = 1.2;
													}
												}
											}
											if self.CU_DURAT > 0.95 {
												x = 2.63;
											}
										}
										if self.CU_PREV_ACCENT == 1 {
											if self.CU_DURAT <= 0.51 {
												if self.L1_F0 <= 0.99 {
													x = 1.03;
												}
												if self.L1_F0 > 0.99 {
													if self.CU_DURAT <= 0.1 {
														x = 1.15;
													}
													if self.CU_DURAT > 0.1 {
														if self.CU_BEGIN <= 95.0 {
															if self.CU_BEGIN <= 91.5 {
																x = 1.44;
															}
															if self.CU_BEGIN > 91.5 {
																if self.CU_CLASS <= 14 {
																	x = 4.38;
																}
																if self.CU_CLASS > 14 {
																	x = 1.33;
																}
															}
														}
														if self.CU_BEGIN > 95.0 {
															x = -0.3;
														}
													}
												}
											}
											if self.CU_DURAT > 0.51 {
												if self.CU_BEGIN <= 58.5 {
													if self.CU_DURAT <= 0.97 {
														x = 0.66;
													}
													if self.CU_DURAT > 0.97 {
														x = 3.3;
													}
												}
												if self.CU_BEGIN > 58.5 {
													if self.L1_CLASS <= 6 {
														x = 0.54;
													}
													if self.L1_CLASS > 6 {
														if self.CU_DURAT <= 0.78 {
															if self.L1_F0 <= 0.81 {
																x = 4.3;
															}
															if self.L1_F0 > 0.81 {
																x = 1.55;
															}
														}
														if self.CU_DURAT > 0.78 {
															x = 0.91;
														}
													}
												}
											}
										}
									}
								}
								if self.CU_DURAT > 0.99 {
									if self.CU_BEGIN <= 4.5 {
										x = 1.52;
									}
									if self.CU_BEGIN > 4.5 {
										if self.CU_DURAT <= 1.04 {
											x = 0.07;
										}
										if self.CU_DURAT > 1.04 {
											x = 0.75;
										}
									}
								}
							}
						}
						if self.CU_DURAT > 1.26 {
							if self.CU_CLASS <= 8 {
								x = -2.12;
							}
							if self.CU_CLASS > 8 {
								if self.CU_BEGIN <= 62.5 {
									if self.CU_BEGIN <= 1.5 {
										x = 1.93;
									}
									if self.CU_BEGIN > 1.5 {
										x = 0.41;
									}
								}
								if self.CU_BEGIN > 62.5 {
									if self.L1_CLASS <= 11 {
										if self.CU_DURAT <= 1.54 {
											x = 0.65;
										}
										if self.CU_DURAT > 1.54 {
											x = -0.23;
										}
									}
									if self.L1_CLASS > 11 {
										x = -0.77;
									}
								}
							}
						}
					}
					if self.L1_CLASS > 13 {
						if self.CU_DURAT <= -0.74 {
							if self.L1_F0 <= 1.11 {
								x = 0.94;
							}
							if self.L1_F0 > 1.11 {
								x = 1.26;
							}
						}
						if self.CU_DURAT > -0.74 {
							if self.CU_CLASS <= 10 {
								if self.CU_CLASS <= 5 {
									if self.CU_BEGIN <= 95.5 {
										x = -2.12;
									}
									if self.CU_BEGIN > 95.5 {
										x = 1.12;
									}
								}
								if self.CU_CLASS > 5 {
									if self.CU_DURAT <= -0.46 {
										x = 0.4;
									}
									if self.CU_DURAT > -0.46 {
										if self.CU_CLASS <= 9 {
											if self.CU_DURAT <= -0.43 {
												x = -0.23;
											}
											if self.CU_DURAT > -0.43 {
												if self.CU_BEGIN <= 93.5 {
													x = 0.24;
												}
												if self.CU_BEGIN > 93.5 {
													x = -0.22;
												}
											}
										}
										if self.CU_CLASS > 9 {
											x = -0.0;
										}
									}
								}
							}
							if self.CU_CLASS > 10 {
								if self.CU_DURAT <= 0.98 {
									if self.CU_CURR_ACCENT == 0 {
										if self.CU_DURAT <= 0.87 {
											if self.L1_F0 <= 1.19 {
												if self.L1_CLASS <= 14 {
													if self.CU_DURAT <= 0.45 {
														x = 1.05;
													}
													if self.CU_DURAT > 0.45 {
														if self.CU_DURAT <= 0.54 {
															x = -1.63;
														}
														if self.CU_DURAT > 0.54 {
															x = 1.21;
														}
													}
												}
												if self.L1_CLASS > 14 {
													x = 0.71;
												}
											}
											if self.L1_F0 > 1.19 {
												x = 1.2;
											}
										}
										if self.CU_DURAT > 0.87 {
											x = -1.24;
										}
									}
									if self.CU_CURR_ACCENT == 1 {
										if self.L1_F0 <= 1.28 {
											if self.CU_BEGIN <= 96.5 {
												if self.CU_PREV_ACCENT == 0 {
													if self.CU_STRESS == 0 {
														x = 0.62;
													}
													if self.CU_STRESS == 1 {
														x = 1.84;
													}
												}
												if self.CU_PREV_ACCENT == 1 {
													x = 0.35;
												}
											}
											if self.CU_BEGIN > 96.5 {
												if self.CU_DURAT <= 0.28 {
													x = -0.25;
												}
												if self.CU_DURAT > 0.28 {
													x = -1.92;
												}
											}
										}
										if self.L1_F0 > 1.28 {
											x = 0.9;
										}
									}
								}
								if self.CU_DURAT > 0.98 {
									x = -0.79;
								}
							}
						}
					}
				}
			}
			if self.L1_F0 > 1.44 {
				if self.L1_F0 <= 3.25 {
					if self.CU_DURAT <= -0.71 {
						if self.L1_F0 <= 2.07 {
							if self.L1_F0 <= 1.75 {
								x = 1.57;
							}
							if self.L1_F0 > 1.75 {
								x = 1.88;
							}
						}
						if self.L1_F0 > 2.07 {
							if self.L1_F0 <= 2.55 {
								if self.CU_CURR_ACCENT == 0 {
									x = 2.27;
								}
								if self.CU_CURR_ACCENT == 1 {
									x = 1.38;
								}
							}
							if self.L1_F0 > 2.55 {
								x = 2.79;
							}
						}
					}
					if self.CU_DURAT > -0.71 {
						if self.CU_CLASS <= 10 {
							if self.CU_CLASS <= 2 {
								x = -2.12;
							}
							if self.CU_CLASS > 2 {
								if self.L1_CLASS <= 11 {
									if self.L1_F0 <= 1.77 {
										if self.CU_DURAT <= -0.25 {
											if self.CU_CLASS <= 7 {
												if self.L1_CLASS <= 10 {
													x = 1.82;
												}
												if self.L1_CLASS > 10 {
													x = 0.82;
												}
											}
											if self.CU_CLASS > 7 {
												x = 0.87;
											}
										}
										if self.CU_DURAT > -0.25 {
											x = 0.7;
										}
									}
									if self.L1_F0 > 1.77 {
										if self.CU_DURAT <= -0.44 {
											x = 1.52;
										}
										if self.CU_DURAT > -0.44 {
											if self.L1_CLASS <= 6 {
												x = 0.99;
											}
											if self.L1_CLASS > 6 {
												if self.CU_CLASS <= 9 {
													x = 1.69;
												}
												if self.CU_CLASS > 9 {
													x = 0.79;
												}
											}
										}
									}
								}
								if self.L1_CLASS > 11 {
									if self.L1_F0 <= 2.17 {
										if self.CU_DURAT <= -0.4 {
											x = 0.7;
										}
										if self.CU_DURAT > -0.4 {
											x = 0.43;
										}
									}
									if self.L1_F0 > 2.17 {
										if self.CU_DURAT <= -0.44 {
											x = 1.32;
										}
										if self.CU_DURAT > -0.44 {
											if self.CU_BEGIN <= 94.5 {
												if self.L1_F0 <= 2.78 {
													x = 0.73;
												}
												if self.L1_F0 > 2.78 {
													x = 1.09;
												}
											}
											if self.CU_BEGIN > 94.5 {
												x = -1.08;
											}
										}
									}
								}
							}
						}
						if self.CU_CLASS > 10 {
							if self.L1_F0 <= 2.28 {
								if self.CU_STRESS == 0 {
									if self.CU_DURAT <= 0.58 {
										if self.L1_CLASS <= 13 {
											if self.L1_CLASS <= 6 {
												if self.CU_DURAT <= -0.05 {
													x = 1.45;
												}
												if self.CU_DURAT > -0.05 {
													x = 1.1;
												}
											}
											if self.L1_CLASS > 6 {
												if self.CU_CLASS <= 12 {
													if self.L1_F0 <= 1.75 {
														x = 1.68;
													}
													if self.L1_F0 > 1.75 {
														x = 2.31;
													}
												}
												if self.CU_CLASS > 12 {
													x = 1.53;
												}
											}
										}
										if self.L1_CLASS > 13 {
											if self.L1_F0 <= 1.81 {
												x = 1.08;
											}
											if self.L1_F0 > 1.81 {
												if self.CU_BEGIN <= 18.5 {
													x = 1.68;
												}
												if self.CU_BEGIN > 18.5 {
													if self.CU_PREV_ACCENT == 0 {
														x = 1.35;
													}
													if self.CU_PREV_ACCENT == 1 {
														if self.CU_DURAT <= 0.41 {
															if self.CU_DURAT <= -0.21 {
																x = 0.65;
															}
															if self.CU_DURAT > -0.21 {
																x = 1.32;
															}
														}
														if self.CU_DURAT > 0.41 {
															x = -0.52;
														}
													}
												}
											}
										}
									}
									if self.CU_DURAT > 0.58 {
										if self.CU_CLASS <= 14 {
											if self.L1_CLASS <= 6 {
												x = 0.53;
											}
											if self.L1_CLASS > 6 {
												x = 1.52;
											}
										}
										if self.CU_CLASS > 14 {
											if self.L1_CLASS <= 15 {
												if self.CU_PREV_ACCENT == 0 {
													x = 0.51;
												}
												if self.CU_PREV_ACCENT == 1 {
													x = -0.41;
												}
											}
											if self.L1_CLASS > 15 {
												x = -1.34;
											}
										}
									}
								}
								if self.CU_STRESS == 1 {
									if self.CU_DURAT <= 0.81 {
										if self.CU_BEGIN <= 4.5 {
											if self.L1_F0 <= 2.01 {
												x = 2.21;
											}
											if self.L1_F0 > 2.01 {
												x = 2.87;
											}
										}
										if self.CU_BEGIN > 4.5 {
											if self.L1_CLASS <= 6 {
												x = 1.28;
											}
											if self.L1_CLASS > 6 {
												if self.L1_F0 <= 1.93 {
													if self.CU_BEGIN <= 98.5 {
														if self.L1_CLASS <= 11 {
															if self.CU_PREV_ACCENT == 0 {
																x = 2.03;
															}
															if self.CU_PREV_ACCENT == 1 {
																x = 1.67;
															}
														}
														if self.L1_CLASS > 11 {
															if self.CU_DURAT <= 0.32 {
																x = 1.85;
															}
															if self.CU_DURAT > 0.32 {
																x = 1.26;
															}
														}
													}
													if self.CU_BEGIN > 98.5 {
														x = -1.54;
													}
												}
												if self.L1_F0 > 1.93 {
													x = 2.17;
												}
											}
										}
									}
									if self.CU_DURAT > 0.81 {
										if self.L1_CLASS <= 11 {
											if self.CU_DURAT <= 1.39 {
												if self.L1_F0 <= 2.18 {
													if self.CU_BEGIN <= 3.5 {
														x = 2.85;
													}
													if self.CU_BEGIN > 3.5 {
														if self.L1_CLASS <= 6 {
															x = 0.82;
														}
														if self.L1_CLASS > 6 {
															if self.L1_F0 <= 1.5 {
																x = 0.6;
															}
															if self.L1_F0 > 1.5 {
																if self.CU_DURAT <= 1.19 {
																	x = 1.71;
																}
																if self.CU_DURAT > 1.19 {
																	x = 2.71;
																}
															}
														}
													}
												}
												if self.L1_F0 > 2.18 {
													x = 6.36;
												}
											}
											if self.CU_DURAT > 1.39 {
												if self.L1_CLASS <= 7 {
													x = 0.57;
												}
												if self.L1_CLASS > 7 {
													if self.CU_DURAT <= 2.64 {
														x = 1.53;
													}
													if self.CU_DURAT > 2.64 {
														x = -0.55;
													}
												}
											}
										}
										if self.L1_CLASS > 11 {
											if self.L1_F0 <= 1.75 {
												if self.L1_F0 <= 1.6 {
													x = 0.42;
												}
												if self.L1_F0 > 1.6 {
													x = 1.87;
												}
											}
											if self.L1_F0 > 1.75 {
												if self.L1_F0 <= 2.13 {
													if self.CU_BEGIN <= 3.5 {
														x = 1.52;
													}
													if self.CU_BEGIN > 3.5 {
														x = -0.44;
													}
												}
												if self.L1_F0 > 2.13 {
													x = 2.03;
												}
											}
										}
									}
								}
							}
							if self.L1_F0 > 2.28 {
								if self.CU_STRESS == 0 {
									if self.CU_DURAT <= 0.82 {
										if self.L1_F0 <= 2.65 {
											if self.CU_CLASS <= 13 {
												if self.CU_DURAT <= -0.21 {
													x = 1.86;
												}
												if self.CU_DURAT > -0.21 {
													x = 1.25;
												}
											}
											if self.CU_CLASS > 13 {
												x = 1.86;
											}
										}
										if self.L1_F0 > 2.65 {
											if self.CU_DURAT <= 0.76 {
												if self.CU_DURAT <= 0.48 {
													x = 2.06;
												}
												if self.CU_DURAT > 0.48 {
													x = 1.13;
												}
											}
											if self.CU_DURAT > 0.76 {
												x = 4.51;
											}
										}
									}
									if self.CU_DURAT > 0.82 {
										if self.CU_DURAT <= 1.57 {
											if self.CU_BEGIN <= 80.0 {
												x = 0.75;
											}
											if self.CU_BEGIN > 80.0 {
												x = -0.98;
											}
										}
										if self.CU_DURAT > 1.57 {
											if self.CU_CLASS <= 15 {
												x = 3.02;
											}
											if self.CU_CLASS > 15 {
												x = -0.45;
											}
										}
									}
								}
								if self.CU_STRESS == 1 {
									if self.CU_DURAT <= 1.12 {
										if self.L1_F0 <= 2.83 {
											if self.CU_CLASS <= 14 {
												if self.CU_DURAT <= 0.75 {
													x = 2.48;
												}
												if self.CU_DURAT > 0.75 {
													if self.CU_BEGIN <= 3.5 {
														x = 2.53;
													}
													if self.CU_BEGIN > 3.5 {
														x = 0.34;
													}
												}
											}
											if self.CU_CLASS > 14 {
												if self.CU_DURAT <= 1.1 {
													x = 2.8;
												}
												if self.CU_DURAT > 1.1 {
													x = 5.33;
												}
											}
										}
										if self.L1_F0 > 2.83 {
											x = 3.03;
										}
									}
									if self.CU_DURAT > 1.12 {
										x = 1.37;
									}
								}
							}
						}
					}
				}
				if self.L1_F0 > 3.25 {
					if self.L1_F0 <= 4.77 {
						if self.CU_CLASS <= 6 {
							if self.L1_F0 <= 3.9 {
								x = 3.49;
							}
							if self.L1_F0 > 3.9 {
								x = 4.32;
							}
						}
						if self.CU_CLASS > 6 {
							if self.CU_CLASS <= 13 {
								if self.CU_CLASS <= 9 {
									if self.L1_F0 <= 4.16 {
										x = 1.61;
									}
									if self.L1_F0 > 4.16 {
										if self.CU_BEGIN <= 82.5 {
											x = 3.0;
										}
										if self.CU_BEGIN > 82.5 {
											if self.CU_BEGIN <= 95.0 {
												x = 0.34;
											}
											if self.CU_BEGIN > 95.0 {
												x = 3.66;
											}
										}
									}
								}
								if self.CU_CLASS > 9 {
									if self.L1_F0 <= 3.84 {
										x = 2.16;
									}
									if self.L1_F0 > 3.84 {
										if self.CU_DURAT <= -0.53 {
											x = 4.14;
										}
										if self.CU_DURAT > -0.53 {
											if self.L1_CLASS <= 14 {
												x = 2.29;
											}
											if self.L1_CLASS > 14 {
												x = 3.64;
											}
										}
									}
								}
							}
							if self.CU_CLASS > 13 {
								if self.CU_STRESS == 0 {
									if self.CU_BEGIN <= 5.5 {
										if self.CU_CLASS <= 16 {
											x = 0.57;
										}
										if self.CU_CLASS > 16 {
											x = 2.58;
										}
									}
									if self.CU_BEGIN > 5.5 {
										x = 3.13;
									}
								}
								if self.CU_STRESS == 1 {
									if self.L1_CLASS <= 6 {
										if self.CU_DURAT <= 0.52 {
											if self.L1_F0 <= 3.45 {
												x = 2.44;
											}
											if self.L1_F0 > 3.45 {
												x = 4.61;
											}
										}
										if self.CU_DURAT > 0.52 {
											x = 1.36;
										}
									}
									if self.L1_CLASS > 6 {
										if self.L1_CLASS <= 8 {
											if self.CU_BEGIN <= 12.5 {
												x = 3.54;
											}
											if self.CU_BEGIN > 12.5 {
												x = 6.04;
											}
										}
										if self.L1_CLASS > 8 {
											x = 3.88;
										}
									}
								}
							}
						}
					}
					if self.L1_F0 > 4.77 {
						if self.L1_F0 <= 6.8 {
							if self.CU_DURAT <= -0.39 {
								if self.CU_CLASS <= 10 {
									x = 5.51;
								}
								if self.CU_CLASS > 10 {
									x = 7.45;
								}
							}
							if self.CU_DURAT > -0.39 {
								if self.CU_CURR_ACCENT == 0 {
									if self.L1_F0 <= 6.32 {
										if self.CU_DURAT <= -0.11 {
											if self.L1_F0 <= 5.39 {
												x = 4.76;
											}
											if self.L1_F0 > 5.39 {
												x = 2.7;
											}
										}
										if self.CU_DURAT > -0.11 {
											x = 6.12;
										}
									}
									if self.L1_F0 > 6.32 {
										if self.L1_F0 <= 6.62 {
											x = 0.06;
										}
										if self.L1_F0 > 6.62 {
											x = 6.27;
										}
									}
								}
								if self.CU_CURR_ACCENT == 1 {
									if self.CU_DURAT <= 1.04 {
										if self.CU_CLASS <= 13 {
											if self.CU_DURAT <= 0.33 {
												if self.CU_DURAT <= -0.03 {
													x = 2.49;
												}
												if self.CU_DURAT > -0.03 {
													x = 0.38;
												}
											}
											if self.CU_DURAT > 0.33 {
												if self.CU_BEGIN <= 32.0 {
													x = 6.39;
												}
												if self.CU_BEGIN > 32.0 {
													x = 3.34;
												}
											}
										}
										if self.CU_CLASS > 13 {
											if self.CU_DURAT <= -0.03 {
												x = 3.33;
											}
											if self.CU_DURAT > -0.03 {
												x = 6.09;
											}
										}
									}
									if self.CU_DURAT > 1.04 {
										x = 0.56;
									}
								}
							}
						}
						if self.L1_F0 > 6.8 {
							if self.CU_BEGIN <= 91.5 {
								if self.L1_F0 <= 7.93 {
									x = 6.72;
								}
								if self.L1_F0 > 7.93 {
									x = 8.59;
								}
							}
							if self.CU_BEGIN > 91.5 {
								if self.L1_CLASS <= 11 {
									x = 4.51;
								}
								if self.L1_CLASS > 11 {
									x = 7.51;
								}
							}
						}
					}
				}
			}
		}
		self.F0 = x;
	}
	fn duration_model(&mut self) {
		let mut x: f32 = 0.0;
		if self.CU_CLASS <= 5 {
			if self.CU_CLASS <= 2 {
				if self.CU_BEGIN <= 41.5 {
					if self.R2_CLASS <= 13 {
						if self.L1_CLASS <= 9 {
							if self.R2_CLASS <= 10 {
								x = 2.83;
							}
							if self.R2_CLASS > 10 {
								x = 1.84;
							}
						}
						if self.L1_CLASS > 9 {
							x = 1.55;
						}
					}
					if self.R2_CLASS > 13 {
						x = 2.26;
					}
				}
				if self.CU_BEGIN > 41.5 {
					if self.CU_BEGIN <= 49.5 {
						if self.R2_CLASS <= 9 {
							x = 4.52;
						}
						if self.R2_CLASS > 9 {
							x = 2.92;
						}
					}
					if self.CU_BEGIN > 49.5 {
						if self.CU_BEGIN <= 85.5 {
							x = 2.33;
						}
						if self.CU_BEGIN > 85.5 {
							if self.R1_CLASS <= 13 {
								if self.R1_CLASS <= 12 {
									x = 2.93;
								}
								if self.R1_CLASS > 12 {
									x = 5.8;
								}
							}
							if self.R1_CLASS > 13 {
								x = 2.11;
							}
						}
					}
				}
			}
			if self.CU_CLASS > 2 {
				if self.CU_CLASS <= 3 {
					if self.R1_CLASS <= 7 {
						x = 6.24;
					}
					if self.R1_CLASS > 7 {
						if self.CU_BEGIN <= 32.5 {
							if self.R1_CLASS <= 11 {
								x = 5.76;
							}
							if self.R1_CLASS > 11 {
								if self.L1_CLASS <= 10 {
									if self.R2_CLASS <= 7 {
										x = 11.32;
									}
									if self.R2_CLASS > 7 {
										x = 6.35;
									}
								}
								if self.L1_CLASS > 10 {
									x = 8.78;
								}
							}
						}
						if self.CU_BEGIN > 32.5 {
							if self.CU_BEGIN <= 51.5 {
								if self.R1_CLASS <= 14 {
									if self.CU_BEGIN <= 44.5 {
										x = 6.74;
									}
									if self.CU_BEGIN > 44.5 {
										x = 7.82;
									}
								}
								if self.R1_CLASS > 14 {
									x = 9.55;
								}
							}
							if self.CU_BEGIN > 51.5 {
								if self.R1_CLASS <= 14 {
									if self.L1_CLASS <= 10 {
										x = 6.67;
									}
									if self.L1_CLASS > 10 {
										x = 7.59;
									}
								}
								if self.R1_CLASS > 14 {
									if self.L2_CLASS <= 14 {
										x = 6.78;
									}
									if self.L2_CLASS > 14 {
										x = 4.35;
									}
								}
							}
						}
					}
				}
				if self.CU_CLASS > 3 {
					if self.L2_CLASS <= 12 {
						if self.CU_BEGIN <= 97.5 {
							x = 4.76;
						}
						if self.CU_BEGIN > 97.5 {
							if self.L1_CLASS <= 12 {
								x = 12.4;
							}
							if self.L1_CLASS > 12 {
								x = 7.14;
							}
						}
					}
					if self.L2_CLASS > 12 {
						x = 4.28;
					}
				}
			}
		}
		if self.CU_CLASS > 5 {
			if self.CU_CLASS <= 6 {
				x = -0.75;
			}
			if self.CU_CLASS > 6 {
				if self.CU_STRESS == 0 {
					if self.CU_CLASS <= 11 {
						if self.CU_CLASS <= 8 {
							if self.R1_CLASS <= 5 {
								x = 0.05;
							}
							if self.R1_CLASS > 5 {
								if self.L1_CLASS <= 6 {
									if self.L2_CLASS <= 11 {
										x = -0.3;
									}
									if self.L2_CLASS > 11 {
										x = -0.01;
									}
								}
								if self.L1_CLASS > 6 {
									if self.L2_CLASS <= 8 {
										x = -0.22;
									}
									if self.L2_CLASS > 8 {
										x = -0.34;
									}
								}
							}
						}
						if self.CU_CLASS > 8 {
							if self.CU_CLASS <= 9 {
								if self.CU_CURR_ACCENT == 0 {
									if self.L1_CLASS <= 6 {
										x = -0.25;
									}
									if self.L1_CLASS > 6 {
										x = 0.06;
									}
								}
								if self.CU_CURR_ACCENT == 1 {
									if self.L1_CLASS <= 7 {
										x = 0.24;
									}
									if self.L1_CLASS > 7 {
										x = 0.1;
									}
								}
							}
							if self.CU_CLASS > 9 {
								if self.R1_CLASS <= 6 {
									if self.R1_CLASS <= 5 {
										x = 0.15;
									}
									if self.R1_CLASS > 5 {
										x = -0.12;
									}
								}
								if self.R1_CLASS > 6 {
									if self.CU_CURR_ACCENT == 0 {
										x = -0.37;
									}
									if self.CU_CURR_ACCENT == 1 {
										x = -0.19;
									}
								}
							}
						}
					}
					if self.CU_CLASS > 11 {
						if self.R1_CLASS <= 5 {
							x = 0.76;
						}
						if self.R1_CLASS > 5 {
							if self.L1_CLASS <= 13 {
								if self.R1_CLASS <= 6 {
									if self.CU_CURR_ACCENT == 0 {
										x = -0.01;
									}
									if self.CU_CURR_ACCENT == 1 {
										x = 0.4;
									}
								}
								if self.R1_CLASS > 6 {
									if self.L1_CLASS <= 6 {
										x = 0.05;
									}
									if self.L1_CLASS > 6 {
										x = -0.09;
									}
								}
							}
							if self.L1_CLASS > 13 {
								if self.L1_CLASS <= 14 {
									x = 0.3;
								}
								if self.L1_CLASS > 14 {
									if self.CU_CLASS <= 12 {
										x = 0.3;
									}
									if self.CU_CLASS > 12 {
										x = 0.08;
									}
								}
							}
						}
					}
				}
				if self.CU_STRESS == 1 {
					if self.CU_CURR_ACCENT == 0 {
						if self.L1_CLASS <= 8 {
							if self.CU_CLASS <= 14 {
								if self.R1_CLASS <= 12 {
									if self.L1_CLASS <= 4 {
										x = 0.24;
									}
									if self.L1_CLASS > 4 {
										x = 0.51;
									}
								}
								if self.R1_CLASS > 12 {
									x = -0.19;
								}
							}
							if self.CU_CLASS > 14 {
								if self.R1_CLASS <= 14 {
									x = 0.54;
								}
								if self.R1_CLASS > 14 {
									if self.R2_CLASS <= 7 {
										x = 1.22;
									}
									if self.R2_CLASS > 7 {
										x = -0.14;
									}
								}
							}
						}
						if self.L1_CLASS > 8 {
							if self.R1_CLASS <= 5 {
								x = 1.5;
							}
							if self.R1_CLASS > 5 {
								x = 0.23;
							}
						}
					}
					if self.CU_CURR_ACCENT == 1 {
						if self.R2_CLASS <= 6 {
							if self.CU_CLASS <= 14 {
								if self.R1_CLASS <= 16 {
									if self.L1_CLASS <= 11 {
										x = 0.99;
									}
									if self.L1_CLASS > 11 {
										x = 0.7;
									}
								}
								if self.R1_CLASS > 16 {
									x = 0.32;
								}
							}
							if self.CU_CLASS > 14 {
								if self.R1_CLASS <= 7 {
									x = 0.78;
								}
								if self.R1_CLASS > 7 {
									if self.CU_CLASS <= 15 {
										x = 1.19;
									}
									if self.CU_CLASS > 15 {
										x = 0.85;
									}
								}
							}
						}
						if self.R2_CLASS > 6 {
							if self.R1_CLASS <= 6 {
								if self.R1_CLASS <= 5 {
									x = 1.91;
								}
								if self.R1_CLASS > 5 {
									x = 1.09;
								}
							}
							if self.R1_CLASS > 6 {
								if self.R1_CLASS <= 16 {
									if self.R2_CLASS <= 10 {
										x = 0.68;
									}
									if self.R2_CLASS > 10 {
										x = 0.45;
									}
								}
								if self.R1_CLASS > 16 {
									if self.CU_CLASS <= 14 {
										x = -0.02;
									}
									if self.CU_CLASS > 14 {
										x = 0.36;
									}
								}
							}
						}
					}
				}
			}
		}
		self.CU_DURAT = x;
	}
	pub fn step(&mut self, phn: u8, strs: u8, prev_accent: u8, curr_accent: u8, p_id: usize, p_total: usize) {
		// update the model
		self.adv(phn, strs, prev_accent, curr_accent, p_id, p_total);
		
		// run the duration tree
		self.duration_model();
		//self.adjust();
		
		// run the pitch tree
		self.pitch_model();
	}
}
