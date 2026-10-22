/*---------------------------------------
| KLATT-STYLE FORMANT SYNTHESIZER       |
|                                       |
| AUTHOR: ANTHONY C. BARTMAN (@ROMMIX0) |
| DATE:   8/11/2026-8/30/2026           |
---------------------------------------*/

use core::f32::consts::PI;

use crate::_vtm::vtm;
use crate::_vtm::m_sr;
use crate::_vtm::FN_INIT;
use crate::_vtbl::vot_tbl;
use crate::_vtbl::formant_tbl;

const one_ms:    i32 = (m_sr as f32 * 0.001) as i32;
const two_ms:    i32 = (m_sr as f32 * 0.002) as i32;
const five_ms:   i32 = (m_sr as f32 * 0.005) as i32;
const ten_ms:    i32 = (m_sr as f32 * 0.010) as i32;
const twenty_ms: i32 = (m_sr as f32 * 0.020) as i32;
const thirty_ms: i32 = (m_sr as f32 * 0.030) as i32;
const forty_ms:  i32 = (m_sr as f32 * 0.040) as i32;
const fifty_ms:  i32 = (m_sr as f32 * 0.050) as i32;
const sixty_ms:  i32 = (m_sr as f32 * 0.060) as i32;
const eighty_ms: i32 = (m_sr as f32 * 0.080) as i32;

// shortest plosive closure before the burst
const min_closure: i32 = thirty_ms;

pub struct engine {
	vtm_eng: vtm,
	marker_A: i32,
	marker_B: i32,
	marker_BA: i32,
	is_first: bool,
	counter: i32,
	phn_l1: u8,
	phn_c:  u8,
	phn_r1: u8,
	pft_l1: u16,
	pft_c:  u16,
	pft_r1: u16,
	pdr_c:  i32,
	pdr_r1: i32,
	pst_l1: bool,
	pub pit_c: f32,
	pit_r1: f32,
	pub pst_c: bool,
	pst_r1: bool,
	GL: f32,
	GL_V: f32,
	RD_reg: f32,
	NZP_reg: f32,
	F1_reg: f32,
	F2_reg: f32,
	F3_reg: f32,
	B1_reg: f32,
	B2_reg: f32,
	B3_reg: f32,
	A2_reg: f32,
	A3_reg: f32,
	A4_reg: f32,
	A5_reg: f32,
	A6_reg: f32,
	A7_reg: f32,
	A8_reg: f32,
	A9_reg: f32,
	A10_reg: f32,
	AB_reg: f32,
	AF_reg: f32,
	AH_reg: f32,
	AV_reg: f32,
	RD_reg_src: f32,
	NZP_reg_src: f32,
	F1_reg_src: f32,
	F2_reg_src: f32,
	F3_reg_src: f32,
	B1_reg_src: f32,
	B2_reg_src: f32,
	B3_reg_src: f32,
	A2_reg_src: f32,
	A3_reg_src: f32,
	A4_reg_src: f32,
	A5_reg_src: f32,
	A6_reg_src: f32,
	A7_reg_src: f32,
	A8_reg_src: f32,
	A9_reg_src: f32,
	A10_reg_src: f32,
	AB_reg_src: f32,
	AF_reg_src: f32,
	AH_reg_src: f32,
	AV_reg_src: f32,
	RD_reg_trg: f32,
	NZP_reg_trg: f32,
	F1_reg_trg: f32,
	F2_reg_trg: f32,
	F3_reg_trg: f32,
	B1_reg_trg: f32,
	B2_reg_trg: f32,
	B3_reg_trg: f32,
	A2_reg_trg: f32,
	A3_reg_trg: f32,
	A4_reg_trg: f32,
	A5_reg_trg: f32,
	A6_reg_trg: f32,
	A7_reg_trg: f32,
	A8_reg_trg: f32,
	A9_reg_trg: f32,
	A10_reg_trg: f32,
	AB_reg_trg: f32,
	AF_reg_trg: f32,
	AH_reg_trg: f32,
	AV_reg_trg: f32,
	end_early: bool,
	slope: f32,
	interp: f32,
	silent_burst: bool,
	master_fade: f32,
	closure: f32,
	burst_INIT: f32,
	burst_VOT: f32,
	burst_C: i32,
	affric_swi: u8,
	cltime: f32,
	aftime: f32,
	optime: f32,
	brtime: f32,
	glot_time: f32,
	end_time: f32,
	sib_time: f32,
	sib_ramp: f32,
	C_A_src: u8,
	C_B_src: u8,
	RD_A_src: f32,
	RD_B_src: f32,
	NZP_A_src: f32,
	F1_A_src: f32,
	F2_A_src: f32,
	F3_A_src: f32,
	B1_A_src: f32,
	B2_A_src: f32,
	B3_A_src: f32,
	A2_src: f32,
	A3_src: f32,
	A4_src: f32,
	A5_src: f32,
	A6_src: f32,
	A7_src: f32,
	A8_src: f32,
	A9_src: f32,
	A10_src: f32,
	AB_src: f32,
	AF_src: f32,
	AH_src: f32,
	AV_src: f32,
	NZP_B_src: f32,
	F1_B_src: f32,
	F2_B_src: f32,
	F3_B_src: f32,
	B1_B_src: f32,
	B2_B_src: f32,
	B3_B_src: f32,
}
impl engine {
	pub fn new() -> engine {
		engine {
			/////////////////////////////
			//          VTM            //
			/////////////////////////////
			vtm_eng: vtm::new(),
			/////////////////////////////
			//          MAIN           //
			/////////////////////////////
			marker_A:  0,
			marker_B:  0,
			marker_BA: 0,
			is_first:  true,
			counter:   0,
			/////////////////////////////
			//     CONTEXT WINDOW      //
			/////////////////////////////
			phn_l1: 0,
			phn_c:  0,
			phn_r1: 0,
			pft_l1: 0,
			pft_c:  0,
			pft_r1: 0,
			pdr_c:  0,
			pdr_r1: 0,
			pst_l1: false,
			pst_c:  false,
			pst_r1: false,
			pit_c: 0.0,
			pit_r1: 0.0,
			/////////////////////////////
			//        REGISTERS        //
			/////////////////////////////
			// traversal registers
			GL:      1.0,				// glottalization F0 level
			GL_V:    1.0,				// glottalization AV level
			RD_reg:  0.0,
			NZP_reg: 0.0,
			F1_reg:  0.0,
			F2_reg:  0.0,
			F3_reg:  0.0,
			B1_reg:  0.0,
			B2_reg:  0.0,
			B3_reg:  0.0,
			A2_reg:  0.0,
			A3_reg:  0.0,
			A4_reg:  0.0,
			A5_reg:  0.0,
			A6_reg:  0.0,
			A7_reg:  0.0,
			A8_reg:  0.0,
			A9_reg:  0.0,
			A10_reg:  0.0,
			AB_reg:  0.0,
			AF_reg:  0.0,
			AH_reg:  0.0,
			AV_reg:  0.0,
			// source registers
			RD_reg_src:  0.0,
			NZP_reg_src: 0.0,
			F1_reg_src:  0.0,
			F2_reg_src:  0.0,
			F3_reg_src:  0.0,
			B1_reg_src:  0.0,
			B2_reg_src:  0.0,
			B3_reg_src:  0.0,
			A2_reg_src:  0.0,
			A3_reg_src:  0.0,
			A4_reg_src:  0.0,
			A5_reg_src:  0.0,
			A6_reg_src:  0.0,
			A7_reg_src:  0.0,
			A8_reg_src:  0.0,
			A9_reg_src:  0.0,
			A10_reg_src:  0.0,
			AB_reg_src:  0.0,
			AF_reg_src:  0.0,
			AH_reg_src:  0.0,
			AV_reg_src:  0.0,
			// target registers
			RD_reg_trg:  0.0,
			NZP_reg_trg: 0.0,
			F1_reg_trg:  0.0,
			F2_reg_trg:  0.0,
			F3_reg_trg:  0.0,
			B1_reg_trg:  0.0,
			B2_reg_trg:  0.0,
			B3_reg_trg:  0.0,
			A2_reg_trg:  0.0,
			A3_reg_trg:  0.0,
			A4_reg_trg:  0.0,
			A5_reg_trg:  0.0,
			A6_reg_trg:  0.0,
			A7_reg_trg:  0.0,
			A8_reg_trg:  0.0,
			A9_reg_trg:  0.0,
			A10_reg_trg:  0.0,
			AB_reg_trg:  0.0,
			AF_reg_trg:  0.0,
			AH_reg_trg:  0.0,
			AV_reg_trg:  0.0,
			/////////////////////////////
			//         SLOPE           //
			/////////////////////////////
			end_early:    false,
			slope:        0.0,
			interp:       0.0,
			silent_burst: false,
			master_fade:  1.0,							// master fader for phrase-final trigger
			closure:      1.0,							// multiplier for AF, AH, and AV
			burst_INIT:   1.0,							// burst initiator fader
			burst_VOT:    1.0,							// burst VOT fader
			burst_C:      0,
			affric_swi:   0,
			cltime:       1.0 / five_ms  as f32,		// time it takes to make a closure before a burst
			aftime:       1.0 / ten_ms   as f32,		// time it takes to fade in an affricate
			optime:       1.0 / two_ms   as f32,		// time it takes for burst to open (1 to 2 ms)
			brtime:       1.0 / five_ms  as f32,		// time it takes to fulfill a burst
			glot_time:    1.0 / ten_ms   as f32,		// glottalization timing
			end_time:     1.0 / fifty_ms as f32,		// phrase-final fade time
			sib_time:     1.0 / sixty_ms as f32,
			sib_ramp:     0.0,
			/////////////////////////////
			//         SOURCE          //
			/////////////////////////////
			C_A_src:   0,
			C_B_src:   0,
			RD_A_src:  0.0,
			RD_B_src:  0.0,
			NZP_A_src: 0.0,
			F1_A_src:  0.0,
			F2_A_src:  0.0,
			F3_A_src:  0.0,
			B1_A_src:  0.0,
			B2_A_src:  0.0,
			B3_A_src:  0.0,
			A2_src:    0.0,
			A3_src:    0.0,
			A4_src:    0.0,
			A5_src:    0.0,
			A6_src:    0.0,
			A7_src:    0.0,
			A8_src:    0.0,
			A9_src:    0.0,
			A10_src:   0.0,
			AB_src:    0.0,
			AF_src:    0.0,
			AH_src:    0.0,
			AV_src:    0.0,
			NZP_B_src: 0.0,
			F1_B_src:  0.0,
			F2_B_src:  0.0,
			F3_B_src:  0.0,
			B1_B_src:  0.0,
			B2_B_src:  0.0,
			B3_B_src:  0.0,
		}
	}
	pub fn init(&mut self) {
		// initialize VTM for first time usage
		self.vtm_eng.init();
	}
	pub fn reset(&mut self) {
		/////////////////////////////
		//          VTM            //
		/////////////////////////////
		self.vtm_eng.reset();
		/////////////////////////////
		//          MAIN           //
		/////////////////////////////
		self.marker_A =  0;
		self.marker_B =  0;
		self.marker_BA = 0;
		self.is_first =  true;
		self.counter =   0;
		/////////////////////////////
		//     CONTEXT WINDOW      //
		/////////////////////////////
		self.phn_l1 = 0;
		self.phn_c =  0;
		self.phn_r1 = 0;
		self.pft_l1 = 0;
		self.pft_c =  0;
		self.pft_r1 = 0;
		self.pdr_c =  0;
		self.pdr_r1 = 0;
		self.pst_l1 = false;
		self.pst_c =  false;
		self.pst_r1 = false;
		self.pit_c = 0.0;
		self.pit_r1 = 0.0;
		/////////////////////////////
		//        REGISTERS        //
		/////////////////////////////
		// traversal registers
		self.GL =      1.0;
		self.GL_V =    1.0;
		self.RD_reg =  0.0;
		self.NZP_reg = 0.0;
		self.F1_reg =  0.0;
		self.F2_reg =  0.0;
		self.F3_reg =  0.0;
		self.B1_reg =  0.0;
		self.B2_reg =  0.0;
		self.B3_reg =  0.0;
		self.A2_reg =  0.0;
		self.A3_reg =  0.0;
		self.A4_reg =  0.0;
		self.A5_reg =  0.0;
		self.A6_reg =  0.0;
		self.A7_reg =  0.0;
		self.A8_reg =  0.0;
		self.A9_reg =  0.0;
		self.A10_reg = 0.0;
		self.AB_reg =  0.0;
		self.AF_reg =  0.0;
		self.AH_reg =  0.0;
		self.AV_reg =  0.0;
		// source registers
		self.RD_reg_src =  0.0;
		self.NZP_reg_src = 0.0;
		self.F1_reg_src =  0.0;
		self.F2_reg_src =  0.0;
		self.F3_reg_src =  0.0;
		self.B1_reg_src =  0.0;
		self.B2_reg_src =  0.0;
		self.B3_reg_src =  0.0;
		self.A2_reg_src =  0.0;
		self.A3_reg_src =  0.0;
		self.A4_reg_src =  0.0;
		self.A5_reg_src =  0.0;
		self.A6_reg_src =  0.0;
		self.A7_reg_src =  0.0;
		self.A8_reg_src =  0.0;
		self.A9_reg_src =  0.0;
		self.A10_reg_src =  0.0;
		self.AB_reg_src =  0.0;
		self.AF_reg_src =  0.0;
		self.AH_reg_src =  0.0;
		self.AV_reg_src =  0.0;
		// target registers
		self.RD_reg_trg =  0.0;
		self.NZP_reg_trg = 0.0;
		self.F1_reg_trg =  0.0;
		self.F2_reg_trg =  0.0;
		self.F3_reg_trg =  0.0;
		self.B1_reg_trg =  0.0;
		self.B2_reg_trg =  0.0;
		self.B3_reg_trg =  0.0;
		self.A2_reg_trg =  0.0;
		self.A3_reg_trg =  0.0;
		self.A4_reg_trg =  0.0;
		self.A5_reg_trg =  0.0;
		self.A6_reg_trg =  0.0;
		self.A7_reg_trg =  0.0;
		self.A8_reg_trg =  0.0;
		self.A9_reg_trg =  0.0;
		self.A10_reg_trg =  0.0;
		self.AB_reg_trg =  0.0;
		self.AF_reg_trg =  0.0;
		self.AH_reg_trg =  0.0;
		self.AV_reg_trg =  0.0;
		/////////////////////////////
		//         SLOPE           //
		/////////////////////////////
		self.end_early =    false;
		self.slope =        0.0;
		self.interp =       0.0;
		self.silent_burst = false;
		self.master_fade =  1.0;
		self.closure =      1.0;
		self.burst_INIT =   1.0;
		self.burst_VOT =    1.0;
		self.burst_C =      0;
		self.affric_swi =   0;
		self.sib_ramp =     0.0;
		/////////////////////////////
		//         SOURCE          //
		/////////////////////////////
		self.C_A_src =   0;
		self.C_B_src =   0;
		self.RD_A_src =  0.0;
		self.RD_B_src =  0.0;
		self.NZP_A_src = 0.0;
		self.F1_A_src =  0.0;
		self.F2_A_src =  0.0;
		self.F3_A_src =  0.0;
		self.B1_A_src =  0.0;
		self.B2_A_src =  0.0;
		self.B3_A_src =  0.0;
		self.A2_src =    0.0;
		self.A3_src =    0.0;
		self.A4_src =    0.0;
		self.A5_src =    0.0;
		self.A6_src =    0.0;
		self.A7_src =    0.0;
		self.A8_src =    0.0;
		self.A9_src =    0.0;
		self.A10_src =    0.0;
		self.AB_src =    0.0;
		self.AF_src =    0.0;
		self.AH_src =    0.0;
		self.AV_src =    0.0;
		self.NZP_B_src = 0.0;
		self.F1_B_src =  0.0;
		self.F2_B_src =  0.0;
		self.F3_B_src =  0.0;
		self.B1_B_src =  0.0;
		self.B2_B_src =  0.0;
		self.B3_B_src =  0.0;
	}
	fn phone_to_feats(&mut self, x: u8) -> u16 {
		/*

		phnfeat bitmap (little endian, so 15 (MSB) to 0 (LSB) is the ordering):
			1  -> consonant/vowel
			2  -> unvoiced/voiced
			3  -> front
			4  -> back
			5  -> nasal
			6  -> alveolar
			7  -> stop
			8  -> bilabial
			9  -> velar
			10 -> sibilant
			11 -> lateral
			12 -> rhotic
			13 -> glide
			14 -> diphthong
			15 -> dental
			16 -> affricate

		*/
		match x {
			47                     => 0x0022,		// dx (alveolar flap)
			40                     => 0x0002,		// v
			1|2|3|4|5|6|7|14|29|38 => 0x0003,		// aa, ae, ah, ao, aw, ax, axr, er, ow
			13|15|19|20|21         => 0x0007,		// eh, ey, ih, ix, iy
			39                     => 0x000B,		// uh, uw
			27                     => 0x0032,		// n
			36                     => 0x0060,		// t
			11                     => 0x0062,		// d
			26                     => 0x0092,		// m
			31                     => 0x00C0,		// p
			9                      => 0x00C2,		// b
			28                     => 0x0112,		// ng
			23                     => 0x0140,		// k
			17                     => 0x0142,		// g
			34|35                  => 0x0220,		// s, sh
			43|44                  => 0x0222,		// z, zh
			24|25|45               => 0x0422,		// l, lx
			32|33|46               => 0x0822,		// r, rx
			42                     => 0x1002,		// y
			41                     => 0x1182,		// w
			8|30                   => 0x2003,		// ay, oy
			37                     => 0x4000,		// th
			12                     => 0x4002,		// dh
			10                     => 0x8260,		// ch
			22                     => 0x8262,		// jh
			_                      => 0x0000,		// otherwise
		}
	}
	fn ms_to_samples(&mut self, x: i32) -> i32 {
		// convert milliseconds to samples
		((x as f32 / 1000.0) * m_sr as f32) as i32
	}
	fn advance(&mut self, x: u8, d: i32, s: bool, cp: f32) {
		// x is phone_id
		// d is duration
		// s is stress
		/////////////////////////////
		//        PHONEME          //
		/////////////////////////////
		self.phn_l1 = self.phn_c;
		self.phn_c  = self.phn_r1;
		self.phn_r1 = x;
		/////////////////////////////
		//        FEATURES         //
		/////////////////////////////
		self.pft_l1 = self.pft_c;
		self.pft_c  = self.pft_r1;
		self.pft_r1 = self.phone_to_feats(x);
		/////////////////////////////
		//        DURATION         //
		/////////////////////////////
		self.pdr_c  = self.pdr_r1;
		self.pdr_r1 = self.ms_to_samples(d);
		/////////////////////////////
		//         STRESS          //
		/////////////////////////////
		self.pst_l1 = self.pst_c;
		self.pst_c  = self.pst_r1;
		self.pst_r1 = s;
		/////////////////////////////
		//    PITCH PASSTHROUGH    //
		/////////////////////////////
		self.pit_c = self.pit_r1;
		self.pit_r1 = cp;
	}
	pub fn is_running(&mut self) -> bool {
		if self.end_early || self.counter >= self.marker_B { return false; }
		true
	}
	pub fn is_ready(&mut self) -> bool {
		if self.phn_c != 0 { return true; }
		false
	}
	pub fn is_not_last(&mut self) -> bool {
		if self.phn_c != 0 && self.phn_r1 == 0 { return false; }
		true
	}
	fn fetch(&mut self) {
		let offset_src: usize = self.phn_c as usize * 31;
		//---------------source---------------------
		self.C_A_src   = formant_tbl[offset_src+0] as u8;		// also target during transition time
		self.C_B_src   = formant_tbl[offset_src+1] as u8;
		self.RD_A_src  = formant_tbl[offset_src+2] as f32 / 100.0;
		self.RD_B_src  = formant_tbl[offset_src+3] as f32 / 100.0;
		//self.NZP_A_src = formant_tbl[offset_src+4] as f32;
		self.NZP_A_src = self.fetch_nsl();
		self.F1_A_src  = formant_tbl[offset_src+5] as f32;
		self.F2_A_src  = formant_tbl[offset_src+6] as f32;
		self.F3_A_src  = formant_tbl[offset_src+7] as f32;
		self.B1_A_src  = formant_tbl[offset_src+8] as f32;
		self.B2_A_src  = formant_tbl[offset_src+9] as f32;
		self.B3_A_src  = formant_tbl[offset_src+10] as f32;
		self.A2_src    = formant_tbl[offset_src+11] as f32;
		self.A3_src    = formant_tbl[offset_src+12] as f32;
		self.A4_src    = formant_tbl[offset_src+13] as f32;
		self.A5_src    = formant_tbl[offset_src+14] as f32;
		self.A6_src    = formant_tbl[offset_src+15] as f32;
		self.A7_src    = formant_tbl[offset_src+16] as f32;
		self.A8_src    = formant_tbl[offset_src+17] as f32;
		self.A9_src    = formant_tbl[offset_src+18] as f32;
		self.A10_src   = formant_tbl[offset_src+19] as f32;
		self.AB_src    = formant_tbl[offset_src+20] as f32;
		self.AF_src    = formant_tbl[offset_src+21] as f32;
		self.AH_src    = formant_tbl[offset_src+22] as f32;
		self.AV_src    = formant_tbl[offset_src+23] as f32;
		//self.NZP_B_src = formant_tbl[offset_src+24] as f32;
		self.NZP_B_src = self.NZP_A_src;
		self.F1_B_src  = formant_tbl[offset_src+25] as f32;
		self.F2_B_src  = formant_tbl[offset_src+26] as f32;
		self.F3_B_src  = formant_tbl[offset_src+27] as f32;
		self.B1_B_src  = formant_tbl[offset_src+28] as f32;
		self.B2_B_src  = formant_tbl[offset_src+29] as f32;
		self.B3_B_src  = formant_tbl[offset_src+30] as f32;

		if self.F1_B_src == 0.0 { self.F1_B_src = self.F1_A_src; }
		if self.F2_B_src == 0.0 { self.F2_B_src = self.F2_A_src; }
		if self.F3_B_src == 0.0 { self.F3_B_src = self.F3_A_src; }
		if self.B1_B_src == 0.0 { self.B1_B_src = self.B1_A_src; }
		if self.B2_B_src == 0.0 { self.B2_B_src = self.B2_A_src; }
		if self.B3_B_src == 0.0 { self.B3_B_src = self.B3_A_src; }
	}
	fn fetch_vot(&mut self) -> (i32, i32) {
		// phoneme id to VOT selector
		let offset: usize = match self.phn_c {
			36 => 1,
			23 => 2,
			9  => 3,
			11 => 4,
			17 => 5,
			_  => 0,
		} * 2;

		// retrieve VOT data and return
		let b_onset = self.ms_to_samples(vot_tbl[offset+0]);
		let t_onset = self.ms_to_samples(vot_tbl[offset+1]);
		(b_onset, t_onset)
	}
	fn fetch_nsl(&mut self) -> f32 {
		match self.pft_c {
			0x0092        => 1000.0,	// M
			0x0032        => 1700.0,	// N
			0x0112        => 3000.0,	// NG
			_             => 0.0,		// otherwise
		}
	}
	fn ptar(&mut self) {
		if self.phn_c == 24 && self.pft_r1 & 0x42 == 0x40 {
			self.phn_c = 45;
		} else if self.phn_c == 32 && self.pft_r1 & 0x42 == 0x40 {
			self.phn_c = 46;
		}
	}
	fn ftar(&mut self) {
		let mut factor: f32 = 1.0;
		if self.pft_c & 0x1 == 1 {
			// STRESS COLORING FOR VOWEL
			factor = if self.pst_c { 0.5 } else { 1.5 };

			// VELAR PINCHING RULE
			if self.pft_l1 & 0x100 != 0 {
				self.F2_A_src += 80.0 * factor;
				self.F3_A_src -= 100.0 * factor;
			}
			if self.pft_r1 & 0x100 != 0 {
				self.F2_B_src += 80.0 * factor;
				self.F3_B_src -= 100.0 * factor;
			}

			// BILABIAL ROUNDING RULE
			if self.pft_l1 & 0x80 != 0 {
				self.F2_A_src -= 50.0 * factor;
			}
			if self.pft_r1 & 0x80 != 0 {
				self.F2_B_src -= 50.0 * factor;
			}

			// SONORANT ADJUSTMENT
			if self.pft_l1 & 0x1C00 != 0 {
				self.F3_A_src -= 150.0 * factor;
			}
			if self.pft_r1 & 0x1C00 != 0 {
				self.F3_B_src -= 150.0 * factor;
			}

			// NASALIZATION
			if self.pft_r1 & 0x10 != 0 {
				self.F2_A_src += 110.0 * factor;
				self.F2_B_src += 110.0 * factor;
			}

			// WIDEN BANDWIDTH
			if self.pft_r1 & 0x3 == 0x3 {
				self.B1_B_src *= 2.5 * factor;
			}
			if self.pft_l1 & 0x10 != 0 {
				self.B1_A_src += 70.0 * factor;
				self.B1_B_src += 70.0 * factor;
			}

			// LATERAL ADJUSTMENT
			if self.pft_r1 & 0x400 != 0 {
				if self.pft_c & 0x4 != 0 {
					self.F2_A_src -= 150.0 * factor;
					self.F2_B_src -= 150.0 * factor;
				} else if self.pft_c & 0x2000 != 0 {
					// diphthong next to lateral
					self.F2_B_src -= 250.0 * factor;
				}
			}

			// FRONT VOWEL ADJUSTMENT
			if self.pft_c & 0x4 != 0 {
				if self.pft_l1 == 0x1C10 {
					self.F2_A_src -= 150.0 * factor;
					self.F2_B_src -= 150.0 * factor;
				}
				if self.pft_l1 & 0x100 != 0 {
					self.F1_A_src += 70.0 * factor;
					self.F2_A_src += 50.0 * factor;
				}
			}

			// VOWEL RHOTICIZATION
			if self.pft_r1 & 0x800 != 0 {
				// rhoticize vowel
				self.F3_A_src -= 250.0 * factor;
				self.F3_B_src -= 250.0 * factor;
			}

			// WIDEN BANDWIDTH AFTER UNVOICED PLOSIVE
			if self.pft_l1 & 0x42 == 0x40 {
				self.B1_A_src += 125.0 * factor;
				self.B2_A_src += 50.0 * factor;
			}

			// BACK VOWEL ALVEOLARIZATION
			if self.pft_c == 0x000B {
				if self.pft_l1 & 0x20 != 0 {
					self.F2_A_src += 250.0 * factor;
				}
				if self.pft_r1 & 0x20 != 0 {
					self.F2_B_src += 250.0 * factor;
				}
			}

		} else if self.pft_c & 0x1 == 0 {
			// NASAL ADJUSTMENT AFTER FRONT VOWEL
			if self.pft_c & 0x10 != 0 && self.pft_l1 & 0x5 == 0x5 {
				self.F2_A_src -= 125.0;
				self.F2_B_src -= 125.0;
				self.F3_A_src -= 100.0;
				self.F3_B_src -= 100.0;
			}
			// VELAR FRONTING/LABIALIZATION RULE
			if self.pft_c & 0x300 != 0 && self.pft_r1 & 0x5 == 0x5 {
				self.F2_A_src += 150.0;
				self.F2_B_src += 150.0;
				self.F3_A_src += 175.0;
				self.F3_B_src += 175.0;
			}
			// NASAL BANDWIDTH ADJUSTMENT (CHEN 1997)
			if self.pft_c & 0x10 != 0 {
				self.B1_A_src += 107.0;
				self.B1_B_src += 107.0;
			}
		}

		// deal with fricative amp params below
		if self.pft_c & 0x8000 == 0 {
			if self.pft_c & 0x41 == 0 && self.pdr_c < eighty_ms {
				// short fricatives don't reach AF target (less than 80 ms), lower AF
				self.AF_src -= 3.0;
			}
			if self.pft_l1 & 0x1 == 1 && self.pft_c & 0x41 == 0 && self.pft_r1 & 0x1 == 1 {
				// fricatives are stronger between two vowels
				self.AF_src += 6.0;
			}
		}

		if self.pft_r1 & 0x4 != 0 {
			// lower AF if front vowel is next
			self.AF_src -= 3.0;
		}

		if self.pst_l1 == false && self.pst_r1 == false && self.pft_c & 0x1 == 0 {
			// unstressed consonant has lower AF
			self.AF_src -= 3.0;
		}

		if self.pft_c & 0x200 != 0 {
			// phoneme is a sibilant, lower AF_reg so we can ramp it up during synthesis
			self.sib_ramp = 0.95;
		}

		// deal whether plosive should be silent
		self.silent_burst = false;
		if self.pft_c & 0x8040 != 0 && self.pft_r1 & 0x50 != 0 {
			// silent burst if another burst is next or nasal
			self.silent_burst = true;
		}
	}
	fn get_control_A_duration(&mut self, x: u8, d: i32) -> i32 {
		(d as f32 * (x as f32 / 100.0)) as i32
	}
	fn get_control_B_duration(&mut self, x: u8, d: i32) -> i32 {
		let xx = 100 - x;
		self.get_control_A_duration(xx, d)
	}
	fn load_registers(&mut self) {
		// load for the first time
		// reset counter
		self.counter = 0;
		// source
		self.RD_reg_src  = self.RD_A_src;
		self.NZP_reg_src = self.NZP_A_src;
		self.F1_reg_src  = self.F1_A_src;
		self.F2_reg_src  = self.F2_A_src;
		self.F3_reg_src  = self.F3_A_src;
		self.B1_reg_src  = self.B1_A_src;
		self.B2_reg_src  = self.B2_A_src;
		self.B3_reg_src  = self.B3_A_src;
		self.A2_reg_src  = 0.0;
		self.A3_reg_src  = 0.0;
		self.A4_reg_src  = 0.0;
		self.A5_reg_src  = 0.0;
		self.A6_reg_src  = 0.0;
		self.A7_reg_src  = 0.0;
		self.A8_reg_src  = 0.0;
		self.A9_reg_src  = 0.0;
		self.A10_reg_src  = 0.0;
		self.AB_reg_src  = 0.0;
		self.AF_reg_src  = 0.0;
		self.AH_reg_src  = 0.0;
		self.AV_reg_src  = 0.0;
		if self.pft_c & 0x40 != 0 { self.closure = 0.0; }
		// target
		self.RD_reg_trg  = self.RD_reg_src;
		self.NZP_reg_trg = self.NZP_reg_src;
		self.F1_reg_trg  = self.F1_reg_src;
		self.F2_reg_trg  = self.F2_reg_src;
		self.F3_reg_trg  = self.F3_reg_src;
		self.B1_reg_trg  = self.B1_reg_src;
		self.B2_reg_trg  = self.B2_reg_src;
		self.B3_reg_trg  = self.B3_reg_src;
		self.A2_reg_trg  = self.A2_src;
		self.A3_reg_trg  = self.A3_src;
		self.A4_reg_trg  = self.A4_src;
		self.A5_reg_trg  = self.A5_src;
		self.A6_reg_trg  = self.A6_src;
		self.A7_reg_trg  = self.A7_src;
		self.A8_reg_trg  = self.A8_src;
		self.A9_reg_trg  = self.A9_src;
		self.A10_reg_trg = self.A10_src;
		self.AB_reg_trg  = self.AB_src;
		self.AF_reg_trg  = self.AF_src;
		self.AH_reg_trg  = self.AH_src;
		self.AV_reg_trg  = self.AV_src;
	}
	fn lin2smooth(&mut self, x: f32) -> f32 {
		// linear to smooth mapping
		// used by interpolator for smoothing
		x * x * (3.0 - 2.0 * x)
	}
	fn lin2decay(&mut self, x: f32) -> f32 {
		// map linear map to exponential decay curve
		// for use with burst rendering
		let mut xx = x;
		if xx > 1.0 { xx = 1.0; }
		if xx <= 0.0 { return 0.0; }
		(-2.0 * PI * (1.0 - xx)).exp()
	}
	fn update_registers(&mut self) {
		// x is interpolation factor (0 < x < 1)
		if self.interp < 1.0 {
			self.interp += self.slope;
			if self.interp > 1.0 { self.interp = 1.0; }
		}

		// assign self.interp to y. this will give us and opportunity
		// to use smooth mappings (such as linear to cosine)
		let y = self.lin2smooth(self.interp);

		// update the interpolator (except amps, those will be done elsewhere)
		self.RD_reg  = self.RD_reg_src  * (1.0 - y) + self.RD_reg_trg  * y;
		self.NZP_reg = self.NZP_reg_src * (1.0 - y) + self.NZP_reg_trg * y;
		self.F1_reg  = self.F1_reg_src  * (1.0 - y) + self.F1_reg_trg  * y;
		self.F2_reg  = self.F2_reg_src  * (1.0 - y) + self.F2_reg_trg  * y;
		self.F3_reg  = self.F3_reg_src  * (1.0 - y) + self.F3_reg_trg  * y;
		self.B1_reg  = self.B1_reg_src  * (1.0 - y) + self.B1_reg_trg  * y;
		self.B2_reg  = self.B2_reg_src  * (1.0 - y) + self.B2_reg_trg  * y;
		self.B3_reg  = self.B3_reg_src  * (1.0 - y) + self.B3_reg_trg  * y;
	}
	fn update_amp_registers(&mut self, c: i32, l: i32) {
		// interpolation smooth mapping algorithm
		let mut y: f32 = 0.0;
		if l > 0 {
			let mut rr: f32 = c as f32 / l as f32;
			if rr < 0.0 { rr = 0.0; }
			if rr > 1.0 { rr = 1.0; }
			y = self.lin2smooth(rr);
		}

		// ramp up sibilant fader
		if self.sib_ramp < 1.0 {
			self.sib_ramp += self.sib_time;
			if self.sib_ramp > 1.0 { self.sib_ramp = 1.0; }
		}

		// update the interpolator for the parallel amps
		self.A2_reg  = self.A2_reg_src  * (1.0 - y) + self.A2_reg_trg * y;
		self.A3_reg  = self.A3_reg_src  * (1.0 - y) + self.A3_reg_trg * y;
		self.A4_reg  = self.A4_reg_src  * (1.0 - y) + self.A4_reg_trg * y;
		self.A5_reg  = self.A5_reg_src  * (1.0 - y) + self.A5_reg_trg * y;
		self.A6_reg  = self.A6_reg_src  * (1.0 - y) + self.A6_reg_trg * y;
		self.A7_reg  = self.A7_reg_src  * (1.0 - y) + self.A7_reg_trg * y;
		self.A8_reg  = self.A8_reg_src  * (1.0 - y) + self.A8_reg_trg * y;
		self.A9_reg  = self.A9_reg_src  * (1.0 - y) + self.A9_reg_trg * y;
		self.A10_reg = self.A10_reg_src * (1.0 - y) + self.A10_reg_trg * y;

		// update the interpolator for main amps
		self.AB_reg = self.AB_reg_src * (1.0 - y) + self.AB_reg_trg * y;
		self.AF_reg = (self.AF_reg_src * (1.0 - y) + self.AF_reg_trg  * y) * self.master_fade * self.closure * self.lin2decay(self.burst_INIT) * self.sib_ramp;
		self.AH_reg = (self.AH_reg_src * (1.0 - y) + self.AH_reg_trg  * y) * self.master_fade * self.closure * self.lin2decay(self.burst_VOT);
		self.AV_reg = (self.AV_reg_src * (1.0 - y) + self.AV_reg_trg  * y) * self.master_fade * self.closure;
	}
	fn update_A_to_B(&mut self) {
		let mut dist = self.marker_B - self.marker_A;
		if dist < thirty_ms { dist = thirty_ms; }
		self.slope = 1.0 / dist as f32;
		self.interp = 0.0;
		// source (A)
		self.RD_reg_src  = self.RD_reg;
		self.NZP_reg_src = self.NZP_reg;
		self.F1_reg_src  = self.F1_reg;
		self.F2_reg_src  = self.F2_reg;
		self.F3_reg_src  = self.F3_reg;
		self.B1_reg_src  = self.B1_reg;
		self.B2_reg_src  = self.B2_reg;
		self.B3_reg_src  = self.B3_reg;
		// target (B)
		self.RD_reg_trg  = self.RD_B_src;
		self.NZP_reg_trg = self.NZP_B_src;
		self.F1_reg_trg  = self.F1_B_src;
		self.F2_reg_trg  = self.F2_B_src;
		self.F3_reg_trg  = self.F3_B_src;
		self.B1_reg_trg  = self.B1_B_src;
		self.B2_reg_trg  = self.B2_B_src;
		self.B3_reg_trg  = self.B3_B_src;
	}
	fn update_B_to_Next(&mut self) {
		// calculate transition slope here
		let mut dist = self.marker_A + self.marker_BA;
		if dist < thirty_ms { dist = thirty_ms; }
		self.slope = 1.0 / dist as f32;
		self.interp = 0.0;

		// set negative count so locus transition can be completed
		// before we preceed to the next phoneme
		self.counter = -(self.marker_BA);

		// source (A)
		self.RD_reg_src  = self.RD_reg;
		self.NZP_reg_src = self.NZP_reg;
		self.F1_reg_src  = self.F1_reg;
		self.F2_reg_src  = self.F2_reg;
		self.F3_reg_src  = self.F3_reg;
		self.B1_reg_src  = self.B1_reg;
		self.B2_reg_src  = self.B2_reg;
		self.B3_reg_src  = self.B3_reg;
		self.A2_reg_src  = self.A2_reg;
		self.A3_reg_src  = self.A3_reg;
		self.A4_reg_src  = self.A4_reg;
		self.A5_reg_src  = self.A5_reg;
		self.A6_reg_src  = self.A6_reg;
		self.A7_reg_src  = self.A7_reg;
		self.A8_reg_src  = self.A8_reg;
		self.A9_reg_src  = self.A9_reg;
		self.A10_reg_src = self.A10_reg;
		self.AB_reg_src  = self.AB_reg;
		self.AF_reg_src  = self.AF_reg;
		self.AH_reg_src  = self.AH_reg;
		self.AV_reg_src  = self.AV_reg;
		// target (B)
		self.RD_reg_trg  = self.RD_A_src;
		self.NZP_reg_trg = self.NZP_A_src;
		self.F1_reg_trg  = self.F1_A_src;
		self.F2_reg_trg  = self.F2_A_src;
		self.F3_reg_trg  = self.F3_A_src;
		self.B1_reg_trg  = self.B1_A_src;
		self.B2_reg_trg  = self.B2_A_src;
		self.B3_reg_trg  = self.B3_A_src;
		self.A2_reg_trg  = self.A2_src;
		self.A3_reg_trg  = self.A3_src;
		self.A4_reg_trg  = self.A4_src;
		self.A5_reg_trg  = self.A5_src;
		self.A6_reg_trg  = self.A6_src;
		self.A7_reg_trg  = self.A7_src;
		self.A8_reg_trg  = self.A8_src;
		self.A9_reg_trg  = self.A9_src;
		self.A10_reg_trg = self.A10_src;
		self.AB_reg_trg  = self.AB_src;
		self.AF_reg_trg  = self.AF_src;
		self.AH_reg_trg  = self.AH_src;
		self.AV_reg_trg  = self.AV_src;
	}
	pub fn load(&mut self, x: u8, d: i32, s: bool, cp: f32) -> bool {
		// x is phoneme
		// d is duration (ms)
		// s is stress
		// cp is CART pitch passthrough

		// reset end early flag
		self.end_early = false;
		
		// init vot variables here
		let mut b_onset: i32 = 0;
		let mut t_onset: i32 = 0;

		self.advance(x, d, s, cp);
		if self.is_ready() {
			// do ptar before anything else
			self.ptar();
			
			// phoneme is ready and loaded up
			// load in formant data
			self.fetch();

			// do formant target re-adjustments before doing anything else
			self.ftar();

			// get duration markers so slopes can be calculated
			if self.pft_c & 0x8040 == 0 {
				// not plosive nor affricate

				// adjust phoneme duration and make sure they don't dip below a minimum threshold.
				//if self.pdr_c < thirty_ms: self.pdr_c = thirty_ms

				//==============================================
				// typical transition times for certain sounds:
				//
				//   + stop  -> 30 ms (~20-40ms)
				//   + fric  -> 40 ms (~30-50ms)
				//   + sonor -> 50 ms (~40-60ms) [default]
				//==============================================
				if self.pft_c & 0x1 == 0 {
					// fricative transition
					if self.pft_r1 & 0x1C10 != 0 {
						// shorten if fricative follows sonorant (-20ms)
						self.marker_A = twenty_ms;
					} else if self.pft_c & 0x200 != 0 {
						// length sibilant a bit
						self.marker_A = sixty_ms;
					} else {
						self.marker_A = forty_ms;
					}
				} else if self.pft_c & 0x1C11 != 0 {
					// vowel/sonorant transition
					if self.pft_c & 0x10 != 0 {
						// if phoneme is a nasal, set marker_A to 0
						self.marker_A = 0;
					} else {
						// otherwise
						self.marker_A = fifty_ms;
					}
				} else {
					// otherwise, get transition from formant table
					self.marker_A = self.get_control_A_duration(self.C_A_src, self.pdr_c);
				}
				//self.marker_A = self.get_control_A_duration(self.C_A_src, self.pdr_c);
				self.marker_B = self.get_control_B_duration(self.C_B_src, self.pdr_c);
			} else if self.pft_c & 0x8000 != 0 {
				// is affricate
				// set default markers for affricate
				self.marker_A = (self.pdr_c as f32 * 0.40) as i32;
				self.marker_B = (self.pdr_c as f32 * 0.80) as i32;
			/*} else if self.phn_c == 47 {
				// deal with alveolar flap here
				self.marker_A = one_ms;
				self.pdr_c = self.ms_to_samples(50);*/
			} else {
				// is plosive
				// get data from VOT table
				(b_onset, t_onset) = self.fetch_vot();
				// lengthen the plosive if it's too short to fit a closure before the burst
				if self.pdr_c < b_onset + min_closure { self.pdr_c = b_onset + min_closure; }
				self.marker_A = self.pdr_c - b_onset;
				self.marker_B = self.pdr_c - t_onset;
			}
			if self.is_first {
				// load in registers if this is the first time
				// and reset counter
				self.marker_BA = self.get_control_A_duration(self.C_B_src, self.pdr_c);
				self.is_first = false;
				self.load_registers();
			} else {
				// do locus calculation here and set counter to negative number
				// so we can do a complete locus transition
				self.update_B_to_Next();
				self.marker_BA = self.get_control_A_duration(self.C_B_src, self.pdr_c);
			}
		}
		self.is_ready()
	}
	fn step_plosive(&mut self) {
		// make plosive rendering easier by having the code here
		if self.counter == 0 {
			if self.pft_c & 0x8040 != 0 {
				// if plosive or affricate, reset variables
				self.burst_C    = 0;
				self.burst_INIT = 1.0;
				self.burst_VOT  = 0.0;
				if self.affric_swi != 0 { self.affric_swi = 0; }
				if self.pft_c & 0x2 != 0 {
					// voiced stops
					self.closure = 0.8;
				} else {
					// otherwise
					self.closure = 0.0;
				}
			} else {
				// if not a plosive, move the faders back to default positions
				self.closure    = 1.0;
				self.burst_INIT = 1.0;
				self.burst_VOT  = 1.0;
			}
		} else if self.counter == self.marker_A {
			// bring self.closure back to 1.0 if less than
			self.closure = 1.0;
		} else if self.pft_c & 0x8040 != 0 && self.counter > self.marker_A {
			if self.silent_burst == false {
				// render unvoiced burst/affricate
				//
				// + start with burst onset first
				//
				// + then fade into burst VOT for aspiration
				//   - this can be turned off if say a sibilant was before
				//     an unvoiced stop. trigger the end_early flag if so.
				//
				if self.pft_c & 0x2 == 0 {
					self.burst_C += 1;
					if self.pft_l1 & 0x200 != 0 || self.pst_l1 || self.pst_r1 {
						// if sibilant is before, or burst is surrounded by unstressed vowels
						// reduce the aspiration, and end the burst early.
						if self.burst_C > ten_ms {
							self.marker_BA = self.counter;
							self.end_early = true;
						}
					}
					if self.affric_swi == 0 {
						self.burst_INIT -= self.brtime;
						if self.pft_c & 0x8000 != 0 && self.burst_INIT < 0.75 {
							self.affric_swi = 1;
						} else if self.burst_INIT < 0.0 {
							self.burst_INIT = 0.0;
						}
					} else if self.affric_swi == 1 {
						self.burst_INIT += self.brtime;
						if self.burst_INIT > 1.0 { self.burst_INIT = 1.0; }
					}
					if self.pft_c & 0x8000 == 0 {
						self.burst_VOT += self.brtime;
						if self.burst_VOT > 1.0 { self.burst_VOT = 1.0; }
					}
				} else {
					if self.burst_INIT < 1.0  { self.burst_INIT = 1.0; }
					if self.burst_VOT < 1.0   { self.burst_VOT  = 1.0; }
				}
			} else {
				// silence burst if another burst or nasal is next
				if self.burst_INIT > 0.0 { self.burst_INIT = 0.0; }
				if self.burst_VOT  > 0.0 { self.burst_VOT  = 0.0; }
			}
		}
	}
	fn step(&mut self) {
		self.counter += 1;
		self.update_amp_registers(self.counter, self.marker_A);
		self.step_plosive();
		if self.counter == self.marker_A {
			
			// update registers for diphthongnization (all phonemes)
			self.update_A_to_B();
			
		} else if self.counter >= 0 && self.counter < self.marker_A {
			
			// update glottalizer when switching to unvoiced phoneme (for AV and F0)
			if self.pft_c & 0x2 == 0 && self.GL > 0.75 {
				self.GL   -= self.glot_time;
				self.GL_V -= self.glot_time;
			}
				
		} else if self.counter > self.marker_A && self.counter < self.marker_B {
			
			// return glottalizer to 1.0 if less than when switching back to voiced phoneme
			if self.pft_c & 0x2 != 0 && self.GL < 1.0 {
				self.GL   += self.glot_time;
				self.GL_V += self.glot_time;
				if self.GL   > 1.0 { self.GL   = 1.0; }
				if self.GL_V > 1.0 { self.GL_V = 1.0; }
			}

			// do phrase-final fade out if AXP or IXP is the phoneme
			if self.phn_c == 48 || self.phn_c == 49 {
				if self.master_fade > 0.0 {
					self.master_fade -= self.end_time;
					if self.master_fade < 0.0 { self.master_fade = 0.0; }
				}
			} else if self.master_fade < 1.0 {
				self.master_fade += self.optime;
				if self.master_fade > 1.0 { self.master_fade = 1.0; }
			}
			
		}
		// update the interpolator
		self.update_registers()
	}
	/*pub fn float_to_16(x: f32) -> (u8, u8) {
		// converts 32 bit float audio to 16-bit (as little endian u8 pair)
		// stored here as a fallback if we need to grab 16-bit audio
		let xx: i16 = (32767.0 * (x).clamp(-1.0, 1.0)) as i16;
		let a: u8 = (xx & 0xFF) as u8;
		let b: u8 = ((xx >> 8) & 0xFF) as u8;
		(a, b)
	}*/
	pub fn sample(&mut self, f0: f32) -> f32 {
		self.step();
		self.vtm_eng.F0  = f0 * self.GL;
		self.vtm_eng.F1  = self.F1_reg;
		self.vtm_eng.F2  = self.F2_reg;
		self.vtm_eng.F3  = self.F3_reg;
		self.vtm_eng.B1  = self.B1_reg;
		self.vtm_eng.B2  = self.B2_reg;
		self.vtm_eng.B3  = self.B3_reg;
		self.vtm_eng.RD  = self.RD_reg;
		self.vtm_eng.FNP = self.NZP_reg;
		self.vtm_eng.A2  = self.A2_reg;
		self.vtm_eng.A3  = self.A3_reg;
		self.vtm_eng.A4  = self.A4_reg;
		self.vtm_eng.A5  = self.A5_reg;
		self.vtm_eng.A6  = self.A6_reg;
		self.vtm_eng.A7  = self.A7_reg;
		self.vtm_eng.A8  = self.A8_reg;
		self.vtm_eng.A9  = self.A9_reg;
		self.vtm_eng.A10 = self.A10_reg;
		self.vtm_eng.AB  = self.AB_reg;
		self.vtm_eng.AF  = self.AF_reg;
		self.vtm_eng.AH  = self.AH_reg;
		self.vtm_eng.AV  = self.AV_reg * self.GL_V;
		self.vtm_eng.step()
	}
}
