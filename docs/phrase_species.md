# The phrase species: the research built in, top-down (2026-11-05)

The keeper, ~06:34: *"I feel like we are not incorporating what we learnt from the research, still building off of
what we made before learning new material and insights"*. Then: *"yes if we have to redo some things it is okay"*.

He was right. After the research, the night went into measuring the old generator and patching two of its parts. The
research describes music built top-down: form first, a motif stated and developed, cadences where phrases end, and
every part reading that structure. Every Dreamride species is built bottom-up: each part is its own function of time,
and no part has a phrase, a cadence or a motif to read, because none exists.

## What was built (`src/compose/species/phrase.txt`)

**Already present, found while building.** The written progressions are phrase-aware: each starts on the tonic and
each phrase ends on a dominant (`harmony.txt`), so every stage seam is a cadence. Harmony had form; nothing played on
top of it read it.

- **One motif a song.**
  - `Mn`, a per-song number, is drawn once by the composer, like the key, and only for a species whose frame holds
    `Mn`.
  - The motif's rhythm is one of four written two-bar figures, as tables of "eighths since this note began" (9 a rest).
  - Its contour is a blancmange read on the position inside the two bars, so the idea recurs.
- **Three leads, one idea.** All three develop the same motif, so the stream's "a new melody" move changes the
  treatment, never the idea:
  - **sentence**: idea, idea again, fragmentation a step higher, a run down, a cadence;
  - **period**: a half cadence, then the idea again and a full cadence;
  - **answer**: two bars of rest for the counter.
- **Parts that read the structure.**
  - **The bass** holds a pedal while the chord contains it (as in the clip 4 A/B that won) and approaches every chord
    change from a half step below.
  - **The pads** are struck only when a voice moves.
  - **The counter** plays guide tones from the voicing.
  - **The kick** plays beats 1 and 3, with one syncopation every second bar, and is silent on the phrase's last beat.
  - **The open hat** sounds only where a half phrase or a phrase ends. It is no longer on every chord change, the form
    that lost the clip 6 A/B.
- **Reached by name only (`draw no`).**
  - A species seed never lands on it, and a stream never turns into it.
  - Adding it to the weighted draw would have moved existing seeds to other species, including the reference song.
  - `tests/species.rs` exempts `draw no` species from "every species is reached", as it already exempted Explore.
  - A generator version can take it into the draw later.

**Checked by reading the notes** (`ab/src/bin/notes.rs`), seed 3, the period:
- bars 1-2 and 5-6 are identical;
- bar 4 is a held half cadence;
- bar 7 runs D C B♭ A down to the cadence.

The first draft's run was snapped to chord tones on every beat and became leaps (C A C F♯). It is now pulled only on
beats 1 and 3, and runs from four steps above. **Known flaw, unfixed:** the sentence's cadence note is the dominant's
chord tone nearest the motif's first degree, so in seed 1 the run ends a third above where the cadence lands, not a
step.

## Registered before the first listen: a mixed blind batch

- **The clips.** Six phrase-species songs and six from the current generator (species seeds, as the app draws them),
  every seed drawn by `crypto.randomInt(1, 1000000)`, version 3, stages 1 to 3. Loudness is matched, and all twelve
  are shuffled together into `clip_01` … `clip_12` with nothing naming the source. The key is sealed by sha256 before
  listening.
- **The keeper rates each clip** good, bad or skip, as in the first batch.
- **Prediction:** the phrase clips are rated good more often than the current clips, AND rated bad at most half as
  often.
- **Falsified if** the phrase clips are rated good no more often than the current ones. Then top-down form, as built
  here, is not what the keeper's ear wants, or not yet enough of it.
- **What a pass would not show:** that the phrase species is good, only that it is better than now; or which of its
  parts did it. It is one listener and six songs a side.

**Full suite** (`cargo test --release --no-fail-fast`, ~07:10): 30 test binaries, 347 passed, 0 failed, 7 ignored.
- The phrase species passes every test every species must pass:
  - a complete book;
  - leads on chord tones on at least 60% of all beats and of strong beats;
  - deterministic streams inside the definition budget;
  - songs that end by themselves;
  - seams no rougher than the music.
- `tests/versions.rs` passes, so seed 152637 (version 1) and the reference song (version 2) play byte for byte, and
  "a seed lands on every species" passes with the `draw no` exemption.

**Sealed ~07:05.**
- Twelve clips (six `phrase:`, six `now:`; the seeds are in the key), matched in RMS to −23.33 dBFS and shuffled into
  `Desktop\Dreamride rating batch 2\clip_01.wav` … `clip_12.wav`.
- Key (outside the repo): `sha256 f4e40877946311b6f7386c8041bab1afdd6f9a4157ceaf729e29dc3b1ffcc1bc`.
- A first sealing attempt matched zero files, because a pattern edit had not applied. It wrote a key with no clips,
  whose digest is void. That key was deleted, not used.

## Result

(appended after listening)
