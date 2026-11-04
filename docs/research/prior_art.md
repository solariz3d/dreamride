# Prior art: people who already make music as functions of time

Research for the composer, 2026-11-02. The question was: who has already made *good* music as mathematical
functions, what did they write, and what did they learn? Every source below was read for this note (fetched
pages, raw source files, the arXiv PDF). Where something could not be confirmed it says so. Formulas in
`code` are quoted from the source exactly; anything marked **(ours)** is our own derivation or reading and
was not stated by the source.

Licences: "verified" means the licence file or the GitHub licence field was read. "Unverified" means none was
found, so treat the code as cite-only.

---

## 1. @measure_plan / Stereo Drift: the reference itself

**Who it is (verified).** The X account `@measure_plan` (https://x.com/measure_plan; its bio, as given in a
search snippet, reads "experiments in computer vision, games, music") and the Instagram account `@stereo.drift`
(https://www.instagram.com/stereo.drift/; the page metadata reads *Alan (@stereo.drift) … "computer vision,
music, games 👾 montreal"*, 68K followers, 313 posts) are the same person. The link is
**https://midisurvivor.com**, a paid browser game for learning notes, scales and chords on a MIDI keyboard. Its
contact block lists the X link `x.com/measure_plan`, the Instagram `stereo.drift` and a contact email (not
reproduced here).

**What was not found.** There is no public GitHub, no public Desmos graph of the city-pop tracks, no blog and no
interview, after searching the web, YouTube's results page, GitHub's API and Desmos. X returned HTTP 402 to the
fetcher, so his individual posts could not be read. **The method is known only from the videos**, through the
formulas transcribed into `DESIGN.md`. Those formulas are the only primary record we have, so here is what each
one *is*, read as technique **(ours)**:

| Formula (from DESIGN.md) | Technique |
|---|---|
| `h(d) = floor((12d + 2) / 7)` | A scale as a **floor of a line**: 7 degrees spread as evenly as possible over 12 semitones. This is the same "maximally even" maths as Euclidean rhythms (§6). Changing the offset rotates the mode (+2 gives minor, +5 gives major; see DESIGN.md). |
| `root(i) = floor(342258473 / 10^(8 - i)) mod 10 - 5` | **An array stored as the digits of one integer**, read with floor/mod. This is the same trick as bytebeat's hex-nibble tables (§3). |
| `fold(z) = (z + 9) mod 12 - 9` | **Voice leading by wrapping**: every chord tone lands in one fixed 12-semitone window, so chords move by small steps instead of jumping with the root. |
| `sw = (floor(2x) + max(frac(2x)/1.16, (frac(2x) - 0.16)/0.84)) / 2` | **Swing as a piecewise-linear time warp.** The two branches meet at frac = 0.58, so within every half beat the second sixteenth sounds at 58% instead of 50%. Every voice reads warped time, so the whole band swings together. |
| `w = u^2 (3 - 2u)` | Smoothstep **glides** between chords, rather than hard switches. |
| 16-digit lead integers per section, harmony a third above | Melody as **digit strings indexed by step**, and harmony as a **function of the melody** (a fixed diatonic offset). |

The lesson in the reference is not one trick. **It is that every musical decision (scale, progression, voicing,
swing, glide) is made by a small, named function, and the voices are compositions of those functions.** The
other sources below show what such songs still need that our first sketch lacks.

---

## 2. Desmos music (`tone()`)

- **The official docs:** Desmos Help, "Tone", https://help.desmos.com/hc/en-us/articles/21373904717197-Tone (behind
  Cloudflare; read via the Wayback Machine snapshot of 2025-12-26).
  - "Tone takes frequency and gain as inputs." Frequency runs 20–20000 Hz, and graphs start muted.
  - Gain is clamped: *"Max gain = min(10, 660/frequency)"*, so low notes may be louder.
  - Pitch: *"f(x)=440·2^{x/12}"*, where x = 0 is A4.
  - The help page **does not state the waveform**, and no polyphony limit was found (unverified).
- **The official melody example:** "Playing a melody", https://www.desmos.com/calculator/05h9f2smtg (read as JSON).
  - The whole sequencer is a list indexed by a time variable `t` that the graph animates: `L=[C,C,R,C,C,R,…]`,
    `tone(L[t])`, with `R=0` as the rest.
  - The gain example (https://www.desmos.com/calculator/7v0f4f0c4n) gives an **envelope as a function of t**:
    `f(x)=sin^2(2x)·e^{-.2x}{x≥0}` with `tone(300, f(t))`.
- **Community songs before `tone()`** used *audio trace* (sonifying a curve's height), e.g. "Desmos Song 4",
  https://www.desmos.com/calculator/z2oofcw6tx, arranged by Mason Hirschi.
  - **Each bar is a piecewise function** of x: `B_{ar1}={0<x<22.5:3, 22.5<x<59.75:5, …}`. The small gaps
    (59.75 → 60) between pieces are what make separate notes audible.
  - Note-separation gaps are a primitive envelope. The lesson is that **legato without gaps blurs into one tone**.
- **Tool: AlexApps99/MIDI2Desmos** (https://github.com/AlexApps99/MIDI2Desmos; licence not checked) converts MIDI
  into Desmos expressions.
- **Forum:** "Play music in Desmos", https://cl.desmos.com/t/play-music-in-desmos/7612.
  - Users sequence notes with interval conditions on t.
  - Its percussion thread (https://cl.desmos.com/t/making-drums-percussion-in-desmos/8095) has **no working drum
    method**, only "a slightly high pitched sound, then a little tap sound".
  - Drums are Desmos's weak point. measure_plan's tracks presumably get theirs elsewhere (unverified: the videos
    would show it).
- **Licence:** public Desmos graphs fall under Desmos's terms, and no licence is stated on the graphs (unverified).
  Learn from them; do not copy.

---

## 3. Bytebeat (viznut, 2011)

### Sources

| Source | URL | Licence |
|---|---|---|
| Heikkilä (viznut), *Discovering novel computer music techniques by exploring the space of short computer programs*, arXiv:1112.1368 | https://arxiv.org/abs/1112.1368 | arXiv non-exclusive distribution licence (cite only) |
| "Algorithmic symphonies from one line of code – how and why?" | http://countercomplex.blogspot.com/2011/10/algorithmic-symphonies-from-one-line-of.html | none stated (cite only) |
| "Some deep analysis of one-line music programs" | http://countercomplex.blogspot.com/2011/10/some-deep-analysis-of-one-line-music.html | none stated |
| Formula collection (2011-10) | http://pelulamu.net/countercomplex/music_formula_collection.txt (the live domain now redirects to a lander; read via web.archive.org) | none stated; each formula is credited to its author |
| greggman/html5bytebeat | https://github.com/greggman/html5bytebeat | **MIT (verified)** |
| Dollchan bytebeat composer (SthephanShinkufag) | https://github.com/SthephanShinkufag/bytebeat-composer, player https://dollchan.net/bytebeat/ | **MIT (verified)** for the player. The 2,091-song library (`/data/library/all.gz`) carries **no per-song licence (unverified)**. |

### The framework

*"for(;;t++) putchar(EXPRESSION)"*: the output is unsigned 8-bit samples at 8000 Hz, and only the low 8 bits
count (paper §2). The players add **floatbeat** (output −1..1, "clean sound without 8-bit distortion") and
**funcbeat** (a function of seconds, which may hold arrays and helpers) (Dollchan README; greggman README).

### Techniques, with the formulas that show them

- **Pitch = t × something.** *"t*(42&t>>10)"* is "the Forty-Two Melody". The pitch is a series of integers made
  by masking a counter.
  - *"Integer series with high values are likely to contain intervals that are dissonant to Western listeners.
    Modulus can be used to restrict the values into a more familiar range"*: `t*((42&t>>10)%14)` (paper §3.3).
  - **The lesson: raw arithmetic gives "detuned or 'Arabic'" intervals (blog); musicality arrived when people
    added lookup tables.**
- **Rhythm and form from bit shifts.** `t>>n` is a counter that ticks every 2ⁿ samples. All of them are
  *"implicitly synchronized with each other"* (blog). The paper's explanation of why this sounds like songs:
  *"The effectivity of long bit shifts for macro-level musical structure is probably related to the effectivity
  of power-of-two structural ratios in pop music … 'eight-bar blues', 'sixteen-bar blues' and 'thirty-two-bar
  form'"* (§3.2). **Power-of-two section lengths are the cheapest structure that sounds intentional.**
- **Melody as a lookup table: digit strings and hex integers.** This is the move every *musical* bytebeat uses.
  - ryg, 44.1 kHz: `((t*("36364689"[t>>13&7]&15))/12&128)+(((((t>>12)^(t>>12)-2)%11*t)/4|t>>13)&127)`. An
    8-character string is the bass/pitch table, indexed by a slow counter. The second half is a *self-similar*
    melody generator, `((t>>12)^(t>>12)-2)%11`, whose note depends on the number of trailing zero bits of the
    step, so the downbeats get different notes from the offbeats (paper §3.3, with a table of the first 16
    notes).
  - Rrrola: `t*(0xCA98>>(t>>9&14)&15)|t>>8`, a **4-note melody stored as hex nibbles of one integer** (paper:
    *"bit-masking for transposing and fading out a short constructed melody element that is encoded as a
    constant"*).
  - kb: `((t/2*(15&(0x234568a0>>(t>>8&28))))|t/2>>(t>>11)^t>>12)+(t/16&t&24)`, eight nibbles as a pitch table.
  - gasman (JS-only) uses **just-intonation ratios**, chosen by 3-bit fields of hex constants, with a
    4-section chord table selected by `t>>14&3`:
    `(t<<3)*[8/9,1,9/8,6/5,4/3,3/2,0][[0xd2d2c8,0xce4088,0xca32c8,0x8e4009][t>>14&3]>>(0x3dbe4688>>((t>>10&15)>9?18:t>>10&15)*3&7)*3&7]`.
    This is the clearest example of **section → chord table → note-in-chord**, three nested lookups.
  - a1k0n (JS): `SS=function(s,o,r,p){var c=s.charCodeAt((t>>r)%p);return c==32?0:31&t*Math.pow(2,c/12-o)},SS("0 0     7 7     037:<<",6,10,32) + (t&4096?SS("037",4,8,3)*(4096-(t&4095))>>12 : 0)`.
    - **Semitones written as characters** ('0' = 0, '7' = 7, ':' = 10, '<' = 12), and **space is a rest**.
    - A second voice arpeggiates `"037"` (a minor triad) under the **decay envelope** `(4096-(t&4095))>>12`,
      and only on alternate halves (`t&4096?…:0`).
    - This is the closest bytebeat gets to our song language.
- **Drums are pitch-falls and noise bursts.**
  - Mu6k: `(int)(3e3/(y=t&16383))&1`. Dividing by the time since the hit gives a fast downward pitch slide,
    which is a kick (blog: *"fast downward pitch-slides"* synthesise drums well).
  - Mu6k's whole song: `(3e3/(y=t&16383)&1)*35 +(x=t*"6689"[t>>16&3]/24&127)*y/4e4 +((t>>8^t>>10|t>>14|x)&63)`.
    It holds a kick, a bass from a 4-digit table with a **swelling envelope** `*y/4e4`, and a lead derived from
    the bass (`|x`). The blog says the lead was found first and the bass then built to "harmonise".
  - Greaserpirate's "perfect c compatible drums", in the Dollchan library: *"These drums can also be added to ANY
    8khz song"*. The drum part is a reusable prefix.
- **Swing as a time warp.** Greaserpirate, "the SWANG algorithm": *"Put t-=t/4&256 before any bytebeat (but
  after you multiply t by the bpm) and you can make the rhythm swing."* This is the same idea as
  measure_plan's `sw`: **warp t once, and everything downstream swings.**
- **Arrangement by layers entering over time.** SthephanShi, "Dark Forest" (Winter Chip XVIII, 2nd place; Dollchan
  library; *"generates chord progressions from the initial pattern '037'"*):
  `T=t/5E3,b=T>>7,k=b+2&4?8:4,S=(s,p,d=0)=>(t*2**(p+'037'[(T-d)/s%3|0]/12+((1-(T>>5&3))%2+1)/4)&128)/3,N=p=>1E5*sin(T*p|0)&64,S(2,0)+S(2,0,3)/2+(b&&S(0,-2))+(b>1&&S(.25,1)*(1-T%16/20))+(b>2&&.4*N(1E6)*(1-(T+k/2)%k/k))+(b>3&&(64/(T%k)&64)+(T&k&&.6*N(330)*(1-T%k/k)))`.
  Read **(ours)**:
  - `b=T>>7` is the **section number**, and each layer is gated by `b&&`, `b>1&&`, `b>2&&` and `b>3&&`, so the
    song **builds by adding one instrument per section**;
  - `S(2,0,3)/2` is the **same arpeggio delayed by 3 steps at half volume: a stateless echo**;
  - `(1-T%k/k)` is a **linear decay per hit**;
  - `((1-(T>>5&3))%2+1)/4` shifts the progression by section;
  - `k` doubles the drum spacing in some sections (half-time feel).

  Chasyxx's variant swaps `'037'` for `[0,4,7,11]`, *"The most beautiful chords."*: a **major-7th** arpeggio.
- **Wrap-around as an instrument.** `(t*9&t>>4|t*5&t>>7|t*3&t/1024)-1`: the `-1` *"effectively introduces a
  percussive instrument by turning minimum amplitude values into maximum values"* (paper §3.4). This is specific
  to 8-bit wrap and not transferable to a float engine.

### Mistakes and limits they reported

- **Most short formulas were found by chance:** *"Several contributors have admitted that they do not understand
  the inner workings of their discoveries at all"* (paper §3.1). Random search dries up: after the discovery
  phase it is *"very difficult to find anything outstanding"* (blog).
- **Longer, more musical formulas are constructed** "section by section" with "constants and tables" (blog), and
  *"The more structured expressions are also more likely to use more traditional approaches"* (paper). **The
  musical ones are the composed ones.**
- **The syntax is the bottleneck:** *"Many individuals … have found the C-like infix syntax limited and
  cumbersome … It is, for example, difficult to reuse previously computed values"* (paper §4). viznut called the
  one-liner form "a mere placeholder". Our language has user functions, which is the cure for this.
- **Aliasing at 8 kHz:** high melodies sound detuned, because alias tones dominate below about `&32`/`&16` (blog).
  Not our problem at 48 kHz with band-limited oscillators, but worth remembering for square and saw voices.

---

## 4. Shader music (Shadertoy `mainSound`, twigl, Wavenerd)

### Sources

| Source | URL | Licence |
|---|---|---|
| 0b5vr, "Shader Music" (Japanese wiki page) | https://scrapbox.io/0b5vr/Shader_Music | none stated |
| 0b5vr, "Reflection of 0mix" (the Revision 2023 *GLSL Techno Live Set*) | https://scrapbox.io/0b5vr/Reflection_of_0mix, https://github.com/0b5vr/0mix | 0mix: no SPDX licence (unverified) |
| 0b5vr/wavenerd-dubplates (complete live-set shaders) | https://github.com/0b5vr/wavenerd-dubplates | **CC BY-NC 4.0 (verified LICENSE)**: non-commercial; learn, don't paste |
| Shadertoy sound shaders named by the community: nimitz "Music toolbox" `MdfXW2`; athibaul "Making Music in Shadertoy – ep1" `ssjyDD`; ambient `4ts3z2`, acid techno `ldfSW2` (HN thread https://news.ycombinator.com/item?id=19470135) | https://www.shadertoy.com/view/… | Shadertoy pages are behind Cloudflare and **could not be read**. Shadertoy's default licence is commonly given as CC BY-NC-SA 3.0 (**unverified**). |

Shadertoy's entry point is `vec2 mainSound(int samp, float time)`, which returns stereo (0b5vr wiki; Natron docs).
An npm package page says Shadertoy caps sound at 60 s (unverified). Wavenerd's `mainAudio(vec4 time)` hands the
shader **four clocks at once: time since the beat, the bar, the 16-bar phrase, and since the start** (wiki). That
is a strong hint about what composers want: **phase within each level of the metre, not just t.**

### Techniques, from the source of `sets/20250322_draw/shaders/210_M7b5.glsl`

- **Rhythm patterns as 16-bit integers.** `seq16(time.y, 0x9192)` is the kick, `0x2424` the clap, `0xaaaa` the
  ride, `0xffff` every step. `seq16` returns the **step index, the time since the last active step, and the time
  until the next active step**. Every envelope is built from those two times:
  `env = smoothstep(0.0, 0.001, q) * exp2(-20.0 * max(t - 0.1, 0.0))`.
  - `smoothstep(…, q)` closes the note just before the next onset, so there are **no clicks and no overlap**.
  - `exp2(-k t)` is the decay.
  - **This is a complete, stateless note model: gate and envelope from the pattern and t alone.**
- **Swing in the step clock:** `const float SWING = 0.51;` inside `t2sSwing` / `s2tSwing`. Even 51% is used,
  which is subtle.
- **Sidechain "duck" derived from the kick's own timing:** `duck = smoothstep(0.0, 0.8 * B2T, t) * smoothstep(0.0, 0.001, q);`
  Then everything else is multiplied by `mix(0.3, 1.0, duck)` (hats, pads, the sonar) or by `duck` (the sub).
  **The pump follows the actual kick pattern, not a fixed beat.**
- **Velocity without randomness state:** `float vel = fract(0.417 * seq.s);`. A step-indexed pseudo-random value
  modulates the hat decay, `exp2(mix(7.0, 5.0, vel))`. **Every hat is a little different; the pattern repeats
  exactly.**
- **Chords:**
  - **voicing as an interval list:** `int CHORD[] = int[](0, 4, 6, 11);`;
  - **progression as a transposition list:** `int PATTERN[] = int[](0, 6, 3, 14, 10);`, stepped once per
    sixteenth and indexed `mod(st, N_PATTERN)`. Five against sixteen gives a **polymetric chord stab** that takes
    80 steps to realign;
  - the comment and file name say M7b5.
- **Stateless delay:** the chord is summed 5 times at `offset = -2.4 * S2T * fiDelay`, with
  `delaydecay = exp2(-0.4 * fiDelay - …)` and **a darker filter and brighter high-pass per repeat**. This is an
  echo with no buffer, the same move as Dark Forest's `S(2,0,3)/2`.
- **Filters without state:** each voice is built from additive partials, and each partial is weighted by an
  analytic filter magnitude (`ladderLPF(freqp, lpfcut, 0.0)`). The cutoff follows an envelope:
  `lpfcut = 200.0 * exp2(6.0 * fenv …)` with `fenv = exp2(-8.0 * t)`. **A filter sweep per note, as pure maths.**
- **Unison and detune from hashes:** `shotgun()` sums 64 sines with hash-chosen ratios, and the sonar uses 64
  partials with Box-Muller-distributed detune `exp2(0.06 * dicen.x)`. Width and richness come from
  **many slightly detuned copies**.
- **Saturation as the mix bus:** `return clip(1.3 * tanh(dest));`, plus `tanh` on individual drums.

### Lessons and limits (0b5vr, "Reflection of 0mix", translated)

- The set is 140 BPM and 256 bars, built from **16-bar loops** processed and chained. The author says this
  **limits how fast sections can change**.
- **Filters, EQ and reverb are unavailable in GLSL music because they need state.** Hence the analytic-filter and
  summed-delay tricks above. (For us **(ours)**: our Rust engine *can* hold state after the voice functions, so a
  real reverb or filter belongs in the engine, not in the song language.)
- Oidos (§5) was an explicit inspiration for the additive pads.

---

## 5. Demoscene synths for 4k and 64k intros

| Synth | URL | Licence (verified) |
|---|---|---|
| 4klang | https://github.com/gopher-atz/4klang | MIT |
| Sointu (4klang fork, multi-platform) | https://github.com/vsariola/sointu | MIT |
| Oidos (additive) | https://github.com/askeksa/Oidos | Zlib |
| WaveSabre (64k, Logicoma) | https://github.com/logicomacorp/WaveSabre | MIT |

These are synth engines; **the music is composed in a tracker or DAW** (Renoise for Oidos: *"the toolchain …
assumes that the music is made using"* Renoise; Sointu has its own tracker that exports `.yml`). So their
composition lessons sit in the **data format**, which rewards structure:

- **A song is per-track order lists over small unique patterns.** Sointu's `examples/patches/radiant.yml`
  (`bpm: 130`, `rowsperbeat: 8`, `rowsperpattern: 16`, `length: 94`): each track has
  `order: [0, 1, 2, 3, 0, 2, 1, 4, …]` and `patterns: [[80, 1, 1, …], …]`.
  - In the pattern rows, `1` = hold, `0` = note off, and other numbers are MIDI notes. In the order list, `-1`
    means the track is silent for that pattern.
  - **Reading the file (ours):**
    - the pad track's 8-pattern cycle `0,1,2,3,0,2,1,4` repeats 6 times, then a `5` (all-off) and 7 bars of
      silence (**a breakdown**), then a **new cycle** `0,1,6,6,0,2,7,8`, and finally a return to the first;
    - the bass and lead tracks are `-1` for the first 8 orders and **enter later**;
    - the drum track has a basic pattern `1` and **variants 2, 3, 4 placed at the end of each 8-pattern
      phrase**: these are fills;
    - a lead track enters only at order 32;
    - the drums' last pattern before the breakdown is a roll (`60` on every row).
  - **The arrangement grammar is: a phrase of 8, a fill in the last bar, layers entering and leaving, a
    breakdown, a changed second cycle, and a return.**
- **Size rewards repetition and quantisation.** Oidos: *"Quantize parameters … since rounder values compress
  better"*; *"Use round velocity values"*; notes are indexed by distinct tone/velocity combinations, so fewer is
  smaller. Sointu: *"Size first, speed second."*
- **Additive synthesis (Oidos):** *"produces sound by adding together a large number of sine waves, known as
  partials"*, grouped in modes, with spread, harmonicity, decay and a seed. The rich pads come from many detuned
  partials, not from complex sequencing.

**The transferable point (ours): what makes 4k music sound composed is the order-list arrangement (repetition,
fills, entries, breakdowns), not clever formulas.** It maps directly onto `floor(t / section_length)` lookups.

---

## 6. Live-coding pattern languages

| Language | URL | Licence (verified) |
|---|---|---|
| Strudel (JS port of Tidal) | https://strudel.cc, code https://codeberg.org/uzu/strudel (formerly github.com/tidalcycles/strudel) | **AGPL-3.0**: ideas only, do not copy code |
| TidalCycles | https://tidalcycles.org, https://github.com/tidalcycles/Tidal | **GPL-3.0** |
| Sonic Pi | https://sonic-pi.net, https://github.com/sonic-pi-net/sonic-pi | **MIT** (main source, per LICENSE.md) |
| Glicol | https://glicol.org, https://github.com/chaosprint/glicol | **MIT** |
| Toussaint, *The Euclidean Algorithm Generates Traditional Musical Rhythms*, Bridges 2005, pp. 47–56 | https://cgm.cs.mcgill.ca/~godfried/publications/banff.pdf | paper (cite) |
| McLean, *Making programming languages to dance to: live coding with Tidal*, FARM '14, pp. 63–70, doi:10.1145/2633638.2633647 | https://slab.org/tmp/p63.pdf | paper (cite) |

**Patterns are functions of time.** Tidal 0.4 models a pattern as a function from time to events (McLean 2014,
per its abstract as summarised by the search; full text not read). That is the same model as ours, with
cycles in place of beats.

**The operators that make groove:**

- **Euclidean rhythms:** `s("bd(3,8,0)")`, where *"(beats,segments,offset)"* (Strudel mini-notation). Tidal's
  docs show the rotations: `x ~ ~ x ~ ~ x ~ (3,8)`, `~ ~ x ~ ~ x ~ x (3,8,1)`, …. Toussaint: these rhythms
  *"have the property that their onset patterns are distributed as evenly as possible"*. Wikipedia: *"the same
  results can be obtained from the Bresenham algorithm"*. Examples: tresillo `x..x..x.` = (3,8), cinquillo
  `x.xx.xx.` = (5,8).
  - **Closed form (ours, checked by hand):** step i of n is an onset when `mod(i·k, n) < k`. For (3,8) this gives
    i = 0, 3, 6 = `x..x..x.` ✓. For (5,8) it gives `x.x.xx.x`, a rotation of the cinquillo. Add an offset to
    rotate. measure_plan's `h(d)` scale is the pitch version of the same evenness.
- **Polymeter:** `{bd sd stab, cp arpy cr arpy}` lets *"the shorter one wrap across the cycle"*, and `{…}%8`
  sets the step count (Tidal docs). For functions: two patterns of different lengths indexed by the same step,
  `mod(step, 3)` against `mod(step, 4)` (0b5vr's 5-against-16 chord is an instance).
- **Alternation per cycle:** `<e5 b4 d5 c5>` plays one per cycle. For functions: `mod(bar, n)` chooses the
  variant.
- **Degrade:** `?` drops an event with 50% chance (`?0.1` gives 10%), and `degradeBy(0.2)`, `sometimesBy(.4, …)`,
  `someCyclesBy(.3, …)` (Strudel). **Strudel's randomness is a pure function of time:** `rand = signal((t,
  controls) => getRandsAtTime(t, 1, controls.randSeed))`, where the legacy generator is an xorshift of
  `frac(t/300)` (packages/core/signal.mjs). **Same time, same "random" value: ghost notes and drops that are
  identical on every play and every export.** Sonic Pi takes the same stance on purpose: *"in Sonic Pi random is
  not truly random … the next run will produce exactly the same sequence … It's the Groundhog Day of music
  synthesis!"*, with `use_random_seed 40` to pick another take (tutorial §4).
- **Conditional transformation by cycle number:** `every 3 rev …`; `whenmod 8 4 (fast 2)` applies *"when the
  cycle number modulo the first argument is greater than or equal to the second"*; `every' 3 1` adds an offset
  (Tidal docs, "conditions"). **The `whenmod 8 6` shape (change the last two bars of every eight) is the fill.**
- **Swing:** `swingBy x n` *"breaks each cycle into n slices, and then delays events in the second half of each
  slice"* by x of the half-slice; `swing` = `swingBy (1/3)` (Tidal docs, "time").
- **Layered echo:** `off 0.125 (# crush 2) $ …` superimposes a transformed copy shifted in time (Tidal). This is
  the canon or echo again: the same stateless delay as §3 and §4.
- **Rings:** Sonic Pi's `(ring 52, 55, 59)[3] #=> 52`: any counter indexes a wrapping list, and *"scale and chord
  are also rings"*. `spread` *"creates a ring of bools with a Euclidean distribution"* (tutorial §8.4). This is
  `mod(index, length)` again.
- **Glicol's `seq`:** `o: speed 2.0 >> seq 60 _~a _ 48__67`. The bar is divided by spaces and each part is
  subdivided by notes and `_` rests. `~a: choose 60 60 0 0 72 72` (*"quantity alters probability"*) gives
  weighted choice by repetition (README).

---

## 7. TI-84 and other calculator music

- **The TI-84 has no `tone()`.** TI's classroom project plays sound through the **TI-Innovator Hub**, using two
  lists, frequencies and durations, and a loop (TI Education, "Project: Making Music with Code"; search snippet,
  not opened in full).
  - Native sound is 1-bit through the link port: **HoustonTracker 2** is *"a music editor/sequencer for Texas
    Instruments graphing calculators"* (YouTube description), and Cemetech hosts sound drivers
    (https://www.cemetech.net/downloads/browse/83plus/asm/sound).
  - **"TI-84 style" in the reference videos is therefore a visual style, not the audio source (ours, inferred).**
    No source ties measure_plan to TI hardware.
- **Other function-of-time systems:**
  - greggman's player supports the stack-language **postfix** form (and Glitch Machine's format); viznut's
    **IBNIZ** VM is named in the paper but was not opened here;
  - **wavepot** (https://github.com/wavepot/wavepot, MIT): its docs are "[wip]" and the README holds no examples,
    so nothing was learned from it.

---

## What this means for our composer

Ranked by how much musicality each would add **to the engine as it stands** (the `examples/citypop_sketch.txt`
voices hold `gate = 1` with a beat-locked `pump`, so the biggest gaps are note shape and arrangement). Each item
names its sources and how it maps into our language (voices as pitch/gate/amp functions of t in beats; user
functions; floor, mod, frac, min, max).

1. **Per-note envelopes from "time since onset" and "time to next onset".** (0b5vr `seq16`; a1k0n
   `(4096-(t&4095))`; Dark Forest `(1-T%k/k)`; the gaps in Desmos Song 4.) This has the biggest single effect:
   without it every voice is an organ.
   - Maps to: `age(x) = x - onset(x)` with `onset(x) = floor(x·4)/4` (or the last active step of a pattern), and
     `amp = min(1, age/0.01)·exp(-k·age)·min(1, (next(x) - x)/0.01)`.
   - Plucks for bass and keys, longer decays for pads, short ones for hats.
2. **Arrangement by layers entering and leaving per section.** (Sointu order lists with `-1`; Dark Forest's
   `b&&`, `b>1&&`; the paper's power-of-two form.)
   - Maps to: `sec(x) = floor(x / 32)`, and per voice `amp *= on(voice, sec(x))`, where `on` reads a digit of an
     integer exactly as `root(i)` does (e.g. `11110111` for "all sections but the breakdown").
   - 8-bar phrases, with an intro of 1–2 layers, a full section, a breakdown and a return.
3. **Fills and variation on the last bar of a phrase.** (Sointu drum variants 2/3/4 at every 8th order;
   Tidal `whenmod 8 6`; ThingCore's riser *"at the end of every 8 bars"*.)
   - Maps to: `last(x) = floor(mod(x, 32) / 28)` (1 in the last bar of 8), selecting an alternative
     pattern integer or a roll (`gate` every step), plus a rising noise/pitch swell into the downbeat.
4. **Sidechain derived from the actual kick pattern, applied to everything else.** (0b5vr `duck`, used on hats,
   pads and sub; `mix(0.3, 1.0, duck)`.)
   - Maps to: `duck(x) = smoothstep(clamp(kick_age(x)/0.8, 0, 1))`, and every non-drum voice's `amp *=
     0.3 + 0.7·duck(x)`.
   - Our `pump` is beat-locked; tying it to the kick's own pattern makes the groove breathe with the drums.
5. **Rhythms as bit patterns and Euclidean gates, one per instrument.** (0b5vr `0x9192`, `0x2424`, `0xaaaa`;
   Toussaint/Strudel `(3,8)`; Sonic Pi `spread`.)
   - Maps to: `bit(n, i) = mod(floor(n / 2^(15 - i)), 2)` and `gate = bit(KICK, step(x))`; Euclid as
     `mod(mod(step + r, n)·k, n) < k`.
   - The composer chooses patterns per genre (offbeat hats, a backbeat clap, a syncopated bass) instead of
     gating everything identically.
6. **Deterministic "random" velocity, ghost notes and drops: a hash of the step.** (0b5vr `fract(0.417 *
   seq.s)`; Strudel `rand` as a function of time; Sonic Pi's repeatable random; Tidal `?`.)
   - Maps to: `hash(i) = frac(sin(i·12.9898)·43758.5453)` **(ours, a common shader hash; any good one works)**.
   - Then `vel = 0.75 + 0.25·hash(step)`, and `gate *= hash(step + 7) > 0.2` for degraded hats.
   - Humanises without breaking loop-exact export.
7. **Extended-chord voicings with voice leading.** (Chasyxx `[0,4,7,11]`, *"The most beautiful chords"*;
   0b5vr `CHORD (0,4,6,11)` plus a `PATTERN` of transpositions; measure_plan's `fold`.) City pop lives on maj7,
   m7, 9ths and slash chords.
   - Maps to: the chord tone `fold(h(deg + 2k))` for k = 0..3 (and 4 for the 9th).
   - Keep `fold` so the pad moves by steps.
   - The composer chooses 7ths by default and triads for contrast.
8. **Stateless echo / canon: the same voice re-evaluated at t − d with decaying gain.** (Dark Forest
   `S(2,0)+S(2,0,3)/2`; 0b5vr's 5-tap chord delay with darker repeats; Tidal `off 0.125`.)
   - Maps to: a user function `lead(x)`, used again in a second voice at `lead(x - 0.75)` with `gain·0.4`. A
     dotted-eighth echo is the city-pop/synth-pop signature.
   - A real feedback delay or reverb belongs in the Rust engine (0b5vr: these need state).
9. **Swing everywhere, plus a little per instrument.** (measure_plan `sw`; SWANG `t-=t/4&256`; 0b5vr
   `SWING = 0.51`; Tidal `swingBy (1/3)`.)
   - Maps to: we already have `sw`. Make it a song-level parameter (51–60%) that every voice and every envelope
     onset reads.
   - The envelopes must use warped time too, or the notes swing but their decays don't.
10. **Arpeggios and melodies as digit/character tables indexed by step, with a rest symbol.** (a1k0n
    `"0 0     7 7     037:<<"` with space = rest; ryg `"36364689"`; Rrrola `0xCA98`; Desmos `L=[…,R,…]`;
    measure_plan's 16-digit leads with `9` = rest.)
    - Already our model. The lesson from bytebeat is that **the musical ones are composed tables, not found
      formulas**: the composer should write motifs (statement, repeat, vary the ending) rather than search
      formula space.
11. **Polymetric layers:** a short pattern against the bar. (0b5vr's 5-step chord transposition against 16
    steps; Tidal `{…}%8`.)
    - Maps to: `mod(step, 3)` or `mod(step, 5)` indexing an arpeggio while the bar is 16. This gives long-period
      variation for free and suits arps and hats.
12. **Timbre by unison/detune and partial-limited (filtered) waves, with a per-note filter envelope.** (0b5vr
    `shotgun`, Box-Muller detune, `ladderLPF` with `lpfcut ∝ exp2(6·fenv)`; Oidos.)
    - Maps to: mostly engine work. Add `detune` and `cutoff` fields to voices, where `cutoff` may be a function
      of the note age, so a pluck opens and closes.
    - Lowest on the list only because it is synthesis, not composition. It would still move us a long way from
      "plays correctly" towards "sounds like a record".

**A caution from all of them:** every source that sounds good is a *composed* structure (tables, order lists,
named sections). None is a clever closed formula. The 2011 bytebeat community found the limit of formula search
within weeks. Our composer should generate structure (form, phrases, motifs, fills, layer entries) and use maths
only to *express* it.
