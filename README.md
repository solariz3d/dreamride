# Dreamride

Dreamride: music written as mathematical functions of time. This is the headless engine (build step 1 of
[DESIGN.md](DESIGN.md)): an expression language, a song file format, synthesis, and a WAV writer.
It uses only the Rust standard library. (The crate and its binaries keep the working name `luminesce*` for now.)

```
cargo build --release
target/release/luminesce-render examples/citypop_sketch.txt out.wav          # 16-bit PCM
target/release/luminesce-render examples/citypop_sketch.txt out.wav --float  # 32-bit float
cargo test
```

The tool prints the duration, the sample count and the peak level. Any error (a syntax error, an
unknown name, a NaN pitch at some beat) stops it with a message and a non-zero exit, and no
output file is written.

## The song format

Plain text, one item per line. `#` starts a comment. Settings, definitions and blocks may come in
any order; definitions may use each other in any order.

```
bpm 100                      # required, (0, 1000]
beats 32                     # required: the length in beats (> 0, up to one hour)
rate 48000                   # optional, 8000..192000 Hz, default 48000
reference 440                # optional: the frequency of pitch 0, default 440 (A4)

h(d) = floor((12d + 5) / 7)  # a function definition
key = 3                      # a named constant (may also use t)

voice lead {                 # a tonal voice
  wave = square              # sine | triangle | saw | square | additive (see Graph instruments)
  pitch = key + h(floor(t))  # semitones above (or below) the reference, as a function of t
  gate = 0.75 - frac(t)      # sounds while > 0; each rise from <= 0 is a new note
  amp = 1                    # optional amplitude multiplier, per sample (default 1)
  gain = 0.2                 # optional, 0..10 (default 1)
  pan = 0.3                  # optional, -1 (left) .. 1 (right), default 0
  attack = 5                 # optional, ms (default 5)
  decay = 100                # optional, ms from full level down to the sustain level (default 100)
  sustain = 1                # optional, 0..1, the level held while the gate is open (default 1)
  release = 30               # optional, ms (default 30)
  cutoff = 1200 + 800 sin(t) # optional low-pass cutoff in Hz, an expression of t (default: no filter)
  resonance = 0.2            # optional, 0..0.95 (default 0, Butterworth)
  filter_env = 2             # optional, octaves added to the cutoff at each note onset, -8..8 (default 0)
  filter_decay = 150         # optional, ms for that boost to fall by 1/e (default 200)
  drive = 1                  # optional soft saturation, 0 (off) .. 10 (default 0)
  detune = 8                 # optional: two oscillators this many cents apart, 0..100 (default 0: one)
  reverb = 0.3               # optional send to the reverb, 0..1 (default 0)
  delay = 0.2                # optional send to the stereo delay, 0..1 (default 0)
}

drum k {                     # a drum voice
  kind = kick                # kick | snare | hat
  trigger = 0.25 - frac(t)   # a hit on every rise from <= 0 to > 0
  amp = 1                    # optional velocity, evaluated at the onset (default 1)
  gain = 0.9                 # optional
  pan = 0                    # optional
  reverb = 0.3               # optional sends, as for voices
  delay = 0
}
```

**Graph instruments.** Desmos `tone()` plays a sine, and graph music builds every sound by adding
sines; Desmos also caps a tone's gain at `min(10, 660 / f)`, so high partials and high notes are
quiet. Two additions write that sound directly:

```
voice lead {
  wave = additive                    # a sum of sine partials at 1, 2, 3, ... times the pitch
  partials = 0, -16, -4.5, -15, -11  # their levels in dB, harmonic 1 first (1 to 64 of them, -120..20 dB)
  rolloff = 660                      # optional: each partial x min(1, 660 / its Hz), the Desmos cap
  ...                                # every other voice field works as before
}

drum kick {
  kind = dash                        # a sine whose frequency dashes down: to + (from - to) e^(-tau/dash)
  trigger = 0.25 - frac(t)
  from = 450                         # Hz at the onset (default 450)
  to = 58                            # Hz it settles at (default 60)
  dash = 23                          # ms for the gap to fall by 1/e (default 23)
  hold = 40                          # ms at full level (default 0), then
  decay = 90                         # ms per 1/e fall (default 150)
  length = 380                       # ms, with a 10% fade at the end (default 400)
  partials = 0, -24, -12             # optional harmonics of the swept sine, dB (default: one sine)
  rolloff = 660                      # optional, as for voices, at each partial's present frequency
}
```

A partial at or above half the sample rate is left out, so nothing aliases: for an additive voice
per sample (it returns when the pitch falls), for a dash for the whole hit if it would reach Nyquist
anywhere in the sweep (so it never switches on mid-hit). Each partial's phase is the voice's phase
times k, reduced to [0, 1) before the sine; a dash's phase is the exact integral of its frequency,
`to tau + (from - to) k (1 - e^(-tau/k))`. `partials = 0` is a sine, sample for sample. `rolloff` is
allowed on `sine` and `additive` voices; `partials` only on `additive` voices and dashes; the dash
fields only on `kind = dash`. Measured in `tests/graph.rs`: the levels to 0.02 dB, the cap's
660/880 and 1/2, no alias above -90 dB, the dash's frequency at set times to 0.3%.

Optional song settings for the effects (all off or neutral by default, so older songs render
byte-identically):

```
loop 1                       # 0 or 1: the song is a loop; effect tails wrap round (see Space)
reverb_size 0.7              # 0..1 (default 0.5)
reverb_damp 0.5              # 0..1 (default 0.5): higher is darker
reverb_width 1               # 0 (mono) .. 1 (default 1)
reverb_predelay 20           # ms, 0..500 (default 0)
delay_beats 0.75             # the delay time in beats, 0.01..16 (default 0.75, a dotted eighth)
delay_feedback 0.35          # 0..0.95 (default 0.35)
delay_tone 3000              # Hz, a low-pass in the feedback: each repeat darker (default 3500)
wow 6                        # tape wow, peak cents, 0..100 (default 0: off)
wow_rate 0.5                 # Hz (default 0.5)
warmth 0.3                   # master saturation and top roll-off, 0..1 (default 0: off)
```

Settings are written without `=`; block fields and definitions with it. Block names are unique.

**Time.** `t` is in beats: sample `i` is at `t = i * bpm / (60 * rate)`. The file has exactly
`round(beats * 60 / bpm * rate)` samples per channel, so a loop of whole bars is loop-exact.

**Pitch.** `pitch = p` sounds at `reference * 2^(p / 12)` Hz: 0 is A4 = 440 Hz, 12 is A5, -9 is
C4. Pitch may be fractional and may glide; the oscillator keeps its phase, so glides are smooth.

**Gates and envelopes.** A voice sounds while `gate > 0`. Each rise of the gate from <= 0 starts a
note: the level ramps linearly up to full over the attack, then linearly down to `sustain` over
the decay, and holds there while the gate stays open. When the gate closes, it falls at full scale
per release time (so from a sustain of 0.4 a 100 ms release takes 40 ms). Every segment is a
bounded slope, so onsets and offsets do not click. With the defaults (sustain 1) this is the
step-1 envelope exactly, and old songs render byte-identically.

**Filter and drive.** `cutoff` puts a resonant low-pass on the voice: a topology-preserving
state-variable filter (V. Zavalishin, *The Art of VA Filter Design*, ch. 4; in A. Simper's
trapezoidal form, Cytomic 2013). Its cutoff is prewarped, so at resonance 0 the response at the
cutoff is exactly -3.01 dB (measured within 0.01 dB at 100 Hz to 15 kHz), and it stays stable when
the cutoff changes every sample, so a cutoff written as a function of `t` is a filter sweep. The
cutoff is limited to 10 Hz .. 0.45 x rate. Resonance r gives a peak of 1 / (sqrt 2 (1 - r)) at the
cutoff (+11 dB at 0.8). `filter_env` adds octaves at each note onset that decay with `filter_decay`:
a pluck. `drive` is `tanh((1 + d) x) / tanh(1 + d)`, after the filter and before the envelope, so a
full-scale peak stays full scale. A non-finite cutoff stops the render like a NaN pitch does.

**Oscillators.** Saw and square are band-limited with PolyBLEP (since step 1). Measured on a
3520 Hz saw at 48 kHz, the energy outside the harmonics is -24.5 dB of the total, against -10.3 dB
for a naive saw: 14.1 dB less aliasing (`polyblep_cuts_alias_energy_against_a_naive_saw`). Two notes in a row need the
gate to drop to 0 or below between them (for example `0.75 - frac(2t)` gives eighth notes), or
the second note is a pitch change inside one note.

**Drums.** The kick is a sine swept from 160 Hz to 48 Hz with an exponential decay (0.6 s); the
snare is noise plus a 185 Hz body (0.3 s); the hat is high-passed noise (0.12 s). The noise comes
from a seeded SplitMix64 generator, so every render is identical. Hits may overlap.

**Detune.** `detune = c` replaces the oscillator with two, at `-c/2` and `+c/2` cents, averaged:
the slow beating of doubled voices (20 cents at A4 beats at 5.08 Hz, measured in
`detune_beats_at_the_difference_of_its_two_oscillators`).

**Space.** Each voice and drum can send to two shared effects, taken before the pan (so its room
and echoes do not follow it to one side):

- **Reverb: Freeverb** (Jezar at Dreampoint, public domain, 2000), written here from its
  description in J. O. Smith's *Physical Audio Signal Processing* (CCRMA): per channel, eight
  lowpass-feedback combs (1116 .. 1617 samples at 44.1 kHz, scaled to the song's rate; the right
  channel 23 samples longer) into four Schroeder allpasses (gain 0.5). Feedback `0.7 + 0.28 size`,
  damping `0.4 damp`, Freeverb's input gain. Measured: a send of 1 returns -3.6 dB of energy at
  size 0.5 and -1.6 dB at 0.86. It decays by more than 60 dB within 20 s even at size 1
  (`the_reverb_decays_and_stays_finite_even_at_full_size`).
- **Delay: a tempo-synced ping-pong.** `delay_beats` at the song's tempo, rounded to the sample
  (at 100 bpm a dotted eighth is exactly 21600 samples). The first repeat is on the left, the
  second on the right, each darker through a one-pole low-pass in the feedback.
- **Wow** reads the finished mix through a slowly moving playhead (cubic interpolation), so every
  voice drifts together like tape; `wow 20` measures as a swing of 20 cents either way.
- **Warmth** is a tanh saturation measured against the mix's own peak, then a low-pass from
  20 kHz (warmth 0) to 7 kHz (warmth 1).

**Tails and exact length.** The file is always exactly `beats` long. With `loop 1`, every effect
runs over the song twice and keeps the second pass: the reverb and echoes still ringing at the
end sound over the start, exactly as they will when the file is looped, and wow is snapped to a
whole number of cycles, so the seam is seamless. Without it, effects start from silence and
anything that would ring past the end is cut. Either way the render is byte-identical every time.

**Mixing.** Voices are summed with equal-power panning. If the peak exceeds -1 dBFS, the whole mix
is scaled down to exactly -1 dBFS and the tool reports by how much. Nothing is clipped.

**Errors at render time.** If a pitch, gate, amp or trigger evaluates to NaN or infinity (for
example `sqrt` of a negative, or a division by zero), or a pitch is above half the sample rate,
the render stops and names the voice, the quantity and the beat.

## The expression language

- Numbers (`3`, `0.5`, `.25`; no exponent notation), the time `t`, the constants `pi` and `tau`,
  and names you define.
- Operators `+ - * / ^` and parentheses. `^` binds tightest and is right-associative
  (`2^3^2 = 512`); unary minus binds looser than `^`, so `-2^2 = -4` (as in Desmos), and
  `2^-1 = 0.5`.
- **Implicit multiplication:** a term followed directly by a name or `(` multiplies: `12d`,
  `2(x + 1)`, `u^2(3 - 2u)`, `2 sin(t)`, `(a)(b)`. It has the same precedence as `*`, so
  `1/2x = (1/2)*x`. A name followed by `(` is always a function call, so a value times a bracket
  needs `*`: write `d*(x + 1)`, not `d(x + 1)` (which is an error: "'d' is not a function").
- Built-in functions: `floor ceil frac abs sign sin cos sqrt` (one argument), `mod min max` (two),
  `clamp(x, lo, hi)` (three). `frac(x) = x - floor(x)`; `sign(0) = 0`; `sin` and `cos` take
  radians; `clamp` is `min(max(x, lo), hi)`.
- `mod(a, b)` is floored, as in Desmos: `a - b*floor(a/b)`, so it is in `[0, b)` for `b > 0`
  (`mod(-1, 12) = 11`) and in `(b, 0]` for `b < 0`. `mod(a, 0)` is NaN.
- Comparisons `< > <= >= == !=` give 1 or 0, bind looser than arithmetic (`1 + 1 == 2` is 1),
  and cannot be chained (`a < b < c` is an error). With a NaN operand the result is NaN, so the
  renderer reports it instead of it quietly reading as false.
- **Expressive built-ins** (`src/expressive.rs`, each pinned by `tests/expressive.rs`):
  - `arch(x, peak)`: the phrase arch: 0 at `x` = 0 and 1, 1 at `peak`, two parabolas (Todd 1992;
    the KTH Phrase rule). Drive dynamics, brightness or anything else by where a note sits in its
    phrase.
  - `phrase(c, depth, peak)`: phrase time, a warp of `c` (counted in phrases) whose tempo is
    `1 + depth (arch - 2/3)`: slow at a phrase's ends, fast at its peak. Whole numbers map to
    themselves, so every phrase keeps its written length and a loop stays loop-exact; `depth` is
    clamped to [0, 1], so the warp always increases and notes never pass each other. Read a voice's
    pattern through it (`step(8 phrase(t / 8, 0.1, 0.6))`) to give it final lengthening and a
    little rubato against a band that stays on the grid.
  - `ritard(x, q, w)`: the Friberg-Sundberg final ritard, tempo `[1 + (w^q - 1) x]^(1/q)` at
    position `x` of the ritard, falling to `w` (q = 3 is the runners' constant braking power).
  - `charge(semis)`: KTH melodic charge of a note `semis` above the chord root (root 0, fifth 1,
    ... tritone 6, b9 6.5). The KTH rule adds 0.2 dB per unit.
  - `vibrato(age, delay, ramp, depth, rate)`: semitones of delayed vibrato: nothing until `delay`
    seconds, then a sine at `rate` Hz growing smoothly to `depth` over `ramp` seconds.
  - `db(x)` is `10^(x/20)`; `smooth(u)` is smoothstep on `u` clamped to [0, 1].
- **Note time.** In a tonal voice's `pitch`, `amp` and `cutoff`, `note_age` is the seconds since
  the current note's onset and `note_left` the seconds until its gate closes (0 or negative once
  it has). The renderer evaluates the gate first, so it knows where each note ends. Scoops, falls,
  vibrato that starts late, an 808 that sags as it runs out: `pitch = p - 0.5 (1 - smooth(note_age
  / 0.08))`. Before the first note, `note_age` counts from the song's start. A `gate`, a drum
  `trigger` or a drum `amp` that reads them is an error, since they decide where notes are. A
  definition that reads them is never cached.
- **Built-ins for the song species** (2026-11-03, `src/expr.rs`, pinned by `tests/species.rs`):
  - `log2(x)`: the octaves in a ratio, so a just interval is written as itself: `12 log2(3/2)` is a pure fifth in
    semitones, `12 k log2(3) / 13` a Bohlen-Pierce step. NaN for `x <= 0` (a constant that is not finite is an error).
  - `iter(F, x0, n)`: the definition `F` (one parameter) applied `floor(n)` times to `x0`, the one way to write an
    iterated map in a language without recursion: `F(x) = r x * (1 - x)` and `X(n) = iter(F, 0.3, n)` is May's
    logistic map, written as it is. `F` must name a definition of one parameter that does not call back into the
    caller. `n` below 0 is 0; above 4096 it is NaN (a guard on the cost per sample: the definition holding the `iter`
    is cached by its arguments, so a pure orbit is computed once per step).
  - `ca(rule, row, width)`: one step of an elementary cellular automaton (Wolfram's numbering) on a circular row of
    `width` cells (1-52) stored as the bits of `row` (cell 0 the lowest; a cell's left neighbour is the next higher
    bit). `iter` steps it: `Rw(n) = iter(Sx, 1, n)` with `Sx(x) = ca(30, x, 10)` is rule 30's row `n`. NaN for a rule
    outside 0-255, a width outside 1-52 or a row outside 0..2^width.
- `if(c, a, b)` is `a` when `c > 0`, otherwise `b`; only the branch taken is evaluated, so the
  other may be undefined there. A NaN condition gives NaN. Two conditions at once are
  `min(c1, c2)` (and) or `max(c1, c2)` (or).
- Definitions: `name = expr` or `name(a, b, ...) = expr` (up to 8 parameters). A parameter named
  `t` shadows the time. A definition may not redefine `t` or a built-in, and definitions may not
  call themselves, directly or through others. A constant that does not use `t` is evaluated once.
- Errors carry the line and column: `line 5, column 15: unknown name 'nope'`. Wrong argument
  counts, unknown functions, syntax errors and unclosed blocks are all reported; nothing silently
  becomes 0.

The text is parsed once into a tree, names are resolved to indices, constants are folded, and the
tree is evaluated for every sample. A definition that takes parameters and does not depend on `t`
(directly, through what it calls, or inside an `if` branch) is a pure function of its arguments,
so it keeps a small cache keyed by their exact bits. Musical helpers are called with the same step
or bar number for thousands of samples in a row, so this is what makes table lookups, sorted
voicings and pattern holds affordable; the results are bit-identical with or without it.

What the language still lacks, found while writing the examples: no state or recursion (`iter` covers an iterated
map of one value, not a rule over a whole history), so a rule
that depends on the previous chord's voicing (greedy voice leading) cannot be written, and the
examples use a fixed sorted window instead; no higher-order functions, so each voice's pattern
helpers (holds, onsets) are written out per voice; no arrays, so tables are digits of integers,
which must stay below 2^53 (a 16-digit pattern must not start with 9).

## The composer

`target/release/luminesce-compose --seed 7 --count 200 --keep 5 --out generated --wav` writes 200 graph songs from a
seed and scores each with a critic of measured components. It keeps the best five in `generated/<UTC time>/` with a
`scores.md` that explains each pick. Rate them in `generated/ratings.txt`. The grammar, the critic and its citations,
and the falsifier registered before the first run (and how it came out) are in `docs/composer.md`.

## Stream mode

A song written while it plays, the way @measure_plan plays a graph: one equation sounds for eight bars, then a line
of it is edited and it plays on. `world\bin\luminesce-world.exe --stream` (or the library's first entry, **Stream**) is
**a new song every time**: since 2026-11-03 arranged from **the move book** (`src/compose/book.txt`), from a seed
drawn from the clock, shown in the equation's header and printed (`stream: play this one again with --stream --seed N`). `--stream --seed N` plays seed N again;
`--stream-g` (or the library's **Stream from G**) starts from G, the song kept from the blind test (`s7-c38`, binary
counting by square waves, with its pluck lead); `--stream-song SONG.txt` starts from any song (`--seed` then drives the
evolution). R replays the stream playing from its first stage. A seed always plays the same stream.

- **The move book (2026-11-03).** The content is written by hand and the stream only arranges it: 43 graph-native
  modules (12 leads, among them four blancmange wanderers like the lead that was liked; counter-lines, arpeggios, seven bass
  lines with pushes, ghosts, approaches and slides; pads; drum patterns as dashes), 12 progressions and 9 seam modules,
  each a few definitions with its shape in words and tags (energy, density, mood, glide). Melodies are register curves
  pulled onto the chord on beats 1 and 3, so any module fits any progression; measured, 0.66-0.73 of a lead's beat
  notes are chord tones (the Desmos corpus: 0.72-0.75). The arc chooses modules by need (an intro quiet, a peak dense)
  and marks seams with rolls, risers, cascades and falls. `target\release\luminesce-stream.exe --seed N --out x.wav`
  renders any stretch headless. Details: `docs/composer.md`, "The move book".
- **Song species (2026-11-03).** Every stream's function looked nearly the same, with only small changes; it needed
  radical changes in its constraints and in what is possible. Every stream now
  starts by choosing a **species**, a universe of constraints with its own tuning, meter, generator family, form,
  timbre and tempo, written as its own book (`src/compose/species/*.txt`) with its own definitions, so the equation
  itself looks different. Nine:

  | species | tuning | meter | generator | timbre | bpm |
  |---|---|---|---|---|---|
  | `lens` (the move book) | 12-tone modes, `h(d)` | 4/4 | blancmange, digits, bits | the pluck, 808 bass | 92-106 |
  | `ratios` | just intonation, `J(r) = Jz + 12 log2(r)` | 3/4 | the harmonic series of each bar's fundamental | pure sines, glass | 80-100 |
  | `chaos` | major pentatonic `pn(d)` | 7/8 (2+2+3) | the logistic map `F(x) = r x (1 - x)`, `iter` | plucks | 110-128 |
  | `automaton` | whole tone `wt(d) = Wk + 2d` | 5/4 | elementary CA, `ca(Wr, x, 10)`, one row a bar | chip squares | 84-104 |
  | `loops` | 19-tone equal temperament | none: loops of 5, 7, 11, 13 ... beats | incommensurable loops (Eno) | sines, soft bells | 60-72 |
  | `tritave` | Bohlen-Pierce, 13 steps to 3:1 | 12/8 | a Lissajous figure in 3:5:7 | odd partials only | 76-92 |
  | `bounce` | quarter tones, maqam | half time | bouncing balls, a damped pendulum, springs | marimba-like | 120-140 |
  | `oneline` | semitone chord roots | 4/4, 8 steps a beat | one curve heard as bass, chords and melody (streaming) | chip squares | 150-170 |
  | `dialogue` | minor pentatonic `pt(d)` | rubato `v = t + 0.32 sin(tau t / 8)` | a call and its inversion, echo or answer | voice lead, pluck | 76-92 |

  A seed picks the species (weighted toward the two built on what was kept: the lens, and the dialogue's voice over
  stillness); a species stream may change species, rarely (three times in ten at a later arc's start), to one whose
  tempo fits. A species' meter lives in its functions: every bar is four beats of `t`, divided into 3, 5, 7 or 12 as
  the species says, so the stage stays eight bars and the seams stay seamless; the loops and the automaton run on the
  stage clock `S` (`w = t + 32 S`) so they never restart at a seam. `--species NAME` (in `luminesce-stream`) plays one;
  `--book` (and `LM_STREAM_LENS` in the C ABI) the lens alone. Streams also **open many ways** now (each species lists
  its openings: a lead alone, the pads and a perc, the counter-line and the hats ...), the opening bass is drawn from
  the whole bass set, and a bass that has played one module through three stages is swapped (repeated testing kept
  hearing the same bassline). Details: `docs/composer.md`, "Song species".
- **Harmony is written, never generated (2026-11-03).** A loops stream whose chord root cycled by
  `mod(c + m, 5)` out of step with the bar sounded bad, and the progression was the cause. Every 12-tone species now plays
  one of twenty progressions written by hand in `src/compose/harmony.txt` (the Royal Road, the marusa changes, ii-V-I
  with a vi-ii-V-I turnaround, iii-VI7-ii-V, a borrowed ivm6, the backdoor bVII9, the tritone substitute, the minor
  climb, a Picardy turn, Dorian and Lydian vamps, a chromatic line cliché, a deceptive cadence once a song, ...), 7ths
  and 9ths throughout, chords on bar lines (one or two a bar, or held), every phrase ending on a dominant into the
  tonic, so every stage seam is a cadence. `src/compose/harmony.rs` voices each chord for three pads with the smallest
  movement from the last (no crossing, fixed registers) and writes it as digit tables the equation reads (`Hb` the
  bass, `Hx Hy Hz` the pads, in the song's key `K`); the species' maths writes the melodies, rhythms and shapes on top
  (`hs`, `sc`, `Hc`), with chord tones on the strong beats. The default **Stream** plays the six species with written
  harmony (lens, dialogue, chaos, automaton, bounce, oneline); the strange tunings (19-tone loops, Bohlen-Pierce,
  quarter-tone `maqam`, just `ratios`) are behind **Explore (strange tunings)** in the Library, `--explore` in
  `luminesce-stream`, `LM_STREAM_EXPLORE` in the C ABI. Details: `docs/composer.md`, "Written harmony".
- **Songs end (2026-11-03).** A book stream scores its own closure after each stage (arcs completed, novelty spent,
  changes tiring, home regained, peaks reached) and ends at an arc's end once it has said what it has to say, never
  before 2.5 minutes and always by 7: a coda brings back the opening's progression and first melody, cadences IV V I
  under a final ritard, strikes one last kick and rings out; then the stream stops (`lm_stream_ended`), and the world shows
  the ending (below). Details: `docs/composer.md`, "The ending". Stream from G and `--stream-song` never end.
- **Stages, in arcs (v2, 2026-11-03).** Each stage is one eight-bar equation (32 beats, its own time `t` from 0), and
  every stage is a change you can hear. Stages come in arcs of six to eight: an intro, one to three builds, a peak, a
  breakdown, a turn and a return; the next arc opens thinned to three layers with a new idea (a new melody generator,
  a modulation or a new progression). The stream opens on a low voice and a pulse (G: the bass and the hats) and the
  rest come in. The moves are whole layers and structures: a layer enters or leaves (the pads as one), the melody's
  generator is replaced by another family's (fractal, squares, Lissajous, Fibonacci, primes, Collatz, rule 30, two
  waves, or residues `ak mod m`), the lead or the kick takes a clearly different groove, the chord digits become a
  new progression, the melody moves an octave, half-time or double-time, the lens moves two or three modes, a
  modulation, a breakdown to two layers, the drop (and a roll into it). Each stage's change is measured from the
  functions (layers, pitch classes, onsets, density, register, melody line); a candidate below 1.0 (one layer
  entering) is not taken while one above exists, and the critic picks among those that clear it. Over 250 stage
  changes from five seeds, v1 averaged 0.82 (68% below 1.0) and v2 averages 3.7, none below. **A budget** keeps it
  simple: at most 6 layers and 13 definitions; a layer entering at the cap pushes another out, and definitions
  nothing reads are removed. Tempo never changes, the key only by a modulation, and no stage repeats any of the last
  eight. Details: `docs/composer.md`, "Stream mode".
- **Seamless.** The engine renders a stage or more ahead on a worker thread, carrying every voice's state across the
  seam (phase, envelope, gate, filter, ringing drum hits; the reverb, delay, wow and warmth run on through). Two
  stages played in a row are one long render of both (largest difference 1.5e-8, f32 rounding) (`tests/stream.rs`). Each stage's level is set from its
  peak before it plays and moves to the next over half a second, so nothing clips and nothing steps.
- **The live equation** (E cycles compact, full, hidden; it starts compact): the playing stage's definitions typeset
  as math, with fractions stacked, floor brackets and parentheses sized to their content, raised exponents, and names
  in italic (Cambria, which ships with Windows). **Compact** is one quiet strip in the top-left corner (beside the menu when it is
  open), at most a fifth of the screen high and never over the draw-head: a header (the stage, what changed, the seed) and the definitions
  one line each in a dense block. When they do not all fit it keeps the lines that changed, then the generators, then
  the lines the voices read, then the lens, and says how many it left out; the time names are not shown. At 1080p it
  is 10 px type; at 150% scaling 14 px (about 70% of the earlier 14 and 20 px, 2026-11-03). **Full** is the large panel
  (15 px type at 1080p, also top left first) (stage, seed,
  every definition, the time names, the voices playing). At a seam the old equation crossfades into the new one, and
  the terms that changed light up for about three seconds. The next stage's equation exists in the engine but is
  never shown.
- **Saving** (S): the last four stages heard (the playing one included) become one ordinary song in `examples\`,
  `stream_s<seed>_<first>-<last>.txt`, which the library picks up at once. Each stage's definitions keep their
  number (`M_0`, `M_1`, ...) and their own time `T_i = t - 32 i`, and `Q = floor(t / 32)` picks the stage, so it
  plays the same functions and can be edited like any song.
- **The trail behind** is the real record: the world draws a window over the playing stage and the one before it,
  built on a thread while the stage before plays and swapped in at the seam (about 7 ms of the main thread). The
  wall is framed once for the whole song (-49 to 54 semitones, every stream measured; a note beyond it widens
  the frame), so a seam where a voice enters higher or lower no longer rescales the wall or moves the camera.
- **The history** (2026-11-03: every new song's seed is captured automatically, so nothing is ever lost and any
  song can be gone back to or removed). Every stream that starts is
  recorded at once, as it is composed, into `%APPDATA%\Dreamride\history\<date>_<seed>.txt` (`_2`, `_3` ... when a
  seed plays twice in a day): the seed, the start time, where it came from (a grammar seed, G, or the seed song's
  whole text), the composer's version (`luminesce.dll fnv1a64 <hash> <size>`, a hash of the engine beside the exe),
  the roster, and **every stage's full song text** with its summary, as each stage is composed; then how far it was
  heard, and `complete` when the stream ended on its own. It is plain text (`world\src\history.h` documents the
  format), appended stage by stage on a worker thread, so no frame waits on the disk and a crash loses nothing
  already written. A write that fails is shown at the top of the screen until one succeeds, and the next write puts
  the whole record down again. Nothing is pruned on its own. See "The History tab" below.
- **The ending.** When a song ends on its own (the engine says so through `lm_stream_ended`, a stage or so ahead,
  with its length, ring-out included), the world plays to the engine's last frame; as the last stage (the ring-out)
  begins, the camera lifts slowly back over **the whole song's trail**, start to end, and holds it; once the ring-out
  has sounded the history marks the entry a finished song with its length, and after a three-second breath a new
  song starts from a fresh seed. The whole trail is every stage merged as a replay merges them, built on a thread
  while the last stages play and swapped in for the two-stage window as the pull-back begins. View, "When a stream
  ends: Start the next song" turns the last step off; the ending card offers **Replay this song** and **Next song**
  either way.

Through the C ABI (`world\src\luminesce.h`): `lm_stream_start` (a seed song or the default, or a seeded species stream;
`LM_STREAM_LENS` = 4 with `LM_STREAM_GRAMMAR` for the lens species alone, `LM_STREAM_EXPLORE` = 8 for a species of
strange tunings),
`lm_stream_pull` (audio blocks), `lm_stream_stage` (a stage's equation markup, the names it changed, what changed in
words, its song text), `lm_stream_window` (the voice record of kept stages, read like a song), `lm_stream_save` and
`lm_stream_error`.

## The examples

```
target/release/luminesce-render examples/citypop_v4.txt citypop_v4.wav        # 40.9 s, renders in ~24 s
target/release/luminesce-render examples/citypop_v3.txt citypop_v3.wav        # 40.9 s, renders in ~11 s
target/release/luminesce-render examples/dark_v3.txt dark_v3.wav              # 26.7 s, renders in ~5 s
target/release/luminesce-render examples/citypop_v2.txt citypop_v2.wav        # 38.4 s, renders in ~9 s
target/release/luminesce-render examples/dark_halftime.txt dark_halftime.wav  # 26.7 s, renders in ~5 s
target/release/luminesce-render examples/graph_takagi.txt graph_takagi.wav    # 40.0 s, renders in ~4 s
target/release/luminesce-render examples/graph_lissajous.txt graph_lissajous.wav  # 38.4 s
target/release/luminesce-render examples/graph_pisano.txt graph_pisano.wav    # 37.6 s
target/release/luminesce-render examples/graph_hanoi.txt graph_hanoi.wav      # 40.0 s
```

**Writing a song with Claude.** The composer's judgment lives in a Claude Code session, not in an algorithm: the skill
`.claude/skills/compose/SKILL.md` teaches a fresh session the song language, the graph way, the taste so far
and the loop (write to `examples\`, render, measure with `tools/measure`, fix, hand over). Ask for a song in this
repository and it loads; the song appears in Dreamride's Library while the window is open.

- **The graph pieces** (`examples/graph_*.txt`) are written the other way round from the city-pop examples. Those
  compose conventional music (note lists, chord charts, a performance layer) and then write it as functions; these
  start from one generator each and let a scale lens make it familiar. Each fits on one screen (11 to 16 short
  definitions, no hand-placed notes), every voice is a graph instrument, and each has shapes a DAW would not draw:
  glides that follow the curve, drums as pitch dashes.
  - **`graph_takagi.txt`**: the Takagi (blancmange) curve, a fractal, read at four time scales 4x apart. Across the
    whole song it is the chord roots (I ii iii IV IV V V V IV V V V IV IV iii ii); the lead is the curve at the phrase,
    the bar and the beat added together, so each beat's figure has the bar's shape. The kick lands wherever the lead
    falls by three degrees, which turns out to be only ever on a beat, but not on every beat.
  - **`graph_lissajous.txt`**: a Lissajous figure sin(p th), cos(q th) whose ratio climbs 2:1, 3:2, 4:3, 5:4, one per
    four bars. The ratio is also the harmony (a drone sounds it as a just interval, 12 log2(p/q), computed by the log
    series) and the rhythm (p hats a beat, q claps a bar). The lead plucks the sum of the two curves, so its range
    swells and shrinks with their interference; the mode lens brightens from Dorian to Lydian as the ratio narrows.
  - **`graph_hanoi.txt`**: the Tower of Hanoi's move sequence, the ruler function (how many times 2 divides k: 0 1 0 2
    0 1 0 3 ...). The chord roots fall by the ruler of the bar and come home every other bar, the lead climbs the
    ruler of the eighth over the chord's fifth, a disc voice glides from move to move, the kick lands where a big disc
    moves, and the mode walks Dorian, Aeolian, Ionian, Mixolydian by the digits of 3254. Written by a session that had
    only the compose skill, as its test.
  - **`graph_pisano.txt`**: the Fibonacci numbers modulo m, which repeat with the Pisano periods 3, 8, 12, 16 and 20,
    so the voices cycle against each other. Mod 7 the second half of the period is the first negated, so every
    two-bar melody, and the sixteen-bar form, is answered by its own inversion; mod 8 inside the bar is a 3-3-2 kick.

  `tests/example_graph.rs` checks these claims from the files' own functions. Measured with `tools/measure`, the
  three sit in the reference recordings' range: band energy (<150, 150-500, 500-2k, 2-6k, >6k Hz, %) 43/29/26/1.7/0.3,
  42/40/18/0.8/0.3 and 38/38/23/1.0/0.2 against the references' 41/37/20/1.6/0.2 and 60/26/13/0.9/0.3; spectral peaks
  per frame 110, 106 and 95 against 108 and 143.
- **`examples/citypop_v4.txt`** is v3's music, step for step, played by graph instruments: every
  tonal voice a sine sum under the Desmos cap, the kick, snare and hat as dashes, no filters or
  detune. The timbres and the kick come from measuring the two reference recordings: lead partials
  at h1 0, h2 -22, h3 -14, h4 -27, h5 -25 ... dB; a kick falling from several hundred Hz to 60-80 Hz
  with a 23 ms dash time; no noise in the drums; a bass that is an octave pair. The file's header
  gives the measurements; `docs/research/graph_sound.md` gives the method and what was not settled.
  `tests/example_music_v4.rs` checks that its notes, chords, bass and drum patterns are v3's.
- **The v3 examples** keep v2's structure and add expression, each written for one feeling, named
  at the top of the file with the choices that serve it. **`examples/citypop_v3.txt`**: wistful
  nostalgia, a night drive remembering someone. An appoggiatura at the hook's peak (D6 over A7b9,
  resolving to C#), a tonic withheld until bar 15, a borrowed minor iv with the one climax note
  falling onto its Db, a beat of silence before the return, and a lead shaped note by note by
  its phrase (arch dynamics, phrase time, High Loud, melodic charge, scoops, late vibrato, sighs),
  in a reverb with a ping-pong echo, tape wow and warmth. **`examples/dark_v3.txt`**: defiance
  with a crack of grief. The motif always pushes up D# -> E, a growl swells per phrase, and for two
  bars the drums stop and the harmony turns to G major: a crying voice sighs down C B A G while
  the 808 sags under it, then the stance returns and lifts to E6 for the first time.
  `tests/example_music_v3.rs` checks these claims against the files' own functions.

- **`examples/citypop_v2.txt`**: sixteen bars of our own in F major at 100 bpm, swung. It is a
  demonstration of musical sense written as functions: a syncopated kick locked to the bass, snare
  ghost notes and hashed hat velocities on one swung grid; 9th and 13th chords with a borrowed
  minor v and two secondary dominants, voiced rootless and sorted in a fixed window (no crossings,
  the loop closes); a bass with octave pops and chromatic approach notes into every change; a
  two-bar call-and-answer hook on chord tones, with anticipations that belong to the next chord,
  space on the downbeats, and a harmony and a lift when it returns; a form of intro, hook,
  breakdown with a filter riser, return and snare fill; pads that re-strike per chord and duck
  under a sidechain read from the kick pattern; and an echo written as the lead a dotted eighth
  late. `tests/example_music.rs` checks these claims against the file's own functions.
- **`examples/dark_halftime.txt`**: the contrast. E harmonic minor at 144 bpm felt in half time:
  an 808 (a driven sine with a pitch punch at each hit and legato octave slides), the snare on
  beat 3, hats as a rate function with triplet and 32nd rolls at phrase ends, one bell motif in a
  3-3-2 rhythm repeated over every chord with its last note lifted at phrase ends, and an 808
  drop-out on the last beat before the loop.
- **`examples/citypop_sketch.txt`** is the step-1 sketch, kept as it was. It is an eight-bar loop of our own at 100 bpm: four triangle pads on
the chord tones of roots stored as the digits of 342258473 (`root`), kept in range with `fold`
and gliding between bars with `smoothstep`; a disco octave bass on the roots; a square lead read
digit by digit from 16-digit integers on a swung sixteenth grid (`sw`), with a harmony a third
above; and kick, snare and swung hats. `examples/citypop_sketch.wav` is its render.

A note on `h`: DESIGN.md gives `h(d) = floor((12d + 2) / 7)`, which yields the natural minor scale
(0 2 3 5 7 8 10 12). The major scale (0 2 4 5 7 9 11 12) is `floor((12d + 5) / 7)`, which the
example uses. A test pins both.

## The world (W2, native)

`world/` is the window: a native C++ host on Direct3D 12 with DXR hardware raytracing (no 3D library). It loads a
song through the engine's C ABI, plays it, and draws it. The original theme is the void:
- The upright wall: time runs along it, pitch goes up.
- Each voice is a glowing tube while its gate is open. Loudness is thickness. Its colour is the voice's, tinted by
  pitch class.
- Each drum hit is a downward dash, its own pitch sweep, at its true height where it fits the wall. The kick is a
  bright streak falling about three octaves, as the reference draws it.
- The picture answers the music:
  - each note's onset flares (its new segment and its draw-head);
  - each drum hit flashes light into the fog and onto the floor;
  - each kick rings the floor's reflection.
  - the bass shakes the water: every voice low on the wall, near the floor, raises standing waves whose ripple
    size is its pitch (see "The bass on the water" below).
- The camera rides with the present, off to the side and looking back; nothing ahead is drawn. By default it
  holds a steady rail with a slow drift; Theme, Camera breathing brings back the breathing with the song (closer and
  lower when it is quiet, back and up when it is dense, slowly, with a small sway on the beat).
- The past fades into fog, and dust drifts through the graph's light.
- A dark near-mirror floor reflects the tubes sharply (raytraced, with a faint grid) and takes their light.

Building needs Rust, MSVC (Visual Studio 2022 or Build Tools with "Desktop development with C++") and the Windows
SDK (for `d3d12`, `dxgi`, `xaudio2` and `dxcompiler.dll`). The GPU needs DXR tier 1.1 (inline ray queries).

```
world\build.bat                        # cargo build --release --lib, then cl; output in world\bin
world\build.bat test                   # the UI logic, equation layout and history tests, then --history-test (no window)
world\bin\luminesce-world.exe --stream # stream mode: a new endless song of evolving equations (see "Stream mode")
world\bin\luminesce-world.exe          # opens the window and plays examples\citypop_v4.txt
world\bin\luminesce-world.exe song.txt # any song
world\bin\luminesce-world.exe --library            # opens on the song library; nothing plays until you pick
world\bin\luminesce-world.exe --songs D:\songs     # also list the *.txt songs in another folder (repeatable)
```

**Dreamride** is the program's name: the window, its menu and the desktop shortcut use it. The desktop's
`Dreamride.lnk` runs `luminesce-world.exe --library`; `world\tools\make_shortcut.ps1` makes it again (after the
folder is renamed, say), and `world\tools\make_icon.ps1` draws the icon, `world\assets\dreamride.ico`, which the
build embeds in the exe.

The window opens at once. A song renders on a loader thread (citypop_v4 takes about 5 s) while the void, or the
song already playing, goes on; the library row and the transport say "rendering", and the new song plays when it
is ready.

**The menu overlay** (Dear ImGui, vendored in `world\src\imgui`, MIT) is dark, translucent and minimal. It fades
away while a song plays and nothing is touched (after 2.5 s), with the cursor; any mouse movement or key brings
it back. It has:
- **a transport bar** at the bottom: title, description, time, a seek bar (click or drag; the mouse wheel over it
  scrubs 2 s a notch), previous, play/pause, next, restart, loop, mute and volume, menu, keys;
- **the menu** (Tab or M), on the left, in four tabs:
  - **Library**: every `*.txt` in `examples\` and each `--songs` folder, with its first-line description; click to
    render and play. It refreshes itself: a song saved, added, renamed or deleted while Dreamride runs shows up
    within about half a second, marked **new** until played (the folders are watched with Win32 change notifications,
    scanned on a thread; a folder that cannot be watched is rescanned every 2 s). Picking the playing song after
    its file changed renders it again;
  - **History**: every stream recorded (see "The History tab" below);
  - **View**: the camera (follow, or pull back to see the trail the song has left), fullscreen, the HUD (the
    song and time, faint in a corner, while the overlay is away), and whether a stream that ends starts the next;
  - **Theme**: bloom, fog density, floor roughness, glow, camera breathing, dust and the bass on the water
    (strength, ripple size, ripple decay), live, with Reset to defaults
    (the theme as launched, `LM_THEME` included);
- **the keys card** (H) and a **quit question** (Esc with nothing open).

**The History tab** lists every recorded stream, starred ones first, then the newest: when it started, its seed, how
many stages were heard and for how long (or "A finished song" and its length), and its stages in a few words ("seed
opens with the hat and bass, lead in, new progression, kick in..."). A badge says **recording** (the stream playing),
**replaying**, **finished**, or **older composer** (recorded by another build of the engine). The star beside each
title stars or unstars it. Click an entry to open it:
- **Stages from / to**: the range the two actions below use (all of them at first).
- **Replay** plays the recorded stages exactly, whatever the composer writes now: they are merged into ordinary songs
  a few stages at a time (the same merge as saving, `T_i = t - 32 i` and `Q` picking the stage; a voice that leaves
  rings out on its own functions, and one whose instrument changes at a seam is released there and a second voice,
  `bass_7` for one struck in stage 7, starts fresh, as the stream plays them), each rendered on a
  thread while the one before plays and started one stage early so the voices and the reverb arrive at the seam
  already sounding. With every stage chosen, and when this engine's composer is the one that recorded it, the stream
  then goes on live from its seed: it is played silently up to the stage after the recording (every recorded stage is
  checked against what the composer writes now), and the world switches to it at the seam and records on into the
  same entry. Recorded by another composer, or a recorded stage that now composes differently, and it says so and
  stops at the end of the recording.
- **Save as song** writes the range as one song, `examples\history_s<seed>_<first>-<last>.txt`, which the Library
  shows at once.
- **Star / Unstar**.
- **Delete** asks in place ("Move this stream to deleted?"), then moves the file into `history\deleted\`; **Undo**, at
  the top of the tab, puts the last one back. **Empty deleted (N)** at the bottom removes those files for good, after
  its own confirm. The stream still recording cannot be deleted until it stops.

`--history-dir DIR` keeps the history somewhere else. Headless runs (captures, benches, tests) record nothing unless
it is given.

| Key | Does |
|---|---|
| Space | play / pause |
| R | restart the song (stream mode: the same stream from its first stage) |
| ← → | seek 5 s (held, it repeats) |
| ↑ ↓ | volume (held, it repeats) |
| N  P | next / previous song in the library |
| L | loop on (repeat the song) / off (at the end, the library's next song) |
| Tab or M | the menu; brings a faded overlay back |
| H | the keys card |
| E | stream mode: the live equation compact, full, hidden (in turn) |
| S | stream mode: save the last four stages as a song in `examples\` |
| F11 | borderless fullscreen |
| Esc | closes the keys card, then the menu; with nothing open asks "Quit Dreamride?", and a second Esc quits (any other key stays) |

**Seeking** starts a new XAudio2 voice at the sample asked for (the rest of the song, then the whole song looped
behind it when it repeats), and the picture jumps with it. A seek stays within the current loop, so the trail
behind the new position is the song's own record up to there, the previous loop included once it has wrapped.

Presentation waits for vsync, so the frame rate is the display's refresh rate. Motion follows the audio: the
visual clock runs on the playback position, smoothed between the audio engine's 10-20 ms position steps. The
window is per-monitor DPI aware: it renders at the display's real resolution, and the overlay scales with it.

The headless harness never opens or shows a window, so it is safe to run while anything else is on screen:

```
luminesce-world.exe --capture 12 a.png --capture 26 b.png   # render offscreen at those song times
luminesce-world.exe --capture-menu library 26 ui.png         # the same with the overlay: transport, library,
                                                             #   view, theme, help, confirm or loading
luminesce-world.exe --ui-scale 1.5 --size 2560 1440 --capture-menu help 12 h.png   # the overlay at 150%
luminesce-world.exe --pull-back --capture 30 p.png           # the pull-back camera
luminesce-world.exe --bench 600                              # GPU and CPU frame times, offscreen
luminesce-world.exe --bench 1200 --bench-ui library          # the same with the overlay built and drawn
luminesce-world.exe --bench 1200 --bench-from 33             # the bench from another song time (default 8 s)
luminesce-world.exe --size 2560 1440 --capture 20 c.png      # any resolution (headless default 1920x1080)
luminesce-world.exe --audio-test     # the playback clock against XAudio2 at volume 0, 3 s
luminesce-world.exe --seek-test      # seeks a silent voice and checks where the audio lands, and the end
luminesce-world.exe --present-test   # 30 frames presented to a swapchain whose window is never shown
luminesce-world.exe --stream --seed 4821 --capture-menu equation 37.4 s.png   # seed 4821 at 37.4 s, compact equation
                                     #   (equation-full for the large panel; captures run in time order, the stream plays up to each)
luminesce-world.exe --stream --bench 600 --bench-from 80 --bench-ui equation   # the bench with the equation drawn
luminesce-world.exe --stream-test 42 # stream mode silently (volume 0, no window): the audio never runs dry, keeps
                                     #   time, and the drawn window follows each seam
luminesce-world.exe --history-test   # records a stream (seed 4821, or --seed N) silently, reads it back, replays it
                                     #   against the engine; build.bat test runs it when the exe is built
luminesce-world.exe --replay-test FILE --history-dir DIR   # replays a history entry silently, as the window does,
                                     #   and checks every stretch comes in, then that it goes on live
luminesce-world.exe --history-dir D --capture-menu history-open 26 h.png   # the History tab (history, history-open,
                                     #   history-delete); --stream --capture-menu ending: the ending card
luminesce-world.exe --stream-test 420 --history-dir D   # a whole song silently, to its end and the next song:
                                     #   the ring-out, the whole-song pull-back, the history's finished entry, auto-next
```

The history, measured (2026-11-03, RTX 5080, 1080p, 1200 frames from 8 s): the bench with the History tab open, GPU
p99 1.08 ms, against 1.10 ms with the Library tab, so the tab is within noise. `--history-test`: one recorded stage
replayed is the stage's own audio sample for sample; two stages replayed, 3,301 pitch samples of the second stage
compared, none differ. `--replay-test` on a five-stage recording: the stretches came in at 0.6 s and 37.2 s, and the
stream went on live from stage 6 at 92.1 s, recording on into the same file.

Stream mode, measured on the same RTX 5080 at 1920x1080 from 80 s (stage 5), 600 frames: GPU p99 0.773 ms with the
equation drawn and 0.773 ms without; CPU p99 0.93 ms with it. `--stream-test 42`: no buffer ran dry, the audio
position stayed within 0.25 s of the wall clock, and the two window swaps took 6.6 and 7.7 ms.

Stream v2 (2026-11-03), seed 4821 from 80 s, 600 frames, **measured while another Dreamride window was playing on the
same GPU**: GPU p99 4.10 ms with the compact equation, 3.61 ms with the full panel, 4.28 ms with no overlay; p50 0.61
ms in all three. The control, citypop_v4 from 8 s with no overlay and no code of this change in its path, measured
4.35 ms p99 in the same conditions, against 1.79 ms recorded above, so the p99s here are the shared GPU's, not the
strip's (the strip sits inside the noise). To be re-run with nothing else drawing. `--stream-test 60`: no buffer ran
dry, the position kept pace, and the three window swaps took 10.6 ms on average (at most 12.9).

`LM_DEBUG_VIEW=1`, `2` or `3` in the environment shows the floor's light, its reflection (x10) or the reflection
rays' hit distances instead of the floor.

**The look is a theme.** Every number of the look is a field of `Theme` in `main.cpp`, and `VoidTheme()` is the
original theme. A later theme is another `Theme` value, with no change to the renderer. The fields cover:
- the palette by voice role;
- the fog;
- the floor's finish;
- each response to the music (flare, thickness, pitch tint, drum flashes, ripple, scattering, dust);
- the camera's quiet and dense poses.

To tune a look without rebuilding, override its numbers for one run, for example
`LM_THEME="floorRough=0.01 dustBright=3"` (`ApplyThemeOverrides` lists the names). An unknown name or a bad value
stops the program with an error.

How it is built (`world/src/main.cpp`, `world/shaders/world.hlsl`; shaders are compiled at launch, so `world.hlsl`
in `bin` can be edited without rebuilding):
- **The song as geometry.** `lm_load_file` renders the song (multi-threaded, 5 s for citypop_v4) and returns the
  trace. Each tonal voice's frames with an open gate become runs, simplified (Ramer-Douglas-Peucker, 2.5 mm). A run
  breaks where the pitch jumps by more than 0.75 semitone in one frame: a new note, not a glide. The runs become
  tubes of 8-sided rings. Brightness is the square root of the level against the voice's loudest. The wall spans
  the tonal voices' pitch range over 5.6 m. A drum's dash is drawn at its true pitch height when its sweep fits
  the wall (citypop_v4's kick: 2.6 m; its snare: 1.8 m). A sweep that does not fit (a hat at 7.6 kHz) is squeezed
  and clamped into the wall. Colour, thickness and depth come from the voice's name, matched against the theme's
  roles (bass, pad, key, lead, kick, snare, hat; other names get a spread of hues). Each ring's radius follows its
  brightness. Each vertex keeps where its note began and its pitch, for the onset flare and the pitch-class tint.
- **Time.** The present is always x = 0: the song's one BLAS is placed by a two-instance TLAS (this loop and the
  previous one), rebuilt every frame. Tubes ahead of the present are discarded in the pixel shader and rejected
  during ray traversal, so they are neither drawn nor reflected. Fog length follows tempo: 16 beats at 120 bpm.
- **The floor.** A near-mirror: four inline ray queries per pixel at fixed rotated-grid offsets. The offsets never
  change, so a line a few millimetres thick is antialiased in the reflection without shimmering. The floor is the
  plane y = 0, so each pixel's floor point comes from the eye ray alone. The gloss is a Gaussian blur, long along
  the view and narrow across, sized by the hit distance and the theme's roughness. It fades out below a pixel, so
  at the void's roughness most of the reflection stands unblurred. Fresnel (Schlick, F0 0.3) and a faint grid.
  A kick rings the floor: a travelling wave packet from under the kick tilts the normal that the reflection rays
  and the Fresnel use. The floor's light is the closed-form irradiance of every recent tube segment as a line
  light, integrated in a compute pass into a floor texture. Segments are eighth-beat for the last four beats and
  half-beat for the older past, both on the song's beat grid; a segment never spans a note change.
- **The bass on the water** (`world/src/water.h`, `BuildBassWaves`, `FloorNormal`). Every tonal voice shakes the
  floor as hard as its live loudness (the level's square root against its loudest) times its coupling,
  `exp(-height / bassWaveReach)`: a bass just above the water rings it, a lead high on the wall barely touches it.
  The pattern is the note's pitch, by the water's own physics at one visual scale:
  - a surface shaken at f answers at f / 2 (Faraday waves);
  - that frequency's wavenumber solves the capillary-gravity dispersion `w^2 = g k + (sigma/rho) k^3` for water;
  - the drawn wavelength is that times `bassWaveScale` (90): 55 Hz is drawn 0.83 m, 110 Hz 0.50 m, 220 Hz 0.31 m.
    A higher note always draws finer ripples; an octave up is 0.6 to 0.63 times the wavelength (f^-2/3 where
    surface tension rules). A glide tightens or loosens the rings as it sounds.
  - Time is slowed 4 times (55 Hz moves at 6.9 Hz on screen) and capped at 10 Hz, so a crest never jumps more
    than a sixth of its wavelength a frame; the crests move at the phase speed, the ring's front at the group
    velocity (1.5 times it for capillary waves). Phase and front are integrated over the trace from the note's
    onset, so every frame and capture at one moment agree, and a glide stays continuous.

  Each note is one source, centred under where its draw-head stood when it began: a note starting sends a fresh
  ring out, and the rings recede with the past like the kick's. The waves lose an e-fold every `bassWaveDecay`
  seconds (1.2), spread as 1/sqrt(r), and stop being driven when the note stops (an inner edge follows the front
  out). The six strongest notes are drawn and summed with the kicks' rings into one surface, which tilts the normal
  the reflection rays and the Fresnel use. Where a wavelength spans fewer than a few pixels it fades rather than
  shimmer. Theme: `bassWaveAmp` (0.009 m; 0 turns it off), `bassWaveScale`, `bassWaveDecay`, `bassWaveReach` (1 m).
  Captures: `world/captures/water_bass_heavy.png`, `water_bass_glide.png` (mid-slide, dark_v3 at 14.82 s) and
  `water_bass_after_kick.png`.
- **The music's response.** Measured from the same trace as the sound, at the same clock:
  - A note's onset flares: its whole new segment, through the vertex's note start, and its draw-head, found by
    walking the trace back to where the note began.
  - Drum hits are point lights for a moment. Each kick is a ring on the floor.
  - Light in the air is the closed-form single scattering of the present (the sounding heads together) and of the
    three latest drum flashes, added along every eye ray.
  - Dust: 3,000 motes fixed in the world, so they recede with the past. They are lit by the floor's light texture
    and the flashes, behind the present only, a fixed pixel size so none crawls.
  - The camera's energy curve is precomputed: every voice's brightness plus drum density, through two cascaded
    8-beat lags (so its velocity is continuous too), normalised (a stream by one fixed scale, so a seam never
    rescales it). Off by default (breathing 0). It mixes the quiet and dense poses. A two-beat
    pendulum of a few centimetres grows with the energy.
- **The image.** 8x MSAA into RGBA16F, resolved with 1/(1+luminance) weights so bright thin lines keep their
  edges. Tubes never get thinner than about a pixel; a distant tube widens and dims by the same ratio, so lines
  neither vanish nor crawl. Then a 13-tap downsample / tent upsample bloom, ACES (Hill's fit), a vignette and a
  one-step triangular dither.

Measured on an RTX 5080 at 1920x1080, 8x MSAA, citypop_v4, 1200 frames (`--bench 1200 --bench-from T`):
- from 8 s: 0.84 ms a frame on average, p99 1.79 ms;
- from 22 s: 0.98 ms, p99 2.06 ms;
- from 33 s, the densest part: 1.05 ms, p99 2.15 ms.

With the bass on the water (2026-11-03, same machine and settings): citypop_v4 from 8 s 0.89 ms, p99 1.28; from
33 s 1.07 ms, p99 1.52; dark_v3 from 13 s (a bass-heavy stretch) 0.76 ms, p99 1.17, against 0.75 ms, p99 1.28 with
it off (`LM_THEME=bassWaveAmp=0`): within the run-to-run noise.

With W2's additions but before the floor light's coarser old segments, the dense part took 2.1 ms (p99 3.06 ms). The 1/8-beat floor light
alone cost 1.3 ms there; W1's figure had been measured at 8 s only.

## Layout

```
src/expr.rs      lexer, parser, name resolution, constant folding, evaluator
src/song.rs      the song file format
src/expressive.rs  the expressive built-ins: arch, phrase, ritard, charge, vibrato
src/render.rs    oscillators, envelopes, note time, drums, sends, mixing and normalisation
src/effects.rs   Freeverb, the ping-pong delay, wow and warmth, and loop-wrapped tails
src/wav.rs       the WAV writer
src/bin/luminesce-render.rs   the command-line tool
src/ffi.rs       the C ABI the world loads (luminesce.dll): load, audio, voices, the trace, and stream mode
src/stream.rs    stream mode's renderer (state carried across seams), worker, windows and saving
src/compose/     the composer (grammar, critic, search), stream.rs (the chain of evolving eight-bar equations,
                 in arcs; seeds/ holds G), audible.rs (how audible a stage change is) and display.rs (equations
                 as typeset markup)
world/           the native world (W2): build.bat, src/main.cpp, src/luminesce.h, src/math_layout.h (the live
                 equation's layout), src/history.h (the stream history: its file format, merging recorded stages
                 into a song, the list, deleting and the writer thread), shaders/world.hlsl; tests/ holds the
                 UI, equation layout and history tests
tests/           expression, song format, reference formulas, render, WAV, example and CLI tests,
                 expressive.rs and effects.rs (the expression maths, note time, the effects),
                 graph.rs (additive voices, the roll-off and the dash), and example_music.rs,
                 example_music_v3.rs, example_music_v4.rs, example_graph.rs: the examples' claims, from their functions,
                 and ffi.rs: the trace and the C ABI; stream.rs: the stage chain, seams, saving, the markup and
                 the stream's C ABI
examples/        citypop_sketch.txt (and its render), citypop_v2.txt, dark_halftime.txt, citypop_v3.txt,
                 dark_v3.txt, citypop_v4.txt, graph_takagi.txt, graph_lissajous.txt, graph_pisano.txt, graph_hanoi.txt
tools/measure/   Python measuring scripts (dev only, need numpy): band energy, peak density, partials,
                 hit pitch tracks
```
