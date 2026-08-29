// bring in cbuf so lexer can use it
use crate::_utils::cbuf;

// text-number-homograph normalizer AIO (all-in-one)
pub struct engine {
	scratch_pad: cbuf,
	punct_prev:  bool,
	digit_prev:  bool,
	is_plural:   bool,
	past_number: u64,
	number:      u64,
}
impl engine {
	pub fn new() -> engine {
		engine {
			scratch_pad: cbuf::new(),
			punct_prev:  false,
			digit_prev:  false,
			is_plural:   false,
			past_number: 0,
			number:      0,
		}
	}
	pub fn is_not_empty(&mut self) -> bool {
		self.scratch_pad.is_not_empty()
	}
	pub fn read(&mut self) -> u8 {
		let cc = self.scratch_pad.read();
		self.scratch_pad.advance();
		cc
	}

	// NUMBER TO WORD PART
	fn reset_ntw(&mut self) {
		self.past_number = 0;
		self.number = 0;
	}
	fn is_number(&mut self, x: u8) -> bool {
		if x >= 0x30 && x <= 0x39 { return true; }
		false
	}
	fn consume_digit(&mut self, x: u8) {
		// consume digit for NTW
		if self.is_number(x) {
			let xx = (x - 0x30) % 10;
			self.number = (self.number * 10) + xx as u64;
			self.past_number = self.number;
		}
	}
	fn process_number(&mut self, x: u64, zero_oh: bool, is_ordinal: bool) {
		// number to word normalization
		if x < 20 {
			match x {
				1 => {
					if is_ordinal {
						self.scratch_pad.write_text(" first ");
					} else {
						self.scratch_pad.write_text(" one ");
					}
				},
				2 => {
					if is_ordinal {
						self.scratch_pad.write_text(" second ");
					} else {
						self.scratch_pad.write_text(" two ");
					}
				},
				3 => {
					if is_ordinal {
						self.scratch_pad.write_text(" third ");
					} else {
						self.scratch_pad.write_text(" three ");
					}
				},
				4|6|7|10|11|13|14|15|16|17|18|19 => {
					if x == 4 || x == 14 {
						self.scratch_pad.write_text(" four");
					} else if x == 6 || x == 16 {
						self.scratch_pad.write_text(" six");
					} else if x == 7 || x == 17 {
						self.scratch_pad.write_text(" seven");
					} else if x == 10 {
						self.scratch_pad.write_text(" ten");
					} else if x == 11 {
						self.scratch_pad.write_text(" eleven");
					}
					if x > 12 {
						if x == 13 {
							self.scratch_pad.write_text(" thir");
						} else if x == 15 {
							self.scratch_pad.write_text(" fif");
						} else if x == 18 {
							self.scratch_pad.write_text(" eigh");
						} else if x == 19 {
							self.scratch_pad.write_text(" nine");
						}
						self.scratch_pad.write_text("teen");
					}
					if is_ordinal {
						self.scratch_pad.write_text("th");
					}
					self.scratch_pad.write_text(" ");
				},
				5|12 => {
					if x == 5 {
						self.scratch_pad.write_text(" fi");
					} else if x == 12 {
						self.scratch_pad.write_text(" twel");
					}
					if is_ordinal {
						self.scratch_pad.write_text("fth");
					} else {
						self.scratch_pad.write_text("ve");
					}
					self.scratch_pad.write_text(" ");
				},
				8 => {
					self.scratch_pad.write_text(" eight");
					if is_ordinal {
						self.scratch_pad.write_text("h");
					}
					self.scratch_pad.write_text(" ");
				},
				9 => {
					self.scratch_pad.write_text(" nin");
					if is_ordinal {
						self.scratch_pad.write_text("th");
					} else {
						self.scratch_pad.write_text("e");
					}
					self.scratch_pad.write_text(" ");
				},
				_ => {
					if zero_oh {
						self.scratch_pad.write_text(" oh ");
					} else {
						self.scratch_pad.write_text(" zero");
						if is_ordinal {
							self.scratch_pad.write_text("th");
						}
						self.scratch_pad.write_text(" ");
					}
				},
			}
		} else if x < 100 {
			let tens = x / 10;
			let ones = x % 10;

			if tens == 2 {
				self.scratch_pad.write_text(" twen");
			} else if tens == 3 {
				self.scratch_pad.write_text(" thir");
			} else if tens == 4 {
				self.scratch_pad.write_text(" for");
			} else if tens == 5 {
				self.scratch_pad.write_text(" fif");
			} else if tens == 6 {
				self.scratch_pad.write_text(" six");
			} else if tens == 7 {
				self.scratch_pad.write_text(" seven");
			} else if tens == 8 {
				self.scratch_pad.write_text(" eigh");
			} else if tens == 9 {
				self.scratch_pad.write_text(" nine");
			}

			if is_ordinal && ones == 0 {
				self.scratch_pad.write_text("tieth ");
			} else {
				// do the 'ones' recursively
				self.scratch_pad.write_text("ty");
				if ones > 0 {
					self.process_number(ones, zero_oh, is_ordinal);
				} else {
					self.scratch_pad.write_text(" ");
				}
			}

		} else if x < 1000 {
			let hundreds = x / 100;
			let unders   = x % 100;
			self.process_number(hundreds, false, false);
			self.scratch_pad.write_text("hundred");
			if is_ordinal && unders == 0 {
				self.scratch_pad.write_text("th ");
			} else if unders > 0 {
				self.process_number(unders, zero_oh, is_ordinal);
			}
			self.scratch_pad.write_text(" ");
		} else if x < 1000000 {
			let thousands = x / 1000;
			let unders    = x % 1000;
			self.process_number(thousands, false, false);
			self.scratch_pad.write_text("thousand");
			if is_ordinal && unders == 0 {
				self.scratch_pad.write_text("th ");
			} else if unders > 0 {
				self.process_number(unders, zero_oh, is_ordinal);
			}
			self.scratch_pad.write_text(" ");
		} else if x < 1000000000 {
			let millions = x / 1000000;
			let unders   = x % 1000000;
			self.process_number(millions, false, false);
			self.scratch_pad.write_text("million");
			if is_ordinal && unders == 0 {
				self.scratch_pad.write_text("th ");
			} else if unders > 0 {
				self.process_number(unders, zero_oh, is_ordinal);
			}
			self.scratch_pad.write_text(" ");
		} else if x < 1000000000000 {
			let billions = x / 1000000000;
			let unders   = x % 1000000000;
			self.process_number(billions, false, false);
			self.scratch_pad.write_text("billion");
			if is_ordinal && unders == 0 {
				self.scratch_pad.write_text("th ");
			} else if unders > 0 {
				self.process_number(unders, zero_oh, is_ordinal);
			}
			self.scratch_pad.write_text(" ");
		} else if x < 1000000000000000 {
			let trillions = x / 1000000000000;
			let unders    = x % 1000000000000;
			self.process_number(trillions, false, false);
			self.scratch_pad.write_text("trillion");
			if is_ordinal && unders == 0 {
				self.scratch_pad.write_text("th ");
			} else if unders > 0 {
				self.process_number(unders, zero_oh, is_ordinal);
			}
			self.scratch_pad.write_text(" ");
		} else if x < 1000000000000000000 {
			let quadrillions = x / 1000000000000000;
			let unders       = x % 1000000000000000;
			self.process_number(quadrillions, false, false);
			self.scratch_pad.write_text("quadrillion");
			if is_ordinal && unders == 0 {
				self.scratch_pad.write_text("th ");
			} else if unders > 0 {
				self.process_number(unders, zero_oh, is_ordinal);
			}
			self.scratch_pad.write_text(" ");
		}
	}
	fn ntw(&mut self, zero_oh: bool, is_ordinal: bool) {
		self.is_plural = false;
		if self.number > 1 { self.is_plural = true; }
		self.process_number(self.number, zero_oh, is_ordinal);
	}

	// TEXT-HOMOGRAPH NORMALIZER PART
	pub fn lex(&mut self, buf: &mut cbuf) -> bool {
		let mut text_done: bool  = false;
		let mut yymarker: usize = 0;

		self.punct_prev = false;
		self.digit_prev = false;
		self.is_plural  = false;

		// Final tag variables available in semantic action.
		/*!svars:re2c format = '#[allow(unused_mut)]\nlet mut @@;\n'; */

		// Intermediate tag variables used by the lexer (must be autogenerated).
		/*!stags:re2c format = 'let mut @@ = 2050;'; */

		%{
			/*-----------------------------------------------*/
			/* GENERIC API IS BEING USED SO WE CAN INTERFACE */
			/* OUR RING BUFFER SETUP WITH THIS LEXER CODE    */
			/*-----------------------------------------------*/
			re2c:api            = generic;
			re2c:YYPEEK         = "buf.read()";
			re2c:YYSKIP         = "buf.advance();";
			re2c:YYBACKUP       = "yymarker = buf.getcurs();";
			re2c:YYRESTORE      = "buf.setcurs(yymarker);";
			re2c:YYRESTORETAG   = "buf.setcurs(@@{tag});";
			re2c:YYSTAGP        = "@@{tag} = buf.getcurs();";
			re2c:YYSTAGN        = "@@{tag} = 2050";
			re2c:YYSHIFTSTAG    = "@@{tag} = (@@{tag} as isize + @@{shift}) as usize; buf.setcurs(@@{tag})";

			re2c:define:YYCTYPE = u8;
			re2c:flags:utf-8    = 1;
			re2c:yyfill:enable  = 0;
			re2c:indent:top     = 1;
			re2c:tags           = 1;

			/* NORMAL GRAMMAR */
			space       = " " | "\x08" | "\x0A";
			comma       = ",";
			period      = ".";
			dquote      = "\x22" | "\u201c" | "\u201d" | "\u201e" | "\u201f";
			squote      = "\x27" | "\u2018" | "\u2019" | "\u201a" | "\u201b";
			exc_point   = "!" | "\u203c" | "\u203d" | "\u2049";
			ques_mark   = "?" | "\u2047" | "\u2048";
			fun_tag     = "__";
			letter      = [a-zA-Z];
			vowel       = [aeiou];
			wbound      = space;
			state_start = comma [ ]+;
			state_end   = period? space;

			/* GRAMMAR FOR NUMBERS */
			digit       = [0-9];
			ordinal_tag = "st" | "nd" | "rd" | "th";
			num_delimit = comma | '\x27' | '_';
			integer_all = num_delimit? digit+;
			whole_num   = integer_all+;
			decimal     = "." digit+;
			plusminus   = "+" | "-";
			numspecial  = "#" | "$";

			/* GRAMMAR FOR PHONE NUMBERS */
			pnum_sep    = "-" | ".";
			area_sep    = ") " | pnum_sep;
			area_code   = "("? digit{3} area_sep;
			phone_num   = area_code? digit{3} pnum_sep digit{4};

			/*---------------------------------*/
			/*  HOMOGRAPHS TO DEAL WITH FIRST  */
			/*                                 */
			/* clever use of case-sensitivity  */
			/* to mark and retrieve homographs */
			/*---------------------------------*/

			" for" " "+ vowel {
				// "fer" becomes "for" before word-initial vowels
				buf.recede();
				self.scratch_pad.write_text(" FOR ");
				break;
			}
			" the" " "+ vowel {
				// change "the" to "thee" if next word
				// starts with a vowel (e.g., "thee apple")
				buf.recede();
				self.scratch_pad.write_text(" thee ");
				break;
			}
			"august of " {
				self.scratch_pad.write_text("AUGUST of ");
				break;
			}
			" bow before" {
				self.scratch_pad.write_text(" BOW before");
				break;
			}
			" bow down" {
				self.scratch_pad.write_text(" BOW down");
				break;
			}
			" bow out" {
				self.scratch_pad.write_text(" BOW out");
				break;
			}
			" bow to " {
				self.scratch_pad.write_text(" BOW to ");
				break;
			}
			" bowed before " {
				self.scratch_pad.write_text(" BOWED before");
				break;
			}
			" bowed down" {
				self.scratch_pad.write_text(" BOWED down");
				break;
			}
			" bowed out" {
				self.scratch_pad.write_text(" BOWED out");
				break;
			}
			" bowed to " {
				self.scratch_pad.write_text(" BOWED to ");
				break;
			}
			"Baton" {
				self.scratch_pad.write_text("Baton");
				break;
			}
			"Bologna" {
				self.scratch_pad.write_text("Bologna");
				break;
			}
			"Polish" {
				self.scratch_pad.write_text("Polish");
				break;
			}
			"Nestle" {
				self.scratch_pad.write_text("Nestle");
				break;
			}
			" close by" {
				self.scratch_pad.write_text(" CLOSE by");
				break;
			}
			" close call" {
				self.scratch_pad.write_text(" CLOSE call");
				break;
			}
			" close contact" {
				self.scratch_pad.write_text(" CLOSE contact");
				break;
			}
			" close enough" {
				self.scratch_pad.write_text(" CLOSE enough");
				break;
			}
			" close friend" {
				self.scratch_pad.write_text(" CLOSE friend");
				break;
			}
			" close quarter" {
				self.scratch_pad.write_text(" CLOSE quarter");
				break;
			}
			" close shave" {
				self.scratch_pad.write_text(" CLOSE shave");
				break;
			}
			" close to " {
				self.scratch_pad.write_text(" CLOSE to ");
				break;
			}
			" close together" {
				self.scratch_pad.write_text(" CLOSE together");
				break;
			}
			" he lead " {
				self.scratch_pad.write_text(" he LEAD ");
				break;
			}
			" she lead " {
				self.scratch_pad.write_text(" she LEAD ");
				break;
			}
			" it lead " {
				self.scratch_pad.write_text(" it LEAD ");
				break;
			}
			" they lead by " {
				// catch this one. this should be "liyd"
				self.scratch_pad.write_text(" they lead by ");
				break;
			}
			"\x27ve lead " {
				self.scratch_pad.write_text("\x27ve LEAD ");
				break;
			}
			" they lead " {
				self.scratch_pad.write_text(" they LEAD ");
				break;
			}
			" lead bullet" {
				self.scratch_pad.write_text(" LEAD bullet");
				break;
			}
			" lead free " {
				self.scratch_pad.write_text(" LEAD free ");
				break;
			}
			" lead paint" {
				self.scratch_pad.write_text(" LEAD paint");
				break;
			}
			" lead pencil" {
				self.scratch_pad.write_text(" LEAD pencil");
				break;
			}
			" lead pipe" {
				self.scratch_pad.write_text(" LEAD pipe");
				break;
			}
			" lead poisoning" {
				self.scratch_pad.write_text(" LEAD poisoning");
				break;
			}
			" lead shot" {
				self.scratch_pad.write_text(" LEAD shot");
				break;
			}
			" lead weight" {
				self.scratch_pad.write_text(" LEAD weight");
				break;
			}
			" live ammo" {
				self.scratch_pad.write_text(" LIVE ammo");
				break;
			}
			" live ammunition" {
				self.scratch_pad.write_text(" LIVE ammunition");
				break;
			}
			" live bait" {
				self.scratch_pad.write_text(" LIVE bait");
				break;
			}
			" live broadcast" {
				self.scratch_pad.write_text(" LIVE broadcast");
				break;
			}
			" live concert" {
				self.scratch_pad.write_text(" LIVE concert");
				break;
			}
			" live event" {
				self.scratch_pad.write_text(" LIVE event");
				break;
			}
			" live fire" {
				self.scratch_pad.write_text(" LIVE fire");
				break;
			}
			" live game" {
				self.scratch_pad.write_text(" LIVE game");
				break;
			}
			" live music" {
				self.scratch_pad.write_text(" LIVE music");
				break;
			}
			" live performance" {
				self.scratch_pad.write_text(" LIVE performance");
				break;
			}
			" live round" {
				self.scratch_pad.write_text(" LIVE round ");
				break;
			}
			" live show" {
				self.scratch_pad.write_text(" LIVE show ");
				break;
			}
			" live wire" {
				self.scratch_pad.write_text(" LIVE wire ");
				break;
			}
			"minute amount" {
				self.scratch_pad.write_text("MINUTE amount ");
				break;
			}
			"minute detail" {
				self.scratch_pad.write_text("MINUTE detail ");
				break;
			}
			"minute droplet" {
				self.scratch_pad.write_text("MINUTE droplet");
				break;
			}
			"minute number" {
				self.scratch_pad.write_text("MINUTE number");
				break;
			}
			"minute percentage" {
				self.scratch_pad.write_text("MINUTE percentage");
				break;
			}
			" how minute the " {
				self.scratch_pad.write_text(" how MINUTE the ");
				break;
			}
			"tear drop" {
				self.scratch_pad.write_text("TEAR drop");
				break;
			}
			"tear duct" {
				self.scratch_pad.write_text("TEAR duct");
				break;
			}
			"tear gas " {
				self.scratch_pad.write_text("TEAR gas ");
				break;
			}
			"tear jerker" {
				self.scratch_pad.write_text("TEAR jerker");
				break;
			}
			"tear stained" {
				self.scratch_pad.write_text("TEAR stained");
				break;
			}
			"tearing up the " {
				self.scratch_pad.write_text("TEARING up the ");
				break;
			}
			" wind around" {
				self.scratch_pad.write_text(" WIND around");
				break;
			}
			" wind back" {
				self.scratch_pad.write_text(" WIND back");
				break;
			}
			" wind down" {
				self.scratch_pad.write_text(" WIND down");
				break;
			}
			" wind through" {
				self.scratch_pad.write_text(" WIND through");
				break;
			}
			" wind up" {
				self.scratch_pad.write_text(" WIND up");
				break;
			}
			"wound around" {
				self.scratch_pad.write_text("WOUND around");
				break;
			}
			"wound back" {
				self.scratch_pad.write_text("WOUND back");
				break;
			}
			"wound down" {
				self.scratch_pad.write_text("WOUND down");
				break;
			}
			"wound through" {
				self.scratch_pad.write_text("WOUND through");
				break;
			}
			"wound up " {
				self.scratch_pad.write_text("WOUND up ");
				break;
			}
			"aired live " {
				self.scratch_pad.write_text("aired LIVE ");
				break;
			}
			" he moped" {
				self.scratch_pad.write_text(" he MOPED");
				break;
			}
			" she moped" {
				self.scratch_pad.write_text(" she MOPED");
				break;
			}
			" they moped" {
				self.scratch_pad.write_text(" they MOPED");
				break;
			}
			"already moped" {
				self.scratch_pad.write_text("already MOPED");
				break;
			}
			" well read " {
				self.scratch_pad.write_text(" well READ ");
				break;
			}
			" he read " {
				self.scratch_pad.write_text(" he READ ");
				break;
			}
			" she read " {
				self.scratch_pad.write_text(" she READ ");
				break;
			}
			"already read " {
				self.scratch_pad.write_text("already READ ");
				break;
			}
			"already used" {
				self.scratch_pad.write_text("already USED");
				break;
			}
			"already uses" {
				self.scratch_pad.write_text("already USES");
				break;
			}
			" an expose" {
				self.scratch_pad.write_text(" an EXPOSE");
				break;
			}
			"broadcasted live " {
				self.scratch_pad.write_text("broadcasted LIVE ");
				break;
			}
			"broadcast live " {
				self.scratch_pad.write_text("broadcast LIVE ");
				break;
			}
			"came close " {
				self.scratch_pad.write_text("came CLOSE ");
				break;
			}
			" can wind " {
				self.scratch_pad.write_text(" can WIND ");
				break;
			}
			"come close " {
				self.scratch_pad.write_text("come CLOSE ");
				break;
			}
			"could wind " {
				self.scratch_pad.write_text("could WIND ");
				break;
			}
			"fairly close " {
				self.scratch_pad.write_text("fairly CLOSE ");
				break;
			}
			"get close " {
				self.scratch_pad.write_text("get CLOSE ");
				break;
			}
			"gets close " {
				self.scratch_pad.write_text("gets CLOSE ");
				break;
			}
			"getting close " {
				self.scratch_pad.write_text("getting CLOSE ");
				break;
			}
			" goes live " {
				self.scratch_pad.write_text(" goes LIVE ");
				break;
			}
			"had moped" {
				self.scratch_pad.write_text("had MOPED");
				break;
			}
			"\x27d read " {
				self.scratch_pad.write_text("\x27d READ ");
				break;
			}
			"had read " {
				self.scratch_pad.write_text("had READ ");
				break;
			}
			"had used" {
				self.scratch_pad.write_text("had USED");
				break;
			}
			"has moped" {
				self.scratch_pad.write_text("has MOPED");
				break;
			}
			"has read " {
				self.scratch_pad.write_text("has READ ");
				break;
			}
			"has used" {
				self.scratch_pad.write_text("has USED");
				break;
			}
			"have moped" {
				self.scratch_pad.write_text("have MOPED");
				break;
			}
			"\x27ve read " {
				self.scratch_pad.write_text("\x27ve READ ");
				break;
			}
			"have read " {
				self.scratch_pad.write_text("have READ ");
				break;
			}
			"have used" {
				self.scratch_pad.write_text("have USED");
				break;
			}
			" he dove " {
				self.scratch_pad.write_text(" he DOVE ");
				break;
			}
			" he entrances" {
				self.scratch_pad.write_text(" he ENTRANCES");
				break;
			}
			" he rebels" {
				self.scratch_pad.write_text(" he REBELS");
				break;
			}
			" he records" {
				self.scratch_pad.write_text(" he RECORDS");
				break;
			}
			" her tear " {
				self.scratch_pad.write_text(" her TEAR ");
				break;
			}
			" her tears" {
				self.scratch_pad.write_text(" her TEARS");
				break;
			}
			" his tear " {
				self.scratch_pad.write_text(" his TEAR ");
				break;
			}
			" his tears" {
				self.scratch_pad.write_text(" his TEARS");
				break;
			}
			"just moped" {
				self.scratch_pad.write_text("just MOPED");
				break;
			}
			"just read " {
				self.scratch_pad.write_text("just READ ");
				break;
			}
			"just used" {
				self.scratch_pad.write_text("just USED");
				break;
			}
			"just uses" {
				self.scratch_pad.write_text("just USES");
				break;
			}
			"let wind " {
				self.scratch_pad.write_text("let WIND ");
				break;
			}
			"lets wind " {
				self.scratch_pad.write_text("lets WIND ");
				break;
			}
			"never moped" {
				self.scratch_pad.write_text("never MOPED");
				break;
			}
			"never used" {
				self.scratch_pad.write_text("never USED");
				break;
			}
			"never uses" {
				self.scratch_pad.write_text("never USES");
				break;
			}
			"no matter how minute" {
				self.scratch_pad.write_text("no matter how MINUTE");
				break;
			}
			" of lead " {
				self.scratch_pad.write_text(" of LEAD ");
				break;
			}
			" one tear " {
				self.scratch_pad.write_text(" one TEAR ");
				break;
			}
			"our lives " {
				self.scratch_pad.write_text("our LIVES ");
				break;
			}
			" own lives " {
				self.scratch_pad.write_text(" own LIVES ");
				break;
			}
			"pretty close " {
				self.scratch_pad.write_text("pretty CLOSE ");
				break;
			}
			"quite close " {
				self.scratch_pad.write_text("quite CLOSE ");
				break;
			}
			"really close " {
				self.scratch_pad.write_text("really CLOSE ");
				break;
			}
			"relatively minute" {
				self.scratch_pad.write_text("relatively MINUTE");
				break;
			}
			"remain close " {
				self.scratch_pad.write_text("remain CLOSE ");
				break;
			}
			"remains close " {
				self.scratch_pad.write_text("remains CLOSE ");
				break;
			}
			" sea bass " {
				self.scratch_pad.write_text(" sea BASS ");
				break;
			}
			" she dove " {
				self.scratch_pad.write_text(" she DOVE ");
				break;
			}
			" she entrances" {
				self.scratch_pad.write_text(" she ENTRANCES");
				break;
			}
			" she rebels" {
				self.scratch_pad.write_text(" she REBELS");
				break;
			}
			" she records" {
				self.scratch_pad.write_text(" she RECORDS");
				break;
			}
			"single tear " {
				self.scratch_pad.write_text("single TEAR ");
				break;
			}
			" so close " {
				self.scratch_pad.write_text(" so CLOSE ");
				break;
			}
			" so minute" {
				self.scratch_pad.write_text(" so MINUTE");
				break;
			}
			"stay close " {
				self.scratch_pad.write_text("stay CLOSE ");
				break;
			}
			"staying close " {
				self.scratch_pad.write_text("staying CLOSE ");
				break;
			}
			"stream live " {
				self.scratch_pad.write_text("stream LIVE ");
				break;
			}
			"streamed live " {
				self.scratch_pad.write_text("streamed LIVE ");
				break;
			}
			"take a bow " {
				self.scratch_pad.write_text("take a BOW ");
				break;
			}
			"takes a bow " {
				self.scratch_pad.write_text("takes a BOW ");
				break;
			}
			"that close " {
				self.scratch_pad.write_text("that CLOSE ");
				break;
			}
			" the expose" {
				self.scratch_pad.write_text(" the EXPOSE");
				break;
			}
			"their lives" {
				self.scratch_pad.write_text("their LIVES");
				break;
			}
			"they dove " {
				self.scratch_pad.write_text("they DOVE ");
				break;
			}
			"they rebel " {
				self.scratch_pad.write_text("they REBEL ");
				break;
			}
			"they record " {
				self.scratch_pad.write_text("they RECORD ");
				break;
			}
			"they\x27ve moped" {
				self.scratch_pad.write_text("they\x27ve MOPED");
				break;
			}
			"they\x27ve read " {
				self.scratch_pad.write_text("they\x27ve READ ");
				break;
			}
			"they\x27ve used" {
				self.scratch_pad.write_text("they\x27ve USED");
				break;
			}
			"this close " {
				self.scratch_pad.write_text("this CLOSE ");
				break;
			}
			" to entrance" {
				self.scratch_pad.write_text(" to ENTRANCE");
				break;
			}
			" to rebel " {
				self.scratch_pad.write_text(" to REBEL ");
				break;
			}
			" to record " {
				self.scratch_pad.write_text(" to RECORD ");
				break;
			}
			" to resume" {
				self.scratch_pad.write_text(" to RESUME");
				break;
			}
			" to sow " {
				self.scratch_pad.write_text(" to SOW ");
				break;
			}
			" to use " {
				self.scratch_pad.write_text(" to USE ");
				break;
			}
			" to wind " {
				self.scratch_pad.write_text(" to WIND ");
				break;
			}
			" too close " {
				self.scratch_pad.write_text(" too CLOSE ");
				break;
			}
			"took a bow " {
				self.scratch_pad.write_text("took a BOW ");
				break;
			}
			" very close " {
				self.scratch_pad.write_text(" very CLOSE ");
				break;
			}
			" very minute" {
				self.scratch_pad.write_text(" very MINUTE");
				break;
			}
			"went live " {
				self.scratch_pad.write_text("went LIVE ");
				break;
			}
			"will wind " {
				self.scratch_pad.write_text("will WIND ");
				break;
			}
			"would wind " {
				self.scratch_pad.write_text("would WIND ");
				break;
			}
			"you dove " {
				self.scratch_pad.write_text("you DOVE ");
				break;
			}
			"your tear " {
				self.scratch_pad.write_text("your TEAR ");
				break;
			}
			"your tears" {
				self.scratch_pad.write_text("your TEARS");
				break;
			}
			" to envelope " {
				self.scratch_pad.write_text(" to ENVELOPE ");
				break;
			}
			" they envelope " {
				self.scratch_pad.write_text(" they ENVELOPE ");
				break;
			}
			" he envelopes" {
				self.scratch_pad.write_text(" he ENVELOPES");
				break;
			}
			" she envelopes" {
				self.scratch_pad.write_text(" she ENVELOPES");
				break;
			}
			" it envelopes" {
				self.scratch_pad.write_text(" it ENVELOPES");
				break;
			}
			" a separate" {
				self.scratch_pad.write_text(" a SEPARATE");
				break;
			}
			" the separate" {
				self.scratch_pad.write_text(" the SEPARATE");
				break;
			}
			" a attribute" {
				self.scratch_pad.write_text(" a ATTRIBUTE");
				break;
			}
			" an attribute" {
				self.scratch_pad.write_text(" an ATTRIBUTE");
				break;
			}
			" the attribute" {
				self.scratch_pad.write_text(" the ATTRIBUTE");
				break;
			}
			" he attributes" {
				self.scratch_pad.write_text(" he ATTRIBUTES");
				break;
			}
			" she attributes" {
				self.scratch_pad.write_text(" she ATTRIBUTES");
				break;
			}
			" it attributes" {
				self.scratch_pad.write_text(" it ATTRIBUTES");
				break;
			}
			" he analyses" {
				self.scratch_pad.write_text(" he ANALYSES");
				break;
			}
			" she analyzes" {
				self.scratch_pad.write_text(" she ANALYZES");
				break;
			}
			" a conflict" {
				self.scratch_pad.write_text(" a CONFLICT");
				break;
			}
			" the conflict" {
				self.scratch_pad.write_text(" the CONFLICT");
				break;
			}
			" of conflicts" {
				self.scratch_pad.write_text(" of CONFLICTS");
				break;
			}
			"n\x27t content" {
				self.scratch_pad.write_text("n\x27t CONTENT");
				break;
			}
			"\x27s content" {
				self.scratch_pad.write_text("\x27s CONTENT");
				break;
			}
			"\x27re content" {
				self.scratch_pad.write_text("\x27re CONTENT");
				break;
			}
			" are content" {
				self.scratch_pad.write_text(" are CONTENT");
				break;
			}
			" been content" {
				self.scratch_pad.write_text(" been CONTENT");
				break;
			}
			" be content" {
				self.scratch_pad.write_text(" be CONTENT");
				break;
			}
			" a contest " {
				self.scratch_pad.write_text(" a CONTEST ");
				break;
			}
			" the contest " {
				self.scratch_pad.write_text(" the CONTEST ");
				break;
			}
			" the contests" {
				self.scratch_pad.write_text(" the CONTESTS");
				break;
			}
			" a contract " {
				self.scratch_pad.write_text(" a CONTRACT ");
				break;
			}
			" the contract " {
				self.scratch_pad.write_text(" the CONTRACT ");
				break;
			}
			" he contracts" {
				self.scratch_pad.write_text(" he CONTRACTS");
				break;
			}
			" she contracts" {
				self.scratch_pad.write_text(" she CONTRACTS");
				break;
			}
			" to defect " {
				self.scratch_pad.write_text(" to DEFECT");
				break;
			}
			" they defect " {
				self.scratch_pad.write_text(" they DEFECT");
				break;
			}
			" he defects" {
				self.scratch_pad.write_text(" he DEFECTS");
				break;
			}
			" she defects" {
				self.scratch_pad.write_text(" she DEFECTS");
				break;
			}
			" to desert " {
				self.scratch_pad.write_text(" to DESERT ");
				break;
			}
			" they desert " {
				self.scratch_pad.write_text(" they DESERT ");
				break;
			}
			" he desert " {
				self.scratch_pad.write_text(" he DESERT ");
				break;
			}
			" she desert " {
				self.scratch_pad.write_text(" she DESERT ");
				break;
			}
			" eat desert" {
				self.scratch_pad.write_text(" eat DESERT");
				break;
			}
			" eating desert" {
				self.scratch_pad.write_text(" eating DESERT");
				break;
			}
			" have desert " {
				self.scratch_pad.write_text(" have DESERT ");
				break;
			}
			" having desert " {
				self.scratch_pad.write_text(" having DESERT ");
				break;
			}
			" his desert " {
				self.scratch_pad.write_text(" his DESERT ");
				break;
			}
			" her desert " {
				self.scratch_pad.write_text(" her DESERT ");
				break;
			}
			" he deserts" {
				self.scratch_pad.write_text(" he DESERTS");
				break;
			}
			" she deserts" {
				self.scratch_pad.write_text(" she DESERTS");
				break;
			}
			" a object " {
				self.scratch_pad.write_text(" a OBJECT ");
				break;
			}
			" an object " {
				self.scratch_pad.write_text(" an OBJECT ");
				break;
			}
			" the object " {
				self.scratch_pad.write_text(" the OBJECT ");
				break;
			}
			" the objects" {
				self.scratch_pad.write_text(" the OBJECTS");
				break;
			}
			" a present " {
				self.scratch_pad.write_text(" a PRESENT ");
				break;
			}
			" the present " {
				self.scratch_pad.write_text(" the PRESENT ");
				break;
			}
			" gave him presents" {
				self.scratch_pad.write_text(" gave him PRESENTS");
				break;
			}
			" gave her presents" {
				self.scratch_pad.write_text(" gave her PRESENTS");
				break;
			}
			" gave them presents" {
				self.scratch_pad.write_text(" gave them PRESENTS");
				break;
			}
			" a project " {
				self.scratch_pad.write_text(" a PROJECT ");
				break;
			}
			" the project " {
				self.scratch_pad.write_text(" the PROJECT ");
				break;
			}
			" his project " {
				self.scratch_pad.write_text(" his PROJECT ");
				break;
			}
			" her project " {
				self.scratch_pad.write_text(" her PROJECT ");
				break;
			}
			" their project " {
				self.scratch_pad.write_text(" their PROJECT ");
				break;
			}
			" he projects" {
				self.scratch_pad.write_text(" he PROJECTS");
				break;
			}
			" she projects" {
				self.scratch_pad.write_text(" she PROJECTS");
				break;
			}
			" to reject " {
				self.scratch_pad.write_text(" to REJECT ");
				break;
			}
			" he rejects" {
				self.scratch_pad.write_text(" he REJECTS");
				break;
			}
			" she rejects" {
				self.scratch_pad.write_text(" she REJECTS");
				break;
			}
			" it rejects" {
				self.scratch_pad.write_text(" it REJECTS");
				break;
			}
			" a convict " {
				self.scratch_pad.write_text(" a CONVICT ");
				break;
			}
			" the convict " {
				self.scratch_pad.write_text(" the CONVICT ");
				break;
			}
			" he convicts" {
				self.scratch_pad.write_text(" he CONVICTS");
				break;
			}
			" she convicts" {
				self.scratch_pad.write_text(" she CONVICTS");
				break;
			}
			" to affect" {
				self.scratch_pad.write_text(" to AFFECT");
				break;
			}
			" he affects" {
				self.scratch_pad.write_text(" he AFFECTS");
				break;
			}
			" she affects" {
				self.scratch_pad.write_text(" she AFFECTS");
				break;
			}
			" it affects" {
				self.scratch_pad.write_text(" it AFFECTS");
				break;
			}
			" to appropriate " {
				self.scratch_pad.write_text(" to APPROPRIATE ");
				break;
			}
			" they appropriate " {
				self.scratch_pad.write_text(" they APPROPRIATE ");
				break;
			}
			" to approximate " {
				self.scratch_pad.write_text(" to APPROXIMATE ");
				break;
			}
			" they approximate " {
				self.scratch_pad.write_text(" they APPROXIMATE ");
				break;
			}
			" to articulate " {
				self.scratch_pad.write_text(" to ARTICULATE ");
				break;
			}
			" to address " {
				self.scratch_pad.write_text(" to ADDRESS ");
				break;
			}
			" to advocate " {
				self.scratch_pad.write_text(" to ADVOCATE ");
				break;
			}
			" he advocates " {
				self.scratch_pad.write_text(" he ADVOCATES ");
				break;
			}
			" she advocates " {
				self.scratch_pad.write_text(" she ADVOCATES ");
				break;
			}
			" it advocates " {
				self.scratch_pad.write_text(" it ADVOCATES ");
				break;
			}
			" to aggregate " {
				self.scratch_pad.write_text(" to AGGREGATE ");
				break;
			}
			" he aggregates " {
				self.scratch_pad.write_text(" he AGGREGATES ");
				break;
			}
			" she aggregates " {
				self.scratch_pad.write_text(" she AGGREGATES ");
				break;
			}
			" it aggregates " {
				self.scratch_pad.write_text(" it AGGREGATES ");
				break;
			}
			" to alternate " {
				self.scratch_pad.write_text(" to ALTERNATE ");
				break;
			}
			" an animate " {
				self.scratch_pad.write_text(" an ANIMATE ");
				break;
			}
			" the animate " {
				self.scratch_pad.write_text(" the ANIMATE ");
				break;
			}
			" a associate " {
				self.scratch_pad.write_text(" a ASSOCIATE ");
				break;
			}
			" an associate " {
				self.scratch_pad.write_text(" an ASSOCIATE ");
				break;
			}
			" an associates" {
				self.scratch_pad.write_text(" an ASSOCIATES");
				break;
			}
			" the associates" {
				self.scratch_pad.write_text(" the ASSOCIATES");
				break;
			}
			" to bath " {
				self.scratch_pad.write_text(" to BATH ");
				break;
			}
			" they bath " {
				self.scratch_pad.write_text(" they BATH ");
				break;
			}
			" he baths " {
				self.scratch_pad.write_text(" he BATHS ");
				break;
			}
			" she baths " {
				self.scratch_pad.write_text(" she BATHS ");
				break;
			}
			" it baths " {
				self.scratch_pad.write_text(" it BATHS ");
				break;
			}
			" to compound " {
				self.scratch_pad.write_text(" to COMPOUND ");
				break;
			}
			" they compound " {
				self.scratch_pad.write_text(" they COMPOUND ");
				break;
			}
			"\x27ve compound " {
				self.scratch_pad.write_text("\x27ve COMPOUND ");
				break;
			}
			" to conduct " {
				self.scratch_pad.write_text(" to CONDUCT ");
				break;
			}
			" he conduct " {
				self.scratch_pad.write_text(" he CONDUCT ");
				break;
			}
			" she conduct " {
				self.scratch_pad.write_text(" she CONDUCT ");
				break;
			}
			" they conduct " {
				self.scratch_pad.write_text(" they CONDUCT ");
				break;
			}
			" a console" {
				self.scratch_pad.write_text(" a CONSOLE");
				break;
			}
			" the console" {
				self.scratch_pad.write_text(" the CONSOLE");
				break;
			}
			" he consoles" {
				self.scratch_pad.write_text(" he CONSOLES");
				break;
			}
			" she consoles" {
				self.scratch_pad.write_text(" she CONSOLES");
				break;
			}
			" it consoles" {
				self.scratch_pad.write_text(" it CONSOLES");
				break;
			}
			" to construct " {
				self.scratch_pad.write_text(" to CONSTRUCT ");
				break;
			}
			" they construct " {
				self.scratch_pad.write_text(" they CONSTRUCT ");
				break;
			}
			" he constructs" {
				self.scratch_pad.write_text(" he CONSTRUCTS");
				break;
			}
			" she constructs" {
				self.scratch_pad.write_text(" she CONSTRUCTS");
				break;
			}
			" it constructs" {
				self.scratch_pad.write_text(" it CONSTRUCTS");
				break;
			}
			" a convert " {
				self.scratch_pad.write_text(" a CONVERT ");
				break;
			}
			" the convert " {
				self.scratch_pad.write_text(" the CONVERT ");
				break;
			}
			" to coordinate " {
				self.scratch_pad.write_text(" to COORDINATE ");
				break;
			}
			" they coordinate " {
				self.scratch_pad.write_text(" they COORDINATE ");
				break;
			}
			" he coordinates" {
				self.scratch_pad.write_text(" he COORDINATES");
				break;
			}
			" she coordinates" {
				self.scratch_pad.write_text(" she COORDINATES");
				break;
			}
			" it coordinates" {
				self.scratch_pad.write_text(" it COORDINATES");
				break;
			}
			" a duplicate " {
				self.scratch_pad.write_text(" a DUPLICATE ");
				break;
			}
			" the duplicate " {
				self.scratch_pad.write_text(" the DUPLICATE ");
				break;
			}
			" he duplicates" {
				self.scratch_pad.write_text(" he DUPLICATES");
				break;
			}
			" she duplicates" {
				self.scratch_pad.write_text(" she DUPLICATES");
				break;
			}
			" it duplicates" {
				self.scratch_pad.write_text(" it DUPLICATES");
				break;
			}
			" a estimate " {
				self.scratch_pad.write_text(" a ESTIMATE ");
				break;
			}
			" an estimate " {
				self.scratch_pad.write_text(" an ESTIMATE ");
				break;
			}
			" the estimate " {
				self.scratch_pad.write_text(" the ESTIMATE ");
				break;
			}
			" he estimates " {
				self.scratch_pad.write_text(" he ESTIMATES ");
				break;
			}
			" she estimates " {
				self.scratch_pad.write_text(" she ESTIMATES ");
				break;
			}
			" it estimates " {
				self.scratch_pad.write_text(" it ESTIMATES ");
				break;
			}
			" a graduate " {
				self.scratch_pad.write_text(" a GRADUATE ");
				break;
			}
			" the graduate " {
				self.scratch_pad.write_text(" the GRADUATE ");
				break;
			}
			" he graduates " {
				self.scratch_pad.write_text(" he GRADUATES ");
				break;
			}
			" she graduates " {
				self.scratch_pad.write_text(" she GRADUATES ");
				break;
			}
			" it graduates " {
				self.scratch_pad.write_text(" it GRADUATES ");
				break;
			}
			" to house " {
				self.scratch_pad.write_text(" to HOUSE ");
				break;
			}
			" they house " {
				self.scratch_pad.write_text(" they HOUSE ");
				break;
			}
			" more moderate " {
				self.scratch_pad.write_text(" more MODERATE ");
				break;
			}
			" a moderate " {
				self.scratch_pad.write_text(" a MODERATE ");
				break;
			}
			" the moderate " {
				self.scratch_pad.write_text(" the MODERATE ");
				break;
			}
			" he moderates " {
				self.scratch_pad.write_text(" he MODERATES ");
				break;
			}
			" she moderates " {
				self.scratch_pad.write_text(" she MODERATES ");
				break;
			}
			" it moderates " {
				self.scratch_pad.write_text(" it MODERATES ");
				break;
			}
			" to mouth " {
				self.scratch_pad.write_text(" to MOUTH ");
				break;
			}
			" they mouth " {
				self.scratch_pad.write_text(" they MOUTH ");
				break;
			}
			" to perfect " {
				self.scratch_pad.write_text(" to PERFECT ");
				break;
			}
			" they perfect " {
				self.scratch_pad.write_text(" they PERFECT ");
				break;
			}
			" to permit " {
				self.scratch_pad.write_text(" to PERMIT ");
				break;
			}
			" they permit " {
				self.scratch_pad.write_text(" they PERMIT ");
				break;
			}
			" he permits" {
				self.scratch_pad.write_text(" he PERMITS ");
				break;
			}
			" she permits" {
				self.scratch_pad.write_text(" she PERMITS ");
				break;
			}
			" it permits " {
				self.scratch_pad.write_text(" it PERMITS ");
				break;
			}
			" to produce " {
				self.scratch_pad.write_text(" to PRODUCE ");
				break;
			}
			" they produce " {
				self.scratch_pad.write_text(" they PRODUCE ");
				break;
			}
			" he produce " {
				self.scratch_pad.write_text(" he PRODUCE ");
				break;
			}
			" she produce " {
				self.scratch_pad.write_text(" she PRODUCE ");
				break;
			}
			" it produce " {
				self.scratch_pad.write_text(" it PRODUCE ");
				break;
			}
			" to progress " {
				self.scratch_pad.write_text(" to PROGRESS ");
				break;
			}
			" they progress " {
				self.scratch_pad.write_text(" they PROGRESS ");
				break;
			}
			" he progresses" {
				self.scratch_pad.write_text(" he PROGRESSES");
				break;
			}
			" she progresses" {
				self.scratch_pad.write_text(" she PROGRESSES");
				break;
			}
			" it progresses" {
				self.scratch_pad.write_text(" it PROGRESSES");
				break;
			}
			" a survey " {
				self.scratch_pad.write_text(" a SURVEY ");
				break;
			}
			" the survey " {
				self.scratch_pad.write_text(" the SURVEY ");
				break;
			}
			" a suspect " {
				self.scratch_pad.write_text(" a SUSPECT ");
				break;
			}
			" the suspect " {
				self.scratch_pad.write_text(" the SUSPECT ");
				break;
			}
			" the suspects " {
				self.scratch_pad.write_text(" the SUSPECTS ");
				break;
			}
			" a affiliate " {
				self.scratch_pad.write_text(" a AFFILIATE ");
				break;
			}
			" an affiliate " {
				self.scratch_pad.write_text(" an AFFILIATE ");
				break;
			}
			" the affiliate " {
				self.scratch_pad.write_text(" the AFFILIATE ");
				break;
			}
			" a conjugate " {
				self.scratch_pad.write_text(" a CONJUGATE ");
				break;
			}
			" the conjugate " {
				self.scratch_pad.write_text(" the CONJUGATE ");
				break;
			}
			" to default " {
				self.scratch_pad.write_text(" to DEFAULT ");
				break;
			}
			" they default " {
				self.scratch_pad.write_text(" they DEFAULT ");
				break;
			}

			/*---------------------*/
			/* STATE ABBREVIATIONS */
			/*---------------------*/

			state_start 'AK' state_end {
				self.scratch_pad.write_text(" alaska ");
				break;
			}
			state_start 'AL' state_end {
				self.scratch_pad.write_text(" alabama ");
				break;
			}
			state_start 'AR' state_end {
				self.scratch_pad.write_text(" arkansas ");
				break;
			}
			state_start 'AZ' state_end {
				self.scratch_pad.write_text(" arizona ");
				break;
			}
			state_start 'CA' state_end {
				self.scratch_pad.write_text(" california ");
				break;
			}
			state_start "CO" state_end {
				self.scratch_pad.write_text(" colorado ");
				break;
			}
			state_start 'CT' state_end {
				self.scratch_pad.write_text(" connecticut ");
				break;
			}
			state_start 'DC' state_end {
				self.scratch_pad.write_text(" district of columbia ");
				break;
			}
			state_start 'DE' state_end {
				self.scratch_pad.write_text(" delaware ");
				break;
			}
			state_start 'FL' state_end {
				self.scratch_pad.write_text(" florida ");
				break;
			}
			state_start 'GA' state_end {
				self.scratch_pad.write_text(" georgia ");
				break;
			}
			state_start "HI" state_end {
				self.scratch_pad.write_text(" hawaii ");
				break;
			}
			state_start 'IA' state_end {
				self.scratch_pad.write_text(" iowa ");
				break;
			}
			state_start "ID" state_end {
				self.scratch_pad.write_text(" idaho ");
				break;
			}
			state_start 'IL' state_end {
				self.scratch_pad.write_text(" illinois ");
				break;
			}
			state_start "IN" state_end {
				self.scratch_pad.write_text(" indiana ");
				break;
			}
			state_start 'KS' state_end {
				self.scratch_pad.write_text(" kansas ");
				break;
			}
			state_start 'KY' state_end {
				self.scratch_pad.write_text(" kentucky ");
				break;
			}
			state_start "LA" state_end {
				self.scratch_pad.write_text(" louisiana ");
				break;
			}
			state_start "MA" state_end {
				self.scratch_pad.write_text(" massachusetts ");
				break;
			}
			state_start 'MD' state_end {
				self.scratch_pad.write_text(" maryland ");
				break;
			}
			state_start "ME" state_end {
				self.scratch_pad.write_text(" maine ");
				break;
			}
			state_start 'MI' state_end {
				self.scratch_pad.write_text(" michigan ");
				break;
			}
			state_start 'MN' state_end {
				self.scratch_pad.write_text(" minnesota ");
				break;
			}
			state_start 'MO' state_end {
				self.scratch_pad.write_text(" missouri ");
				break;
			}
			state_start 'MS' state_end {
				self.scratch_pad.write_text(" mississippi ");
				break;
			}
			state_start 'MT' state_end {
				self.scratch_pad.write_text(" montana ");
				break;
			}
			state_start 'NC' state_end {
				self.scratch_pad.write_text(" north carolina ");
				break;
			}
			state_start 'ND' state_end {
				self.scratch_pad.write_text(" north dakota ");
				break;
			}
			state_start 'NE' state_end {
				self.scratch_pad.write_text(" nebraska ");
				break;
			}
			state_start 'NH' state_end {
				self.scratch_pad.write_text(" new hampshire ");
				break;
			}
			state_start 'NJ' state_end {
				self.scratch_pad.write_text(" new jersey ");
				break;
			}
			state_start 'NM' state_end {
				self.scratch_pad.write_text(" new mexico ");
				break;
			}
			state_start 'NV' state_end {
				self.scratch_pad.write_text(" nevada ");
				break;
			}
			state_start 'NY' state_end {
				self.scratch_pad.write_text(" new york ");
				break;
			}
			state_start "OH" state_end {
				self.scratch_pad.write_text(" ohio ");
				break;
			}
			state_start "OK" state_end {
				self.scratch_pad.write_text(" oklahoma ");
				break;
			}
			state_start "OR" state_end {
				self.scratch_pad.write_text(" oregon ");
				break;
			}
			state_start 'PA' state_end {
				self.scratch_pad.write_text(" pennsylvania ");
				break;
			}
			state_start 'RI' state_end {
				self.scratch_pad.write_text(" rhode island ");
				break;
			}
			state_start 'SC' state_end {
				self.scratch_pad.write_text(" south carolina ");
				break;
			}
			state_start 'SD' state_end {
				self.scratch_pad.write_text(" south dakota ");
				break;
			}
			state_start 'TN' state_end {
				self.scratch_pad.write_text(" tennessee ");
				break;
			}
			state_start 'TX' state_end {
				self.scratch_pad.write_text(" texas ");
				break;
			}
			state_start 'UT' state_end {
				self.scratch_pad.write_text(" utah ");
				break;
			}
			state_start 'VA' state_end {
				self.scratch_pad.write_text(" virginia ");
				break;
			}
			state_start 'VT' state_end {
				self.scratch_pad.write_text(" vermont ");
				break;
			}
			state_start 'WA' state_end {
				self.scratch_pad.write_text(" washington ");
				break;
			}
			state_start 'WI' state_end {
				self.scratch_pad.write_text(" wisconsin ");
				break;
			}
			state_start 'WV' state_end {
				self.scratch_pad.write_text(" west virginia ");
				break;
			}
			state_start 'WY' state_end {
				self.scratch_pad.write_text(" wyoming ");
				break;
			}

			/*---------------------*/
			/* OTHER ABBREVIATIONS */
			/*---------------------*/

			"\u33da" {
				self.scratch_pad.write_text(" p r ");
				break;
			}
			wbound 'ph.d.' wbound {
				self.scratch_pad.write_text(" p h d ");
				break;
			}
			wbound "yr" wbound {
				self.scratch_pad.write_text(" year ");
				break;
			}
			wbound "yrs" wbound {
				self.scratch_pad.write_text(" years ");
				break;
			}
			wbound "wk" wbound {
				self.scratch_pad.write_text(" week ");
				break;
			}
			wbound "wks" wbound {
				self.scratch_pad.write_text(" weeks ");
				break;
			}
			wbound 'ttyl' wbound {
				self.scratch_pad.write_text(" talk to you later ");
				break;
			}
			wbound 'utc' wbound {
				self.scratch_pad.write_text(" coordinated universal time ");
				break;
			}
			wbound 'vlly' wbound | wbound 'vly' wbound {
				self.scratch_pad.write_text(" valley ");
				break;
			}
			wbound 'v' period? 's' period? {
				self.scratch_pad.write_text(" versus ");
				break;
			}
			wbound 'w/e' wbound {
				self.scratch_pad.write_text(" whatever ");
				break;
			}
			wbound 'w/o' wbound {
				self.scratch_pad.write_text(" without ");
				break;
			}
			wbound 'w/' wbound {
				self.scratch_pad.write_text(" with ");
				break;
			}
			wbound 'wdt' wbound {
				self.scratch_pad.write_text(" western daylight time ");
				break;
			}
			wbound 'wst' wbound {
				self.scratch_pad.write_text(" western standard time ");
				break;
			}
			wbound 'nt' period? ' wt' period? {
				self.scratch_pad.write_text(" net weight ");
				break;
			}
			wbound 'wt' wbound {
				self.scratch_pad.write_text(" weight ");
				break;
			}
			wbound 'wtaf' wbound {
				self.scratch_pad.write_text(" what the actual fuck ");
				break;
			}
			wbound 'wtf' wbound {
				self.scratch_pad.write_text(" what the fuck ");
				break;
			}
			wbound 'wth' wbound {
				self.scratch_pad.write_text(" what the hell ");
				break;
			}
			wbound 'ymmd' wbound {
				self.scratch_pad.write_text(" you made my day ");
				break;
			}
			wbound 'tyvm' wbound {
				self.scratch_pad.write_text(" thank you very much ");
				break;
			}
			wbound 'ymmv' wbound {
				self.scratch_pad.write_text(" your mileage may vary ");
				break;
			}
			wbound 'trnpk' wbound |  wbound 'tpke' wbound | wbound 'tpk' wbound {
				self.scratch_pad.write_text(" turnpike ");
				break;
			}
			wbound "trl" wbound {
				self.scratch_pad.write_text(" trail ");
				break;
			}
			wbound 'tl;dr' wbound | wbound 'tldr' wbound {
				self.scratch_pad.write_text(" too long , didn\x27t read . ");
				break;
			}
			wbound 'ste.' {
				self.scratch_pad.write_text(" suite ");
				break;
			}
			wbound 'stfu' wbound {
				self.scratch_pad.write_text(" shut the fuck up ");
				break;
			}
			wbound 'tba' wbound {
				self.scratch_pad.write_text(" to be announced ");
				break;
			}
			wbound 'tbc' wbound {
				self.scratch_pad.write_text(" to be confirmed ");
				break;
			}
			wbound 'tbd' wbound {
				self.scratch_pad.write_text(" to be determined ");
				break;
			}
			wbound 'tbh' wbound {
				self.scratch_pad.write_text(" to be honest ");
				break;
			}
			wbound 'tbp' wbound {
				self.scratch_pad.write_text(" to be provided ");
				break;
			}
			wbound 'tbr' wbound {
				self.scratch_pad.write_text(" to be resolved ");
				break;
			}
			wbound 'thx' wbound | wbound 'thnx' wbound {
				self.scratch_pad.write_text(" thanks ");
				break;
			}
			wbound 'Sr.' {
				self.scratch_pad.write_text(" senior , ");
				break;
			}
			wbound "st." | wbound "ST." {
				self.scratch_pad.write_text(" street ");
				break;
			}
			wbound 'sq.' {
				self.scratch_pad.write_text(" square ");
				break;
			}
			wbound 'Cpl.' {
				self.scratch_pad.write_text(" corporal ");
				break;
			}
			wbound 'Sgt.' {
				self.scratch_pad.write_text(" sergeant ");
				break;
			}
			wbound 'smdh' wbound {
				self.scratch_pad.write_text(" shaking my damn head ");
				break;
			}
			wbound 'smh' wbound {
				self.scratch_pad.write_text(" shaking my head ");
				break;
			}
			wbound 'pst' wbound {
				self.scratch_pad.write_text(" pacific standard time ");
				break;
			}
			wbound 'rdg' wbound {
				self.scratch_pad.write_text(" ridge ");
				break;
			}
			wbound 'rlly' wbound | wbound 'rly' wbound {
				self.scratch_pad.write_text(" really ");
				break;
			}
			wbound 'rm' wbound {
				self.scratch_pad.write_text(" room ");
				break;
			}
			wbound 'rn' wbound {
				self.scratch_pad.write_text(" right now ");
				break;
			}
			wbound 'rofl' wbound | "\U0001f923" {
				self.scratch_pad.write_text(" rolling on the floor laughing ");
				break;
			}
			wbound 'Prof.' {
				self.scratch_pad.write_text(" professor ");
				break;
			}
			wbound 'pdt' wbound {
				self.scratch_pad.write_text(" pacific daylight time ");
				break;
			}
			wbound 'pkg' wbound | "\U0001f4e6" {
				self.scratch_pad.write_text(" package ");
				break;
			}
			wbound 'pkwy' wbound {
				self.scratch_pad.write_text(" parkway ");
				break;
			}
			wbound 'pl.' {
				self.scratch_pad.write_text(" place ");
				break;
			}
			wbound 'plz' wbound {
				self.scratch_pad.write_text(" plaza ");
				break;
			}
			wbound 'ppl' wbound {
				self.scratch_pad.write_text(" people ");
				break;
			}
			'pat. pend.' {
				self.scratch_pad.write_text(" patent pending ");
				break;
			}
			wbound 'n/a' wbound {
				self.scratch_pad.write_text(" not applicable ");
				break;
			}
			wbound 'ngl' wbound {
				self.scratch_pad.write_text(" not gonna lie ");
				break;
			}
			wbound 'nsfw' wbound {
				self.scratch_pad.write_text(" not safe for work ");
				break;
			}
			wbound 'nvm' wbound {
				self.scratch_pad.write_text(" never mind ");
				break;
			}
			wbound 'ofc' wbound {
				self.scratch_pad.write_text(" of course ");
				break;
			}
			wbound 'omg' wbound {
				self.scratch_pad.write_text(" oh my god ");
				break;
			}
			wbound 'Mtn.' {
				self.scratch_pad.write_text(" mountain ");
				break;
			}
			wbound 'Mtns.' {
				self.scratch_pad.write_text(" mountains ");
				break;
			}
			wbound 'Mt.' {
				self.scratch_pad.write_text(" mount ");
				break;
			}
			wbound 'Mr.' {
				self.scratch_pad.write_text(" mister ");
				break;
			}
			wbound 'Messrs.' {
				self.scratch_pad.write_text(" misters ");
				break;
			}
			wbound 'Mrs.' {
				self.scratch_pad.write_text(" missus ");
				break;
			}
			wbound 'Ms.' {
				self.scratch_pad.write_text(" miz ");
				break;
			}
			wbound 'msg' wbound {
				self.scratch_pad.write_text(" message ");
				break;
			}
			wbound 'mst.' {
				self.scratch_pad.write_text(" mountain standard time ");
				break;
			}
			wbound 'mdse.' {
				self.scratch_pad.write_text(" merchandise ");
				break;
			}
			wbound 'mdt.' {
				self.scratch_pad.write_text(" mountain daylight time ");
				break;
			}
			wbound 'mgmt' wbound {
				self.scratch_pad.write_text(" management ");
				break;
			}
			wbound 'misc.' {
				self.scratch_pad.write_text(" miscellaneous ");
				break;
			}
			wbound 'mmw' wbound {
				self.scratch_pad.write_text(" mark my words ");
				break;
			}
			wbound 'mss.' {
				self.scratch_pad.write_text(" manuscripts ");
				break;
			}
			wbound 'Comdr.' | wbound 'Cmdr.' | wbound 'Cdr.' {
				self.scratch_pad.write_text(" commander ");
				break;
			}
			"Gov." {
				self.scratch_pad.write_text(" governor ");
				break;
			}
			wbound 'Lt.' {
				self.scratch_pad.write_text(" lieutenant ");
				break;
			}
			wbound 'Ltd' wbound {
				self.scratch_pad.write_text(" limited ");
				break;
			}
			'Maj.' {
				self.scratch_pad.write_text(" major ");
				break;
			}
			wbound 'Brig.' {
				self.scratch_pad.write_text(" brigade ");
				break;
			}
			"Gen." {
				self.scratch_pad.write_text(" general ");
				break;
			}
			wbound "lb" wbound {
				self.scratch_pad.write_text(" pound ");
				break;
			}
			wbound 'lbs' wbound {
				self.scratch_pad.write_text(" pounds ");
				break;
			}
			wbound 'lmao' wbound {
				self.scratch_pad.write_text(" laughing my ass off ");
				break;
			}
			wbound 'lmfao' wbound {
				self.scratch_pad.write_text(" laughing my fucking ass off ");
				break;
			}
			wbound 'ln' wbound {
				self.scratch_pad.write_text(" lane ");
				break;
			}
			wbound 'Jr.' {
				self.scratch_pad.write_text(" junior , ");
				break;
			}
			'kHz' wbound | "\u3391" {
				self.scratch_pad.write_text(" kilohertz ");
				break;
			}
			wbound 'idk' wbound {
				self.scratch_pad.write_text(" i don\x27t know ");
				break;
			}
			wbound 'iirc' wbound {
				self.scratch_pad.write_text(" if i remember correctly ");
				break;
			}
			wbound 'ikr' wbound {
				self.scratch_pad.write_text(" i know right ? ");
				break;
			}
			wbound 'ikyl' wbound {
				self.scratch_pad.write_text(" i know you\x27re lying ");
				break;
			}
			wbound 'imho' wbound {
				self.scratch_pad.write_text(" in my humble opinion ");
				break;
			}
			wbound 'imo' wbound {
				self.scratch_pad.write_text(" in my opinion ");
				break;
			}
			wbound 'jfc' wbound {
				self.scratch_pad.write_text(" jesus fucking christ ");
				break;
			}
			wbound 'gtfo' wbound {
				self.scratch_pad.write_text(" get the fuck out ");
				break;
			}
			wbound 'ffs' wbound {
				self.scratch_pad.write_text(" for fuck sake ");
				break;
			}
			wbound 'tfg' wbound {
				self.scratch_pad.write_text(" the former guy ");
				break;
			}
			wbound 'npc' wbound {
				self.scratch_pad.write_text(" non playable character ");
				break;
			}
			wbound 'dm' wbound {
				self.scratch_pad.write_text(" direct message");
				break;
			}
			wbound 'tfw' wbound {
				self.scratch_pad.write_text(" that feeling when ");
				break;
			}
			wbound 'tfti' wbound {
				self.scratch_pad.write_text(" thanks for the invite ");
				break;
			}
			wbound 'tifu' wbound {
				self.scratch_pad.write_text(" today i fucked up ");
			}
			wbound 'pfp' wbound {
				self.scratch_pad.write_text(" profile pic ");
				break;
			}
			wbound 'ootd' wbound {
				self.scratch_pad.write_text(" outfit of the day ");
				break;
			}
			wbound 'tmi' wbound {
				self.scratch_pad.write_text(" too much information ");
				break;
			}
			wbound 'tbt' wbound {
				self.scratch_pad.write_text(" throw back thursday ");
				break;
			}
			wbound 'tyt' wbound {
				self.scratch_pad.write_text(" take your time ");
				break;
			}
			wbound 'fyp' wbound {
				self.scratch_pad.write_text(" for you page ");
				break;
			}
			wbound 'icymi' wbound {
				self.scratch_pad.write_text(" in case you missed it , ");
				break;
			}
			wbound 'fomo' wbound {
				self.scratch_pad.write_text(" fear of missing out ");
				break;
			}
			wbound 'irl' wbound {
				self.scratch_pad.write_text(" in real life ");
				break;
			}
			wbound 'jk' wbound {
				self.scratch_pad.write_text(" just kidding ");
				break;
			}
			wbound 'lmk' wbound {
				self.scratch_pad.write_text(" let me know ");
				break;
			}
			wbound 'idc' wbound {
				self.scratch_pad.write_text(" i don\x27t care ");
				break;
			}
			wbound 'hbd' wbound {
				self.scratch_pad.write_text(" happy birthday ! ");
				break;
			}
			wbound 'g2g' wbound {
				self.scratch_pad.write_text(" got to go ");
				break;
			}
			wbound 'wfh' wbound {
				self.scratch_pad.write_text(" working from home ");
				break;
			}
			wbound 'wywh' wbound {
				self.scratch_pad.write_text(" wish you were here ");
				break;
			}
			wbound 'b2c' wbound {
				self.scratch_pad.write_text(" business to consumer ");
				break;
			}
			wbound 'fbf' wbound {
				self.scratch_pad.write_text(" flashback friday ");
				break;
			}
			wbound 'ftfy' wbound {
				self.scratch_pad.write_text(" fixed that for you ");
				break;
			}
			wbound 'iykyk' wbound {
				self.scratch_pad.write_text(" if you know you know");
				break;
			}
			wbound 'mfw' wbound {
				self.scratch_pad.write_text(" my face when ");
				break;
			}
			wbound 'tgif' wbound {
				self.scratch_pad.write_text(" thank god its friday ");
				break;
			}
			wbound 'wbw' wbound {
				self.scratch_pad.write_text(" way back wednesday ");
				break;
			}
			wbound 'yolo' wbound {
				self.scratch_pad.write_text(" you only live once ");
				break;
			}
			wbound 'mtfbwy' wbound {
				self.scratch_pad.write_text(" may the force be with you ");
				break;
			}
			wbound 'ttfn' wbound {
				self.scratch_pad.write_text(" ta ta for now ");
				break;
			}
			wbound 'Inc.' {
				self.scratch_pad.write_text(" incorporated , ");
				break;
			}
			wbound 'Int\x27l' wbound | wbound 'Intl' wbound {
				self.scratch_pad.write_text(" international ");
				break;
			}
			wbound 'Corp.' {
				self.scratch_pad.write_text(" corporation ");
			}
			wbound 'jct' wbound {
				self.scratch_pad.write_text(" junction ");
				break;
			}
			wbound 'jmo' wbound {
				self.scratch_pad.write_text(" just my opinion ");
				break;
			}
			wbound "i.e." {
				self.scratch_pad.write_text(" in other words , ");
				break;
			}
			wbound 'fwiw' wbound {
				self.scratch_pad.write_text(" for what its worth ");
				break;
			}
			wbound 'fyi' wbound {
				self.scratch_pad.write_text(" for your information ");
				break;
			}
			wbound 'gov\x27t' wbound | wbound 'govt' wbound {
				self.scratch_pad.write_text(" government ");
				break;
			}
			wbound 'hmu' wbound {
				self.scratch_pad.write_text(" hit me up ");
				break;
			}
			wbound 'hr' wbound {
				self.scratch_pad.write_text(" hour ");
				break;
			}
			wbound 'hrs' wbound {
				self.scratch_pad.write_text(" hours ");
				break;
			}
			wbound 'hwy' wbound {
				self.scratch_pad.write_text(" highway ");
				break;
			}
			wbound "EST" wbound {
				self.scratch_pad.write_text(" eastern standard time ");
				break;
			}
			wbound "est." {
				self.scratch_pad.write_text(" established ");
				break;
			}
			wbound 'eta.' {
				self.scratch_pad.write_text(" estimated time of arrival ");
				break;
			}
			wbound "etc." {
				self.scratch_pad.write_text(" et cetera ");
				break;
			}
			wbound 'approx.' {
				self.scratch_pad.write_text(" approximately ");
				break;
			}
			wbound 'expwy' wbound {
				self.scratch_pad.write_text(" express way ");
				break;
			}
			wbound 'Esq.' {
				self.scratch_pad.write_text(" esquire ");
				break;
			}
			wbound "dr." | wbound "DR." {
				self.scratch_pad.write_text(" drive ");
				break;
			}
			wbound 'drs.' {
				self.scratch_pad.write_text(" doctors ");
				break;
			}
			wbound "e.g." {
				self.scratch_pad.write_text(" for example , ");
				break;
			}
			wbound 'edt' wbound {
				self.scratch_pad.write_text(" eastern daylight time ");
				break;
			}
			wbound 'cst' wbound {
				self.scratch_pad.write_text(" central standard time ");
				break;
			}
			wbound 'diy' wbound {
				self.scratch_pad.write_text(" do it yourself ");
				break;
			}
			wbound 'dept' wbound {
				self.scratch_pad.write_text(" department ");
				break;
			}
			wbound 'equiv.' {
				self.scratch_pad.write_text(" equivalent ");
				break;
			}
			wbound 'assoc.' {
				self.scratch_pad.write_text(" associates ");
				break;
			}
			wbound 'assn.' {
				self.scratch_pad.write_text(" association ");
				break;
			}
			wbound 'Univ.' {
				self.scratch_pad.write_text(" university ");
				break;
			}
			wbound "Co." {
				self.scratch_pad.write_text(" company ");
				break;
			}
			wbound 'Col.' {
				self.scratch_pad.write_text(" colonel ");
				break;
			}
			wbound 'Capt.' {
				self.scratch_pad.write_text(" captain ");
				break;
			}
			wbound 'cdt' wbound {
				self.scratch_pad.write_text(" central daylight time ");
				break;
			}
			wbound 'ave.' {
				self.scratch_pad.write_text(" avenue ");
				break;
			}
			wbound 'avg.' {
				self.scratch_pad.write_text(" average ");
				break;
			}
			wbound 'b/c' wbound {
				self.scratch_pad.write_text(" because ");
				break;
			}
			wbound 'b/w' wbound {
				self.scratch_pad.write_text(" between ");
				break;
			}
			wbound 'bbq' wbound {
				self.scratch_pad.write_text(" barbeque ");
				break;
			}
			wbound 'bff' wbound {
				self.scratch_pad.write_text(" best friends forever ");
				break;
			}
			wbound 'bldg' wbound {
				self.scratch_pad.write_text(" building ");
				break;
			}
			wbound 'blk' wbound {
				self.scratch_pad.write_text(" block ");
				break;
			}
			wbound 'blvd' wbound {
				self.scratch_pad.write_text(" boulevard ");
				break;
			}
			wbound "brb" wbound {
				self.scratch_pad.write_text(" be right back ");
				break;
			}
			wbound 'btw' wbound {
				self.scratch_pad.write_text(" by the way ");
				break;
			}
			wbound 'Att. Gen.' {
				self.scratch_pad.write_text(" attorney general ");
				break;
			}
			wbound 'accts' wbound {
				self.scratch_pad.write_text(" accountants ");
				break;
			}
			wbound 'acct' wbound {
				self.scratch_pad.write_text(" accountant ");
				break;
			}
			wbound "ADT" wbound {
				self.scratch_pad.write_text(" atlantic daylight time ");
				break;
			}
			wbound 'afaic' wbound {
				self.scratch_pad.write_text(" as far as i am concerned ");
				break;
			}
			wbound 'afaik' wbound {
				self.scratch_pad.write_text(" as far as i know ");
				break;
			}
			wbound 'afk' wbound {
				self.scratch_pad.write_text(" away from keyboard ");
				break;
			}
			wbound 'agcy.' {
				self.scratch_pad.write_text(" agency ");
				break;
			}
			wbound "Apt." | wbound "APT." {
				self.scratch_pad.write_text(" apartment ");
				break;
			}
			wbound "ASAP" wbound {
				self.scratch_pad.write_text(" as soon as possible ");
				break;
			}
			wbound 'a.s.l.' {
				self.scratch_pad.write_text(" age , sex , and location. ");
				break;
			}
			wbound 'asst.' {
				self.scratch_pad.write_text(" assistant ");
				break;
			}
			wbound "AST" wbound {
				self.scratch_pad.write_text(" atlantic standard time ");
				break;
			}
			wbound 'Thurs.' | wbound 'Thur.' | wbound 'Thu.' {
				self.scratch_pad.write_text(" thursday ");
				break;
			}
			wbound 'Tues.' | wbound 'Tue.' {
				self.scratch_pad.write_text(" tuesday ");
				break;
			}
			wbound "St." {
				self.scratch_pad.write_text(" saint ");
				break;
			}
			wbound 'Nov.' {
				self.scratch_pad.write_text(" november ");
				break;
			}
			wbound 'Oct.' {
				self.scratch_pad.write_text(" october ");
				break;
			}
			wbound 'Sept.' | wbound 'Sep.' {
				self.scratch_pad.write_text(" september ");
				break;
			}
			wbound "MoF" wbound {
				self.scratch_pad.write_text(" male or female ");
				break;
			}
			wbound "NaN" wbound {
				self.scratch_pad.write_text(" not a number ");
				break;
			}
			wbound 'Jul.' {
				self.scratch_pad.write_text(" july ");
				break;
			}
			wbound 'Jun.' {
				self.scratch_pad.write_text(" june ");
				break;
			}
			wbound 'Mar.' {
				self.scratch_pad.write_text(" march ");
				break;
			}
			wbound 'Mon.' {
				self.scratch_pad.write_text(" monday ");
				break;
			}
			wbound "Ft." {
				self.scratch_pad.write_text(" fort ");
				break;
			}
			wbound "Dr." {
				self.scratch_pad.write_text(" doctor ");
				break;
			}
			wbound 'Apr.' {
				self.scratch_pad.write_text(" april ");
				break;
			}
			wbound 'Feb.' {
				self.scratch_pad.write_text(" february ");
				break;
			}
			wbound 'Fri.' {
				self.scratch_pad.write_text(" friday ");
				break;
			}
			wbound 'ctrl' wbound {
				self.scratch_pad.write_text(" control ");
				break;
			}
			wbound 'ctr' wbound {
				self.scratch_pad.write_text(" center ");
				break;
			}
			wbound 'pvt' wbound {
				self.scratch_pad.write_text(" private ");
				break;
			}
			wbound 'rd' wbound {
				self.scratch_pad.write_text(" road ");
				break;
			}
			wbound 'crk' wbound {
				self.scratch_pad.write_text(" creek ");
				break;
			}
			wbound 'mdw' wbound {
				self.scratch_pad.write_text(" meadow ");
				break;
			}
			wbound '\x27bout' wbound {
				self.scratch_pad.write_text(" about ");
				break;
			}
			wbound 'c\x27mon' wbound {
				self.scratch_pad.write_text(" come on ");
				break;
			}
			wbound 'auth. ver.' {
				self.scratch_pad.write_text(" authorized version ");
				break;
			}
			wbound 'auth.' {
				self.scratch_pad.write_text(" authorized ");
				break;
			}
			wbound 'aux.' {
				self.scratch_pad.write_text(" auxiliary ");
				break;
			}
			wbound 'shpt' wbound {
				self.scratch_pad.write_text(" shipment ");
				break;
			}
			wbound 'fwd' wbound {
				self.scratch_pad.write_text(" forward ");
				break;
			}
			wbound 'wpm' wbound {
				self.scratch_pad.write_text(" words per minute ");
				break;
			}
			wbound 'vol.' {
				self.scratch_pad.write_text(" volume ");
				break;
			}
			wbound "pp." {
				self.scratch_pad.write_text(" pages ");
				break;
			}

			/*--------------------*/
			/* START OF MANY2MANY */
			/*--------------------*/

			"\xA7"{2,} {
				self.scratch_pad.write_text(" sections ");
				break;
			}
			"\xA7" {
				self.scratch_pad.write_text(" section ");
				break;
			}
			"%" {
				self.scratch_pad.write_text(" percent ");
				break;
			}
			"&" {
				self.scratch_pad.write_text(" and ");
				break;
			}
			"( \u0361\xb0 \u035c\u0296 \u0361\xb0)" {
				self.scratch_pad.write_text(" lenny face ");
				break;
			}
			"/" {
				self.scratch_pad.write_text(" slash ");
				break;
			}
			":'(" | ":'-(" | "\U0001f622" {
				self.scratch_pad.write_text(" crying ");
				break;
			}
			":')" | ":'-)" {
				self.scratch_pad.write_text(" tears of happiness ");
				break;
			}
			":-*" | ":x" | ":*" {
				self.scratch_pad.write_text(" kissy face ");
				break;
			}
			/* SAD FACE RULE INCLUDES 'DISSAPPOINTED BUT RELIEVED' RULE AS WELL       */
			/* IT'S CLOSER TO APPLE'S 'SAD BUT RELIEVED' RULE MINUS THE RELIEVED PART */
			/* IT WOULD BE PRETTY RUDE TO KEEP THE RELIEVED PART IN LOL               */
			':L' | ":\\" | ":-/" | ":/" | "=\\" | "=/" | '=L' | ":<" | ":-(" | ':-c' | ":(" | ':c' | ":-<" | ":=(" | ":[" | ":-[" | "\U0001f625" {
				self.scratch_pad.write_text(" sad face ");
				break;
			}
			":-}" | ":^)" | "=)" | ":->" | "=]" | ":)" | ":>" | ":}" | ":-]" | ":]" | ":-)" {
				self.scratch_pad.write_text(" happy face ");
				break;
			}
			"=D" | ":-D" | ":D" {
				self.scratch_pad.write_text(" big grin ");
				break;
			}
			":-J" {
				self.scratch_pad.write_text(" tongue in cheek ");
				break;
			}
			'=O' | ":0" | "=0" | ':-O' | ':O' | ":-0" {
				self.scratch_pad.write_text(" shocked face ");
				break;
			}
			":-\u00fe" | ":\u00de" | ":-b" | ':-P' | ":\u00fe" | ":-\u00de" | ':P' {
				self.scratch_pad.write_text(" tongue sticking out ");
				break;
			}
			":X" | ":#" | ":-X" | ":-&" | ":-#" | ":&" {
				self.scratch_pad.write_text(" sealed lips ");
				break;
			}
			":|" | ":-|" {
				self.scratch_pad.write_text(" straight face ");
				break;
			}
			";]" | ";-)" | ";>" | ";^)" | ";D" | ";3" | ";)" | ";-]" | "\U0001f609" {
				self.scratch_pad.write_text(" wink ");
				break;
			}
			"<" {
				self.scratch_pad.write_text(" less than ");
				break;
			}
			"=" {
				self.scratch_pad.write_text(" equals ");
				break;
			}
			">" {
				self.scratch_pad.write_text(" greater than ");
				break;
			}
			">:[" | ">:(" | "\U0001f620" {
				self.scratch_pad.write_text(" angry face ");
				break;
			}
			">:-)" | ">;)" | "3:-)" | ">;-)" | "}:-)" | ">:3" | ">;3" | "3:)" | "}:)" | ">:)" | "\U0001f608" {
				self.scratch_pad.write_text(" devilish smile ");
				break;
			}
			">_<" {
				self.scratch_pad.write_text(" annoyed ");
				break;
			}
			"@" {
				self.scratch_pad.write_text(" at ");
				break;
			}
			"\u221C" {
				self.scratch_pad.write_text(" fourth root of ");
				break;
			}
			"\u221B" | "3\u221A" {
				self.scratch_pad.write_text(" cube root of ");
				break;
			}
			"\u221A" {
				self.scratch_pad.write_text(" square root of ");
				break;
			}
			"Bq" | "\u33c3" {
				self.scratch_pad.write_text(" becquerel");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"GiB" | "GB" {
				self.scratch_pad.write_text(" gigabyte");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"\u3393" | 'GHz' {
				self.scratch_pad.write_text(" gigahertz ");
				break;
			}
			"Gy" {
				self.scratch_pad.write_text(" gray");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"\u33cb" {
				self.scratch_pad.write_text(" horse power ");
				break;
			}
			"\u3390" | 'Hz' {
				self.scratch_pad.write_text(" hertz ");
				break;
			}
			"KiB" | "KB" {
				self.scratch_pad.write_text(" kilobyte");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"K\u207b\xb9" {
				self.scratch_pad.write_text(" reciprocal kelvin");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"MB" | "MiB" {
				self.scratch_pad.write_text(" megabyte");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			'MHz' | "\u3392" {
				self.scratch_pad.write_text(" megahertz ");
				break;
			}
			"\u33ab" | "MPa" {
				self.scratch_pad.write_text(" megapascal");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			wbound 'O\x27' {
				self.scratch_pad.write_text(" oh ");
				break;
			}
			'o-o' | 'o_o' {
				self.scratch_pad.write_text(" surprised face ");
				break;
			}
			"PiB" | "PB" {
				self.scratch_pad.write_text(" petabyte");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"TB" | "TiB" {
				self.scratch_pad.write_text(" terabyte");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"\\" {
				self.scratch_pad.write_text(" back slash ");
				break;
			}
			"_" {
				self.scratch_pad.write_text(" underscore ");
				break;
			}
			"`" {
				self.scratch_pad.write_text(" back tick ");
				break;
			}
			wbound 'b8' wbound {
				self.scratch_pad.write_text(" bait ");
				break;
			}
			wbound 'bps' wbound {
				self.scratch_pad.write_text(" bit");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" per second ");
				break;
			}
			"cm" | "\u339d" {
				self.scratch_pad.write_text(" centimeter");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"\u33a0" | "cm\u00b2" {
				self.scratch_pad.write_text(" squared centimeter");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"cm\u00b3" | "\u33a4" {
				self.scratch_pad.write_text(" cubic centimeter");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			wbound "cwt" wbound {
				self.scratch_pad.write_text(" hundred weight");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"dBFS" {
				self.scratch_pad.write_text(" decibel");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" at full scale ");
				break;
			}
			"dB" | "\u33c8" {
				self.scratch_pad.write_text(" decibel");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"dL" | "\u3397" {
				self.scratch_pad.write_text(" deciliter");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			wbound "dwt" wbound {
				self.scratch_pad.write_text(" penny weight");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			wbound 'gr8' wbound {
				self.scratch_pad.write_text(" great ");
				break;
			}
			"kV" {
				self.scratch_pad.write_text(" kilo volt");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"kW" | "\u33be" {
				self.scratch_pad.write_text(" kilo watt");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"kg" | "\u338f" {
				self.scratch_pad.write_text(" kilogram");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"kL" | "\u3398" {
				self.scratch_pad.write_text(" kiloliter");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"\u339e" | "km" {
				self.scratch_pad.write_text(" kilometer");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"\u33a2" | "km\xb2" {
				self.scratch_pad.write_text(" squared kilometer");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"\u33a6" | "km\xb3" {
				self.scratch_pad.write_text(" cubic kilometer");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			wbound "kts" wbound {
				self.scratch_pad.write_text(" knots ");
				break;
			}
			"\u33cf" | wbound "kt" wbound {
				self.scratch_pad.write_text(" knot ");
				break;
			}
			wbound 'l8r' wbound {
				self.scratch_pad.write_text(" later ");
				break;
			}
			wbound "lm" {
				self.scratch_pad.write_text(" lumen");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			wbound 'lol' wbound | wbound ' l0l' wbound {
				self.scratch_pad.write_text(" laugh out loud ");
				break;
			}
			wbound 'lvl' wbound {
				self.scratch_pad.write_text(" level ");
				break;
			}
			"\u33d3" | wbound "lx" wbound {
				self.scratch_pad.write_text(" lux");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("es");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"m/s/s" {
				self.scratch_pad.write_text(" meter");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" per second per second ");
				break;
			}
			"m/s" | "\u33a7" {
				self.scratch_pad.write_text(" meter");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" per second ");
				break;
			}
			"\u33a8" | "m/s\xb2" {
				self.scratch_pad.write_text(" squared meter");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" per second ");
				break;
			}
			"m/s\xb3" {
				self.scratch_pad.write_text(" cubic meter");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" per second ");
				break;
			}
			wbound '/s' wbound {
				self.scratch_pad.write_text(" sarcasm tag ");
				break;
			}
			wbound 'm8' wbound {
				self.scratch_pad.write_text(" mate ");
				break;
			}
			"mA" | "\u3383" {
				self.scratch_pad.write_text(" milliamp");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"mV" | "\u33b7" {
				self.scratch_pad.write_text(" millivolt");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"mW" | "\u33bd" {
				self.scratch_pad.write_text(" milliwatt");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"\u338e" | wbound "mg" wbound {
				self.scratch_pad.write_text(" milligram");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			wbound 'mi.' {
				self.scratch_pad.write_text(" mile");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"\u3396" | "mL" wbound {
				self.scratch_pad.write_text(" milliliter");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"\u339c" | wbound "mm" {
				self.scratch_pad.write_text(" millimeter");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"mm\xb2" | "\u339f" {
				self.scratch_pad.write_text(" squared millimeter");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"mm\xb3" | "\u33a3" {
				self.scratch_pad.write_text(" cubic millimeter");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			'mpg' {
				self.scratch_pad.write_text(" mile");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" per gallon ");
				break;
			}
			wbound 'mph' {
				self.scratch_pad.write_text(" mile");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" per hour ");
				break;
			}
			"\u33a1" | "m\xb2" {
				self.scratch_pad.write_text(" square meter");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"m\xb3" | "\u33a5" {
				self.scratch_pad.write_text(" cubic meter");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"m\u207b\xb9" {
				self.scratch_pad.write_text(" reciprocal meter");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"00b" wbound {
				self.scratch_pad.write_text("oob");
				break;
			}
			"00t" wbound {
				self.scratch_pad.write_text("oot");
				break;
			}
			wbound "nmi" wbound {
				self.scratch_pad.write_text(" nautical mile");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"\u339a" | wbound "nm" wbound {
				self.scratch_pad.write_text(" nanometer");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"\u33b1" | wbound "ns" wbound {
				self.scratch_pad.write_text(" nanosecond");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			wbound "fl. dr." {
				self.scratch_pad.write_text(" fluid dram");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			wbound "fl." {
				self.scratch_pad.write_text(" fluid ");
				break;
			}
			wbound "oz." {
				self.scratch_pad.write_text(" ounce");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			wbound "pt." {
				self.scratch_pad.write_text(" pint ");
				break;
			}
			wbound "pts" wbound {
				self.scratch_pad.write_text(" pints ");
				break;
			}
			wbound "qt" wbound {
				self.scratch_pad.write_text(" quart ");
				break;
			}
			wbound "qts" wbound {
				self.scratch_pad.write_text(" quarts ");
				break;
			}
			wbound "rad/s" wbound | "\u33ae" {
				self.scratch_pad.write_text(" radian");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" per second ");
				break;
			}
			wbound "tbsp" wbound {
				self.scratch_pad.write_text(" table spoon");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			wbound "tsp" wbound {
				self.scratch_pad.write_text(" tea spoon");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			wbound "yd" wbound {
				self.scratch_pad.write_text(" yard");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			wbound 'w8' wbound {
				self.scratch_pad.write_text(" wait ");
				break;
			}
			"XO" {
				self.scratch_pad.write_text(" hugs and kisses ");
				break;
			}
			"X-D" | "XD" | "\U0001f606" {
				self.scratch_pad.write_text(" laughing ");
				break;
			}
			"~" {
				self.scratch_pad.write_text(" tilde ");
				break;
			}
			"\u00a2" {
				self.scratch_pad.write_text(" cents ");
				break;
			}
			"\u00a4" {
				self.scratch_pad.write_text(" currency sign ");
				break;
			}
			"\u00a9" {
				self.scratch_pad.write_text(" copyright ");
				break;
			}
			"\u00ae" {
				self.scratch_pad.write_text(" registered trademark ");
				break;
			}
			"\u2103" | "\u00B0C" {
				self.scratch_pad.write_text(" degree");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" celsius ");
				break;
			}
			"\u2109" | "\u00B0F" {
				self.scratch_pad.write_text(" degree");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" fahrenheit ");
				break;
			}
			"\u00B0K" {
				self.scratch_pad.write_text(" degree");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" kelvin ");
				break;
			}
			"\u00B0" {
				self.scratch_pad.write_text(" degree");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"\u00B1" {
				self.scratch_pad.write_text(" plus or minus ");
				break;
			}
			"\u00bc" {
				self.scratch_pad.write_text(" one quarter ");
				break;
			}
			"\u00bd" {
				self.scratch_pad.write_text(" one half ");
				break;
			}
			"\u00be" {
				self.scratch_pad.write_text(" three quarters ");
				break;
			}
			"\u00d7" {
				self.scratch_pad.write_text(" times ");
				break;
			}
			"\u00f7" {
				self.scratch_pad.write_text(" divided by ");
				break;
			}
			"\u0295\u2022\u1d25\u2022 \u0294" | "\u0295\u2022\u1d25\u2022\u0294" | "\u0295 \u2022\u1d25\u2022\u0294" {
				self.scratch_pad.write_text(" bear face ");
				break;
			}
			"\u039e" {
				self.scratch_pad.write_text(" xi ");
				break;
			}
			"\u03c6" | "\u03a6" {
				self.scratch_pad.write_text(" phi ");
				break;
			}
			"\u0391" | "\u03b1" {
				self.scratch_pad.write_text(" alpha ");
				break;
			}
			"\u03b2" | "\u0392" {
				self.scratch_pad.write_text(" beta ");
				break;
			}
			"\u03b3" | "\u0393" {
				self.scratch_pad.write_text(" gamma ");
				break;
			}
			"\u0394" | "\u03b4" {
				self.scratch_pad.write_text(" delta ");
				break;
			}
			"\u03b5" | "\u0395" {
				self.scratch_pad.write_text(" epsilon ");
				break;
			}
			"\u0396" | "\u03b6" {
				self.scratch_pad.write_text(" zeta ");
				break;
			}
			"\u03b7" | "\u0397" {
				self.scratch_pad.write_text(" eta ");
				break;
			}
			"\u03b8" | "\u0398" {
				self.scratch_pad.write_text(" theta ");
				break;
			}
			"\u0399" | "\u03b9" {
				self.scratch_pad.write_text(" iota ");
				break;
			}
			"\u039a" | "\u03ba" {
				self.scratch_pad.write_text(" kappa ");
				break;
			}
			"\u03bb" | "\u039b" {
				self.scratch_pad.write_text(" lambda ");
				break;
			}
			"\u03bcA" {
				self.scratch_pad.write_text(" micro amp");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"\u03bcF" | "\u338c" {
				self.scratch_pad.write_text(" micro farad");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"\u03bcN" {
				self.scratch_pad.write_text(" micro newton");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"\u03bcW" {
				self.scratch_pad.write_text(" micro watt");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"\u03bcg" | "\u338d" {
				self.scratch_pad.write_text(" micro gram");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"\u03bcL" | "\u3395" {
				self.scratch_pad.write_text(" micro liter");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"\u03bcm" | "\u339b" {
				self.scratch_pad.write_text(" micro meter");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"\u33b2" | "\u03bcs" {
				self.scratch_pad.write_text(" micro second");
				if self.is_plural {
					self.is_plural = false;
					self.scratch_pad.write_text("s");
				}
				self.scratch_pad.write_text(" ");
				break;
			}
			"\u03bc" {
				self.scratch_pad.write_text(" micro ");
				break;
			}
			"\u03bf" | "\u039f" {
				self.scratch_pad.write_text(" omicron ");
				break;
			}
			"\u03c0" | "\u03a0" {
				self.scratch_pad.write_text(" pi ");
				break;
			}
			"\u03c1" | "\u03a1" {
				self.scratch_pad.write_text(" rho ");
				break;
			}
			"\u03a3" | "\u03f2" | "\u03c2" | "\u03f9" | "\u03c3" {
				self.scratch_pad.write_text(" sigma ");
				break;
			}
			"\u03a4" | "\u03c4" {
				self.scratch_pad.write_text(" tau ");
				break;
			}
			"\u03a5" | "\u03c5" {
				self.scratch_pad.write_text(" upsilon ");
				break;
			}
			"\u03c7" | "\u03a7" {
				self.scratch_pad.write_text(" chi ");
				break;
			}
			"\u03c8" | "\u03a8" {
				self.scratch_pad.write_text(" psi ");
				break;
			}
			"\u2030" | "\u0609" | "\x89" {
				self.scratch_pad.write_text(" per mille ");
				break;
			}
			"\u2031" | "\u060a" {
				self.scratch_pad.write_text(" per thousand ");
				break;
			}
			"\u0ca0_\u0ca0" | "\u0ca0__\u0ca0" | "\u0ca0\u76ca\u0ca0" {
				self.scratch_pad.write_text(" look of disapproval ");
				break;
			}
			"\u20a0" {
				self.scratch_pad.write_text(" european currency unit ");
				break;
			}
			"\u2100" {
				self.scratch_pad.write_text(" account of ");
				break;
			}
			"\u2101" {
				self.scratch_pad.write_text(" addressed to the subject ");
				break;
			}
			"\u2105" {
				self.scratch_pad.write_text(" care of ");
				break;
			}
			"\u2106" {
				self.scratch_pad.write_text(" cada una ");
				break;
			}
			"\u2113" {
				self.scratch_pad.write_text(" liquid volume ");
				break;
			}
			"\u2116" {
				self.scratch_pad.write_text(" number ");
				break;
			}
			"\u2117" {
				self.scratch_pad.write_text(" sound recording copyright ");
				break;
			}
			"\u2120" {
				self.scratch_pad.write_text(" service mark ");
				break;
			}
			"\u2122" {
				self.scratch_pad.write_text(" trade mark ");
				break;
			}
			"\u2127" | "\u03c9" | "\u2126" {
				self.scratch_pad.write_text(" ohm ");
				break;
			}
			"\u2150" {
				self.scratch_pad.write_text(" one seventh ");
				break;
			}
			"\u2151" {
				self.scratch_pad.write_text(" one ninth ");
				break;
			}
			"\u2152" {
				self.scratch_pad.write_text(" one tenth ");
				break;
			}
			"\u2153" {
				self.scratch_pad.write_text(" a third ");
				break;
			}
			"\u2154" {
				self.scratch_pad.write_text(" two thirds ");
				break;
			}
			"\u2155" {
				self.scratch_pad.write_text(" one fifth ");
				break;
			}
			"\u2156" {
				self.scratch_pad.write_text(" two fifths ");
				break;
			}
			"\u2157" {
				self.scratch_pad.write_text(" three fifths ");
				break;
			}
			"\u2158" {
				self.scratch_pad.write_text(" four fifths ");
				break;
			}
			"\u2159" {
				self.scratch_pad.write_text(" one sixth ");
				break;
			}
			"\u215a" {
				self.scratch_pad.write_text(" five sixths ");
				break;
			}
			"\u215b" {
				self.scratch_pad.write_text(" one eighth ");
				break;
			}
			"\u215c" {
				self.scratch_pad.write_text(" three eighths ");
				break;
			}
			"\u215d" {
				self.scratch_pad.write_text(" five eighths ");
				break;
			}
			"\u215e" {
				self.scratch_pad.write_text(" seven eighths ");
				break;
			}
			"\u215f" {
				self.scratch_pad.write_text(" one over ");
				break;
			}
			"\u2189" {
				self.scratch_pad.write_text(" zero third ");
				break;
			}
			"\u2194" {
				self.scratch_pad.write_text(" left right arrow ");
				break;
			}
			"\u2195" {
				self.scratch_pad.write_text(" up down arrow ");
				break;
			}
			"\u2196" {
				self.scratch_pad.write_text(" upper left arrow ");
				break;
			}
			"\u2197" {
				self.scratch_pad.write_text(" upper right arrow ");
				break;
			}
			"\u2198" {
				self.scratch_pad.write_text(" lower right arrow ");
				break;
			}
			"\u2199" {
				self.scratch_pad.write_text(" lower left arrow ");
				break;
			}
			"\u21a9" {
				self.scratch_pad.write_text(" leftwards arrow with hook ");
				break;
			}
			"\u21aa" {
				self.scratch_pad.write_text(" rightwards arrow with hook ");
				break;
			}
			"\u2205" {
				self.scratch_pad.write_text(" null sign ");
				break;
			}
			"\u2213" {
				self.scratch_pad.write_text(" minus or plus ");
				break;
			}
			"\u2248" {
				self.scratch_pad.write_text(" approximately equals ");
				break;
			}
			"\u2260" {
				self.scratch_pad.write_text(" does not equal ");
				break;
			}
			"\u2264" {
				self.scratch_pad.write_text(" less than or equal to ");
				break;
			}
			"\u2265" {
				self.scratch_pad.write_text(" greater than or equal to ");
				break;
			}
			"\u231a" {
				self.scratch_pad.write_text(" watch ");
				break;
			}
			"\u23f3" | "\u231b" {
				self.scratch_pad.write_text(" hour glass ");
				break;
			}
			"\u2328" {
				self.scratch_pad.write_text(" keyboard ");
				break;
			}
			"\u23cf" {
				self.scratch_pad.write_text(" eject ");
				break;
			}
			"\u23e9" {
				self.scratch_pad.write_text(" fast forward ");
				break;
			}
			"\u23ea" {
				self.scratch_pad.write_text(" rewind ");
				break;
			}
			"\u23eb" {
				self.scratch_pad.write_text(" double up arrow ");
				break;
			}
			"\u23ec" {
				self.scratch_pad.write_text(" double down arrow ");
				break;
			}
			"\u23ed" {
				self.scratch_pad.write_text(" track next ");
				break;
			}
			"\u23ee" {
				self.scratch_pad.write_text(" track previous ");
				break;
			}
			"\u23f0" {
				self.scratch_pad.write_text(" alarm clock ");
				break;
			}
			"\u23f1" {
				self.scratch_pad.write_text(" stop watch ");
				break;
			}
			"\u23f8" {
				self.scratch_pad.write_text(" pause button ");
				break;
			}
			"\u23f9" {
				self.scratch_pad.write_text(" stop button ");
				break;
			}
			"\u23fa" {
				self.scratch_pad.write_text(" record button ");
				break;
			}
			"\u25ab" | "\u25aa" {
				self.scratch_pad.write_text(" small square ");
				break;
			}
			"\u25b6" {
				self.scratch_pad.write_text(" forward arrow ");
				break;
			}
			"\u25c0" {
				self.scratch_pad.write_text(" backward arrow ");
				break;
			}
			"\u25fb" | "\u25fc" {
				self.scratch_pad.write_text(" medium square ");
				break;
			}
			"\u25fd" | "\u25fe" {
				self.scratch_pad.write_text(" medium small square ");
				break;
			}
			"\u2600" {
				self.scratch_pad.write_text(" sunny ");
				break;
			}
			"\u2601" {
				self.scratch_pad.write_text(" cloudy ");
				break;
			}
			"\u2602" | "\u2614" {
				self.scratch_pad.write_text(" umbrella ");
				break;
			}
			"\u2603" {
				self.scratch_pad.write_text(" snow man ");
				break;
			}
			"\u2604" {
				self.scratch_pad.write_text(" comet ");
				break;
			}
			"\u2611" {
				self.scratch_pad.write_text(" ballot box with check ");
				break;
			}
			"\u2612" {
				self.scratch_pad.write_text(" x mark ");
				break;
			}
			"\u2613" {
				self.scratch_pad.write_text(" saltire ");
				break;
			}
			"\u2615" {
				self.scratch_pad.write_text(" coffee ");
				break;
			}
			"\u2617" | "\u2616" {
				self.scratch_pad.write_text(" upward shogi piece ");
				break;
			}
			"\u2618" {
				self.scratch_pad.write_text(" sham rock ");
				break;
			}
			"\u261a" | "\u261c" | "\U0001f448" {
				self.scratch_pad.write_text(" point left ");
				break;
			}
			"\u261b" | "\u261e" | "\U0001f449" {
				self.scratch_pad.write_text(" point right ");
				break;
			}
			"\u261d" | "\U0001f446" {
				self.scratch_pad.write_text(" point up ");
				break;
			}
			"\u261f" | "\U0001f447" {
				self.scratch_pad.write_text(" point down ");
				break;
			}
			"\u2620" {
				self.scratch_pad.write_text(" skull and cross bones ");
				break;
			}
			"\u2621" {
				self.scratch_pad.write_text(" dangerous bend ");
				break;
			}
			"\u2622" {
				self.scratch_pad.write_text(" radioactive ");
				break;
			}
			"\u2623" {
				self.scratch_pad.write_text(" biohazard ");
				break;
			}
			"\u2626" {
				self.scratch_pad.write_text(" orthodox cross ");
				break;
			}
			"\u262a" {
				self.scratch_pad.write_text(" star and crescent ");
				break;
			}
			"\u262e" {
				self.scratch_pad.write_text(" peace symbol ");
				break;
			}
			"\u262f" {
				self.scratch_pad.write_text(" yin yang ");
				break;
			}
			"\u2638" {
				self.scratch_pad.write_text(" wheel of dharma ");
				break;
			}
			"\u2639" | "\U0001f626" {
				self.scratch_pad.write_text(" frowning ");
				break;
			}
			"\u263a" {
				self.scratch_pad.write_text(" relaxed ");
				break;
			}
			"\u263f" {
				self.scratch_pad.write_text(" mercury ");
				break;
			}
			"\u2640" {
				self.scratch_pad.write_text(" female sign ");
				break;
			}
			"\u2641" {
				self.scratch_pad.write_text(" earth ");
				break;
			}
			"\u2642" {
				self.scratch_pad.write_text(" male sign ");
				break;
			}
			"\u2643" {
				self.scratch_pad.write_text(" jupiter ");
				break;
			}
			"\u2644" {
				self.scratch_pad.write_text(" saturn ");
				break;
			}
			"\u2645" | "\u26e2" {
				self.scratch_pad.write_text(" uranus ");
				break;
			}
			"\u2646" {
				self.scratch_pad.write_text(" neptune ");
				break;
			}
			"\u2647" {
				self.scratch_pad.write_text(" pluto ");
				break;
			}
			"\u2648" {
				self.scratch_pad.write_text(" aries ");
				break;
			}
			"\u2649" {
				self.scratch_pad.write_text(" taurus ");
				break;
			}
			"\u264a" {
				self.scratch_pad.write_text(" gemini ");
				break;
			}
			"\u264b" {
				self.scratch_pad.write_text(" cancer ");
				break;
			}
			"\u264c" {
				self.scratch_pad.write_text(" leo ");
				break;
			}
			"\u264d" {
				self.scratch_pad.write_text(" virgo ");
				break;
			}
			"\u264e" {
				self.scratch_pad.write_text(" libra ");
				break;
			}
			"\u264f" {
				self.scratch_pad.write_text(" scorpius ");
				break;
			}
			"\u2650" {
				self.scratch_pad.write_text(" sagittarius ");
				break;
			}
			"\u2651" {
				self.scratch_pad.write_text(" capricorn ");
				break;
			}
			"\u2652" {
				self.scratch_pad.write_text(" aquarius ");
				break;
			}
			"\u2653" {
				self.scratch_pad.write_text(" pisces ");
				break;
			}
			"\u265a" | "\u2654" {
				self.scratch_pad.write_text(" king ");
				break;
			}
			"\u2655" | "\u265b" {
				self.scratch_pad.write_text(" queen ");
				break;
			}
			"\u265c" | "\u2656" {
				self.scratch_pad.write_text(" rook ");
				break;
			}
			"\u265d" | "\u2657" {
				self.scratch_pad.write_text(" bishop ");
				break;
			}
			"\u2658" | "\u265e" {
				self.scratch_pad.write_text(" knight ");
				break;
			}
			"\u2659" | "\u265f" {
				self.scratch_pad.write_text(" pawn ");
				break;
			}
			"\u2660" | "\u2664" {
				self.scratch_pad.write_text(" spades ");
				break;
			}
			"\u2667" | "\u2663" {
				self.scratch_pad.write_text(" clubs ");
				break;
			}
			"\u2661" | "\u2665" {
				self.scratch_pad.write_text(" hearts ");
				break;
			}
			"\u2666" | "\u2662" {
				self.scratch_pad.write_text(" diamonds ");
				break;
			}
			"\u2668" {
				self.scratch_pad.write_text(" hot springs ");
				break;
			}
			"\u266d" {
				self.scratch_pad.write_text(" flat ");
				break;
			}
			"\u266f" {
				self.scratch_pad.write_text(" sharp ");
				break;
			}
			"\u267b" {
				self.scratch_pad.write_text(" recycle ");
				break;
			}
			"\u267e" {
				self.scratch_pad.write_text(" infinity ");
				break;
			}
			"\u267f" {
				self.scratch_pad.write_text(" wheel chair ");
				break;
			}
			"\u2692" {
				self.scratch_pad.write_text(" hammer pick ");
				break;
			}
			"\u2693" {
				self.scratch_pad.write_text(" anchor ");
				break;
			}
			"\u2694" {
				self.scratch_pad.write_text(" crossed swords ");
				break;
			}
			"\u2695" {
				self.scratch_pad.write_text(" medical symbol ");
				break;
			}
			"\u2696" {
				self.scratch_pad.write_text(" scales ");
				break;
			}
			"\u2697" {
				self.scratch_pad.write_text(" alembic ");
				break;
			}
			"\u2699" {
				self.scratch_pad.write_text(" gear ");
				break;
			}
			"\u269b" {
				self.scratch_pad.write_text(" atom ");
				break;
			}
			"\u269c" {
				self.scratch_pad.write_text(" fleur de lis ");
				break;
			}
			"\u26a0" {
				self.scratch_pad.write_text(" warning sign ");
				break;
			}
			"\u26a1" {
				self.scratch_pad.write_text(" zap ");
				break;
			}
			"\u26a7" {
				self.scratch_pad.write_text(" transgender symbol ");
				break;
			}
			"\u26ab" | "\u26aa" {
				self.scratch_pad.write_text(" circle ");
				break;
			}
			"\u26b0" {
				self.scratch_pad.write_text(" coffin ");
				break;
			}
			"\u26bd" {
				self.scratch_pad.write_text(" soccer ");
				break;
			}
			"\u26be" {
				self.scratch_pad.write_text(" baseball ");
				break;
			}
			"\u26c4" {
				self.scratch_pad.write_text(" snow man ");
				break;
			}
			"\u26c5" {
				self.scratch_pad.write_text(" partly sunny ");
				break;
			}
			"\u26ce" {
				self.scratch_pad.write_text(" ophiuchus ");
				break;
			}
			"\u26cf" {
				self.scratch_pad.write_text(" pick ");
				break;
			}
			"\u26d4" {
				self.scratch_pad.write_text(" no entry ");
				break;
			}
			"\u26e9" {
				self.scratch_pad.write_text(" shinto shrine ");
				break;
			}
			"\u26ea" {
				self.scratch_pad.write_text(" church ");
				break;
			}
			"\u26f2" {
				self.scratch_pad.write_text(" fountain ");
				break;
			}
			"\u26f3" {
				self.scratch_pad.write_text(" golf ");
				break;
			}
			"\u26f4" {
				self.scratch_pad.write_text(" ferry ");
				break;
			}
			"\u26f5" {
				self.scratch_pad.write_text(" sail boat ");
				break;
			}
			"\u26fa" {
				self.scratch_pad.write_text(" tent ");
				break;
			}
			"\u26fd" {
				self.scratch_pad.write_text(" fuel pump ");
				break;
			}
			"\u2702" {
				self.scratch_pad.write_text(" scissors ");
				break;
			}
			"\u2705" {
				self.scratch_pad.write_text(" white check mark ");
				break;
			}
			"\u2708" {
				self.scratch_pad.write_text(" air plane ");
				break;
			}
			"\u2709" {
				self.scratch_pad.write_text(" envelope ");
				break;
			}
			"\u270a" {
				self.scratch_pad.write_text(" fist ");
				break;
			}
			"\u270b" {
				self.scratch_pad.write_text(" raised hand ");
				break;
			}
			"\u270c" {
				self.scratch_pad.write_text(" victory hand ");
				break;
			}
			"\u270d" {
				self.scratch_pad.write_text(" writing hand ");
				break;
			}
			"\u270f" | "\U0001f4dd" {
				self.scratch_pad.write_text(" pencil ");
				break;
			}
			"\u2712" {
				self.scratch_pad.write_text(" black nib ");
				break;
			}
			"\u2714" {
				self.scratch_pad.write_text(" heavy check mark ");
				break;
			}
			"\u2716" {
				self.scratch_pad.write_text(" heavy multiplication x ");
				break;
			}
			"\u271d" {
				self.scratch_pad.write_text(" cross ");
				break;
			}
			"\u2721" {
				self.scratch_pad.write_text(" star of david ");
				break;
			}
			"\u2728" {
				self.scratch_pad.write_text(" sparkles ");
				break;
			}
			"\u2733" {
				self.scratch_pad.write_text(" eight spoked asterisk ");
				break;
			}
			"\u2734" {
				self.scratch_pad.write_text(" eight pointed star ");
				break;
			}
			"\u2744" {
				self.scratch_pad.write_text(" snowflake ");
				break;
			}
			"\u2747" {
				self.scratch_pad.write_text(" sparkle ");
				break;
			}
			"\u274c" {
				self.scratch_pad.write_text(" tic tac toe x ");
				break;
			}
			"\u274e" {
				self.scratch_pad.write_text(" negative squared cross mark ");
				break;
			}
			"\u2753" | "\u2754" {
				self.scratch_pad.write_text(" bold question mark ");
				break;
			}
			"\u2757" | "\u2755" {
				self.scratch_pad.write_text(" bold exclamation point ");
				break;
			}
			"\u2763" {
				self.scratch_pad.write_text(" heart exclamation ");
				break;
			}
			"\u2764" {
				self.scratch_pad.write_text(" heart ");
				break;
			}
			"\u2795" {
				self.scratch_pad.write_text(" heavy plus sign ");
				break;
			}
			"\u2796" {
				self.scratch_pad.write_text(" heavy minus sign ");
				break;
			}
			"\u2797" {
				self.scratch_pad.write_text(" heavy division sign ");
				break;
			}
			"\u27a1" {
				self.scratch_pad.write_text(" right arrow ");
				break;
			}
			"\u27b0" | "\u27bf" {
				self.scratch_pad.write_text(" curly loop ");
				break;
			}
			"\u2934" {
				self.scratch_pad.write_text(" heading up arrow ");
				break;
			}
			"\u2935" {
				self.scratch_pad.write_text(" heading down arrow ");
				break;
			}
			"\u2b05" {
				self.scratch_pad.write_text(" left arrow ");
				break;
			}
			"\u2b06" {
				self.scratch_pad.write_text(" up arrow ");
				break;
			}
			"\u2b07" {
				self.scratch_pad.write_text(" down arrow ");
				break;
			}
			"\u2b1b" | "\u2b1c" {
				self.scratch_pad.write_text(" large square ");
				break;
			}
			"\u2b50" | "\U0001f31f" {
				self.scratch_pad.write_text(" star ");
				break;
			}
			"\u2b55" {
				self.scratch_pad.write_text(" tic tac toe circle ");
				break;
			}
			"\u33cc" {
				self.scratch_pad.write_text(" inches ");
				break;
			}
			"\U0001f300" {
				self.scratch_pad.write_text(" cyclone ");
				break;
			}
			"\U0001f301" {
				self.scratch_pad.write_text(" foggy ");
				break;
			}
			"\U0001f302" {
				self.scratch_pad.write_text(" closed umbrella ");
				break;
			}
			"\U0001f303" {
				self.scratch_pad.write_text(" night with stars ");
				break;
			}
			"\U0001f304" {
				self.scratch_pad.write_text(" sun rise over mountains ");
				break;
			}
			"\U0001f305" {
				self.scratch_pad.write_text(" sun rise ");
				break;
			}
			"\U0001f306" {
				self.scratch_pad.write_text(" city dusk ");
				break;
			}
			"\U0001f307" {
				self.scratch_pad.write_text(" city sunset ");
				break;
			}
			"\U0001f308" {
				self.scratch_pad.write_text(" rainbow ");
				break;
			}
			"\U0001f309" {
				self.scratch_pad.write_text(" bridge at night ");
				break;
			}
			"\U0001f30a" {
				self.scratch_pad.write_text(" ocean ");
				break;
			}
			"\U0001f30b" {
				self.scratch_pad.write_text(" volcano ");
				break;
			}
			"\U0001f30c" {
				self.scratch_pad.write_text(" milky way ");
				break;
			}
			"\U0001f30d" {
				self.scratch_pad.write_text(" earth africa ");
				break;
			}
			"\U0001f30e" {
				self.scratch_pad.write_text(" earth americas ");
				break;
			}
			"\U0001f30f" {
				self.scratch_pad.write_text(" earth asia ");
				break;
			}
			"\U0001f310" {
				self.scratch_pad.write_text(" globe with meridians ");
				break;
			}
			"\U0001f311" {
				self.scratch_pad.write_text(" new moon ");
				break;
			}
			"\U0001f312" {
				self.scratch_pad.write_text(" waxing crescent moon ");
				break;
			}
			"\U0001f313" {
				self.scratch_pad.write_text(" first quarter moon ");
				break;
			}
			"\U0001f314" {
				self.scratch_pad.write_text(" waxing gibbous moon ");
				break;
			}
			"\U0001f315" {
				self.scratch_pad.write_text(" full moon ");
				break;
			}
			"\U0001f316" {
				self.scratch_pad.write_text(" waning gibbous moon ");
				break;
			}
			"\U0001f317" {
				self.scratch_pad.write_text(" last quarter moon ");
				break;
			}
			"\U0001f318" {
				self.scratch_pad.write_text(" waning crescent moon ");
				break;
			}
			"\U0001f319" {
				self.scratch_pad.write_text(" crescent moon ");
				break;
			}
			"\U0001f31a" {
				self.scratch_pad.write_text(" new moon with face ");
				break;
			}
			"\U0001f31b" {
				self.scratch_pad.write_text(" first quarter moon with face ");
				break;
			}
			"\U0001f31c" {
				self.scratch_pad.write_text(" last quarter moon with face ");
				break;
			}
			"\U0001f31d" {
				self.scratch_pad.write_text(" full moon with face ");
				break;
			}
			"\U0001f31e" {
				self.scratch_pad.write_text(" sun with face ");
				break;
			}
			"\U0001f320" {
				self.scratch_pad.write_text(" stars ");
				break;
			}
			"\U0001f324" {
				self.scratch_pad.write_text(" white sun small cloud ");
				break;
			}
			"\U0001f326" {
				self.scratch_pad.write_text(" white sun rain cloud ");
				break;
			}
			"\U0001f327" {
				self.scratch_pad.write_text(" cloud rain ");
				break;
			}
			"\U0001f32b" {
				self.scratch_pad.write_text(" fog ");
				break;
			}
			"\U0001f32d" {
				self.scratch_pad.write_text(" hot dog ");
				break;
			}
			"\U0001f32e" {
				self.scratch_pad.write_text(" taco ");
				break;
			}
			"\U0001f330" {
				self.scratch_pad.write_text(" chestnut ");
				break;
			}
			"\U0001f331" {
				self.scratch_pad.write_text(" seedling ");
				break;
			}
			"\U0001f332" {
				self.scratch_pad.write_text(" evergreen tree ");
				break;
			}
			"\U0001f333" {
				self.scratch_pad.write_text(" deciduous tree ");
				break;
			}
			"\U0001f334" {
				self.scratch_pad.write_text(" palm tree ");
				break;
			}
			"\U0001f335" {
				self.scratch_pad.write_text(" cactus ");
				break;
			}
			"\U0001f336" {
				self.scratch_pad.write_text(" hot pepper ");
				break;
			}
			"\U0001f337" {
				self.scratch_pad.write_text(" tulip ");
				break;
			}
			"\U0001f338" {
				self.scratch_pad.write_text(" cherry blossom ");
				break;
			}
			"\U0001f339" {
				self.scratch_pad.write_text(" rose ");
				break;
			}
			"\U0001f33a" {
				self.scratch_pad.write_text(" hibiscus ");
				break;
			}
			"\U0001f33b" {
				self.scratch_pad.write_text(" sunflower ");
				break;
			}
			"\U0001f33c" {
				self.scratch_pad.write_text(" blossom ");
				break;
			}
			"\U0001f33d" {
				self.scratch_pad.write_text(" corn ");
				break;
			}
			"\U0001f33e" {
				self.scratch_pad.write_text(" ear of rice ");
				break;
			}
			"\U0001f33f" {
				self.scratch_pad.write_text(" herb ");
				break;
			}
			"\U0001f340" {
				self.scratch_pad.write_text(" four leaf clover ");
				break;
			}
			"\U0001f341" {
				self.scratch_pad.write_text(" maple leaf ");
				break;
			}
			"\U0001f342" {
				self.scratch_pad.write_text(" fallen leaf ");
				break;
			}
			"\U0001f343" {
				self.scratch_pad.write_text(" leaves ");
				break;
			}
			"\U0001f344" {
				self.scratch_pad.write_text(" mushroom ");
				break;
			}
			"\U0001f345" {
				self.scratch_pad.write_text(" tomato ");
				break;
			}
			"\U0001f346" {
				self.scratch_pad.write_text(" egg plant ");
				break;
			}
			"\U0001f347" {
				self.scratch_pad.write_text(" grapes ");
				break;
			}
			"\U0001f348" {
				self.scratch_pad.write_text(" melon ");
				break;
			}
			"\U0001f349" {
				self.scratch_pad.write_text(" watermelon ");
				break;
			}
			"\U0001f34a" {
				self.scratch_pad.write_text(" tangerine ");
				break;
			}
			"\U0001f34b" {
				self.scratch_pad.write_text(" lemon ");
				break;
			}
			"\U0001f34c" {
				self.scratch_pad.write_text(" banana ");
				break;
			}
			"\U0001f34d" {
				self.scratch_pad.write_text(" pineapple ");
				break;
			}
			"\U0001f34e" {
				self.scratch_pad.write_text(" apple ");
				break;
			}
			"\U0001f34f" {
				self.scratch_pad.write_text(" green apple ");
				break;
			}
			"\U0001f350" {
				self.scratch_pad.write_text(" pear ");
				break;
			}
			"\U0001f351" {
				self.scratch_pad.write_text(" peach ");
				break;
			}
			"\U0001f352" {
				self.scratch_pad.write_text(" cherries ");
				break;
			}
			"\U0001f353" {
				self.scratch_pad.write_text(" strawberry ");
				break;
			}
			"\U0001f354" {
				self.scratch_pad.write_text(" hamburger ");
				break;
			}
			"\U0001f355" {
				self.scratch_pad.write_text(" pizza ");
				break;
			}
			"\U0001f356" {
				self.scratch_pad.write_text(" meat on bone ");
				break;
			}
			"\U0001f357" {
				self.scratch_pad.write_text(" poultry leg ");
				break;
			}
			"\U0001f358" {
				self.scratch_pad.write_text(" rice cracker ");
				break;
			}
			"\U0001f359" {
				self.scratch_pad.write_text(" rice ball ");
				break;
			}
			"\U0001f35a" {
				self.scratch_pad.write_text(" rice ");
				break;
			}
			"\U0001f35b" {
				self.scratch_pad.write_text(" curry ");
				break;
			}
			"\U0001f35c" {
				self.scratch_pad.write_text(" ramen ");
				break;
			}
			"\U0001f35d" {
				self.scratch_pad.write_text(" spaghetti ");
				break;
			}
			"\U0001f35e" {
				self.scratch_pad.write_text(" bread ");
				break;
			}
			"\U0001f35f" {
				self.scratch_pad.write_text(" fries ");
				break;
			}
			"\U0001f360" {
				self.scratch_pad.write_text(" sweet potato ");
				break;
			}
			"\U0001f361" {
				self.scratch_pad.write_text(" dango ");
				break;
			}
			"\U0001f362" {
				self.scratch_pad.write_text(" oden ");
				break;
			}
			"\U0001f363" {
				self.scratch_pad.write_text(" sushi ");
				break;
			}
			"\U0001f364" {
				self.scratch_pad.write_text(" fried shrimp ");
				break;
			}
			"\U0001f365" {
				self.scratch_pad.write_text(" fish cake ");
				break;
			}
			"\U0001f368" | "\U0001f366" {
				self.scratch_pad.write_text(" ice cream ");
				break;
			}
			"\U0001f367" {
				self.scratch_pad.write_text(" shaved ice ");
				break;
			}
			"\U0001f369" {
				self.scratch_pad.write_text(" doughnut ");
				break;
			}
			"\U0001f36a" {
				self.scratch_pad.write_text(" cookie ");
				break;
			}
			"\U0001f36b" {
				self.scratch_pad.write_text(" chocolate bar ");
				break;
			}
			"\U0001f36c" {
				self.scratch_pad.write_text(" candy ");
				break;
			}
			"\U0001f36d" {
				self.scratch_pad.write_text(" lollipop ");
				break;
			}
			"\U0001f36e" {
				self.scratch_pad.write_text(" custard ");
				break;
			}
			"\U0001f36f" {
				self.scratch_pad.write_text(" honey pot ");
				break;
			}
			"\U0001f370" {
				self.scratch_pad.write_text(" cake ");
				break;
			}
			"\U0001f371" {
				self.scratch_pad.write_text(" bento ");
				break;
			}
			"\U0001f372" {
				self.scratch_pad.write_text(" stew ");
				break;
			}
			"\U0001f373" {
				self.scratch_pad.write_text(" cooking ");
				break;
			}
			"\U0001f374" {
				self.scratch_pad.write_text(" fork and knife ");
				break;
			}
			"\U0001f375" {
				self.scratch_pad.write_text(" tea ");
				break;
			}
			"\U0001f376" {
				self.scratch_pad.write_text(" sake ");
				break;
			}
			"\U0001f377" {
				self.scratch_pad.write_text(" wine glass ");
				break;
			}
			"\U0001f378" {
				self.scratch_pad.write_text(" cock tail ");
				break;
			}
			"\U0001f379" {
				self.scratch_pad.write_text(" tropical drink ");
				break;
			}
			"\U0001f37a" {
				self.scratch_pad.write_text(" beer ");
				break;
			}
			"\U0001f37b" {
				self.scratch_pad.write_text(" beers ");
				break;
			}
			"\U0001f37c" {
				self.scratch_pad.write_text(" baby bottle ");
				break;
			}
			"\U0001f37d" {
				self.scratch_pad.write_text(" fork and knife next to a plate ");
				break;
			}
			"\U0001f37f" {
				self.scratch_pad.write_text(" popcorn ");
				break;
			}
			"\U0001f380" {
				self.scratch_pad.write_text(" ribbon ");
				break;
			}
			"\U0001f381" {
				self.scratch_pad.write_text(" gift ");
				break;
			}
			"\U0001f382" {
				self.scratch_pad.write_text(" birthday ");
				break;
			}
			"\U0001f383" {
				self.scratch_pad.write_text(" jack oh lantern ");
				break;
			}
			"\U0001f384" {
				self.scratch_pad.write_text(" christmas tree ");
				break;
			}
			"\U0001f385" {
				self.scratch_pad.write_text(" santa ");
				break;
			}
			"\U0001f386" {
				self.scratch_pad.write_text(" fireworks ");
				break;
			}
			"\U0001f387" {
				self.scratch_pad.write_text(" sparkler ");
				break;
			}
			"\U0001f388" {
				self.scratch_pad.write_text(" balloon ");
				break;
			}
			"\U0001f389" {
				self.scratch_pad.write_text(" popper ");
				break;
			}
			"\U0001f38a" {
				self.scratch_pad.write_text(" confetti ball ");
				break;
			}
			"\U0001f38b" {
				self.scratch_pad.write_text(" tanabata tree ");
				break;
			}
			"\U0001f38c" {
				self.scratch_pad.write_text(" crossed flags ");
				break;
			}
			"\U0001f38d" {
				self.scratch_pad.write_text(" bamboo ");
				break;
			}
			"\U0001f38e" {
				self.scratch_pad.write_text(" dolls ");
				break;
			}
			"\U0001f38f" {
				self.scratch_pad.write_text(" flags ");
				break;
			}
			"\U0001f390" {
				self.scratch_pad.write_text(" wind chime ");
				break;
			}
			"\U0001f391" {
				self.scratch_pad.write_text(" moon viewing ceremony ");
				break;
			}
			"\U0001f392" {
				self.scratch_pad.write_text(" school satchel ");
				break;
			}
			"\U0001f393" {
				self.scratch_pad.write_text(" mortar board ");
				break;
			}
			"\U0001f396" {
				self.scratch_pad.write_text(" military medal ");
				break;
			}
			"\U0001f397" {
				self.scratch_pad.write_text(" reminder ribbon ");
				break;
			}
			"\U0001f3a4" | "\U0001f399" {
				self.scratch_pad.write_text(" microphone ");
				break;
			}
			"\U0001f39e" {
				self.scratch_pad.write_text(" film frames ");
				break;
			}
			"\U0001f3a0" {
				self.scratch_pad.write_text(" carousel horse ");
				break;
			}
			"\U0001f3a1" {
				self.scratch_pad.write_text(" ferris wheel ");
				break;
			}
			"\U0001f3a2" {
				self.scratch_pad.write_text(" roller coaster ");
				break;
			}
			"\U0001f3a3" {
				self.scratch_pad.write_text(" fishing pole and fish ");
				break;
			}
			"\U0001f3a5" {
				self.scratch_pad.write_text(" movie camera ");
				break;
			}
			"\U0001f3a6" {
				self.scratch_pad.write_text(" cinema ");
				break;
			}
			"\U0001f3a7" {
				self.scratch_pad.write_text(" head phones ");
				break;
			}
			"\U0001f3a8" {
				self.scratch_pad.write_text(" art ");
				break;
			}
			"\U0001f3a9" {
				self.scratch_pad.write_text(" tophat ");
				break;
			}
			"\U0001f3aa" {
				self.scratch_pad.write_text(" circus tent ");
				break;
			}
			"\U0001f3ab" {
				self.scratch_pad.write_text(" ticket ");
				break;
			}
			"\U0001f3ac" {
				self.scratch_pad.write_text(" clapper ");
				break;
			}
			"\U0001f3ad" {
				self.scratch_pad.write_text(" performing arts ");
				break;
			}
			"\U0001f3ae" {
				self.scratch_pad.write_text(" video game ");
				break;
			}
			"\U0001f3af" {
				self.scratch_pad.write_text(" dart ");
				break;
			}
			"\U0001f3b0" {
				self.scratch_pad.write_text(" slot machine ");
				break;
			}
			"\U0001f3b1" {
				self.scratch_pad.write_text(" eight ball ");
				break;
			}
			"\U0001f3b2" {
				self.scratch_pad.write_text(" game die ");
				break;
			}
			"\U0001f3b3" {
				self.scratch_pad.write_text(" bowling ");
				break;
			}
			"\U0001f3b4" {
				self.scratch_pad.write_text(" flower playing cards ");
				break;
			}
			"\U0001f3b5" {
				self.scratch_pad.write_text(" musical note ");
				break;
			}
			"\U0001f3b6" {
				self.scratch_pad.write_text(" musical notes ");
				break;
			}
			"\U0001f3b7" {
				self.scratch_pad.write_text(" saxophone ");
				break;
			}
			"\U0001f3b8" {
				self.scratch_pad.write_text(" guitar ");
				break;
			}
			"\U0001f3b9" {
				self.scratch_pad.write_text(" musical keyboard ");
				break;
			}
			"\U0001f3ba" {
				self.scratch_pad.write_text(" trumpet ");
				break;
			}
			"\U0001f3bb" {
				self.scratch_pad.write_text(" violin ");
				break;
			}
			"\U0001f3bc" {
				self.scratch_pad.write_text(" musical score ");
				break;
			}
			"\U0001f3bd" {
				self.scratch_pad.write_text(" running shirt with sash ");
				break;
			}
			"\U0001f3be" {
				self.scratch_pad.write_text(" tennis ");
				break;
			}
			"\U0001f3bf" {
				self.scratch_pad.write_text(" ski ");
				break;
			}
			"\U0001f3c0" {
				self.scratch_pad.write_text(" basketball ");
				break;
			}
			"\U0001f3c1" {
				self.scratch_pad.write_text(" checkered flag ");
				break;
			}
			"\U0001f3c2" {
				self.scratch_pad.write_text(" snow boarder ");
				break;
			}
			"\U0001f3c3" {
				self.scratch_pad.write_text(" person running ");
				break;
			}
			"\U0001f3c4" {
				self.scratch_pad.write_text(" person surfing ");
				break;
			}
			"\U0001f3c5" {
				self.scratch_pad.write_text(" medal ");
				break;
			}
			"\U0001f3c6" {
				self.scratch_pad.write_text(" trophy ");
				break;
			}
			"\U0001f3c7" {
				self.scratch_pad.write_text(" horse racing ");
				break;
			}
			"\U0001f3c8" {
				self.scratch_pad.write_text(" football ");
				break;
			}
			"\U0001f3c9" {
				self.scratch_pad.write_text(" rugby football ");
				break;
			}
			"\U0001f3ca" {
				self.scratch_pad.write_text(" person swimming ");
				break;
			}
			"\U0001f3cf" {
				self.scratch_pad.write_text(" cricket game ");
				break;
			}
			"\U0001f3d6" {
				self.scratch_pad.write_text(" beach ");
				break;
			}
			"\U0001f3db" {
				self.scratch_pad.write_text(" classical building ");
				break;
			}
			"\U0001f3dd" {
				self.scratch_pad.write_text(" island ");
				break;
			}
			"\U0001f3de" {
				self.scratch_pad.write_text(" park ");
				break;
			}
			"\U0001f3e0" {
				self.scratch_pad.write_text(" house ");
				break;
			}
			"\U0001f3e1" {
				self.scratch_pad.write_text(" house with garden ");
				break;
			}
			"\U0001f3e2" {
				self.scratch_pad.write_text(" office ");
				break;
			}
			"\U0001f3e3" {
				self.scratch_pad.write_text(" post office ");
				break;
			}
			"\U0001f3e4" {
				self.scratch_pad.write_text(" european post office ");
				break;
			}
			"\U0001f3e5" {
				self.scratch_pad.write_text(" hospital ");
				break;
			}
			"\U0001f3e6" {
				self.scratch_pad.write_text(" bank ");
				break;
			}
			"\U0001f3e7" {
				self.scratch_pad.write_text(" A t m ");
				break;
			}
			"\U0001f3e8" {
				self.scratch_pad.write_text(" hotel ");
				break;
			}
			"\U0001f3e9" {
				self.scratch_pad.write_text(" love hotel ");
				break;
			}
			"\U0001f3ea" {
				self.scratch_pad.write_text(" convenience store ");
				break;
			}
			"\U0001f3eb" {
				self.scratch_pad.write_text(" school ");
				break;
			}
			"\U0001f3ec" {
				self.scratch_pad.write_text(" department store ");
				break;
			}
			"\U0001f3ed" {
				self.scratch_pad.write_text(" factory ");
				break;
			}
			"\U0001f3ee" {
				self.scratch_pad.write_text(" izakaya lantern ");
				break;
			}
			"\U0001f3ef" {
				self.scratch_pad.write_text(" japanese castle ");
				break;
			}
			"\U0001f3f0" {
				self.scratch_pad.write_text(" european castle ");
				break;
			}
			"\U0001f3f4" | "\U0001f3f3" {
				self.scratch_pad.write_text(" flag ");
				break;
			}
			"\U0001f3f5" {
				self.scratch_pad.write_text(" rosette ");
				break;
			}
			"\U0001f3f9" {
				self.scratch_pad.write_text(" bow and arrow ");
				break;
			}
			"\U0001f400" {
				self.scratch_pad.write_text(" rat ");
				break;
			}
			"\U0001f401" | "\U0001f42d" {
				self.scratch_pad.write_text(" mouse ");
				break;
			}
			"\U0001f402" {
				self.scratch_pad.write_text(" ox ");
				break;
			}
			"\U0001f403" {
				self.scratch_pad.write_text(" water buffalo ");
				break;
			}
			"\U0001f42e" | "\U0001f404" {
				self.scratch_pad.write_text(" cow ");
				break;
			}
			"\U0001f42f" | "\U0001f405" {
				self.scratch_pad.write_text(" tiger ");
				break;
			}
			"\U0001f406" {
				self.scratch_pad.write_text(" leopard ");
				break;
			}
			"\U0001f430" | "\U0001f407" {
				self.scratch_pad.write_text(" rabbit ");
				break;
			}
			"\U0001f431" | "\U0001f408" {
				self.scratch_pad.write_text(" cat ");
				break;
			}
			"\U0001f432" | "\U0001f409" {
				self.scratch_pad.write_text(" dragon ");
				break;
			}
			"\U0001f40a" {
				self.scratch_pad.write_text(" crocodile ");
				break;
			}
			"\U0001f433" | "\U0001f40b" {
				self.scratch_pad.write_text(" whale ");
				break;
			}
			"\U0001f40c" {
				self.scratch_pad.write_text(" snail ");
				break;
			}
			"\U0001f40d" {
				self.scratch_pad.write_text(" snake ");
				break;
			}
			"\U0001f434" | "\U0001f40e" {
				self.scratch_pad.write_text(" horse ");
				break;
			}
			"\U0001f40f" {
				self.scratch_pad.write_text(" ram ");
				break;
			}
			"\U0001f410" {
				self.scratch_pad.write_text(" goat ");
				break;
			}
			"\U0001f411" {
				self.scratch_pad.write_text(" sheep ");
				break;
			}
			"\U0001f412" | "\U0001f435" {
				self.scratch_pad.write_text(" monkey ");
				break;
			}
			"\U0001f413" {
				self.scratch_pad.write_text(" rooster ");
				break;
			}
			"\U0001f414" {
				self.scratch_pad.write_text(" chicken ");
				break;
			}
			"\U0001f436" | "\U0001f415" {
				self.scratch_pad.write_text(" dog ");
				break;
			}
			"\U0001f437" | "\U0001f416" {
				self.scratch_pad.write_text(" pig ");
				break;
			}
			"\U0001f417" {
				self.scratch_pad.write_text(" boar ");
				break;
			}
			"\U0001f418" {
				self.scratch_pad.write_text(" elephant ");
				break;
			}
			"\U0001f419" {
				self.scratch_pad.write_text(" octopus ");
				break;
			}
			"\U0001f41a" {
				self.scratch_pad.write_text(" shell ");
				break;
			}
			"\U0001f41b" {
				self.scratch_pad.write_text(" bug ");
				break;
			}
			"\U0001f41c" {
				self.scratch_pad.write_text(" ant ");
				break;
			}
			"\U0001f41d" {
				self.scratch_pad.write_text(" bee ");
				break;
			}
			"\U0001f41e" {
				self.scratch_pad.write_text(" lady beetle ");
				break;
			}
			"\U0001f41f" {
				self.scratch_pad.write_text(" fish ");
				break;
			}
			"\U0001f420" {
				self.scratch_pad.write_text(" tropical fish ");
				break;
			}
			"\U0001f421" {
				self.scratch_pad.write_text(" blow fish ");
				break;
			}
			"\U0001f422" {
				self.scratch_pad.write_text(" turtle ");
				break;
			}
			"\U0001f423" {
				self.scratch_pad.write_text(" hatching chick ");
				break;
			}
			"\U0001f424" {
				self.scratch_pad.write_text(" baby chick ");
				break;
			}
			"\U0001f425" {
				self.scratch_pad.write_text(" hatched chick ");
				break;
			}
			"\U0001f426" {
				self.scratch_pad.write_text(" bird ");
				break;
			}
			"\U0001f427" {
				self.scratch_pad.write_text(" penguin ");
				break;
			}
			"\U0001f428" {
				self.scratch_pad.write_text(" koala ");
				break;
			}
			"\U0001f429" {
				self.scratch_pad.write_text(" poodle ");
				break;
			}
			"\U0001f42a" {
				self.scratch_pad.write_text(" camel ");
				break;
			}
			"\U0001f42c" {
				self.scratch_pad.write_text(" dolphin ");
				break;
			}
			"\U0001f438" {
				self.scratch_pad.write_text(" frog ");
				break;
			}
			"\U0001f439" {
				self.scratch_pad.write_text(" hamster ");
				break;
			}
			"\U0001f43a" {
				self.scratch_pad.write_text(" wolf ");
				break;
			}
			"\U0001f43b" {
				self.scratch_pad.write_text(" bear ");
				break;
			}
			"\U0001f43c" {
				self.scratch_pad.write_text(" panda ");
				break;
			}
			"\U0001f43d" {
				self.scratch_pad.write_text(" pig nose ");
				break;
			}
			"\U0001f43e" {
				self.scratch_pad.write_text(" feet ");
				break;
			}
			"\U0001f43f" {
				self.scratch_pad.write_text(" chipmunk ");
				break;
			}
			"\U0001f440" {
				self.scratch_pad.write_text(" eyes ");
				break;
			}
			"\U0001f441" {
				self.scratch_pad.write_text(" eye ");
				break;
			}
			"\U0001f442" {
				self.scratch_pad.write_text(" ear ");
				break;
			}
			"\U0001f443" {
				self.scratch_pad.write_text(" nose ");
				break;
			}
			"\U0001f444" {
				self.scratch_pad.write_text(" lips ");
				break;
			}
			"\U0001f445" {
				self.scratch_pad.write_text(" tongue ");
				break;
			}
			"\U0001f44a" {
				self.scratch_pad.write_text(" punching hand ");
				break;
			}
			"\U0001f44b" {
				self.scratch_pad.write_text(" waving hand ");
				break;
			}
			"\U0001f44c" {
				self.scratch_pad.write_text(" okay hand ");
				break;
			}
			"\U0001f44d" {
				self.scratch_pad.write_text(" thumbs up ");
				break;
			}
			"\U0001f44e" {
				self.scratch_pad.write_text(" thumbs down ");
				break;
			}
			"\U0001f44f" {
				self.scratch_pad.write_text(" clap ");
				break;
			}
			"\U0001f450" {
				self.scratch_pad.write_text(" open hands ");
				break;
			}
			"\U0001f451" {
				self.scratch_pad.write_text(" crown ");
				break;
			}
			"\U0001f452" {
				self.scratch_pad.write_text(" womans hat ");
				break;
			}
			"\U0001f453" {
				self.scratch_pad.write_text(" eye glasses ");
				break;
			}
			"\U0001f454" {
				self.scratch_pad.write_text(" neck tie ");
				break;
			}
			"\U0001f455" {
				self.scratch_pad.write_text(" shirt ");
				break;
			}
			"\U0001f456" {
				self.scratch_pad.write_text(" jeans ");
				break;
			}
			"\U0001f457" {
				self.scratch_pad.write_text(" dress ");
				break;
			}
			"\U0001f458" {
				self.scratch_pad.write_text(" kimono ");
				break;
			}
			"\U0001f459" {
				self.scratch_pad.write_text(" bikini ");
				break;
			}
			"\U0001f45a" {
				self.scratch_pad.write_text(" womans clothes ");
				break;
			}
			"\U0001f45b" {
				self.scratch_pad.write_text(" purse ");
				break;
			}
			"\U0001f45c" {
				self.scratch_pad.write_text(" hand bag ");
				break;
			}
			"\U0001f45d" {
				self.scratch_pad.write_text(" pouch ");
				break;
			}
			"\U0001f45e" {
				self.scratch_pad.write_text(" mans shoe ");
				break;
			}
			"\U0001f45f" {
				self.scratch_pad.write_text(" athletic shoe ");
				break;
			}
			"\U0001f460" {
				self.scratch_pad.write_text(" high heel ");
				break;
			}
			"\U0001f461" {
				self.scratch_pad.write_text(" sandal ");
				break;
			}
			"\U0001f462" {
				self.scratch_pad.write_text(" boot ");
				break;
			}
			"\U0001f463" {
				self.scratch_pad.write_text(" foot prints ");
				break;
			}
			"\U0001f464" {
				self.scratch_pad.write_text(" bust in silhouette ");
				break;
			}
			"\U0001f465" {
				self.scratch_pad.write_text(" busts in silhouette ");
				break;
			}
			"\U0001f466" {
				self.scratch_pad.write_text(" boy ");
				break;
			}
			"\U0001f467" {
				self.scratch_pad.write_text(" girl ");
				break;
			}
			"\U0001f468" {
				self.scratch_pad.write_text(" man ");
				break;
			}
			"\U0001f469" {
				self.scratch_pad.write_text(" woman ");
				break;
			}
			"\U0001f46a" {
				self.scratch_pad.write_text(" family ");
				break;
			}
			"\U0001f46b" {
				self.scratch_pad.write_text(" couple ");
				break;
			}
			"\U0001f46c" {
				self.scratch_pad.write_text(" two men holding hands ");
				break;
			}
			"\U0001f46d" {
				self.scratch_pad.write_text(" two women holding hands ");
				break;
			}
			"\U0001f46e" {
				self.scratch_pad.write_text(" police officer ");
				break;
			}
			"\U0001f46f" {
				self.scratch_pad.write_text(" people with bunny ears partying ");
				break;
			}
			"\U0001f470" {
				self.scratch_pad.write_text(" person with veil ");
				break;
			}
			"\U0001f471" {
				self.scratch_pad.write_text(" blond haired person ");
				break;
			}
			"\U0001f472" {
				self.scratch_pad.write_text(" man with chinese cap ");
				break;
			}
			"\U0001f473" {
				self.scratch_pad.write_text(" person wearing turban ");
				break;
			}
			"\U0001f474" {
				self.scratch_pad.write_text(" older man ");
				break;
			}
			"\U0001f475" {
				self.scratch_pad.write_text(" older woman ");
				break;
			}
			"\U0001f476" {
				self.scratch_pad.write_text(" baby ");
				break;
			}
			"\U0001f477" {
				self.scratch_pad.write_text(" construction worker ");
				break;
			}
			"\U0001f478" {
				self.scratch_pad.write_text(" princess ");
				break;
			}
			"\U0001f479" {
				self.scratch_pad.write_text(" japanese ogre ");
				break;
			}
			"\U0001f47a" {
				self.scratch_pad.write_text(" japanese goblin ");
				break;
			}
			"\U0001f47b" {
				self.scratch_pad.write_text(" ghost ");
				break;
			}
			"\U0001f47c" {
				self.scratch_pad.write_text(" angel ");
				break;
			}
			"\U0001f47d" {
				self.scratch_pad.write_text(" alien ");
				break;
			}
			"\U0001f47e" {
				self.scratch_pad.write_text(" space invader ");
				break;
			}
			"\U0001f47f" {
				self.scratch_pad.write_text(" demon ");
				break;
			}
			"\U0001f480" {
				self.scratch_pad.write_text(" skull ");
				break;
			}
			"\U0001f481" {
				self.scratch_pad.write_text(" person tipping hand ");
				break;
			}
			"\U0001f482" {
				self.scratch_pad.write_text(" guard ");
				break;
			}
			"\U0001f483" {
				self.scratch_pad.write_text(" dancer ");
				break;
			}
			"\U0001f484" {
				self.scratch_pad.write_text(" lip stick ");
				break;
			}
			"\U0001f485" {
				self.scratch_pad.write_text(" nail care ");
				break;
			}
			"\U0001f486" {
				self.scratch_pad.write_text(" person getting a massage ");
				break;
			}
			"\U0001f487" {
				self.scratch_pad.write_text(" person getting a haircut ");
				break;
			}
			"\U0001f488" {
				self.scratch_pad.write_text(" barber ");
				break;
			}
			"\U0001f489" {
				self.scratch_pad.write_text(" syringe ");
				break;
			}
			"\U0001f48a" {
				self.scratch_pad.write_text(" pill ");
				break;
			}
			"\U0001f48b" {
				self.scratch_pad.write_text(" kiss mark ");
				break;
			}
			"\U0001f48c" {
				self.scratch_pad.write_text(" love letter ");
				break;
			}
			"\U0001f48d" {
				self.scratch_pad.write_text(" ring ");
				break;
			}
			"\U0001f48e" {
				self.scratch_pad.write_text(" gem ");
				break;
			}
			"\U0001f48f" {
				self.scratch_pad.write_text(" couple kiss ");
				break;
			}
			"\U0001f490" {
				self.scratch_pad.write_text(" bouquet ");
				break;
			}
			"\U0001f491" {
				self.scratch_pad.write_text(" couple with heart ");
				break;
			}
			"\U0001f492" {
				self.scratch_pad.write_text(" wedding ");
				break;
			}
			"\U0001f493" {
				self.scratch_pad.write_text(" heart beat ");
				break;
			}
			"\U0001f494" {
				self.scratch_pad.write_text(" broken heart ");
				break;
			}
			"\U0001f495" {
				self.scratch_pad.write_text(" two hearts ");
				break;
			}
			"\U0001f496" {
				self.scratch_pad.write_text(" sparkling heart ");
				break;
			}
			"\U0001f497" {
				self.scratch_pad.write_text(" heart pulse ");
				break;
			}
			"\U0001f498" {
				self.scratch_pad.write_text(" cupid ");
				break;
			}
			"\U0001f499" {
				self.scratch_pad.write_text(" blue heart ");
				break;
			}
			"\U0001f49a" {
				self.scratch_pad.write_text(" green heart ");
				break;
			}
			"\U0001f49b" {
				self.scratch_pad.write_text(" yellow heart ");
				break;
			}
			"\U0001f49c" {
				self.scratch_pad.write_text(" purple heart ");
				break;
			}
			"\U0001f49d" {
				self.scratch_pad.write_text(" gift heart ");
				break;
			}
			"\U0001f49e" {
				self.scratch_pad.write_text(" revolving hearts ");
				break;
			}
			"\U0001f49f" {
				self.scratch_pad.write_text(" heart decoration ");
				break;
			}
			"\U0001f4a0" {
				self.scratch_pad.write_text(" diamond shape with a dot inside ");
				break;
			}
			"\U0001f4a1" {
				self.scratch_pad.write_text(" bulb ");
				break;
			}
			"\U0001f4a2" {
				self.scratch_pad.write_text(" anger mark ");
				break;
			}
			"\U0001f4a3" {
				self.scratch_pad.write_text(" bomb ");
				break;
			}
			"\U0001f4a4" {
				self.scratch_pad.write_text(" zeez ");
				break;
			}
			"\U0001f4a5" {
				self.scratch_pad.write_text(" boom ");
				break;
			}
			"\U0001f4a6" {
				self.scratch_pad.write_text(" sweat drops ");
				break;
			}
			"\U0001f4a7" {
				self.scratch_pad.write_text(" droplet ");
				break;
			}
			"\U0001f4a8" {
				self.scratch_pad.write_text(" dashing away ");
				break;
			}
			"\U0001f4a9" {
				self.scratch_pad.write_text(" poop ");
				break;
			}
			"\U0001f4aa" {
				self.scratch_pad.write_text(" muscle ");
				break;
			}
			"\U0001f4ab" | "\U0001f635" {
				self.scratch_pad.write_text(" dizzy ");
				break;
			}
			"\U0001f5e8" | "\U0001f4ad" | "\U0001f4ac" | "\U0001f5ef" {
				self.scratch_pad.write_text(" speech bubble ");
				break;
			}
			"\U0001f4ae" {
				self.scratch_pad.write_text(" white flower ");
				break;
			}
			/* HUNDRED POINTS EMOJI                                  */
			/* COULD BE EMPHASIZED AS "KEEPING IT A HUNDRED PERCENT" */
			"\U0001f4af" {
				self.scratch_pad.write_text(" keeping it a hundred percent ");
				break;
			}
			"\U0001f4b0" {
				self.scratch_pad.write_text(" money bag ");
				break;
			}
			"\U0001f4b1" {
				self.scratch_pad.write_text(" currency exchange ");
				break;
			}
			"\U0001f4b2" {
				self.scratch_pad.write_text(" heavy dollar sign ");
				break;
			}
			"\U0001f4b3" {
				self.scratch_pad.write_text(" credit card ");
				break;
			}
			"\U0001f4b4" {
				self.scratch_pad.write_text(" yen bill ");
				break;
			}
			"\U0001f4b5" {
				self.scratch_pad.write_text(" dollar bill ");
				break;
			}
			"\U0001f4b6" {
				self.scratch_pad.write_text(" euro bill ");
				break;
			}
			"\U0001f4b7" {
				self.scratch_pad.write_text(" pound bill ");
				break;
			}
			"\U0001f4b8" {
				self.scratch_pad.write_text(" money with wings ");
				break;
			}
			"\U0001f4b9" {
				self.scratch_pad.write_text(" chart ");
				break;
			}
			"\U0001f4ba" {
				self.scratch_pad.write_text(" seat ");
				break;
			}
			"\U0001f4bb" {
				self.scratch_pad.write_text(" computer ");
				break;
			}
			"\U0001f4bc" {
				self.scratch_pad.write_text(" brief case ");
				break;
			}
			"\U0001f4bd" {
				self.scratch_pad.write_text(" mini disk ");
				break;
			}
			"\U0001f4be" {
				self.scratch_pad.write_text(" floppy disk ");
				break;
			}
			"\U0001f4bf" {
				self.scratch_pad.write_text(" c d ");
				break;
			}
			"\U0001f4c0" {
				self.scratch_pad.write_text(" d v d ");
				break;
			}
			"\U0001f4c1" {
				self.scratch_pad.write_text(" folder icon ");
				break;
			}
			"\U0001f4c2" {
				self.scratch_pad.write_text(" open folder icon ");
				break;
			}
			"\U0001f4c3" {
				self.scratch_pad.write_text(" page with curl ");
				break;
			}
			"\U0001f4c4" {
				self.scratch_pad.write_text(" page facing up ");
				break;
			}
			"\U0001f4c5" {
				self.scratch_pad.write_text(" date ");
				break;
			}
			"\U0001f4c6" {
				self.scratch_pad.write_text(" calendar ");
				break;
			}
			"\U0001f4c7" {
				self.scratch_pad.write_text(" card index ");
				break;
			}
			"\U0001f4c8" {
				self.scratch_pad.write_text(" chart with upwards trend ");
				break;
			}
			"\U0001f4c9" {
				self.scratch_pad.write_text(" chart with downwards trend ");
				break;
			}
			"\U0001f4ca" {
				self.scratch_pad.write_text(" bar chart ");
				break;
			}
			"\U0001f4cb" {
				self.scratch_pad.write_text(" clip board ");
				break;
			}
			"\U0001f4cc" {
				self.scratch_pad.write_text(" push pin ");
				break;
			}
			"\U0001f4cd" {
				self.scratch_pad.write_text(" round push pin ");
				break;
			}
			"\U0001f4ce" {
				self.scratch_pad.write_text(" paper clip ");
				break;
			}
			"\U0001f4cf" {
				self.scratch_pad.write_text(" straight ruler ");
				break;
			}
			"\U0001f4d0" {
				self.scratch_pad.write_text(" triangular ruler ");
				break;
			}
			"\U0001f4d1" {
				self.scratch_pad.write_text(" book mark tabs ");
				break;
			}
			"\U0001f4d2" {
				self.scratch_pad.write_text(" ledger ");
				break;
			}
			"\U0001f4d3" {
				self.scratch_pad.write_text(" note book ");
				break;
			}
			"\U0001f4d4" {
				self.scratch_pad.write_text(" note book with decorative cover ");
				break;
			}
			"\U0001f4d5" {
				self.scratch_pad.write_text(" closed book ");
				break;
			}
			"\U0001f4d6" {
				self.scratch_pad.write_text(" open book ");
				break;
			}
			"\U0001f4d7" {
				self.scratch_pad.write_text(" green book ");
				break;
			}
			"\U0001f4d8" {
				self.scratch_pad.write_text(" blue book ");
				break;
			}
			"\U0001f4d9" {
				self.scratch_pad.write_text(" orange book ");
				break;
			}
			"\U0001f4da" {
				self.scratch_pad.write_text(" stack of books ");
				break;
			}
			"\U0001f4db" {
				self.scratch_pad.write_text(" name badge ");
				break;
			}
			"\U0001f4dc" {
				self.scratch_pad.write_text(" paper scroll ");
				break;
			}
			"\U0001f4de" {
				self.scratch_pad.write_text(" telephone receiver ");
				break;
			}
			"\U0001f4df" {
				self.scratch_pad.write_text(" pager ");
				break;
			}
			"\U0001f4e0" {
				self.scratch_pad.write_text(" fax ");
				break;
			}
			"\U0001f4e1" {
				self.scratch_pad.write_text(" satellite ");
				break;
			}
			"\U0001f4e2" {
				self.scratch_pad.write_text(" loud speaker ");
				break;
			}
			"\U0001f4e3" {
				self.scratch_pad.write_text(" mega phone ");
				break;
			}
			"\U0001f4e4" {
				self.scratch_pad.write_text(" out box tray ");
				break;
			}
			"\U0001f4e5" {
				self.scratch_pad.write_text(" in box tray ");
				break;
			}
			"\U0001f4e7" {
				self.scratch_pad.write_text(" e mail ");
				break;
			}
			"\U0001f4e8" {
				self.scratch_pad.write_text(" incoming envelope ");
				break;
			}
			"\U0001f4e9" {
				self.scratch_pad.write_text(" envelope with arrow ");
				break;
			}
			"\U0001f4ea" {
				self.scratch_pad.write_text(" mail box closed ");
				break;
			}
			"\U0001f4eb" {
				self.scratch_pad.write_text(" mail box ");
				break;
			}
			"\U0001f4ec" {
				self.scratch_pad.write_text(" mail box with mail ");
				break;
			}
			"\U0001f4ed" {
				self.scratch_pad.write_text(" mail box with no mail ");
				break;
			}
			"\U0001f4ee" {
				self.scratch_pad.write_text(" postal box ");
				break;
			}
			"\U0001f4ef" {
				self.scratch_pad.write_text(" postal horn ");
				break;
			}
			"\U0001f4f0" {
				self.scratch_pad.write_text(" news paper ");
				break;
			}
			"\U0001f4f1" {
				self.scratch_pad.write_text(" mobile phone ");
				break;
			}
			"\U0001f4f2" {
				self.scratch_pad.write_text(" calling ");
				break;
			}
			"\U0001f4f3" {
				self.scratch_pad.write_text(" vibration mode ");
				break;
			}
			"\U0001f4f4" {
				self.scratch_pad.write_text(" mobile phone off ");
				break;
			}
			"\U0001f4f5" {
				self.scratch_pad.write_text(" no mobile phones ");
				break;
			}
			"\U0001f4f6" {
				self.scratch_pad.write_text(" signal strength ");
				break;
			}
			"\U0001f4f7" {
				self.scratch_pad.write_text(" camera ");
				break;
			}
			"\U0001f4f9" {
				self.scratch_pad.write_text(" video camera ");
				break;
			}
			"\U0001f4fa" {
				self.scratch_pad.write_text(" t v ");
				break;
			}
			"\U0001f4fb" {
				self.scratch_pad.write_text(" radio ");
				break;
			}
			"\U0001f4fc" {
				self.scratch_pad.write_text(" v h s ");
				break;
			}
			"\U0001f4fd" {
				self.scratch_pad.write_text(" projector ");
				break;
			}
			"\U0001f500" {
				self.scratch_pad.write_text(" twisted rightwards arrows ");
				break;
			}
			"\U0001f501" {
				self.scratch_pad.write_text(" repeat ");
				break;
			}
			"\U0001f502" {
				self.scratch_pad.write_text(" repeat one ");
				break;
			}
			"\U0001f503" {
				self.scratch_pad.write_text(" clockwise arrows ");
				break;
			}
			"\U0001f504" {
				self.scratch_pad.write_text(" counter clockwise arrows ");
				break;
			}
			"\U0001f505" {
				self.scratch_pad.write_text(" low brightness ");
				break;
			}
			"\U0001f506" {
				self.scratch_pad.write_text(" high brightness ");
				break;
			}
			"\U0001f507" {
				self.scratch_pad.write_text(" mute ");
				break;
			}
			"\U0001f508" {
				self.scratch_pad.write_text(" speaker ");
				break;
			}
			"\U0001f509" {
				self.scratch_pad.write_text(" sound ");
				break;
			}
			"\U0001f50a" {
				self.scratch_pad.write_text(" loud sound ");
				break;
			}
			"\U0001f50b" {
				self.scratch_pad.write_text(" battery ");
				break;
			}
			"\U0001f50c" {
				self.scratch_pad.write_text(" electric plug ");
				break;
			}
			"\U0001f50e" | "\U0001f50d" {
				self.scratch_pad.write_text(" magnifying glass ");
				break;
			}
			"\U0001f50f" {
				self.scratch_pad.write_text(" lock with ink pen ");
				break;
			}
			"\U0001f510" {
				self.scratch_pad.write_text(" closed lock with key ");
				break;
			}
			"\U0001f511" {
				self.scratch_pad.write_text(" key ");
				break;
			}
			"\U0001f512" {
				self.scratch_pad.write_text(" lock ");
				break;
			}
			"\U0001f513" {
				self.scratch_pad.write_text(" unlock ");
				break;
			}
			"\U0001f514" {
				self.scratch_pad.write_text(" bell ");
				break;
			}
			"\U0001f515" {
				self.scratch_pad.write_text(" no bell ");
				break;
			}
			"\U0001f516" {
				self.scratch_pad.write_text(" book mark ");
				break;
			}
			"\U0001f517" {
				self.scratch_pad.write_text(" link ");
				break;
			}
			"\U0001f518" {
				self.scratch_pad.write_text(" radio button ");
				break;
			}
			"\U0001f519" {
				self.scratch_pad.write_text(" back ");
				break;
			}
			"\U0001f51a" {
				self.scratch_pad.write_text(" end ");
				break;
			}
			"\U0001f51b" {
				self.scratch_pad.write_text(" on ");
				break;
			}
			"\U0001f51c" {
				self.scratch_pad.write_text(" soon ");
				break;
			}
			"\U0001f51d" {
				self.scratch_pad.write_text(" top ");
				break;
			}
			"\U0001f51e" {
				self.scratch_pad.write_text(" no one under eighteen ");
				break;
			}
			"\U0001f51f" {
				self.scratch_pad.write_text(" key cap ten ");
				break;
			}
			"\U0001f525" {
				self.scratch_pad.write_text(" fire ");
				break;
			}
			"\U0001f526" {
				self.scratch_pad.write_text(" flash light ");
				break;
			}
			"\U0001f527" {
				self.scratch_pad.write_text(" wrench ");
				break;
			}
			"\U0001f528" {
				self.scratch_pad.write_text(" hammer ");
				break;
			}
			"\U0001f529" {
				self.scratch_pad.write_text(" nut and bolt ");
				break;
			}
			"\U0001f52a" {
				self.scratch_pad.write_text(" knife ");
				break;
			}
			"\U0001f52b" {
				self.scratch_pad.write_text(" gun ");
				break;
			}
			"\U0001f52c" {
				self.scratch_pad.write_text(" microscope ");
				break;
			}
			"\U0001f52d" {
				self.scratch_pad.write_text(" telescope ");
				break;
			}
			"\U0001f52e" {
				self.scratch_pad.write_text(" crystal ball ");
				break;
			}
			"\U0001f52f" {
				self.scratch_pad.write_text(" six pointed star ");
				break;
			}
			"\U0001f530" {
				self.scratch_pad.write_text(" beginner symbol ");
				break;
			}
			"\U0001f531" {
				self.scratch_pad.write_text(" trident ");
				break;
			}
			"\U0001f533" | "\U0001f532" {
				self.scratch_pad.write_text(" square button ");
				break;
			}
			"\U0001f534" {
				self.scratch_pad.write_text(" red circle ");
				break;
			}
			"\U0001f535" {
				self.scratch_pad.write_text(" blue circle ");
				break;
			}
			"\U0001f536" {
				self.scratch_pad.write_text(" large orange diamond ");
				break;
			}
			"\U0001f537" {
				self.scratch_pad.write_text(" large blue diamond ");
				break;
			}
			"\U0001f538" {
				self.scratch_pad.write_text(" small orange diamond ");
				break;
			}
			"\U0001f539" {
				self.scratch_pad.write_text(" small blue diamond ");
				break;
			}
			"\U0001f53a" {
				self.scratch_pad.write_text(" small red triangle ");
				break;
			}
			"\U0001f53b" {
				self.scratch_pad.write_text(" small red triangle down ");
				break;
			}
			"\U0001f53c" {
				self.scratch_pad.write_text(" arrow up small ");
				break;
			}
			"\U0001f53d" {
				self.scratch_pad.write_text(" arrow down small ");
				break;
			}
			"\U0001f549" {
				self.scratch_pad.write_text(" om symbol ");
				break;
			}
			"\U0001f54a" {
				self.scratch_pad.write_text(" dove ");
				break;
			}
			"\U0001f54b" {
				self.scratch_pad.write_text(" kaaba ");
				break;
			}
			"\U0001f54c" {
				self.scratch_pad.write_text(" mosque ");
				break;
			}
			"\U0001f550" {
				self.scratch_pad.write_text(" one oh clock ");
				break;
			}
			"\U0001f551" {
				self.scratch_pad.write_text(" two oh clock ");
				break;
			}
			"\U0001f552" {
				self.scratch_pad.write_text(" three oh clock ");
				break;
			}
			"\U0001f553" {
				self.scratch_pad.write_text(" four oh clock ");
				break;
			}
			"\U0001f554" {
				self.scratch_pad.write_text(" five oh clock ");
				break;
			}
			"\U0001f555" {
				self.scratch_pad.write_text(" six oh clock ");
				break;
			}
			"\U0001f556" {
				self.scratch_pad.write_text(" seven oh clock ");
				break;
			}
			"\U0001f557" {
				self.scratch_pad.write_text(" eight oh clock ");
				break;
			}
			"\U0001f558" {
				self.scratch_pad.write_text(" nine oh clock ");
				break;
			}
			"\U0001f559" {
				self.scratch_pad.write_text(" ten oh clock ");
				break;
			}
			"\U0001f55a" {
				self.scratch_pad.write_text(" eleven oh clock ");
				break;
			}
			"\U0001f55b" {
				self.scratch_pad.write_text(" twelve oh clock ");
				break;
			}
			"\U0001f55c" {
				self.scratch_pad.write_text(" one thirty ");
				break;
			}
			"\U0001f55d" {
				self.scratch_pad.write_text(" two thirty ");
				break;
			}
			"\U0001f55e" {
				self.scratch_pad.write_text(" three thirty ");
				break;
			}
			"\U0001f55f" {
				self.scratch_pad.write_text(" four thirty ");
				break;
			}
			"\U0001f560" {
				self.scratch_pad.write_text(" five thirty ");
				break;
			}
			"\U0001f561" {
				self.scratch_pad.write_text(" six thirty ");
				break;
			}
			"\U0001f562" {
				self.scratch_pad.write_text(" seven thirty ");
				break;
			}
			"\U0001f563" {
				self.scratch_pad.write_text(" eight thirty ");
				break;
			}
			"\U0001f564" {
				self.scratch_pad.write_text(" nine thirty ");
				break;
			}
			"\U0001f565" {
				self.scratch_pad.write_text(" ten thirty ");
				break;
			}
			"\U0001f566" {
				self.scratch_pad.write_text(" eleven thirty ");
				break;
			}
			"\U0001f567" {
				self.scratch_pad.write_text(" twelve thirty ");
				break;
			}
			"\U0001f573" {
				self.scratch_pad.write_text(" hole ");
				break;
			}
			"\U0001f574" {
				self.scratch_pad.write_text(" levitate ");
				break;
			}
			"\U0001f575" {
				self.scratch_pad.write_text(" detective ");
				break;
			}
			"\U0001f577" {
				self.scratch_pad.write_text(" spider ");
				break;
			}
			"\U0001f578" {
				self.scratch_pad.write_text(" spider web ");
				break;
			}
			"\U0001f579" {
				self.scratch_pad.write_text(" joystick ");
				break;
			}
			"\U0001f58a" {
				self.scratch_pad.write_text(" pen ballpoint ");
				break;
			}
			"\U0001f58b" {
				self.scratch_pad.write_text(" pen fountain ");
				break;
			}
			"\U0001f58c" {
				self.scratch_pad.write_text(" paint brush ");
				break;
			}
			"\U0001f590" {
				self.scratch_pad.write_text(" hand splayed ");
				break;
			}
			"\U0001f595" {
				self.scratch_pad.write_text(" middle finger ");
				break;
			}
			"\U0001f596" {
				self.scratch_pad.write_text(" vulcan gesture ");
				break;
			}
			"\U0001f5a4" {
				self.scratch_pad.write_text(" black heart ");
				break;
			}
			"\U0001f5b1" {
				self.scratch_pad.write_text(" mouse three button ");
				break;
			}
			"\U0001f5d1" {
				self.scratch_pad.write_text(" waste basket ");
				break;
			}
			"\U0001f5e1" {
				self.scratch_pad.write_text(" dagger ");
				break;
			}
			"\U0001f5e3" {
				self.scratch_pad.write_text(" speaking head ");
				break;
			}
			"\U0001f5fb" {
				self.scratch_pad.write_text(" mount fuji ");
				break;
			}
			"\U0001f5fc" {
				self.scratch_pad.write_text(" tokyo tower ");
				break;
			}
			"\U0001f5fd" {
				self.scratch_pad.write_text(" statue of liberty ");
				break;
			}
			"\U0001f5fe" {
				self.scratch_pad.write_text(" japan ");
				break;
			}
			"\U0001f5ff" {
				self.scratch_pad.write_text(" moyai ");
				break;
			}
			"\U0001f600" {
				self.scratch_pad.write_text(" grinning ");
				break;
			}
			"\U0001f601" {
				self.scratch_pad.write_text(" grin ");
				break;
			}
			"\U0001f602" {
				self.scratch_pad.write_text(" joy ");
				break;
			}
			"\U0001f603" {
				self.scratch_pad.write_text(" smiling with open mouth ");
				break;
			}
			"\U0001f604" {
				self.scratch_pad.write_text(" smiling with open mouth , and smiling eyes ");
				break;
			}
			"\U0001f605" {
				self.scratch_pad.write_text(" sweat smile ");
				break;
			}
			"\U0001f607" {
				self.scratch_pad.write_text(" innocent ");
				break;
			}
			"\U0001f60a" {
				self.scratch_pad.write_text(" blush ");
				break;
			}
			"\U0001f60b" {
				self.scratch_pad.write_text(" yum ");
				break;
			}
			"\U0001f60c" {
				self.scratch_pad.write_text(" relieved ");
				break;
			}
			"\U0001f60d" {
				self.scratch_pad.write_text(" heart eyes ");
				break;
			}
			"\U0001f60e" {
				self.scratch_pad.write_text(" sunglasses ");
				break;
			}
			"\U0001f60f" {
				self.scratch_pad.write_text(" smirk ");
				break;
			}
			"\U0001f610" {
				self.scratch_pad.write_text(" neutral expression ");
				break;
			}
			"\U0001f611" {
				self.scratch_pad.write_text(" expressionless ");
				break;
			}
			"\U0001f612" {
				self.scratch_pad.write_text(" unamused ");
				break;
			}
			"\U0001f613" {
				self.scratch_pad.write_text(" sweat ");
				break;
			}
			"\U0001f614" {
				self.scratch_pad.write_text(" pensive ");
				break;
			}
			"\U0001f615" {
				self.scratch_pad.write_text(" confused ");
				break;
			}
			"\U0001f616" {
				self.scratch_pad.write_text(" confounded ");
				break;
			}
			"\U0001f617" {
				self.scratch_pad.write_text(" kissing ");
				break;
			}
			"\U0001f618" {
				self.scratch_pad.write_text(" kissing heart ");
				break;
			}
			"\U0001f619" {
				self.scratch_pad.write_text(" kissing smiling eyes ");
				break;
			}
			"\U0001f61a" {
				self.scratch_pad.write_text(" kissing closed eyes ");
				break;
			}
			"\U0001f61b" {
				self.scratch_pad.write_text(" stuck out tongue ");
				break;
			}
			"\U0001f61c" {
				self.scratch_pad.write_text(" stuck out tongue winking eye ");
				break;
			}
			"\U0001f61d" {
				self.scratch_pad.write_text(" stuck out tongue closed eyes ");
				break;
			}
			"\U0001f61e" {
				self.scratch_pad.write_text(" disappointed ");
				break;
			}
			"\U0001f61f" {
				self.scratch_pad.write_text(" worried ");
				break;
			}
			"\U0001f621" {
				self.scratch_pad.write_text(" rage ");
				break;
			}
			"\U0001f623" {
				self.scratch_pad.write_text(" persevere ");
				break;
			}
			"\U0001f624" {
				self.scratch_pad.write_text(" triumph ");
				break;
			}
			"\U0001f627" {
				self.scratch_pad.write_text(" anguished ");
				break;
			}
			"\U0001f628" {
				self.scratch_pad.write_text(" fearful ");
				break;
			}
			"\U0001f629" {
				self.scratch_pad.write_text(" weary ");
				break;
			}
			"\U0001f62a" {
				self.scratch_pad.write_text(" sleepy ");
				break;
			}
			"\U0001f62b" {
				self.scratch_pad.write_text(" tired ");
				break;
			}
			"\U0001f62c" {
				self.scratch_pad.write_text(" grimacing ");
				break;
			}
			"\U0001f62d" {
				self.scratch_pad.write_text(" crying loudly ");
				break;
			}
			"\U0001f62e" {
				self.scratch_pad.write_text(" open mouth ");
				break;
			}
			"\U0001f62f" {
				self.scratch_pad.write_text(" hushed ");
				break;
			}
			"\U0001f630" {
				self.scratch_pad.write_text(" cold sweat ");
				break;
			}
			"\U0001f631" {
				self.scratch_pad.write_text(" scream ");
				break;
			}
			"\U0001f632" {
				self.scratch_pad.write_text(" astonished ");
				break;
			}
			"\U0001f633" {
				self.scratch_pad.write_text(" flushed ");
				break;
			}
			"\U0001f634" {
				self.scratch_pad.write_text(" sleeping ");
				break;
			}
			"\U0001f636" {
				self.scratch_pad.write_text(" no mouth ");
				break;
			}
			"\U0001f637" {
				self.scratch_pad.write_text(" mask ");
				break;
			}
			"\U0001f638" {
				self.scratch_pad.write_text(" grinning cat ");
				break;
			}
			"\U0001f639" {
				self.scratch_pad.write_text(" joyful cat ");
				break;
			}
			"\U0001f63a" {
				self.scratch_pad.write_text(" smiling cat ");
				break;
			}
			"\U0001f63b" {
				self.scratch_pad.write_text(" cat with heart eyes ");
				break;
			}
			"\U0001f63c" {
				self.scratch_pad.write_text(" smirking cat ");
				break;
			}
			"\U0001f63d" {
				self.scratch_pad.write_text(" kissing cat ");
				break;
			}
			"\U0001f63e" {
				self.scratch_pad.write_text(" pouting cat ");
				break;
			}
			"\U0001f63f" {
				self.scratch_pad.write_text(" crying cat ");
				break;
			}
			"\U0001f640" {
				self.scratch_pad.write_text(" screaming cat ");
				break;
			}
			"\U0001f641" {
				self.scratch_pad.write_text(" slight frown ");
				break;
			}
			"\U0001f642" {
				self.scratch_pad.write_text(" slight smile ");
				break;
			}
			"\U0001f643" {
				self.scratch_pad.write_text(" upside down ");
				break;
			}
			"\U0001f644" {
				self.scratch_pad.write_text(" rolling eyes ");
				break;
			}
			"\U0001f645" {
				self.scratch_pad.write_text(" person gesturing no ");
				break;
			}
			"\U0001f646" {
				self.scratch_pad.write_text(" person gesturing okay ");
				break;
			}
			"\U0001f647" {
				self.scratch_pad.write_text(" person bowing ");
				break;
			}
			"\U0001f648" {
				self.scratch_pad.write_text(" see no evil ");
				break;
			}
			"\U0001f649" {
				self.scratch_pad.write_text(" hear no evil ");
				break;
			}
			"\U0001f64a" {
				self.scratch_pad.write_text(" speak no evil ");
				break;
			}
			"\U0001f64b" {
				self.scratch_pad.write_text(" person raising hand ");
				break;
			}
			"\U0001f64c" {
				self.scratch_pad.write_text(" raised hands ");
				break;
			}
			"\U0001f64d" {
				self.scratch_pad.write_text(" person frowning ");
				break;
			}
			"\U0001f64e" {
				self.scratch_pad.write_text(" person pouting ");
				break;
			}
			"\U0001f64f" {
				self.scratch_pad.write_text(" pray ");
				break;
			}
			"\U0001f680" {
				self.scratch_pad.write_text(" rocket ");
				break;
			}
			"\U0001f681" {
				self.scratch_pad.write_text(" helicopter ");
				break;
			}
			"\U0001f682" {
				self.scratch_pad.write_text(" steam locomotive ");
				break;
			}
			"\U0001f683" {
				self.scratch_pad.write_text(" railway car ");
				break;
			}
			"\U0001f684" | "\U0001f685" {
				self.scratch_pad.write_text(" bullet train ");
				break;
			}
			"\U0001f686" {
				self.scratch_pad.write_text(" train ");
				break;
			}
			"\U0001f687" {
				self.scratch_pad.write_text(" metro ");
				break;
			}
			"\U0001f688" {
				self.scratch_pad.write_text(" light rail ");
				break;
			}
			"\U0001f689" {
				self.scratch_pad.write_text(" station ");
				break;
			}
			"\U0001f68a" {
				self.scratch_pad.write_text(" tram ");
				break;
			}
			"\U0001f68b" {
				self.scratch_pad.write_text(" tram car ");
				break;
			}
			"\U0001f68c" {
				self.scratch_pad.write_text(" bus ");
				break;
			}
			"\U0001f68d" {
				self.scratch_pad.write_text(" oncoming bus ");
				break;
			}
			"\U0001f68e" {
				self.scratch_pad.write_text(" trolley bus ");
				break;
			}
			"\U0001f68f" {
				self.scratch_pad.write_text(" bus stop ");
				break;
			}
			"\U0001f690" {
				self.scratch_pad.write_text(" mini bus ");
				break;
			}
			"\U0001f691" {
				self.scratch_pad.write_text(" ambulance ");
				break;
			}
			"\U0001f692" {
				self.scratch_pad.write_text(" fire engine ");
				break;
			}
			"\U0001f694" | "\U0001f693" {
				self.scratch_pad.write_text(" police car ");
				break;
			}
			"\U0001f695" | "\U0001f696" {
				self.scratch_pad.write_text(" taxi ");
				break;
			}
			"\U0001f697" {
				self.scratch_pad.write_text(" red car ");
				break;
			}
			"\U0001f698" {
				self.scratch_pad.write_text(" oncoming automobile ");
				break;
			}
			"\U0001f699" {
				self.scratch_pad.write_text(" blue car ");
				break;
			}
			"\U0001f69a" | "\U0001f69b" {
				self.scratch_pad.write_text(" truck ");
				break;
			}
			"\U0001f69c" {
				self.scratch_pad.write_text(" tractor ");
				break;
			}
			"\U0001f69d" {
				self.scratch_pad.write_text(" monorail ");
				break;
			}
			"\U0001f69e" {
				self.scratch_pad.write_text(" mountain railway ");
				break;
			}
			"\U0001f69f" {
				self.scratch_pad.write_text(" suspension railway ");
				break;
			}
			"\U0001f6a0" {
				self.scratch_pad.write_text(" mountain cableway ");
				break;
			}
			"\U0001f6a1" {
				self.scratch_pad.write_text(" aerial tramway ");
				break;
			}
			"\U0001f6a2" {
				self.scratch_pad.write_text(" ship ");
				break;
			}
			"\U0001f6a3" {
				self.scratch_pad.write_text(" person rowing boat ");
				break;
			}
			"\U0001f6a4" {
				self.scratch_pad.write_text(" speed boat ");
				break;
			}
			"\U0001f6a5" {
				self.scratch_pad.write_text(" traffic light ");
				break;
			}
			"\U0001f6a6" {
				self.scratch_pad.write_text(" vertical traffic light ");
				break;
			}
			"\U0001f6a7" {
				self.scratch_pad.write_text(" construction ");
				break;
			}
			"\U0001f6a8" {
				self.scratch_pad.write_text(" rotating light ");
				break;
			}
			"\U0001f6a9" {
				self.scratch_pad.write_text(" triangular flag on post ");
				break;
			}
			"\U0001f6aa" {
				self.scratch_pad.write_text(" door ");
				break;
			}
			"\U0001f6ab" {
				self.scratch_pad.write_text(" no entry sign ");
				break;
			}
			"\U0001f6ac" {
				self.scratch_pad.write_text(" smoking ");
				break;
			}
			"\U0001f6ad" {
				self.scratch_pad.write_text(" no smoking ");
				break;
			}
			"\U0001f6ae" {
				self.scratch_pad.write_text(" put litter in its place ");
				break;
			}
			"\U0001f6af" {
				self.scratch_pad.write_text(" do not litter ");
				break;
			}
			"\U0001f6b0" {
				self.scratch_pad.write_text(" potable water ");
				break;
			}
			"\U0001f6b1" {
				self.scratch_pad.write_text(" non potable water ");
				break;
			}
			"\U0001f6b2" {
				self.scratch_pad.write_text(" bike ");
				break;
			}
			"\U0001f6b3" {
				self.scratch_pad.write_text(" no bicycles ");
				break;
			}
			"\U0001f6b4" {
				self.scratch_pad.write_text(" person biking ");
				break;
			}
			"\U0001f6b5" {
				self.scratch_pad.write_text(" person mountain biking ");
				break;
			}
			"\U0001f6b6" {
				self.scratch_pad.write_text(" person walking ");
				break;
			}
			"\U0001f6b7" {
				self.scratch_pad.write_text(" no pedestrians ");
				break;
			}
			"\U0001f6b8" {
				self.scratch_pad.write_text(" children crossing ");
				break;
			}
			"\U0001f6b9" {
				self.scratch_pad.write_text(" mens ");
				break;
			}
			"\U0001f6ba" {
				self.scratch_pad.write_text(" womens ");
				break;
			}
			"\U0001f6bb" {
				self.scratch_pad.write_text(" restroom ");
				break;
			}
			"\U0001f6bc" {
				self.scratch_pad.write_text(" baby symbol ");
				break;
			}
			"\U0001f6bd" {
				self.scratch_pad.write_text(" toilet ");
				break;
			}
			"\U0001f6be" {
				self.scratch_pad.write_text(" w c ");
				break;
			}
			"\U0001f6bf" {
				self.scratch_pad.write_text(" shower ");
				break;
			}
			"\U0001f6c0" {
				self.scratch_pad.write_text(" bath ");
				break;
			}
			"\U0001f6c1" {
				self.scratch_pad.write_text(" bath tub ");
				break;
			}
			"\U0001f6c2" {
				self.scratch_pad.write_text(" passport control ");
				break;
			}
			"\U0001f6c3" {
				self.scratch_pad.write_text(" customs ");
				break;
			}
			"\U0001f6c4" {
				self.scratch_pad.write_text(" baggage claim ");
				break;
			}
			"\U0001f6c5" {
				self.scratch_pad.write_text(" left luggage ");
				break;
			}
			"\U0001f6e0" {
				self.scratch_pad.write_text(" tools ");
				break;
			}
			"\U0001f6e1" {
				self.scratch_pad.write_text(" shield ");
				break;
			}
			"\U0001f6e4" {
				self.scratch_pad.write_text(" railway track ");
				break;
			}
			"\U0001f6f7" {
				self.scratch_pad.write_text(" sled ");
				break;
			}
			"\U0001f7e1" {
				self.scratch_pad.write_text(" yellow circle ");
				break;
			}
			"\U0001f7e2" {
				self.scratch_pad.write_text(" green circle ");
				break;
			}
			"\U0001f7e3" {
				self.scratch_pad.write_text(" purple circle ");
				break;
			}
			"\U0001f7e5" {
				self.scratch_pad.write_text(" red square ");
				break;
			}
			"\U0001f7e6" {
				self.scratch_pad.write_text(" blue square ");
				break;
			}
			"\U0001f7e7" {
				self.scratch_pad.write_text(" orange square ");
				break;
			}
			"\U0001f7e8" {
				self.scratch_pad.write_text(" yellow square ");
				break;
			}
			"\U0001f7e9" {
				self.scratch_pad.write_text(" green square ");
				break;
			}
			"\U0001f7ea" {
				self.scratch_pad.write_text(" purple square ");
				break;
			}
			"\U0001f7eb" {
				self.scratch_pad.write_text(" brown square ");
				break;
			}
			"\U0001f90c" {
				self.scratch_pad.write_text(" pinched fingers ");
				break;
			}
			"\U0001f90d" {
				self.scratch_pad.write_text(" white heart ");
				break;
			}
			"\U0001f910" {
				self.scratch_pad.write_text(" zipper mouth ");
				break;
			}
			"\U0001f911" {
				self.scratch_pad.write_text(" money mouth ");
				break;
			}
			"\U0001f912" {
				self.scratch_pad.write_text(" fever face ");
				break;
			}
			"\U0001f913" {
				self.scratch_pad.write_text(" nerd ");
				break;
			}
			"\U0001f914" {
				self.scratch_pad.write_text(" thinking ");
				break;
			}
			"\U0001f915" {
				self.scratch_pad.write_text(" head bandage ");
				break;
			}
			"\U0001f916" {
				self.scratch_pad.write_text(" robot ");
				break;
			}
			"\U0001f917" {
				self.scratch_pad.write_text(" hugging ");
				break;
			}
			"\U0001f918" {
				self.scratch_pad.write_text(" metal ");
				break;
			}
			"\U0001f919" {
				self.scratch_pad.write_text(" call me ");
				break;
			}
			"\U0001f91b" {
				self.scratch_pad.write_text(" left facing fist ");
				break;
			}
			"\U0001f91c" {
				self.scratch_pad.write_text(" right facing fist ");
				break;
			}
			"\U0001f91d" {
				self.scratch_pad.write_text(" hand shake ");
				break;
			}
			"\U0001f91f" {
				self.scratch_pad.write_text(" love you gesture ");
				break;
			}
			"\U0001f920" {
				self.scratch_pad.write_text(" cowboy ");
				break;
			}
			"\U0001f921" {
				self.scratch_pad.write_text(" clown ");
				break;
			}
			"\U0001f922" {
				self.scratch_pad.write_text(" nauseated ");
				break;
			}
			"\U0001f924" {
				self.scratch_pad.write_text(" drooling ");
				break;
			}
			"\U0001f925" {
				self.scratch_pad.write_text(" lying face ");
				break;
			}
			"\U0001f926" {
				self.scratch_pad.write_text(" person facepalming ");
				break;
			}
			"\U0001f927" {
				self.scratch_pad.write_text(" sneezing ");
				break;
			}
			"\U0001f928" {
				self.scratch_pad.write_text(" raised eyebrow ");
				break;
			}
			"\U0001f929" {
				self.scratch_pad.write_text(" star struck ");
				break;
			}
			"\U0001f92a" {
				self.scratch_pad.write_text(" zany face ");
				break;
			}
			"\U0001f92b" {
				self.scratch_pad.write_text(" shushing ");
				break;
			}
			"\U0001f92c" {
				// grawlix to grawlicks to pronounce more easily
				self.scratch_pad.write_text(" face with grawlicks symbols over mouth ");
				break;
			}
			"\U0001f92d" {
				self.scratch_pad.write_text(" face with hand over mouth ");
				break;
			}
			"\U0001f92e" {
				self.scratch_pad.write_text(" vomiting ");
				break;
			}
			"\U0001f92f" {
				self.scratch_pad.write_text(" exploding head ");
				break;
			}
			"\U0001f932" {
				self.scratch_pad.write_text(" palms up together ");
				break;
			}
			"\U0001f934" {
				self.scratch_pad.write_text(" prince ");
				break;
			}
			"\U0001f935" {
				self.scratch_pad.write_text(" person in tuxedo ");
				break;
			}
			"\U0001f937" {
				self.scratch_pad.write_text(" person shrugging ");
				break;
			}
			"\U0001f93a" {
				self.scratch_pad.write_text(" person fencing ");
				break;
			}
			"\U0001f93c" {
				self.scratch_pad.write_text(" people wrestling ");
				break;
			}
			"\U0001f93d" {
				self.scratch_pad.write_text(" person playing water polo ");
				break;
			}
			"\U0001f940" {
				self.scratch_pad.write_text(" wilted rose ");
				break;
			}
			"\U0001f942" {
				self.scratch_pad.write_text(" champagne glass ");
				break;
			}
			"\U0001f943" {
				self.scratch_pad.write_text(" tumbler glass ");
				break;
			}
			"\U0001f947" {
				self.scratch_pad.write_text(" first place ");
				break;
			}
			"\U0001f948" {
				self.scratch_pad.write_text(" second place ");
				break;
			}
			"\U0001f949" {
				self.scratch_pad.write_text(" third place ");
				break;
			}
			"\U0001f94b" {
				self.scratch_pad.write_text(" martial arts uniform ");
				break;
			}
			"\U0001f951" {
				self.scratch_pad.write_text(" avocado ");
				break;
			}
			"\U0001f952" {
				self.scratch_pad.write_text(" cucumber ");
				break;
			}
			"\U0001f954" {
				self.scratch_pad.write_text(" potato ");
				break;
			}
			"\U0001f955" {
				self.scratch_pad.write_text(" carrot ");
				break;
			}
			"\U0001f956" {
				self.scratch_pad.write_text(" french bread ");
				break;
			}
			"\U0001f95a" {
				self.scratch_pad.write_text(" egg ");
				break;
			}
			"\U0001f95b" {
				self.scratch_pad.write_text(" milk ");
				break;
			}
			"\U0001f95c" {
				self.scratch_pad.write_text(" peanuts ");
				break;
			}
			"\U0001f95d" {
				self.scratch_pad.write_text(" kiwi ");
				break;
			}
			"\U0001f966" {
				self.scratch_pad.write_text(" broccoli ");
				break;
			}
			"\U0001f970" {
				self.scratch_pad.write_text(" smiling face with three hearts ");
				break;
			}
			"\U0001f971" {
				self.scratch_pad.write_text(" yawning ");
				break;
			}
			"\U0001f972" {
				self.scratch_pad.write_text(" smiling with tears ");
				break;
			}
			"\U0001f973" {
				self.scratch_pad.write_text(" partying face ");
				break;
			}
			"\U0001f974" {
				self.scratch_pad.write_text(" woozy face ");
				break;
			}
			"\U0001f975" {
				self.scratch_pad.write_text(" hot face ");
				break;
			}
			"\U0001f976" {
				self.scratch_pad.write_text(" cold face ");
				break;
			}
			"\U0001f978" {
				self.scratch_pad.write_text(" disguised face ");
				break;
			}
			"\U0001f979" {
				self.scratch_pad.write_text(" holding back tears ");
				break;
			}
			"\U0001f97a" {
				self.scratch_pad.write_text(" pleading ");
				break;
			}
			"\U0001f980" {
				self.scratch_pad.write_text(" crab ");
				break;
			}
			"\U0001f981" {
				self.scratch_pad.write_text(" lion ");
				break;
			}
			"\U0001f982" {
				self.scratch_pad.write_text(" scorpion ");
				break;
			}
			"\U0001f984" {
				self.scratch_pad.write_text(" unicorn ");
				break;
			}
			"\U0001f985" {
				self.scratch_pad.write_text(" eagle ");
				break;
			}
			"\U0001f987" {
				self.scratch_pad.write_text(" bat ");
				break;
			}
			"\U0001f988" {
				self.scratch_pad.write_text(" shark ");
				break;
			}
			"\U0001f98a" {
				self.scratch_pad.write_text(" fox ");
				break;
			}
			"\U0001f98b" {
				self.scratch_pad.write_text(" butterfly ");
				break;
			}
			"\U0001f98c" {
				self.scratch_pad.write_text(" deer ");
				break;
			}
			"\U0001f98f" {
				self.scratch_pad.write_text(" rhino ");
				break;
			}
			"\U0001f992" {
				self.scratch_pad.write_text(" giraffe ");
				break;
			}
			"\U0001f993" {
				self.scratch_pad.write_text(" zebra ");
				break;
			}
			"\U0001f994" {
				self.scratch_pad.write_text(" hedgehog ");
				break;
			}
			"\U0001f99a" {
				self.scratch_pad.write_text(" peacock ");
				break;
			}
			"\U0001f99c" {
				self.scratch_pad.write_text(" parrot ");
				break;
			}
			"\U0001f99d" {
				self.scratch_pad.write_text(" raccoon ");
				break;
			}
			"\U0001f99f" {
				self.scratch_pad.write_text(" mosquito ");
				break;
			}
			"\U0001f9a0" {
				self.scratch_pad.write_text(" microbe ");
				break;
			}
			"\U0001f9ad" {
				self.scratch_pad.write_text(" seal ");
				break;
			}
			"\U0001f9c0" {
				self.scratch_pad.write_text(" cheese ");
				break;
			}
			"\U0001f9cc" {
				self.scratch_pad.write_text(" troll ");
				break;
			}
			"\U0001f9d0" {
				self.scratch_pad.write_text(" face with monocle ");
				break;
			}
			"\U0001f9d1" {
				self.scratch_pad.write_text(" adult ");
				break;
			}
			"\U0001f9d8" {
				self.scratch_pad.write_text(" sitting in lotus position ");
				break;
			}
			"\U0001f9db" {
				self.scratch_pad.write_text(" vampire ");
				break;
			}
			"\U0001f9df" {
				self.scratch_pad.write_text(" zombie ");
				break;
			}
			"\U0001f9e1" {
				self.scratch_pad.write_text(" orange heart ");
				break;
			}
			"\U0001f9ea" {
				self.scratch_pad.write_text(" test tube ");
				break;
			}
			"\U0001f9eb" {
				self.scratch_pad.write_text(" petri dish ");
				break;
			}
			"\U0001f9ec" {
				self.scratch_pad.write_text(" d n A ");
				break;
			}
			"\U0001f9ed" {
				self.scratch_pad.write_text(" compass ");
				break;
			}
			"\U0001f9f2" {
				self.scratch_pad.write_text(" magnet ");
				break;
			}
			"\U0001f9f8" {
				self.scratch_pad.write_text(" teddy bear ");
				break;
			}
			"\U0001f9f9" {
				self.scratch_pad.write_text(" broom ");
				break;
			}
			"\U0001f9ff" {
				self.scratch_pad.write_text(" nazar amulet ");
				break;
			}
			"\U0001fa75" {
				self.scratch_pad.write_text(" light blue heart ");
				break;
			}
			"\U0001fa78" {
				self.scratch_pad.write_text(" drop of blood ");
				break;
			}
			"\U0001fa79" {
				self.scratch_pad.write_text(" adhesive bandage ");
				break;
			}
			"\U0001fa82" {
				self.scratch_pad.write_text(" parachute ");
				break;
			}
			"\U0001fa84" {
				self.scratch_pad.write_text(" magic wand ");
				break;
			}
			"\U0001fa86" {
				self.scratch_pad.write_text(" nesting dolls ");
				break;
			}
			"\U0001fa90" {
				self.scratch_pad.write_text(" ringed planet ");
				break;
			}
			"\U0001fac2" {
				self.scratch_pad.write_text(" people hugging ");
				break;
			}
			"\U0001fad0" {
				self.scratch_pad.write_text(" blue berries ");
				break;
			}
			"\U0001fad7" {
				self.scratch_pad.write_text(" pouring liquid ");
				break;
			}
			"\U0001fae0" {
				self.scratch_pad.write_text(" melting face ");
				break;
			}
			"\U0001fae1" {
				self.scratch_pad.write_text(" saluting ");
				break;
			}
			"\U0001fae2" {
				self.scratch_pad.write_text(" face with open eyes and hand over mouth ");
				break;
			}
			"\U0001fae3" {
				self.scratch_pad.write_text(" face with peeking eye ");
				break;
			}
			"\U0001fae4" {
				self.scratch_pad.write_text(" face with diagonal mouth ");
				break;
			}
			"\U0001fae5" {
				self.scratch_pad.write_text(" dotted line face ");
				break;
			}
			"\U0001fae7" {
				self.scratch_pad.write_text(" bubbles ");
				break;
			}
			"\U0001faf0" {
				self.scratch_pad.write_text(" hand with index finger and thumb crossed ");
				break;
			}
			"\U0001faf1" {
				self.scratch_pad.write_text(" rightwards hand ");
				break;
			}
			"\U0001faf2" {
				self.scratch_pad.write_text(" leftwards hand ");
				break;
			}
			"\U0001faf6" {
				self.scratch_pad.write_text(" heart hands ");
				break;
			}

			/*-------------------------*/
			/* SOME PUNCTUATIONS RULES */
			/*-------------------------*/

			space exc_point space {
				self.scratch_pad.write_text(" exclaimation point ");
				break;
			}
			space "(" space {
				self.scratch_pad.write_text(" left parenthesis ");
				break;
			}
			space ")" space {
				self.scratch_pad.write_text(" right parenthesis ");
				break;
			}
			"*" {
				self.scratch_pad.write_text(" asterisk ");
				break;
			}
			space comma space {
				self.scratch_pad.write_text(" comma ");
				break;
			}
			space period space {
				self.scratch_pad.write_text(" period ");
				break;
			}
			space ":" space {
				self.scratch_pad.write_text(" colon ");
				break;
			}
			space ";" space {
				self.scratch_pad.write_text(" semicolon ");
				break;
			}
			":" space | ";" space | period space {
				if !self.punct_prev {
					self.scratch_pad.write_text(" . ");
					self.punct_prev = true;
				}
				break;
			}
			space ques_mark space {
				self.scratch_pad.write_text(" question mark ");
				break;
			}
			space "[" space {
				self.scratch_pad.write_text(" left square bracket ");
				break;
			}
			space dquote space {
				self.scratch_pad.write_text(" double quote ");
				break;
			}
			space squote space {
				self.scratch_pad.write_text(" single quote ");
				break;
			}
			squote {
				// write out the apostrophe
				self.scratch_pad.write_text("\x27");
				break;
			}
			space "]" space {
				self.scratch_pad.write_text(" right square bracket ");
				break;
			}
			"^" {
				self.scratch_pad.write_text(" caret ");
				break;
			}
			space "{" space {
				self.scratch_pad.write_text(" left curly bracket ");
				break;
			}
			space "}" space {
				self.scratch_pad.write_text(" right curly bracket ");
				break;
			}
			"|" {
				self.scratch_pad.write_text(" vertical bar ");
				break;
			}
			exc_point {
				if !self.punct_prev {
					self.scratch_pad.write_text(" ! ");
					self.punct_prev = true;
				}
				break;
			}
			ques_mark {
				if !self.punct_prev {
					self.scratch_pad.write_text(" ? ");
					self.punct_prev = true;
				}
				break;
			}
			':00' " "? 'a' period? 'm' period? {
				self.scratch_pad.write_text(" A m ");
				break;
			}
			':00' " "? 'p' period? 'm' period? {
				self.scratch_pad.write_text(" p m ");
				break;
			}
			':00' {
				self.scratch_pad.write_text(" oh clock ");
				break;
			}
			'am' {
				if self.digit_prev {
					self.digit_prev = false;
					self.scratch_pad.write_text(" A m ");
				} else {
					self.scratch_pad.write_text("am");
				}
				break;
			}
			'pm' {
				if self.digit_prev {
					self.digit_prev = false;
					self.scratch_pad.write_text(" p m ");
				} else {
					self.scratch_pad.write_text("pm");
				}
				break;
			}

			space "-" space {
				self.scratch_pad.write_text(" minus ");
				break;
			}

			dquote | comma | space "(" | ")" space {
				if !self.punct_prev {
					self.scratch_pad.write_text(" , ");
					self.punct_prev = true;
				}
				break;
			}

			/*---------------------*/
			/* FUN STUFF GOES HERE */
			/*---------------------*/

			fun_tag "version" fun_tag {
				self.scratch_pad.write_text(" version one ");
				break;
			}
			fun_tag "author" fun_tag {
				self.scratch_pad.write_text(" anthony c bartman ");
				break;
			}
			fun_tag "build_date" fun_tag {
				self.scratch_pad.write_text(" august twenty fifth , twenty twenty six ");
				break;
			}

			/*-----------------*/
			/* LEFT OVER RULES */
			/*-----------------*/

			/* DEAL WITH UPPERCASE LETTERS FIRST */
			"A" period? {
				self.scratch_pad.write_text(" A");
				break;
			}
			"B" period? {
				self.scratch_pad.write_text(" b");
				break;
			}
			"C" period? {
				self.scratch_pad.write_text(" c");
				break;
			}
			"D" period? {
				self.scratch_pad.write_text(" d");
				break;
			}
			"E" period? {
				self.scratch_pad.write_text(" e");
				break;
			}
			"F" period? {
				self.scratch_pad.write_text(" f");
				break;
			}
			"G" period? {
				self.scratch_pad.write_text(" g");
				break;
			}
			"H" period? {
				self.scratch_pad.write_text(" h");
				break;
			}
			"I" period? {
				self.scratch_pad.write_text(" i");
				break;
			}
			"J" period? {
				self.scratch_pad.write_text(" j");
				break;
			}
			"K" period? {
				self.scratch_pad.write_text(" k");
				break;
			}
			"L" period? {
				self.scratch_pad.write_text(" l");
				break;
			}
			"M" period? {
				self.scratch_pad.write_text(" m");
				break;
			}
			"N" period? {
				self.scratch_pad.write_text(" n");
				break;
			}
			"O" period? {
				self.scratch_pad.write_text(" o");
				break;
			}
			"P" period? {
				self.scratch_pad.write_text(" p");
				break;
			}
			"Q" period? {
				self.scratch_pad.write_text(" q");
				break;
			}
			"R" period? {
				self.scratch_pad.write_text(" r");
				break;
			}
			"S" period? {
				self.scratch_pad.write_text(" s");
				break;
			}
			"T" period? {
				self.scratch_pad.write_text(" t");
				break;
			}
			"U" period? {
				self.scratch_pad.write_text(" u");
				break;
			}
			"V" period? {
				self.scratch_pad.write_text(" v");
				break;
			}
			"W" period? {
				self.scratch_pad.write_text(" w");
				break;
			}
			"X" period? {
				self.scratch_pad.write_text(" x");
				break;
			}
			"Y" period? {
				self.scratch_pad.write_text(" y");
				break;
			}
			"Z" period? {
				self.scratch_pad.write_text(" z");
				break;
			}

			/* NORMAL ROUTINE FOR LETTERS */
			letter {
				if self.is_plural  { self.is_plural  = false; }
				if self.punct_prev { self.punct_prev = false; }
				if self.digit_prev { self.digit_prev = false; }
				buf.recede();
				self.scratch_pad.write(buf.read());
				buf.advance();
				break;
			}

			/* ROUTINE FOR NUMBERS STARTING WITH ZEROES */
			@t1 "0" whole_num {
				self.digit_prev = true;
				yymarker = buf.getcurs();
				buf.setcurs(t1);
				while buf.getcurs() < yymarker {
					self.reset_ntw();
					self.consume_digit(buf.read());
					self.ntw(true, false);
					buf.advance();
				}
				break;
			}

			/* ROUTINE FOR NUMBERS GREATER THAN QUADRILLION */
			@t1 whole_num{19,} {
				self.digit_prev = true;
				yymarker = buf.getcurs();
				buf.setcurs(t1);
				while buf.getcurs() < yymarker {
					self.reset_ntw();
					self.consume_digit(buf.read());
					self.ntw(true, false);
					buf.advance();
				}
				break;
			}

			/* ROUTINE FOR U.S. PHONE NUMBERS */
			@t1 phone_num {
				self.digit_prev = true;
				yymarker = buf.getcurs();
				buf.setcurs(t1);
				while buf.getcurs() < yymarker {
					if buf.read() == 0x2D || buf.read() == 0x20 {
						self.scratch_pad.write_text(" , ");
					} else if self.is_number(buf.read()) {
						self.reset_ntw();
						self.consume_digit(buf.read());
						self.ntw(true, false);
					}
					buf.advance();
				}
				break;
			}

			/* ROUTINE FOR ORDINAL NUMBERS */
			@t1 whole_num ordinal_tag {
				// this will be a naive method.
				// non-matching ordinal suffixes
				// will be fixed automatically
				self.digit_prev = false;
				yymarker = buf.getcurs();
				buf.setcurs(t1);
				self.reset_ntw();
				while buf.getcurs() < yymarker {
					self.consume_digit(buf.read());
					buf.advance();
				}
				self.ntw(false, true);
				break;
			}

			/* ROUTINES FOR PROCESSING MOST NUMBERS */
			@t1 plusminus? @t2 numspecial? @t3 whole_num @t4 decimal? @t5 {
				yymarker = buf.getcurs();
				if t1 != t2 {
					buf.setcurs(t1);
					if buf.read() == 0x2B {
						self.scratch_pad.write_text(" positive ");
					} else if self.digit_prev {
						self.scratch_pad.write_text(" through ");
					} else {
						self.scratch_pad.write_text(" negative ");
					}
				}
				if t2 != t3 {
					buf.setcurs(t2);
					if buf.read() == 0x23 {
						self.scratch_pad.write_text(" number ");
					}
				}
				buf.setcurs(t3);
				self.reset_ntw();
				while buf.getcurs() < t4 {
					self.consume_digit(buf.read());
					buf.advance();
				}
				self.ntw(false, false);
				if t2 != t3 {
					buf.setcurs(t2);
					if buf.read() == 0x24 {
						self.scratch_pad.write_text(" dollar");
						if self.is_plural {
							self.scratch_pad.write_text("s");
						}
						self.scratch_pad.write_text(" ");
						if t4 != t5 {
							self.scratch_pad.write_text(" and ");
							buf.setcurs(t4);
							self.reset_ntw();
							while buf.getcurs() < t5 {
								self.consume_digit(buf.read());
								buf.advance();
							}
							self.ntw(false, false);
							self.scratch_pad.write_text(" cent");
							if self.is_plural {
								self.scratch_pad.write_text("s");
							}
							self.scratch_pad.write_text(" ");
						}
					}
				} else if t4 != t5 {
					self.scratch_pad.write_text(" point ");
					buf.setcurs(t4 + 1);
					while buf.getcurs() < t5 {
						self.reset_ntw();
						self.consume_digit(buf.read());
						self.ntw(true, false);
						buf.advance();
					}
				}
				self.digit_prev = true;
				buf.setcurs(yymarker);
				break;
			}

			/* LEFT OVER DOLLAR SIGN RULE */
			"$" {
				self.scratch_pad.write_text(" dollar sign ");
				break;
			}
			/* LEFT OVER HASH RULE */
			"#" {
				// treat hash symbol not attached to number
				// as "hash tag"
				self.scratch_pad.write_text(" hash tag ");
				break;
			}
			/* LEFT OVER PLUS RULE */
			"+" {
				self.scratch_pad.write_text(" plus ");
				break;
			}
			/* COMPOUND HYPHEN RULE */
			"-" letter {
				buf.recede();
				self.scratch_pad.write_text(" ");
				break;
			}
			/* LEFT OVER HYPHEN RULE */
			"-" {
				if self.digit_prev {
					self.scratch_pad.write_text(" , ");
				} else {
					self.scratch_pad.write_text(" dash ");
				}
				break;
			}
			/* LEFT OVER PERIOD RULE */
			"." letter {
				// translate left over period to "dot" if letter is right
				buf.recede();
				self.scratch_pad.write_text(" dot ");
				break;
			}
			"." {
				self.scratch_pad.write_text(" . ");
				break;
			}
			/* LEFT OVER RULE OVERALL */
			* {
				// output whitespace when no match is made including whitespace
				self.scratch_pad.write_text(" ");
				break;
			}
			/* END OF TEXT */
			[\x00] {
				// end of stream. send signal to synthesizer that the text stream is done.
				text_done = true;
				break;
			}
		%}
		text_done
	}
}
