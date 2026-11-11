# Code rules for Glottis TTS

Code is read far more often than it is written. Write it so a person can follow it without the history.

## House style (from the hand-written code)

- Tabs for indentation. Keep lines to about 120 columns; break a long expression up instead of letting it run.
- One statement per line. No `a; b; c;` lines, no `const A: u8 = 1; const B: u8 = 2;` on one line.
- Each module opens with a short block comment saying what it is.
- Struct fields are aligned, one per line, with a short trailing comment when the name doesn't say enough:

  ```rust
  pub struct cbuf {
      size:   isize,        // buffer size
      head:   isize,        // producer
      tail:   isize,        // consumer
  }
  ```
- Stateful parts are small structs with `new()`, `reset()` and `step()` (or `read()`/`write()`), driven by a loop.
- Prefer a plain `for`/`while` loop to a long iterator chain or a closure inside a closure. A chain of two or three
  adapters is fine; past that, use a loop with named locals.
- Group the steps of a long rule set under short headers (`// VELAR PINCHING RULE`) and keep each step a few lines.

## Names

- Keep the established names (synth fields, consts, Klatt parameters, `engine`, `model`, `cbuf`, ...). Don't rename
  them, even where they break Rust conventions.
- New names say what the thing is: `next_f2`, not `nf`; `is_last`, not `flag`.
- No bare phone IDs or feature bits in new code (`41`, `0x800`): use a named constant, and reuse one if it exists
  (`PAU`, `PH_*`) rather than defining a second copy in another module.

## Functions

- One job per function, named for that job. Aim for a screenful (~40 lines); a long rule list is fine if it reads as
  a list.
- Few arguments; no boolean flag arguments that switch between two behaviours, write two functions instead.
- No hidden side effects: a function that looks like a query doesn't change state.
- Errors are `Result`, with enough context to act on. No `unwrap()` in library code outside tests.

## Comments

- Say why, not what. Short and plain, lowercase is fine: `// fall to L* after the rise`.
- One or two lines. If a comment needs a paragraph, the code probably needs a helper with a good name.
- Don't narrate history (`now`, `no longer`, `was changed to`); that belongs in the commit.
- No commented-out code; git keeps it.
- No mentions of third-party sources, products or people in anything new. No tool or skill tags in comments.

## Simplicity

- YAGNI: no options, traits or generics for a need that doesn't exist yet.
- DRY: one helper instead of the same few lines in two places. Look for an existing one first.
- Leave code a little cleaner than you found it, but keep cleanups out of commits that change the sound.
- `no_std` library; `libm` is the only dependency. Don't add another without asking. Licences: Apache/MIT only.

## Tests

- `cargo test` passes before and after every change.
- A test checks one behaviour and its name says which (`diphthongs_stay_oral_next_to_a_nasal`).
- Anything that changes the sound gets an A/B render to listen to before it is committed.
- Generated files are not edited by hand: `_text.rs` comes from `_textskeleton.rs` via `tools/build_normalizer.py`.

## Commits

- One change per commit. A short one-line message (under ~70 characters), no trailers, no attribution lines.
