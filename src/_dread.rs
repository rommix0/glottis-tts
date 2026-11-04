/*
DICTIONARY READER

dictionary format:
==================

+ jump table containing offset for each starting letter of the alphabet

+ entry format:

    0xFD: end of letter partition
        - when no match is made, and this is read back, no entry has been
          found. use LTS mode.
    0xFE: end of word mode
        - check if strcmp was 0 the whole time. if not, skip this entry
          and read to 0xFF without retrieving anything
    0xFF: end of entry

    1st byte    -> content/function (function is 0x1, content is 0x0)
    2nd to 0xFE -> ASCII word
    .. to 0xFF  -> phonemes

*/

use crate::_dict::dictdat;
use crate::_dict::getLetterOffset;

pub struct dictreader {
	pos:      usize,		// read position in dictdat
	word:     [u8; 32],		// the word to look up
	wordlen:  u8,
	funcword: bool,
	is_valid: bool,
}
impl dictreader {
	pub fn new() -> dictreader {
		dictreader {
			pos:      0,
			word:     [0; 32],
			wordlen:  0,
			funcword: false,
			is_valid: false,
		}
	}
	pub fn reset(&mut self) { *self = dictreader::new(); }
	pub fn isFunctionWord(&self) -> bool { self.funcword }
	pub fn readIn(&mut self, x: u8) {
		// read in the word to look up
		if (self.wordlen as usize) < 32 {
			if x > 0x20 && x < 0x7F {
				self.is_valid = true;
				self.word[self.wordlen as usize] = x;
				self.wordlen += 1;
			}
		} else {
			self.is_valid = false;
		}
	}
	pub fn readOut(&mut self) -> u8 {
		// read out phoneme stream of found word
		let x = dictdat[self.pos];
		if x != 0xFF { self.pos += 1; }
		x
	}
	pub fn search(&mut self) -> bool {
		// a naive scan through the word's letter partition; on a match pos is at its phones
		if !self.is_valid { return false; }
		self.pos = getLetterOffset(self.word[0].to_ascii_lowercase());
		loop {
			if dictdat[self.pos] == 0xFD { return false; }
			self.funcword = dictdat[self.pos] == 1;
			self.pos += 1;
			let entry = self.pos;
			for lowercase in [false, true] {
				let mut matched = true;
				let mut length: u8 = 0;
				let mut i = 0;
				self.pos = entry;
				loop {
					let mut x = self.word[i];
					let y = dictdat[self.pos];
					if lowercase { x = x.to_ascii_lowercase(); }
					i += 1;
					self.pos += 1;
					if y == 0xFE { break; }
					length += 1;
					if x != y { matched = false; }
				}
				if matched && length == self.wordlen { return true; }
			}
			// skip this entry's phones
			loop {
				let y = dictdat[self.pos];
				self.pos += 1;
				if y == 0xFF { break; }
			}
		}
	}
}
