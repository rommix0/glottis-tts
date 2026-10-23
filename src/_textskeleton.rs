// bring in cbuf so lexer can use it
use crate::_utils::cbuf;
// fixed-text rules (homographs, abbreviations, units, emoji) from tools/normalizer_rules.txt
use crate::_text_rules::{RULES, FIRST};

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

	// FIXED-TEXT RULE PART
	fn match_len(buf: &mut cbuf, pat: &[u8]) -> Option<usize> {
		// how many bytes of the input at the cursor the pattern matches, if it does; the cursor stays put
		let start = buf.getcurs();
		let (mut i, mut n) = (0, 0);
		let hit = loop {
			if i == pat.len() { break true; }
			let c = buf.read();
			let ok = match pat[i] {
				0x01 => { i += 1; c == b' ' || c == 0x08 || c == 0x0A },
				0x02 => { i += 2; c.is_ascii_alphabetic() && c.to_ascii_lowercase() == pat[i - 1] },
				p => { i += 1; c == p },
			};
			if !ok { break false; }
			buf.advance();
			n += 1;
		};
		buf.setcurs(start);
		if hit { Some(n) } else { None }
	}
	fn lexer_len(buf: &mut cbuf) -> usize {
		// the longest match at the cursor of the lexer rules a fixed-text rule can lose to:
		// " for" or " the" before a vowel, a ":00" time, a "__tag__"
		let start = buf.getcurs();
		let mut best = 0;
		for w in [&b" for"[..], &b" the"[..]] {
			if let Some(n) = Self::match_len(buf, w) {
				for _ in 0..n { buf.advance(); }
				let mut k = n;
				while buf.read() == b' ' { buf.advance(); k += 1; }
				if k > n && matches!(buf.read(), b'a' | b'e' | b'i' | b'o' | b'u') { best = best.max(k + 1); }
				buf.setcurs(start);
			}
		}
		if let Some(n) = Self::match_len(buf, b":00") {
			let mut k = n;
			for _ in 0..n { buf.advance(); }
			if buf.read() == b' ' { buf.advance(); k += 1; }
			let ap = buf.read().to_ascii_lowercase();
			if ap == b'a' || ap == b'p' {
				buf.advance();
				let mut j = k + 1;
				if buf.read() == b'.' { buf.advance(); j += 1; }
				if buf.read().to_ascii_lowercase() == b'm' {
					buf.advance();
					j += 1;
					if buf.read() == b'.' { j += 1; }
					k = j;
				} else {
					k = n;
				}
			} else {
				k = n;
			}
			best = best.max(k);
			buf.setcurs(start);
		}
		for tag in [&b"__version__"[..], b"__author__", b"__build_date__"] {
			if let Some(n) = Self::match_len(buf, tag) { best = best.max(n); }
		}
		best.max(Self::number_len(buf))
	}
	fn number_len(buf: &mut cbuf) -> usize {
		// a number may start with a digit separator ("_00"), so it can outlast a fixed-text rule for "_"
		let start = buf.getcurs();
		let digit = |c: u8| c.is_ascii_digit();
		let mut n = 0;
		loop {
			let save = (buf.getcurs(), n);
			if matches!(buf.read(), b',' | b'\'' | b'_') { buf.advance(); n += 1; }
			if !digit(buf.read()) { buf.setcurs(save.0); n = save.1; break; }
			while digit(buf.read()) { buf.advance(); n += 1; }
		}
		if n > 0 {
			let c = buf.read();
			buf.advance();
			let d = buf.read();
			if c == b'.' && digit(d) {
				n += 1;
				while digit(buf.read()) { buf.advance(); n += 1; }
			} else if matches!((c, d), (b's', b't') | (b'n', b'd') | (b'r', b'd') | (b't', b'h')) {
				n += 2;
			}
		}
		buf.setcurs(start);
		n
	}
	fn table_rule(&mut self, buf: &mut cbuf) -> bool {
		// the longest fixed-text rule at the cursor (the earlier on a tie) writes its replacement, unless a lexer
		// rule matches longer there; the lexer's " for"/" the" rules come first, so they also win a tie
		let first = buf.read();
		let mut best: Option<(usize, usize)> = None;
		for &r in FIRST[first as usize] {
			if let Some(n) = Self::match_len(buf, RULES[r as usize].0) {
				if best.map_or(true, |(b, _)| n > b) { best = Some((n, r as usize)); }
			}
		}
		let Some((n, r)) = best else { return false };
		let lex = Self::lexer_len(buf);
		if lex > n || (lex == n && matches!(first, b' ')) { return false; }
		for _ in 0..n { buf.advance(); }
		let (_, text, plural, rest) = RULES[r];
		self.scratch_pad.write_text(text);
		if self.is_plural && !plural.is_empty() {
			self.is_plural = false;
			self.scratch_pad.write_text(plural);
		}
		self.scratch_pad.write_text(rest);
		true
	}

	// TEXT-HOMOGRAPH NORMALIZER PART
	pub fn lex(&mut self, buf: &mut cbuf) -> bool {
		let mut text_done: bool  = false;
		let mut yymarker: usize = 0;

		self.punct_prev = false;
		self.digit_prev = false;
		self.is_plural  = false;

		if self.table_rule(buf) { return false; }

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

			/*-------------------------*/
			/* SOME PUNCTUATIONS RULES */
			/*-------------------------*/

			space exc_point {
				self.scratch_pad.write_text(" exclaimation point ");
				break;
			}
			"(" space {
				self.scratch_pad.write_text(" left parenthesis ");
				break;
			}
			space ")" {
				self.scratch_pad.write_text(" right parenthesis ");
				break;
			}
			"*" {
				self.scratch_pad.write_text(" asterisk ");
				break;
			}
			space comma {
				self.scratch_pad.write_text(" comma ");
				break;
			}
			space period {
				self.scratch_pad.write_text(" period ");
				break;
			}
			space ":" {
				self.scratch_pad.write_text(" colon ");
				break;
			}
			space ";" {
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
			space ques_mark {
				self.scratch_pad.write_text(" question mark ");
				break;
			}
			"[" space {
				self.scratch_pad.write_text(" left square bracket ");
				break;
			}
			space dquote {
				self.scratch_pad.write_text(" double quote ");
				break;
			}
			space squote {
				self.scratch_pad.write_text(" single quote ");
				break;
			}
			squote {
				// write out the apostrophe
				self.scratch_pad.write_text("\x27");
				break;
			}
			space "]" {
				self.scratch_pad.write_text(" right square bracket ");
				break;
			}
			"^" {
				self.scratch_pad.write_text(" caret ");
				break;
			}
			"{" space {
				self.scratch_pad.write_text(" left curly bracket ");
				break;
			}
			space "}" {
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
				self.scratch_pad.write_text(" version one point one ");
				break;
			}
			fun_tag "author" fun_tag {
				self.scratch_pad.write_text(" anthony c bartman ");
				break;
			}
			fun_tag "build_date" fun_tag {
				self.scratch_pad.write_text(" august twenty ninth , twenty twenty six ");
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
