# The graph sound: what the reference recordings are made of

Measured 2026-11-02 for the v4 round, after listening tests found v3 did not sound like graph music: in the reference videos
the whole sound comes out of the graph, and the kick is a fast downward pitch dash. The audio is the two reference videos' soundtracks
(AAC 128 kb/s, 48 kHz, mixed to mono); the scripts are in `tools/measure/` (numpy, dev only). These are hand measurements.
Each figure says how many cases it rests on.

## What Desmos plays

- `tone(frequency, gain)` plays a sine; its gain is capped at `min(10, 660 / frequency)` (Desmos help centre, "Tones").
  Above 66 Hz the cap is an amplitude proportional to 1/f: -6 dB per octave.
- A public Desmos synth graph (`desmos.com/calculator/v4ap2rzot8`, by @phras_22, read from its saved state) builds its
  sounds from several `tone()` calls whose folders are named "sine wave": richer sounds are sums of sines.

## What the recordings show

| Finding | Evidence | Used as |
|---|---|---|
| Tones are harmonic sine sums. A lead note's partials: h1 0, h2 -22, h3 -14, h4 -27, h5 -25, h6 -34, h7 -33, h8 -41, h9 -42, h10 -48 dB | 7 lead notes in city, 500-1240 Hz, within a few dB of each other (one high note, D6, was purer) | the lead's list, with 20 log10 k added back for the cap |
| Chord tones carry the same h3 and h5 | the intro chord: 660.1 Hz = 3 x 220.0 beside E5 at 658.5; A3's h5 at 1100 Hz | the pads |
| The chords shimmer at 1-2 Hz (+-3 cents, 8 dB) with no detune | demodulating A3 and C6 in the intro: pitch and level move together at the beat rate, which is beating, not vibrato | no `detune` in v4; equal-tempered chords of harmonic tones beat by themselves |
| The bass is an octave pair: F1 at -11 dB under F2 at 0, with h4 -12 and h8 -21 | the intro chord | the bass list |
| The kick is a dash: `f = f1 + (f0 - f1) e^(-tau/k)`, k about 23 ms, f0 400-600 Hz, f1 60-80 Hz, level held about 50 ms | 4 city kicks (fits at 2 of them, both k 22.6-24 ms); v's kicks fall from about 300 Hz to 40-60 Hz in 10-20 ms (too dense to fit) | `kind = dash`, 450 to 58 Hz, 23 ms |
| The dash carries harmonics like the tones': h2 weak, h3 about -12, h4-h5 about -20, h6-h7 about -25, h8 about -29 dB | 4 city kicks, 2 times each; spread +-5 dB | the kick's list |
| No noise drums. Above 2 kHz the recordings are tonal | spectral flatness 2-10 kHz: city 0.002, v 0.024, white noise 0.57 | snare and hat are dashes too |
| The off-beat hits are short tonal dashes, about 360 to 240 Hz in 15 ms | 2 city off-beats | the snare (a guess at its exact shape) |
| The lead slides up into each note in about 15-20 ms | 1 note tracked, 600 to 782 Hz | v4's scoop: 30 ms |
| Both videos build up: peak density rises from about 55 to 130 per frame (city) and 79 to 217 (v) as layers are added | per-5-second medians | compare v4's full sections with theirs, not whole-file medians |

The peak-density metric is not inflated by the codec: our v4 render through AAC 128 kb/s and back counted 70 against 71.

## Not settled

- The hat: no distinct hat was found in either recording; v4's is a short high dash held quiet by the cap, a guess.
- Whether the reference creators set each partial's gain by hand or rode the cap; the measured output is what v4 matches.
- Why the reference's frames hold more weak partials than v4's in 500 Hz-8 kHz, at equal band energy: probably more
  simultaneous notes as the videos build, not measured note by note.
