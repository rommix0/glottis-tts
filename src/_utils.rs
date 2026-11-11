/*

UTILITIES

the ring buffer the front end streams text and tokens through,
and the table from phone tokens to phones and stress

*/
use alloc::{vec, vec::Vec};
use crate::_phone::*;

// ring buffer
pub struct cbuf {
	size:   isize,			// buffer size (a power of two; doubles when full)
	buffer: Vec<u8>,		// growable buffer
	head:   isize,			// producer
	tail:   isize,			// consumer
	prev:   u8				// previous character for trimming when writing text
}
impl cbuf {
	pub fn new() -> cbuf {
		cbuf {
			size:   2048,
			buffer: vec![0; 2048],
			head:   0,
			tail:   0,
			prev:   0,
		}
	}
	pub fn is_not_empty(&self) -> bool { self.head != self.tail }
	fn grow(&mut self) {
		// double the buffer, moving the queued bytes to the start in order.
		// this makes saved cursors stale, so only grow while filling, before reading with saved cursors
		let mut b = vec![0u8; 2 * self.size as usize];
		let mut n = 0;
		let mut t = self.tail;
		while t != self.head {
			b[n] = self.buffer[t as usize];
			n += 1;
			t = (t + 1) & (self.size - 1);
		}
		self.buffer = b;
		self.size *= 2;
		self.tail = 0;
		self.head = n as isize;
	}
	pub fn recede(&mut self) {
		self.tail = (self.tail - 1) & (self.size - 1);
	}
	pub fn advance(&mut self) {
		if self.is_not_empty() {
			self.tail = (self.tail + 1) & (self.size - 1);
		}
	}
	pub fn getcurs(&mut self) -> usize  { self.tail as usize }
	pub fn setcurs(&mut self, x: usize) { self.tail = x as isize; }
	pub fn read(&self) -> u8 { if self.is_not_empty() { self.buffer[self.tail as usize] } else { 0x20 } }
	pub fn write(&mut self, x: u8) {
		if (self.head + 1) & (self.size - 1) == self.tail { self.grow(); }
		self.buffer[self.head as usize] = x;
		self.head = (self.head + 1) & (self.size - 1);
		self.prev = x;
	}
	pub fn write_text(&mut self, txt: &str) {
		for x in txt.bytes() {
			// NUL is the stream's end marker, so turn one in the text into a space
			let x = if x == 0 { 0x20 } else { x };
			if self.prev != 0x20 || x != 0x20 { self.write(x); }
		}
	}
}

// a phone token as (phone, second phone or SIL for none, stress, second stress)
pub fn PHNIDToPhoneme(token: u8) -> (u8, u8, u8, u8) {
	match token {
		0            => (AA, SIL, 2, 0),		// "aa
		1            => (AE, SIL, 2, 0),		// "ae
		2            => (AH, SIL, 2, 0),		// "ah
		3            => (AO, SIL, 2, 0),		// "ao
		4            => (AW, SIL, 2, 0),		// "aw
		5            => (AY, SIL, 2, 0),		// "ay
		6            => (EH, SIL, 2, 0),		// "eh
		7            => (ER, SIL, 2, 0),		// "er
		8            => (EY, SIL, 2, 0),		// "ey
		9            => (IH, SIL, 2, 0),		// "ih
		10           => (IY, SIL, 2, 0),		// "iy
		11           => (OW, SIL, 2, 0),		// "ow
		12           => (OY, SIL, 2, 0),		// "oy
		13           => (UH, SIL, 2, 0),		// "uh
		14           => (UW, SIL, 2, 0),		// "uw
		15           => (AA, SIL, 1, 0),		// 'aa
		16           => (AE, SIL, 1, 0),		// 'ae
		17           => (AH, SIL, 1, 0),		// 'ah
		18           => (AO, SIL, 1, 0),		// 'ao
		19           => (AW, SIL, 1, 0),		// 'aw
		20           => (AY, SIL, 1, 0),		// 'ay
		21           => (EH, SIL, 1, 0),		// 'eh
		22           => (EH, M, 1, 0),			// 'eh|m
		23           => (ER, SIL, 1, 0),		// 'er
		24           => (EY, SIL, 1, 0),		// 'ey
		25           => (IH, SIL, 1, 0),		// 'ih
		26           => (IY, SIL, 1, 0),		// 'iy
		27           => (OW, SIL, 1, 0),		// 'ow
		28           => (OY, SIL, 1, 0),		// 'oy
		29           => (UH, SIL, 1, 0),		// 'uh
		30           => (UW, SIL, 1, 0),		// 'uw
		32           => (AA, SIL, 0, 0),		// aa
		33           => (AE, SIL, 0, 0),		// ae
		34           => (AO, SIL, 0, 0),		// ao
		35           => (AW, SIL, 0, 0),		// aw
		36           => (AX, SIL, 0, 0),		// ax
		37           => (AXR, SIL, 0, 0),		// axr
		38           => (AY, SIL, 0, 0),		// ay
		39           => (B, SIL, 0, 0),			// b
		40           => (B, Y, 0, 0),			// b|y
		41           => (CH, SIL, 0, 0),		// ch
		42           => (D, SIL, 0, 0),			// d
		43           => (DH, SIL, 0, 0),		// dh
		44           => (EH, SIL, 0, 0),		// eh
		45           => (EY, SIL, 0, 0),		// ey
		46           => (F, SIL, 0, 0),			// f
		47           => (F, Y, 0, 0),			// f|y
		48           => (G, SIL, 0, 0),			// g
		49           => (G, Y, 0, 0),			// g|y
		50           => (G, Z, 0, 0),			// g|z
		51           => (HH, SIL, 0, 0),		// hh
		52           => (HH, Y, 0, 0),			// hh|y
		53           => (IX, SIL, 0, 0),		// ix
		54           => (IY, SIL, 0, 0),		// iy
		55           => (JH, SIL, 0, 0),		// jh
		56           => (K, SIL, 0, 0),			// k
		57           => (K, S, 0, 0),			// k|s
		58           => (K, Y, 0, 0),			// k|y
		59           => (L, SIL, 0, 0),			// l
		60           => (LX, SIL, 0, 0),		// lx
		61           => (M, SIL, 0, 0),			// m
		62           => (M, AX, 0, 0),			// m|ax
		63           => (M, Y, 0, 0),			// m|y
		64           => (N, SIL, 0, 0),			// n
		65           => (NG, SIL, 0, 0),		// ng
		66           => (OW, SIL, 0, 0),		// ow
		67           => (OY, SIL, 0, 0),		// oy
		68           => (P, SIL, 0, 0),			// p
		69           => (P, Y, 0, 0),			// p|y
		70           => (R, SIL, 0, 0),			// r
		71           => (RX, SIL, 0, 0),		// rx
		72           => (S, SIL, 0, 0),			// s
		73           => (SH, SIL, 0, 0),		// sh
		74           => (S, IY, 0, 2),			// s|"iy
		75           => (S, IY, 0, 1),			// s|'iy
		76           => (T, SIL, 0, 0),			// t
		77           => (TH, SIL, 0, 0),		// th
		78           => (UH, SIL, 0, 0),		// uh
		79           => (UW, SIL, 0, 0),		// uw
		80           => (V, SIL, 0, 0),			// v
		81           => (W, SIL, 0, 0),			// w
		82           => (Y, SIL, 0, 0),			// y
		83           => (Y, UW, 0, 2),			// y|"uw
		84           => (Y, UW, 0, 1),			// y|'uw
		85           => (Y, AX, 0, 0),			// y|ax
		86           => (Y, UW, 0, 0),			// y|uw
		87           => (Z, SIL, 0, 0),			// z
		88           => (ZH, SIL, 0, 0),		// zh
		89           => (Z, AX, 0, 0),			// z|ax
		TOKEN_CW     => (CW, SIL, 0, 0),		// content word boundary
		TOKEN_COMMA  => (COMMA, SIL, 0, 0),		// comma
		TOKEN_PERIOD => (PERIOD, SIL, 0, 0),	// period
		TOKEN_QMARK  => (QMARK, SIL, 0, 0),		// question mark
		TOKEN_BANG   => (BANG, SIL, 0, 0),		// exclamation point
		TOKEN_FW     => (FW, SIL, 0, 0),		// function word boundary
		_            => (SIL, SIL, 0, 0),		// otherwise
	}
}
