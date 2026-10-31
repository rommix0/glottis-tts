// suffix matching: a word the dictionary lacks may be a known stem with an inflection or a common suffix; its
// phones are then the stem's and the suffix's. A stem is known when the dictionary has it, or when it is a CMUdict
// word the letter-to-sound tree says right (the KNOWN hashes). Mirrors compose() in tools/build_lexicon.py.
use crate::_known::*;
use crate::_lts;
use alloc::{format, string::{String, ToString}, vec, vec::Vec};

const MIN_STEM: usize = 3;

// how a suffix spelled the stem before it, and the suffix's phones
#[derive(Clone, Copy)]
enum Way { Same, AddE, AddY, IToY, Undouble, Sibilant }
#[derive(Clone, Copy)]
enum Ending { Phones(&'static [u8]), S, Ed }

const SUFFIXES: [(&str, &[Way], Ending); 11] = [
	("ness", &[Way::Same, Way::IToY], Ending::Phones(&[PH_N, PH_AX, PH_S])),
	("less", &[Way::Same, Way::IToY], Ending::Phones(&[PH_L, PH_AX, PH_S])),
	("ful",  &[Way::Same, Way::IToY], Ending::Phones(&[PH_F, PH_AX, PH_L])),
	("ing",  &[Way::Same, Way::AddE, Way::Undouble], Ending::Phones(&[PH_IX, PH_NG])),
	("ies",  &[Way::AddY], Ending::Phones(&[PH_Z])),
	("ied",  &[Way::AddY], Ending::Phones(&[PH_D])),
	("ed",   &[Way::Same, Way::AddE, Way::Undouble], Ending::Ed),
	("'s",   &[Way::Same], Ending::S),
	("ly",   &[Way::Same], Ending::Phones(&[PH_L, PH_IY])),
	("es",   &[Way::Sibilant], Ending::S),
	("s",    &[Way::Same], Ending::S),
];

fn fnv(w: &str) -> u32 {
	w.bytes().fold(0x811C9DC5u32, |h, b| (h ^ b as u32).wrapping_mul(0x01000193))
}

pub fn known(w: &str) -> bool {
	KNOWN.binary_search(&fnv(w)).is_ok()
}

fn stem(s: &str, way: Way) -> Option<String> {
	let b = s.as_bytes();
	match way {
		Way::Same => Some(s.to_string()),
		Way::AddE => Some(format!("{}e", s)),
		Way::AddY => Some(format!("{}y", s)),
		Way::IToY => s.strip_suffix('i').map(|t| format!("{}y", t)),
		Way::Undouble => {
			let n = b.len();
			if n > 1 && b[n - 1] == b[n - 2] && !b"aeiouy".contains(&b[n - 1]) { Some(s[..n - 1].to_string()) } else { None }
		},
		Way::Sibilant => if ["s", "x", "z", "ch", "sh"].iter().any(|e| s.ends_with(e)) { Some(s.to_string()) } else { None },
	}
}

fn ending(e: Ending, stem: &[u8]) -> Vec<u8> {
	let kind = LAST_KIND[*stem.last().unwrap() as usize];
	match e {
		Ending::Phones(p) => p.to_vec(),
		Ending::S => match kind { 4 | 5 => vec![PH_IX, PH_Z], 1 | 3 => vec![PH_S], _ => vec![PH_Z] },
		Ending::Ed => match kind { 1 | 2 => vec![PH_IX, PH_D], 3 | 4 => vec![PH_T], _ => vec![PH_D] },
	}
}

// a stem's phones: the dictionary's, or the tree's for a known word
fn lookup(w: &str, dict: &mut dyn FnMut(&str) -> Option<Vec<u8>>) -> Option<Vec<u8>> {
	if let Some(p) = dict(w) { if !p.is_empty() { return Some(p); } }
	if known(w) {
		let p = _lts::word(&w.bytes().map(_lts::letter_code).collect::<Vec<u8>>());
		if !p.is_empty() { return Some(p); }
	}
	None
}

// phone classes for a lowercase word built from a known stem and up to `depth` suffixes, or None
pub fn compose(w: &str, dict: &mut dyn FnMut(&str) -> Option<Vec<u8>>, depth: u32) -> Option<Vec<u8>> {
	let p = lookup(w, dict);
	if p.is_some() || depth == 0 { return p; }
	for &(suf, ways, e) in SUFFIXES.iter() {
		if w.len() >= suf.len() + MIN_STEM && w.ends_with(suf) {
			let s = &w[..w.len() - suf.len()];
			for &way in ways {
				if let Some(st) = stem(s, way) {
					if let Some(mut q) = compose(&st, dict, depth - 1) {
						let tail = ending(e, &q);
						q.extend(tail);
						return Some(q);
					}
				}
			}
		}
	}
	None
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::_dread::dictreader;

	fn dict_lookup(r: &mut dictreader, w: &str) -> Option<Vec<u8>> {
		r.reset();
		for b in w.bytes() { r.readIn(b); }
		if !r.search() { return None; }
		let mut out = Vec::new();
		loop {
			let p = r.readOut();
			if p == 0xFF { break; }
			out.push(p);
		}
		Some(out)
	}

	#[test]
	fn matches_the_builder() {
		let mut r = dictreader::new();
		for &(w, want) in SAMPLES.iter() {
			let got = compose(w, &mut |s| dict_lookup(&mut r, s), 2);
			assert_eq!(got.as_deref(), Some(want), "{}", w);
		}
	}

	#[test]
	fn suffix_follows_the_stems_last_sound() {
		// -s: cat s, dog z, kiss ix z; -ed: want ix d, kiss t, hug d
		assert_eq!(ending(Ending::S, &[PH_T]), vec![PH_S]);
		assert_eq!(ending(Ending::S, &[PH_D]), vec![PH_Z]);
		assert_eq!(ending(Ending::S, &[PH_S]), vec![PH_IX, PH_Z]);
		assert_eq!(ending(Ending::Ed, &[PH_T]), vec![PH_IX, PH_D]);
		assert_eq!(ending(Ending::Ed, &[PH_S]), vec![PH_T]);
		assert_eq!(ending(Ending::Ed, &[PH_N]), vec![PH_D]);
	}

	#[test]
	fn unknown_stem_is_left_to_the_tree() {
		let mut r = dictreader::new();
		assert!(compose("zorblats", &mut |s| dict_lookup(&mut r, s), 2).is_none());
	}
}
