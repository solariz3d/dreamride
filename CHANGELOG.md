# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- **Drums sampled from real drummers** (`src/compose/groove.rs`, `src/compose/groove_model.txt`, `docs/groove.md`).
  The written lens drums were fixed patterns and the keeper heard the hats as "way too basic". The lens book's kick,
  hat, clap and perc modules are now 24 grooves generated from statistics learned from Magenta's Groove MIDI Dataset
  (CC BY 4.0). They cover per-step hit odds, loudness, timing, bar-to-bar change and fills, and each bar is kept inside
  the band of real bars. On held-out data, city pop grooves classify as city pop 62% of the time, against 64% for real
  drummers; latin fails the same test and is not used. The learner and that test are in `tools/groove/`. Tests:
  `tests/groove.rs` (7, including every rendered hit checked against its table).
- **The first reference song** (`docs/references.md`). Seed 885243 at version 2 is a oneline song the keeper heard
  through to its own ending ("pretty gas", "first fully completed song") and asked to keep in the repo. It is kept as
  its seed file, as the stretches the app saved, and as the whole song in one file
  (`examples/reference_s885243_v2_whole_song.txt`), which plays on any build. `tests/versions.rs` holds version 2 to
  it byte for byte, and holds the live-saved stages 11-14 to be part of it, which proves the reference is what was
  heard.
- **Generator versions and seed files** (`docs/versions.md`). A seed plus its generator version now replays the same
  song forever. `Composer::with_version`, `Stream::start_version`, and `GENERATOR_VERSION` (2) with
  `GENERATOR_VERSIONS` ([1, 2]); version 1 is the 05:45 instruments, frozen in `src/compose/timbre_v1.rs`. `S` in a
  stream also writes `stream_s<seed>_v<version>.seed`. The Library lists `*.seed` files, and opening one replays the
  whole stream at its version. New C ABI: `lm_stream_start_seed_file`, `lm_stream_save_seed`, `lm_stream_version`.
  Guard: `tests/versions.rs` holds version 1 to the keeper's saved seed 152637 byte for byte, and was shown to fail
  when one frozen number changed. Added `examples/stream_s152637_v1.seed`.
- **Drawn drum kits** (`timbre.rs`, version 2). The keeper: "the same tweeting hi hats are getting so stale"; every
  groove used one hat, a dash falling 7600 to 5400 Hz in 5 ms. Each song now draws a kick, a clap or snare, a hat and
  an open hat, each matched to the written sound's one-hit energy. No drawn hat sweeps more than 5% (checked over
  200 palettes). The engine's noise `hat` and `snare` take `decay` and `length`, built-in values when absent. Also
  drawn per instrument: the rolloff (660 a third of the time, else 1500-6000 Hz), held-lead filters, more detune and
  drive. The registered brightness span (1.0 harmonic) **failed at 0.70** and is kept as an ignored test.
- **Instruments drawn per song** (`src/compose/timbre.rs`, `docs/timbre.md`). Every module carried one fixed sound,
  so every song wore the same lead, bass and pads; the keeper noticed. A book song now draws one instrument per tonal
  role from its seed and keeps it for the song. Each role draws from its own ranges: bass low and tight, pads slow and
  detuned, the arp a pluck, the lead a pluck, a held lead or a bell. Each instrument's gain is rescaled to the written
  one's loudness (harmonics, rolloff, filter and sustain). A gliding line holds its notes whatever its tone. A set
  whose full band scores more than 0.1 below the written instruments against the two reference recordings is drawn
  again. Registered run: 20 of 20 songs within the margin (18 required), 20 different leads, all three kinds. Explore
  tunings keep their written sound. Only the played text is dressed; how the stream composes is unchanged
  (`Composer::dressed`, `roster_eq`). Tests: `tests/timbre.rs` (6).
- **`world/launch.ps1`**: starts the world, rebuilding first when any engine or world source is newer than the built
  app. It refuses to start the old binary when the build fails, and refuses to rebuild while the app is running. It
  sets `CARGO_TARGET_DIR` to this repo's `target`, which `build.bat` assumes, because a machine-wide one otherwise
  breaks the link. It runs the build with `[Process]::Start` and `WaitForExit`, after two traps were hit or measured:
  `Start-Process -Wait` never returns (MSVC's PDB server outlives the build), and `Start-Process -PassThru` gives a
  null exit code. `world/launch.vbs` runs it with no console window, and the desktop shortcut points there.
  Verified: a touched source rebuilt and started the app in 14 s. Not yet exercised: the failed-build dialog.

### Changed

- **A song's novelty counts families, not modules** (`Composer::closure_parts`). A slot's generated grooves are one
  family (`groove::family`), so 24 takes of one groove do not make the book 24 ideas bigger. Without this, songs
  ended by the clock (3 of 10 by closure, where 5 is required); with it, 7 of 10. Written modules are unaffected.
  **Roll back:** restore the module count in `closure_parts` and set `book::GROOVES` low (about 3), or remove the
  `replace_drums` call in `book::all`.

### Fixed

- **A saved stream stretch keeps the instruments that were heard** (`stream::save_text`). It merged each stage's
  undressed equation, so the drawn instruments were lost. The keeper saved seed 152637 after five minutes of listening,
  and the file carried the written lead (odd harmonics, a pluck) instead of the held lead he heard. It now merges each
  stage's played text. Test: `a_saved_stretch_keeps_the_instruments_that_were_heard` fails without the fix.
  **Recovered:** `examples/stream_s152637_1-2_heard.txt` and `_12-15_heard.txt` were saved from the 05:45 instrument
  code, rebuilt by reversing the later edit. That rebuild matched the 05:45 engine kept from `world/bin` byte for byte
  on stages 1 and 2, including the drawn lead. Saved without instruments, it reproduces the app's own two files byte
  for byte. The original files are kept beside them.
- **A due bass is swapped on the boost path too** (`Composer`, the "too quiet a change" boost). A bass that had played
  `BASS_STAYS` stages was swapped in every normal candidate but not in boosted ones, and boosting the unchanged base
  could carry the old bass past its stay. The gap was always there. The new drums changed the stream's random path,
  and seed 2 then held one bass for 7 stages (`the_bass_changes_as_a_song_goes_on` allows 6). With the fix, its
  longest run is 3.

- **Written harmony: the progression layer** (`src/compose/harmony.txt`, `src/compose/harmony.rs`; a loops
  stream whose chord root cycled by `mod(c + m, 5)` every 11 beats, out of step with the bar, was judged to sound bad
  because of its progression). Harmony is written, never
  generated: twenty progressions written by hand (Roman numerals, 7ths, 9ths and colours, a mode, a phrase length,
  energy and mood), each starting home and ending on a dominant, a cadence at every phrase end and at every stage
  seam, chords only on bar lines and half bars, a deceptive turn at most once a song. Each chord is voiced for three
  pads with the smallest movement from the one before (no crossing, fixed registers) and stored as one-digit-a-half-bar
  tables (`Hb`, `Hx`, `Hy`, `Hz`) read by shared helpers (`Hr`, `Hv`, `Hc`, `sc`, `hs`) in a key `K` drawn per song.
  Tests: `tests/harmony.rs` (11: parsing, cadences, bar lines, exact tables in any key, minimal voice leading, guide
  tones), plus a species test that every lead and counter lands on chord tones, and the deceptive turn and home key.
- **Explore (strange tunings)**: the species of strange tunings (19-tone `loops`, Bohlen-Pierce `tritave`, `ratios`,
  and `maqam`, the quarter-tone bounce kept whole) are no longer in the default stream. `Seed::Explore`,
  `luminesce-stream --explore`, `LM_STREAM_EXPLORE` (8) in the C ABI, and a Library entry beside Stream.
- Excerpts `examples/harmony_<species>.wav` (2 minutes each, not committed: 23 MB).

- **Song species: a layer above the move book** (`src/compose/species/*.txt`, `src/compose/book.rs`,
  `src/compose/stream.rs`; every stream's function looked nearly the same, with only small changes, and needed
  radical changes in its constraints and in what is possible). Every stream now
  picks a species first, each its own universe of constraints and its own definitions, so the equation itself looks
  different: `lens` (the move book: 12-tone modes, 4/4), `ratios` (just intonation from whole-number ratios, in 3/4),
  `chaos` (the logistic map near the edge of chaos, in 7/8, pentatonic), `automaton` (elementary cellular automata,
  rules 30/110/90/45, in 5/4 whole tone), `loops` (Eno's incommensurable loops in 19-tone equal temperament),
  `tritave` (Bohlen-Pierce in 12/8, odd harmonics), `bounce` (bouncing balls, a pendulum and springs in quarter-tone
  maqam, half time), `oneline` (one curve heard as bass, chords and melody by auditory streaming, 150-170 bpm) and
  `dialogue` (call and response on a breathing rubato clock, minor pentatonic). Each is a book with a `== species`
  header (tuning, meter, generator, form, timbre, bpm range, weight, openings). A seed picks the species, weighted
  toward the lens and the dialogue (what was kept); a species stream may change species at a later arc (chance 0.3,
  a species whose tempo fits). `luminesce-stream --species NAME` plays one, `--book` the lens alone; the C ABI's
  `LM_STREAM_LENS` (4, with `LM_STREAM_GRAMMAR`) the lens alone. Tests: `tests/species.rs` (23: every module renders
  over every progression, every species streams deterministically and audibly within the budget, ends by itself,
  pulls without clipping, seams no rougher than its music; scaffolds share at most a tenth of their names; exact
  ratios, 19-TET, Bohlen-Pierce and quarter-tone steps; loops realign only at their LCM; the logistic map and the
  CA as written; every species' window holds every roster voice: a species without an arpeggio, bass or pads
  keeps silent ones of the roster's kind, which the C ABI window test caught). Two-minute excerpts:
  `examples/species_<name>.wav`. Details: `docs/composer.md`, "Song species".
- **Three built-ins** (`src/expr.rs`): `log2(x)` (the live equation writes it log with a lowered 2); `iter(F, x0, n)`, a definition of one parameter applied n times
  (at most 4096), the iterated map the language had no way to write; `ca(rule, row, width)`, one step of an
  elementary cellular automaton on a ring. Why: the species needed the harmony written as ratios, the logistic map
  written as itself and the automaton's rule as a number on screen.

- **The bass moves the water** (`world/src/water.h`, `world/src/main.cpp`, `world/shaders/world.hlsl`; the idea:
  basslines close to the water could make it vibrate, the way the kicks push waves). Every voice low on
  the wall shakes the floor into standing waves, as hard as its live loudness times `exp(-height / reach)`, and the
  ripple size is the note's pitch: Faraday's half frequency, water's capillary-gravity dispersion, times one visual
  scale (55 Hz is drawn 0.83 m, 110 Hz 0.50 m, 220 Hz 0.31 m; a higher note is always finer, so a glide visibly
  tightens or loosens the rings). Each note sends a fresh ring out from under where its draw-head began, the ring's
  front at the group velocity; the waves decay, stop being driven when the note stops, and sum with the kicks' rings
  into one surface that bends the raytraced reflection. Why: the program draws the maths that makes the sound, and
  the floor answered only the kick. Theme values `bassWaveAmp` (0.009 m, 0 turns it off), `bassWaveScale`,
  `bassWaveDecay` and `bassWaveReach`, the first three as Theme-tab sliders. Tests: `world/tests/water_test.cpp`
  (in `build.bat test`: the mapping is monotonic and in range, the dispersion and group velocity are right at both
  limits, the coupling falls with height). Bench at 1080p on the RTX 5080: p99 1.17-1.52 ms, unchanged within
  noise. Captures: `world/captures/water_bass_heavy.png`, `water_bass_glide.png`, `water_bass_after_kick.png`.
  Not carried: at a looped song's wrap the previous loop's notes send no waves (the kicks' rings are carried).

### Changed

- **Prepared for a public release: the docs, comments and skill are in the writer's own words.** Quotations of the
  author and references to him by role were restated plainly (same meaning, same reasons) across the README, DESIGN,
  CHANGELOG, CREDITS, `docs/`, the compose skill, source and test comments, and one test's assertion message (wording
  only, no logic). The author's exact words now live in `.claude/skills/compose/taste.local.md`, which is gitignored
  (`*.local.md`) and which the skill tells a session to read first when it exists. The README and DESIGN titles say
  **Dreamride**; the crate and binaries keep the working name `luminesce*`. A third party's contact email was dropped
  from `docs/research/prior_art.md`. No behaviour changed.
- **Every 12-tone species plays the written progressions** and keeps its own generators for melody, rhythm and shape
  (`book.txt`, `species/chaos.txt`, `automaton.txt`, `bounce.txt`, `oneline.txt`, `dialogue.txt`): the lens's leads
  pull to the chord through `hs`, its basses follow the written bass notes with approach notes; chaos's regime `r`
  and the automaton's rule moved from the progression slot to the leads; the automaton left whole tone (it cannot hold
  a dominant resolving home); the bounce's balls land on chord tones; oneline's curve reads the voicing; the
  dialogue's bass and pads moved onto the grid under its breathing voices. Their own progression modules are gone.
  The mode move no longer changes a written progression's mode; the coda plays IV V7 I in the song's home key.
- Tests whose assumptions this changed, each said in `docs/composer.md` ("Tests changed on purpose"): the lens's
  progression count, the bass and coda checks reading `Hr` instead of `R`, the species' shared vocabulary and tuning
  axis, the default seed's species set, and the opening-bass and bass-variety bounds of a smaller default pool.
- The compose skill's taste and rules: "harmony is written, never generated".

- **Streams open many ways and the bass moves** (`src/compose/book.rs`, `src/compose/stream.rs`; repeated testing
  kept hearing the same bassline, and the stream history showed two of the last three
  streams opening on one bass line, every one on the bass and the hats). The opening bass is drawn from the whole
  bass set (the intro's need made the quiet ones likeliest); each species lists several openings (a lead alone,
  pads and a perc, the counter-line and hats, ...); a playing bass that has kept one module three stages is swapped,
  and a swap takes a stale slot first. Measured over 160 seeds: 27 opening basses, none above 7.5%; 22 openings,
  none above 11%. `tests/book.rs`'s seed-7 opening assertion now checks the lens's openings instead of the bass
  and hats (the old expectation is the behaviour that was asked to change).
- **A stream ends by its own closure only after two arcs** (`Composer::should_end`): a species with a small book spent
  its novelty in one arc and ended at 2.5 minutes. The lens's songs already took two.
- **`grammar::uses` reads a name right after a number** (`2v` is 2 times v), so definitions read that way are kept and
  their helpers added.
- **The default seeded stream is a species stream** (`Seed::Species`; `lm_stream_start` with `LM_STREAM_GRAMMAR`).
  Replays are unaffected (the history keeps every stage's text). `tests/book.rs`'s C ABI test now passes
  `LM_STREAM_LENS` to compare the same lens stream. The world needs `LM_STREAM_LENS = 4` in `world/src/luminesce.h`
  only to ask for the lens alone.

- **The live equation sits in the top-left corner, in smaller type** (`world/src/math_layout.h`,
  `world/src/main.cpp`; the equation was wanted smaller and in the top-left corner). Compact type is
  about 70% of what it was (10 px at 1080p, was 14; 14 px at 150%, was 20), the full panel's too (15 px at 1080p,
  was 21). Both go top left first, beside the menu when it is open, and clear of the draw-head; when the head
  covers the top left they fall back to the old places. Capture: `world/captures/eq_topleft.png`.

- **The world answers the engine's real ending** (`world/src/main.cpp`). The stub `StubStreamEnded` is gone; the
  world asks `lm_stream_ended`, which knows the song's length (ring-out included) a stage or so ahead. The feeder
  plays to that last frame and no further (the engine's own ring-out decays there, so the world's four-second fade is
  gone); as the last stage, the ring-out, begins, the camera lifts back; once it has sounded the history marks the
  entry finished with its length, and auto-next and **Replay this song** work as before.
- **The pull-back at a song's end shows the whole song's trail**, not the last two stages: every stage is merged
  (`hist::MergeStages`, as a replay merges them) into one song, rendered at 24 kHz on a thread while the last stages
  play (it is drawn, never played; measured 19-23 s for a 17-stage song, against 30-54 s at 48 kHz), swapped in for
  the window as the pull-back begins, and the camera and fog set back far enough to frame all of it, start to end.
  If it is late the pull-back starts over the last stages and the camera eases further back when it comes in; the
  hold waits for it (up to 25 s more). A stream stopped meanwhile never waits for the build; quitting joins it. If
  the merge fails (the world draws at most 32 voices; a whole song merged 17-29 over five seeds), the pull-back
  shows the last stages and says why on stderr. `--capture-menu ending` captures the whole-song pull-back. The
  stream's clock now reads the stream voice's own rate, not the drawn song's (the 24 kHz trail had doubled it;
  caught by the full-song `--stream-test`).
- **`--stream-test` follows a song to its end** when it ends within the time given: the audio stops at the engine's
  end, the pull-back frames the whole song, the ending card holds, the history (with `--history-dir`) marks it
  finished, and auto-next starts a new song.

### Removed

- `--stub-stream-end N` and `StubStreamEnded`: the engine's signal replaces them, and `--stream-test` runs the real
  ending.

### Fixed

- **The camera no longer moves at a stream's seams** (`world/src/main.cpp`). Reported: at the seam
  from one eight-bar stage to the next, where more voices come in, the camera moved unnaturally, growing to fit.
  Two causes, both measured (`--camera-seam-test`, seed 4821, per-frame moves at 60 fps): the wall's pitch range
  was set by the first window and widened at each seam where a voice came in higher or lower, so the whole wall
  rescaled (0.140 -> 0.130 -> 0.090 m a semitone over three seams) and the camera's target with it; and the energy
  the camera breathes with was normalised per window, so a louder stage re-scaled it and the eye jumped up to
  2.55 m in one frame (against 6 mm a frame between seams), then surged for seconds. Now a stream's wall is framed
  once for the whole song (-49..54 semitones, which holds every stream measured: six book seeds and two G; a note beyond
  it still widens the frame, and says so on stdout); a stream's energy has one fixed scale, measures levels as they
  sound (not against each voice's loudest in the window), and each window starts its lags from the window before's
  state, so two windows agree on the stage they share; and the breathing is off by default (Theme, Camera breathing
  brings it back, now slower: 8-beat lags, was 1.5; with it on, the largest jump at a seam is 0.18 mm on seed 4821
  and 1.9 mm on seed 3 over 16 seams, against 290 mm before the lags were seeded). The framing barely
  differs between songs (a song's wall is 5.6 m high unless it is very narrow), so nothing changes between songs
  either. Test: `luminesce-world.exe --camera-seam-test` (in `build.bat test`): six seams of a seeded stream: no
  jump at a seam (within 1 mm), the seam frame moves no more than the frames before it, no surge after it, and the
  wall keeps one scale and range. Before the fix it failed every check (largest jump 2,557 mm, seam-frame move 441x);
  after, the largest jump is 0.000 mm and the seam frame moves 1.00x (seed 4821; G, 6 seams, the same).

- **A move-book stream could not be drawn at all**: `lm_stream_window` failed with "no roster voice 'rise'", because
  the roster names the seam voice `rise` while the roster song (built from stage 1, its stash and `fill`) never
  held one. The world could not show a book stream's first window, and a replay could not go on from its seed
  (`--history-test`'s "the stream goes on from its seed after the recorded stages" failed). The roster song now holds
  each seam voice, from the book's first seam module that plays it (`src/compose/stream.rs`). Test:
  `a_book_streams_window_holds_every_roster_voice_through_the_c_abi`.
- **A replay merged across a book stage that defines `T(u)`** (the blancmange curve) did not load: the stage's `T`,
  renamed `T_i`, collided with the merge's own time name `T_i`. The time names now step aside (`T2_i`) as `Q` does
  (`world/src/history.h`).
- **A replay now plays an instrument swap as the stream does.** Since the move book, the stream releases a voice
  whose instrument changes at a seam and strikes the new one fresh; the merge gave every stage the last stage's
  settings (a stage recorded at gain 0.4 replayed at 0.5). A change of instrument is now a second voice (`bass_7`,
  struck in stage 7), and a voice that leaves rings out on its own functions instead of its pitch dropping to 0.
- **`--history-test`'s "two stages replayed play the second stage's own functions" measured the wrong thing for the
  move book** (the test was wrong, and is changed): it compared the stage's pitch with the merged song's at an instant
  up to a millisecond earlier (the merged song's frame grid does not fall on the stage's start), demanding equality
  within 1e-6 semitones. v2's stepped melodies passed; the book's continuous glides moved within that millisecond
  (926 of 10,455 samples differed, by about 0.001 semitones). It now checks the merged pitch lies between the stage's
  own frames on either side (0 of 10,455 outside), with a tolerance of 0.1% instead of 1%. It also now merges every
  stretch of the recording and loads it in the engine. The unit test that pinned "the last stage's settings" is
  replaced by tests of the swap, the release and the `T(u)` collision.

### Added

- **The move book: the stream's content written by hand** (`src/compose/book.txt`, `src/compose/book.rs`). Listening
  tests on stream v2 found the kick good and the way it changes intelligent, but the instruments and melodies flat and
  still in the style of MIDI rather than abstract graph music like the reference video. The design decided: the brain writes the material, the algorithm
  only arranges it. The book holds 43 generator modules (12 leads, 4 counter-lines, 5 arpeggios, 7 basses, 4 pads,
  4 kicks, 3 hats, 2 claps, 2 percs), 12 progressions (city-pop and darker, as digit integers or a mod cycle) and 9 seam
  modules (rolls, a doubling roll, a riser, a dash cascade, a fall, a reversed swell, a glissando, a stutter, a tom run),
  each a few definitions with a one-line shape and tags (energy, density, mood, glide). Melodies are an absolute
  register curve (the blancmange curve that was liked, at several scales; Lissajous, Weierstrass-like and Lorenz-like sums;
  bits; Fibonacci mod 7) pulled onto the active chord on beats 1 and 3 (`cs`), so any module sits on any progression;
  13 of the 21 melodic modules glide or bend. Basses are lines (the correction: more fluid, more complex basslines
  instead of the root on every beat): pushes, ghosts, approach notes, walks, 808 slides, a slow blancmange, a call and
  response with the kick, folded into one low octave (`lo`, `lf`). Measured over all 12 progressions, every lead puts
  0.66-0.73 of its beat notes on chord tones (strong beats 0.99-1.00; the corpus 0.72-0.75), moves by step 0.26-0.77
  of the time and spans 8.1-11.2 semitones (`tests/book.rs`).
- **The stream draws from the book.** `Seed::Book(seed)` is the default stream (`--stream`, the library's Stream, and
  `LM_STREAM_GRAMMAR` through the C ABI; v2's grammar seed stays as `LM_STREAM_GRAMMAR_SONG`). The arc planner, the
  audibility rule and the critic stay; a new melody, a new progression, a groove and the new **Swap** move (a playing
  bass, pads, arpeggio or counter-line re-voiced) now install whole modules chosen by the stage's need (an intro and a
  breakdown low energy, a peak dense), avoiding those heard in the last four stages; seam modules mark the seams into
  every return and breakdown, every new arc and half the peaks. The grammar's generators remain the lead's fallback.
  The equation shows only what plays: a waiting layer's lines come back when it enters.
- **Songs end** (book streams). A song should have a length limit, and ideally know from its own structure when to
  end. After each stage the composer scores its
  closure: arcs completed (of two), novelty spent (book modules heard), fatigue (the last arc's audibility against the
  liveliest), home re-established (the opening progression playing), and full peaks reached. At an arc's end, past
  150 s, a closure of 0.7 ends the song, or the knowledge that another arc would not fit before 420 s; once closure
  passes 0.5 the next return heads home (the opening progression and first lead come back). The ending is composed:
  a coda of bass, struck pads, the first lead an octave down and the kick, the opening's bars then IV V I, under the
  Friberg-Sundberg final ritard (q = 3, to half tempo, integrated in closed form as `tr`), one last kick dash on the
  tonic, then a stage that holds every voice closed while it rings into the reverb. The stream stops where the ring-out
  falls below -80 dBFS; `Stream::ended_frames` and the new `lm_stream_ended` say so, and `pull` returns fewer frames
  there. Over ten seeds songs end at 264-334 s, after two arcs, eight of ten by closure.
- **`luminesce-stream`**: render a stretch of any stream to WAV, headless (`--seed N --from S --seconds S`,
  `--grammar` for v2's).

### Changed

- **The audibility measure hears a legato line.** A new pitch that settles for a sixteenth inside a held note is a note
  (melody and pitch, not a new onset), so a gliding lead is not heard as one long note.
- **A voice whose instrument changes at a seam is released and struck again** (`src/stream.rs`): swapping a sounding
  note's partials, gain or amp is a step in the waveform. Measured: such a swap read 0.0667 against the music's 0.0018
  before the change, 0.0002 after.
- **Book streams' budget**: 20 definitions, the book's shared helpers not counted (they are vocabulary, like the time
  names); the ending's clock `tr` is a time name.

- **The stream history: nothing is lost.** Every new song's seed should be captured automatically, so nothing is
  ever lost and any song can be gone back to or removed. Every stream that starts is recorded at once into `%APPDATA%\Dreamride\history\<date>_<seed>.txt`: its seed,
  start time, origin (grammar seed, G, or the seed song's text), the composer's version (an FNV-1a hash of
  `luminesce.dll`, so a recording made by another build is known not to replay the same from its seed) and every
  stage's full song text as it is composed, so it replays exactly after the composer changes (it is being rewritten
  now). Plain text, appended on a worker thread (no frame waits on the disk); a failed write is shown on screen until
  one succeeds, and the next write puts the whole record down again. Nothing is pruned on its own.
  (`world/src/history.h`.)
- **The History tab** (beside Library, View, Theme): starred first, then newest; date and time, seed, stages heard
  and for how long, the stages in a few words. Per entry: **Replay** (the recorded stages exactly, merged into songs a
  few stages at a time, each starting a stage early so seams arrive sounding; then, with the same composer, live on
  from its seed, checked stage by stage and recorded into the same entry; otherwise it says so and stops), **Save as
  song** (all or a range of stages, into `examples\`, where the Library shows it), **Star**, and **Delete** with an
  in-place confirm, moving the file to `history\deleted\`, with **Undo** for the last delete and **Empty deleted**
  behind its own confirm.
- **The ending, world side, against a stub.** When a stream ends on its own (an engine signal the composer's rewrite
  is adding), the last notes ring out, the camera lifts slowly back over the trail and holds, the history marks the
  entry a finished song with its length, and after a breath a new song starts from a fresh seed; View has the
  auto-next setting, and the ending card has **Replay this song**. The engine has no such signal yet, so the world asks
  `StubStreamEnded` (in `main.cpp`), which never reports an end unless `--stub-stream-end N` is given. Replacing that
  function's body with the engine's call turns it on.
- `--history-dir`, `--history-test [DIR]` (also run by `build.bat test` when the exe is built), `--replay-test FILE`,
  `--stub-stream-end N`, and `--capture-menu history | history-open | history-delete | ending`. 21 unit tests in
  `world/tests/history_test.cpp`: the format round-trips (whole and appended), replay text equals the recorded stage
  text, merging, delete and undo, sorting and stars, write failures reported and recovered.

### Changed

- **The floor is just water: no grid** (the grid on the water is removed; the floor is only water).
  `VoidTheme`'s `floorGrid` is 0 (was 0.45); `LM_THEME="floorGrid=0.45"` brings it back for a run.

- **Stream mode v2: every stage is a change you can hear, in arcs, under a budget.** Listening to v1 live:
  the equation did not change enough every eight bars, and its subtle changes stopped working as it grew more
  complex, though the bones were right. v1 changed a number a stage, mostly a
  melody phase by a quarter, and let the voices pile up.
  - **Arcs** (`plan_arc`): intro, one to three builds, peak, breakdown, turn, return (6-8 stages); the next arc opens
    thinned to three layers with a new melody generator, a modulation or a new progression. The stream opens on a low
    voice and a pulse and builds.
  - **Layer-level moves**: a layer enters or leaves; the melody's generator is replaced by another family's (the
    grammar's seven, now exposed as `grammar::generator`, plus two waves and residues), keeping its bar index and
    register; a new groove (the lead's `P` or a kick pattern `Q`, 5 or 3 of 16 steps different); a new progression
    (4 of 8 bars different); an octave; half or double time; a mode two or three steps away; a modulation; a breakdown
    to two layers; the drop, with a roll into it.
  - **Audibility, measured and required** (`src/compose/audible.rs`): layers in or out, pitch classes, onsets,
    density, register and the melody line, from the functions; a candidate below 1.0 (one layer entering) is not taken
    while one above exists, and the critic picks among those above. Over the same 250 changes v1 averaged 0.82 (68%
    below 1.0) and v2 averages 3.71 (median 3.40, none below).
  - **A complexity budget**: at most 6 layers and 13 definitions; a layer entering at the cap pushes another out
    (never the lead), and definitions nothing reads are tidied away.
  - Seams, tempo continuity and determinism per seed are as before; the stage's role, arc and audibility are on
    `Stage`. Breaking for anyone reading a chain: the same seed writes a different chain than v1, and v1's `Move`
    variants (`Mode`/`Roots`/`Rhythm`/`Melody`/`AddVoice`/`DropVoice`/`Fill`/`Revoice`/`Breakdown`/`Return`) are
    replaced; saved songs (`stream_s*.txt`) are ordinary songs and play as they were. Rolling back is reverting commit
    8db4c8b.
- **The live equation is compact by default; E cycles compact, full, hidden.** The equation took up too much
  space for what it conveys. Compact is one quiet strip along the bottom edge, at most
  a fifth of the screen high and never over the draw-head, with the definitions one line each in a dense block; when
  they do not all fit it keeps the changed lines, then the generators, then the voice lines, then the lens (the time
  names are not shown). 14 px type and 11% of the height at 1080p, 20 px and 18% at 150%. The engine's display lines
  now mark each definition `G` (generator or stored integer), `V` (voice line), `L` (lens) or `T` (time name) instead of
  `D`/`T`. `--capture-menu equation` captures the compact strip, `equation-full` the panel.
- **A new song every time.** Stream always played the same, already familiar song: the default stream was always G. **Stream** (and
  `--stream`) now starts the best grammar song from a seed drawn from the clock (1-999999), shown in the equation's
  header and printed so `--stream --seed N` plays it again; **Stream from G** (and `--stream-g`) keeps G and its pluck
  lead; R replays the stream playing instead of drawing a new seed.
- Measured: `--stream-test 60` all ok. The frame-time bench ran while another Dreamride window shared the GPU, which
  inflated every p99 (the untouched citypop control: 4.35 ms against 1.79 ms recorded); the compact strip sits inside
  that noise (p99 4.10 ms with it, 4.28 ms without, p50 0.61 ms both). To be re-run on a quiet GPU.

### Added

- **A corpus of human-made Desmos music, read for the composer** (`docs/research/desmos_corpus.md`, with
  `docs/research/desmos_corpus/`: 64 public graph states in `raw/`, their sources and authors in `sources.md`, and the
  scripts that re-derive the numbers). The grammar and critic were invented from a few examples and their output was
  judged well short of the hand-written songs; this records what well-received function music actually does (note data
  through hand-built instruments, a progression under a melody whose beat notes are 72-75% chord tones, falling-pitch
  kicks, spectral tilt per role, short loops played all-in) and lists specific grammar and critic changes. Research
  only: no code changed.

- **Stream mode: an endless song of eight-bar equations, written as it plays, with the equation shown live.** The
  composer's songs fell well short of the hand-written ones, and the idea that followed: show the equation in real time,
  writing an eight-bar equation, then stitching a new one after it, and so on, instead of a premade song. That is how @measure_plan plays a graph, and what DESIGN.md
  already promised (the future is not written yet). `--stream [--seed N] [--stream-song FILE]`, or the library's
  **Stream** entry.
  - **The composer as a chain** (`src/compose/stream.rs`). Stage 1 is the kept song `s7-c38` (the one keep from the
    blind test, with the pluck lead that was liked), a grammar seed, or any song, put in stream form: chord roots as eight
    digits `N`, the rhythm as a 16-bit integer `P`, the mode as a term `c`, voices silent in the first section held
    back. Each stage edits one or two things (mode, root digits, rhythm bits, one melody term, a voice in or out, a roll
    into the seam); every 4-8 stages a turn (a modulation term `K`, a new melody, a breakdown, the drums' return). The
    critic picks the best of six candidates and never returns to any of the last eight states. Deterministic per seed.
  - **Seamless streaming render** (`src/stream.rs`). A worker renders a stage ahead, carrying every voice's state
    across the seam and running one reverb, delay, wow and warmth through the stream; per-stage gain moves between
    stages over half a second. Two stages equal one long render to 1.5e-8 (f32 rounding); seams measure 0.11-0.68x the music's own
    roughness, against 4.3-14.7x for a renderer that drops its state.
  - **The C ABI**: `lm_stream_start`, `_pull`, `_stage` (equation markup, changed names, summary, song text),
    `_stage_at`, `_window` (the voice record of kept stages, read like a song), `_roster`, `_info`, `_save`, `_error`,
    `_free`.
  - **The live equation** (`world/src/math_layout.h`, E toggles): the playing stage's definitions typeset as math
    (stacked fractions, sized floor brackets and parentheses, raised exponents, italic names; Cambria from Windows), a
    stage counter and what changed, in a dark panel placed clear of the draw-head; the old equation crossfades into the
    new at each seam and the changed terms light up for about three seconds.
  - **Save what you liked** (S): the last four stages become one ordinary song in `examples\`
    (`stream_s<seed>_<a>-<b>.txt`), which the library picks up at once.
  - **Tests**: 21 in `tests/stream.rs` (the chain, every move against `diff`, seams with a control, one-long-render
    equality, saving, the markup, the C ABI and its null handling); 20 in `world/tests/math_layout_test.cpp` (parsing,
    fractions, brackets, scripts, line breaking, crossfade and highlight timing, panel placement, the Stream entry and
    saved names), run by `world\build.bat test`; and `--stream-test`, a silent run of the window's live path (no window).
  - Captures `world/captures/stream_*.png`; bench at 1080p with the equation drawn: GPU p99 0.77 ms, CPU p99 0.93 ms.
  - Twelve hand mutations of `src/stream.rs`, `src/compose/stream.rs` and `display.rs`: eleven killed, one equivalent
    (popping the predelay ring before or after pushing returns the same sample). Two survived the first run (`note_age`
    across a seam, the melody move editing the bar index inside `R`); the tests that now catch them were added.

- **The composer, first version** (`luminesce-compose`, `src/compose/`, `docs/composer.md`). The question was whether
  an algorithm could write songs like the three hand-written graph songs instead of hand curation. It generates, then
  judges.
  - **A grammar** of graph songs: seven generator families (Takagi-Landsberg fractal, square-wave sums, Lissajous ratios,
    Fibonacci mod m, primes and Collatz as digit tables, rule 30 bits), a scale lens with per-section modes, role
    mappings, four forms and the graph instruments. It writes complete, readable song files with a header that says
    what each voice reads.
  - **A critic** of 13 measured components with cited rationale and documented weights: melody range, steps, contour,
    repetition; Longuet-Higgins-Lee syncopation and density; Sethares/Plomp-Levelt roughness on the voices' own
    partials and root motion; section contrast, return and energy; and band energy and peak density of the engine's own
    render (a Rust port of `tools/measure/analyze.py`).
  - **A search**, deterministic per seed, that renders only the finalists, writes `generated/<UTC time>/` with
    `scores.md`, and appends to `generated/ratings.txt` for the author's verdicts.
  - **The falsifier was registered before the first run, and its first criterion failed.** The hand-written songs
    ranked above 76%, 39% and 76% of 200 random samples, against a registered 80%. Grammar and critic share their
    priors on form, and the hand songs don't build. The weaker test is kept, ignored, naming the record.
  - 24 tests in `tests/compose.rs`, one ignored as above. Eight hand mutations of the critic and the writer: seven
    killed, one equivalent.

- **The library refreshes itself.** A song saved, added, renamed or deleted in `examples\` or a `--songs` folder while
  Dreamride runs shows up in the Library without a restart, marked **new** until it is played. Composing in a Claude
  Code session is the loop chosen for composing, and a restart per song broke it. The folders
  are watched with Win32 change notifications (`world/src/folder_watch.h`), polled with a zero timeout each frame; a
  folder that cannot be watched is rescanned every 2 s. The scan runs on a thread 0.3 s after the last change (it
  measured 1-10 ms for ten songs, too long for a frame), and the merge keeps the playing, rendering, queued and
  failed songs pointing at the right rows. Picking the playing song after its file changed renders it again, and a
  failed song that is rewritten loses its old error. `ui::MergeRescan`, `ui::RemapIndex` and `ui::RescanClock` in
  `ui_logic.h`; 11 new tests in `world/tests/ui_logic_test.cpp`, one of them against a real watched folder.
- **A `compose` skill for Claude Code** (`.claude/skills/compose/SKILL.md`): how a fresh session writes a graph song
  without the conversation that taught it. The song language checked against the parser, the graph way as rules, the
  taste so far, what was liked and what was not (2026-11-02), the instrument kit, and the loop (write, render, measure
  against the reference range, fix, hand over), with a checklist. It was tested cold: a session with only the skill
  wrote `examples/graph_hanoi.txt`, and what it got wrong (no rule for a crowded 150-500 Hz band, a lead range that
  caused it, no way to find which voice owns a band, an example that used undefined names) was fixed in the skill.
- **`examples/graph_hanoi.txt`**: the Tower of Hanoi's move sequence (the ruler function, the trailing zero bits of
  k) as chord roots, lead, a gliding disc voice and the kick, with the mode walking Dorian, Aeolian, Ionian,
  Mixolydian by the digits of 3254. Written from the skill alone.
- **Dreamride's menu, keys and launcher** (the world's window; Dreamride is the program's name). It needed
  shortcuts and a menu to navigate more easily: the window played one song named on the command line, with
  Esc, Space and F11 only.
  - **A menu overlay** drawn with Dear ImGui (vendored in `world/src/imgui`, MIT): dark, translucent, Segoe UI,
    scaled by the monitor's DPI. It fades away 2.5 s into a song when nothing is touched, cursor included, and any
    mouse movement or key brings it back. A transport bar (title, description, time, seek bar, previous,
    play/pause, next, restart, loop, mute and volume); a menu (Tab or M) with **Library** (every `*.txt` in
    `examples/` and `--songs` folders, each with its first-line description), **View** (follow or pull-back camera,
    fullscreen, a faint HUD) and **Theme** (bloom, fog density, floor roughness, glow, camera breathing and dust,
    live, with Reset); a keys card (H); a quit question.
  - **Songs render off the main thread.** The window opens at once and the song renders on a loader thread (about
    5 s for citypop_v4) while the current one keeps playing, then swaps in between frames. Asking for another song
    meanwhile queues it (the latest ask wins); one that fails to load says why, in the library and the transport.
  - **Keys**: Space play/pause, R restart, ←/→ seek 5 s, ↑/↓ volume, N/P next/previous song, L loop, Tab or M
    menu, H keys, F11 fullscreen, Esc closes the card, then the menu, then asks before quitting. The mouse wheel
    over the seek bar scrubs 2 s a notch.
  - **Seeking.** A seek starts a new XAudio2 voice at that sample (the rest of the song, then the whole song looped
    behind it when it repeats), and the picture jumps with it. It stays within the current loop, so the trail
    behind is the song's own record, the previous loop included once the song has wrapped. **Loop** on repeats the
    song; off, the song holds at its end and the library's next song renders and plays.
  - **The camera's pull-back** (View): it lifts up and back over a second, with the fog thinned, to show the trail
    the song has left (DESIGN's "Pull-back").
  - **Theme**: `fogDensity`, `glow` and `breathe` (all 1 in the void theme, so it looks as before), also settable
    with `LM_THEME`.
  - **A desktop shortcut**, `Dreamride.lnk`, which opens on the library (`--library`), made by
    `world/tools/make_shortcut.ps1` (it will not replace an existing one without `-Force`). Its icon,
    `world/assets/dreamride.ico` (a glowing line rising to a draw-head over a dark floor, drawn by
    `world/tools/make_icon.ps1`), is embedded in the exe through `world/src/world.rc`.
  - **Harness**: `--capture-menu PANEL SECONDS OUT.png` draws the overlay into a headless capture (transport,
    library, view, theme, help, confirm or loading), `--ui-scale` sets its scale, `--pull-back` the camera,
    `--bench-ui PANEL` benches with it, and `--seek-test` seeks a silent voice and checks where the audio lands.
    Still no window opens in any of them. At 1080p on an RTX 5080, with the library panel drawn every frame, the
    GPU frame averaged 0.80-0.83 ms (p99 1.83-1.91 ms) against 0.82 ms (p99 1.87-1.89 ms) without it.
  - **Tests** (`world/tests/ui_logic_test.cpp`, `world\build.bat test`; the world's first, 28 tests): the song
    library's scan and first-line descriptions (CRLF, BOM, empty files, missing folders, duplicates, non-ASCII
    names, every repository example has one), the key map (every key, repeats, no key bound twice, the keys card
    covers every key), seek clamping, volume steps, song index wrapping, time format, and what Esc, Tab and H do.
    `--seek-test` caught, before it shipped, that with loop off the picture jumped back to the last seek point at
    the song's end (XAudio2 resets its position count when a stream ends); the end now holds.

- Three graph pieces, `examples/graph_takagi.txt`, `graph_lissajous.txt` and `graph_pisano.txt`, written the graph way
  round. Listening tests found citypop_v4 close to an average MIDI file in a DAW: it composes conventional music (note
  lists, chord charts, a performance layer) and then writes it as functions. These start from one generator each, and a
  scale lens makes it familiar: the Takagi fractal curve read at four time scales (form, phrase, bar, beat); a Lissajous
  figure whose ratio p:q is also the drone's just interval and the hats' and claps' rates; the Fibonacci numbers mod m,
  whose Pisano periods set the voices cycling against each other and answer each melody with its inversion. Each fits on
  one screen (11 to 16 short definitions, no hand-placed notes), uses only graph instruments, and has shapes a DAW would
  not draw (glides that follow the curve, drums as pitch dashes). No language change was needed. Measured with
  `tools/measure`, all three sit in the reference recordings' range (README, "The examples").
- Tests (14 more, 202 in all, `tests/example_graph.rs`): the mode lens `floor((12d + c)/7)` walks the seven modes one
  sharp at a time; the glide lens holds each scale tone and slides monotonically to the next; the Takagi roots are the
  curve and mirror round its middle, and its curve-driven kick lands only on beats but not on all of them; the
  Lissajous drone interval is 12 log2(p/q) within 0.001 semitone, its hats and claps run at p a beat and q a bar, and its
  lead stays in each section's mode (this test caught the lead transposed out of the drone's key while writing it);
  Binet's formula is exact through F(40); the Pisano melody and form invert every half period and the kick is a 3-3-2
  in every bar; every tonal voice that should be on the scale is, at every step; all three render finite and exact.
- The world, stage W2. W1, once run, called for a sharper reflection and a more expressive picture, so the
  picture now answers the music. Still within the void theme, and its look now lives in parameters a later theme
  can change.
  - **A near-mirror floor.** Reflections are sharp enough to read single lines: roughness 0.0035 instead of 0.02,
    and four reflection rays per pixel at fixed sub-pixel offsets, so thin lines are antialiased in the mirror
    without shimmering under motion. The gloss blur fades out below a pixel. A subtle Fresnel (F0 0.3) and a faint
    grid remain.
  - **Note onsets flare:** the new segment and its draw-head. Draw-heads follow the live level. Loudness is tube
    thickness. Colour is tinted by pitch class on the circle of fifths, so a chord change shifts the tint.
  - **Drum dashes at their true size** where the sweep fits the wall. The kick is a 2.6 m streak falling about
    three octaves, as the reference draws it; it was squeezed to 0.9 m. A hit flashes light into the fog (closed-form
    single scattering) and onto the floor. Each kick sends a slight ripple through the floor's reflection.
  - **The camera breathes with the song.** It is closer and lower in quiet parts and back and up in dense ones, set
    by an energy curve measured from the trace and smoothed twice so it never lurches, with a few-centimetre sway on
    the beat.
  - **Dust drifts in the graph's light,** receding with the past. The present glows faintly in the fog.
  - **`Theme`** (`world/src/main.cpp`, `VoidTheme()` the original) holds the palette, fog, floor finish, every
    response and the camera poses. `LM_THEME="name=value ..."` overrides them for a run, and an unknown name or a
    bad value is an error.
  - **`--bench-from SECONDS`:** bench any part of the song.
  - Captures are `world/captures/w2_*.png` (not committed), including the instant of a kick (`w2_15s_kick.png`,
    `w2_33s_dense_kick.png`).
- The world, stage W1 (`world/`): a native Direct3D 12 + DXR host that plays a song and draws it, the original theme
  (the void). Native was chosen over the WebGL plan, to match Dreamzone-RT. Each tonal voice is a glowing tube
  along its pitch curve while its gate is open. Each drum hit is a short downward dash: its own pitch sweep, squeezed
  to under a metre and ending where it settles. The camera rides with the present, off to the side and looking back.
  Nothing ahead of the present is drawn, and the past fades into fog whose length follows tempo. A dark glossy floor
  mirrors the tubes through DXR inline ray queries (one mirror ray per pixel, the gloss as a blur sized by hit
  distance, so no sampling noise under motion) and takes their light (a closed-form line-light integral, every
  frame). Also HDR, 8x MSAA with a luminance-weighted resolve, a minimum one-pixel tube width, a 13-tap bloom, ACES
  and dithering. Presentation is vsync-locked to the display. The playback position (XAudio2) is the master clock,
  smoothed between the audio engine's 10-20 ms steps. Measured silently: within 6 ms of the audio, speed within 1%
  of real time, never backwards.
  Headless harness, which never opens a window: `--capture SECONDS OUT.png` (repeatable) and `--bench [FRAMES]`. At
  1920x1080 with 8x MSAA on an RTX 5080, citypop_v4 averages 0.91 ms a frame (p99 1.96 ms).
- A C ABI for the engine (`src/ffi.rs`, built as `luminesce.dll`; header `world/src/luminesce.h`), so the world
  loads a song, plays the engine's own buffer and draws from the same numbers. `render_traced` records, while
  rendering, every tonal voice's gate, pitch, amp and level every N samples, plus every drum onset and velocity.
  The traced mix is bit-identical to `render`'s.
- Tests (11 more, 188 in all, `tests/ffi.rs`):
  - the trace against an independent evaluation of each voice's expressions, note time and documented envelope;
  - the traced mix against the plain one, bit for bit;
  - the level as a bound on what the voice put in the mix;
  - drum hits as the trigger's rising edges;
  - `lm_sample`'s frame choice and range;
  - the audio buffer;
  - every C entry point with null pointers, bad indices, bad UTF-8, missing files and short error buffers;
  - citypop_v4 through the file entry.

### Changed

- The world's window is titled **Dreamride** (and the song playing), and the window, the menu and the shortcut use
  that name; the folder, the repository and `luminesce-world.exe` keep theirs until the rename. Esc no longer quits
  at once: it closes what is open, then asks "Quit Dreamride?", and a second Esc quits (any other key stays). The
  window is per-monitor DPI aware, so on a scaled display it renders at the display's real resolution instead of
  being stretched (more pixels to draw there). The song geometry builders take the song and theme they build from,
  so a song can be built on the loader thread; what they build is unchanged.
- The world's floor light uses half-beat segments for the past older than four beats; the last four beats keep
  eighth-beat segments. Both are on the song's beat grid, and no segment spans a note change. The 1/8-beat floor
  light was 1.3 ms of the frame in citypop_v4's dense part, where a frame took 2.1 ms on average (p99 3.06 ms with
  W2's additions). That part now takes 1.05 ms (p99 2.15 ms) at 1080p on an RTX 5080. The older floor light is
  fogged and dim, so the coarser segments do not show. To roll back, set `coarse` in `BuildSegments` to 0.125.
- Voices render on up to 8 threads into their own buffers, summed in voice order with the same operations, so the
  mix is bit-identical. All six examples' float WAVs match the bytes rendered before the change. citypop_v4 renders
  in 5.4 s instead of 22 s, which the world pays at every launch. Peak memory rises by about 9 bytes per sample for
  each voice in flight (about 140 MB for a 41 s song). To roll back, revert `render_with` to a sequential loop: the
  output does not change.
- The crate is now `crate-type = ["rlib", "cdylib"]`, so `cargo build` also produces `luminesce.dll`.

- Graph instruments, because listening tests found v3 did not sound like graph music, only a synth on a graph: in the
  reference videos the whole sound comes out of the graph (Desmos `tone()` sines added together), and the kick is a fast
  downward pitch dash.
  `wave = additive` with `partials = <dB per harmonic>` sums sine partials at whole multiples of the pitch;
  `rolloff = <Hz>` (on sine and additive voices) scales each partial by min(1, rolloff / its frequency), the Desmos
  gain cap `min(10, 660/f)`; `kind = dash` is a drum whose sine (and optional harmonics) falls as
  `to + (from - to) e^(-tau/dash)`, with its phase in closed form, a hold, a decay and a length. Partials at or above
  Nyquist are left out, so nothing aliases. Existing waves and drums are unchanged, and every existing example renders
  byte-identically (checked against renders taken before the change).
- `examples/citypop_v4.txt`: v3's music, step for step, revoiced with graph instruments and rebalanced against
  measurements of the two reference recordings (`docs/research/graph_sound.md`). Band energy (<150, 150-500, 500-2k,
  2-6k, >6k Hz) 50/28/21/1.1/0.1 % against the references' 41/37/20/1.6/0.2 and 60/26/13/0.9/0.3 (v3 was
  67/10/20/2.3/0.7); spectral peaks per frame 84 (sections 67-92) against v3's 57. v3 is kept unchanged for A/B.
  Musical changes beyond the voices: the chord root added as a fifth pad tone, a pad layer an octave up, v3's
  breakdown filter riser carried as a pad layer two octaves up that fades in, and a 30 ms scoop instead of 80.
- `tools/measure/`: the Python scripts that measured the references and our renders (dev only, need numpy).
- Tests (17 more, 177 in all): partial frequencies and levels, a 0 dB single partial equal to the sine, the
  roll-off's ratios, no alias above Nyquist, the dash's frequency and phase at set times, its hold and decay, its
  harmonics left out at Nyquist, field checks; and v4's claims (v3's notes, chords, bass and drum patterns step for
  step; graph-only voices; finite and exact length).

- An expression layer, because listening tests found v2 structurally right but without emotion: correct
  music needs a performance. Expressive built-ins in the language (`src/expressive.rs`):
  `arch` (Todd's phrase arch, the KTH Phrase rule), `phrase` (a phrase-time warp whose tempo
  follows the arch and keeps every phrase its written length, so loops stay exact and notes never
  reorder), `ritard` (the Friberg-Sundberg final ritard), `charge` (KTH melodic charge),
  `vibrato` (delayed and growing), `db` and `smooth`.
- Note time: `note_age` and `note_left` (seconds) in a voice's pitch, amp and cutoff, so a pitch
  can scoop into a note, grow a late vibrato, fall at its end, or sag as it runs out. The renderer
  now evaluates each voice's gate first to know where every note ends. Gates and triggers may not
  read them (an error), and definitions that read them are never cached.
- `detune` per voice: two oscillators a given number of cents apart.
- Send effects: `reverb` and `delay` sends per voice and drum; Freeverb (written from its published
  structure and tuning), a tempo-synced ping-pong delay with a darkening feedback, tape `wow` on the
  whole mix, and `warmth`. New song settings for each. With `loop 1` tails wrap round the loop and
  wow completes whole cycles, so the file loops seamlessly; the length stays exact either way.
  Songs that use none of this render byte-identically (checked for the sketch, v2 and dark).
- `examples/citypop_v3.txt` (wistful nostalgia) and `examples/dark_v3.txt` (defiance with a crack
  of grief), each naming its feeling at the top and the choices that serve it: appoggiaturas at
  peaks, a withheld tonic, a borrowed minor iv with a saved climax note, a sudden shift to the
  relative major, a held breath before the return, and every lead note shaped by its phrase. v2 is
  kept unchanged for comparison.
- Tests (37 more, 160 in all): the arch's shape and peak, phrase time keeping boundaries and order
  and lengthening phrase ends, the ritard, the charge table, vibrato onset and depth (including a
  zero ramp), note time restarting per note and counting down to the gate, note-reading
  definitions not cached, detune beating at the right rate, the reverb decaying and staying finite
  at full size, the delay landing on the beat to the sample (left then right), predelay shifting
  the room exactly, looped tails wrapping, wow's depth in cents, range checks; and the v3 files'
  musical claims (appoggiaturas resolve down by step, the climax and the borrowed note come only
  where stated, the tonic is withheld until bar 15, lengthened notes never run into the next, the
  held breath is silent but for the room, D# always pushes up to E, the crack is the only major
  moment and the drums stop for it).

- Two new examples, because the first sketch worked but did not sound like music that knows
  what is hard: `examples/citypop_v2.txt` (16 bars, F major, swung: a kick-and-bass groove with
  ghost notes, colour chords voiced rootless and sorted, approach notes, a call-and-answer hook
  with anticipations and space, a form with a breakdown, riser, return and fill, and a sidechain
  duck read from the kick pattern) and `examples/dark_halftime.txt` (16 bars, E harmonic minor,
  144 bpm half-time: an 808 with punch and slides, hat rolls as a rate function, one repeated bell
  motif, an 808 drop-out before the loop). `tests/example_music.rs` checks their musical claims
  from their own functions (voices never cross, the lead is on chord tones on every beat it
  sounds, approach notes approach, the 808 slides without re-striking, rolls only at phrase ends).
- Envelope decay and sustain (`decay`, `sustain`), so plucks and pads can breathe instead of
  holding at full level like an organ. Defaults reproduce the step-1 envelope exactly.
- A resonant low-pass per voice (`cutoff` as an expression of t, `resonance`, `filter_env`,
  `filter_decay`): a TPT state-variable filter (Zavalishin; Simper's form), exact -3.01 dB at the
  cutoff and stable under per-sample modulation, so sweeps and risers can be written as functions.
- `drive`, a normalised tanh saturation per voice.
- Comparison operators `< > <= >= == !=` (1 or 0, NaN in gives NaN out, no chaining) and a lazy
  `if(c, a, b)`, so musical rules (pattern codes, holds, "is this a chord change") read as rules
  instead of `min`/`max` tricks.
- A result cache for definitions that take parameters and do not depend on t. Pattern and chord
  helpers are called with the same step number for thousands of samples; the cache makes the new
  examples render about 4x faster than real time instead of slower than real time. Values are
  bit-identical with or without it, and the step-1 example still renders byte-identically.
- Tests: -3 dB at the cutoff (100 Hz to 15 kHz), 12 dB/octave, the resonance peak, a fast swept
  resonant filter stays finite, the filter envelope opens and closes, decay to sustain and release,
  drive keeps the peak, PolyBLEP alias energy against a naive saw (14.1 dB less), comparisons,
  `if`, the cache (including a function that reads t only inside an `if` branch).

- The headless engine, build step 1 of DESIGN.md, so that songs written as functions of time can
  be heard before any window or live playback exists. Standard library only.
  - An expression language: numbers, `t`, `pi`, `tau`, `+ - * / ^`, unary minus, parentheses,
    implicit multiplication (`12d`, `u^2(3 - 2u)`), the built-ins floor, ceil, frac, mod (floored,
    as in Desmos), min, max, abs, sign, sin, cos, sqrt and clamp, and user definitions with up to
    8 parameters. Parsed once, resolved to indices, constant-folded, evaluated per sample. Errors
    give the line and column.
  - A plain-text song format: `bpm`, `beats`, `rate`, `reference`, definitions, `voice` blocks
    (sine, triangle, saw, square; pitch in semitones from A4 = 440 Hz; gate; amp; pan; attack and
    release) and `drum` blocks (kick, snare, hat on the rising edges of a trigger).
  - Synthesis with PolyBLEP saw and square, click-free linear envelopes, deterministic drums
    (seeded SplitMix64 noise), equal-power panning, and normalisation to -1 dBFS when a mix would
    clip, reported rather than silent. NaN or infinite values stop the render with the voice and
    beat.
  - A WAV writer (stereo, 16-bit PCM or 32-bit float), writing through a temporary file.
  - `luminesce-render <song.txt> <out.wav> [--float]`, printing duration, sample count and peak.
  - `examples/citypop_sketch.txt`, an eight-bar city-pop loop of our own using h, root, fold,
    swing and smoothstep, and its render.
