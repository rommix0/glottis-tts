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
		}
	}
	pub fn reset(&mut self) {
		self.start    = 0;
		self.memcount = 0;
		self.counter  = 0;
		self.funcword = false;
		self.wordlen  = 0;
		for x in 0..32 { self.wordload[x] = 0; }
	}
	fn upperToLower(&mut self, x: u8) -> u8 {
		let xx = x;
		match xx {
			0x41 => 0x61,
			0x42 => 0x62,
			0x43 => 0x63,
			0x44 => 0x64,
			0x45 => 0x65,
			0x46 => 0x66,
			0x47 => 0x67,
			0x48 => 0x68,
			0x49 => 0x69,
			0x4A => 0x6A,
			0x4B => 0x6B,
			0x4C => 0x6C,
			0x4D => 0x6D,
			0x4E => 0x6E,
			0x4F => 0x6F,
			0x50 => 0x70,
			0x51 => 0x71,
			0x52 => 0x72,
			0x53 => 0x73,
			0x54 => 0x74,
			0x55 => 0x75,
			0x56 => 0x76,
			0x57 => 0x77,
			0x58 => 0x78,
			0x59 => 0x79,
			0x5A => 0x7A,
			_    => xx,
		}
	}
	pub fn isFunctionWord(&mut self) -> bool { self.funcword }
	pub fn readIn(&mut self, x: u8) {
		// read in the word to look up
		if self.counter < 32 && x > 0x20 && x < 0x7F {
			self.wordload[self.counter] = x;
			self.counter += 1;
			self.wordlen += 1;
		}
	}
	pub fn readOut(&mut self) -> u8 {
		// read out phoneme stream of found word
		let xx = dictdat[self.memcount];
		if xx != 0xFF { self.memcount += 1; }
		xx
	}
	pub fn search(&mut self) -> bool {
		// find the word using naive lookup method
		// set up some variables
		let mut x:       u8   = 0;
		let mut y:       u8   = 0;
		let mut strcomp: bool = false;
		let mut found:   bool = false;
		let mut length:  u8   = 0;

		self.start = getLetterOffset(self.upperToLower(self.wordload[0]));
		self.memcount = self.start;

		// search loop
		loop {

			// end of partition, break out
			if dictdat[self.memcount] == 0xFD { break; }

			// check if function word
			self.funcword = if dictdat[self.memcount] == 1 { true } else { false };
			self.memcount += 1;
			self.start = self.memcount;

			// match loop
			for wmode in 0..2 {
				strcomp = true;
				length  = 0;
				self.counter  = 0;
				self.memcount = self.start;
				loop {
					x = self.wordload[self.counter];
					y = dictdat[self.memcount];
					if wmode == 1 { x = self.upperToLower(x); }

					self.counter  += 1;
					self.memcount += 1;
					if x == 0 || y == 0xFE { break; }
					length += 1;

					//println!("wordload: {}, dictdat: {}", x, y);

					if x != y && strcomp { strcomp = false; }
				}
				if strcomp {
					if length != self.wordlen {
						strcomp = false;
					} else {
						found = true;
						break;
					}
				}
				/*if wmode == 0 {
					println!("==============");
					println!("TRY LOWERCASE!");
					println!("==============");
				}*/
			}

			// found routine
			if found {
				//println!("FOUND!");
				break;
			} else {
				/*println!("");
				println!("SKIP TO NEXT ONE!");
				println!("");*/
				loop {
					y = dictdat[self.memcount];
					self.memcount += 1;
					if y == 0xFF { break; }
				}
			}
		}
		found
	}
}
