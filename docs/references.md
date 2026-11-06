# Reference songs

Songs the keeper heard and kept, each held by a test so that no change can quietly alter them. A reference is the
outside every generator can be measured against: proof that the system once made this, and a target that says what
"good" sounded like to the person it is for.

## 2. Seed 239, version 3, hand-composed from stage 8: the oneline species (2026-11-06, ~03:38)

**The first reference whose shape was chosen by judgment, not rolled by the composer.**

**How it was found.**
- The keeper played seed 239 and saved it good at stages 1–4, 4–7 and 6–9. He marked it bad at 8–11, the first song
  on both sides: *"the last good one was the best one, and even better, there was a stage of it where it went bad that I
  also marked"*.
- He asked for it whole, then: *"do you want to take a shot at hand composing ontop of the 239 whole seed song,
  changing what you see needs to be better, i love the kicks and how the arps go at 1 min"*.
- After hearing the result: *"what that song does good is insane i like it"*. Of the down-pitched versions (tape-style
  and musical, both −2 semitones): *"the downpitch was bad i more so mean the song itself"*.

**What was changed, and why.**
- **The diagnosis.** In the composer's version, the song's best idea, `oneline/lead.four` (the arps, stages 6–7, 0:59 to
  1:23), played for 24 seconds. Then `lead.revealed`, the melody alone and gliding, replaced it for five stages, peaking
  an octave up, and the pitch classes per stage went from 5 to 9.
- **The changes.** By hand, from stage 8, each a single module swap through `book::install`:
  - stage 8 keeps `lead.revealed` for one stage, a reveal;
  - stages 9–10 take `lead.three`, the same curve in triplets, as a contrast built from the song's own material;
  - stages 11–14 bring `lead.four` back at the peak, through to the coda.
- **What did not change.** Stages 1–7, the kicks (the kick block is byte-identical to the composer's, sha256
  `e3aef7098cf8229f…` in both), every other part, and the coda.
- **The measure.** Pitch classes per stage 9–14 went from 9 9 9 9 8 8 to 6 6 5 5 7 7.

**The files.**

| what | file |
|---|---|
| the whole song, 16 stages, 128 bars at 161 bpm, 190.8 s | `examples/reference_s239_v3_hand_composed.txt` (sha256 `a0ad5a1994dda7f9…`) |
| the script that made it (every choice readable) | `research/2026-11-05/tools/src/bin/hand239.rs` |
| the composer's own version, for comparison | `fullsong 239 3` in the same tools |

**What it teaches the new generator** (`HANDOFF.md` §4):
- a song needs an identity it keeps;
- a contrast section should come from the song's own material;
- the climax can be the return of what the listener loved, not something new.

Version 3 cannot compose it, so a test cannot hold the composer to it. The text file is the reference, and it plays on
any build.

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
