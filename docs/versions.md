# Generator versions: a seed is a song, forever (2026-11-04)

The keeper saved seed 152637 after listening to it for five minutes, and asked: *"shouldnt it save the whole seed, or is
the initial seed not predetermined to give the same mutations?"*

A seed **is** predetermined: the same seed and the same code compose and play the same stream, every stage, every
mutation, every instrument. But code changes. Within the hour after he heard it, the instrument code changed (drawn
kits, a drawn rolloff), and seed 152637 started drawing other instruments. So a seed alone is not an address. **A seed
plus a generator version is.**

## The rule

- `compose::stream::GENERATOR_VERSION` is the version new streams are composed by. `GENERATOR_VERSIONS` lists every
  version this build can replay.
- **Any change that alters what an existing version plays is a new version.** The old code stays, frozen, beside the
  new: `timbre_v1.rs` beside `timbre.rs`. Frozen files say so in their first line and are never edited.
- `tests/versions.rs` is the guard. Version 1 must replay the keeper's saved stretches of seed 152637
  (`examples/stream_s152637_1-2_heard.txt`, `_12-15_heard.txt`) byte for byte. A change that breaks it fails the
  suite: make the change a new version instead. The guard was shown to bite: one changed number in a frozen pad's
  attack range failed it on stages 12-15.
- The guard covers what seed 152637 exercises: the automaton species, its stream moves, version 1's instruments. A
  change elsewhere (another species, the lens drums) is not caught by it. Such a change should get its own golden
  seed when it matters to someone.

| version | what it is | code |
|---|---|---|
| 1 | the instruments as of 2026-11-04 05:45, the build seed 152637 was heard on | `src/compose/timbre_v1.rs` |
| 2 | drawn kits (no chirping hats), a drawn rolloff, held-lead filters | `src/compose/timbre.rs` |

## Seed files

`S` in a stream now saves two things: the last stages as a song, as before, and a **seed file**,
`stream_s<seed>_v<version>.seed`, in the same folder. A seed file is four lines:

```text
# Dreamride seed: the automaton species, seed 152637 ...
seed 152637
kind species
version 1
```

The Library lists `*.seed` files beside songs (their first comment line is the description). Opening one replays the
whole endless stream from stage 1, at its version (`lm_stream_start_seed_file`). A stream started from a song has no
seed file; its stages are saved as a song. The format and its errors: `stream::seed_file_text`, `stream::read_seed_file`.

## How version 1 was recovered, so it can be trusted

1. `timbre.rs` as of 05:45 was rebuilt by running the later edit in reverse. The two pure deletions in that edit were
   put back by hand at their known places.
2. The rebuild was checked against the **actual 05:45 engine**, a copy of `world/bin/luminesce.dll` kept before any
   rebuild. Asked through its C ABI for seed 152637's played stages 1 and 2, it matched the rebuild byte for byte,
   including the drawn lead (a held lead, 11 harmonics, sustain 0.62, cutoff 5.3 kHz). The app had saved the written
   lead instead: odd harmonics, a pluck. That is the save bug fixed the same day.
3. Saved without instruments, the rebuild reproduces the app's own two save files byte for byte, so every stage of
   both stretches is the one he heard.
4. End to end, the new engine started from `examples/stream_s152637_v1.seed` plays stages 1 and 2 identical to the
   05:45 engine.
