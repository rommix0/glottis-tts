mod _utils;
mod _text;
mod _duration;
mod _pitch;
mod _lts;
mod _vtm;
mod _vtbl;
mod _synth;
mod _dict;
mod _dread;

use std::fs;
use std::io::Write;
use std::io;
use std::env;
use _vtm::m_sr;
use hound;

use crate::_utils::cbuf;
use crate::_utils::PHNIDToPhoneme;
use crate::_utils::getVowelCount;
use crate::_utils::isVowel;
use crate::_dread::dictreader;
use crate::_lts::lts;

fn main() -> Result<(), Box<dyn std::error::Error>> {

	// read in text from argument
	let mut input = String::new();
	if let Some(text_arg) = env::args().nth(1) {
		input = text_arg;
	} else {
		println!("please enter text!");
		return Ok(());
	}

	// setup wav file stuff here
	let spec = hound::WavSpec {
		channels: 1,
		sample_rate: m_sr as u32,
		bits_per_sample: 32,
		sample_format: hound::SampleFormat::Float,
	};
	let mut writer = hound::WavWriter::create("tts_demo.wav", spec).unwrap();

	// initialize ring buffers
	let mut buf  = cbuf::new();				// text stream buffer
	let mut wbuf = cbuf::new();				// word buffer
	let mut pbuf = cbuf::new();				// phoneme buffer

	// initialize dictionary reader
	let mut dict_reader = dictreader::new();

	// read in text stream into input ring buffer
	buf.write(0x20);
	buf.write_text(&input);
	buf.write(0x20);
	buf.write(0x00);

	// initialize engines
	let mut synth_engine = _synth::engine::new();
	let mut text_engine  = _text::engine::new();
	let mut lts_engine   = _lts::pvec::new();
	let mut dur_model    = _duration::model::new();
	let mut pit_model    = _pitch::model::new();
	synth_engine.init();

	// initialize variables
	let mut pa: u8                = 0;
	let mut pb: u8                = 0;
	let mut sa: u8                = 0;
	let mut sb: u8                = 0;
	let mut l4: u8                = 0;
	let mut l3: u8                = 0;
	let mut l2: u8                = 0;
	let mut l1: u8                = 0;
	let mut c:  u8                = 0;
	let mut r1: u8                = 0;
	let mut r2: u8                = 0;
	let mut r3: u8                = 0;
	let mut r4: u8                = 0;
	let mut pout: u8              = 0;
	let mut prev: u8              = 0;
	let mut cc: u8                = 0;
	let mut cin: u8               = 0;
	let mut vwl_count: u16        = 0;
	let mut yyreset: usize        = 0;
	let mut isFunctionWord:  bool = false;
	let mut release_flag: bool    = false;
	let mut past_phone: u8        = 0;
	let mut phone: u8             = 0;
	let mut stress: u8            = 0;
	let mut duration: i32         = 0;
	let mut is_stressed: bool     = false;
	let mut is_final: bool        = false;
	let mut finish_off: bool      = false;
	let mut ready: bool           = false;
	let mut F0: f32               = 125.0;

	// proceed with the text normalizer/LTS pipeline
	while buf.is_not_empty() {
		if text_engine.lex(&mut buf) { break; }
		while text_engine.is_not_empty() {
			cc = text_engine.read();
			if wbuf.is_not_empty() && cc == 0x20 {
				// process word when whitespace is
				// hit and word buffer is not empty
				dict_reader.reset();
				yyreset = wbuf.getcurs();
				while wbuf.is_not_empty() {
					// read word into dictionary reader
					cin = wbuf.read();
					wbuf.advance();
					if cin == 0x20 { continue; }
					dict_reader.readIn(cin);
				}

				if dict_reader.search() {
					//println!("dictionary mode!");
					// get phoneme stream for word
					// if found in the dictionary
					if dict_reader.isFunctionWord() {
						// function word boundary
						pbuf.write(95);
					} else {
						// content word boundary
						pbuf.write(90);
					}

					// retrieve the phoneme sequence from dictionary
					// until null character 0xFF is hit
					loop {
						pout = dict_reader.readOut();
						//print!("{} ", pout);
						if pout == 0xFF { break; }
						vwl_count += getVowelCount(pout);
						pbuf.write(pout);
					}
					//print!("\n");
				} else {
					// if no dictionary match, then
					// do letter-to-sound stuff here!
					//println!("LTS mode!");
					lts_engine.reset();
					wbuf.setcurs(yyreset);
					prev = 0;
					while lts_engine.is_not_last() {
						cin = wbuf.read();
						wbuf.advance();
						(l4, l3, l2, l1, c, r1, r2, r3, r4) = lts_engine.step(cin);
						if lts_engine.is_ready() {
							// NOTE FOR TERMINAL PUNCTUATIONS
							//
							// word boundary is erased first before
							// adding terminal punctuations to the stream
							if c == 28 {
								// exclamation point
								pbuf.write(94);
							} else if c == 29 {
								// comma
								pbuf.write(91);
							} else if c == 30 {
								// period
								pbuf.write(92);
							} else if c == 31 {
								// question mark
								pbuf.write(93);
							} else {
								// normal case
								pbuf.write(90);
								pout = lts(l4, l3, l2, l1, c, r1, r2, r3, r4);
								if pout != 31 && pout != prev {
									// write out phoneme when not null and not a run-on
									pbuf.write(pout);
									vwl_count += getVowelCount(pout);
									prev = pout;
								}
							}
						}
					}
				}
			} else {
				// write to word buffer if not ready to parse yet
				wbuf.write(cc);
			}
		}
	}
	pbuf.write(90);

	// proceed with the synthesis
	dur_model.reset();
	pit_model.reset();
	synth_engine.reset();
	let mut last_syl: bool = false;
	isFunctionWord = false;
	if vwl_count > 0 { vwl_count -= 1; }

	while dur_model.is_not_last() {
		pa = 0;
		pb = 0;
		sa = 0;
		sb = 0;
		if pbuf.is_not_empty() {
			(pa, pb, sa, sb) = PHNIDToPhoneme(pbuf.read());
			pbuf.advance();
		}
		loop {
			// decrement vowel counter for is_final syllable trigger
			if vwl_count > 0 && isVowel(pa) {
				vwl_count -= 1;
			} else if vwl_count == 0 {
				last_syl = true;
			}

			// feed phoneme, stress, and last_syl trigger into model
			dur_model.step(pa, sa, last_syl);

			// fetch result when ready
			if dur_model.is_ready() {
				(past_phone, phone, stress, duration, is_final) = dur_model.fetch();
				//if is_final { pit_model.set_final(0); }
				if phone != 50 && phone != 55 {
					// output phoneme, stress, and duration
					// when phoneme is not a word boundary

					// reset pitch model if past phone is terminal punctuation
					// as long as current phone is not terminal punctuation
					if phone != 52 || phone != 53 || phone != 54 {
						if past_phone == 52 || past_phone == 53 || past_phone == 54 {
							pit_model.reset();
						}
					}

					// convert phrase boundaries to release phoneme (AXP or IXP)
					// and change duration to match duration profile
					// close to that of DECTalk.
					release_flag = false;
					if phone == 51 {
						// comma (do an upstep)
						pit_model.upstep();
						duration = 150;
						release_flag = true;
					} else if phone == 52 || phone == 53 || phone == 54 {
						match phone {
							53 => { pit_model.set_final(2); },		// question mark
							54 => { pit_model.set_final(3); },		// exclamation point
							_  => { pit_model.set_final(0); },		// period
						}
						duration = 400;
						release_flag = true;
					}
					if release_flag {
						// check if past phoneme is alveolar stop (T, D)
						phone = match past_phone {
							11|36 => 49,			// is alveolar. use IXP release
							_     => 48,			// otherwise, use AXP release
						};
					}

					// output phone, stress, and duration for debugging
					//println!("{} {} {}", phone, stress, duration);

					// note. duration is shorter for function words
					// lets do that here
					if isFunctionWord { duration = (duration as f32 * 0.9) as i32; }

					// feed phoneme and duration into synthesizer
					// and write out to WAV file
					is_stressed = if stress != 0 { true } else { false };
					ready = synth_engine.load(phone, duration, is_stressed);
					if ready {
						while synth_engine.is_running() {
							pit_model.set(synth_engine.pst_c);
							F0 = pit_model.step();
							writer.write_sample(synth_engine.sample(F0)).unwrap();
						}
					}
				} else if phone == 50 {
					// content word - word boundary
					if isFunctionWord == false {
						// do compound stress if content word
						// is after a content word
						pit_model.set_compound(true);
					}
					isFunctionWord = false;
				} else if phone == 55 {
					// function word boundary
					pit_model.set_compound(false);
					if isFunctionWord == false {
						// do a downstep if content word
						// was before a function word
						pit_model.downstep();
					}
					isFunctionWord = true;
				}
			}
			// assign pb and sb to pa and sa to process those next
			pa = pb;
			sa = sb;
			pb = 0;
			sb = 0;
			if pa == 0 { break; }
		}
	}

	// if formant synth not done, let's finish up the synthesis
	finish_off = true;
	while synth_engine.is_not_last() {
		if finish_off {
			finish_off = false;
			ready = synth_engine.load(48, 100, false);
		} else {
			ready = synth_engine.load(0, 50, false);
		}
		if ready {
			while synth_engine.is_running() {
				F0 = pit_model.step();
				writer.write_sample(synth_engine.sample(F0)).unwrap();
			}
		}
	}

	// flush WAV file and write to it
	writer.finalize().unwrap();

	Ok(())
}
