# Dreamride handoff, 2026-11-05 (laptop, ~00:05 to ~07:40 local)

Written at the keeper's ask before leaving: *"lets do a comprehensive plan to put as a handoff within the repo for
dreamride we will continue work on it another time"*. Read this first, then the documents it points to. Everything
below is on disk. **Nothing from this session is committed**: last commit `227c7d0` (2026-11-04). `git status` lists
the session's work.

---

## 1. Where the keeper ended, in his words (read this before anything else)

Two corrections at the end of the session set the direction. Both were right.

1. *"I feel like we are not incorporating what we learnt from the research, still building off of what we made before
   learning new material and insights"*. He also said: *"yes if we have to redo some things it is okay"*.
2. After hearing the new phrase species (batch 2, overall): *"i think we are losing it some how, sounds like a kid
   first learning fl studio lmfao"*. Then: *"i think you are still working with too much of the old system before we
   did our research because you are commited to what we have already made and how it IS good"*.

**What that means for the next session.** The research was applied *inside* the old generator: the same stage-by-stage
composer with random moves, the same slots, the same drawn instruments and drums, the same fixed mix. The phrase
species swapped what goes into the slots, and that is all it changed. The next step is not another module or species.
It is a generator designed from the research up (section 4). **The engine stays**: music as functions of time is the
medium. The old *generator* is what is in question.

**The open question only the keeper can answer.** Is the "kid learning FL Studio" sound mostly the **writing** (the
notes, the patterns, the square phrases) or mostly the **sound** (the instruments, the drums, the mix)? It was asked at
~07:33 and not answered. The answer decides where the rebuild starts (section 4).

---

## 2. What was built this session (uncommitted; each has its doc and tests)

| what | where | status |
|---|---|---|
| **Generator version 3**: in a lens stage the counter listens to the lead (imitation, contrary motion, answer) | `src/compose/dialogue.rs`, `docs/dialogue.md`, `tests/dialogue.rs` (5) | built and tested. **The blind A/B is unrated** (section 3). Reaches only lens stages: 27 of 660 stages over 40 species seeds play differently |
| Version 3 composes the same moves as version 2 (`Composer::judged_text`) | `src/compose/stream.rs` | one existing test changed: `tests/book.rs`, audibility now checked against the judged text |
| **Save fix**: some lens stretches failed to save (`'T_1' is defined twice`), at version 2 too | `src/stream.rs`, `tests/stream.rs` `every_lens_stretch_saves` | fixed; the test fails on the old code |
| **B key in the app**: marks the last 4 stages as bad, into `judged\bad\` (S still saves the good ones) | `world/src/main.cpp`, `ui_logic.h`, `world/tests/ui_logic_test.cpp` | built, world tests OK |
| **The song measured against itself**: self-surprise, roughness, repetition, macroharmony, centricity, conjunct motion | `src/compose/measure.rs`, `tests/measure.rs` (9), `src/bin/luminesce-measure.rs` | built. `cargo run --release --bin luminesce-measure -- .` scores the registration once there are 8 good and 8 bad **songs** |
| **The phrase species**: one motif a song in phrase forms, parts that read the structure | `src/compose/species/phrase.txt`, `docs/phrase_species.md` | built, every species test passes. **`draw no`**: reached only by name (`Seed::Named`), so no existing seed moves. Judged "kid learning FL" overall, but batch 2 is unrated per clip |
| `draw no` species key; `Mn` motif number drawn per song (only for a species whose frame has `Mn`) | `src/compose/book.rs`, `src/compose/stream.rs`, `tests/species.rs` | the reach test exempts `draw no` species, as it already exempted Explore |

**Full suite at the end** (`cargo test --release --no-fail-fast`, ~07:10): **30 test binaries, 347 passed, 0 failed,
7 ignored.** `tests/versions.rs` passes, so seed 152637 (version 1) and the reference song 885243 (version 2) still
play byte for byte. World: `world\build.bat test` reported TESTS OK after the B key. The app on the desktop shortcut
was last built at ~04:10 (the B key, version 3). The engine changed after that (the measures, the phrase species), so
the shortcut's launcher rebuilds it on the next open. The phrase species will then be in the app, but no seed draws
it: it opens only from a seed file with `kind named` and `species phrase`.

**Build note (laptop).** A global `CARGO_TARGET_DIR` points at another project. For Dreamride, set
`CARGO_TARGET_DIR=C:\Users\zackn\Downloads\dreamride\target` (the launcher does this itself).

---

## 3. What was learned, with what held and what failed

The research is in `docs/what_makes_music_bad.md` (the perceptual findings: expectation, repetition, syncopation,
roughness; registered predictions and their amendments) and `docs/implementing_theory.md` (music theory: Tymoczko,
Krumhansl–Schmuckler, Caplin, von Hippel & Huron, Farbood, MorpheuS; how each maps onto a function-of-time engine).

**Results, each registered before it ran:**

| prediction | result | where |
|---|---|---|
| Version 3 dialogue: the keeper hears the voices talking at v3 | **unrated**, the blind clips are still waiting | `docs/dialogue.md` |
| Unblinded "it's definitely better" at v3 | **not v3**: all four saves were byte-identical at v2 and v3 (none entered the lens) | `docs/dialogue.md` |
| Hypothesis K: bad songs move to distant keys | **FALSIFIED**: bad 2 of 6 fail, good 7 of 9 fail | `docs/what_makes_music_bad.md` |
| Clip 4: the pedal-aware bass is preferred | **held**, "better but still bad" | `docs/aware_parts_ab_2026-11-05.md` |
| Clip 6: a noise hat plus an open hat at each chord change is preferred | **FALSIFIED**: he picked the original | same |
| The six measures separate bad from good | **not yet scored**: 2 bad songs from marks, 4 from batch 1, 9 good; scoring needs 8 bad songs | `docs/what_makes_music_bad.md` |
| The phrase species is rated good more often than the current generator | **unrated per clip**: overall "kid learning FL" | `docs/phrase_species.md` |

**Findings worth carrying:**
- **Values, not species.** The keeper's claim, confirmed twice on uncurated data. The same species was good and bad
  (chaos: batch clips 3 and 4; oneline: 1 and 6; automaton: 152637 and 85749), and the same progression appeared in a
  good song and a bad one. A floor cannot ban species; it has to judge what a song sounds like.
- **The keeper's ear is precise.** Clip 5 turned "ugly at 35 seconds", and stage 3 (a new progression, E major to E
  minor) begins at 34.6 s.
- **"Aware" recurs in his feedback** (the bass "aware of itself", the hat "aware of progression"). In this engine a
  part is aware of whatever its formula reads (`frac(t)` the beat, `Hr(i)` the chord, `mod(t, 32)` the phrase,
  `f(t - d)` itself). Most old parts read only `t`.
- **The rules give the floor, not the music.** Applied literally (kick on 1 and 3, a pop bass figure, square 8-bar
  sentences, everything on the grid), they produced "a kid learning FL". That is a finding about this approach, not
  only about the species.
- **The rating batch was the fastest honest signal of the night.** Random seeds, blind, rated in one sitting.

---

## 4. The plan: a generator designed from the research up

Not built yet, and not started.

### 4.0 First, answer the open question (five minutes, the keeper)

Writing or sound? If it is unclear, a blind test can split it: the same phrase-species song played with the current
synths, and the same notes through better sounds (or the other way round). Whichever change he hears decides the
order of 4.1 and 4.5.

### 4.1 The song plan (replaces the stage-by-stage composer for the new generator)

Before any note, a song draws its whole plan:
- **its form:** sections and their lengths, e.g. intro (4 bars), A (8), A (8), B (8), A (8), outro (4), with
  variants such as AABA, verse–chorus or build–drop;
- **its tension curve** across the sections (Farbood: start with each section's direction, up, down or flat, not its
  height; MorpheuS: a target tension profile);
- **its key, tempo, motif and instrumentation**, drawn once.

The plan is data the functions read: `section(t)`, `bar_in_section(t)`, `phrase_end(t)`, `tension(t)`. That is the
"structure layer" every part reads. Today's stream (stages chosen one by one by random moves and audibility) does not
plan, so it cannot hold a song's shape.

### 4.2 Harmony per section

Written progressions stay. Their phrase-aware cadences were the one structural thing the old system had right. They are
chosen by section role: stable in A, moving away in B, cadencing at section ends, the return home at the last A.
**Clip 5's lesson is a hypothesis, not a rule:** a progression change at a section boundary must be prepared. Hypothesis
K failed, so "distant key" is not the cause, and the cause of clip 5 is still unknown.

### 4.3 The motif, developed across the song

One motif a song, as in the phrase species, but developed across sections, not only inside an 8-bar phrase:
- stated in A;
- varied in the second A;
- contrasted (inverted or fragmented, in another register) in B;
- returned whole at the last A.

**Avoid what made the phrase species sound naive:**
- every phrase exactly 8 square bars;
- every note on the grid;
- the stock rhythms.

The research allows irregular phrase lengths (Caplin's expansions), off-grid timing (the Groove MIDI Dataset already
in `src/compose/groove.rs` has real drummers' micro-timing), and motif rhythms learned rather than chosen from four.

### 4.4 Arrangement that follows the plan

What plays in each section is chosen for the section's tension, not drawn at random. Parts read the structure:
- the bass pedals at section starts (the A/B winner);
- the drums fill into section ends, not at every stage seam;
- the counter answers in the lead's gaps (version 3's listening, generalised);
- a part enters for a reason: the B section, the build.

### 4.5 Sound and mix: the research gap

Nothing this session studied production. "A kid learning FL" is, in producers' language, usually about the sound:
- stock-sounding synths;
- no dynamics or automation;
- drums that are not tuned or layered;
- no mix (levels, EQ, space).

**The next research session** should cover what makes a production sound finished. It should come back with what
can be measured or computed in this engine: spectral balance, dynamics across a section, filter and level automation
following the tension curve, and the drum sound.

### 4.6 How each step is tested (the method that worked tonight)

- **Register before running:** the prediction, what would falsify it, and what a pass would not show. Write it in the
  doc, then run. Hypothesis K was falsified ten minutes after it was written, instead of becoming a build.
- **Blind, loudness-matched comparisons** against what exists now, with sha256-sealed keys. A rating batch of random
  seeds, rated good, bad or skip in one sitting, was the fastest signal.
- **Every claim about a change re-checked before it is told:** the "v3 is better" listening turned out to be songs v3
  never touched.
- **Old versions keep playing byte for byte** (`tests/versions.rs`). A new generator is a new version and never
  changes an old one.

---

## 5. Waiting on the keeper (blind, sealed, unrated)

| what | clips | key (sealed, do not open before rating) | its doc |
|---|---|---|---|
| batch 2: 6 phrase-species clips and 6 from the current generator, shuffled | `Desktop\Dreamride rating batch 2\clip_01..12` | `research/2026-11-05/keys_sealed/batch2_key.txt`, sha256 `f4e40877…` | `docs/phrase_species.md` |
| batch 1, clips 8 to 16 (1 to 7 are rated and opened) | `Desktop\Dreamride rating batch\clip_08..16` | `…/batch1_key.txt`, sha256 `a116b7d7…` | `docs/what_makes_music_bad.md` |
| the version 3 dialogue A/B | `Downloads\dreamride_ab\dialogue\clip_1.wav, clip_2.wav` | `…/dialogue_ab_key.txt`, sha256 `97df6463…` | `docs/dialogue.md` |

Opened already, kept for the record: `…/aware_ab_key.txt` (sha256 `daac263d…`). Rating batch 1's clips 8 to 16 would
very likely bring the bad side to 8 songs, which is what lets `luminesce-measure` score the six measures.

**The keys are in the repo. If it is pushed, they are public.** The keeper should not open them before rating, and
nobody else's reading them matters.

---

## 6. Where everything is

- **The docs, in the order to read them:**
  1. this file;
  2. `docs/what_makes_music_bad.md`;
  3. `docs/implementing_theory.md`;
  4. `docs/phrase_species.md`;
  5. `docs/aware_parts_ab_2026-11-05.md`;
  6. `docs/dialogue.md`.

  Also in the repo: `docs/references.md` (the reference song) and `docs/versions.md`.
- **The judged songs:** good in `examples\` (S saves with their seed files); bad in `judged\bad\` (B marks: seeds 85749
  and 187301). The rated batch 1 songs are listed in `docs/what_makes_music_bad.md`. They are not copied into the
  judged folders yet: when scoring, copy clips 1, 2, 3 and 7 to `judged\bad\` and 4 and 6 to a good folder, and teach
  `luminesce-measure` to read `judged\good\` too.
- **The session's tools** (they lived in a temp folder; now kept): `research/2026-11-05/tools/`, a small crate
  depending on the engine by path. Build it with its own target dir, e.g.
  `cd research/2026-11-05/tools && cargo build --release`. The bins:
  - `batch`, `batch2`: rating batches;
  - `notes`: a voice's notes bar by bar;
  - `keys`, `keyrule`: keys per stage, and hypothesis K;
  - `phrase`: list a phrase-species stream;
  - `v2v3`: does v3 play a seed differently;
  - `lensshare`: how often v3 changes anything;
  - `duet`, `duetstats`: the dialogue A/B stretch;
  - the rest are from 2026-11-04.
- **To hear a phrase-species song in the app:** copy any `research/2026-11-05/batch2/phrase_s*_v3.seed` into
  `examples\` (the library lists seed files) and open it. The format is `seed N`, `kind named`, `species phrase`,
  `version 3`.
- **The batch songs and seed files:** `research/2026-11-05/batch1/`, `batch2/`, `aware/` (the edited clips 4 and 6 and
  the script that made them), and `dialogue_ab/`. The WAVs are not kept: they render from the text files with
  `luminesce-render`, and the desktop folders hold them.

---

## 7. Also open (smaller, from before)

- **Clip 7's kick** ("weird position") was **not** built. The planned fix, anchoring the kick to the downbeat, was
  already true of `bounce/kick.half`. Ask what was weird.
- **Clip 6's hat:** separate the sound from the awareness with one more pair (the original against the noise hat
  alone).
- **The pitch knob** (from 2026-11-04): tape-style or musical transpose? Unanswered.
- **The brightness measure** (from 2026-11-04) cannot hear a pluck's onset. A corrected one needs its own registration.
- **The dialogue in other species** is superseded by 4.4 if the new generator is built.
- **Commit and push:** nothing from this session is committed. The keeper decides when, and whether the sealed keys
  go public with it.
