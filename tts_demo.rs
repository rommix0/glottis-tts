// the command line around the synthesizer, for trying it by hand: reads the options, writes tts_demo.wav
//   cargo run --release --example tts_demo -- "text to speak."
use std::env;
use std::fs::File;
use std::future::Future;
use std::io::{BufWriter, Write};
use std::pin::pin;
use std::task::{Context, Poll, Waker};
use tts_synth::{_vtm, SpeechSink, Speaker, SAMPLE_RATE, normalize, phone_token, to_i16};

type Wav = hound::WavWriter<BufWriter<File>>;

// mono at the synth's rate, as 16-bit integers or 32-bit floats
fn wav_spec(pcm16: bool) -> hound::WavSpec {
	let (bits, format) = if pcm16 { (16, hound::SampleFormat::Int) } else { (32, hound::SampleFormat::Float) };
	hound::WavSpec { channels: 1, sample_rate: SAMPLE_RATE, bits_per_sample: bits, sample_format: format }
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

impl SpeechSink for WavOut {
	type Error = hound::Error;

	async fn sample(&mut self, x: f32, seg: usize) -> Result<(), hound::Error> {
		if self.writer.spec().sample_format == hound::SampleFormat::Int {
			self.writer.write_sample(to_i16(x))?;
		} else {
			self.writer.write_sample(x)?;
		}
		if let Some(d) = self.dump.as_mut() { d.write_all(&(seg as u32).to_le_bytes())?; }
		Ok(())
	}
	fn trace(&mut self, line: &str) { eprintln!("{}", line); }
}

// run the synthesis to its end; nothing here makes it wait, so this never spins
fn block_on<F: Future>(f: F) -> F::Output {
	let mut f = pin!(f);
	let mut cx = Context::from_waker(Waker::noop());
	loop {
		if let Poll::Ready(r) = f.as_mut().poll(&mut cx) { return r; }
	}
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

	block_on(async {
		if let Some(tk) = opts.phone {
			// the phone as a word of its own
			sp.sentence(&[90, tk, 92], &[""], &mut out).await?;
		} else {
			sp.speak(&opts.text, &mut out).await?;
		}
		// if formant synth not done, let's finish up the synthesis
		sp.finish(&mut out).await
	})?;

	out.writer.finalize()?;

	Ok(())
}
