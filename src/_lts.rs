// letter-to-sound: two decision trees pick each letter's phone class, left to right, from the 4 letters either
// side, whether those are vowels and the classes given to the 2 letters before; the second also sees the first
// tree's classes for the 2 letters after. The word then gets exactly one primary stress.
// The trees' tables are in _lts_tree.rs, built by tools/build_lexicon.py, whose decode() is this file in Python.
use crate::_lts_tree::*;
use alloc::vec::Vec;

const WINDOW: usize = 4;
const FEATURES: usize = 2 * (2 * WINDOW + 1) + 4;

// letters grouped by kind (vowels, y, sonorants, stops, fricatives), so one threshold splits off a group
const ORDER: &[u8] = b"'aeiouywhlrmnbpdtgkcqfvszxj";

// 0 outside the word, else 1 + the letter's place in ORDER
pub fn letter_code(x: u8) -> u8 {
	let x = x.to_ascii_lowercase();
	ORDER.iter().position(|&c| c == x).map_or(0, |i| i as u8 + 1)
}

fn is_vowel(code: u8) -> bool { (2..=6).contains(&code) }

fn leaf(root: usize, f: &[u8; FEATURES]) -> usize {
	// a node's left child is the node after it
	let mut i = root;
	while FEATURE[i] != 255 {
		i = if f[FEATURE[i] as usize] <= THRESHOLD[i] { i + 1 } else { RIGHT[i] as usize };
	}
	i
}

// the letters around x, the classes given before it, which of those letters are vowels and (second tree) the first
// tree's classes after it
fn features(codes: &[u8], x: usize, hist: [u8; 2], ahead: Option<&[u8]>) -> [u8; FEATURES] {
	let n = codes.len();
	let mut f = [0u8; FEATURES];
	for k in 0..2 * WINDOW + 1 {
		let y = x as isize + k as isize - WINDOW as isize;
		if y >= 0 && (y as usize) < n { f[k] = codes[y as usize]; }
		f[2 * WINDOW + 3 + k] = is_vowel(f[k]) as u8;
	}
	f[2 * WINDOW + 1] = hist[0];
	f[2 * WINDOW + 2] = hist[1];
	if let Some(a) = ahead {
		f[FEATURES - 2] = a.get(x + 1).copied().unwrap_or(START);
		f[FEATURES - 1] = a.get(x + 2).copied().unwrap_or(START);
	}
	f
}

// phone classes for a word given as letter codes: silent letters dropped,
// a class repeated on the next letter played once
pub fn word(codes: &[u8]) -> Vec<u8> {
	let n = codes.len();
	let mut ahead = Vec::with_capacity(n);
	let mut hist = [START, START];
	for x in 0..n {
		let c = CLASS[leaf(0, &features(codes, x, hist, None))];
		ahead.push(c);
		hist = [hist[1], c];
	}
	let (mut cls, mut prim, mut best) = (Vec::with_capacity(n), Vec::with_capacity(n), Vec::with_capacity(n));
	let mut hist = [START, START];
	for x in 0..n {
		let i = leaf(SECOND, &features(codes, x, hist, Some(&ahead)));
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
