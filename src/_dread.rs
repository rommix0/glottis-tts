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
	start:    usize,
	memcount: usize,
	counter:  usize,
	wordload: [u8; 32],
	wordlen:  u8,
	funcword: bool,
	is_valid: bool,
}
impl dictreader {
	pub fn new() -> dictreader {
		dictreader {
			start:    0,
			memcount: 0,
			counter:  0,
			wordload: [0; 32],
			wordlen:  0,
			funcword: false,
			is_valid: false,
		}
	}
	pub fn reset(&mut self) { *self = dictreader::new(); }
	pub fn isFunctionWord(&mut self) -> bool { self.funcword }
	pub fn readIn(&mut self, x: u8) {
		// read in the word to look up
		if self.counter < 32 {
			if x > 0x20 && x < 0x7F {
				self.is_valid = true;
				self.wordload[self.counter] = x;
				self.counter += 1;
				self.wordlen += 1;
			}
		} else {
			self.is_valid = false;
		}
	}
	pub fn readOut(&mut self) -> u8 {
		// read out phoneme stream of found word
		let xx = dictdat[self.memcount];
		if xx != 0xFF { self.memcount += 1; }
		xx
	}
	pub fn search(&mut self) -> bool {
		// return false if not valid
		if !self.is_valid { return false; }
		
		// find the word using naive lookup method
		// set up some variables
		let mut found:   bool = false;

		self.start = getLetterOffset(self.wordload[0].to_ascii_lowercase());
		self.memcount = self.start;

		// search loop
		loop {

			// end of partition, break out
			if dictdat[self.memcount] == 0xFD { break; }

			// check if function word
			self.funcword = dictdat[self.memcount] == 1;
			self.memcount += 1;
			self.start = self.memcount;

			// match loop
			for wmode in 0..2 {
				let mut strcomp = true;
				let mut length: u8 = 0;
				self.counter  = 0;
				self.memcount = self.start;
				loop {
					let mut x = self.wordload[self.counter];
					let y = dictdat[self.memcount];
					if wmode == 1 { x = x.to_ascii_lowercase(); }

					self.counter  += 1;
					self.memcount += 1;
					if y == 0xFE { break; }
					length += 1;

					if x != y { strcomp = false; }
				}
				if strcomp && length == self.wordlen {
					found = true;
					break;
				}
			}

			if found { break; }
			// skip this entry's phones
			loop {
				let y = dictdat[self.memcount];
				self.memcount += 1;
				if y == 0xFF { break; }
			}
		}
		found
	}
}
