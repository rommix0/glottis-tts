/*

PHONE IDS

the phones the synthesizer plays, numbered as the formant table's rows,
and the boundary phones the front end puts between words and phrases

*/

pub const SIL: u8 = 0;			// the silent row, or no phone at all
pub const AA:  u8 = 1;
pub const AE:  u8 = 2;
pub const AH:  u8 = 3;
pub const AO:  u8 = 4;
pub const AW:  u8 = 5;
pub const AX:  u8 = 6;
pub const AXR: u8 = 7;
pub const AY:  u8 = 8;
pub const B:   u8 = 9;
pub const CH:  u8 = 10;
pub const D:   u8 = 11;
pub const DH:  u8 = 12;
pub const EH:  u8 = 13;
pub const ER:  u8 = 14;
pub const EY:  u8 = 15;
pub const F:   u8 = 16;
pub const G:   u8 = 17;
pub const HH:  u8 = 18;
pub const IH:  u8 = 19;
pub const IX:  u8 = 20;
pub const IY:  u8 = 21;
pub const JH:  u8 = 22;
pub const K:   u8 = 23;
pub const L:   u8 = 24;
pub const LX:  u8 = 25;			// dark l
pub const M:   u8 = 26;
pub const N:   u8 = 27;
pub const NG:  u8 = 28;
pub const OW:  u8 = 29;
pub const OY:  u8 = 30;
pub const P:   u8 = 31;
pub const R:   u8 = 32;
pub const RX:  u8 = 33;			// r after a vowel
pub const S:   u8 = 34;
pub const SH:  u8 = 35;
pub const T:   u8 = 36;
pub const TH:  u8 = 37;
pub const UH:  u8 = 38;
pub const UW:  u8 = 39;
pub const V:   u8 = 40;
pub const W:   u8 = 41;
pub const Y:   u8 = 42;
pub const Z:   u8 = 43;
pub const ZH:  u8 = 44;
pub const DX:  u8 = 47;			// alveolar flap
pub const AXP: u8 = 48;			// phrase-final release
pub const IXP: u8 = 49;			// phrase-final release after an alveolar

// boundaries
pub const CW:     u8 = 50;		// content word
pub const COMMA:  u8 = 51;
pub const PERIOD: u8 = 52;
pub const QMARK:  u8 = 53;
pub const BANG:   u8 = 54;
pub const FW:     u8 = 55;		// function word
pub const PAU:    u8 = 56;		// silent pause

// the same boundaries as tokens from the dictionary and letter-to-sound
pub const TOKEN_CW:     u8 = 90;
pub const TOKEN_COMMA:  u8 = 91;
pub const TOKEN_PERIOD: u8 = 92;
pub const TOKEN_QMARK:  u8 = 93;
pub const TOKEN_BANG:   u8 = 94;
pub const TOKEN_FW:     u8 = 95;
