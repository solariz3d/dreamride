# What people who made function music sound good actually did: a Desmos corpus

Research for the composer, 2026-11-03. The algorithmic composer was judged well short of songs
written with judgment, and one reason is that its grammar and critic were invented from a few examples. This
note reads **64 public Desmos graphs that play songs, beats or self-playing generative music**, chosen from about 250
music graphs found, and asks what the well-received ones share. The raw states are in `desmos_corpus/raw/`, the sources
(author, where found, how it was received) in `desmos_corpus/sources.md`, and the scripts that re-derive every number
below in `desmos_corpus/scripts/` (commands at the end).

Formula fragments are quoted from the graphs' LaTeX, rewritten into plain notation (`\frac{a}{b}` as `a/b`, subscripted
names like `g_{t16}` kept as `g_t16`), with the source hash. They are quoted to show a technique; none is to be copied
into our examples.

## The short version

1. **Almost every well-received song is note data played through hand-built instruments, not a generator.** Of the 27
   graph families I read by hand, 21 store their notes in lists (15 as step lists indexed by a clock, 6 as event lists
   of (pitch, time)). Only 2 make their notes by a rule (both polyrhythms, where the rhythm ratio is also the pitch
   ratio). The generative pieces that map a fractal or a random walk straight to pitch were entered as *sound effects*
   in the one music contest, or scored low on Reddit.
2. **What their authors spent the formulas on is the sound and the groove, not the melody:** additive timbres with a
   tuned spectral tilt (17 of 27), a decay envelope per note (9), falling-pitch kicks and noise-like drums made of many
   random partials (6 with noise drums, 4 with a falling kick in what I read), glides, sidechain ducking, echo copies.
3. **Harmony and melody are separate layers.** A slow chord or bass list (one change per bar or two) and a faster
   melody list, and on the beats **72-75% of melody notes are triad tones of the bass note** (Better Off Alone and
   TETRIS, measured). Pitch and rhythm are often separate lists too (a pitch list and a gate list of different lengths).
4. **The best-received tone() songs are short loops, played all-in.** Better Off Alone (r/desmos 259) is one 4-bar loop;
   SzefRon's loop (121k YouTube views) is four chords on a 32-unit clock cycle; neither builds. Builds and sections
   appear in the longer pieces (Overtime, Transmission, A Mathematical Ballad). Our critic's *structure energy* would
   mark the two most popular down.
5. **The note lists agree with the critic on some bands and not others** (section 4): leads change pitch by step only
   23-77% of the time (the critic wants 50-90%), leap 5+ semitones at most 15% of the time, span 9-12 semitones, and
   repeat a bar 36-58% (the critic wants 45-80%).

## 1. The corpus

| category | graphs | what it is |
|---|---|---|
| cover | 37 | a known piece transcribed into lists (the large majority of Desmos music); Overtime, a reversed re-arrangement, counted here |
| original | 8 | composed by the graph's author: supersaw sequencer, Transmission, A Mathematical Ballad, Mini Desmos Song, SzefRon's loop, "Song", aloboi-style, SynthWave |
| generative | 11 | a rule makes the notes: polyrhythms (4), fractal and Mandelbrot orbits (3), random, Fibonacci and Markov generators, Shepard tone |
| beat | 3 | drum loops (Berrynote's two, the amen break) |
| resynth | 5 | a recording's frequency and amplitude frames replayed (rickroll that "sings", Believer, My Way, Bad Apple, Rush E): they sound like the record because they are the record |

(Six graphs are from before `tone()` and play a curve's height through Desmos's audio trace: five covers and SynthWave.)

**How they were found.** A full pass over r/desmos since tone() arrived (14,000 posts from the Arctic Shift archive,
filtered by music words, then the original posters' graph links), 27 YouTube searches (graph links in descriptions),
the Desmos forum's music threads, GitHub READMEs, the official Desmos Art Contest galleries, and the three result
galleries of the **2026 Desmos Music Challenge** (run by the YouTube channel Mathemagic Land; 16 song finalists, 12
sound effects, 11 instruments). Details and every link are in `sources.md`.

**"Well regarded" means one of:** a finalist place in the Music Challenge, a place in an Art Contest gallery, an
r/desmos score of 20 or more, or 7,000+ YouTube views. 59 of the 64 meet one. These are popularity and curation
signals, not ratings of sound: a "Desmos sings Never Gonna Give You Up" post (401) is popular for the trick. I did not
listen to any graph (section 8).

## 2. The techniques, ranked

Two counts. **Hand** is my reading of the sound expressions of 27 well-received graph families (the three songs on
DIXERION's engine count once): the technique is there and carries the music. **Regex** is a pattern over the LaTeX of
all 59 well-received graphs (`scripts/tech.py`): presence only, and it misses many (its falling-kick pattern found 1).
Where the regex is useless the column says so.

| rank | technique | hand (of 27) | regex (of 59) | examples |
|---|---|---|---|---|
| 1 | **Additive timbre**: the frequency times a list of harmonic numbers, a gain per harmonic | 17 | 42 | jbfiikq4mx, bqrffiubis, 8kxhmzypm8, a9jlvp2yl8, gaowmi4hi0 |
| 2 | **Step lists**: notes in a list, indexed by a clock (`floor` of time, `mod` its length) | 15 | 25 | jbfiikq4mx, 2sil5zcnun, e2iix1hhkk, 3mcvjtyd4q |
| 3 | **A ticker as the clock** (actions advance time each frame, not a slider) | 14 | 34 | 8kxhmzypm8, 2sil5zcnun, 3mcvjtyd4q |
| 4 | **Loops by `mod` of time**: a 4-bar loop, or a few chords on a 32-unit cycle | 11 | 20 | jbfiikq4mx, fuyhyjnedt, vcldbih0wa |
| 5 | **Spectral tilt or a filter on the partials** (gain `1/n^p` with p tuned, or a lowpass curve over n) | 10 | 13 | 8kxhmzypm8, jbfiikq4mx, gaowmi4hi0, idy42pofff |
| 6 | **A decay envelope per note** | 9 | 16 | bqrffiubis, mbokvmxbzr, a9jlvp2yl8 |
| 7 | **Chords as stacked voices**, one list per chord tone (voice leading written out) | 8 | 8 | e2iix1hhkk, gaowmi4hi0, wcq51fhx0m |
| 8 | **Glides** between notes (smoothstep, tanh, or a power-law slide) | 6 | 12 | jbfiikq4mx, bqrffiubis, agcq6pwjzr, 7ielw63oyk |
| 8 | **Noise drums from many random partials** with a fast decay | 6 | 12 | bqrffiubis, 8kxhmzypm8, a9jlvp2yl8, mbokvmxbzr |
| 8 | **Sections that gate layers** | 6 | 8 | bqrffiubis, a9jlvp2yl8, 3mcvjtyd4q |
| 8 | **Event lists** of (pitch, time) or (start, length, pitch) | 6 | 25 | 8kxhmzypm8, a9jlvp2yl8, lonntptn3d |
| 12 | **A gate list separate from the pitch list** (often a different length) | 4 | (no pattern) | jbfiikq4mx, vcldbih0wa, agcq6pwjzr, gaowmi4hi0 |
| 12 | **Kick = a falling pitch** | 4 | 1 (misses) | jbfiikq4mx, bqrffiubis, fuyhyjnedt, mbokvmxbzr |
| 14 | Drum hit-time lists | 3 | | mbokvmxbzr, a9jlvp2yl8, 3bwlb8y8di |
| 14 | Detune / unison copies | 3 | 4 | gaowmi4hi0, idy42pofff, y5tldfmcxx |
| 16 | Echo: the same line read later, quieter | 2 | 3 | bqrffiubis, a9jlvp2yl8 |
| 16 | Arpeggio indexing a chord table; chord = root + interval list | 2 each | | fuyhyjnedt, 3mcvjtyd4q, qeth5bpf0u |
| 16 | Vibrato; a continuous pitch curve; ratio rhythm = harmonic pitch; rests as `0/0` | 2 each | | 7ielw63oyk, petadzc4sb, 4tca7a3nay, w07779vzrr |
| 20 | Sidechain ducking; per-note filter sweep; snare as falling pitch; section transposition; tempo ramp; inharmonic partials; variable step length; intro then loop | 1 each | | jbfiikq4mx, bqrffiubis, a9jlvp2yl8, lonntptn3d, wcq51fhx0m, 8vvg5m9zfr |
| | **Digit or bit tables** (`floor(N/10^k) mod 10`, the reference's trick) | **0** | 0 | none in the corpus |

The last row matters: the trick our grammar is built on (integers as tables) appears in **no** graph here. Desmos has
real lists, so nobody needs it. It is a fine encoding for our language (which has no lists), but it is not what made
anyone's music good.

## 3. The patterns, quoted

### The clock and the lists

- **One function plays any list at any rate, looping** (bqrffiubis, DeeFeeCee, who wrote every line's purpose into the
  graph): `B(a, b) = a[mod(floor(Beat/b), length(a)) + 1]` where `b` is the step in quarter notes. A list of length 1
  repeats every step ("no need to fill a list with 64 instances of the number 1"). `D(a, b, c)` is the same, `c` beats
  later: the echo.
- **The whole song is one 4-bar loop** (jbfiikq4mx, Better Off Alone, r/desmos 259): `g_t1 = mod(t, 1)` is the loop
  phase; the lead is `l_notes[64 g_t1 + 1]` (64 sixteenths), the bass `b_notes[4 g_t1 + 1]` with
  `b_notes = [40, 39, 44, 42]` (one note a bar), and `g_t16 = mod(16t, 1)` is the phase within a beat.
- **Pitch and rhythm in separate lists**: Better Off Alone has `l_notes`, `l_gates` and `l_slides`, all 64 long;
  family mart (vcldbih0wa) reads a 32-entry pitch list at `T/8` and a 64-entry 0/1 gate list at `T/4`; snake eyes
  (agcq6pwjzr) a 64-entry pitch list against a 256-entry gate list.
- **Intro, then loop** (8vvg5m9zfr): `t_music = {10t < 258: 10t, 258 + mod(10t - 258, 1488)}`.
- **Variable step lengths** (wcq51fhx0m): the ticker advances `a` by `Tempo·dt·B[a+1]/24000`, so each step carries its
  own duration.

### Harmony

- **Voicings written out, one list per chord tone, with a verse/chorus switch** (e2iix1hhkk, silent road): five voices,
  each `s_k = (1 - C)·S_k[mod(T_c, 16) + 1] + C·S_kC[mod(T_c, 16) + 1]`; the verse voicing `S_3 = [14,14,14,10,…,17,…,15]`
  and the chorus voicing `S_3C = [14,…,21,…]`: same rhythm, the upper voices lifted in the chorus.
- **A chord table and a chord-time map** (3mcvjtyd4q, A Mathematical Ballad, an original): `c_ltime = [0,0,1,1,2,0,3,4]`
  picks the chord for each of 8 slots, `c_left` holds the chords as interval triples, and the backing arpeggiates
  `c_left[3·c_ltime[floor(t_1)] + t_0 + 1]` with `t_0 = mod(…, 3)` (three notes per chord).
- **Arpeggio from a chord table, the chord changing every 8 clock units** (fuyhyjnedt, SzefRon, 121k views):
  `chords[mod(2t, 4) + 1 + 4·floor(mod(t/8, 4))]` over a bass `bass[mod(t/8, 4) + 1]`, `bass = [-24, -29, -32, -25]`.
- **Extended voicings** (y5tldfmcxx, Beatuds): each chord is a set like `[13, 10, 8, 7, 3, 1, 0]` (7ths, 9ths, 11ths),
  with a bass note an octave or two below.
- **Chord = root + intervals as functions** (qeth5bpf0u): `ChordMin(…)`, `ChordMaj(…)` take a root and a time window.

### Melody

- Mostly lists (section 4 measures them). Two graphs draw the melody as **a continuous pitch curve**: 7ielw63oyk builds
  each phrase from smoothstep segments `c(x, x1, x2, y1, y2) = y1 + (y2 - y1)(3u² - 2u³)` plus a vibrato whose depth
  is shaped, `0.2/sqrt(1 + (l(x - x1))⁴)·sin(8π(x - x1))`; petadzc4sb sums smoothed boxes,
  `c_1(x) = -2·l(x,4,8) - 4·l(x,8,12) - 5·l(x,12,16) …`.
- **Slides as data** (jbfiikq4mx): `l_note = notes[i+1] + (1 - mod(i,1))⁴·slides[i+1]·slide·12`, a power-law scoop
  into flagged notes.

### Drums

- **Kick: a fast power-law pitch fall, tuned to the bass** (jbfiikq4mx):
  `tone(f(b_note - 12 + 31(1 - g_t16)¹²)·i, … i^(-1.8)) for i = [1…32]`: it starts 19 semitones above the bass note
  and settles an octave below it, every beat, with 32 harmonics tilted by `n^-1.8`.
- Other kicks: `tone([1, 4/3, 2, 3]·40·Kick·E_Kick, …)` with `E_Kick = (1 - x/1)²` (bqrffiubis: frequency and gain fall
  together); `tone(220·2^(-20x²/12), 2/x)` (mbokvmxbzr, r/desmos 241: a quadratic fall in semitones, gain 1/time);
  `tone(500(f_1(mod(t/4, 1)) - 0.35), 50(…))` (fuyhyjnedt: the envelope is the frequency).
- **Snare and hats: noise made of random partials** (the forum's question "how do I make drums" has no answer in the
  thread; the corpus answers it): `9001·random(50, 16)` and `24000·random(50, 8)` (bqrffiubis);
  `30·sort(1.87^(15.32·random(200, t)^0.4))` and hats `2.3^(11.25·random(150)^0.01)` (8kxhmzypm8);
  `lowF + (highF - lowF)·random(res)` with an exponential decay, kick 0-300 Hz, snare 100-1200 Hz (a9jlvp2yl8);
  `220·4^random(100)` weighted by a Gaussian around 330 Hz (mbokvmxbzr).
- **Ducking from the kick's own phase** (jbfiikq4mx): `g_ducking = 1 - hermite((1 - g_t16)²)`; the bass and lead are
  multiplied by it, the kick by `(1 - g_ducking)·0.6 + 0.4`.
- **Fills as data** (bqrffiubis): snare patterns end `…, 1, 1, R, 1`, a hat pattern ends `R, R, R, R, 1, R, 1, R`.

### Timbre

- **Spectral tilt per role** (8kxhmzypm8, Mini Desmos Song, Art Contest): lead `8/(π²(2i-1)^2.3)` on odd harmonics
  (triangle-like), pad square `^1.6`, chord saw `^1.5`, bass saw `1/i^1.2` up to 150 harmonics. jbfiikq4mx: kick
  `i^-1.8`. a9jlvp2yl8: leads `1/n²`, bass `1/n`, 16 harmonics. mbokvmxbzr and y5tldfmcxx: `1/(1.1^P·1.6^H)`, a gain
  that falls with the harmonic number H **and** with pitch P.
- **A filter as a gain curve over the partials**: Butterworth `1/sqrt(1 + (f/c)^(2n))` on a square's partials with a
  per-note cutoff sweep `c = f(lerp(macro, 95, 131) + (1 - g_t32)^1.8·48)` (jbfiikq4mx: the cutoff falls 48 semitones
  within each eighth); a logistic lowpass `1/(1 + e^(n - l + 5))` (gaowmi4hi0); a resonant one with a `q` (idy42pofff).
- **Pulse width as Fourier** (gaowmi4hi0): `|sin(nπ·p_w)|/n`; "when p_w = 0.5 every other harmonic goes to zero".
- **Instrument laws** (lonntptn3d, DevDoors): piano partials stretched, `s·sqrt(1 + s²/1024)`, gains `e^(0.3(1-s))/s²`;
  bells `s·sqrt(1 + s²/128)`; clarinet odd harmonics `2.4^(1-s)`; flute `6^(1-s)` with a breathing `sin(3t + …)`.
- **Unison**: copies at `±0.5·detune` semitones with gains 0.9 and 0.8 (idy42pofff); a copy at `π·74.3` against
  `π·73.8` at half gain (y5tldfmcxx).

### Envelopes

- **`(1 - a/b)^c` with c chosen per voice** (bqrffiubis): kick c = 2, snare 1.2, hat 5, bass 3, arpeggio 0.5;
  a negative b flips it into a swell (`E_HighBass` swells over two beats).
- **Superellipse** `(1 - x^p)^(1/p)` per beat (fuyhyjnedt); **piecewise ADSR** (mbokvmxbzr, a9jlvp2yl8, 8kxhmzypm8);
  **exponential**, `3^(-n)` for a piano (cyd1baqnyg).

### Effects and form

- **Echo as a second reading of the same list**: `D(Arpeg1, 0.5, 3/2)` at 1/3 of the gain (bqrffiubis);
  a "Parrot" 3 beats later at ×0.6, a second at ×0.36 (a9jlvp2yl8).
- **Sections gate layers**: 12 sections of 32 beats, each tone multiplied by `Section0123` etc. (bqrffiubis); the
  second pass of A Mathematical Ballad adds two harmony lines to the melody (`s_ect` 1 → 2); hats enter at
  `t > t_Section2` (a9jlvp2yl8).
- **A key change up a whole tone** for sections 4-5: every frequency times `2^(1/6)` (bqrffiubis).
- **An accelerando** solved as a quadratic for time-at-beat (a9jlvp2yl8, `t_AtBeat` with `quadFormula`).

### The generative pieces

- **Rhythm ratio = pitch ratio** (4tca7a3nay, r/desmos 193): 21 voices at speeds `poly = [50…70]`; voice i sounds the
  i-th harmonic of the fundamental when its ball crosses an integer, gain `i^-damping`. (w07779vzrr, a Music Challenge
  finalist: voice a is `tone(i_a·r_base)` gated by `mod(r_t·i_a, 1) < i_a·r_noteLength`.) This is our skill's own rule
  "ratios as harmony and rhythm at once", and it is the one generative idea in the corpus that people rated as music.
- **Fractal orbits as pitch** (cjcu7govza `tone(|f^n(c_1)|·330)`, 0tlexdfyjb, e8vwkgezij `350(Re(z) + 2.5)`): entered
  as *sound effects* in the challenge, or scored 68.
- **Random, Fibonacci and Markov generators** (1bikdj5g4c, r20eqqtxwl, krs6vorqns): a scale table (major, minor,
  pentatonic; just or equal), a rhythm list, and a random or sequence index into the scale. Scores 2-23.

## 4. Numbers from the note lists

`scripts/melstats.py` reads the lists directly. Moves are between consecutive *different* pitches (held notes are
stored as repeats); "bar repeat" is the critic's measure (mean over bars of the best slot-for-slot match with an
earlier bar).

| list | step | steps 1-2 st | thirds 3-4 st | leaps 5+ st | turn after a leap | range p5-p95 | bar repeat |
|---|---|---|---|---|---|---|---|
| Better Off Alone, lead (jbfiikq4mx) | 16th | 0.23 | 0.62 | 0.15 | 1.0 | 12 | 0.58 |
| TETRIS, lead (2sil5zcnun) | 16th | 0.65 | 0.29 | 0.06 | 0.5 | 10 | 0.47 |
| A Mathematical Ballad, melody (3mcvjtyd4q) | one 300 ms tick | 0.71 | 0.29 | 0.00 | none | 9 | 0.36 |
| Overtime 2.0, bridge melody (bqrffiubis) | half | 0.77 | 0.08 | 0.15 | 1.0 | 12 | 0.25 (16 steps) |
| Bach Cello Suite 1, 20 bars (34gdqnbihn) | 16th | 0.38 | 0.25 | 0.37 | 0.64 | 22 | 0.41 |
| TETRIS, bass | 8th | 0.03 | 0.00 | 0.97 | 0.98 | 20 | 0.27 |
| Overtime 2.0, low bass | half | 0.58 | 0.00 | 0.42 | 0.8 | 9 | 0.33 |
| silent road, bass (e2iix1hhkk) | 16th | 0.50 | 0.00 | 0.50 | 0.33 | 7 | 0.81 |

**Chord-tone agreement** (`scripts/chordtone.py`): of the lead notes on beats, the share whose interval above the bass
note is 0, 3, 4 or 7 semitones: **0.75** in Better Off Alone (16 beats; 0.88 counting 7ths) and **0.72** in TETRIS (29
beats; 0.76 counting 7ths). For scale: a random diatonic note lands on a triad tone of the root 3 times in 7 (0.43).

**Note onsets per beat** in the two fast leads (a new pitch, or a pitch after a rest): 0.88 (Better Off Alone) and
1.24 (TETRIS); notes are held for 2-4 steps (69-79% of steps repeat the previous pitch).

**Tempo**, where a graph states it: Overtime 175 BPM (its slider), TETRIS 150 (its slider), Better Off Alone about 144
(its time slider runs three 16-beat loops in 20 s). Our grammar samples 88-112.

## 5. What separates the good ones from the demos

The demos are numerous and popular: "Cursed Desmos Sounds" (1.3M views for one video) and its sequels, "uncanny math
sounds", the Weierstrass function played as audio, fractal orbits, single synth patches. They share one thing: **one
sound, made by one expression, with no metre**. The well-received songs differ in six ways, in order of how often:

1. **Roles.** Bass, harmony, melody, and (in the dance and game songs) drums, each its own voice with its own register
   and timbre. A demo is one voice.
2. **A grid.** Everything on one step clock (a 16th or 8th), loops of 4 bars, sections of 32 beats in Overtime.
3. **A progression under a melody.** Chord or bass lists changing once a bar or two, a melody whose beat notes are
   mostly chord tones (0.72-0.75).
4. **Notes that end.** Decay envelopes, gates with rests, glides at chosen notes. Pure sustained tones are the
   official example's sound and every demo's.
5. **Instruments, not sines.** Additive spectra with a tilt chosen per role (p = 1.2 to 2.3), filtered and sometimes
   detuned.
6. **Drums that are pitch falls and noise.** The one technique the Desmos forum asked for and never answered in its
   thread; the good songs all solved it the same two ways.

What does *not* separate them: cleverness of encoding (Giant Steps and Looping The Rooms play plain sines from lists,
and both are challenge finalists, for their harmony, arrangement and visuals), or a generator. The generative graphs
that get rated as music are the ones where the rule makes a **metre** (polyrhythms), not the ones where it makes the
**pitch**.

## 6. What this means for our composer

The composer derives roots, melody and rhythm from **one** generator through a lens, then judges with a critic whose
bands were set without a corpus. The corpus says the human songs put the judgment in three separate places (a
progression, a melody fitted to it, instruments and drums), and it gives numbers for several bands. Each change below
names what to build and the evidence. None needs a new engine feature unless it says so.

### The grammar (`src/compose/grammar.rs`)

1. **Split harmony from melody: a progression layer.** Sample a 4- or 8-chord loop of scale-degree roots first (stored
   as digits, as now), one chord a bar or a chord every two bars, with root motion drawn from the critic's own weights
   (fourth up, fifth up, step, third), and the last chord leading back to the first. Then let the family's generator
   make the *melody contour* only. Evidence: Better Off Alone `[40, 39, 44, 42]`, SzefRon `[-24, -29, -32, -25]` at
   one chord per 8 clock units (the arpeggio steps 16 times in that span), the Ballad's chord-time map
   `[0,0,1,1,2,0,3,4]`, silent road's 16-step voicings. Today `R(i)` comes from the same generator as `M(k)`, which no
   song in the corpus does.
2. **Snap the melody's beat notes to chord tones.** On beat steps, replace the generator's degree d by the nearest
   stacked-third degree of the bar's root R, in the same octave: with `u = mod(d - R, 7)`,
   `ct(d, R) = d - u + 2 floor(u/2 + 0.5)` (u rounds to 0, 2, 4 or 6: root, third, fifth, seventh in the lens);
   off-beat steps keep d. Target 0.6-0.9 triad tones on beats (corpus 0.72-0.75; random 0.43).
3. **A gate pattern separate from the pitch sequence, with held notes.** Give the lead a 32- or 64-step gate pattern
   (bits of two or more integers, or a Euclidean pattern per bar) whose cycle differs from the pitch cycle, and let a
   pitch that repeats be *held* (no new onset). Target about 0.8-1.3 note onsets per beat for a lead at 16th
   resolution (corpus 0.88 and 1.24). Add an optional slide bit per step, `pitch += (1 - frac(4t))^4·slide·s`, a
   scoop into flagged notes (jbfiikq4mx).
4. **Kick tuned to the key, falling fast.** Set the kick dash's `to` an octave below the bar's bass root and its `from`
   about 31 semitones above `to`, with the shortest `dash` (jbfiikq4mx: `31(1 - φ)^12` semitones). The current fixed
   450 → 58 Hz kick ignores the key.
5. **Ducking.** When the kick is on every beat, define `duck = 1 - smooth((1 - frac(t))²)` (0 at the hit, back to 1 by
   the next beat) and multiply the bass's, pads' and lead's `amp` by it, as jbfiikq4mx does (it also scales the kick by
   `0.4 + 0.6(1 - duck)`, which a dash cannot read). Our language has `smooth` and `amp` expressions; no engine change.
6. **Timbre by spectral tilt per role.** Generate partial sets as `-20·p·log10(n)` dB (odd harmonics only for the
   square and triangle families): bass p = 1.2, chord saw 1.5, square pad 1.6, triangle lead 2.3, kick 1.8
   (8kxhmzypm8, jbfiikq4mx). Today there are three fixed lead sets.
7. **Envelope shapes per role.** Decay as `max(0, 1 - note_age/L)^c` in `amp` with c: arpeggio 0.5, snare 1.2, kick 2,
   bass 3, hat 5 (bqrffiubis). The grammar's arp and lead already decay; make the curve a choice.
8. **An echo voice.** For the arp or lead, a second voice reading the same pitch function at `t - d`, d = 0.75 or 1.5
   beats, gain 1/3 (bqrffiubis) or ×0.6 (a9jlvp2yl8). It is the graph-way delay, and it draws a second line on the
   wall.
9. **Forms from the corpus**, added to `FORMS`: *loop all-in* (one 4-bar loop for the whole song: the two most popular
   tone() songs); *intro then loop* (`{t < I: t, I + mod(t - I, L)}`); *second pass harmonised* (melody alone, then
   melody and a third above); *verse and chorus by voicing* (the same progression, the chorus voicing's upper voices
   up a fourth to an octave, as in silent road); *up a whole tone* for the last section (bqrffiubis). Fills in the last
   bar of every 8: snare `…1, 1, 0, 1`, hats dropping out for half a bar.
10. **Generative families: make metre, not pitch.** Add a *ratio* family where voices run at consecutive integer rates
    and each sounds its own harmonic (4tca7a3nay, w07779vzrr), and use the existing families (Takagi, Fibonacci, rule
    30, primes, Collatz) for the melody contour and for variation between passes, not as the raw pitch stream.
11. **Tempo range.** Corpus songs that state a tempo run 144-175 BPM (the dance and game songs). Keep 88-112 for the
    city-pop reference, and add a 120-150 band for a loop-all-in form.

**Two corpus techniques conflict with the skill's rules, and are the author's call:** detuned unison copies and filters
(the skill says "no filters, no detune"; the corpus uses filters on partials in 3 well-received graphs and unison in
3), and noise drums (rule 5 says "never noise"; the corpus builds snares and hats from 30-200 random partials in 6).
Noise from random partials would need inharmonic partials in a dash, which the engine does not have (its partials are
harmonic).

### The critic (`src/compose/critic.rs`)

1. **Add chord-tone agreement**: the share of lead notes on beats that are triad tones of the bass pitch class at that
   bar. Score 1 for 0.6-0.9, 0 at 0.35. The current critic never compares the melody with the harmony.
2. **Re-band melody steps**: score 1 for 0.2-0.8 of pitch changes being 1-2 semitones (corpus 0.23-0.77; Better Off
   Alone, the most popular, is 0.23 because it moves in thirds), and stop penalising thirds when they are chord tones.
3. **Add a leap budget**: leaps of 5+ semitones at most 0.15 of a lead's pitch changes (corpus 0.00-0.15), with a turn
   after half or more of them (0.5-1.0). The current contour component measures the turn but not the budget.
4. **Re-band bar repetition**: 0.35-0.6 for a lead (corpus 0.36-0.58; the critic's 0.45-0.8 marks the Ballad and Bach
   down).
5. **Narrow lead range**: 9-14 semitones p5-p95 (corpus 9-12; the critic accepts up to 17).
6. **Score note length, not only density**: note onsets per beat 0.8-1.3 for a 16th-grid lead, with held notes
   allowed; the current density measure counts events and rewards busier songs up to 11 a beat.
7. **Make *structure energy* conditional.** Do not ask a 64-beat loop to build: the two best-received tone() songs
   keep every layer on, which is the case composer.md found the hand songs failing on. Apply build and contrast only
   to songs with more than one 32-beat section that the form says differ.
8. **Add roles**: one point each for a bass voice, a harmony voice, a melody, and (for forms with drums) a kick that
   falls in pitch and a backbeat. Demos have one role.
9. **Add per-note decay** on plucked roles (arp, lead, bass): the amplitude at the note's end below half its peak.
10. **Floor every component** (composer.md already proposes it): nothing in the corpus has a dead component.

**Weights:** these are bands from 5 leads and 3 basses. Treat them as priors for the pairwise fit already planned in
composer.md ("Learning from ratings"), not as truths.

### What not to take from the corpus

- **Transcription.** 37 of 64 are covers in note lists. The lesson is the *structure* of those lists (a slow
  progression, a melody fitted to it, held notes, separate gates), not the lists.
- **Resynthesis.** The highest Reddit scores (546, 401, 174) replay recordings frame by frame.
- **Integer tables as a virtue.** They are our language's workaround for having no lists, not a technique anyone here
  used.

## 7. Calibration pieces

The critic could be checked against a few corpus pieces written in our song language: each should score well, and its
deliberately broken versions (melody shifted off the chords, drums removed, gates deleted) should score worse. **Do
not commit translations of other people's songs**; these notes are feasibility only. The originals are the better
choice (no third-party song copyright, only the graph author's), and even those would need the author's permission to
ship.

| piece | kind | expressible in our language? | what it would test |
|---|---|---|---|
| SzefRon's loop (fuyhyjnedt) | original | **yes, closely**: 4-entry bass and 16-entry chord tables as digit integers, arpeggio by `mod`, kick and snare as falling-pitch voices, vibrato in `pitch`, a superellipse envelope per clock unit in `amp` | roles, the falling kick, an all-in loop |
| A Mathematical Ballad (3mcvjtyd4q) | original | **yes**: 48-step melody and two harmony lines as a few digit integers each (offsets -10..7 fit in two digits), the chord table and chord-time map likewise | chord-tone agreement, second-pass harmonisation |
| supersaw sequencer (gaowmi4hi0) | original | **mostly**: a 16-step root table and a 64-step octave table; the random arpeggio needs a sine hash; the logistic lowpass becomes fixed partials | progression under a sequence |
| Polyrhythms (4tca7a3nay) | generative | **yes, natively**, with fewer voices: voice i, gate `frac(r_i·t) < w`, pitch the constant `12·log2(i)` written as a number | the ratio family |
| Better Off Alone loop (jbfiikq4mx) | cover | yes (64-step lead, gate and slide tables, filtered bass via `cutoff`, ducking via `amp`), but a cover | ducking, filter sweep, the tuned kick |
| Transmission (a9jlvp2yl8), Mini Desmos Song (8kxhmzypm8) | original | **partly**: event lists with irregular times and noise drums do not fit | not recommended |

## 8. What I could not verify

- **The sound.** I did not listen to any graph. "Sounds good" is inferred from curation and popularity; descriptions
  of how a graph sounds come from its own notes or its post's comments.
- **Ranking inside the Music Challenge.** The three result galleries list finalists; whether their order is a ranking
  is not stated. The results video (youtube.com/watch?v=trtC9TdVxsM) was not watched.
- **Art Contest years.** The gallery bundles for art-2023, -2024 and -2025 overlap, so the contest year of a graph is
  uncertain; that it was featured is not.
- **Authorship** where the graph does not credit anyone (family mart, Shepards Tone, the amen break); whether the
  Reddit user Biansci and the graph author nyansci are the same person.
- **Counts.** The hand tally is one reader's, over 27 graph families; the regex tally undercounts several techniques
  (its kick pattern found 1 where reading found 4). Reddit scores are the Arctic Shift archive's values, which can lag
  the live site. Better Off Alone's tempo is derived from its slider, not stated.
- **Coverage.** Reddit was read through an archive (Reddit returned 403); the forum's search rate-limited; TikTok and
  Discord were not searched.

## Reproducing the numbers

From `docs/research/desmos_corpus` (Python 3, standard library only):

```
py -I scripts/melstats.py raw        # section 4 table
py -I scripts/chordtone.py raw       # chord-tone agreement
py -I scripts/tech.py raw            # regex counts in section 2 (writes tech_matrix.json in the current folder)
py -I scripts/hand_tally.py          # hand counts in section 2 (the tally is written out in the script)
py -I scripts/tones_only.py raw/<hash>.json   # a graph's tone() lines and the definitions they read
py -I scripts/dump_state.py raw/<hash>.json   # every expression, folder, slider and the ticker
```
