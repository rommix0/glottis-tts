# Glottis TTS

A formant text-to-speech synthesizer for English, written in Rust.

The majority of this code has been written by hand by Anthony C. Bartman, while Claude Opus 5.5 was used for assistance during the touch-up stage.

It turns plain English text into mono, signed 16-bit speech at 22050 Hz. The library is `#![no_std]` and needs only an allocator, so the same code runs on a desktop or on a microcontroller with an FPU.

```
text -> normalizer -> dictionary / suffix matching / letter-to-sound
     -> utterance -> durations and pitch -> formant synthesizer -> samples
```

## Features

- Text normalizer for numbers, money, times, abbreviations and symbols
- Pronunciation from a dictionary, stem + suffix matching, and a letter-to-sound tree trained on CMUdict
- Pitch contours for statements, yes/no and wh-questions, exclamations and commas
- 8 voices and an adjustable speaking rate
- Async, streaming output: samples go to your writer as they are made, so it can feed a sound card in any runtime
- Float samples with per-phone segment indices and a trace, for lip sync
- Direct phone input that skips the text front end

## Quick start

```rust
use tts_synth::{WriteSample, synthesize};

struct Samples(Vec<i16>);

impl WriteSample for Samples {
    type Error = std::convert::Infallible;
    async fn write_sample(&mut self, s: i16) -> Result<(), Self::Error> {
        self.0.push(s);
        Ok(())
    }
}

// inside an async context:
let mut out = Samples(Vec::new());
synthesize("Hello, world.".to_string(), &mut out).await?;
```

Try it from the command line (writes `tts_demo.wav`):

```bash
cargo run --release --example tts_demo -- "this is an example sentence."
```

Options include `--voice N`, `--rate R`, `--16bit`, `--text`, `--trace` and `--phone N`.

See [how to use.txt](how%20to%20use.txt) for the full guide: voices, rate, float samples and timing, phone input, and running without `std`.

## Tools

The `tools/` folder holds the Python scripts that generate parts of `src/`:

- `build_lexicon.py` builds the letter-to-sound tree and exceptions dictionary from CMUdict
- `build_normalizer.py` builds the text normalizer from `normalizer_rules.txt` (needs `re2c`)
- `formant_editor.py` is a Tk GUI for editing the formant table with live preview

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
