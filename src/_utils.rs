// ring buffer
pub struct cbuf {
	size:   isize,			// buffer size
	buffer: [u8; 2048],		// buffer itself (2048 is hard coded here)
	head:   isize,			// producer
	tail:   isize,			// consumer
	prev:   u8				// previous character for trimming when writing text
}
impl cbuf {
	pub fn new() -> cbuf {
		cbuf {
			size:   2048,
			buffer: [0; 2048],
			head:   0,
			tail:   0,
			prev:   0,
		}
	}
	pub fn is_not_empty(&mut self) -> bool {
		if self.head == self.tail { return false; }
		true
	}
	fn enqueue(&mut self, x: u8) {
		let nxt = (self.head + 1) & (self.size - 1);
		if nxt == self.tail { panic!("cbuf:BUFFER OVERFLOW!!"); }
		self.buffer[self.head as usize] = x;
		self.head = nxt;
		//println!("enqueue: t: {}, h: {}", self.tail, self.head);
	}
	fn dequeue(&mut self) -> u8 {
		let mut output: u8 = 0x20;
		if self.is_not_empty() {
			output = self.buffer[self.tail as usize];
		}
		//println!("dequeue: t: {}, h: {}", self.tail, self.head);
		output
	}
	pub fn erase(&mut self) {
		// THIS CAN BE DANGEROUS WHEN USED INCORRECTLY
		// only used for certain cases such as allophonic recoding.
		//
		// this erases the last written token by
		// simply backing up the producer (head) pointer
		if self.is_not_empty() {
			self.head = (self.head - 1) & (self.size - 1);
		}
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
	pub fn read(&mut self) -> u8 { self.dequeue() }
	pub fn write(&mut self, x: u8) {
		self.enqueue(x);
		self.prev = x;
	}
	pub fn write_text(&mut self, txt: &str) {
		for x in txt.bytes() {
			if self.prev == 0x20 && x == 0x20 {
				// pass
			} else {
				self.write(x);
			}
		}
	}
	pub fn len(&mut self) -> usize {
		(self.head - self.tail) as usize
	}
}

// convert phoneme tokens and separate phoneme and stress
pub fn PHNIDToPhoneme(x: u8) -> (u8, u8, u8, u8) {
	let mut stress_a: u8 = 0;
	let mut stress_b: u8 = 0;
	let mut phone_a:  u8 = 0;
	let mut phone_b:  u8 = 0;
	match x {
		0 => {
			// "aa
			phone_a  = 1;
			stress_a = 2;
		},
		1 => {
			// "ae
			phone_a  = 2;
			stress_a = 2;
		},
		2 => {
			// "ah
			phone_a  = 3;
			stress_a = 2;
		},
		3 => {
			// "ao
			phone_a  = 4;
			stress_a = 2;
		},
		4 => {
			// "aw
			phone_a  = 5;
			stress_a = 2;
		},
		5 => {
			// "ay
			phone_a  = 8;
			stress_a = 2;
		},
		6 => {
			// "eh
			phone_a  = 13;
			stress_a = 2;
		},
		7 => {
			// "er
			phone_a  = 14;
			stress_a = 2;
		},
		8 => {
			// "ey
			phone_a  = 15;
			stress_a = 2;
		},
		9 => {
			// "ih
			phone_a  = 19;
			stress_a = 2;
		},
		10 => {
			// "iy
			phone_a  = 21;
			stress_a = 2;
		},
		11 => {
			// "ow
			phone_a  = 29;
			stress_a = 2;
		},
		12 => {
			// "oy
			phone_a  = 30;
			stress_a = 2;
		},
		13 => {
			// "uh
			phone_a  = 38;
			stress_a = 2;
		},
		14 => {
			// "uw
			phone_a  = 39;
			stress_a = 2;
		},
		15 => {
			// 'aa
			phone_a  = 1;
			stress_a = 1;
		},
		16 => {
			// 'ae
			phone_a  = 2;
			stress_a = 1;
		},
		17 => {
			// 'ah
			phone_a  = 3;
			stress_a = 1;
		},
		18 => {
			// 'ao
			phone_a  = 4;
			stress_a = 1;
		},
		19 => {
			// 'aw
			phone_a  = 5;
			stress_a = 1;
		},
		20 => {
			// 'ay
			phone_a  = 8;
			stress_a = 1;
		},
		21 => {
			// 'eh
			phone_a  = 13;
			stress_a = 1;
		},
		22 => {
			// 'eh|m
			phone_a  = 13;
			phone_b  = 26;
			stress_a = 1;
		},
		23 => {
			// 'er
			phone_a  = 14;
			stress_a = 1;
		},
		24 => {
			// 'ey
			phone_a  = 15;
			stress_a = 1;
		},
		25 => {
			// 'ih
			phone_a  = 19;
			stress_a = 1;
		},
		26 => {
			// 'iy
			phone_a  = 21;
			stress_a = 1;
		},
		27 => {
			// 'ow
			phone_a  = 29;
			stress_a = 1;
		},
		28 => {
			// 'oy
			phone_a  = 30;
			stress_a = 1;
		},
		29 => {
			// 'uh
			phone_a  = 38;
			stress_a = 1;
		},
		30 => {
			// 'uw
			phone_a  = 39;
			stress_a = 1;
		},
		32 => {
			// aa
			phone_a  = 1;
		},
		33 => {
			// ae
			phone_a  = 2;
		},
		34 => {
			// ao
			phone_a  = 4;
		},
		35 => {
			// aw
			phone_a  = 5;
		},
		36 => {
			// ax
			phone_a  = 6;
		},
		37 => {
			// axr
			phone_a  = 7;
		},
		38 => {
			// ay
			phone_a  = 8;
		},
		39 => {
			// b
			phone_a  = 9;
		},
		40 => {
			// b|y
			phone_a  = 9;
			phone_b  = 42;
		},
		41 => {
			// ch
			phone_a  = 10;
		},
		42 => {
			// d
			phone_a  = 11;
		},
		43 => {
			// dh
			phone_a  = 12;
		},
		44 => {
			// eh
			phone_a  = 13;
		},
		45 => {
			// ey
			phone_a  = 15;
		},
		46 => {
			// f
			phone_a  = 16;
		},
		47 => {
			// f|y
			phone_a  = 16;
			phone_b  = 42;
		},
		48 => {
			// g
			phone_a  = 17;
		},
		49 => {
			// g|y
			phone_a  = 17;
			phone_b  = 42;
		},
		50 => {
			// g|z
			phone_a  = 17;
			phone_b  = 43;
		},
		51 => {
			// hh
			phone_a  = 18;
		},
		52 => {
			// hh|y
			phone_a  = 18;
			phone_b  = 42;
		},
		53 => {
			// ix
			phone_a  = 20;
		},
		54 => {
			// iy
			phone_a  = 21;
		},
		55 => {
			// jh
			phone_a  = 22;
		},
		56 => {
			// k
			phone_a  = 23;
		},
		57 => {
			// k|s
			phone_a  = 23;
			phone_b  = 34;
		},
		58 => {
			// k|y
			phone_a  = 23;
			phone_b  = 42;
		},
		59 => {
			// l
			phone_a  = 24;
		},
		60 => {
			// lx
			phone_a  = 25;
		},
		61 => {
			// m
			phone_a  = 26;
		},
		62 => {
			// m|ax
			phone_a  = 26;
			phone_b  = 6;
		},
		63 => {
			// m|y
			phone_a  = 26;
			phone_b  = 42;
		},
		64 => {
			// n
			phone_a  = 27;
		},
		65 => {
			// ng
			phone_a  = 28;
		},
		66 => {
			// ow
			phone_a  = 29;
		},
		67 => {
			// oy
			phone_a  = 30;
		},
		68 => {
			// p
			phone_a  = 31;
		},
		69 => {
			// p|y
			phone_a  = 31;
			phone_b  = 42;
		},
		70 => {
			// r
			phone_a  = 32;
		},
		71 => {
			// rx
			phone_a  = 33;
		},
		72 => {
			// s
			phone_a  = 34;
		},
		73 => {
			// sh
			phone_a  = 35;
		},
		74 => {
			// s|"iy
			phone_a  = 34;
			phone_b  = 21;
			stress_b = 2;
		},
		75 => {
			// s|'iy
			phone_a  = 34;
			phone_b  = 21;
			stress_b = 1;
		},
		76 => {
			// t
			phone_a  = 36;
		},
		77 => {
			// th
			phone_a  = 37;
		},
		78 => {
			// uh
			phone_a  = 38;
		},
		79 => {
			// uw
			phone_a  = 39;
		},
		80 => {
			// v
			phone_a  = 40;
		},
		81 => {
			// w
			phone_a  = 41;
		},
		82 => {
			// y
			phone_a  = 42;
		},
		83 => {
			// y|"uw
			phone_a  = 42;
			phone_b  = 39;
			stress_b = 2;
		},
		84 => {
			// y|'uw
			phone_a  = 42;
			phone_b  = 39;
			stress_b = 1;
		},
		85 => {
			// y|ax
			phone_a  = 42;
			phone_b  = 6;
		},
		86 => {
			// y|uw
			phone_a  = 42;
			phone_b  = 39;
		},
		87 => {
			// z
			phone_a  = 43;
		},
		88 => {
			// zh
			phone_a  = 44;
		},
		89 => {
			// z|ax
			phone_a  = 43;
			phone_b  = 6;
		},
		90 => {
			// content word boundary
			phone_a = 50;
		},
		91 => {
            // comma
			phone_a = 51;
		},
		92 => {
            // period
			phone_a = 52;
		},
		93 => {
            // question mark
			phone_a = 53;
		},
		94 => {
            // exclamation point
			phone_a = 54;
		},
		95 => {
			// function word boundary
			phone_a = 55;
		},
		_  => {
			// _
			stress_a = 0;
			stress_b = 0;
			phone_a  = 0;
			phone_b  = 0;
		},
	}
	(phone_a, phone_b, stress_a, stress_b)
}

