# The composer (first version)

The question, 2026-11-02: could an algorithm make songs like the hand-written examples on its own, instead of hand
curation? The three `examples/graph_*.txt` songs were written by hand; each grows
from one generator through a scale lens. This composer writes songs like them on its own: **generate, then judge.**

```
cargo build --release
target/release/luminesce-compose --seed 7 --count 200 --keep 5 --out generated --wav \
    --compare examples/graph_takagi.txt examples/graph_lissajous.txt examples/graph_pisano.txt
```

Code: `src/compose/` (`grammar.rs`, `critic.rs`, `sound.rs`, `search.rs`, `rng.rs`), CLI `src/bin/luminesce-compose.rs`,
tests `tests/compose.rs`. Standard library only, like the engine.

## Registered before the first run (written 2026-11-02, before the critic had scored any hand-written song)

The room's practice: say what would count as working and as failing before running it.

**The composer works if all of these hold:**

1. **The critic prefers the hand-written songs.** With every candidate of the registered run rendered
   (`--seed 7 --count 200 --render-all`), each of the three hand-written graph songs scores above **80%** of the random
   grammar samples by total score. *Fails* if any of the three is at or below 80%. *Fails hard* if any is below the
   median, which would mean the critic cannot tell the hand songs from noise in the grammar.
2. **A listener keeps the composer's songs blind.** The top 5 of a run are mixed with the three hand-written graph
   songs, renamed so neither the listener nor the files say which is which, and the listener marks each keep or skip by
   ear. *Works* if the listener keeps **at least 2 of the 5** composed songs. *Fails* on 0 or 1. (A stronger form, not required: the composed
   songs' keep rate is at least half the hand songs'.)
3. **The machinery is sound.** Every generated file parses and renders with no NaN; a seed always writes the same
   songs; the critic's maths passes its known-answer tests (`tests/compose.rs`).

**What a pass would not show.** Criterion 1 is weak evidence on its own: the critic was designed by someone who had read
the hand songs, so it may encode their habits (one generator, a lens, graph instruments) rather than musical quality.
Criterion 2 is the outside: only a listener's ears can say whether a top-ranked song is music. If 1 passes and 2
fails, the critic is measuring familiarity with the hand songs, not what is heard, and the next step is learning its
weights from the ratings (below), not tuning it by hand.

**What I expect, so the surprise is measurable:** criterion 1 passes for Takagi and Pisano and is closest to failing for
Lissajous (its sections each change ratio, so nothing returns bar for bar, and its lead is wide). Criterion 2: 2 of 5.

## The registered run: what happened (2026-11-02, after the registration above)

`--seed 7 --count 200 --keep 5 --render-all --wav`, compared against the three hand songs: 200 candidates written, scored
and rendered in 112.5 s on 16 threads (the function scoring alone takes under a second; the renders are the rest).
Folder: `generated/20261103-004208/`, with its `scores.md`.

**Criterion 1 failed.** Each hand song was to score above 80% of the random samples by total score:

| hand song | total | above this % of 200 samples (total) | above this % (function score only) |
|---|---|---|---|
| graph_takagi | 0.785 | 76% | 44% |
| graph_pisano | 0.784 | 76% | 46% |
| graph_lissajous | 0.725 | **39%**: below the median, the "fails hard" case | 10% |

The samples' function scores ranged 0.58 to 0.94 (median 0.77, 80th percentile 0.82). The kept five scored 0.84 to 0.90 by
total. My expectation (Takagi and Pisano pass, Lissajous closest to failing) was right about the order and wrong about
the level: none passed.

**Why, read from the components** (scores.md has every number):

- **The grammar and the critic share their priors, so the random samples are not random with respect to the critic.**
  The grammar's forms (a layer entering each section, a break and return) were written from the same sources the
  critic's structure components cite (prior_art.md 5, genres.md 3.3). The samples meet the critic's idea of form by
  construction. The hand songs keep almost every layer on from bar 1, so they lose on **structure energy** (Takagi
  0.22, Pisano 0.00, against a sample median of 0.66). The registration named this as the way a pass could be hollow.
  What happened is the mirror of it: the shared priors made the hand songs *fail*.
- **The hand songs win on sound.** By band energy and spectral peak density, measured on the engine's own render
  against the reference recordings, the hand songs score 0.97–0.98 (bands) and 0.87–1.00 (peaks). The five kept songs
  score 0.61–0.93 and 0.16–0.74: they count 67–83 peaks per frame against the hand songs' about 110 and the references'
  108–143. That is why the hand songs move from the 44th percentile (functions) to the 76th (total).
- **Lissajous is built not to repeat.** Each section changes the ratio, so its melody repetition (0.16) and its return
  (0.00) score low, and its wide lead (19 semitones) loses on range. The critic reads "nothing comes back" as a fault;
  for this song it is the idea.
- **The weighted sum lets a dead component through.** Kept song 3 (s7-c178) scores **0.00 on melody steps** (all
  leaps, no steps) and still ranks third. A rule that a song must clear a floor on every component would have stopped
  it. Not changed in this version, so that the registered critic stays the one that ran.

**What this means.** The critic as built measures "follows the grammar's own recipe" about as much as "sounds like the
hand songs", and it does not yet prefer the songs the listener called good. Criterion 2 (the listener's ears, blind)
is still open and matters more. If 2 of the 5 are kept, the grammar is making music and the critic is ranking within it without
knowing why. If fewer are kept, the critic needs the ratings before it can be trusted.

**Changes made after the registration.** One, made before the registered run but after a diagnostic had printed the
hand songs' function scores: `tone_roughness` was changed from "sum the partial pairs across two tones" to "the
roughness the two add to each other's spectrum", because the unison test showed two identical harmonic tones scoring
rougher than an octave (each tone's own neighbouring harmonics were counted again). It left the hand songs' roughness
scores unchanged to two decimals. Nothing else was tuned. The test
`the_hand_written_graph_songs_score_above_the_median_random_sample` encodes criterion 1's weaker form; it fails for
all three songs by function score, and is kept in the suite as an ignored test that names this record, so
`cargo test --test compose -- --ignored` re-runs the claim. It is not deleted, and not relaxed until it passes.

## The grammar (`src/compose/grammar.rs`)

A seed and a candidate index give a `Choices` (SplitMix64, so a seed always writes the same songs on any thread count).
`write` turns it into a song file of 7 to 12 definitions, up to 7 time names and 9 to 13 voices, with a header comment
that says in words what every voice reads. A second pass moves the lead by octaves until its median sits between A4
and A5.

| Family | Generator | Roots `R(i)` (bar i) | Melody `M(k)` (lead step k) | Parameters sampled |
|---|---|---|---|---|
| fractal | Takagi-Landsberg `T(u) = sum w^k tri(2^k u)` | `floor(a T(i/16))` | root + `floor(sum a_j T(k/S_j))` at 3 of 5 time scales (4 bars to 1 beat) | w in {0.5, 0.6, 0.7}, 5-6 terms, a, scales, amplitudes |
| squares | `sq(v) = floor(2 frac v)`, square waves (Walsh-like) | two squares 2-16 bars long | root + 3-4 squares, periods 2-64 steps, coefficients -3..4, phases | periods, coefficients, phases |
| lissajous | `X = sin(p tau x/16)`, `Y = cos(q tau x/16)`, p:q superparticular | `floor(a Y(4i) + 0.5)` or a drone | root + `floor(m (X + Y) + 0.5)` | ratios climbing (2:1..5:4 or 3:2..6:5) or falling, a, m |
| fibonacci | Binet's `F(k)` mod m, period the Pisano period | `sg(F(i) mod m)` | root + `sg(F(k) mod m)` | m in {5, 7, 8} |
| primes | 15 primes mod m, stored as the digits of one integer | `sg(D(i + o))` | root + `sg(D(k))` | first prime, m, offset |
| collatz | a Collatz trajectory mod m, as digits | as primes | as primes | start (a trajectory of 10+ steps), m |
| rule30 | rule 30's centre column, 48 bits as one integer | 3 bits read as a number | root + 3 bits read as a number | kick offset |

Shared choices:

- tempo 88–112, and the key;
- **the lens** `h(d) = floor((12d + c) / 7)`: fixed (Aeolian to Lydian), brightening a sharp a section, darkening, or a
  mode per section from four digits;
- **the lead** on eighths or sixteenths, plucked (gate 0.5, 0.7 or 0.85 of the step) or legato, with or without a
  glide; rests none, Euclidean E(k, 8), or the family's own mask;
- a harmony a diatonic third or fifth above, or none; an arpeggio; 3 or 4 pads in thirds (for Lissajous, a just-ratio
  drone pair instead); a contour voice (continuous families only);
- **the bass** on the beat, walking root, third and fifth by the generator, or in octaves;
- **the kick** on every beat, on 1 and 3, Euclidean E(3, 5 or 7, 16), where the melody falls, or from the generator's
  sequence; **the clap** a tonal falling line or a dash on 2 and 4, the family's own (q a bar), or none; **the hats**
  on eighths or sixteenths, loudness from the generator;
- **the form**: four sections of four bars, each layer on or off per section by four digits (`lay(0111)`), from four
  templates (build; build, break, return; the hand songs' all-in shape; an arch).

The instruments are the hand songs' graph instruments, measured from the references.

## The critic (`src/compose/critic.rs`, `src/compose/sound.rs`)

It reads the song's own functions on a grid of 48 points a beat. A tonal voice gives **events**: a gate rise, or a new
pitch that holds within half a semitone for a sixteenth (so a legato line that glides from note to note still has
notes). The lead is the voice named `lead`, or else the melodic voice with the most events; the bass is `bass`, or else
the lowest. Voices named kick, snare, clap, hat, perc or rim are percussion. Each measurement becomes a score in 0..1 by
a stated band; the total is the weighted mean.

| Component | Weight | Measures | Target (score 1) | Why |
|---|---|---|---|---|
| melody range | 0.05 | 5th to 95th percentile of the lead's pitches, semitones | 9-17 (0 at 4 and 26) | a hook spans about a 10th (genres.md, melody) |
| melody steps | 0.06 | share of the lead's moves of 1-2 semitones, scaled down past 50% repeated notes | 0.5-0.9 | pitch proximity: small intervals dominate melodies (Vos and Troost 1989; Huron 2006, ch. 5) |
| melody contour | 0.06 | share of leaps of 5+ semitones followed by a turn; the up/down balance | turns 0.55+, ups 0.4-0.6 | post-skip reversal (von Hippel and Huron 2000); the melodic arch (Huron 1996) |
| melody repetition | 0.08 | mean over bars of the best match with an earlier bar (sixteenth slots, pitch and onset) | 0.45-0.8 | repeat more than feels safe, but not a one-bar loop (Margulis 2014; genres.md 3.4) |
| rhythm syncopation | 0.12 | Longuet-Higgins and Lee per bar, summed over the drum and bass streams | 2.5-9 (0 at 0.5 and 18) | medium syncopation maximises groove (Witek et al. 2014); the band from canonical patterns: four on the floor 0, son clave 4, every offbeat sixteenth 15 |
| rhythm density | 0.08 | events a beat, all voices | 3-11 | convention (a sixteenth hat alone is 4) |
| harmony roughness | 0.12 | Sethares' Plomp-Levelt model on the voices' actual partials, twice a beat | a major triad scores 1, a C-C#-D cluster 0, both in the pad timbre | consonance as measured roughness (Plomp and Levelt 1965; Sethares) |
| harmony roots | 0.08 | the bass's pitch class at each bar: mean plausibility of its changes | fourth up 1, fifth up 0.85, step 0.75, third 0.65, semitone 0.5, tritone 0.25; under 2 changes at most 0.4 | root-motion frequencies (de Clercq and Temperley 2011; Rohrmeier and Cross 2008), from memory |
| structure contrast | 0.07 | neighbouring sections: layers, bass pitch content, lead register, density | 0.3+ | contrast in two or more dimensions (genres.md 3.3, rule 10) |
| structure return | 0.06 | best bar-for-bar match of lead and bass between sections two or more apart | 0.55+ | the return of a cycle (prior_art.md 5); AABA |
| structure energy | 0.07 | a build (first section at most 0.9 of the peak) and the swing between sections | build, swing 0.15-0.6 | build-up and drop were the most pleasurable parts for dancers (Solberg and Jensenius 2017) |
| sound bands | 0.09 | band energy <150, 150-500, 500-2k, 2-6k, >6k Hz of the engine's render | inside the two reference recordings' range per band, Gaussian in log distance outside | graph_sound.md |
| sound peaks | 0.06 | median spectral peaks per 8192-sample frame within 40 dB, 80 Hz - 8 kHz | 108-143 | graph_sound.md |

`sound.rs` is `tools/measure/analyze.py` written in Rust (radix-2 FFT, Hann frames). On `graph_takagi.txt` it gives the
documented figures within 3 points per band and 10 peaks (`tests/compose.rs`). One difference: band energy averages
frames rather than taking one FFT of the whole file.

**Not cited, chosen by me:** the edges of every band where no source gives a number (range, density, contrast, return,
energy swing), the weights, and the scaling past 50% repeated notes. Witek et al.'s own syncopation index has
instrument weights (their supplement, Text S2, was not read), so their 0-81 scale does not transfer; ours is plain LHL
with Fitch and Rosenfeld's weights.

## Search and CLI (`src/compose/search.rs`)

Write and score all N from their functions (threads; under a second for 200), sort, render the best `--finalists`
(default 20, or all with `--render-all`) for the sound score, keep the best K by total. The run folder gets the kept
songs as `<rank>_<id>_<family>.txt` (and `.wav` with `--wav`) and a `scores.md`: every component of every kept song,
the hand songs' rows and percentiles when `--compare` is given, and per pick its strongest and weakest components, its
description and its parameters. The folder is named by UTC time (std has no time zone). The songs and scores depend only
on the seed.

## The listener's ears close the loop: ratings

`generated/ratings.txt`, one line per song: `<id> <keep | skip | 1-5> [note]` (the id is `s<seed>-c<index>`). Each run
appends a comment with its kept ids. `search::parse_ratings` reads it and rejects a malformed line by number.

**The blind test for criterion 2** (not built as a command): copy the five kept WAVs and the three hand songs' WAVs into
one folder, rename them A to H in a shuffled order, keep the key elsewhere, and have the listener write `A keep` ... `H skip`.

### Learning from ratings (the plan, not built)

Each song already has a vector of 13 component scores. With the ratings:

1. **Pairwise first (Bradley and Terry 1952).** Every keep/skip pair, and every pair of different star ratings, is a
   comparison "a over b". Fit weights w so that P(a over b) = sigmoid(w . (x_a - x_b)): a logistic regression on
   differences, with no intercept. It needs no calibration of one listener's stars against each other, and 8 rated songs already
   give up to 28 pairs.
2. **Keep the critic's weights as the prior.** Fit with an L2 penalty pulling toward the current weights. With a few
   dozen ratings it can only move the weights a little, which is right: a handful of ratings should not erase the
   literature.
3. **Measure it out of sample.** Leave one song out, refit, predict it, and report the pairwise accuracy against the
   current critic's. Adopt new weights only if they predict held-out pairs better. Register that threshold before
   fitting.
4. **Then learn what the components miss.** Ratings the components cannot explain point at a missing measurement, not
   at a weight. The first candidates from this run: a floor on every component (a dead melody should not pass), and a
   loudness-based energy curve from the render rather than from gains.

It is Newton's method on 13 numbers, well within std. It waits only for ratings.

## What I am unsure of

- Whether any of the five kept songs is music to a listener. Nothing above measures that.
- The bands I set without a source (above). They are guesses stated as numbers, and the ratings are what should replace
  them.
- `structure energy` reads voice gains as loudness, but gains are not comparable across instruments: a dash hat has gain
  1.3-2 because the cap makes it quiet. A loudness curve from the render would be better.
- Roughness ignores the envelope (attack, decay) and the note-time parts of `amp`. It is a sonority measure.
- Three of the five kept songs are from the square-wave family. That may be real (square sums make stepwise, repeating
  lines) or a bias in how the critic sees them.

## Stream mode: a chain of eight-bar equations (`src/compose/stream.rs`, `src/stream.rs`)

Listening to the composer's songs, 2026-11-02: it works, but falls well short of the hand-written songs. The idea that
followed: show the equation in real time and work like the original source, writing an eight-bar equation, then
stitching a new one after it, and so on, instead of a premade song in the conventional sense. In @measure_plan's videos the equation sits under the graph with a
stage counter, and each stage edits or adds a term. Stream mode makes the composer work that way, and makes DESIGN.md's
line literal: the future does not exist yet, because the composer writes it as it plays.

**Version 2 (2026-11-03).** Listening to v1 live: the equation did not change enough every eight bars, its subtle
changes stopped working as it grew more complex, though the bones were right; and the one song it generated was the
already familiar kept song. v1 changed one number a stage (most
often a melody phase by a quarter) and let the voices pile up, and its default seed was always G, the song already
known. v2 keeps the bones (stream form, seamless seams, the state carried, the C ABI) and replaces the moves, the order
and the default.

**The seed.** The library's **Stream** (and `--stream`) is the best of six grammar songs from a seed drawn from the clock
each time it starts (1 to 999999, shown in the equation's header and printed, so `--stream --seed N` plays it again).
**Stream from G** (`--stream-g`) starts from G, `s7-c38` (kept for its odd simplicity and its high plucked lead), embedded
as `src/compose/seeds/kept_s7-c38_squares.txt`; `--stream-song` from any song file. The seed is put in **stream form**
(`normalise`), each step only where its pattern is found:

- `lay(dddd)` section switches go (the voices that waited for a section wait in the stash).
- `h(d) = floor((12d + 5) / 7)` becomes `c = 5` and `h(d) = floor((12d + c) / 7)`.
- `R(i)` becomes eight digits: its values on bars 0-7, offset into 0..9, stored as `N` and read back by
  `R(i) = mod(floor(N / 10^(7 - mod(i, 8))), 10) - 5` (the reference's chord-root trick). `s7-c38` gives `N = 57465746`.
- `on(k)` becomes a 16-bit integer `P` read by `mod(floor(P / 2^mod(k, 16)), 2)`.
- The lead takes the pluck: partials `0, -4, -6, -12, -10, -16, -15, -21`, attack 5, decay 300, sustain 0.55, release 150.

Then it **opens** (`opening`): one low voice (the bass, else the pads, the drone, ...) and a pulse (the hats, else the arp
or the kick), the rest in the stash. G opens on its bass and hats; a test checks the bass plays G's own functions.

**Arcs** (`plan_arc`, `Role`). Stages come in phrases: intro, one to three builds, peak, breakdown, turn, return. The first
arc builds in two or three stages (7-8 stages in all), later arcs in one or two (6-7). Each role has its own moves (each a
list applied in order, weighted):

| role | moves (weight) |
|---|---|
| intro (a later arc) | thin to three layers + new melody (2); thin + modulate + new progression (1); thin + progression + groove (1); thin + new melody + mode (1) |
| build | a layer in (3); in + groove (2); in + progression (2); in + octave (1); groove + progression (1) |
| peak | two layers in (3); in + progression (2); in + octave (1); in + half/double time (1); the drop (1) |
| breakdown | down to two layers (3); and a mode shift (1) |
| turn | new melody (3); half/double time (2); new melody + a layer (1); progression + mode (1); modulate (1); half/double time + a layer (1); groove + mode (1) |
| return | the drop (3); drop + progression (2); drop + half/double time (1); drop + octave (1) |

The stage before a return also takes a roll into the seam (gone again at the next stage).

**The moves** (`Move`), each a change of a layer or a structure:

| move | what changes |
|---|---|
| enter | a stashed layer comes in (the pads come and go as one layer, the drone pair as one); at the layer budget the least important layer leaves (never the lead), except for a second layer in the same stage |
| leave | a layer leaves (never the lead; at least two stay) |
| melody | `M`'s generator is replaced by another family's, from `grammar::generator` (fractal, squares, Lissajous, Fibonacci, primes, Collatz, rule 30) or two of its own (two waves, `floor(a sin(tau k / p) + b cos(tau k / q) + 0.5)`; residues `mod(ak + floor(k / rate), m) - m/2`); the bar index `R(floor(k / spb))` is kept; helpers are added under fresh names where a different one holds the name; an octave term `± 7` keeps the lead's median within six semitones of where it was. Only while the lead or harmony plays |
| groove | the lead's `P` becomes a Euclidean E(k, 16) at least 5 of 16 steps away (added to the lead's gate, and the harmony's, the first time), or the kick reads a pattern integer `Q` from a table of ten, at least 3 steps away |
| progression | `N` becomes one of ten progressions (I vi IV V, vi IV I V, a descending line, ...), at least 4 of 8 bars different |
| register | the melody moves an octave (`M ± 7` degrees): down when its median is above G5, up when below D#4, else either way |
| feel | half-time or double-time: `e`/`u` between `2t` and `4t`, with `M`'s bar index following |
| mode | `c` by two or three steps |
| modulate | `K` up a fourth (or down a fifth past +7), as in v1 |
| breakdown, thin | down to two (three) layers: the lead and tonal layers first, the harmony last (it only doubles the lead) |
| drop | the stashed layers return in order (kick, lead, pads, bass, harmony, hats, clap, ...) up to the budget, at least two |

After each move `tidy` removes the definitions nothing reads (from the voices playing or waiting) and adds a standard time
name a new helper reads.

**Audibility** (`src/compose/audible.rs`). A stage is sampled 16 times a beat. For each voice (not the roll) the onsets and
the pitch at each onset; two stages then differ by: the layers that entered or left (1 each), the total variation between
their pitch-class distributions (x2), the largest Jaccard distance of one voice's onset sixteenths (x1.5), the change in
onsets per beat as |log2| (x1), the lead's median pitch in octaves (x1), and the share of sixteenths where the lead's
sounding note differs (x1). One layer entering scores 1.0, the **threshold**. v1's nudges measure below it (a melody phase:
0.50; a root digit: 0.28; tested), a new progression above (1.58).

**Choosing.** Eight candidates per stage from the role's moves; each is checked against the budget and the last eight
states, written, parsed, measured and scored by the critic from its functions (without the structure components). Among
those at or above the threshold the critic's best is kept. If none clears it, the three loudest candidates (and the plain
stage) are boosted by another move (a layer in, a new melody, a progression, a groove, a layer out, half/double time) until
one does. In 250 changes from five seeds none fell below.

**The budget.** At most 6 layers and 13 definitions (time names and the roll not counted); a stage that starts above it
(a grammar seed can) may not grow. G has 7 layers, so they are never all in at once.

**The measured difference.** The same 250 stage changes (stages 1-51 of G with seeds 1 and 2, and grammar seeds 3, 11
and 42): **v1 mean 0.822, median 0.434, 169 of 250 (68%) below 1.0; v2 mean 3.71, median 3.40, minimum 1.06, none below.**
Breakdowns and returns measure 4-7, builds 1.1-3.

Tested in `tests/stream.rs`: determinism; every stage parsing and rendering; tempo and key continuity; arcs (lengths and
order over 150 planned arcs, and a chain following its plans); every stage at or above the threshold with the mean at least
3x v1's; the measure (zero against itself, v1 nudges below, a layer and a progression above); the budget (60 stages from
four seeds, and a layer entering at the cap pushing another out, never the lead; unread definitions tidied); no return to
the last eight states; a new melody keeping its bar index and register over 60 draws and reaching at least six families;
every move applying, parsing, reporting exactly what it changed, and leaving the settings alone; the opening.

**What I am unsure of.** The threshold says a change is *audible*, not that it is *good*: the critic still picks among the
audible ones and may measure familiarity, not music. The breakdown's change (4-7) is much larger than a build's (1-3), which
is the point of an arc but may feel abrupt at a seam; the seams themselves stay seamless (the tests above). Whether
"stages that are heard" is the missing ingredient or only half of it is for a listener's ears. A stage's `note_left` sees only to the end
of its stage (no seed song uses it).

**v1 (2026-11-02, replaced).** Each stage changed one or two numbers (the mode by a step, a root digit, a rhythm bit, a
melody coefficient, time scale or phase, a voice in or out, a fill), with a turn every 4-8 stages (a modulation, two or
three melody numbers, the drums out or back); the critic picked the best of six. Its seed was always G.

### The move book (2026-11-03): the brain writes, the algorithm arranges

Listening to v2: the kick was good and the way it changes intelligent, but the instruments and melodies lost the feel
and were still in the style of MIDI rather than abstract graph music like the reference video; it improved with
listening, but the content itself was flat. So the arc planner, the audibility rule and the critic stay, and the content they arrange is now written by
judgment, in `src/compose/book.txt`, one file a person can read. `Seed::Book(seed)` is the default stream; v2's grammar
seed and G remain (`--stream-g`, `LM_STREAM_GRAMMAR_SONG`), unchanged.

**A module** fills one slot and carries a one-line shape and tags: energy and density 1-3, mood words, and `glide` when the
line bends or glides continuously (measured: tagged modules spend at least 4% of their sounding time between semitones,
untagged ones less). Every definition is a constant reading `t` or a function of a step index, and the voice blocks only
read them (`pitch = L`, `gate = Lg`), so the equation panel shows the whole shape. The equation shows only what plays: a
waiting layer's lines leave with it and come back when it enters (`book::finish`).

| slot | modules | what they are |
|---|---|---|
| progression | 12 | `c` and `R(i)`: royal road, turnaround, ii V I, 4-3-6, the circle of fifths as `3i mod 7`, Lydian float, vi IV I V, Aeolian climb, held minor, Dorian vamp, a lament falling `-floor(i/2) mod 4`, Phrygian |
| lead | 12 | four blancmange wanderers (the saved lead's family: eighths, quarters, triplets, bits), Lissajous, Weierstrass bend, a sigh with late vibrato, binary counting, comets falling by `2^(-6x)`, a 3-against-4 staircase, a Lorenz-like drift, Fibonacci mod 7 |
| counter | 4 | a slow cosine over chord tones, glass bells on E(5,16), a swell that breathes up a step, the blancmange read backwards an octave down |
| arp | 5 | a triangle over the chord, bits, a two-octave spiral, Fibonacci mod 3, a legato glide through the chord |
| bass | 7 | anticipation, a breathing root-fifth slide, 808 slides, a walk with chromatic approach and a ghost, a slow blancmange, a call and response with the kick, sixteenth funk |
| pads | 4 | struck, swelling as sin², folded into one octave (voice leading), off-beat stabs |
| drums | 11 | kick patterns (four, a pickup, half time, E(5,16)) keeping v2's kick sound; hats (blancmange loudness, bits, off-beat); claps (backbeat, a ghost); perc (a tom on E(3,8), a rim on Fibonacci mod 4) |
| seam | 9 | a roll, a roll doubling each beat, a riser, a dash cascade, a fall, a reversed swell, a glissando, a stutter, a tom run |

**The contract that lets any module sit on any progression.** A progression writes `R(i)`, the root of bar `i` as a scale
degree; every tonal module reads its chord as `h(R(j) + 2m)`. A melody is an *absolute* register curve `d(k)` in degrees,
pulled onto the nearest triad tone of the bar's root on beats 1 and 3 only: `cs(d, r, k, p) = r + sn(d - r, k, p)`, where
`sn` applies `ct(z) = z - mod(z, 7) + 2 floor(mod(z, 7) / 2) + (mod(z, 7) > 5)` on every p-th step. In between it wanders
freely (the coordinator's reading of the listening notes: the rule is met by the strong beats). A melody relative to the root
inherited the root's motion and spanned 15-20 semitones; the absolute curve spans 8-11. A bass keeps one low octave:
`lo(p) = mod(p + 4, 12) - 40` folds the bar's root to E1-D#2 and `lf(d, r)` builds degree `d` on it.

**Measured** (`tests/book.rs`, each lead over all 12 progressions): beat notes on triad tones 0.66-0.73 (strong beats
0.99-1.00; corpus 0.72-0.75, a random diatonic note 0.43); steps 0.26-0.77 of the moves (corpus 0.23-0.77); range
8.1-11.2 semitones (corpus 9-12; band tested 8-15); bar repetition 0.14-0.67 (corpus 0.36-0.58: the bits and binary leads
repeat more than the corpus, the Weierstrass and Lorenz drifts less; not enforced). Notes are counted as the line's
landings, so a glide counts as the note it reaches. Every bass module, in every bar over every progression, has an
off-beat onset or a pitch other than the root, and the basses' notes a beat rise with energy (0.66, 1.25, 2.0 on average).

**Arranging.** The moves keep v2's names; in a book stream a new melody, a new progression (4 of 8 bars different), a
groove (a playing drum's module) and the new **Swap** (a playing bass, pads, arpeggio or counter-line) install a module
chosen by the stage's need: weight `[1, 0.35, 0.1]` by the energy distance and `[1, 0.6, 0.3]` by the density distance,
need (energy, density) intro (1, 1), build (2, 2), peak (3, 3), breakdown (1, 1), turn (2, 1), return (3, 2); modules
heard in the last four stages are avoided. A layer that enters takes a module for the stage's need half the time. Each
stage still tries eight candidates and the critic picks among those above the audibility threshold. Seam modules
mark every seam into a return or a breakdown, into a new arc, and half of those into a peak, chosen by where they lead
(`into-return`, `into-strip`, `into-peak`, `into-intro`). The grammar's generators remain: when every lead is excluded,
`grammar_lead` writes one from a grammar family (prefixed `Lx`). Budget: 6 layers, 20 definitions, the shared helpers not
counted.

**Two engine changes it needed.** The audibility measure counts a pitch that settles inside a held note as a note (a
legato lead was otherwise one note a phrase). And a voice whose instrument changes at a seam (another module's partials,
gain or amp) is released with its old settings while the new one starts from rest: a spectrum swapped under a sounding
note was a click (0.0667 against the music's 0.0018; with the change 0.0002). The book's own seam test compares a seam
with the music on both sides of it (a new line that starts higher is as rough after the seam as at it) and keeps the
dropped-state control, which it catches at 8-9 of 11 seams.

**Sound** (`luminesce-stream --seed N --from 40 --seconds 150`, then `tools/measure/analyze.py`): seed 1 39.6/32.5/27.0/
0.7/0.2 % with 50 peaks a frame, seed 2 44.0/37.1/17.7/1.0/0.2 % with 48, seed 3 30.2/37.6/27.7/3.9/0.7 % with 61 (the
references 41/37/20/1.6/0.2 with 108 and 60/26/13/0.9/0.3 with 143). The bands sit near the references; the peak density
is about half theirs: the stream's breakdowns run on two layers and the plucks have eight partials. Getting there took
three corrections, each measured: the basses had climbed to A3 with the progression's roots (now folded), the bass gain
dropped 0.28 to 0.16 and the kick rose 0.55 to 0.68, and the glass leads and stabs took denser spectra.

### The ending (2026-11-03): a song that knows when it has said it

A song should have a length limit, and ideally know from its own structure when to end. Not a timer: after each stage the composer scores its own **closure**, 0..1,
`0.3 arcs + 0.25 novelty + 0.15 fatigue + 0.15 home + 0.15 energy`:

| part | measures |
|---|---|
| arcs | returns heard, of two (three arcs and a coda do not fit in 420 s at 92-106 bpm) |
| novelty | distinct book modules heard, of half the book (about 27 of 55): the novelty spent |
| fatigue | `1.5 - (last arc's mean audibility / the liveliest arc's)`, clamped: changes that have stopped surprising |
| home | 1 while the opening's progression is playing again |
| energy | full peaks (five or more layers), of two |

At the end of an arc, once the song with its coda would reach **150 s**, it ends when closure is at least **0.7**, or when
another arc (six stages at least) would not fit before **420 s** with its ring-out (12 s kept for it); at any stage it
ends if one more stage would break 420 s. Why those numbers: 150 s is one whole arc and then some, a little under a pop
song's 3-4 minutes, so a song never stops before it has gone somewhere and come back; 420 s is an extended city-pop mix
or a long electronic track, and by then about four arcs have heard most of the book, so its novelty is spent anyway.
When an arc starts with closure at 0.5 or more, its return **heads home**: the opening's progression and the first lead
heard come back (kept only if the stage stays audible), a bookend for the ending to reprise.

**The ending is composed** (`push_coda`, `push_ring`): the coda keeps the bass, struck pads, the lead and the kick; brings
the opening progression and the first lead back; the lead goes an octave down; the progression plays the opening's first
four bars, then IV, V and the tonic (`R(i) = if(i < 4, ..., 3 (i == 4) + 4 (i == 5))`), in the home key (`K = 0`). Every
function runs on the ending's clock `tr`: in time for 16 beats, then the Friberg-Sundberg final ritard (tempo
`(1 + (w^q - 1) x)^(1/q)`, q = 3, w = 1/2; integrated and inverted in closed form so the last two bars take 10.29 beats),
then half speed. At score beat 24 the bass and lead land on the tonic, the kick dashes once, and a beat and a half later
every gate closes. A last stage holds every voice closed with its pitch frozen where the coda left it, so the releases and
the reverb ring out; the stream stops where that tail falls below -80 dBFS. `Stream::ended_frames` and `lm_stream_ended`
(C) say so and give the length; `pull` returns fewer frames there and none after. The world does not yet respond to it.

**Measured over ten seeds**: codas end at 264-334 s, after two arcs; eight of ten ended by closure (0.70-0.87 before the
coda), two because another arc would not fit (0.68 and 0.70, both just under). Closure rises over every song, from about
0.17 at the opening. The first version counted arcs of three and only four of ten ended by closure: three arcs and a coda
do not fit in 420 s, so the clock did the deciding. Counting arcs of two fixed the model rather than the test, which
still requires most songs to end by closure. Tested: every song
ends within the bounds, the coda's progression lands IV V I I, bass and lead sound the tonic at the end and are silent by
beat 30, the last kick falls at the ritard's end, the ring-out plays nothing new, endings are the same per seed, and a
pulled stream stops at its reported length through the Rust API and the C ABI. Grammar and G streams never end.

**What I am unsure of.** Every song now ends after two arcs, at 4.5-5.5 minutes; a third arc never happens, so the
upper bound is rarely what ends one. Whether two arcs feel like a whole song is for a listener's ears. Whether 0.7 feels finished is a guess
stated as a number. The book is one writer's taste on one day: its modules are measured against the corpus, not against
a listener, and the peak density is half the references'.

### Song species (2026-11-03): a layer above the book

The move-book streams' functions looked nearly the same every time, with only a few things changing; they needed
radical changes in the constraints and in what is possible. That was right. Every book stream
shared one scaffold: the lens `h(d)` on 12-tone modes, chord roots as digit integers `R(i)`, the same glide helper,
4/4 in eighths and sixteenths, the same timbres. The book varied the content inside one world. A **species** is a
different world: its own tuning, meter, generator family, form, timbre and tempo, and its own definitions, so the
equation on screen is different, not only its numbers.

*Since 2026-11-03 the six 12-tone species carry written harmony instead of their own chord arithmetic, and the
strange tunings are behind an Explore option: see "Written harmony" below. The table is the species as first built.*

**Built as books.** Each species is a book in the move book's format (`src/compose/species/<name>.txt`, the lens is
`book.txt`), with a `== species` header (name, title, tuning, meter, generator, form, timbre, a bpm range, a weight,
the ways it may open, and `register no` where the octave does not exist), its own helpers and frame, and modules in
the same slots. Everything the stream already did (the arcs, the audibility rule, the budget, choosing by need, seams,
the closure and the coda) works on any species, because the equation carries its species (`Equation::species`) and
`book::of(eq)` gives its book. Module ids carry the species (`chaos/lead.orbit`); voice names stay the roster's, and a
roster voice is a drum in every species or in none (`tests/species.rs` checks), so the world's roster never changes.

| species | tuning | meter | generator | form | timbre | bpm |
|---|---|---|---|---|---|---|
| `lens` | 12-tone modes through `h(d) = floor((12d + c) / 7)` | 4/4 | blancmange sums, digit integers, bits | loop arcs | the pluck, 808 bass | 92-106 |
| `ratios` | just intonation, `J(r) = Jz + 12 log2(r)`: fundamentals as ratios `Q(i)` (numerator and denominator digit by digit), chords 5:6:7 or the undertone 1/4:1/5:1/6 | 3/4 | the harmonic series of each bar's fundamental (the 7th and 11th harmonics sound between the keys) | loop arcs | pure sines, glass bells | 80-100 |
| `chaos` | major pentatonic `pn(d)` from the digits 97420 | 7/8, 2+2+3 | the logistic map, `F(x) = r x * (1 - x)`, `X(n) = iter(F, 0.3, n)`; the progression slot sets `r` (3.5, 3.5699, 3.7, 3.8282); the arpeggio draws the bifurcation diagram, the counter-line is the butterfly (the same map from 0.3001) | loop arcs | plucks, glass | 110-128 |
| `automaton` | whole tone, `wt(d) = Wk + 2d` | 5/4 (ten eighths a bar) | elementary cellular automata: `Sx(x) = ca(Wr, x, 10)`, row n `Rw(n) = iter(Sx, Wz, n)`, one row a bar on the stream clock; rhythm is the cells, melody the neighbourhood code the rule reads | loop arcs | chip squares, dash percussion | 84-104 |
| `loops` | 19-tone equal temperament, `e19(k) = Ek + 12k/19`, through a 19-tone diatonic `Gm(d)` | none | loops of 4.5, 5, 7, 11, 13, 17, 19, 23 and 29 beats on `w = t + 32 S` (Eno) | a single evolving texture | sines, soft bells | 60-72 |
| `tritave` | Bohlen-Pierce, `bp(k) = Tk + 12 k log2(3) / 13`, the Lambda mode `lam(d) = floor(3d/2)` | 12/8 | a Lissajous figure in the odd ratios 3:5:7 | loop arcs | odd partials only | 76-92 |
| `bounce` | 24-tone (quarter tones) through maqam Rast, Bayati, Hijaz | half time | a bouncing ball, `bk(u, e) = floor(log2(1 - u/8) / log2(e))` (the fall takes the phrase); a damped pendulum; damped springs | loop arcs | marimba-like bells | 120-140 |
| `oneline` | chords as semitone roots and a third size (no lens) | 4/4, 8 steps a beat | one curve leaping bass, chord tone, melody, chord tone 21 times a second (auditory streaming); its melody the ruler sequence | loop arcs, the breakdown can take the illusion apart | chip squares | 150-170 |
| `dialogue` | minor pentatonic `pt(d)` | 4/4 under rubato, `v = t + 0.32 sin(tau t / 8)` | a two-bar call and its inversion, echo or answer | call and response inside the arcs | a voice lead with late vibrato, the pluck answering | 76-92 |

**The equation looks different.** Three lines as the panel shows them:

- `chaos`: `F(x) = r x * (1 - x)`, `X(n) = iter(F, 0.3, n)`, `r = 3.8282`, `Ol = pn(floor(3 + 9 X(mod(floor(1.75t), 28))))`
- `ratios`: `J(r) = Jz + 12 log2(r)`, `Q(i) = dg(Qn, i) / dg(Qd, i)`, `Qn = 17471731`, `Qd = 14361421`, `Jp(m, i) = J(Q(i) (5 + m) / 4)`
- `automaton`: `Sx(x) = ca(Wr, x, 10)`, `Rw(n) = iter(Sx, Wz, n)`, `Wr = 30`, `Cn = wt(3 + nb(rn, mod(floor(2.5t), 10)))`

Measured (`tests/species.rs`, `every_species_writes_its_own_equation`): no two species share more than a tenth of
their definition names (Jaccard over all names, time names and the stream's own `roll`, `sweep`, `S`, `K` left out);
every pair differs on at least four of tuning, meter, generator, timbre and tempo.

**Meter without changing the stage.** A stage stays 32 beats of `t` (eight bars of four), so the seams, the window and
the world's clock do not change. A species' meter lives in its functions: each bar is four beats of `t` divided as the
meter says, three pulses (`floor(3t/4)`), five (`floor(5t/4)`), seven eighths (`floor(7t/4)`) or twelve eighths
(`floor(3t)`). The felt tempo is then the bpm times pulses over four, which is what each species' bpm range was chosen
for. **Loops that run across seams.** A stage's `t` starts at 0, so a loop of 7 beats would restart at every seam and
realign with everything every 32 beats. The `loops` and `automaton` species read the stage clock `S` (written by the
composer, `S = k` at stage k, `set_clock`), the loops as `w = t + 32 S`, the automaton's row as `8 S + floor(t / 4)`;
measured, the four loops of 5, 7, 11 and 13 beats realign every 5005 beats and at no shorter shift.

**Three built-ins** (`src/expr.rs`), each an honest mathematical object rather than a helper written as a series:
`log2(x)`; `iter(F, x0, n)`, the iterated map (the definition holding it is cached by its arguments, so a pure orbit
costs one step per new index; n at most 4096); `ca(rule, row, width)`, one step of an elementary cellular automaton on
a ring. `grammar::uses` also learned that `2v` reads `v` (a name right after a number), which the dialogue's breathing
clock needed.

**Choosing.** `Seed::Species(seed)` (the default stream, `LM_STREAM_GRAMMAR` in the C ABI) picks the species by weight:
the lens 2, the dialogue 1.5 (the two built on what was kept: the wandering blancmange and the pluck; the voice over
stillness), the chaos 1.2 (a wandering lead), the rest 1. Over 600 seeds every species is reached 50-105 times.
`Seed::Named { species, seed }` (`--species NAME`) plays one and never changes; `Seed::Book` (`--book`,
`LM_STREAM_LENS`) the lens alone. A species stream may change species at the start of a later arc, with chance 0.3,
to one whose bpm range (widened by an eighth) holds the stream's tempo, which never changes: measured, 10 changes in
30 streams of 18 stages. The new species opens as its own first stage would, and its opening becomes the home the
coda returns to.

**Openings and the bass (the coordinator's measurement of the stream history, 2026-11-03).** Two of the last three book
streams opened on the identical bass line (`bass.anticipate`), every one opened on the bass and the hats, and the bass
sat through several stages. Now: each species lists its openings (the lens nine: the bass and hats, the pads and a
perc, the lead alone, the kick and bass, the arp and counter-line, ...; `book::opening_shape`); the opening bass is
drawn from the whole bass set, not by the intro's need (which made the quiet basses likeliest); a playing bass that
has kept one module for three stages (`BASS_STAYS`) is swapped in every candidate stage; and a swap takes a stale slot
first. Measured over 160 species seeds: 27 opening basses, none in more than 12 (7.5%); 22 openings, none in more than
18 (11%), the bass and hats in 10 (6%); over 80 lens seeds each of the 7 basses opens 8-16 times. Over 22 lens stages,
3-5 basses are heard and none plays more than 5 stages running.

**Endings.** Every species ends by the book stream's rules, with one change: by its own closure only after two arcs
(a species with a small book spends its novelty in one arc, and ended at 2.5 minutes). The coda's tonal cadence (IV V
I, the tonic landing, the struck pads) is the lens's; another species' coda brings its opening back under the same
final ritard and last kick and rings out. Measured (seed 5): 207-413 s; `loops`, whose stages are 27-32 s long, ends
mid-way through its second arc when one more would pass 7 minutes.

**What it took from the tests.** Every module of every species renders alone over each of its progressions (no NaN, no
silence); every species streams deterministically with every stage audible (mean change 3.2-5.2, none under 1.0), within the budget, never clipping, and its seams no rougher than the two stages they join, while the
dropped-state control is caught at 33 of 54 seams; the tunings are exact (ratios to 1e-12; 19 steps of 19-TET and 13
of Bohlen-Pierce are 2:1 and 3:1; the Lambda mode is 0 1 3 4 6 7 9 10 12; Rast is 0 2 3.5 5 7 9 10.5); every species' window holds all 13 roster voices of their kinds (a species without an arpeggio, a bass or
pads holds silent ones: the C ABI window test caught `oneline` without them); the logistic orbit is bit-identical to the map iterated in Rust; `ca` matches a direct reading of the rule table. The seam test
compares a seam with the whole of the two stages around it, not a quarter second: a bouncing ball's notes come two
beats and more apart, so a seam can hold a lone note onset (bounce, seam 4: 0.0287 against the lead's own onsets
0.0296), with a tenth of headroom where two onsets meet at a seam.

**Hand mutations** (each applied alone, the species tests run, the source restored): 13 of 13 caught: `ca` with left
and right swapped; `iter` one step long; `iter` without its guard; `log2` as `ln`; the stage clock not written; no
bass refresh; the opening bass chosen by need; always the first opening; never a species turn; the seed ignored when
choosing a species; the turned stage not finished (its helpers missing); `uses` blind to `2v`; the Lambda mode off by
one.

**Sound** (`luminesce-stream --seed 3 --species NAME --from 30 --seconds 120 --out examples/species_NAME.wav`, then
`tools/measure/analyze.py`; the WAVs are 23 MB each and not committed). Bands < 150 / 150-500 / 500-2k / 2-6k / > 6k Hz
and median spectral peaks a frame:

| species | bands % | peaks |
|---|---|---|
| lens | 52.9 / 34.0 / 12.3 / 0.5 / 0.2 | 40 |
| ratios | 46.2 / 35.9 / 17.9 / 0.1 / 0.0 | 21 |
| chaos | 26.8 / 48.1 / 24.1 / 1.0 / 0.1 | 51 |
| automaton | 39.6 / 47.0 / 11.7 / 0.9 / 0.8 | 33 |
| loops | 39.9 / 33.7 / 25.8 / 0.6 / 0.0 | 12 |
| tritave | 32.7 / 41.2 / 25.6 / 0.3 / 0.1 | 29 |
| bounce | 34.5 / 42.0 / 23.1 / 0.2 / 0.2 | 35 |
| oneline | 37.1 / 41.6 / 20.7 / 0.4 / 0.1 | 81 |
| dialogue | 10.7 / 57.9 / 30.2 / 1.2 / 0.0 | 30 |

(The references: 41/37/20/1.6/0.2 with 108 peaks, and 60/26/13/0.9/0.3 with 143.) Most sit near the references'
bands; the peak density is lower, as for the book (pure sines and odd-partial timbres have few partials by design;
the loops are sparse by design). Two were corrected after the first render: the chaos arpeggio went down an octave
(its 500-2k band was 43.7%), and the dialogue's answers came up to the call's register. Not yet right: the dialogue's
stretch here is crowded in 150-500 Hz and thin below 150 (its bass sat out much of this stretch), and the automaton's
pads crowd 150-500 (47%); both are one seed's two minutes, not a fix.

**What I am unsure of.** Whether these are music to a listener: radical by construction, familiar by the lenses and the
timbres, measured only by the machine. The audibility rule and the critic were written for the lens; the critic still
picks among candidates in every species and may prefer lens-like content. A species turn mid-song changes the
instruments at a seam (the renderer releases the old and strikes the new, as for any module swap); it has been
measured only by the seam test. `oneline` at 150-170 bpm makes stages of 11-13 s, so its arcs pass quickly. The world
may want a word for the species in the equation header; it shows the equation as it is now.

### Written harmony (2026-11-03): harmony is written, never generated

A loops stream saved as `examples/stream_s808394_3-6.txt` did not sound good most of the time, and not because the
sound was cheap: the progression was the problem. That stream's
chord root cycled through five steps by `mod(c + m, 5)` and changed every 11 beats, out of step with the bar: harmony
by arithmetic, with no tension, no release and no cadence. The one generated-era piece judged alright, citypop_v4,
ran on real functional harmony. So the harmony is now written by judgment and the maths writes everything on top.

**The progressions** (`src/compose/harmony.txt`, read by `src/compose/harmony.rs`): twenty, each one stage (eight bars)
of Roman numerals with a mode, a phrase length, energy, density and mood.

| name | mode | the chart |
|---|---|---|
| royal_road | major | Imaj9 \| IVmaj7 \| V7 \| iii7 vi7 \| IVmaj7 \| V7 \| III7 vi7 \| ii9 V7 |
| marusa | minor | i9 \| vii7 III7 \| VImaj7 \| V7 \| i9 \| vii7 III7 \| VImaj7 \| V7b9 (the "Just the Two of Us" changes from the minor home) |
| two_five_one | major | Imaj9 \| Imaj9 \| ii9 \| V13 \| Imaj9 \| vi9 \| ii9 \| V7sus V7 |
| three_six_two_five | major | Imaj9 \| iii7 VI7 \| ii9 \| V13 \| Imaj9 \| iii7 VI7b9 \| ii9 V7sus \| V7 |
| borrowed_four | major | Imaj9 \| IVmaj7 \| iii7 \| vi9 \| IVmaj7 \| ivm6 \| iii7 vi7 \| ii9 V7sus |
| backdoor | major | Imaj9 \| IVmaj7 \| iv7 \| bVII9 \| Imaj9 \| vi9 \| ii9 iv7 \| bVII13 |
| tritone | major | Imaj9 \| VI7 \| ii9 \| bII7 \| I69 \| VI7b9 \| ii9 \| bII9 |
| turnaround | major | Imaj9 VI7 \| ii9 V7 \| iii7 VI7b9 \| ii9 V13 \| IVmaj7 III7 \| vi9 II7 \| ii9 V7sus \| V7b9 |
| minor_climb | minor | i9 \| VImaj7 \| IIImaj7 \| VII7 \| i9 \| VImaj7 \| IIImaj7 VII7 \| V7b9 |
| picardy | minor | i9 \| iv9 \| v7 \| V7b9 \| I69 \| iv9 \| v7 V7sus \| V7b9 |
| dorian_vamp | dorian | i9 \| IV13 (x3) ... \| VImaj7 \| V7#9 |
| lydian_float | lydian | Imaj9 \| II6 (x3) ... \| ii7 \| V9sus V7 |
| line_cliche | major | Imaj9 \| Imaj7/7 \| I7/7 \| VI7 \| ivm6/3 \| I/5 \| II7/3 \| V7sus V7 (bass C B Bb A Ab G F# G) |
| descending | major | Imaj9 \| Imaj7/7 \| vi9 \| vi7/7 \| IVmaj7 \| I/3 \| ii7 \| V7sus V7 (bass C B A G F E D G) |
| deceptive | major | Imaj9 \| iii7 \| IVmaj7 \| V7sus V7 \| vi9! \| IVmaj7 \| ii9 \| V13 |
| axis | major | Iadd9 \| V/3 \| vi7 \| IVmaj7 (x2), the last bar IVmaj7 V7sus |
| lofi_fall | major | Imaj9 \| IVmaj9 \| iii7 \| ii9 \| Imaj9 \| IVmaj9 \| iii7 VI7b9 \| ii9 V9sus |
| deep_minor | minor | i9 \| % \| iv9 \| % \| i9 \| % \| VImaj7 \| V7#9 |
| circle | major | Imaj9 \| IVmaj7 \| viiø7 \| III7b9 \| vi9 \| II9 \| ii9 \| V13 |
| minor_circle | minor | i9 \| iv9 \| VII9 \| IIImaj7 \| VImaj7 \| iiø7 \| V7b9 \| iiø7 V7b9 |

The rules every one keeps (tests/harmony.rs proves them): it starts on the tonic and ends on a dominant (V in any
colour, V7sus, viiø7, bII7, bVII7), so **every stage seam is a cadence** whatever the next progression is; each phrase
(4 or 8 bars) ends on a dominant and the next begins on the tonic unless the chord is marked as a deceptive turn
(`vi9!`, one progression, which the stream plays at most once a song); chords change only on bar lines and half bars.
The plan named "I–I/7–vi–vi/5" for the descending bass; the line needs vi7/7 (G under Am) to keep falling by
step, so that is what `descending` writes.

**Voicing.** Three pad voices over the bass. Each chord quality has one set of three pad tones chosen by hand: the third
and the seventh (the guide tones) and one colour, the written extension (9, 11, 13, b9, #9) or else the fifth; triads
keep their root. The voices live in fixed overlapping registers a tritone apart (`K - 16 .. K - 8`, `K - 10 .. K - 2`,
`K - 4 .. K + 4`; nine semitones each, so each table needs one digit a half bar), chosen by measuring every layout
against every chord written: the one where every chord has a voicing with no adjacent minor seconds and the voices move
least. The first chord takes the voicing nearest the middle of the registers; every next chord takes, of all its
voicings (low < middle < high, no crossing, the right pitch classes), the one with the **smallest total movement** from
the one before. Measured: 4.2 semitones a change for the three voices together. The bass plays the written bass note
(the root, or an inversion written with `/3 /5 /7`).

**As the language reads it.** A progression module writes six lines: `c` (the melody's mode for the lens `h`), `Hm`,
`Ho` and `Hb` (the bass note, one digit a half bar, as steps on the circle of fifths from an origin; half steps for the
chromatic line the circle cannot hold in ten digits), and `Hx Hy Hz` (each pad voice, one digit a half bar above its
register's floor). The stream sets the key `K` once a song (E to B, from the seed; a modulation at an arc's turn may
move it, and the coda goes home). The shared vocabulary (helpers, not counted against the budget) reads them:
`Hr(i)` the bass note, `Hv(m, i)` pad voice m, `Hc(m, i)` a ladder of the chord's tones from the bass's pitch class
under the pads, `sc(x, i)` the chord tone nearest x, `hs(d, k, p)` scale degree d pulled to the chord when step k falls
on a half bar. The tables are integers below 2^53 and are read back exactly in any key (tested for every slot).

**The maths on top.** Every 12-tone species kept its generators and rhythms and gave up its own chord arithmetic:

- `lens`: the leads' `cs`/`sn` triad snap became `hs` (the same fractals, the nearest chord tone on beats 1 and 3);
  pads play the voicing; arpeggios climb `Hc`; basses read `lb(i)` (the bass note in the low octave) and `l5(i)` (the
  chord tone nearest a fifth above), with their approach notes into the next bar's bass.
- `chaos`: the regime `r` moved from the progression slot to the leads (3.8282 the orbit, 3.5 the cobweb, 3.7 the
  burst; 3.5699 without a lead); the pentatonic follows the progression's mode; notes on the 7/8's accents (eighths 0,
  2, 4) land on a chord tone (`ac`); the bifurcation arpeggio plays the chord's ladder.
- `automaton`: the rule moved to the leads (110, 90, 30; 45 without a lead); the whole-tone scale went, because a
  whole-tone scale cannot hold a dominant resolving to a tonic; the neighbourhood code is now a scale degree, on a chord
  tone at each half bar (and the 3+2 accent).
- `bounce`: each bounce lands on a chord tone, so a ball settling is an arpeggio falling; the quarter-tone original is
  kept whole as the species `maqam`.
- `oneline`: the curve's bass is the progression's bass note, its chord tones the pads' voicing, its melody the ladder.
- `dialogue`: the voices sing the pentatonic of the progression's mode on their breathing clock and land on a chord
  tone at each half bar of it; the bass and pads moved onto the grid, so the harmony keeps the bar while the voices
  breathe over it.

**Default and Explore.** The default stream (`Seed::Species`, `LM_STREAM_GRAMMAR`) chooses among the six species with
written harmony and turns only to another of them. The strange tunings (`loops` in 19-tone, `tritave` in Bohlen-Pierce,
`maqam` in quarter tones, `ratios` in just intonation) are behind the Explore option: `Seed::Explore`,
`LM_STREAM_EXPLORE` (8, with `LM_STREAM_GRAMMAR`), `luminesce-stream --explore`, and the world's Library entry
**Explore (strange tunings)** beside **Stream**. `ratios` stays there: a just-intonation chord built on each bar's
fundamental cannot also be a 12-tone progression in tune, and retuning it to one would remove what it is.

**The coda** plays the home progression's first four bars and then IV (iv in minor), V7 and the tonic for the last
two bars (`harmony::coda`), in the song's home key, so the final ritard lands on home in every harmony species.

**Tests changed on purpose** (each assumption the plan changed): `tests/book.rs` expects 16-24 progressions (was
8-12), reads the bass note from `Hr` instead of the lens's `R`, checks the coda's bass notes IV V I instead of `R`, and
accepts the coda's progression id (`<home>.coda`); `tests/species.rs` leaves the shared harmony names out of each
species' vocabulary (and asks ten own names of a harmony species, not twelve), compares tunings only between species
that do not share the written harmony, treats `bounce` and `maqam` as one species in two tunings, expects the default
seed never to reach an Explore species, bounds the likeliest opening bass at 13% (the pool fell from 27 basses in nine
species to 19 in six; the likeliest is expected at 6.5%), and asks three basses of a song only where the bass plays
long enough for the swap rule to take it twice (seed 1's bass sits out 15 of 22 stages; it sat out 14 before and
passed with three basses exactly). `luminesce::compose::book::line_stats` now counts a beat note as a chord tone when
its pitch class is the bass note's or a pad's (sevenths and colours included), read from the tables.

**Sound** (`luminesce-stream --seed 3 --species NAME --from 30 --seconds 120 --out examples/harmony_NAME.wav`, then
`tools/measure/analyze.py`; 23 MB each, not committed). Bands < 150 / 150-500 / 500-2k / 2-6k / > 6k Hz and median
spectral peaks a frame (the references: 41/37/20/1.6/0.2 with 108, and 60/26/13/0.9/0.3 with 143):

| species | bands % | peaks |
|---|---|---|
| lens | 39.0 / 31.8 / 25.5 / 3.0 / 0.6 | 42 |
| dialogue | 31.7 / 56.6 / 9.7 / 1.8 / 0.2 | 36 |
| chaos | 34.4 / 37.6 / 26.4 / 1.4 / 0.1 | 56 |
| automaton | 47.0 / 38.4 / 13.4 / 0.6 / 0.5 | 36 |
| bounce | 34.5 / 44.6 / 20.2 / 0.4 / 0.3 | 35 |
| oneline | 34.3 / 42.9 / 22.3 / 0.4 / 0.2 | 95 |

Most sit near the references; the dialogue still crowds 150-500 Hz (56.6%, it was 57.9% before the harmony), and the
lens is brighter than before (3.0% at 2-6k against 0.5%: the 9ths and 13ths). Each 150-second render took 7-18 s.

**Melody on the chord** (`tests/species.rs`, averaged over every third progression): the lens leads 0.65-0.81 of
beat notes on chord tones and 1.00 on beats 1 and 3 (the corpus 0.72-0.75; tests/book.rs holds the lens to the corpus
band); dialogue 0.62-0.91 and 0.77-0.88 (the voices breathe off the grid it is measured on); bounce 0.63-1.00; chaos
0.61-0.77 and 0.64-1.00 and automaton 0.65-1.00 (their accents fall off the 4/4 grid); oneline's curve is all chord
tones by design (1.00). Every lead and counter of every harmony species is held to at least 0.6 of both.

**Hand mutations** (each applied alone, its tests run, the source restored): 13 of 14 caught: the voice leading taking
the largest movement; the table read without its wrap at 16; the bass origin off by one; IV counted as a dominant; the
pads' registers moved; the coda without its dominant; the chord pull choosing the farther tone; `/3` reading the fifth;
the coda's home key not restored; the key not drawn from the seed; the default stream choosing an Explore species; a
species turn crossing into Explore; a lens lead without its chord pull. Missed: removing the call that spends the
deceptive progression (in 40 songs it never came back on its own); the function that decides it is tested directly.

**What I am unsure of.** Whether the voicing registers suit every species' timbre: they were chosen for the chords, not
the instruments. The melodies' weak beats walk the progression's scale, which over a secondary or substitute dominant
(III7, bII7) puts a passing note against a chord tone for an eighth; the strong beats are always right. A modulation at
an arc's turn moves the key without a pivot (the old key's V7 into the new key's I, a step down), which reads as the
turn it is, not as a cadence. The chaos and automaton lines are measured on a 4/4 grid their accents do not fall on.

### The seams

The renderer (`src/stream.rs`) keeps each voice's state across a seam by its name: oscillator phase, envelope level and
stage, the gate's last value (a note held over the seam is not struck again), the filter, the note's start, and every drum
hit still ringing. A voice that leaves is released with its last stage's functions (its time running on); one that
returns takes its own state back. The reverb (with its predelay), the delay, the wow (reading behind, since a stream cannot
read ahead) and the warmth (against a fixed full-scale reference instead of the song's peak) are one instance for the
stream.

- **One long render.** A song whose functions repeat every 32 beats, rendered as two stages, equals the offline render of
  the 64-beat song to 1.5e-8 per sample at most (f32 rounding), drum hits on the same samples (a pad held across the seam
  whose amp reads `note_age`, a filtered pluck,
  a drum ringing over it, reverb with predelay and a delay).
- **No click at a seam.** At each of seven seams of the default stream, the largest second difference within two samples
  of the seam against the largest in the quarter second before it: 0.11x to 0.68x (the seam is smoother than the music
  around it). The same stages with the state dropped at each seam (the control): 4.3x to 14.7x, so the measure catches a
  click. The pulled audio, with the gain ramps, measures the same (0.11x to 0.61x over five seams).
- **Level.** Each stage's gain keeps its peak 1 dB under -1 dBFS, rises at most 1 dB a stage and never above 1, and moves
  between stages over half a second (down at the end of the louder stage's predecessor, up at the start of the next), so
  every sample stays under the ceiling. Pulled in odd block sizes, the samples are identical.
