---
name: compose
description: Write, make, generate or vary a Dreamride (Luminesce) song the graph way, as a few math functions of time, then render, measure and fix it. Use when asked for a new Dreamride song, a graph song, a variation of one, or "another one like X".
---

# Composing a Dreamride song the graph way

You are the composer. There is no algorithm here that carries the taste; you do, by following these rules and by
reading what the author kept. Work in the repository root. A song is one plain-text file; the Dreamride window
lists every `*.txt` in `examples\` and picks up a new one while it runs.

If `.claude/skills/compose/taste.local.md` exists (it is kept local and never published), read it first: it holds the
author's exact words behind the preferences below.

## The taste so far (from listening tests, 2026-11-02 and 2026-11-03)

- Songs must sound like graph music, not a MIDI file in a DAW: unique, abstract sound and shape coming out of a graph,
  made familiar, rather than conventional music written as functions.
- Correct structure is not enough: a song needs emotion (the reason for the expression layer).
- The graph instruments (v4, additive sine partials) worked.
- The kick should be a fast downward pitch dash.
- The first algorithmic composer worked but fell well short of the hand-written songs. One of its songs was kept,
  G = `generated/20261103-004208/5_s7-c38_squares.txt` (binary counting by square waves), liked for being simple in
  an odd way and for its high plucked lead. That pluck is its `voice lead`: additive, partials `0, -4, -6, -12, -10, -16,
  -15, -21`, attack 5, decay 300, sustain 0.55, release 150, gliding into the next note over the last quarter of each
  step. Reach for it as a lead.
- dark_v3 (2026-11-03) was loved for the moment at 20 seconds where a wavering lead imitates a voice. That is bars
  13–14, the planned "crack of grief": drums stop, the one major chord, a crying lead with late vibrato sighing
  C → B → A → G while the 808 sags. A voice-like lead (late vibrato, falling sighs) over a sudden stillness lands.
- Stream v2 (2026-11-03): the way it changes over time was judged intelligent and the kick good, but the content
  was flat and too much like MIDI (it improved with repeated listening). The saved `examples/stream_s955576_3-6.txt`
  was liked for a simple bass under a lead that wanders in an interesting way. That lead is a Takagi-style fractal of triangle
  waves read at three time scales, gliding over the last 40% of each eighth, held legato, in the pluck timbre, over a
  simple bass: a wandering self-similar curve over a plain floor.
- Basses (2026-11-03): fluid, moving, more complex basslines, never the root on every beat. The root on every beat is
  out. Write bass lines that move:
  syncopated bit-pattern rhythms, ghost notes, octave jumps, approach notes into chord changes, glides and slides.
- Harmony (2026-11-03, on a loops stream saved as `examples/stream_s808394_3-6.txt`): it did not sound good, and the
  cause was the chord progression itself, not the sound quality. That stream's chord root cycled by `mod(c + m, 5)`, changing every 11 beats, out of step with the bar: harmony
  by arithmetic, no tension, no release, no cadence. The one generated piece judged alright (citypop_v4) ran on real
  functional harmony. **Harmony is written, never generated.** Take a progression from `src/compose/harmony.txt` (or
  write one with the same care: Roman numerals, 7ths and 9ths, a dominant at every phrase end resolving to the tonic),
  and let the maths write only the melody, the rhythm and the shapes on top of it.
- The reference is @measure_plan (Alan, Stereo Drift): graphing-calculator city pop. Learn the principles; **never copy
  his songs** or transcribe his melodies.

If `generated\ratings.txt` exists, read it first: it is the author's keep/skip history. Lean toward what was kept,
away from what was skipped, and say which kept song you leaned on.

## Harmony is written, never generated (since 2026-11-03)

- **The progression is chosen, not computed.** No `mod(c + m, 5)` roots, no digits picked for their pattern, no chord
  root that follows a generator. Every 12-tone stream species plays one of the progressions in
  `src/compose/harmony.txt` (20 of them: the Royal Road, the marusa changes, ii-V-I, the backdoor, the tritone
  substitute, the minor climb, a Picardy turn, Dorian and Lydian vamps, a chromatic line cliche, ...).
- **Chords change on bar lines**, one a bar or two (each then half a bar), or one held over two bars. Phrases are 4 or
  8 bars; each ends on a dominant (V in any colour, V7sus, viiø7, bII7, bVII7) and the next begins on the tonic, unless a
  deceptive turn is marked, once in a song.
- **Voice it properly.** 7ths and 9ths (11ths and 13ths where the mood allows); three voices spread across a tenth or
  more; each chord the nearest voicing to the one before (the smallest total movement, no crossing); the bass on the
  root or an inversion written on purpose (`I/3`, `V/3`, a falling line `Imaj7/7`).
- **The maths sits on top.** Melodies take a chord tone on the strong beats (1 and 3) and wander in the scale between;
  basses outline the progression's bass notes with approach notes into each change; arpeggios climb the voicing.
- In a hand-written song, write the progression's tables yourself the way the stream does (one digit a half bar), or
  copy a progression module's lines from a stream's saved stretch; either way the chords are a chart you chose.
- The stream's equation shows it as `Hb` (the bass on the circle of fifths, `Hm`/`Ho` its step and origin) and
  `Hx Hy Hz` (the pads, low to high), read by `Hr(i)`, `Hv(m, i)`, `Hc(m, i)` (the chord's ladder), `sc(x, i)` (the
  chord tone nearest x) and `hs(d, k, p)` (degree d, pulled to the chord on the strong beats), in the key `K`.

## Species (since 2026-11-03)

The stream composes in **species** (`src/compose/species/*.txt`; `docs/composer.md`, "Song species"). The default
stream plays the six that carry written harmony in 12-tone: the lens, the logistic map in 7/8, a cellular automaton in
5/4, bouncing balls, one curve heard as a band, and a call and response on a breathing clock. The strange tunings (just
intonation in 3/4, Eno loops in 19-TET, Bohlen-Pierce in 12/8, the quarter-tone maqam bounce) are behind the
Explore option, where the harmony is the tuning's own. The rules below are the lens's. A hand-written song may take another species' universe instead (its tuning
function in place of `h`, its meter as a division of the four-beat bar); say which in the header. Radical changes in
the constraints are wanted: a new song may be the place to try one.

## The graph way (rules)

1. **One screen.** About 8-16 short definitions (time names such as `n = floor(4t)`, `e = floor(2t)`, `k = floor(t/2)`
   do not count), then the blocks. Fewer is fine if every voice still derives from them; never pad the count. No note lists, no hand-placed notes, no
   per-note expression layer. If you are typing a melody, stop: find the function that makes it.
2. **A few generators make everything.** Pick one or two and derive every voice from them, at several time scales:
   - a fractal read at the song, the phrase, the bar and the beat (Takagi `T(u) = sum tri(2^k u)/2^k`, Weierstrass sums);
   - the bits of an integer as rhythm (`mod(floor(N / 2^k), 2)`), digits of an integer as a table (a written progression is stored this way: chosen, never picked for the pattern)
     (`mod(floor(N / 10^(7 - i)), 10)`);
   - ratios as harmony and rhythm at once (p:q is a just interval, p hats a beat, q claps a bar);
   - integer sequences mod m (Fibonacci mod m repeats with the Pisano period; `k^2 mod p`, `floor(k phi) mod m`);
   - curves (Lissajous, Lorenz-like sums of sines, a spiral's radius) sampled on a grid.
3. **A scale lens makes it familiar** (under a written progression, rule "Harmony is written" above, whose mode sets c).
   Generators give degrees; `h(d) = floor((12d + c) / 7)` turns degree d into
   semitones. c picks the mode: 2 Aeolian (natural minor), 3 Dorian, 4 Mixolydian, 5 Ionian (major), 6 Lydian; c = 0
   Locrian, 1 Phrygian. Let c change by section (a mode shift is the cheapest change of light). Chords are stacked
   thirds of the lens: `h(R)`, `h(R + 2)`, `h(R + 4)`, `h(R + 6)`.
4. **At least one continuous shape a DAW could not draw**: a voice that glides along the curve itself, a range that
   swells with interference, a pitch that follows a fractal unquantized and is only pulled to the scale by a glide lens
   `g(u) = h(floor(u)) + (h(floor(u) + 1) - h(floor(u))) smooth((frac(u) - 0.75) / 0.25)`.
5. **Drums are downward pitch dashes**, never noise: `drum` with `kind = dash`, or a voice whose pitch falls fast
   (`pitch = 24 - 60 frac(t)` with a short gate). Their triggers come from the generators too (a kick where the lead
   falls, where `F mod 8 == 0`, where a bit is set).
6. **Every tonal voice is a graph instrument**: `wave = additive` with `rolloff = 660`. No filters, no detune.
7. **Abstract made familiar, and felt.** The soul is not added on top; choose a generator whose shape means something
   (falling and returning, answering itself by inversion, a ratio narrowing toward unison) and make the form follow it:
   an entrance per 8 or 16 beats, a mode that darkens then brightens, one place where the curve's peak meets the
   brightest harmony (a major chord in the brightest mode). Say in the header what feeling the shape carries.

## The song language (exact; checked against `src/song.rs`, `src/expr.rs`)

```
# One-line description: the Library shows this first line. Then a few comment lines: the idea, then "Measured: ...".
bpm 96                 # settings have NO '='. Required: bpm (0, 1000], beats (> 0, at most one hour)
beats 64               # 64 beats at 90-110 bpm is 35-43 s, the usual length
loop 1                 # the song is a loop: reverb and delay tails wrap round the seam
reverb_size 0.7        # 0..1        reverb_damp 0..1   reverb_width 0..1   reverb_predelay 0..500 ms
delay_beats 0.75       # 0.01..16    delay_feedback 0..0.95   delay_tone 100..20000 Hz
warmth 0.3             # 0..1        wow 0..100 cents, wow_rate 0.01..20 Hz   rate 8000..192000   reference (A4 Hz)

tri(v) = abs(v - floor(v + 0.5))      # definitions use '='; any order; up to 8 parameters
n = floor(4t)                          # a constant may use t
```

Expressions: numbers (`3`, `.25`; **no** `1e3`), `t` (beats), `pi`, `tau`, `+ - * / ^` (`^` right-assoc, `-2^2 = -4`),
comparisons `< > <= >= == !=` give 1 or 0 and cannot chain. **Implicit multiplication**: `12d`, `2 sin(t)`,
`(a)(b)`, `(t >= 16)(0.5 - f)`. A name followed by `(` is always a call, so write `d*(x + 1)`, not `d(x + 1)`.
Built-ins, and only these: `floor ceil frac abs sign sin cos sqrt` (1 arg), `mod min max` (2; `mod` is floored,
`mod(-1, 12) = 11`), `clamp(x, lo, hi)`, `if(c, a, b)` (a when c > 0), `smooth(u)` (smoothstep on clamped u),
`db(x) = 10^(x/20)`, `arch(x, peak)`, `phrase(c, depth, peak)`, `ritard(x, q, w)`, `charge(semis)`,
`vibrato(age, delay, ramp, depth, rate)`, and (since 2026-11-03) `log2(x)` (a just interval is `12 log2(3/2)`),
`iter(F, x0, n)` (definition `F` of one parameter applied `floor(n)` times to `x0`, n at most 4096: an iterated map,
`F(x) = r x * (1 - x)`, `X(n) = iter(F, 0.3, n)`) and `ca(rule, row, width)` (one step of Wolfram's elementary
cellular automaton on a ring of `width` cells held as the bits of `row`). There is **no exp, ln, tan, atan, pow,
random**: use `^`, `log2`, or a sine hash `frac(sin(k * 12.9898) * 43758.5453)`.
No recursion (an iterated map of one value goes through `iter`), no arrays, no state: tables are digits of integers
below 2^53. `e` is not a constant (free as a name).
`note_age` and `note_left` (seconds) may be read only in a voice's `pitch`, `amp`, `cutoff`.

```
voice lead {                          # names: letters, digits, _ and -; unique across all blocks
  wave = additive                     # sine | triangle | saw | square | additive
  partials = 0, -16, -4.5, -15, -11   # dB per harmonic from h1, 1..64 of them, -120..20 (additive and dash only)
  rolloff = 660                       # each partial x min(1, 660/Hz): the Desmos tone() cap (sine/additive/dash)
  pitch = h(L(n)) - 2                 # EXPRESSION: semitones from A4 (0 = 440 Hz, -9 = C4, -33 = C2)
  gate = (t >= 16)(0.75 - frac(2t))   # EXPRESSION: sounds while > 0; each rise from <= 0 is a new note
  amp = 1                             # EXPRESSION, per sample (default 1)
  gain = 0.2                          # numbers from here on: 0..10
  pan = 0.1                           # -1..1
  attack = 5                          # ms (0.1..10000); decay 100; sustain 0..1 (default 1); release 30
  reverb = 0.3                        # sends 0..1; delay = 0.25
}                                     # also: cutoff (expr), resonance, filter_env, filter_decay, drive, detune (unused here)

drum kick {
  kind = dash                         # dash | kick | snare | hat (the last three are the older built-ins: do not use)
  trigger = (mod(floor(4t), 8) == 0)(0.5 - frac(4t))   # EXPRESSION: a hit on each rise from <= 0 to > 0
  amp = 1                             # EXPRESSION, read at the onset (velocity)
  from = 450                          # Hz at onset; to = 58 Hz it settles at; dash = 23 ms per 1/e of the gap
  to = 58
  dash = 23
  hold = 40                           # ms at full level, then decay ms per 1/e, cut at length ms
  decay = 90
  length = 380
  partials = 0, -24, -12, -20, -19
  rolloff = 660
  gain = 0.5                          # pan, reverb, delay as for voices
}
```

A constant may use `t` and may be read inside a function (`h(d) = floor((12d + C) / 7)` with `C` changing by section).
Two notes in a row need the gate to fall to 0 or below between them, or the second is a glide inside one note.
Watch implicit multiplication: a term followed by a name or `(` multiplies, but a term followed by a number does not
(`Q(j) 8` and `2 2^u` are errors: write `8 Q(j)`, `2 * 2^u`), and `x (1 - x)` is a call of `x` (write `x * (1 - x)`).
`0.75 - frac(2t)` gives eighths; `0.97 - frac(t / 4)` re-strikes a pad every bar; a legato line is
`gate = 1` (or `gate = (t >= 16)`) with a gliding pitch. Product gates switch sections on: `(t >= 16)(...)`, `(floor(t/16) != 2)(...)`.

Drums (`kind = dash`) are not tonal voices: no `wave`, only the dash fields. **The instrument kit** (measured from the reference recordings; reuse, vary slightly):

| role | partials | notes |
|---|---|---|
| lead | `0, -16, -4.5, -15, -11, -18.4, -16.1, -22.9, -22.9, -28` | gain 0.16-0.2; most notes between 0 and +20 (A4-F6): lower, it crowds 150-500 Hz |
| pad / chord tone | `0, -12, -5, -11, -9, -12, -11, -13, -12, -14, -13, -15, -14, -15, -15, -16` | gain 0.05-0.09 each, 3-4 voices, roots around -14; plus 1-2 quiet copies an octave up (gain 0.03) |
| arp / pluck | `0, -4, -6, -12, -10, -16, -15, -21` | attack 2, decay 120, sustain 0.3 |
| bass (octave pair) | `-11, 0, -26, -12, -28, -20, -32, -21` | gain 0.24-0.3, pitch around -36 |
| kick dash | from 450, to 58, dash 23, hold 40, decay 90, length 380, `0, -24, -12, -20, -19` | gain 0.5-0.7 |
| clap dash | from 820, to 190, dash 9, hold 6, decay 55, length 260, `0, -18, -9, -16, -15, -21, -21, -25` | gain 0.4 |
| hat dash | from 7600, to 5400, dash 5, decay 12, length 70, `0, -10` | gain 1.3-3 (the cap makes it quiet) |
| voice clap | `0, -6, -9, -12`, `pitch = 24 - 60 frac(t)`, `gate = (mod(floor(t), 2) == 1)(frac(t) < 0.09)` | gain 0.6, attack 1, release 20 |

## The loop

1. **Read** `generated\ratings.txt` if it exists. Pick a generator you have not used in `examples\graph_*.txt`
   (list them; read one only to see the format, never to copy its idea).
2. **Write** `examples\graph_<idea>.txt`: description line, a short header (the generator, how each voice reads it,
   the feeling), settings, 10-16 definitions, blocks.
3. **Render** (from the repo root; the release exe already exists, build only if it is missing):
   ```
   target\release\luminesce-render.exe examples\graph_<idea>.txt examples\graph_<idea>.wav
   ```
   A parse error names the line and column; a NaN or a pitch above Nyquist names the voice and the beat. Fix and
   re-run. Scaled down by up to 3 dB is fine; more, lower the loudest gains. (An `unknown field` or built-in that this
   file says exists means the exe is older than the source: `cargo build --release`.)
4. **Measure** (Python with numpy is `py` on this machine):
   ```
   py tools\measure\analyze.py examples\graph_<idea>.wav      # WAV paths only; it has no options
   ```
   Compare with the reference recordings and the kept graph songs:

   | | <150 Hz | 150-500 | 500-2k | 2-6k | >6k | peaks/frame |
   |---|---|---|---|---|---|---|
   | reference "city" | 41 | 37 | 20 | 1.6 | 0.2 | 108 |
   | reference "v" | 60 | 26 | 13 | 0.9 | 0.3 | 143 |
   | graph songs so far | 38-43 | 29-40 | 18-26 | 0.8-1.7 | 0.2-0.3 | 95-110 |

   "Near" means each band within about a fifth of its value (35 against 38 is near), and peaks 90-145. What the numbers mean and the fix:
   - <150 under 35 %: the bass or kick is weak (raise bass gain, lower the bass an octave); over 60 %: boomy.
   - 150-500 over 40 %: the lead, pads or a glide voice sit too low. Move the lead up (most notes above 0) or a glide
     voice up an octave; keep only the pad roots and the bass below -12.
   - 500-2k over 28 %: the lead and upper pads are too loud or too high; lower their gains or pitch.
   - 2-6k over 2 %: bright partials on high notes; drop the lead an octave or trim upper partials.
   - peaks/frame under 90: peaks are counted within 40 dB of the loudest component, usually a held bass. **Trade bass
     gain for kick gain first** (the kick is transient, so the median frame is no longer ruled by the bass; e.g. bass
     0.32 -> 0.27, kick 0.5 -> 0.7), then add simultaneous voices (an arp, an octave-up pad copy, a counter-line),
     which alone moved it little (+3). Lowering the other voices' gains makes it worse. Over 150: crowded.
   - >6k under 0.2 %: raise the hat (gain up to 3 is fine) or reverb_size a little.
   - Raising kick and hat pushes the scaling toward 3 dB; if it passes, lower everything a little rather than one voice.

   **Which voice owns a band**: solo it. Copy the song to the scratchpad, set every other block's `gain = 0` (allowed),
   render, measure. **A section**: slice the WAV (seconds = beats x 60 / bpm), then measure the slice:
   ```
   py -c "import wave,sys;w=wave.open(sys.argv[1]);r=w.getframerate();a,b=map(float,sys.argv[2:4]);w.setpos(int(a*r));d=w.readframes(int((b-a)*r));o=wave.open(sys.argv[4],'wb');o.setparams(w.getparams());o.writeframes(d)" song.wav 15 40 part.wav
   ```
   If the song builds, its full sections should sit in the range, not only the whole-file median.
5. **Listen for the graph, by the numbers you can check**: print a few of your functions' values at chosen beats (a
   tiny `py -c` script that mirrors the definitions exactly: `mod` is floored like Python's `%`, comparisons are 1 or
   0, `^` is `**`) to confirm the roots, the rhythm and the falls do what the
   header claims. A claim in the header that you did not check is a guess: check it or delete it.
6. **Write the measurement** as the header's last comment lines, in this form:
   `# Measured (tools/measure/analyze.py): band energy 40/36/22/1.2/0.2 %, 104 spectral peaks per frame (the`
   `# references: 41/37/20/1.6/0.2 % with 108, and 60/26/13/0.9/0.3 % with 143).`
7. **Hand it over**: tell the user the file is in the Dreamride Library (it appears there by itself, marked "new", within
   a second if the window is open), what the generator is, what to listen for, and the numbers.

## Checklist before handing a song over

- [ ] First line is a `#` comment that says what the song is (the Library shows it).
- [ ] About 8-16 definitions; no note lists; the one chart is a written progression (cadence at every phrase end),
      and every other voice derives from the generators on top of it.
- [ ] A scale lens `h`, with a mode change somewhere, or a stated reason for none.
- [ ] At least one continuous shape (glide, swell, unquantized curve).
- [ ] Drums are dashes; every tonal voice is `additive` with `rolloff = 660`.
- [ ] Renders without error; scaled down by less than about 3 dB.
- [ ] Band energy and peaks within or near the table (peaks 90+); anything outside is explained or fixed.
- [ ] Every claim in the header was checked by a number.
- [ ] Not a copy of any @measure_plan song, nor of an existing `examples\graph_*.txt` idea.
