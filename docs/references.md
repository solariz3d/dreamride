# Reference songs

Songs the keeper heard and kept, each held by a test so that no change can quietly alter them. A reference is the
outside every generator can be measured against: proof that the system once made this, and a target that says what
"good" sounded like to the person it is for.

## 1. Seed 885243, version 2: the oneline species (2026-11-04, ~07:10)

The keeper, after listening: *"the one i saved right now is pretty gas"*, then *"first fully completed song, id wager
one of our lucky ones"*, and *"make sure it is saved, even put it on the repo as the first reference, even if it
wasnt found on the latest build"*.

| what | file |
|---|---|
| the seed: opening it replays the whole stream at version 2 | `examples/stream_s885243_v2.seed` |
| **the whole song** as one song file, 16 stages, 128 bars at 153 bpm, 200.8 s with its ring-out; it plays on any build, whatever the generators become | `examples/reference_s885243_v2_whole_song.txt` |
| the stretches the app saved while he listened | `examples/stream_s885243_1-3.txt`, `_1-4.txt`, `_11-14.txt` |

What it is:
- The **oneline** species: one melodic line, with its kit and instruments drawn by version 2. Its hat is a noise hat
  with a 45 ms decay, not the old chirp, and its lead is a pluck with a bright rolloff at 3268 Hz.
- **It ended by its own closure**, not by the clock: 0.78 before its coda, against the 0.70 that ends a song.
- Its coda went home: the opening progression (`oneline/prog.lydian_float`) and the first melody (`oneline/lead.rolled`)
  came back, then IV V7 I under a final ritard, a last kick, and it rang out.

**Held by** `tests/versions.rs`, `version_2_plays_the_first_reference_song_whole`. Version 2 must play the whole song
byte for byte. The stretch the app saved live (stages 11 to 14) must be part of it byte for byte too, which is the
proof that the reference is what he heard, not a later regeneration.

**One difference from the live stream, stated:** as one file, the whole song is normalised to its loudest moment (it
peaked at +5.55 dBFS and was scaled by -6.55 dB), so it plays back quieter than the stream did. The music is the same.

## 0. Seed 152637, version 1: the automaton species (2026-11-04, ~06:16)

Listened to for five minutes, and the first seed saved. It is not a whole song (no ending was heard), so it is not
numbered as a reference, but it is held the same way: `version_1_replays_the_keepers_saved_seed_byte_for_byte`. Its
recovery is in `docs/versions.md`.
