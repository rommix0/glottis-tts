use alloc::{vec, vec::Vec};

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
		// double the buffer, keeping the queued bytes in order from the start; cursors saved before this are
		// stale, so a buffer only grows while it is filled, before it is read with saved cursors
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
			// a NUL in the text would end it early: it is the stream's end marker
			let x = if x == 0 { 0x20 } else { x };
			if self.prev != 0x20 || x != 0x20 { self.write(x); }
		}
	}
}

// a phone token as (phone, second phone, stress, second stress)
pub fn PHNIDToPhoneme(token: u8) -> (u8, u8, u8, u8) {
	match token {
		0  => (1, 0, 2, 0),		// "aa
		1  => (2, 0, 2, 0),		// "ae
		2  => (3, 0, 2, 0),		// "ah
		3  => (4, 0, 2, 0),		// "ao
		4  => (5, 0, 2, 0),		// "aw
		5  => (8, 0, 2, 0),		// "ay
		6  => (13, 0, 2, 0),	// "eh
		7  => (14, 0, 2, 0),	// "er
		8  => (15, 0, 2, 0),	// "ey
		9  => (19, 0, 2, 0),	// "ih
		10 => (21, 0, 2, 0),	// "iy
		11 => (29, 0, 2, 0),	// "ow
		12 => (30, 0, 2, 0),	// "oy
		13 => (38, 0, 2, 0),	// "uh
		14 => (39, 0, 2, 0),	// "uw
		15 => (1, 0, 1, 0),		// 'aa
		16 => (2, 0, 1, 0),		// 'ae
		17 => (3, 0, 1, 0),		// 'ah
		18 => (4, 0, 1, 0),		// 'ao
		19 => (5, 0, 1, 0),		// 'aw
		20 => (8, 0, 1, 0),		// 'ay
		21 => (13, 0, 1, 0),	// 'eh
		22 => (13, 26, 1, 0),	// 'eh|m
		23 => (14, 0, 1, 0),	// 'er
		24 => (15, 0, 1, 0),	// 'ey
		25 => (19, 0, 1, 0),	// 'ih
		26 => (21, 0, 1, 0),	// 'iy
		27 => (29, 0, 1, 0),	// 'ow
		28 => (30, 0, 1, 0),	// 'oy
		29 => (38, 0, 1, 0),	// 'uh
		30 => (39, 0, 1, 0),	// 'uw
		32 => (1, 0, 0, 0),		// aa
		33 => (2, 0, 0, 0),		// ae
		34 => (4, 0, 0, 0),		// ao
		35 => (5, 0, 0, 0),		// aw
		36 => (6, 0, 0, 0),		// ax
		37 => (7, 0, 0, 0),		// axr
		38 => (8, 0, 0, 0),		// ay
		39 => (9, 0, 0, 0),		// b
		40 => (9, 42, 0, 0),	// b|y
		41 => (10, 0, 0, 0),	// ch
		42 => (11, 0, 0, 0),	// d
		43 => (12, 0, 0, 0),	// dh
		44 => (13, 0, 0, 0),	// eh
		45 => (15, 0, 0, 0),	// ey
		46 => (16, 0, 0, 0),	// f
		47 => (16, 42, 0, 0),	// f|y
		48 => (17, 0, 0, 0),	// g
		49 => (17, 42, 0, 0),	// g|y
		50 => (17, 43, 0, 0),	// g|z
		51 => (18, 0, 0, 0),	// hh
		52 => (18, 42, 0, 0),	// hh|y
		53 => (20, 0, 0, 0),	// ix
		54 => (21, 0, 0, 0),	// iy
		55 => (22, 0, 0, 0),	// jh
		56 => (23, 0, 0, 0),	// k
		57 => (23, 34, 0, 0),	// k|s
		58 => (23, 42, 0, 0),	// k|y
		59 => (24, 0, 0, 0),	// l
		60 => (25, 0, 0, 0),	// lx
		61 => (26, 0, 0, 0),	// m
		62 => (26, 6, 0, 0),	// m|ax
		63 => (26, 42, 0, 0),	// m|y
		64 => (27, 0, 0, 0),	// n
		65 => (28, 0, 0, 0),	// ng
		66 => (29, 0, 0, 0),	// ow
		67 => (30, 0, 0, 0),	// oy
		68 => (31, 0, 0, 0),	// p
		69 => (31, 42, 0, 0),	// p|y
		70 => (32, 0, 0, 0),	// r
		71 => (33, 0, 0, 0),	// rx
		72 => (34, 0, 0, 0),	// s
		73 => (35, 0, 0, 0),	// sh
		74 => (34, 21, 0, 2),	// s|"iy
		75 => (34, 21, 0, 1),	// s|'iy
		76 => (36, 0, 0, 0),	// t
		77 => (37, 0, 0, 0),	// th
		78 => (38, 0, 0, 0),	// uh
		79 => (39, 0, 0, 0),	// uw
		80 => (40, 0, 0, 0),	// v
		81 => (41, 0, 0, 0),	// w
		82 => (42, 0, 0, 0),	// y
		83 => (42, 39, 0, 2),	// y|"uw
		84 => (42, 39, 0, 1),	// y|'uw
		85 => (42, 6, 0, 0),	// y|ax
		86 => (42, 39, 0, 0),	// y|uw
		87 => (43, 0, 0, 0),	// z
		88 => (44, 0, 0, 0),	// zh
		89 => (43, 6, 0, 0),	// z|ax
		90 => (50, 0, 0, 0),	// content word boundary
		91 => (51, 0, 0, 0),	// comma
		92 => (52, 0, 0, 0),	// period
		93 => (53, 0, 0, 0),	// question mark
		94 => (54, 0, 0, 0),	// exclamation point
		95 => (55, 0, 0, 0),	// function word boundary
		_  => (0, 0, 0, 0),		// otherwise
	}
}
