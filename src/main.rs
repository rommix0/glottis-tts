// the command line around the synthesizer: reads the options, writes tts_demo.wav
use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use tts_synth::{_vtm, _vtm::m_sr, Out, Speaker, normalize, phone_token};

type Wav = hound::WavWriter<BufWriter<File>>;

// mono at the synth's rate, as 16-bit integers or 32-bit floats
fn wav_spec(pcm16: bool) -> hound::WavSpec {
	let (bits, format) = if pcm16 { (16, hound::SampleFormat::Int) } else { (32, hound::SampleFormat::Float) };
	hound::WavSpec { channels: 1, sample_rate: m_sr as u32, bits_per_sample: bits, sample_format: format }
}

// command line options
struct Opts {
	text:  String,
	rate:  f32,
	trace: bool,
	text_only: bool,	// print the normalized text instead of speaking
	seg_dump: Option<String>,	// file for the playing segment index of every sample
	pcm16: bool,	// write 16-bit integer samples instead of 32-bit float
	voice: usize,	// voice profile number
	phone: Option<u8>,	// token of a single phone to say instead of the text
	table: Option<String>,	// formant table file to use instead of the built-in one
}

fn parse_args() -> Result<Opts, String> {
	let mut opts = Opts { text: String::new(), rate: 1.0, trace: false, text_only: false, seg_dump: None, pcm16: false, voice: 0, phone: None, table: None };
	let mut have_text = false;
	let mut args = env::args().skip(1);
	while let Some(a) = args.next() {
		if a == "--trace" {
			opts.trace = true;
		} else if a == "--16bit" {
			opts.pcm16 = true;
		} else if a == "--text" {
			opts.text_only = true;
		} else if a == "--seg-dump" {
			opts.seg_dump = Some(args.next().ok_or("--seg-dump needs a file name")?);
		} else if a == "--table" {
			opts.table = Some(args.next().ok_or("--table needs a file name")?);
		} else if a == "--phone" {
			let p = args.next().and_then(|v| v.parse().ok()).unwrap_or(0);
			opts.phone = Some(phone_token(p).ok_or(format!("phone {} can't be said on its own", p))?);
		} else if a == "--voice" {
			let n = args.next().and_then(|v| v.parse().ok()).unwrap_or(usize::MAX);
			if n >= _vtm::VOICES.len() { return Err(format!("voice must be between 0 and {}", _vtm::VOICES.len() - 1)); }
			opts.voice = n;
		} else if a == "--rate" {
			let r: f32 = args.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
			if !(0.5..=2.0).contains(&r) { return Err("rate must be between 0.5 and 2.0".to_string()); }
			opts.rate = r;
		} else if !have_text {
			opts.text = a;
			have_text = true;
		}
	}
	if !have_text && opts.phone.is_none() { return Err("please enter text!".to_string()); }
	Ok(opts)
}

// the WAV file, the segment dump and the trace
struct WavOut {
	writer: Wav,
	dump:   Option<BufWriter<File>>,	// the playing segment index of every sample
}

impl Out for WavOut {
	fn sample(&mut self, x: f32, seg: usize) {
		if self.writer.spec().sample_format == hound::SampleFormat::Int {
			self.writer.write_sample((x.clamp(-1.0, 1.0) * 32767.0).round() as i16).unwrap();
		} else {
			self.writer.write_sample(x).unwrap();
		}
		if let Some(d) = self.dump.as_mut() { d.write_all(&(seg as u32).to_le_bytes()).unwrap(); }
	}
	fn trace(&mut self, line: &str) { eprintln!("{}", line); }
}

fn read_table(path: &str) -> Result<&'static [u16; 1550], String> {
	// the numbers of the formant table in order, anything else between them is skipped
	let text = std::fs::read_to_string(path).map_err(|e| format!("can't read {path}: {e}"))?;
	let vals = text.split(|c: char| !c.is_ascii_digit()).filter(|t| !t.is_empty())
		.map(|t| t.parse().map_err(|_| format!("{path}: {t} is out of range"))).collect::<Result<Vec<u16>, _>>()?;
	let tbl: [u16; 1550] = vals.try_into().map_err(|v: Vec<u16>| format!("{path} has {} numbers, expected 1550", v.len()))?;
	Ok(Box::leak(Box::new(tbl)))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {

	let opts = match parse_args() {
		Ok(o) => o,
		Err(e) => {
			println!("{}", e);
			std::process::exit(1);
		}
	};

	if opts.text_only {
		println!("{}", normalize(&opts.text));
		return Ok(());
	}

	let mut out = WavOut { writer: hound::WavWriter::create("tts_demo.wav", wav_spec(opts.pcm16))?, dump: None };
	if let Some(path) = &opts.seg_dump { out.dump = Some(BufWriter::new(File::create(path)?)); }

	let mut sp = Speaker::new();
	sp.rate = opts.rate;
	sp.trace = opts.trace;
	sp.synth.set_voice(_vtm::VOICES[opts.voice]);
	if let Some(path) = &opts.table { sp.synth.tbl = read_table(path)?; }

	if let Some(tk) = opts.phone {
		// the phone as a word of its own
		sp.sentence(&[90, tk, 92], &[String::new()], &mut out);
	} else {
		sp.speak(&opts.text, &mut out);
	}

	// if formant synth not done, let's finish up the synthesis
	sp.finish(&mut out);

	out.writer.finalize()?;

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use tts_synth::{_dur, _utils, _utt, _utt::{Utterance, Boundary}};

	// the samples and the playing segment of each
	struct Collect { x: Vec<f32>, segs: Vec<usize> }
	impl Out for Collect {
		fn sample(&mut self, x: f32, seg: usize) { self.x.push(x); self.segs.push(seg); }
	}

	// say one sentence of tokens into out
	fn say_tokens(tokens: &[u8], texts: &[&str], out: &mut dyn Out) {
		let texts: Vec<String> = texts.iter().map(|t| t.to_string()).collect();
		let mut sp = Speaker::new();
		sp.sentence(tokens, &texts, out);
		sp.finish(out);
	}

	// render one sentence of tokens into samples
	fn render(tokens: &[u8], texts: &[&str]) -> Collect {
		let mut out = Collect { x: Vec::new(), segs: Vec::new() };
		say_tokens(tokens, texts, &mut out);
		out
	}

	// render one sentence of tokens into a WAV file, 16-bit or float
	fn render_file(tokens: &[u8], texts: &[&str], pcm16: bool, tag: &str) -> std::path::PathBuf {
		let path = env::temp_dir().join(format!("tts_synth_test_{}_{}.wav", std::process::id(), tag));
		let mut out = WavOut { writer: hound::WavWriter::create(&path, wav_spec(pcm16)).unwrap(), dump: None };
		say_tokens(tokens, texts, &mut out);
		out.writer.finalize().unwrap();
		path
	}

	#[test]
	fn every_spoken_phone_has_a_token_of_its_own() {
		for p in 1..=44 {
			let (a, b, s, _) = _utils::PHNIDToPhoneme(phone_token(p).unwrap());
			assert_eq!((a, b), (p, 0));
			assert!(!_utt::is_vowel(p) || s == 2 || matches!(p, 6 | 7 | 20), "phone {} stress {}", p, s);
		}
		assert!(phone_token(0).is_none() && phone_token(48).is_none());
	}

	#[test]
	fn sixteen_bit_output_is_the_float_output_rounded() {
		// it is .
		let (tokens, texts) = (&[95, 53, 76, 95, 53, 87, 92], &["it", "is"]);
		let fp = render_file(tokens, texts, false, "f32");
		let ip = render_file(tokens, texts, true, "i16");
		let mut r = hound::WavReader::open(&ip).unwrap();
		assert_eq!((r.spec().bits_per_sample, r.spec().sample_format), (16, hound::SampleFormat::Int));
		let ints: Vec<i16> = r.samples::<i16>().map(|x| x.unwrap()).collect();
		let floats: Vec<f32> = hound::WavReader::open(&fp).unwrap().samples::<f32>().map(|x| x.unwrap()).collect();
		let _ = (std::fs::remove_file(&fp), std::fs::remove_file(&ip));
		assert_eq!(ints.len(), floats.len());
		assert!(ints.iter().any(|&x| x != 0), "16-bit output is silent");
		for (i, (&a, &b)) in ints.iter().zip(&floats).enumerate() {
			assert!((a as f32 - b.clamp(-1.0, 1.0) * 32767.0).abs() <= 0.5, "sample {}: {} for {}", i, a, b);
		}
	}

	#[test]
	fn playback_follows_the_schedule() {
		// it is , cat . : every segment plays in turn, starting where the durations before it add up to, less the
		// transition the phone before it carries in
		let (tokens, texts) = ([95, 53, 76, 95, 53, 87, 91, 90, 56, 16, 76, 92], ["it", "is", "cat"]);
		let segs: Vec<u32> = render(&tokens, &texts).segs.iter().map(|&s| s as u32).collect();
		let texts: Vec<String> = texts.iter().map(|t| t.to_string()).collect();

		// the schedule the driver plays: the start pause, then each phrase's segments, release and pause
		let mut utt = Utterance::build(&tokens, &texts);
		_dur::assign(&mut utt, 1.0);
		let mut durs = vec![(_dur::START_PAUSE_MS, true)];		// (ms, is a pause)
		for ph in &utt.phrases {
			let range = utt.words[ph.words.start].segs.start..utt.words[ph.words.end - 1].segs.end;
			durs.extend(range.map(|i| (utt.segs[i].dur_ms, false)));
			if ph.end != Boundary::None { durs.extend([(_dur::RELEASE_MS, false), (_dur::pause_after(ph.end, 1.0), true)]); }
		}
		let samples = |ms: f32| (ms.round() * m_sr as f32 / 1000.0) as i64;
		let ms = |n: i64| n as f32 * 1000.0 / m_sr as f32;
		assert!(segs.windows(2).all(|w| w[0] <= w[1]), "segments play out of order");
		let mut due = 0;
		for (k, &(d, _)) in durs.iter().enumerate() {
			let start = segs.iter().position(|&s| s == k as u32).unwrap_or_else(|| panic!("segment {} never plays", k)) as i64;
			let off = ms(start - due);
			if k > 0 {
				let (prev, after_pause) = durs[k - 1];
				// a pause carries nothing over, so the timeline is back on schedule after one
				if after_pause { assert!(off.abs() <= 1.0, "segment {} after a pause starts {} ms off", k, off); }
				assert!(off <= 1.0 && off >= -prev, "segment {} starts {} ms off (the phone before lasts {} ms)", k, off, prev);
			}
			due += samples(d);
		}
	}

	#[test]
	fn nothing_sounds_after_the_final_pause() {
		// it is .
		let out = render(&[95, 53, 76, 95, 53, 87, 92], &["it", "is"]).x;
		// the final pause: the last run of silence longer than 300 ms
		let silent = |x: &f32| x.abs() < 1e-4;
		let mut run = 0;
		let mut pause_end = None;
		for (i, x) in out.iter().enumerate() {
			if silent(x) { run += 1; } else { if run > m_sr * 3 / 10 { pause_end = Some(i); } run = 0; }
		}
		if let Some(i) = pause_end {
			let peak = out[i..].iter().fold(0.0f32, |m, x| m.max(x.abs()));
			panic!("sound after the final pause, peak {}", peak);
		}
	}

	// the normalizer's output with spaces collapsed
	fn says(t: &str) -> String { normalize(t).split_whitespace().collect::<Vec<_>>().join(" ") }

	#[test]
	fn long_input_does_not_crash() {
		let long = "Dr. Smith paid $45.50 for 3 km of road. ".repeat(250);
		assert!(says(&long).matches("doctor smith").count() == 250);
		let digits = "7".repeat(3000);
		assert_eq!(says(&digits).matches("seven").count(), 3000);
	}

	#[test]
	fn units_follow_the_number() {
		assert!(says("It is 100 km away.").contains("one hundred kilometers"), "{}", says("It is 100 km away."));
		assert!(says("It is 1 km away.").contains("one kilometer away"), "{}", says("It is 1 km away."));
	}

	#[test]
	fn times_and_ranges() {
		assert!(says("10:30 am").contains("ten thirty A m"), "{}", says("10:30 am"));
		assert!(says("at 3pm").contains("three p m"), "{}", says("at 3pm"));
		assert!(says("at 5 a.m. today").contains("five A m today"), "{}", says("at 5 a.m. today"));
		assert!(says("pages 3-5").contains("three through five"), "{}", says("pages 3-5"));
	}

	#[test]
	fn years_and_decades() {
		for (t, want) in [("In 1999 we", "nineteen ninety nine"), ("In 1905 we", "nineteen oh five"),
			("In 1900 we", "nineteen hundred"), ("In 2026 we", "twenty twenty six"), ("In 2005 we", "two thousand five"),
			("the 1990s", "nineteen nineties"), ("the '90s", "the nineties"), ("the 2000s", "two thousands"),
			("the 1900s", "nineteen hundreds"), ("it cost $1999", "one thousand nine hundred ninety nine dollars"),
			("1,999 people", "one thousand nine hundred ninety nine people")] {
			assert!(says(t).contains(want), "{} -> {}", t, says(t));
		}
	}

	#[test]
	fn dates_fractions_and_money() {
		for (t, want) in [("on 10/23/2026.", "october twenty third , twenty twenty six"), ("on 1/5/99.", "january fifth , ninety nine"),
			("add 1/2 cup", "one half cup"), ("add 3/4 cup", "three quarters cup"), ("a 5/8 inch", "five eighths inch"),
			("add 1/3 cup", "one third cup"), ("It was $1.5 million.", "one point five million dollars"),
			("and $20 billion.", "twenty billion dollars"), ("on Jan. 5 we", "january fifth we"), ("on March 3 we", "march third we")] {
			assert!(says(t).contains(want), "{} -> {}", t, says(t));
		}
	}

	#[test]
	fn abbreviations_keep_the_sentence_end() {
		for (t, want) in [("He lives on Elm St.", "elm street ."), ("St. Louis is far.", "saint louis is far"),
			("They met at the IEEE.", "i e e e ."), ("He got a Grade A.", "grade A ."), ("Visit www.example.com now.", "w w w dot example dot com")] {
			assert!(says(t).contains(want), "{} -> {}", t, says(t));
		}
	}

	#[test]
	fn capitals_numerals_and_quotes() {
		for (t, want) in [("NASA and the", "nasa and the"), ("HELLO WORLD", "hello world"), ("a PAT. PEND. sign", "patent pending"), ("Chapter IV begins", "chapter four begins"),
			("World War II ended", "world war two ended"), ("Henry VIII was", "henry the eighth was"), ("'Twas the night", "'twas the night"),
			("'Hello,' she said", "hello , ' she said"), ("A cat sat.", "a cat sat"), ("She left. \"Bye,\" he said.", "she left . bye ,"), ("He said \"hi\" to me.", "he said hi"), ("A B C.", "A b c"), ("Hi :) there", "hi happy face there")] {
			assert!(says(t).contains(want), "{} -> {}", t, says(t));
		}
	}

	#[test]
	fn times_web_addresses_and_versions() {
		for (t, want) in [("at 10:05 today", "ten oh five today"), ("at 3:07 pm", "three oh seven p m"), ("Wow :0 there", "wow shocked face there"),
			("Visit https://example.com/page now.", "visit example dot com slash page now"), ("See http://a.org.", "see a dot org ."),
			("on 2026-10-23.", "october twenty third , twenty twenty six"), ("Version 2.0.1 is out.", "version two point oh point one is out")] {
			assert!(says(t).contains(want), "{} -> {}", t, says(t));
		}
	}

	#[test]
	fn units_stay_out_of_words() {
		for t in ["background", "bookmark", "acme", "a kgb agent", "kganakga"] {
			assert_eq!(says(t), t);
		}
		assert!(says("5 kg").contains("five kilograms"), "{}", says("5 kg"));
		assert!(says("5kg of rice").contains("five kilograms of rice"), "{}", says("5kg of rice"));
	}

	#[test]
	fn units_after_numbers() {
		for (t, want) in [("5ft tall", "five feet tall"), ("1 ft", "one foot"), ("6ft2in", "six feet two inches"), ("2hrs later", "two hours later"),
			("30mins", "thirty minutes"), ("50mg dose", "fifty milligrams dose"), ("5kgs", "five kilograms"), ("2.5kg", "two point five kilograms"),
			("12oz can", "twelve ounces can"), ("1 mph", "one mile per hour"), ("1,000 ft", "one thousand feet"), ("10ms delay", "ten milliseconds delay"),
			("100km/h", "one hundred kilometers per hour"), ("1 km/h", "one kilometer per hour"), ("100 km/hr", "one hundred kilometers per hour"),
			("6 in a row", "six in a row"), ("5 minutes", "five minutes"), ("3 secrets", "three secrets")] {
			assert!(says(t).contains(want), "{} -> {}", t, says(t));
		}
	}

	#[test]
	fn pounds_euros_yen_and_heights() {
		for (t, want) in [("It costs \u{a3}20 now", "twenty pounds now"), ("just \u{a3}1 each", "one pound each"), ("about \u{20ac}15 now", "fifteen euros now"),
			("\u{20ac}1.50 each", "one euro and fifty cents each"), ("\u{a3}3.50 each", "three pounds and fifty pence each"), ("\u{a5}500 now", "five hundred yen now"), ("only \u{20ac}0.99 each", "only ninety nine cents each"),
			("a \u{a3}2 million deal", "two million pounds deal"), ("He's 6'2\" tall.", "six foot two tall"), ("She is 5'10\" tall.", "five foot ten tall"),
			("It costs 12.50 dollars.", "twelve dollars and fifty cents ."), ("only 1.01 euros", "one euro and one cent"), ("0.25 dollars", "twenty five cents"),
			("3.00 dollars", "three dollars"), ("It rose 1.50 percent.", "one point five oh percent")] {
			assert!(says(t).contains(want), "{} -> {}", t, says(t));
		}
	}

	#[test]
	fn a_closing_quote_keeps_its_punctuation() {
		for (t, want) in [("She said \"Hello.\" and left.", "she said hello , and left ."), ("He asked \"Why?\" and left.", "why ? and left ."),
			("She left. \"Bye,\" he said.", "she left . bye ,")] {
			assert!(says(t).contains(want), "{} -> {}", t, says(t));
		}
		assert_eq!(says("He said \"Bye.\""), "he said bye .");
	}

	#[test]
	fn years_by_context() {
		for (t, want) in [("There were 2015 votes.", "two thousand fifteen votes"), ("In 2015 votes were cast", "twenty fifteen votes"),
			("He was born in 1066.", "ten sixty six"), ("the 2020 election", "twenty twenty election"), ("1066 people came", "one thousand sixty six people"),
			("It ran 1500 miles", "one thousand five hundred miles"), ("In 1999 we", "nineteen ninety nine"), ("1999 was good", "nineteen ninety nine was")] {
			assert!(says(t).contains(want), "{} -> {}", t, says(t));
		}
	}

	#[test]
	fn homographs_follow_their_context() {
		// the capitalized spelling selects the dictionary's other reading
		for (t, want) in [("I will record a record.", "i will RECORD a record ."), ("I object to that object.", "i object to that OBJECT"),
			("We present a present.", "we present a PRESENT"), ("Please close it, it is close.", "please close it , it is CLOSE"),
			("The subject is hard.", "the SUBJECT is hard"), ("Prices will increase after the increase.", "will increase after thee INCREASE"),
			("They export grain.", "they EXPORT grain"), ("a separate room", "a SEPARATE room"), ("to separate them", "to separate them"),
			("an estimate", "an ESTIMATE"), ("to estimate it", "to estimate it"), ("The project is late.", "the PROJECT is late"),
			("They project growth.", "they project growth"), ("It affects us.", "it AFFECTS us"), ("He records it.", "he RECORDS it"),
			("Objects fell.", "OBJECTS fell"), ("the objective", "thee objective")] {
			assert!(says(t).contains(want), "{} -> {}", t, says(t));
		}
	}

	#[test]
	fn a_run_of_marks_is_one_mark() {
		// a question if any of them asks, else an exclamation; one mark, so one sentence
		for (t, want) in [("You did that?!", "you did that ?"), ("You did that!?", "you did that ?"), ("No way!!!", "no way !"),
			("Really??", "really ?"), ("Seriously\u{203d}", "seriously ?"), ("What\u{2049}", "what ?")] {
			assert_eq!(says(t), want, "{}", t);
		}
		// a mark after a space is still read out
		assert!(says("Is it ? yes").contains("question mark"), "{}", says("Is it ? yes"));
	}
}
