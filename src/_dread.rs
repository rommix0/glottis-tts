/*
DICTIONARY READER

dictionary format:
==================

+ jump table containing offset for each starting letter of the alphabet

+ entry format:

    1st byte   -> how many letters the word shares with the entry before it (0-31),
                  plus 0x20 for a function word
    then       -> the rest of the word's letters, ASCII, the last one with 0x80 set
    then       -> the phonemes, the last one with 0x80 set

    0xFF in place of the 1st byte ends the letter partition: no entry has
    been found, use LTS mode.

*/

use crate::_dict::dictdat;
use crate::_dict::getLetterOffset;

pub struct dictreader {
	pos:      usize,		// read position in dictdat
	done:     bool,			// the found word's last phone has been read out
	word:     [u8; 32],		// the word to look up
	wordlen:  u8,
	funcword: bool,
	is_valid: bool,
}
impl dictreader {
	pub fn new() -> dictreader {
		dictreader {
			pos:      0,
			done:     true,
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
		// read out phoneme stream of found word, then 0xFF
		if self.done { return 0xFF; }
		let x = dictdat[self.pos];
		self.pos += 1;
		self.done = x & 0x80 != 0;
		x & 0x7F
	}
	pub fn search(&mut self) -> bool {
		// a naive scan through the word's letter partition, rebuilding each entry's word from the one before;
		// an entry matches the word as given or in lowercase. On a match pos is at its phones
		if !self.is_valid { return false; }
		let n = self.wordlen as usize;
		let mut entry = [0u8; 32];
		let mut pos = getLetterOffset(self.word[0].to_ascii_lowercase());
		loop {
			let head = dictdat[pos];
			if head == 0xFF { return false; }
			pos += 1;
			let mut len = (head & 0x1F) as usize;
			loop {
				let c = dictdat[pos];
				pos += 1;
				entry[len] = c & 0x7F;
				len += 1;
				if c & 0x80 != 0 { break; }
			}
			if len == n && (entry[..n] == self.word[..n]
				|| entry[..n].iter().zip(&self.word).all(|(&e, w)| e == w.to_ascii_lowercase())) {
				self.funcword = head & 0x20 != 0;
				self.pos = pos;
				self.done = false;
				return true;
			}
			// skip this entry's phones
			while dictdat[pos] & 0x80 == 0 { pos += 1; }
			pos += 1;
		}
	}
}
