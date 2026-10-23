// letter-to-sound: a decision tree picks each letter's phone class from the 4 letters either side and the classes
// given to the 2 letters before; the word then gets exactly one primary stress.
// The tree's tables are in _lts_tree.rs, built by tools/build_lexicon.py, whose decode() is this file in Python.
use crate::_lts_tree::*;

const WINDOW: usize = 4;

// 0 outside the word, 1 apostrophe, 2.. for a to z
pub fn letter_code(x: u8) -> u8 {
	match x {
		b'\'' => 1,
		b'a'..=b'z' => x - b'a' + 2,
		b'A'..=b'Z' => x - b'A' + 2,
		_ => 0,
	}
}

fn leaf(f: &[u8; 2 * WINDOW + 3]) -> usize {
	let mut i = 0;
	while FEATURE[i] != 255 {
		i = if f[FEATURE[i] as usize] <= THRESHOLD[i] { LEFT[i] } else { RIGHT[i] } as usize;
	}
	i
}

// phone classes for a word given as letter codes: silent letters dropped, a class repeated on the next letter
// played once
pub fn word(codes: &[u8]) -> Vec<u8> {
	let n = codes.len();
	let (mut cls, mut prim, mut best) = (Vec::with_capacity(n), Vec::with_capacity(n), Vec::with_capacity(n));
	let mut hist = [START, START];
	for x in 0..n {
		let mut f = [0u8; 2 * WINDOW + 3];
		for k in 0..2 * WINDOW + 1 {
			let y = x as isize + k as isize - WINDOW as isize;
			if y >= 0 && (y as usize) < n { f[k] = codes[y as usize]; }
		}
		f[2 * WINDOW + 1] = hist[0];
		f[2 * WINDOW + 2] = hist[1];
		let i = leaf(&f);
		cls.push(CLASS[i]);
		prim.push(PRIMARY_P[i]);
		best.push(BEST_PRIMARY[i]);
		hist = [hist[1], CLASS[i]];
	}

	// exactly one primary stress: extra ones become secondary, or the likeliest letter takes it
	if n > 0 && cls.iter().filter(|&&c| PRIMARY[c as usize]).count() != 1 {
		let mut k = 0;
		for j in 1..n { if prim[j] > prim[k] { k = j; } }
		for j in 0..n {
			if j != k && PRIMARY[cls[j] as usize] { cls[j] = DEMOTE[cls[j] as usize]; }
		}
		if !PRIMARY[cls[k] as usize] { cls[k] = best[k]; }
	}

	let mut out: Vec<u8> = Vec::with_capacity(n);
	for c in cls {
		if c != SILENT && out.last() != Some(&c) { out.push(c); }
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;

	fn codes(w: &str) -> Vec<u8> { w.bytes().map(letter_code).collect() }

	#[test]
	fn matches_the_trained_tree() {
		for &(w, want) in SAMPLES.iter() {
			assert_eq!(word(&codes(w)), want, "{}", w);
		}
	}

	#[test]
	fn a_word_gets_one_primary_stress() {
		for w in ["zorblat", "quixanthe", "flumberton", "blorptastic", "unfrimbled", "tagliatellerino", "xyzzy"] {
			let n = word(&codes(w)).iter().filter(|&&c| PRIMARY[c as usize]).count();
			assert_eq!(n, 1, "{} has {} primary stresses", w, n);
		}
	}
}
