/*

TEXT NORMALIZER

reads numbers, dates, money, roman numerals, abbreviations, units and
homographs out as words, so the rest of the front end sees only words
and punctuation.

_textskeleton.rs is the re2c source of _text.rs: edit the skeleton,
then run tools/build_normalizer.py to regenerate _text.rs.

*/
use crate::_utils::cbuf;
// fixed-text rules (homographs, abbreviations, units, emoji) from tools/normalizer_rules.txt
use crate::_text_rules::{RULES, FIRST};
use crate::_dread::dictreader;
use alloc::{format, string::{String, ToString}, vec, vec::Vec};

// a Roman numeral from II to XXXIX written the standard way
fn roman(s: &str) -> Option<u64> {
	if s.len() < 2 || !s.bytes().all(|c| matches!(c, b'I' | b'V' | b'X')) { return None; }
	let v = (1..=39).find(|&v| {
		let tens = &"XXX"[..v / 10];
		let ones = ["", "I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX"][v % 10];
		s.strip_prefix(tens) == Some(ones)
	})?;
	Some(v as u64)
}

// a day number ends here: not followed by more of a number, an ordinal ending, a time or a fraction
fn ends_day(c: u8, d: u8) -> bool {
	!c.is_ascii_alphanumeric() && c != b'/' && !(matches!(c, b':' | b',' | b'.') && d.is_ascii_digit())
}

fn is_space(c: u8) -> bool { matches!(c, b' ' | b'\t' | b'\n') }

fn consume(buf: &mut cbuf, n: usize) { for _ in 0..n { buf.advance(); } }

// English text normalizer: numbers, homographs, abbreviations
pub struct engine {
	scratch_pad: cbuf,
	// a number's plurality and "after a digit" carry on across lex() calls until a letter or punctuation clears them
	digit_prev:  bool,
	is_plural:   bool,
	number:      u64,
	last_year:   Option<bool>,	// whether the last four-digit number was read as a year, for the next in "X to Y"
	folded:      bool,			// accented letters are folded once, before the first lex()
	open_quote:  bool,			// an opening single quote was dropped: the next quote after a word closes it
}
impl engine {
	pub fn new() -> engine {
		engine {
			scratch_pad: cbuf::new(),
			digit_prev:  false,
			is_plural:   false,
			number:      0,
			last_year:   None,
			folded:      false,
			open_quote:  false,
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

	// write a word or words with a space either side
	fn say(&mut self, w: &str) {
		self.scratch_pad.write_text(" ");
		self.scratch_pad.write_text(w);
		self.scratch_pad.write_text(" ");
	}
	// write a pause or sentence mark; clears the number's plural and "after a digit"
	fn pause(&mut self, mark: &str) {
		self.digit_prev = false;
		self.is_plural = false;
		self.say(mark);
	}

	// NUMBER TO WORD PART
	fn reset_ntw(&mut self) { self.number = 0; }
	fn digits_aloud(&mut self, buf: &mut cbuf, from: usize, to: usize) {
		// each digit on its own ("007" oh oh seven), thousands commas skipped
		buf.setcurs(from);
		while buf.getcurs() != to {
			if buf.read().is_ascii_digit() {
				self.reset_ntw();
				self.consume_digit(buf.read());
				self.ntw(true, false);
			}
			buf.advance();
		}
	}
	fn consume_digit(&mut self, x: u8) {
		// saturates past u64 (20+ digits), which then reads the wrong digits; keep the digit text if that matters
		if x.is_ascii_digit() { self.number = self.number.saturating_mul(10).saturating_add((x - b'0') as u64); }
	}
	fn process_number(&mut self, x: u64, zero_oh: bool, is_ordinal: bool) {
		const ONES: [&str; 20] = ["zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
			"eleven", "twelve", "thirteen", "fourteen", "fifteen", "sixteen", "seventeen", "eighteen", "nineteen"];
		const ONES_TH: [&str; 20] = ["zeroth", "first", "second", "third", "fourth", "fifth", "sixth", "seventh",
			"eighth", "ninth", "tenth", "eleventh", "twelfth", "thirteenth", "fourteenth", "fifteenth", "sixteenth",
			"seventeenth", "eighteenth", "nineteenth"];
		const TENS: [&str; 10] = ["", "", "twen", "thir", "for", "fif", "six", "seven", "eigh", "nine"];
		const SCALES: [(u64, &str); 6] = [(1_000_000_000_000_000, "quadrillion"), (1_000_000_000_000, "trillion"),
			(1_000_000_000, "billion"), (1_000_000, "million"), (1000, "thousand"), (100, "hundred")];
		if x == 0 && zero_oh {
			self.scratch_pad.write_text(" oh ");
		} else if x < 20 {
			self.say(if is_ordinal { ONES_TH[x as usize] } else { ONES[x as usize] });
		} else if x < 100 {
			let (tens, ones) = (x / 10, x % 10);
			self.scratch_pad.write_text(" ");
			self.scratch_pad.write_text(TENS[tens as usize]);
			if is_ordinal && ones == 0 {
				self.scratch_pad.write_text("tieth ");
			} else {
				self.scratch_pad.write_text("ty");
				if ones > 0 {
					self.process_number(ones, zero_oh, is_ordinal);
				} else {
					self.scratch_pad.write_text(" ");
				}
			}
		} else if x < 1_000_000_000_000_000_000 {
			let &(unit, name) = SCALES.iter().find(|s| x >= s.0).unwrap();
			let unders = x % unit;
			self.process_number(x / unit, false, false);
			self.scratch_pad.write_text(name);
			if is_ordinal && unders == 0 {
				self.scratch_pad.write_text("th ");
			} else if unders > 0 {
				self.process_number(unders, zero_oh, is_ordinal);
			}
			self.scratch_pad.write_text(" ");
		} else {
			// past the largest scale: digit by digit
			let mut p = 1u64;
			while x / p >= 10 { p *= 10; }
			while p > 0 { self.process_number(x / p % 10, false, false); p /= 10; }
		}
	}
	fn ntw(&mut self, zero_oh: bool, is_ordinal: bool) {
		self.is_plural = self.number > 1;
		self.process_number(self.number, zero_oh, is_ordinal);
	}

	// CONTEXT RULE PART
	// rules that look further around than a fixed pattern: run before the fixed-text rules and the lexer
	fn ahead(buf: &mut cbuf, max: usize) -> Vec<u8> {
		// peek up to max bytes ahead (stops at the end of the text) without moving the cursor
		let start = buf.getcurs();
		let mut out = Vec::new();
		while out.len() < max {
			let c = buf.read();
			if c == 0x00 { break; }
			out.push(c);
			buf.advance();
		}
		buf.setcurs(start);
		out
	}
	fn prev_byte(buf: &mut cbuf) -> u8 {
		buf.recede();
		let c = buf.read();
		buf.advance();
		c
	}
	fn prev_word(buf: &mut cbuf) -> String {
		// get the word before the cursor (skipping spaces, tabs and newlines), as written
		let start = buf.getcurs();
		buf.recede();
		while is_space(buf.read()) { buf.recede(); }
		let mut w = Vec::new();
		while buf.read().is_ascii_alphabetic() && w.len() < 32 {
			w.push(buf.read());
			buf.recede();
		}
		buf.setcurs(start);
		w.reverse();
		String::from_utf8(w).unwrap_or_default()
	}
	fn number_before_prev_word(buf: &mut cbuf) -> bool {
		// is there a four-digit number right before the previous word ("1987 to")
		let start = buf.getcurs();
		buf.recede();
		while buf.read() == b' ' { buf.recede(); }
		for _ in 0..32 { if !buf.read().is_ascii_alphabetic() { break; } buf.recede(); }
		while buf.read() == b' ' { buf.recede(); }
		let mut n = 0;
		while buf.read().is_ascii_digit() && n < 5 { n += 1; buf.recede(); }
		let found = n == 4 && !buf.read().is_ascii_alphanumeric();
		buf.setcurs(start);
		found
	}
	fn is_word(w: &str) -> bool {
		// the dictionary has it, or it is a word the letter-to-sound tree says right
		if crate::_morph::known(w) { return true; }
		let mut r = dictreader::new();
		for b in w.bytes() { r.readIn(b); }
		r.search()
	}
	fn words(&mut self, n: u64, ordinal: bool, zero_oh: bool) -> String {
		// spell out n as the number reader would
		let saved = core::mem::replace(&mut self.scratch_pad, cbuf::new());
		self.process_number(n, zero_oh, ordinal);
		let mut out = Vec::new();
		while self.is_not_empty() { out.push(self.read()); }
		self.scratch_pad = saved;
		// write_text already keeps one space between words
		String::from_utf8(out).unwrap_or_default().trim().into()
	}
	fn year_words(&mut self, y: u64) -> String {
		// 1999 nineteen ninety nine, 1905 nineteen oh five, 1900 nineteen hundred, 2005 two thousand five
		if (2000..=2009).contains(&y) || y < 1000 { return self.words(y, false, false); }
		let (hi, lo) = (y / 100, y % 100);
		let lo = if lo == 0 {
			"hundred".to_string()
		} else if lo < 10 {
			format!("oh {}", self.words(lo, false, false))
		} else {
			self.words(lo, false, false)
		};
		format!("{} {}", self.words(hi, false, false), lo)
	}
	fn plural_last(s: &str) -> String {
		// nineties, tens, hundreds, thousands
		match s.strip_suffix("ty") { Some(t) => format!("{}ties", t), None => format!("{}s", s) }
	}
	fn digits(a: &[u8], from: usize) -> (u64, usize) {
		// the value and length of the digit run at a[from..]
		let mut v = 0u64;
		let mut n = 0;
		while from + n < a.len() && a[from + n].is_ascii_digit() && n < 18 {
			v = v * 10 + (a[from + n] - b'0') as u64;
			n += 1;
		}
		(v, n)
	}
	fn fraction(a: &[u8], mut k: usize) -> (Vec<u8>, usize) {
		// read the digits after a decimal point at a[k], and where they end
		let at = |i: usize| a.get(i).copied().unwrap_or(0);
		let mut frac = Vec::new();
		if at(k) == b'.' && at(k + 1).is_ascii_digit() {
			k += 1;
			while at(k).is_ascii_digit() { frac.push(at(k) - b'0'); k += 1; }
		}
		(frac, k)
	}
	fn amount(a: &[u8], from: usize) -> (u64, Vec<u8>, usize) {
		// read the number at a[from..] (thousands commas, decimal digits), and where it ends
		let at = |i: usize| a.get(i).copied().unwrap_or(0);
		let (mut v, n) = Self::digits(a, from);
		let mut k = from + n;
		while at(k) == b',' && (1..=3).all(|i| at(k + i).is_ascii_digit()) && !at(k + 4).is_ascii_digit() {
			v = v.saturating_mul(1000).saturating_add(Self::digits(a, k + 1).0);
			k += 4;
		}
		let (frac, k) = Self::fraction(a, k);
		(v, frac, k)
	}
	fn scale_after(a: &[u8], k: usize) -> Option<(&'static str, usize)> {
		// find a scale word after spaces at a[k..] ("$20 billion"), and where it ends
		let at = |i: usize| a.get(i).copied().unwrap_or(0);
		let mut j = k;
		while is_space(at(j)) { j += 1; }
		if j == k { return None; }
		["thousand", "million", "billion", "trillion"].into_iter()
			.find(|w| a.get(j..j + w.len()).is_some_and(|x| x.eq_ignore_ascii_case(w.as_bytes()))
				&& !at(j + w.len()).is_ascii_alphabetic())
			.map(|w| (w, j + w.len()))
	}
	fn point(&mut self, frac: &[u8]) -> String {
		// " point five oh" for decimal digits, or nothing
		let p: String = frac.iter().map(|&d| format!(" {}", self.words(d as u64, false, true))).collect();
		if frac.is_empty() { p } else { format!(" point{}", p) }
	}
	fn context_rule(&mut self, buf: &mut cbuf) -> bool {
		const MONTHS: [&str; 12] = ["january", "february", "march", "april", "may", "june", "july", "august",
			"september", "october", "november", "december"];
		const SHORT_MONTHS: [(&str, usize); 13] = [("jan.", 0), ("feb.", 1), ("mar.", 2), ("apr.", 3), ("jun.", 5),
			("jul.", 6), ("aug.", 7), ("sep.", 8), ("sept.", 8), ("oct.", 9), ("nov.", 10), ("dec.", 11), ("may", 4)];
		// a numeral after one of these words is a plain number ("chapter four"); after another name, a ruler's
		// ordinal ("henry the eighth")
		const CARDINAL_AFTER: [&str; 16] = ["chapter", "part", "volume", "book", "act", "scene", "war", "phase",
			"level", "stage", "round", "season", "episode", "apollo", "rocky", "section"];
		// a four-digit number after one of these words is a year ("in 2015 votes were cast")
		const YEAR_AFTER: [&str; 15] = ["in", "since", "until", "till", "during", "circa", "year", "from", "by",
			"early", "late", "mid", "spring", "summer", "winter"];
		// a four-digit number after one of these words is a count, not a year ("over 1500", "scored 2015")
		const COUNT_AFTER: [&str; 21] = ["about", "over", "under", "nearly", "almost", "approximately", "roughly",
			"exactly", "only", "just", "than", "some", "total", "scored", "won", "lost", "paid", "cost", "costs",
			"earned", "sold"];
		// a four-digit number after one of these words is read like the one before it ("1987 to 1990", "2015 to 1987")
		const RANGE_AFTER: [&str; 4] = ["to", "and", "or", "through"];
		// words ending in s that are not plural nouns
		const NOT_PLURAL: [&str; 18] = ["was", "is", "has", "as", "its", "his", "hers", "ours", "yours", "theirs",
			"this", "thus", "plus", "minus", "always", "perhaps", "besides", "unless"];
		const PLURAL: [&str; 8] = ["people", "men", "women", "children", "feet", "teeth", "mice", "geese"];
		// homographs read by the word before: (word, capitalized reading is the verb, verb when nothing tells).
		// the other reading is the noun or adjective
		const STRESS_PAIRS: &[(&str, bool, bool)] = &[
			("address", true, false), ("affect", true, true), ("affects", true, true), ("attribute", false, false),
			("attributes", true, false), ("compound", true, false), ("conduct", true, false),
			("conflict", false, false), ("conflicts", false, false), ("console", false, false),
			("consoles", true, false), ("construct", true, false), ("constructs", true, false),
			("contest", false, false), ("contests", false, false), ("contract", false, false),
			("contracts", true, false), ("convert", false, true), ("convict", false, false), ("convicts", true, false),
			("defect", true, false), ("defects", true, false), ("desert", true, false), ("deserts", true, false),
			("object", false, false), ("objects", false, false), ("perfect", true, false), ("permit", true, false),
			("permits", true, false), ("present", false, false), ("presents", false, false), ("produce", true, true),
			("progress", true, false), ("progresses", true, true), ("project", false, false), ("projects", true, false),
			("rebel", true, false), ("rebels", true, false), ("record", true, false), ("records", true, false),
			("reject", true, true), ("rejects", true, true), ("survey", false, false), ("suspect", false, false),
			("suspects", false, false), ("advocate", true, false), ("advocates", true, false),
			("aggregate", true, false), ("alternate", true, false), ("animate", false, true),
			("appropriate", true, false), ("approximate", true, false), ("articulate", true, false),
			("associate", false, false), ("associates", false, false), ("coordinate", true, false),
			("coordinates", true, false), ("duplicate", false, false), ("duplicates", true, false),
			("estimate", false, false), ("estimates", true, false), ("graduate", false, false),
			("graduates", true, false), ("moderate", false, false), ("moderates", true, false),
			("separate", false, false), ("increase", false, true), ("increases", false, true),
			("decrease", false, true), ("decreases", false, true), ("import", false, true), ("imports", false, false),
			("export", true, false), ("exports", true, false), ("insult", false, true), ("insults", false, false),
			("protest", true, false), ("protests", true, false), ("subject", false, false), ("subjects", true, false),
			("transfer", false, true), ("transfers", false, false), ("transport", false, false),
			("transports", false, true), ("refund", false, false), ("refunds", false, false), ("contrast", true, false),
			("contrasts", true, false), ("update", false, true), ("updates", false, false), ("upgrade", false, true),
			("upgrades", false, false), ("discount", false, false), ("discounts", false, false)];
		// a verb follows these (the plain form) ...
		const BEFORE_VERB: &[&str] = &["to", "will", "would", "can", "could", "shall", "should", "may", "might", "must",
			"i", "you", "we", "they", "don't", "didn't", "won't", "can't", "cannot", "not", "never", "please", "let's",
			"who", "doesn't"];
		// ... and these (the form in -s)
		const BEFORE_VERB_S: &[&str] = &["he", "she", "it", "who", "that", "which"];
		// a noun or adjective follows these
		const BEFORE_NOUN: &[&str] = &["a", "an", "the", "this", "that", "these", "those", "my", "your", "his", "her",
			"its", "our", "their", "no", "every", "each", "any", "some", "another", "of", "new", "old", "first", "last",
			"final", "big", "great", "good", "best", "is", "are", "was", "were", "be", "been", "very", "so", "too",
			"quite"];
		// these come before a noun, so a word of STRESS_PAIRS (or wind, bow) right before one is a verb
		// ("Record a song")
		const DETERMINERS: &[&str] = &["a", "an", "the", "this", "that", "these", "those", "my", "your", "his", "her",
			"its", "our", "their", "some", "every", "each"];
		// "tear" before these is the rip ("a tear in the paper", "tear it up")
		const RIP_NEXT: &[&str] = &["in", "on", "up", "down", "apart", "off", "out", "through", "into", "it", "them"];
		// an adjective follows these ("it is close", "she was content")
		const BEFORE_ADJ: &[&str] = &["is", "are", "was", "were", "be", "been", "am", "so", "very", "too", "quite",
			"really", "fairly", "pretty", "stay", "stayed", "remain", "feel"];
		let a = Self::ahead(buf, 48);
		if a.is_empty() { return false; }
		let at = |i: usize| a.get(i).copied().unwrap_or(0);
		let prev = Self::prev_byte(buf);
		// next word after spaces at a[from..] (none if no space comes first), and a plural noun test
		let word_after = |from: usize| {
			let mut k = from;
			while is_space(at(k)) { k += 1; }
			let mut w = String::new();
			if k > from { while at(k).is_ascii_alphabetic() { w.push(at(k).to_ascii_lowercase() as char); k += 1; } }
			(w, k)
		};
		let plural = |w: &str| PLURAL.contains(&w) || (w.len() >= 3 && w.ends_with('s')
			&& !w.ends_with("ss") && !w.ends_with("us") && !w.ends_with("is") && !NOT_PLURAL.contains(&w));

		// an opening double quote is not read out
		if is_space(at(0)) {
			let q = if at(1) == b'"' {
				1
			} else if at(1) == 0xE2 && at(2) == 0x80 && (0x9C..=0x9F).contains(&at(3)) {
				3
			} else {
				0
			};
			if q > 0 && at(1 + q).is_ascii_alphanumeric() {
				self.scratch_pad.write_text(" ");
				consume(buf, 1 + q);
				return true;
			}
		}

		// a quote after a space: a decade ('90s), a word that keeps its apostrophe ('twas), or an opening quote
		if is_space(at(0)) {
			let q = if at(1) == b'\'' {
				1
			} else if a[1..].starts_with(&[0xE2, 0x80, 0x99]) || a[1..].starts_with(&[0xE2, 0x80, 0x98]) {
				3
			} else {
				0
			};
			if q > 0 {
				let r = 1 + q;
				if at(r).is_ascii_digit() && at(r + 1) == b'0' && at(r) != b'0' && at(r + 2) == b's'
					&& !at(r + 3).is_ascii_alphabetic() {
					let w = self.words(((at(r) - b'0') * 10) as u64, false, false);
					self.say(&Self::plural_last(&w));
					consume(buf, r + 3);
					return true;
				}
				if at(r).is_ascii_alphabetic() {
					let mut w = String::from("'");
					let mut k = r;
					while at(k).is_ascii_alphabetic() { w.push(at(k).to_ascii_lowercase() as char); k += 1; }
					if Self::is_word(&w) {
						self.say(&w);
						consume(buf, k);
					} else {
						self.scratch_pad.write_text(" ");
						consume(buf, r);
						self.open_quote = true;
					}
					return true;
				}
			}
		}

		// a sentence mark inside a closing double quote: a period becomes a comma if the sentence goes on
		if matches!(at(0), b'.' | b'?' | b'!') {
			let q = if at(1) == b'"' { 1 } else if a[1..].starts_with(&[0xE2, 0x80, 0x9D]) { 3 } else { 0 };
			if q > 0 {
				let mut k = 1 + q;
				while is_space(at(k)) { k += 1; }
				let mark = if at(0) == b'.' && at(k).is_ascii_lowercase() {
					","
				} else {
					match at(0) { b'?' => "?", b'!' => "!", _ => "." }
				};
				self.pause(mark);
				consume(buf, 1 + q);
				return true;
			}
		}

		// a run of ? and ! (or one interrobang) is one mark: ? if any of them asks, else !
		if !is_space(prev) {
			let (mut k, mut marks, mut asks, mut double_mark) = (0, 0, false, false);
			loop {
				match (at(k), at(k + 1), at(k + 2)) {
					(b'?', _, _) => { k += 1; asks = true; },
					(b'!', _, _) => { k += 1; },
					(0xE2, 0x80, 0xBC) => { k += 3; double_mark = true; },
					(0xE2, 0x80, 0xBD) | (0xE2, 0x81, 0x87..=0x89) => { k += 3; asks = true; double_mark = true; },
					_ => break,
				}
				marks += 1;
			}
			if marks >= 2 || (double_mark && asks) {
				self.pause(if asks { "?" } else { "!" });
				consume(buf, k);
				return true;
			}
		}

		// a decimal without its leading zero: .75 point seven five, also after a bracket or a quote ("(.5)")
		let d = is_space(at(0)) as usize;
		if at(d) == b'.' && at(d + 1).is_ascii_digit()
			&& (d == 1 || (!prev.is_ascii_alphanumeric() && !matches!(prev, b'.' | b'$'))) {
			let (frac, k) = Self::fraction(&a, d);
			let p = self.point(&frac);
			self.say(&p);
			consume(buf, k);
			self.digit_prev = true;
			return true;
		}

		// the minutes of a time under ten: 10:05 ten oh five
		if at(0) == b':' && prev.is_ascii_digit() && at(1) == b'0' && (b'1'..=b'9').contains(&at(2))
			&& !at(3).is_ascii_digit() {
			let m = self.words((at(2) - b'0') as u64, false, false);
			self.scratch_pad.write_text(&format!(" oh {} ", m));
			consume(buf, 3);
			self.digit_prev = true;
			return true;
		}

		// spell out a web address's scheme
		if !prev.is_ascii_alphabetic() {
			let low = a.to_ascii_lowercase();
			for (scheme, say) in [(&b"https://"[..], " h t t p s colon slash slash "),
				(b"http://", " h t t p colon slash slash ")] {
				if low.starts_with(scheme) && at(scheme.len()).is_ascii_alphabetic() {
					self.scratch_pad.write_text(say);
					consume(buf, scheme.len());
					return true;
				}
			}
		}

		// pounds, euros and yen: the amount, then the currency (and pence or cents)
		let currency = match (at(0), at(1), at(2)) {
			(0xC2, 0xA3, _) => Some((2, "pound", "pounds", "penny", "pence")),
			(0xE2, 0x82, 0xAC) => Some((3, "euro", "euros", "cent", "cents")),
			(0xC2, 0xA5, _) => Some((2, "yen", "yen", "", "")),
			_ => None,
		};
		if let Some((len, one, many, sub_one, sub_many)) = currency {
			if at(len).is_ascii_digit() {
				// the whole amount, with its thousands commas
				let (v, frac, mut k) = Self::amount(&a, len);
				let num = self.words(v, false, false);
				let text = if let Some((w, end)) = Self::scale_after(&a, k) {
					let point = self.point(&frac);
					k = end;
					format!(" {}{} {} {} ", num, point, w, many)
				} else {
					let unit = if v == 1 { one } else { many };
					let subunit = if frac.is_empty() || sub_one.is_empty() { String::new() } else {
						let c = frac[0] as u64 * 10 + frac.get(1).copied().unwrap_or(0) as u64;
						if c == 0 {
							String::new()
						} else {
							format!(" and {} {}", self.words(c, false, false), if c == 1 { sub_one } else { sub_many })
						}
					};
					// under one: only the pence or cents
					if v == 0 && !subunit.is_empty() {
						format!(" {} ", &subunit[5..])
					} else {
						format!(" {} {}{} ", num, unit, subunit)
					}
				};
				self.scratch_pad.write_text(&text);
				consume(buf, k);
				self.digit_prev = false;
				return true;
			}
		}

		// numbers at the start of a word
		if at(0).is_ascii_digit() && !prev.is_ascii_digit() && !matches!(prev, b'.' | b',' | b'/' | b':') {
			let (v1, n1) = Self::digits(&a, 0);
			let after = |i: usize| !at(i).is_ascii_digit() && at(i) != b'/';
			// a height in feet and inches: 6'2" six foot two
			if n1 == 1 && at(1) == b'\'' {
				let (v2, n2) = Self::digits(&a, 2);
				let q = if at(2 + n2) == b'"' {
					1
				} else if a.get(2 + n2..5 + n2) == Some(&[0xE2, 0x80, 0x9D][..]) {
					3
				} else {
					0
				};
				if (1..=2).contains(&n2) && v2 < 12 && q > 0 {
					let (f, i) = (self.words(v1, false, false), self.words(v2, false, false));
					self.scratch_pad.write_text(&format!(" {} foot {} ", f, i));
					consume(buf, 2 + n2 + q);
					return true;
				}
			}
			// an amount with cents before its currency word: 12.50 dollars
			if at(n1) == b'.' {
				let (c, nc) = Self::digits(&a, n1 + 1);
				let mut j = n1 + 1 + nc;
				while is_space(at(j)) { j += 1; }
				let low = a[j..].to_ascii_lowercase();
				let unit = [("dollars", "dollar"), ("dollar", "dollar"), ("euros", "euro"), ("euro", "euro")]
					.into_iter()
					.find(|(w, _)| low.starts_with(w.as_bytes()) && !at(j + w.len()).is_ascii_alphabetic());
				if let (2, true, Some((w, one))) = (nc, j > n1 + 3, unit) {
					let cents = if c == 1 {
						" one cent".to_string()
					} else {
						format!(" {} cents", self.words(c, false, false))
					};
					let whole = format!(" {} {}{}", self.words(v1, false, false), one, if v1 == 1 { "" } else { "s" });
					let text = match (v1, c) {
						(0, _) => cents,
						(_, 0) => whole,
						_ => format!("{} and{}", whole, cents)
					};
					self.scratch_pad.write_text(&format!("{} ", text));
					consume(buf, j + w.len());
					return true;
				}
			}
			// a unit after a number, with or without a space: 5ft, 2 hrs, 2.5kg
			// (last field: only when joined to the number)
			const UNITS: [(&str, &str, &str, bool); 26] = [
				("ft", "foot", "feet", false), ("in", "inch", "inches", true), ("yd", "yard", "yards", false),
				("yds", "yard", "yards", false), ("mi", "mile", "miles", false), ("hr", "hour", "hours", false),
				("hrs", "hour", "hours", false), ("min", "minute", "minutes", false),
				("mins", "minute", "minutes", false), ("sec", "second", "seconds", false),
				("secs", "second", "seconds", false), ("ms", "millisecond", "milliseconds", true),
				("mg", "milligram", "milligrams", false), ("kg", "kilogram", "kilograms", false),
				("kgs", "kilogram", "kilograms", false), ("lb", "pound", "pounds", false),
				("lbs", "pound", "pounds", false), ("oz", "ounce", "ounces", false),
				("ml", "milliliter", "milliliters", false), ("km", "kilometer", "kilometers", false),
				("cm", "centimeter", "centimeters", false), ("mm", "millimeter", "millimeters", false),
				("mph", "mile per hour", "miles per hour", false),
				("km/h", "kilometer per hour", "kilometers per hour", false),
				("km/hr", "kilometer per hour", "kilometers per hour", false), ("m", "meter", "meters", false)];
			{
				let (v, frac, k) = Self::amount(&a, 0);
				let joined = !is_space(at(k));
				let mut j = k;
				while is_space(at(j)) { j += 1; }
				let low = a[j..].to_ascii_lowercase();
				// "m" only in lower case and not before more of a unit: "3 M", "m/s" and "m\u{b2}" are not meters
				let meter = at(j) == b'm' && !matches!(at(j + 1), b'0'..=b'9' | b'/' | 0x80..=0xFF);
				// a spaced "in" is inches before punctuation, the end or a size word ("6 in, 2 lb", "12 in tall"),
				// else the preposition ("6 in the morning", "1 in 10")
				let inch = {
					let mut k = j + 2;
					if at(k) == b'.' { k += 1; }
					while is_space(at(k)) { k += 1; }
					let mut e = k;
					while at(e).is_ascii_alphabetic() { e += 1; }
					let next = a.get(k..e).unwrap_or(&[]).to_ascii_lowercase();
					!at(k).is_ascii_alphanumeric() && at(k) != b'$'
						|| matches!(&next[..], b"tall" | b"long" | b"wide" | b"high" | b"deep" | b"thick" | b"by" | b"x"
							| b"across" | b"square")
				};
				let unit = UNITS.iter()
					.filter(|u| (joined || !u.3 || (u.0 == "in" && inch)) && low.starts_with(u.0.as_bytes())
						&& !at(j + u.0.len()).is_ascii_alphabetic() && (u.0 != "m" || meter))
					.max_by_key(|u| u.0.len());
				if let Some(&(ab, one, many, _)) = unit {
					let num = self.words(v, false, false);
					let point = self.point(&frac);
					let name = if v == 1 && frac.is_empty() { one } else { many };
					self.scratch_pad.write_text(&format!(" {}{} {} ", num, point, name));
					// eat the abbreviation's period too when the sentence goes on in lower case ("5 ft. long")
					let mut k = j + ab.len() + 1;
					while is_space(at(k)) { k += 1; }
					let dot = at(j + ab.len()) == b'.' && k > j + ab.len() + 1 && at(k).is_ascii_lowercase();
					consume(buf, j + ab.len() + dot as usize);
					self.digit_prev = false;
					return true;
				}
			}
			// a date written year-month-day
			if n1 == 4 && at(4) == b'-' {
				let (m, nm) = Self::digits(&a, 5);
				let (d, nd) = Self::digits(&a, 8);
				if nm == 2 && at(7) == b'-' && nd == 2 && !at(10).is_ascii_digit() && (1..=12).contains(&m)
					&& (1..=31).contains(&d) {
					let day = self.words(d, true, false);
					let year = self.year_words(v1);
					self.scratch_pad.write_text(&format!(" {} {} , {} ", MONTHS[m as usize - 1], day, year));
					consume(buf, 10);
					self.digit_prev = true;
					return true;
				}
			}
			// a version or other run of three or more dotted numbers: 2.0.1 two point oh point one
			{
				let mut parts = vec![v1];
				let mut k = n1;
				while at(k) == b'.' && at(k + 1).is_ascii_digit() {
					let (v, n) = Self::digits(&a, k + 1);
					parts.push(v);
					k += 1 + n;
				}
				if parts.len() >= 3 && !at(k).is_ascii_alphabetic() {
					for (i, &v) in parts.iter().enumerate() {
						let w = self.words(v, false, true);
						self.scratch_pad.write_text(if i == 0 { " " } else { " point " });
						self.scratch_pad.write_text(&w);
					}
					self.scratch_pad.write_text(" ");
					consume(buf, k);
					self.digit_prev = true;
					return true;
				}
			}
			if at(n1) == b'/' {
				let (v2, n2) = Self::digits(&a, n1 + 1);
				let p2 = n1 + 1 + n2;
				// a date: month / day / year
				if n1 <= 2 && (1..=2).contains(&n2) && at(p2) == b'/' && (1..=12).contains(&v1)
					&& (1..=31).contains(&v2) {
					let (v3, n3) = Self::digits(&a, p2 + 1);
					if (n3 == 2 || n3 == 4) && after(p2 + 1 + n3) {
						let day = self.words(v2, true, false);
						let year = if n3 == 4 {
							self.year_words(v3)
						} else if v3 < 10 {
							format!("oh {}", self.words(v3, false, false))
						} else {
							self.words(v3, false, false)
						};
						self.scratch_pad.write_text(&format!(" {} {} , {} ", MONTHS[v1 as usize - 1], day, year));
						consume(buf, p2 + 1 + n3);
						self.digit_prev = true;
						return true;
					}
				}
				// a proper fraction up to sixteenths
				if n1 <= 2 && n2 <= 2 && n2 > 0 && after(p2) && v1 >= 1 && v1 < v2 && (2..=16).contains(&v2) {
					let num = self.words(v1, false, false);
					let den = match (v2, v1 > 1) {
						(2, false) => "half".to_string(), (2, true) => "halves".to_string(),
						(4, false) => "quarter".to_string(), (4, true) => "quarters".to_string(),
						(_, plural) => {
							let d = self.words(v2, true, false);
							if plural { format!("{}s", d) } else { d }
						},
					};
					self.scratch_pad.write_text(&format!(" {} {} ", num, den));
					consume(buf, p2);
					return true;
				}
				// an even split or a score: 50/50 fifty fifty, 60/40, 20/20
				if n1 == 2 && n2 == 2 && after(p2) && (v1 == v2 || v1 + v2 == 100) {
					let w = format!(" {} {} ", self.words(v1, false, false), self.words(v2, false, false));
					self.scratch_pad.write_text(&w);
					consume(buf, p2);
					return true;
				}
			}
			let ends = |i: usize| !at(i).is_ascii_alphanumeric();
			let plain = !(matches!(at(n1), b',' | b'\'' | b'_' | b'.' | b':') && at(n1 + 1).is_ascii_digit());
			// a decade: the 1990s, the 2000s, the 90s (a possessive s may follow an apostrophe)
			let s_at = if at(n1) == b's' {
				Some(n1)
			} else if at(n1) == b'\'' && at(n1 + 1) == b's' {
				Some(n1 + 1)
			} else {
				None
			};
			if let Some(s) = s_at {
				if v1 % 10 == 0 && ends(s + 1) && ((n1 == 4 && (1100..=2099).contains(&v1)) || (n1 == 2 && v1 >= 10)) {
					let w = if n1 == 4 { self.year_words(v1) } else { self.words(v1, false, false) };
					self.say(&Self::plural_last(&w));
					consume(buf, s + 1);
					return true;
				}
			}
			// a year: four plain digits from 1100 to 2099, or from 1001 after a word that leads one
			// ("in 1066", "in 2015 votes were cast"). otherwise not before a plural noun ("2015 votes"),
			// after a count word ("over 1500") or falling to the next number ("the vote was 2015 to 1987").
			// after "to", "and"... read it like the number before
			let ordinal = matches!((at(n1), at(n1 + 1)), (b's', b't') | (b'n', b'd') | (b'r', b'd') | (b't', b'h'));
			let before = if is_space(prev) { Self::prev_word(buf).to_ascii_lowercase() } else { String::new() };
			let cue = YEAR_AFTER.contains(&before.as_str());
			let (w, k) = word_after(n1);
			let counted = plural(&w);
			// "X to Y": a count when Y is smaller or counts something ("2015 to 1987", "1300 to 1400 visitors")
			let range_count = {
				let mut j = k;
				while is_space(at(j)) { j += 1; }
				let (v2, n2) = Self::digits(&a, j);
				w == "to" && n2 == 4 && ends(j + 4) && (v2 < v1 || plural(&word_after(j + 4).0))
			};
			let in_range = (1100..=1999).contains(&v1) || (2010..=2099).contains(&v1);
			let year = if cue { (1001..=1999).contains(&v1) || (2010..=2099).contains(&v1) }
				else if counted || range_count || COUNT_AFTER.contains(&before.as_str()) { false }
				else if RANGE_AFTER.contains(&before.as_str()) && Self::number_before_prev_word(buf)
					&& let Some(y) = self.last_year { y && in_range }
				else { in_range };
			if n1 == 4 && plain && !ordinal && ends(n1) && at(n1) != b'/' {
				self.last_year = Some(year);
			}
			if n1 == 4 && plain && !ordinal && ends(n1) && at(n1) != b'/' && year {
				let w = self.year_words(v1);
				self.say(&w);
				consume(buf, 4);
				self.digit_prev = true;
				self.is_plural = true;
				return true;
			}
		}

		// money with a scale word: $1.5 million, $20 billion
		if at(0) == b'$' && at(1).is_ascii_digit() {
			let (v, n) = Self::digits(&a, 1);
			let (frac, k) = Self::fraction(&a, 1 + n);
			if let Some((scale, end)) = Self::scale_after(&a, k) {
				let (num, point) = (self.words(v, false, false), self.point(&frac));
				self.scratch_pad.write_text(&format!(" {}{} {} dollars ", num, point, scale));
				consume(buf, end);
				return true;
			}
		}

		// a month and a day: march third, jan. fifth
		if is_space(at(0)) && at(1).is_ascii_alphabetic() {
			let low = a.to_ascii_lowercase();
			let mut found: Option<(usize, usize)> = None;
			for (m, name) in MONTHS.iter().enumerate() {
				if low[1..].starts_with(name.as_bytes()) { found = Some((m, name.len())); }
			}
			for (name, m) in SHORT_MONTHS {
				if low[1..].starts_with(name.as_bytes()) && found.is_none_or(|(_, n)| name.len() > n) {
					found = Some((m, name.len()));
				}
			}
			if let Some((m, n)) = found {
				let mut k = 1 + n;
				if is_space(at(k)) {
					while is_space(at(k)) { k += 1; }
					let (d, nd) = Self::digits(&a, k);
					if (1..=2).contains(&nd) && (1..=31).contains(&d) && ends_day(at(k + nd), at(k + nd + 1)) {
						let day = self.words(d, true, false);
						self.scratch_pad.write_text(&format!(" {} {} ", MONTHS[m], day));
						consume(buf, k + nd);
						self.digit_prev = true;
						return true;
					}
				}
			}
		}

		// "St.": saint before a name, street otherwise
		if is_space(at(0)) && a[1..].starts_with(b"St.") {
			let mut k = 4;
			while is_space(at(k)) { k += 1; }
			let saint = k > 4 && at(k).is_ascii_uppercase();
			self.scratch_pad.write_text(if saint { " saint " } else { " street " });
			consume(buf, 4);
			if !saint { self.end_after_period(buf, true); }
			return true;
		}

		// "www." spelled letter by letter
		if !prev.is_ascii_alphabetic() && a.len() >= 4 && a[..4].eq_ignore_ascii_case(b"www.") {
			self.scratch_pad.write_text(" w w w dot ");
			consume(buf, 4);
			return true;
		}

		// a word read by the word before it: "to record" against "a record", "it is close"
		if at(0).is_ascii_alphabetic() && !prev.is_ascii_alphabetic() && prev != b'\'' {
			let mut n = 0;
			while at(n).is_ascii_alphabetic() { n += 1; }
			let w: String = a[..n].iter().map(|c| c.to_ascii_lowercase() as char).collect();
			let plain = a[1..n].iter().all(u8::is_ascii_lowercase) && !matches!(at(n), b'\'' | b'-');
			let before = if is_space(prev) { Self::prev_word(buf).to_ascii_lowercase() } else { String::new() };
			let b = before.as_str();
			// the next two words, when only spaces come between
			let (next, k) = word_after(n);
			let next2 = if next.is_empty() || at(k) == b'\'' { String::new() } else { word_after(k).0 };
			let det_next = DETERMINERS.contains(&next.as_str());
			let pair = STRESS_PAIRS.iter().find(|p| p.0 == w);
			// the cues only look one word either side, so they miss a verb or noun further off
			// ("the wind that blew", "bow of the ship")
			let cap = if !plain { None } else if let Some(&(_, cap_verb, default_verb)) = pair {
				let verb = if w.ends_with('s') { BEFORE_VERB_S.contains(&b) } else { BEFORE_VERB.contains(&b) };
				let noun = !verb && BEFORE_NOUN.contains(&b);
				// no cue before but a determiner after: a verb ("Record a song"), not after a preposition
				// ("in contrast the")
				let verb = verb || (!noun && det_next
					&& !matches!(b, "in" | "on" | "at" | "by" | "for" | "with" | "from" | "into" | "without"
						| "under"));
				Some(if verb { cap_verb } else if noun { !cap_verb } else { cap_verb == default_verb })
			} else {
				match w.as_str() {
					"close" => Some(BEFORE_ADJ.contains(&b)),
					"content" => Some(BEFORE_ADJ.contains(&b) || next == "with"),
					// past tense after have/was..., or before a time gone by ("you read yesterday", "read it last
					// week"); not after "to" or a modal ("will read")
					"read" => {
						let modal = BEFORE_VERB.contains(&b) && !matches!(b, "i" | "you" | "we" | "they" | "who");
						let past = ["yesterday", "last", "earlier", "ago"];
						let gone = past.contains(&next.as_str()) || (!det_next && past.contains(&next2.as_str()));
						Some(matches!(b, "have" | "has" | "had" | "having" | "was" | "were" | "been" | "being" | "ve")
							|| (!modal && gone))
					},
					"wind" => Some(BEFORE_VERB.contains(&b)
						|| (!BEFORE_NOUN.contains(&b) && (det_next || next == "up" || next == "down"))),
					"bow" => Some(BEFORE_VERB.contains(&b) || det_next
						|| matches!(next.as_str(), "to" | "down" | "out" | "before")),
					// crying after a determiner ("a tear fell"), the rip before a particle ("a tear in the paper")
					"tear" => Some(DETERMINERS.contains(&b) && !RIP_NEXT.contains(&next.as_str())),
					"tears" => Some(!BEFORE_VERB_S.contains(&b) && !det_next && !RIP_NEXT.contains(&next.as_str())),
					// tiny before a plural noun ("the minute details") or after very/so/too
					"minute" => Some(matches!(b, "very" | "so" | "too") || (plural(&next) && next != "minutes"
						&& !matches!(b, "a" | "an" | "one" | "every" | "each" | "per" | "last" | "next" | "this"
							| "that"))),
					_ => None,
				}
			};
			if cap == Some(true) {
				// a word clears the number state before it (plural, after a digit), like any letter
				self.digit_prev = false;
				self.is_plural = false;
				self.say(&w.to_ascii_uppercase());
				consume(buf, n);
				return true;
			}
		}

		// a run of capitals: a Roman numeral, a word the dictionary knows (NASA, HELLO), or letters to spell
		if at(0).is_ascii_uppercase() && !prev.is_ascii_alphabetic() && prev != b'\'' {
			let mut n = 0;
			while at(n).is_ascii_uppercase() { n += 1; }
			// a fixed-text rule as long or longer here ("MPG", "PAT. PEND.") goes first
			let longer = Self::table_best(buf).is_some_and(|(len, _)| len >= n);
			if n >= 2 && !at(n).is_ascii_lowercase() && !longer {
				let run = core::str::from_utf8(&a[..n]).unwrap_or("");
				if let Some(v) = roman(run) {
					let before = Self::prev_word(buf);
					let ruler = before.as_bytes().first().is_some_and(|c| c.is_ascii_uppercase())
						&& !CARDINAL_AFTER.contains(&before.to_ascii_lowercase().as_str());
					let w = if ruler {
						format!("the {}", self.words(v, true, false))
					} else {
						self.words(v, false, false)
					};
					self.say(&w);
					consume(buf, n);
					return true;
				}
				let low = run.to_ascii_lowercase();
				// read two capitals as a word next to an all-capital word ("THIS IS IT", "THE US ARMY"),
				// spell them among lowercase words ("the UK")
				let caps = |w: &[u8]| w.len() >= 2 && w.iter().all(u8::is_ascii_uppercase);
				let k = n + a[n..].iter().take_while(|c| is_space(**c)).count();
				let next = &a[k..k + a[k..].iter().take_while(|c| c.is_ascii_alphabetic()).count()];
				let in_caps = n == 2 && !self.digit_prev
					&& (caps(Self::prev_word(buf).as_bytes()) || (k > n && caps(next)));
				if (n >= 3 || in_caps) && Self::is_word(&low) {
					self.say(&low);
					consume(buf, n);
					return true;
				}
			}
			// the article "A" before a word
			if at(0) == b'A' && n == 1 && is_space(at(1)) {
				let mut k = 1;
				while is_space(at(k)) { k += 1; }
				if at(k).is_ascii_alphabetic() && at(k + 1).is_ascii_alphabetic() {
					self.scratch_pad.write_text(" a");
					consume(buf, 1);
					return true;
				}
			}
		}

		// an emoticon after a space is the emoticon, not "colon"
		if is_space(at(0)) && matches!(at(1), b':' | b';' | b'=') {
			buf.advance();
			let face = Self::table_best(buf).is_some_and(|(n, _)| n >= 2 && n >= Self::lexer_len(buf));
			buf.recede();
			if face {
				self.scratch_pad.write_text(" ");
				buf.advance();
				return true;
			}
		}
		false
	}

	// FIXED-TEXT RULE PART
	fn match_len(buf: &mut cbuf, pat: &[u8]) -> Option<usize> {
		// the length of an exact pattern if it matches at the cursor (None if not), without moving the cursor
		Self::match_by(buf, pat, |c, p| c == p)
	}
	fn match_rule(buf: &mut cbuf, pat: &str, any_case: bool) -> Option<usize> {
		// the same for a fixed-text rule: a space matches any space, an any-case rule ignores case
		Self::match_by(buf, pat.as_bytes(), |c, p| {
			if p == b' ' {
				is_space(c)
			} else if any_case {
				c.to_ascii_lowercase() == p
			} else {
				c == p
			}
		})
	}
	fn match_by(buf: &mut cbuf, pat: &[u8], same: impl Fn(u8, u8) -> bool) -> Option<usize> {
		let start = buf.getcurs();
		let mut hit = true;
		for &p in pat {
			if !same(buf.read(), p) {
				hit = false;
				break;
			}
			buf.advance();
		}
		buf.setcurs(start);
		if hit { Some(pat.len()) } else { None }
	}
	fn lexer_len(buf: &mut cbuf) -> usize {
		// the longest match at the cursor of the lexer rules a fixed-text rule can lose to:
		// " for" or " the" before a vowel, a ":00" time, a "__tag__"
		let start = buf.getcurs();
		let mut best = 0;
		for w in [&b" for"[..], &b" the"[..]] {
			if let Some(n) = Self::match_len(buf, w) {
				consume(buf, n);
				let mut k = n;
				while buf.read() == b' ' { buf.advance(); k += 1; }
				if k > n && matches!(buf.read(), b'a' | b'e' | b'i' | b'o' | b'u') { best = best.max(k + 1); }
				buf.setcurs(start);
			}
		}
		if let Some(n) = Self::match_len(buf, b":00") {
			let mut k = n;
			consume(buf, n);
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
		let mut n = 0;
		loop {
			let save = (buf.getcurs(), n);
			if matches!(buf.read(), b',' | b'\'' | b'_') { buf.advance(); n += 1; }
			if !buf.read().is_ascii_digit() { buf.setcurs(save.0); n = save.1; break; }
			while buf.read().is_ascii_digit() { buf.advance(); n += 1; }
		}
		if n > 0 {
			let c = buf.read();
			buf.advance();
			let d = buf.read();
			if c == b'.' && d.is_ascii_digit() {
				n += 1;
				while buf.read().is_ascii_digit() { buf.advance(); n += 1; }
			} else if matches!((c, d), (b's', b't') | (b'n', b'd') | (b'r', b'd') | (b't', b'h')) {
				n += 2;
			}
		}
		buf.setcurs(start);
		n
	}
	fn end_after_period(&mut self, buf: &mut cbuf, before_capital: bool) {
		// let an abbreviation's period also end the sentence when only spaces follow,
		// or (before_capital) when the next word starts with a capital
		let start = buf.getcurs();
		while is_space(buf.read()) { buf.advance(); }
		let c = buf.read();
		buf.setcurs(start);
		if c == 0x00 || (before_capital && c.is_ascii_uppercase()) { self.scratch_pad.write_text(" . "); }
	}
	fn behind(buf: &mut cbuf, max: usize) -> Vec<u8> {
		// peek up to max bytes behind, in text order (stops at the start of the text) without moving the cursor
		let start = buf.getcurs();
		let mut out = Vec::new();
		while out.len() < max {
			buf.recede();
			let c = buf.read();
			if c == 0x00 { break; }
			out.push(c);
		}
		buf.setcurs(start);
		out.reverse();
		out
	}
	fn initial(buf: &mut cbuf) -> bool {
		// are the capital and period before the cursor a name's initial ("John F. Kennedy", "J. R. R. Tolkien")?
		// yes if an initial is next to it, or a capitalized word comes before it inside the sentence.
		// a name starting a sentence reads as a sentence end ("John F. Kennedy spoke." reads "john f . kennedy")
		let a = Self::ahead(buf, 8);
		let k = a.iter().take_while(|c| is_space(**c)).count();
		if a.get(k).is_some_and(u8::is_ascii_uppercase) && a.get(k + 1) == Some(&b'.') { return true; }
		let b = Self::behind(buf, 40);
		let at = |i: usize| if i < b.len() { b[i] } else { 0 };
		// the last letter of a capital run is no initial ("the UK. Then", "a TV. Then")
		if b.len() >= 3 && at(b.len() - 3).is_ascii_alphabetic() { return false; }
		let mut i = b.len().saturating_sub(2);
		while i > 0 && is_space(at(i - 1)) { i -= 1; }
		if i >= 2 && at(i - 1) == b'.' && at(i - 2).is_ascii_uppercase()
			&& (i < 3 || !at(i - 3).is_ascii_alphabetic()) { return true; }
		let end = i;
		while i > 0 && at(i - 1).is_ascii_alphabetic() { i -= 1; }
		let capital = end > i && at(i).is_ascii_uppercase();
		while i > 0 && is_space(at(i - 1)) { i -= 1; }
		capital && i > 0 && (at(i - 1).is_ascii_alphabetic() || at(i - 1) == b',')
	}
	fn fold(buf: &mut cbuf) {
		// fold accented Latin letters to plain ones ("café" cafe, "Straße" strasse), whole text, before any is read
		const FOLD: [(&str, &str); 21] = [("ÀÁÂÃÄÅ", "A"), ("àáâãäå", "a"), ("Ç", "C"), ("ç", "c"),
			("ÈÉÊË", "E"), ("èéêë", "e"), ("ÌÍÎÏ", "I"), ("ìíîï", "i"), ("Ñ", "N"), ("ñ", "n"),
			("ÒÓÔÕÖØ", "O"), ("òóôõöø", "o"), ("ÙÚÛÜ", "U"), ("ùúûü", "u"), ("Ý", "Y"),
			("ýÿ", "y"), ("ß", "ss"), ("Æ", "AE"), ("æ", "ae"), ("Œ", "OE"), ("œ", "oe")];
		let start = buf.getcurs();
		let mut text = Vec::new();
		while buf.is_not_empty() { text.push(buf.read()); buf.advance(); }
		buf.setcurs(start);
		let Ok(s) = core::str::from_utf8(&text) else { return };
		if s.is_ascii() { return; }
		let mut out = cbuf::new();
		let mut b = [0u8; 4];
		for c in s.chars() {
			let to = FOLD.iter().find(|f| f.0.contains(c)).map_or(&*c.encode_utf8(&mut b), |f| f.1);
			for x in to.bytes() { out.write(x); }
		}
		*buf = out;
	}
	fn table_best(buf: &mut cbuf) -> Option<(usize, usize)> {
		// find the longest fixed-text rule at the cursor, the earlier on a tie: (length, rule)
		let first = buf.read();
		let mut best: Option<(usize, usize)> = None;
		for &r in FIRST[first as usize] {
			let (pat, any_case, _) = RULES[r as usize];
			if let Some(n) = Self::match_rule(buf, pat, any_case) {
				if best.is_none_or(|(b, _)| n > b) { best = Some((n, r as usize)); }
			}
		}
		best
	}
	fn table_rule(&mut self, buf: &mut cbuf) -> bool {
		// write the replacement of the longest fixed-text rule at the cursor, unless a lexer rule matches longer.
		// the lexer's " for"/" the" rules come first, so they also win a tie
		let first = buf.read();
		let Some((n, r)) = Self::table_best(buf) else { return false };
		// a rule that starts (ends) with a letter starts (ends) a word: no "kg" inside "background" or "kgb"
		let (pat, _, replacement) = RULES[r];
		let pat = pat.as_bytes();
		let letter = |c: Option<&u8>| c.is_some_and(|c| c.is_ascii_alphabetic());
		if letter(pat.first()) && Self::prev_byte(buf).is_ascii_alphabetic() { return false; }
		if letter(pat.last()) {
			consume(buf, n);
			let next = buf.read();
			for _ in 0..n { buf.recede(); }
			if next.is_ascii_alphabetic() { return false; }
		}
		let lex = Self::lexer_len(buf);
		if lex > n || (lex == n && matches!(first, b' ')) { return false; }
		consume(buf, n);
		// "bit{s} per second": the part in braces is added only after a plural number
		let (text, plural, rest) = match replacement.split_once('{') {
			Some((text, t)) => { let (plural, rest) = t.split_once('}').unwrap_or((t, "")); (text, plural, rest) }
			None => (replacement, "", ""),
		};
		self.scratch_pad.write_text(text);
		if self.is_plural && !plural.is_empty() {
			self.is_plural = false;
			self.scratch_pad.write_text(plural);
		}
		self.scratch_pad.write_text(rest);
		// a unit or abbreviation ends what a number before it started
		self.digit_prev = false;
		// a title stays with the name after it ("Dr. Smith"); other abbreviations end a sentence before a capital
		const TITLES: [&str; 20] = ["mister", "missus", "miz", "misters", "doctor", "professor", "sergeant", "corporal",
			"lieutenant", "major", "general", "colonel", "captain", "commander", "governor", "mount", "fort",
			"attorney general", "for example ,", "in other words ,"];
		if pat.last() == Some(&b'.') && !text.trim_end().ends_with('.') {
			self.end_after_period(buf, !TITLES.contains(&text.trim()));
		}
		true
	}

	// TEXT-HOMOGRAPH NORMALIZER PART
	pub fn lex(&mut self, buf: &mut cbuf) -> bool {
		let mut text_done: bool  = false;
		let mut yymarker: usize = 0;

		if !self.folded { self.folded = true; Self::fold(buf); }
		if self.context_rule(buf) { return false; }
		if self.table_rule(buf) { return false; }

		// Final tag variables available in semantic action.
		/*!svars:re2c format = '#[allow(unused_mut)]\nlet mut @@;\n'; */

		// Intermediate tag variables used by the lexer (must be autogenerated).
		/*!stags:re2c format = 'let mut @@ = usize::MAX;'; */

		%{
			/* generic API: the lexer reads through the ring buffer */
			re2c:api            = generic;
			re2c:YYPEEK         = "buf.read()";
			re2c:YYSKIP         = "buf.advance();";
			re2c:YYBACKUP       = "yymarker = buf.getcurs();";
			re2c:YYRESTORE      = "buf.setcurs(yymarker);";
			re2c:YYRESTORETAG   = "buf.setcurs(@@{tag});";
			re2c:YYSTAGP        = "@@{tag} = buf.getcurs();";
			re2c:YYSTAGN        = "@@{tag} = usize::MAX";
			re2c:YYSHIFTSTAG    = "@@{tag} = (@@{tag} as isize + @@{shift}) as usize; buf.setcurs(@@{tag})";

			re2c:define:YYCTYPE = u8;
			re2c:flags:utf-8    = 1;
			re2c:yyfill:enable  = 0;
			re2c:indent:top     = 1;
			re2c:tags           = 1;

			/* NORMAL GRAMMAR */
			space       = " " | "\x09" | "\x0A";
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

			/* homographs first: an uppercase word marks the other reading */

			" for" " "+ vowel {
				// "fer" becomes "for" before word-initial vowels
				buf.recede();
				self.scratch_pad.write_text(" FOR ");
				break;
			}
			" the" " "+ vowel {
				// "thee" before a vowel ("thee apple")
				buf.recede();
				self.scratch_pad.write_text(" thee ");
				break;
			}

			/*---------------------*/
			/* STATE ABBREVIATIONS */
			/*---------------------*/

			@t1 comma [ ]+ @t2 letter letter period? space {
				// a state in capitals after a comma: "Austin, TX" reads texas; lowercase "or", "pa", "al" stay words.
				// any other two letters: just the comma
				const STATES: [(&[u8; 2], &str); 51] = [
					(b"AK", "alaska"), (b"AL", "alabama"), (b"AR", "arkansas"), (b"AZ", "arizona"),
					(b"CA", "california"), (b"CO", "colorado"), (b"CT", "connecticut"), (b"DC", "district of columbia"),
					(b"DE", "delaware"), (b"FL", "florida"), (b"GA", "georgia"), (b"HI", "hawaii"),
					(b"IA", "iowa"), (b"ID", "idaho"), (b"IL", "illinois"), (b"IN", "indiana"),
					(b"KS", "kansas"), (b"KY", "kentucky"), (b"LA", "louisiana"), (b"MA", "massachusetts"),
					(b"MD", "maryland"), (b"ME", "maine"), (b"MI", "michigan"), (b"MN", "minnesota"),
					(b"MO", "missouri"), (b"MS", "mississippi"), (b"MT", "montana"), (b"NC", "north carolina"),
					(b"ND", "north dakota"), (b"NE", "nebraska"), (b"NH", "new hampshire"), (b"NJ", "new jersey"),
					(b"NM", "new mexico"), (b"NV", "nevada"), (b"NY", "new york"), (b"OH", "ohio"),
					(b"OK", "oklahoma"), (b"OR", "oregon"), (b"PA", "pennsylvania"), (b"RI", "rhode island"),
					(b"SC", "south carolina"), (b"SD", "south dakota"), (b"TN", "tennessee"), (b"TX", "texas"),
					(b"UT", "utah"), (b"VA", "virginia"), (b"VT", "vermont"), (b"WA", "washington"),
					(b"WI", "wisconsin"), (b"WV", "west virginia"), (b"WY", "wyoming"),
				];
				let end = buf.getcurs();
				buf.setcurs(t2);
				let pair = [buf.read(), { buf.advance(); buf.read() }];
				let dot = { buf.advance(); buf.read() == b'.' };
				if let Some(st) = STATES.iter().find(|st| *st.0 == pair) {
					buf.setcurs(end);
					self.say(st.1);
					if dot { self.end_after_period(buf, true); }
				} else {
					// as the comma rule
					buf.setcurs(t1);
					buf.advance();
					self.pause(",");
				}
				break;
			}

			/* section signs */

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
				self.scratch_pad.write_text(" exclamation point ");
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
				// between two numbers it is times: 5 * 3
				let next = Self::ahead(buf, 8).into_iter().find(|c| !is_space(*c)).unwrap_or(0);
				buf.recede();
				let before = Self::behind(buf, 8).into_iter().rev().find(|c| !is_space(*c)).unwrap_or(0);
				buf.advance();
				self.scratch_pad.write_text(if before.is_ascii_digit() && next.is_ascii_digit() {
					" times "
				} else {
					" asterisk "
				});
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
				self.pause(".");
				// leave the space, so an abbreviation starting the next sentence still has its word boundary
				buf.recede();
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
				// an apostrophe, or the closing quote of a dropped opening one ('single ones')
				if self.open_quote && !buf.read().is_ascii_alphabetic() {
					self.open_quote = false;
					self.scratch_pad.write_text(" ");
				} else {
					self.scratch_pad.write_text("\x27");
				}
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
				self.pause("!");
				break;
			}
			ques_mark {
				self.pause("?");
				break;
			}
			':00' " "? 'a' period? 'm' period? {
				self.scratch_pad.write_text(" A m ");
				if Self::prev_byte(buf) == b'.' { self.end_after_period(buf, true); }
				break;
			}
			':00' " "? 'p' period? 'm' period? {
				self.scratch_pad.write_text(" p m ");
				if Self::prev_byte(buf) == b'.' { self.end_after_period(buf, true); }
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
			'a.m.' {
				// a time of day; its period may also end the sentence
				self.digit_prev = false;
				self.scratch_pad.write_text(" A m ");
				self.end_after_period(buf, true);
				break;
			}
			'p.m.' {
				self.digit_prev = false;
				self.scratch_pad.write_text(" p m ");
				self.end_after_period(buf, true);
				break;
			}

			space "-" space {
				// a spaced dash is a pause, but minus between numbers: 5 - 3
				let end = buf.getcurs();
				let next = buf.read();
				for _ in 0..4 { buf.recede(); }
				let before = buf.read();
				buf.setcurs(end);
				if before.is_ascii_digit() && next.is_ascii_digit() {
					self.scratch_pad.write_text(" minus ");
				} else {
					self.pause(",");
				}
				break;
			}
			space? "\u2014" space? {
				// an em dash is a pause
				self.pause(",");
				break;
			}

			@t1 dquote {
				// a quote next to a comma adds no pause of its own: "Quoted," she said
				let end = buf.getcurs();
				let next = buf.read();
				buf.setcurs(t1);
				let before = Self::prev_byte(buf);
				buf.setcurs(end);
				if before == b',' || next == b',' {
					self.scratch_pad.write_text(" ");
				} else {
					self.pause(",");
				}
				break;
			}
			comma | space "(" | ")" space {
				self.pause(",");
				break;
			}

			/*---------------------*/
			/* FUN STUFF GOES HERE */
			/*---------------------*/

			fun_tag "version" fun_tag {
				self.scratch_pad.write_text(" version one point five ");
				break;
			}
			fun_tag "author" fun_tag {
				self.scratch_pad.write_text(" anthony c bartman ");
				break;
			}
			fun_tag "build_date" fun_tag {
				self.scratch_pad.write_text(" november tenth , twenty twenty six ");
				break;
			}

			/*-----------------*/
			/* LEFT OVER RULES */
			/*-----------------*/

			/* DEAL WITH UPPERCASE LETTERS FIRST */
			[A-Z] period? {
				// a capital letter alone is spelled ("A" stays the article's capital)
				let dot = Self::prev_byte(buf) == b'.';
				if dot { buf.recede(); }
				let c = Self::prev_byte(buf);
				if dot { buf.advance(); }
				self.scratch_pad.write_text(" ");
				self.scratch_pad.write(if c == b'A' { c } else { c.to_ascii_lowercase() });
				if dot { let initial = Self::initial(buf); self.end_after_period(buf, !initial); }
				break;
			}

			/* NORMAL ROUTINE FOR LETTERS */
			letter {
				self.is_plural  = false;
				self.digit_prev = false;
				self.scratch_pad.write(Self::prev_byte(buf));
				break;
			}

			/* ROUTINE FOR U.S. PHONE NUMBERS */
			@t1 phone_num {
				self.digit_prev = true;
				let end = buf.getcurs();
				buf.setcurs(t1);
				while buf.getcurs() < end {
					if matches!(buf.read(), b'-' | b' ') {
						self.scratch_pad.write_text(" , ");
					} else if buf.read().is_ascii_digit() {
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
				// the suffix is not checked: "21th" reads twenty first
				self.digit_prev = false;
				let end = buf.getcurs();
				buf.setcurs(t1);
				self.reset_ntw();
				while buf.getcurs() < end {
					self.consume_digit(buf.read());
					buf.advance();
				}
				self.ntw(false, true);
				break;
			}

			/* ROUTINES FOR PROCESSING MOST NUMBERS */
			@t1 plusminus? @t2 numspecial? @t3 whole_num @t4 decimal? @t5 {
				let end = buf.getcurs();
				// a bare number starting with a zero or past quadrillions: digit by digit
				buf.setcurs(t3);
				let zero = buf.read() == b'0';
				let mut digits = 0;
				while buf.getcurs() != t4 { digits += buf.read().is_ascii_digit() as usize; buf.advance(); }
				// (past quadrillions, a sign, "$" or "#" is still read with it)
				let long = digits >= 19;
				if t1 == t3 && t4 == t5 && ((zero && digits > 1) || long) {
					self.digit_prev = true;
					self.digits_aloud(buf, t3, end);
					break;
				}
				if t1 != t2 {
					buf.setcurs(t1);
					// skip the sign if the word before already names it ("Negative -42", "plus +5");
					// "minus" or "plus" after a number is the operator ("5 minus -3")
					let said = Self::prev_word(buf).to_ascii_lowercase();
					let operator = {
						let b = Self::behind(buf, 48);
						let b = b.trim_ascii_end();
						b[..b.len().saturating_sub(said.len())].trim_ascii_end().last().is_some_and(u8::is_ascii_digit)
					};
					let named = |sign: &str, op: &str| said == sign || (said == op && !operator);
					if buf.read() == b'+' {
						if !named("positive", "plus") { self.scratch_pad.write_text(" positive "); }
					} else if self.digit_prev && Self::prev_byte(buf).is_ascii_digit() {
						// a falling or even pair is a score (won 3-2, three to two), a rising one a range (pages 3-5).
						// always a range after a time, a decimal, a run of numbers (10:30-11:30, 555-1234-7)
						// or a number read digit by digit
						let b = Self::behind(buf, 24);
						let mut i = b.len();
						while i > 0 && (b[i - 1].is_ascii_digit() || b[i - 1] == b',') { i -= 1; }
						let whole = i < b.len() && b[i].is_ascii_digit() && (b[i] != b'0' || b.len() - i == 1)
							&& b.len() - i < 19 && (i == 0 || !matches!(b[i - 1], b':' | b'.' | b'-'));
						let value = |d: &[u8]| d.iter()
							.filter(|c| c.is_ascii_digit())
							.fold(0u64, |v, c| v.saturating_mul(10).saturating_add((c - b'0') as u64));
						let was = value(&b[i..]);
						buf.setcurs(t3);
						let mut now = Vec::new();
						while buf.getcurs() < t4 { now.push(buf.read()); buf.advance(); }
						buf.setcurs(t1);
						let score = whole && value(&now) <= was;
						self.scratch_pad.write_text(if score { " to " } else { " through " });
					} else if !named("negative", "minus") {
						self.scratch_pad.write_text(" negative ");
					}
				}
				if t2 != t3 {
					buf.setcurs(t2);
					if buf.read() == b'#' {
						self.scratch_pad.write_text(" number ");
					}
				}
				// dollars: the cents first, since under a dollar only the cents are read and .00 reads no cents
				let dollar = t2 != t3 && { buf.setcurs(t2); buf.read() == b'$' };
				self.reset_ntw();
				buf.setcurs(t4);
				while buf.getcurs() < t5 {
					self.consume_digit(buf.read());
					buf.advance();
				}
				let cents = self.number;
				buf.setcurs(t3);
				self.reset_ntw();
				while buf.getcurs() < t4 {
					self.consume_digit(buf.read());
					buf.advance();
				}
				let cents_only = dollar && cents > 0 && self.number == 0 && !long;
				if long {
					self.digits_aloud(buf, t3, t4);
					self.is_plural = true;
				} else if !cents_only {
					self.ntw(false, false);
				}
				if t2 != t3 {
					if dollar {
						if !cents_only {
							self.scratch_pad.write_text(" dollar");
							if self.is_plural {
								self.scratch_pad.write_text("s");
							}
							self.scratch_pad.write_text(" ");
						}
						if cents > 0 {
							if !cents_only { self.scratch_pad.write_text(" and "); }
							self.number = cents;
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
				buf.setcurs(end);
				break;
			}

			/* LEFT OVER DOLLAR SIGN RULE */
			"$" {
				self.scratch_pad.write_text(" dollar sign ");
				break;
			}
			/* LEFT OVER HASH RULE */
			"#" {
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
				// a dot inside a name: "example.com"
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
				// end of text: tell the caller the stream is done
				text_done = true;
				break;
			}
		%}
		text_done
	}
}
