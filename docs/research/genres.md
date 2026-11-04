# Genre recipes for the composer

Research for the composer module (build step 4 in `DESIGN.md`). Written 2026-11-02. Licences and sources for everything
cited here are in [`sources.md`](sources.md).

**How to read the figures.** Tempos and swing values marked with a URL come from that source. Drum grids, chord lists
and arrangement lengths are the genre's conventional archetypes as producers teach them, written out so a program can
use them; they are a starting point, not measurements of a corpus. Where a figure is a practitioner convention with no
source I could check, it says **(convention)**. Where sources disagree, both values are given.

## Notation used throughout

- **Grid:** one bar of 4/4 = 16 steps. Step 1 is the downbeat; beats fall on steps 1, 5, 9 and 13. Groups of four are
  separated by a space.
- **Velocity:** a digit 1–9, roughly MIDI velocity = 14 × digit (9 ≈ 127, 5 ≈ 70, 2 ≈ 28, a ghost note). `-` is a rest.
- **Swing:** given in Roger Linn's convention, the one MPCs use: the percentage of an 8th note's duration taken by its
  FIRST 16th. 50% is straight; 66.7% is a triplet shuffle. Linn: "I merely delay the second 16th note within each 8th
  note", and 54% "will loosen up the feel without it sounding like swing"
  ([Attack Magazine interview](https://attackmagazine.com/features/interview/roger-linn-swing-groove-magic-mpc-timing/)).
  - **As a timing offset:** each even-numbered 16th (steps 2, 4, 6 …) is delayed by `(S − 50)/100 × 0.5` beats. At
    58%: +0.04 beat (20 ms at 120 BPM). At 62%: +0.06 beat. At 66.7%: +0.083 beat.
  - **8th-note swing** (lo-fi, boom bap, shuffles) is the same rule one level up: the offbeat 8th is delayed by
    `(S − 50)/100 × 1` beat.
  - **The reference formula is 58% 16th swing.** In `DESIGN.md`, `sw = (floor(2x) + max(frac(2x)/1.16,
    (frac(2x) − 0.16)/0.84)) / 2` maps real time `u = frac(2x) = 0.58` to warped time 0.5: the second 16th of each
    8th sounds at 58% of the 8th. (Derived here from the formula; not stated by its author.)
- **Chords:** Roman numerals relative to the key's tonic. Upper case = major triad base, lower case = minor.
  `maj7`, `7` (dominant), `m7`, `m7b5`, `9`, `sus4`, `6`. `bVII` etc. = borrowed from the parallel minor.

---

## 1. Which genres suit this program

Each genre is scored 1–5 on the five criteria in the brief. **Loop** = periodic, repetitive structure (functions of `t`
excel at it). **Groove** = carried by rhythm and feel. **Rules** = harmony expressible as compact rules. **Synth** = the
sound is achievable with oscillators, filters, noise drums, delay and reverb, with no samples. **Appeal** = audience
size today and fit with the project's reference.

| Rank | Genre | Loop | Groove | Rules | Synth | Appeal | Total | Why, in one line |
|---|---|---|---|---|---|---|---|---|
| 1 | **City pop** | 4 | 5 | 4 | 4 | 5 | 22 | The reference itself; rich but rule-shaped harmony; FM electric piano and slap-ish bass are synthesisable |
| 2 | **Lo-fi hip hop** | 5 | 5 | 5 | 5 | 4 | 24 | Loops of 2–4 chords, swing, and a lo-fi finish that *hides* the plainness of simple synthesis |
| 3 | **Synthwave** | 5 | 3 | 5 | 5 | 4 | 22 | Native synth music: saw/square/PWM, arpeggios as functions, straight time |
| 4 | **House** (incl. deep / French / future funk) | 5 | 5 | 4 | 4 | 4 | 22 | Four-on-the-floor loops, sidechain is a formula, reuses city pop's 7th/9th chords |
| 5 | **Trap** (and phonk as a variant) | 5 | 4 | 4 | 5 | 5 | 23 | An 808 is a sine with a pitch envelope; hat rolls are rate functions; huge audience |
| 6 | **Chiptune** | 5 | 3 | 4 | 5 | 3 | 20 | Literally pulse/triangle/noise; arpeggiated chords are the idiom |
| 7 | **Techno** | 5 | 4 | 3 | 4 | 3 | 19 | Purest loop music; development is filter and density automation, not harmony |
| 8 | **UK garage (2-step)** | 4 | 5 | 4 | 3 | 3 | 19 | Swing and skipped kicks are encodable; loses points because vocal chops carry the genre |
| 9 | Drum & bass | 4 | 4 | 3 | 2 | 4 | 17 | Reese bass synthesises well, but the sampled break (Amen, Think) is the genre's identity |
| 10 | Phonk | 5 | 3 | 3 | 3 | 4 | 18 | Distorted 808 and tuned 808 cowbell are easy; the Memphis vocal samples are not. Covered under trap |
| 11 | Boom bap | 4 | 5 | 3 | 2 | 3 | 17 | Built on chopped samples; the drums and swing transfer to lo-fi, which is where they live here |
| 12 | Ambient | 3 | 1 | 4 | 5 | 3 | 16 | Trivial to synthesise but no groove; useful as the composer's *breakdown* and intro vocabulary |
| 13 | Vaporwave / future funk | 3 | 3 | 4 | 2 | 3 | 15 | Vaporwave is slowed samples by definition; future funk is folded into house below |

Lo-fi totals highest but ranks below city pop because city pop is the project's stated reference and exercises the
richer harmony that lo-fi then reuses. Trap scores well and is placed fifth because its melodic content is thin, so it
teaches the composer less.

---

## 2. Recipes

### 2.1 City pop (1977–1988 Japanese AOR / boogie; the Stereo Drift reference)

- **Tempo:** 95–125 BPM; the boogie/disco-leaning tracks sit around 110–120 **(convention)**.
- **Swing:** straight to light, 50–56% on 16ths. The reference formula uses 58%.
- **Drums** (live-kit feel, disco/boogie roots):

  ```
  Kick   9--- ---- 8-5- ----
  Snare  ---- 9--2 ---- 9-2-
  CHat   6363 6363 6363 6363
  OHat   ---- ---- ---- ---- (variant: --5- --5- --5- --5- for the disco chorus)
  ```

  Ghost snares at velocity 2 on steps 8 and 15; a tom or snare fill in the last two beats of every 8th bar.
- **Harmony:** major key, every chord at least a 7th; 9ths and 11ths on top; one or two chords per bar.
  - Royal road (王道進行): `IVmaj7 – V7 – iii7 – vi7`, and its long form `IVmaj7 – V7 – iii7 – vi7 – ii7 – V7 – I`
    ([Wikipedia](https://en.wikipedia.org/wiki/Royal_road_progression)). Variant with a secondary dominant:
    `IVmaj7 – V7 – III7 – vi7`.
  - The "Just the Two of Us" / marusa (丸サ) progression: `IVmaj7 – III7 – vi7 – v7 – I7`, the I7 being V7/IV so the loop
    restarts ([nippon.com, which gives the original as D♭M7–C7–Fm7–E♭m7–A♭7](https://www.nippon.com/ja/japan-topics/g01096/)).
  - Turnarounds: `ii7 – V7 – Imaj7 – vi7`; `Imaj7 – VI7 – ii7 – V7` (VI7 = V7/ii).
  - Borrowed colour: `IVmaj7 – ivm6` (or `ivm7`), resolving to `iii7` or `Imaj7`; `V7sus4` before `V7`, or used *instead
    of* V (often voiced as IV/V).
  - Harmonic rhythm: one chord per bar in verses, two per bar in turnarounds.
- **Voicing:** close voicings, 4–5 notes, kept within an octave and a half by the reference's `fold` (it holds the top
  note near a fixed register). Rootless (3–7–9 or 7–3–13) in the electric piano, with the root in the bass.
- **Bass:** the busiest voice after the hats. Root on beat 1, octave jump on the "and"s (root–octave–root), 16th pickups,
  chromatic approach from a semitone below or above into each chord change, occasional slides into the root. Slap-style:
  octave "pops" on steps 7 and 15.
- **Melody / hook:** range of about a 10th; mostly 8th and 16th notes with syncopated anticipations (notes on step 16
  tied over the bar line). The motif is 1 bar, stated twice, answered by a varied 2-bar phrase (an AABA' 4-bar shape).
  Pentatonic plus the 9th and the major 7th; chord tones on the strong beats. The reference stores a lead as one 16-digit
  integer per section, which is exactly this: one bar of 16 steps per section, repeated.
- **Harmony voice:** a parallel third above the lead (the reference does this), or a guitar/brass doubling.
- **Arrangement (typical 4–5 min song; for an 8-bar-loop program, scale it down):** intro 4–8 bars (keys and drums, or
  the hook on synth brass) → verse A 8 → pre-chorus B 4–8 (royal road, rising) → chorus 8 (full band, open hats) →
  interlude 4 (the hook instrumentally) → verse 2 → chorus ×2 → sax or synth solo 8 → final chorus → fade or a cold end on
  `Imaj9`.
- **Sound palette:**
  - Electric piano: 2-operator FM, a modulator at a 1:1 ratio with a fast-decaying index (bell attack), and a carrier
    with a slow exponential decay. Light chorus.
  - Bass: a saw or square through a lowpass at about 600–1200 Hz with a short filter envelope (a pluck); add a short
    noise transient for the slap.
  - Synth brass stabs: 2–3 detuned saws (about ±7 cents) and a lowpass with an envelope attack of 20–50 ms.
  - Pad: detuned saws, slow attack.
  - Drums: a sine-sweep kick (about 150→55 Hz in 50 ms), a noise-plus-200 Hz-tone snare, and high-passed noise hats.
  - Space: a room reverb on the snare and a ping-pong 1/8-dotted delay on the lead **(convention)**.

### 2.2 Lo-fi hip hop

- **Tempo:** 70–90 BPM; some sources say 80–110
  ([Orphiq](https://orphiq.com/resources/what-is-lofi-music),
  [BeatSync Pro](https://beatsyncpro.ai/blog/make-lo-fi-beats-ableton-live-2026.html)).
- **Swing:** 8th-note swing of about 55–62%; one guide suggests "50–70%" groove (a single source). Snare laid back by
  roughly 10–30 ms behind the grid, the "drunk" or Dilla feel **(convention)**.
- **Drums:**

  ```
  Kick   9--- ---- --7- 5---    (variant: 9--- ---7 --6- ----)
  Snare  ---- 8--- ---- 8---
  CHat   5-3- 5-3- 5-3- 5-3-    (8ths, swung; drop 1 in 4 at random)
  Ghost  ---- ---2 ---- --2-
  ```

  Kick and snare velocities jitter ±1 digit per bar; hats jitter ±2.
- **Harmony:** 2- or 4-chord loops, all 7ths and 9ths, often minor.
  - `ii9 – V13 – Imaj9 – vi9` (a slow ii–V–I–vi).
  - `IVmaj7 – iii7 – ii7 – Imaj7` (descending diatonic).
  - Minor: `im9 – ivm9`, `im7 – bVImaj7 – bVII7` (or `– v7`).
  - Neo-soul: `IVmaj7 – III7 – vi7` (the marusa shape again), and `Imaj7 – i m7` as a borrowed minor shift.
- **Bass:** sparse; roots on beat 1 and an anticipated note on step 15 or 16, often a sine or a soft triangle.
  Sometimes no bass at all, the keys' left hand carrying it.
- **Melody / hook:** short (1–2 bars), sparse (3–6 notes per bar), pentatonic. Repeated nearly verbatim across the
  loop; variation by dropping notes rather than adding them.
- **Arrangement:** a 2–3 minute loop. Intro with filtered keys (lowpass at about 800 Hz, opening over 4 bars) → drums enter →
  A 8–16 bars → B (the melody changes, or the drums drop to kick-only) 8 → A → outro filtering back down. Changes are
  subtractive: mute drums for 1–2 bars before a section.
- **Sound palette:** a Rhodes-like FM piano or a sine-plus-harmonics piano; a wow and flutter pitch LFO (0.5–2 Hz, a few
  cents); a lowpass at about 4–8 kHz on the master; vinyl crackle (sparse random clicks plus band-passed noise, "around
  −40 dB" ([Mode Audio](https://modeaudio.com/magazine/lofi-hip-hop-5-production-essentials) /
  [Violet Recording](https://violetrecording.com/how-to-make-lofi-music/))); bit-depth or sample-rate reduction on the
  drums; soft kick (a sine at 50–60 Hz with a short click).

### 2.3 Synthwave (outrun, retrowave, darksynth)

- **Tempo:** 80–118 BPM; darksynth runs faster (about 120–150) **(convention)**.
- **Swing:** none. Straight 16ths are the idiom.
- **Drums:**

  ```
  Kick   9--- ---- 9--- ----    (driving variant: four-on-the-floor 9--- 9--- 9--- 9---)
  Snare  ---- 9--- ---- 9---    (huge gated reverb)
  CHat   5555 5555 5555 5555    (or 8ths 6-6- 6-6- ...)
  Toms   last bar of 8: descending 16th tom run on beats 3–4
  ```

- **Harmony:** minor keys. `i – bVI – bIII – bVII` (Am–F–C–G), `i – bVI – bVII` (– i), `i – bVII – bVI – bVII`,
  `bVI – bVII – i`. Triads and add9 or sus2 more than 7ths. One chord per bar, sometimes per two bars.
- **Bass:** relentless 8th notes on the root, or 16th "octave bounce" (root, octave, root, octave) **(convention)**;
  follows the chord root; no approach notes.
- **Arpeggio:** a 16th-note arpeggio over the chord tones (up, or up-down), 1–2 octaves, through a 1/8-dotted delay. This
  is a pure function: `note(t) = chord[ floor(4t) mod n ]`.
- **Melody / hook:** long notes (half and whole notes) on a lead with portamento; range about an octave; motif 2 bars;
  stated, repeated a step higher or with a changed last note.
- **Arrangement:** intro 8 (pads + arp) → verse 16 (drums, bass) → chorus 16 (lead, open snare reverb) → breakdown 8
  (pads + arp only, drums out) → build 4–8 (snare roll, filter opening) → chorus 16 → outro 8.
- **Sound palette:** detuned saws (2–7 voices, ±10–20 cents) through a resonant lowpass for the lead and pads; PWM pads
  (a pulse whose width is swept by an LFO at about 0.2–1 Hz); a square or saw bass with a filter envelope; a gated reverb
  snare (noise burst plus a long reverb cut hard at about 150–250 ms) **(convention)**; a chorus on pads; big plate reverb.

### 2.4 House (deep, classic, French / future funk as variants)

- **Tempo:** 118–128 BPM. Future funk 115–128 (hard future funk 125–135) per one commercial source
  ([Tunee](https://www.tunee.ai/music-generator/future-funk)); NI's future funk tutorial builds at 120
  ([Native Instruments](https://blog.native-instruments.com/future-funk-music/)).
- **Swing:** 50–58% on 16ths (the hats; the kick stays straight) **(convention)**.
- **Drums:**

  ```
  Kick   9--- 9--- 9--- 9---
  Clap   ---- 8--- ---- 8---
  CHat   -4-4 -4-4 -4-4 -4-4    (or full 16ths 3535 ...)
  OHat   --7- --7- --7- --7-    (the offbeat 8th: the house signature)
  Perc   shaker 16ths at 2–3; a rim or conga on steps 4, 11, 14
  ```

- **Sidechain** (the "pump"), as a function: gain after each kick at time `t_k`:
  `g(t) = 1 − d·exp(−(t − t_k)/τ)`, with `d` = 0.5–0.9 and `τ` about 0.08–0.15 beat; applied to bass, chords and pads
  **(convention for the values)**.
- **Harmony:** deep house loops minor 7th / 9th / 11th stabs: `im9 – ivm9`, `im7 – bVIImaj7`, `ii9 – V13sus4`. Classic
  organ / piano house: `I – V – vi – IV` with 7ths, or the gospel `IV – V – iii – vi` (the royal road again). French /
  future funk: the city pop progressions from 2.1, filtered.
- **Bass:** the offbeat 8th (steps 3, 7, 11, 15), or a syncopated 16th riff with octave jumps; sidechained so it never
  sits on the kick.
- **Chord stabs:** short (1/16–1/8), syncopated, e.g. on steps 4, 7, 11, 14 **(convention)**.
- **Melody / hook:** often absent; the hook is the chord stab rhythm or a vocal. When present: a 1-bar riff, repeated
  for 16–32 bars, varied by filter.
- **Arrangement** (DJ-friendly, in 8-bar phrases): intro 16–32 (drums only, for mixing) → groove 16 (bass enters) →
  main 32 (chords) → breakdown 16 (kick out, chords and pads, filter sweep) → build 8 (snare/clap roll accelerating
  8ths→16ths→32nds, white-noise riser, highpass on the mix) → drop 32 → outro 16–32 (drums only).
- **French / future funk:** the whole mix through a lowpass whose cutoff opens over 8–16 bars (200 Hz → 20 kHz,
  exponential), heavier sidechain, chopped and repitched city pop phrases (here: the city pop generator's own phrases,
  sliced into 1/8 chunks and retriggered).
- **Sound palette:** a sine kick with a click (about 50–60 Hz body); noise clap with 3 staggered noise bursts about
  10 ms apart **(convention)**; an organ (sine partials at 1, 2, 3, 4) or an M1-style piano stab (FM); a sine or
  filtered-saw bass.

### 2.5 Trap (with phonk)

- **Tempo:** 130–150 BPM, felt in half-time (snare on beat 3). Equivalently 65–75 BPM with 32nd-note hats.
- **Swing:** none on the hats; the bounce comes from triplet rolls.
- **Drums (half-time):**

  ```
  Kick   9--- ---- --7- ----    (+ 808 under every kick)
  Snare  ---- ---- 9--- ----
  CHat   7575 7575 7575 7575    (16ths at 140 = fast; rolls replace bars 4 and 8 partly)
  OHat   ---- ---- ---- --6-
  ```

  **Hat rolls** as a function: within a roll segment, the hit rate jumps from 16ths to 16th-triplets, 32nds, or 64ths
  for a half-beat to a beat, with velocity ramping up or down and an optional pitch ramp of ±2–5 semitones
  **(convention)**.
- **808:** a sine at the bass note with a pitch drop at onset (an octave or so above, settling within about 30–60 ms),
  long decay (0.5–2 s), saturated for upper harmonics so it is heard on small speakers. **Glide:** when two 808 notes
  overlap, the pitch slides from the first to the second over about 30–120 ms (portamento) **(convention)**. Typical
  glide intervals: up an octave, a 5th, or down a minor 3rd. The 808 IS the bass and largely the kick's low end.
- **Harmony:** minor or modal; often no chords at all, only a 2–4 note melody loop. When chords: `i – bVI`, `i – iv`,
  harmonic minor's `i – V`, phrygian `i – bII`. Diminished and harmonic-minor colour for darkness.
- **Melody / hook:** a 1- or 2-bar loop of bells, plucks, flute or choir; 4–8 notes per bar; repeated with no variation
  for 8 bars, then one note changed.
- **Arrangement:** intro 4–8 (melody alone, often filtered) → hook 16 (everything) → verse 16 (hats thinner, 808 simpler)
  → hook 16 → bridge 8 (drums drop out) → hook 16. Drop-outs of the 808 for one beat before a section are the main fill.
- **Phonk variant** (drift phonk): tempo sources conflict: 130–160
  ([EDMProd](https://www.edmprod.com/how-to-make-phonk/)) against 90–120 per
  [Native Instruments](https://blog.native-instruments.com/phonk-music/). Both may describe the same groove at double and
  half time. The lead is a **tuned 808 cowbell** (two square waves at about 540 and 800 Hz, the classic TR-808 recipe,
  band-passed, short decay; **(convention, frequencies unverified here)**) playing a minor melody, heavily distorted, over
  a distorted 808.
- **Sound palette:** sine 808 plus a soft-clip saturator; a punchy short kick; a clap or snare with a short reverb;
  sine-bell or FM-bell melodies; reverse-reverb swells before sections.

### 2.6 Chiptune (NES-style)

- **Tempo:** 120–180 BPM; tempos are often quantised to the frame clock (60 Hz NTSC), so note lengths are whole numbers
  of frames.
- **Swing:** none, or a shuffle by alternating 7- and 5-frame steps (a frame-exact swing) **(convention)**.
- **Channels:** the 2A03 has two pulse channels with duty cycles of 12.5%, 25%, 50% and 75%, one triangle (stepped,
  used for bass), and one noise channel (drums). Writing for 4 voices is the genre's constraint and its sound.
- **Drums on the noise channel:** kick = a short low-period noise burst (or a fast triangle pitch drop), snare = longer
  mid noise, hats = very short high noise.

  ```
  Kick   9--- ---- 9-9- ----
  Snare  ---- 9--- ---- 9---
  Hat    -5-5 -5-5 -5-5 -5-5
  ```

- **Harmony:** I–V–vi–IV, `vi – IV – I – V`, `i – bVI – bVII – i`; heroic `bVI – bVII – I` cadences.
- **Arpeggiated chords:** one pulse channel cycles the chord tones every frame or every 2 frames (30–60 notes per second)
  to fake a chord on a single voice. A pure function: `note(t) = chord[ floor(t · f_frame) mod 3 ]`.
- **Bass:** the triangle in 8ths, octave bouncing.
- **Melody / hook:** fast and wide (up to 2 octaves), 16th runs, vibrato after a delay (onset + about 150 ms, ±20–30 cents
  at 5–7 Hz) **(convention)**. Duty-cycle changes (12.5% → 50%) mark sections.
- **Arrangement:** game-loop form: intro 4 → A 8 → A 8 → B 8 → loop back to A, seamless.
- **Sound palette:** naive (non-band-limited) pulse and a 16-step quantised triangle are authentic. Little or no reverb;
  echo is faked by a second voice repeating the line quieter, an 8th late.

### 2.7 Techno

- **Tempo:** 125–135 (classic / Detroit / minimal), 135–150 (modern peak-time and hard techno) **(convention)**.
- **Swing:** none on the kick; slight (52–55%) on percussion.
- **Drums:**

  ```
  Kick   9--- 9--- 9--- 9---
  Rumble -3-3 -3-3 -3-3 -3-3    (reverb tail of the kick, lowpassed at about 150 Hz, sidechained)
  Clap   ---- 7--- ---- 7---    (often absent)
  OHat   --7- --7- --7- --7-
  CHat   5353 5353 5353 5353
  Perc   a 3-step or 5-step loop against the 16-step bar (polymeter): e.g. a rim every 3 steps
  ```

- **Harmony:** one chord or none. A single minor 7th or minor 9th stab, or a one-note bass, for the whole track. Tension
  comes from timbre (filter cutoff, resonance, delay feedback), not from chord change.
- **Bass:** offbeat 16ths, or a rolling 16th bassline where every step after the kick plays and the kick step is silent.
- **Hook:** a 1-bar or polymetric riff (3 or 5 steps long against 16) that phases against the bar; Euclidean rhythms
  E(3,8), E(5,16), E(7,16) are idiomatic (Toussaint, see `sources.md`).
- **Arrangement:** in 16- and 32-bar blocks; elements enter one every 16 bars; a breakdown at about 1/2 to 2/3 of the
  track (kick out for 16–32 bars) with a long filter rise; the kick returns on a downbeat.
- **Sound palette:** a sine kick with a long pitch envelope and saturation; a rumble (reverb into a lowpass, sidechained to the
  kick); FM or noise percussion; resonant filter sweeps automated over 32–64 bars; a dub delay with feedback 50–80%.

### 2.8 UK garage (2-step)

- **Tempo:** 130–138 per [Wikipedia](https://en.wikipedia.org/wiki/UK_garage); 2-step specifically about 130–135
  ([Orphiq](https://orphiq.com/resources/what-is-uk-garage)); NI's example is at 132
  ([Native Instruments](https://blog.native-instruments.com/uk-garage-music/)).
- **Swing:** 16ths at about 54–60% ([MusicRadar](https://www.musicradar.com/how-to/uk-garage-tutorial),
  [SampleFocus](https://samplefocus.com/blog/how-to-produce-uk-garage-music)); the shuffle is the genre.
- **Drums** (the kick skips beats; per NI, kick on beat 1 and halfway through beat 3, snares on 2 and 4):

  ```
  Kick   9--- ---- --8- ----    (variants add a kick on step 8 or 16: 9--- ---6 --8- ---5)
  Snare  ---- 9--- ---- 9---
  CHat   -5-5 -5-5 -5-5 -5-5    (swung 16ths, or offbeat only)
  OHat   --6- --6- --6- --6-
  Shaker 3434 3434 3434 3434
  ```

- **Harmony:** minor 7ths and 9ths, often 2-chord: `im9 – ivm9`, `im7 – bVIImaj7`, `ivm9 – v m7`; organ stabs.
- **Bass:** sub (a sine) with syncopated phrasing and slides, or a "warp" bass (filtered square with LFO); avoids
  beat 1 sometimes, so the kick has it alone.
- **Hook:** a chopped vocal (here: a formant-filtered synth phrase or a pitched short sample) stuttered on 16ths; a
  1-bar motif.
- **Arrangement:** intro 16 (drums) → verse 16 → chorus / drop 16 → breakdown 8 → drop → outro 16; in 8-bar phrases.
- **Sound palette:** a short tight kick, a crisp noise snare, shuffling shakers; a sine sub; an organ (sine-partial)
  or M1-style stab.

### 2.9 Shorter notes on the rest

- **Drum & bass:** 170–176 BPM; the two-step DnB pattern is kick on 1 and on step 11, snare on 5 and 13 (a breakbeat
  shape); a Reese bass (two detuned saws, ±10–30 cents, lowpassed and slowly phasing). Without break samples, a synthetic
  break can be built from the same short noise hits.
- **Boom bap:** 85–95 BPM, 8th swing 55–66%, kick on 1 and step 11, snare 5 and 13, chopped samples. Here, use lo-fi's
  recipe with harder drums and less filtering.
- **Ambient:** no grid; long tones (8–32 s envelopes), slow LFO (0.01–0.1 Hz) on filters, reverb tails of 5–20 s; chords
  as sus2 and add9 stacks of 5ths. The composer's intros and breakdowns can borrow it.
- **Vaporwave:** slowed (playback rate 0.7–0.85, pitch drops with it), chopped and looped samples of 80s music with heavy
  reverb. The program can imitate it by running its own city pop output at a lowered rate.

---

## 3. What makes music hit "hard" (genre-independent)

Each rule is stated as something the composer can do, followed by the evidence for it and how strong that evidence is.

### 3.1 Groove and microtiming

1. **Aim for medium syncopation, not maximal.** In Witek et al. 2014, medium syncopation in funk drum breaks produced
   the most wanting-to-move and pleasure (an inverted U; 66 participants rating 50 drum breaks in a web survey)
   ([PLOS ONE, CC BY](https://doi.org/10.1371/journal.pone.0094446)). Matthews et al. 2019 replicated the inverted U for
   rhythm (201 participants), but harmonic complexity did not show a clear one
   ([PLOS ONE](https://doi.org/10.1371/journal.pone.0204539)). **Encodable:** compute a syncopation index per bar
   (Witek et al.'s index is "broadly modelled on" Longuet-Higgins & Lee, with a flatter metre and extra per-instrument
   weights; its exact weights are in the paper's supplement, Text S2, not read here) and keep the drum-plus-bass pattern in a middle band;
   the kick on 1 and the backbeat stay put, and syncopation lives in the bass, ghost notes and stabs.
2. **Quantise or use a consistent swing; do not add random jitter and call it human.** Senn et al. 2016 found fully
   quantised music and music with the original performer's microtiming were "rated equally high on groove", and
   *exaggerated* microtiming lowered the ratings ([Frontiers](https://www.frontiersin.org/journals/psychology/articles/10.3389/fpsyg.2016.01487/full)).
   Frühauf, Kopiez & Platz 2013 and Davies et al. 2013 are cited as finding microtiming associated with *less* groove
   (secondary reports; the original papers were not read). **Encodable:** systematic offsets per instrument (snare a
   fixed few ms late, hats swung), not per-note randomness. Velocity variation is safer than timing variation.
3. **Swing is a parameter with a sweet spot, not on/off.** Linn: settings between 50% and about 70% turn a rigid beat
   into one that moves people; 66% is not always best (interview above). **Encodable:** genre default + a small search
   (±4%) scored by ear or by a later listening test.
4. **Beats have width.** Danielsen's "beat bin" theory: a beat is perceived as a zone of some width and shape, so
   layered sounds slightly apart (a kick and bass a few ms apart, a laid-back snare) can still read as one beat
   ([UiO RADR report](https://hf.uio.no/imv/english/research/projects/rhythm/sluttrapport.pdf)). This is why item 2's
   systematic offsets work. (Theory; widths in ms not taken from the book here.)

### 3.2 Bass and kick

5. **Give the low end one owner at a time.** Sidechain the bass to the kick (the gain formula in 2.4), or don't play bass
   on kick steps (techno's rolling bass, house's offbeat bass), or make the 808 the kick (trap). **Tune the kick** to the key
   root or 5th when it has a pitched body **(convention)**.
6. **Low frequencies drive movement.** Very-low-frequency sound (reported as 8–37 Hz) that listeners could not detect
   increased dancing at a live concert by about 12% (Cameron et al. 2022, *Current Biology*; figures from press coverage
   of the paper, original not read). Hove et al. 2014 (PNAS) argued that timing is perceived better in lower voices, which
   would explain why the bass carries rhythm; Wojtczak et al. 2017 did not replicate that advantage. **Treat the
   mechanism as contested; the practical rule (a strong sub under the rhythm) stands on the dance study.**
7. **Saturate the bass for small speakers.** A pure sine 808 below about 60 Hz vanishes on laptop speakers; adding
   harmonics (soft clipping) lets the ear infer the fundamental **(convention, the missing-fundamental effect)**.

### 3.3 Tension and release, and contrast

8. **Build, remove, return.** Solberg & Jensenius (Empirical Musicology Review, 2017; 16 dancers, motion capture)
   found movement fell during breakdowns and rose at drops; participants rated the build-up and drop most pleasurable
   ([UiO record](https://duo.uio.no/handle/10852/55312)). **Encodable:** a drop hits hard in proportion to what was
   *absent* before it: take the kick and sub out for the last 1–8 bars, raise a riser (noise through a rising bandpass,
   pitch up an octave over 4–8 bars), accelerate the snare roll (8ths → 16ths → 32nds), highpass the mix and let it
   go on the drop's downbeat, and leave one beat of near-silence before the downbeat.
9. **Anticipation is half the reward.** Salimpoor et al. 2011 (Nature Neuroscience) measured dopamine release in the
   caudate during the *anticipation* of a peak and in the nucleus accumbens at the peak itself (cite-only; from memory,
   not re-read for this file: verify before quoting). Huron's ITPRA theory (*Sweet Anticipation*, MIT Press 2006)
   frames the same thing. **Encodable:** signal the arrival (risers, a dominant chord held, a pickup fill) so the
   listener predicts it, then deliver on the predicted downbeat.
10. **Contrast between sections in at least two dimensions.** Density (number of active voices), register (bass and lead
    spread), brightness (filter cutoff) and rhythm (half-time vs full) should change at section boundaries
    **(convention)**. The composer's energy curve can drive these four numbers directly.

### 3.4 Hooks and repetition

11. **Repeat more than feels safe, vary less than feels necessary.** Margulis (*On Repeat*, Oxford UP 2014) argues
    repetition is what makes sound heard as music and lets listeners participate (cite-only). **Encodable:** a motif of
    1–2 bars, stated 2×, then a varied answer (AABA' or AAAB); vary the ending, not the beginning; keep the rhythm and
    change one or two pitches.
12. **Hook = rhythm first, pitch second.** A hook should be recognisable when clapped. Keep its rhythm fixed across
    repetitions and harmonisations **(convention)**.

### 3.5 Harmony

13. **Rich chords help pleasure but not groove.** In Matthews et al. 2019, low- and medium-complexity chords were rated
    similarly. **Encodable:** spend harmonic complexity on colour (7ths, 9ths, a borrowed iv), not on fast chord changes;
    a harmonic rhythm of 1–2 chords per bar.
14. **Voice-lead by the smallest motion.** Keep common tones, move other voices by step (the reference's `fold` does a
    crude version). The cost function: sum of |semitone moves| between voicings, minimised over inversions within a
    register window (Tymoczko's voice-leading geometry; see `sources.md`).
15. **Roughness as a check, not a rule.** A Plomp–Levelt / Sethares roughness score over the synthesised partials can
    flag voicings that are muddy in the low register (e.g. thirds below about C3) **(the "low interval limit",
    convention)**.

---

## 4. Recommendation

**Master first: city pop, lo-fi hip hop, synthwave.**

1. **City pop.** It is the project's reference and the hardest harmony of the three; get its chord engine (the royal road,
   marusa, ii–V chains, secondary dominants, a borrowed iv) and its busy bass right, and most of the harmony work for
   lo-fi, house and future funk is done.
2. **Lo-fi hip hop.** It reuses city pop's harmony engine at half the density and adds the two things the composer most
   needs to learn separately: 8th-note swing and subtractive variation. Its lo-fi finish forgives simple synthesis while
   the engine matures.
3. **Synthwave.** It is straight-time, triad-based and arpeggio-driven, so it tests the other half of the composer:
   arrangement and the energy curve (breakdown, build, return) over a deliberately simple harmony. Its sounds are the
   engine's native oscillators.

House is the natural fourth: it adds the four-on-the-floor drop and the sidechain formula, and future funk joins it to
city pop.

**Smallest set of knowledge files to vendor** (details and licences in `sources.md`):

1. **`docs/research/genres.md`** (this file): the recipes, ours.
2. **A progression table as data** (`progressions.json`, written by us): the progressions in section 2 as Roman-numeral
   lists with qualities and per-genre weights. Our own facts, no licence issue. Optionally re-weighted later from the
   **McGill Billboard** annotations (CC0, safe to vendor) or from Chordonomicon (CC BY-NC per its current card, not to
   be vendored).
3. **A groove-template file** (`grooves.json`, written by us): per genre, per instrument, the 16-step velocity grid plus
   a mean timing offset per step. Start from the grids above; refine the offsets from **Magenta's Groove MIDI Dataset**
   (CC BY 4.0: usable with attribution), by computing per-step mean microtiming and velocity by style.
4. **Selected Open Music Theory chapters** (CC BY-SA 4.0) on voice leading and pop harmony, vendored as extracts with
   attribution in a folder of their own, so the share-alike applies to those files only.

Everything else (Strudel, TidalCycles, Sonic Pi) is reference to read, not to copy: their docs are copyleft (GPL-3.0 /
AGPL-3.0 / CC BY-SA), and Strudel's FAQ asks that contributed examples and tracks not be used to train AI models without
permission.
