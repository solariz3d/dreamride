# Voices that listen: generator version 3 (2026-11-05)

The keeper, after the first reference song: it is "interesting, but still missing something, like coherence", and he
reached for Hans Zimmer: music as a conversation, going back and forth, where sometimes it works and other times it
is "two people talking to each other while both arent listening". Then: *"dialogue shouldnt be its own thing there
should be different species of dialogue within the species themselves"*.

## What was true before

No lens counter-line ever reads the lead. Every one reads only the chords (`Hc`, the voicing), so the lead and the
counter agree on the topic (key, chord, tempo) and never respond to what the other does. The dialogue species has
call and response, but on a timetable: two bars each, whatever is played.

## What version 3 does (lead and counter, the lens first)

In this engine listening can be literal: every voice is a function of time, and any voice may read another's. When a
lens stage plays both a lead and a counter, a song draws one way of listening for the whole song:

- **imitation:** the counter plays what the lead played `d` beats ago (1, 2 or 4), a sixth or an octave below,
  snapped to the chord now sounding (`sc`). The lead's functions are read at an earlier time: every definition the
  lead's pitch and gate read that depends on `t` is copied with `t` replaced by `(t - d)`. That is a canon, built by
  reading the other voice.
- **contrary:** the counter mirrors the lead about the lead's own median, a fifth below, snapped to the chord: it
  rises when the lead falls. It keeps its own rhythm.
- **answer:** the counter sounds only where the lead's gate is closed: it speaks in the gaps the lead actually
  leaves, not on a clock. It is drawn only for a stage whose lead leaves gaps: the lead's gate, sampled every 16th over
  the stage, is open less than 80% of the time. Otherwise the stage falls back to imitation, since an answer to a
  voice that never pauses is silence.

Versions 1 and 2 are untouched. The reference song (seed 885243, version 2) and seed 152637 (version 1) still play
byte for byte (`tests/versions.rs`).

## Registered before the first run

1. **Mechanics (tests/dialogue.rs):**
   - in an imitation stage, the rendered counter's pitch equals the lead's pitch `d` beats earlier plus the interval,
     snapped to the chord, at every sampled frame where both sound;
   - in an answer stage, the counter never sounds while the lead's gate is open;
   - in a contrary stage, the lead's and the counter's pitch steps move in opposite directions more often than the
     same direction.
2. **The ears:** in a blind A/B of the same lens song at version 2 and version 3, a stretch where both lead and counter
   play, the keeper picks version 3 as the one where the voices are talking to each other.

**Falsified if** the keeper picks version 2, or hears no difference. Then literal listening between two voices is not
what was missing, or not enough of it.

**What a pass would not show:** that every species should listen this way, or that these three kinds are the right
ones. It is one pair, one species, one listener.

## Built (2026-11-05 ~01:10 to ~01:40, uncommitted)

- `pub mod dialogue;`, `GENERATOR_VERSION = 3`, `GENERATOR_VERSIONS = [1, 2, 3]`. A song draws its kind once
  (`dialogue::draw(seed)`). `Composer::dressed` applies it to a stage whose species is the lens, before the
  instruments. A stream can wander between species, so this is decided per stage, not per song.
- **Two things the registration did not foresee, both found by tests:**
  1. **The composer judges each candidate against the previous stage's played text** (its audibility). So a counter
     that listens would have changed which moves a seed takes, and "the same lens song at version 2 and version 3"
     would not have been the same song. From version 3 the composer judges by the stage without the listening
     (`judged_text`). Versions 1 and 2 still read the played text, as before. Held by
     `version_3_lens_stages_listen_and_version_2_stages_do_not`: every stage composes the same equation at both
     versions, and a stage without a duet plays the same text.
  2. **Function parameters shadow definitions.** `mn(x, a, b)` reads its argument `b`, not the song's
     `b = frac(t)`. The first `delayed` treated the parameter as the global, so it rewrote `mn`'s parameter into
     `b_d1` and changed the function. The imitation oracle caught it: counter −11 against the oracle's −7 at t 5.01.
     Fixed: a definition's free names exclude its parameters.
- The ring-out holds each voice's played pitch with its gate shut, so `apply` leaves a counter whose gate is the
  literal `0` alone.
- **A save bug older than this work**, found saving the A/B stretch: `save_text` names each stage's time `T_i`, and a
  stage-local definition named `T` (the lens's fractal, unshared when one stage of the stretch lacks it) was also
  renamed `T_i`. The saved song did not parse (`'T_1' is defined twice`), at version 2 as well. When that happens, the
  time is now named from a free base. Test first: `tests/stream.rs` `every_lens_stretch_saves`.

**Registered point 1 (mechanics), `cargo test --release --test dialogue`: 5 of 5 pass.**
- The imitation oracle is independent of the code it checks. It takes the ORIGINAL song's lead, evaluated by the
  engine at `t - d`, adds the interval, and snaps the result with the engine's own `sc` (`Program::compile_expr`). The
  counter must equal it to 1e-6 at every 16th where lead and counter both sound, for d 1/2/4 and intervals
  −12/−9/−8, over the duet stages of lens seeds 1 to 12.
- Each check was shown to be able to fail:
  - the imitation check failed on the real shadowing bug;
  - `imitation_is_not_the_lead_now` is the same oracle without the delay, and it must disagree;
  - the answer check failed when the counter's gate ignored the lead (mutant A);
  - the contrary check failed when the mirror followed the lead (mutant B).
- After each mutant, dialogue.rs was restored, sha256 993a915b…, identical before and after.

**Full suite** (`cargo test --release --no-fail-fast`, ~01:55): 28 test binaries, 338 passed, 0 failed, 7 ignored.
- `tests/versions.rs` passes: version 1 still replays seed 152637 byte for byte, and version 2 still plays the
  reference song whole.
- One existing test was changed: `tests/book.rs`, `every_book_stage_is_audible_within_the_budget_and_never_repeats`.
  The recorded audibility is now checked against what the composer judged (`Composer::judged_text`). At versions 1 and
  2 that is the played text, as before. The played change is still required to be audible, and at version 3 it is.

## The blind A/B (registered point 2), set up 2026-11-05 ~01:32, before listening

- **The stretch, chosen by rule and not by ear:** the first lens seed (`Seed::Book`, counting up from 1) with four
  consecutive stages of which at least three play both a lead and a counter. That is **seed 1, stages 2 to 5**:
  128 beats at 106 bpm, 72.45 s. The bin is `ab/src/bin/duet.rs` in the session scratchpad.
- **What it actually tests.** Seed 1 draws **answer** (delay 2, interval −12). But its lead is open 0.97 of every duet
  stage, which is not under `ANSWER_DUTY`, so stages 3 to 5 fall back to **imitation**: the counter plays the lead
  two beats later, an octave below, snapped to the chord. In stage 2 the lead is silent, so it is the same at both
  versions. Both voices sound together 0.94 of stages 3 to 5 at both versions (`ab/src/bin/duetstats.rs`). So this is
  an A/B of **imitation**; answer and contrary are not tested by it.
- **Finding on the way:** the lens leads hardly pause, so the answer kind will rarely fire as built. This is the
  same continuous-gate problem the lead A/B of 2026-11-04 found.
- **Loudness matched.** Peak normalisation scaled the two files differently, which left version 3 0.70 dB louder by
  RMS (v2 −18.14, v3 −17.44 dBFS). The louder clip was scaled down by 0.70 dB, and both now measure −18.143 dBFS RMS.
- **Assignment:** `crypto.randomInt(2)` assigned the two clips to `Downloads\dreamride_ab\dialogue\clip_1.wav` and
  `clip_2.wav`. The key is in a file outside the repo and the listening folder, and it was not shown to the writer:

      sha256 97df6463cf9bb794ca2a4a03db35f12c35bc8a10fee146b24c64a47cb09066e7

- **The question to the listener:** in which clip are the two melodic voices talking to each other? "Neither" and
  "same" are allowed answers.
- **Prediction (from the registration above):** the listener picks version 3. **Falsified if** they pick version 2,
  or hear no difference.

### Heard unblinded in the app, 2026-11-05 ~02:40 (NOT the registered test)

The keeper played version 3 in the app and saved four seeds: *"it is defintelly better … but it can still make
sections or big parts of songs that lose its coherence and begin to unwind in ways that just sound bad, but the
parts that are good are a lot better"*.

Measured afterwards with `ab/src/bin/v2v3.rs`:
- all four saves (410229, 23421 and 598575 oneline; 497865 dialogue) never enter the lens;
- they play **byte-identical at versions 2 and 3** at every stage;
- each saved stretch equals its regenerated version-3 stretch.

So what he heard was not the listening. Across 40 species seeds (`ab/src/bin/lensshare.rs`), 7 start in the lens,
106 of 660 stages are lens stages, and **27 of 660 (4%) play differently at version 3**. The keeper's reading, on
being told: *"damn luck rolls XD"*. The blind clips are still the test of registered point 2.

## Status (2026-11-05 ~01:05, before a context compaction; uncommitted)

- **Written:** this file (the registration above) and `src/compose/dialogue.rs` (`Kind`, `draw`, `apply`, the delayed
  copy of the lead's t-reading definitions, the lead's gate duty and median read by evaluating the played song).
- **Not yet done:**
  1. register `pub mod dialogue;` in `src/compose/mod.rs`;
  2. in `src/compose/stream.rs`:
     - `GENERATOR_VERSION = 3`, `GENERATOR_VERSIONS = [1, 2, 3]`, and the version list in its doc comment;
     - a `dialogue: Option<super::dialogue::Kind>` field on `Composer`, set in `with_version` when `version >= 3` and
       the stage's species is the lens (`book::of(eq).species.name == "lens"`), with `super::dialogue::draw(seed)`;
     - in `Composer::dressed`, apply `dialogue::apply` before the instruments for version 3;
  3. check that `4(t - 2)`-style substitution parses (substitution writes `(t - d)` where `t` was);
  4. `tests/dialogue.rs` for registered point 1;
  5. run `tests/versions.rs`: versions 1 and 2 must still pass, byte for byte;
  6. run the full suite;
  7. the blind A/B of one lens song at versions 2 and 3, with a coin-flip key sealed by sha256 and recorded here
     before listening, as in `docs/lead_ab_2026-11-04.md`.
- **Open design question:** answer stages keep the counter's own pitch line, speaking only in the gaps. Imitation and
  contrary replace it.

## Result

(appended after the run)
