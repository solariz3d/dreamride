# Expression: what turns correct music into music that feels like something

Researched 2026-11-02 for the v3 round, after listening tests found v2 structurally right but lacking emotion.
What the engine and the v3 examples use, where it comes from, and what was and was not verified.

## Performance rules (encoded in `src/expressive.rs`)

| Finding | Source | Verified | Used as |
|---|---|---|---|
| High Loud: +3 dB per octave, `dL = (N - 60)/4 k` dB | Friberg, "Generative rules for music performance", *Computer Music Journal* 15(2), 1991, rule DPC 1B | Read in the paper's text | `(pitch - ref) / 4` in the songs' dB |
| Melodic charge by distance from the chord root around the circle of fifths: C 0, G 1, D 2, A 3, E 4, B 5, F# 6, Db 6.5, Ab 5.5, Eb 4.5, Bb 3.5, F 2.5; `dL = 0.2 C k` dB | Same paper, rule DPC 2A (after Sundberg) | Read in the paper's text | `charge(semis)`, at k = 2 |
| Phrase rule: the last tone of a phrase lengthened (+40 k ms) and followed by a micropause (80 k ms) | Same paper, rule GMA 1 | Read (partly garbled by the PDF extraction) | the gates: inner notes detached, phrase-final notes held to full length |
| Phrase arch: tempo and loudness rise into the phrase and fall at its end, roughly parabolic | KTH rule pages (speech.kth.se, "Phrase Arch"); Todd, "The dynamics of dynamics", JASA 91:3540, 1992 | KTH page read; Todd through secondary summaries only | `arch(x, peak)`, and `phrase()` whose tempo follows it |
| Final ritard `v(x) = [1 + (w^q - 1) x]^(1/q)`; q = 3 is constant braking power (stopping runners); listeners preferred q = 2 and 3 | Friberg & Sundberg, "Does music performance allude to locomotion?", JASA 105:1469, 1999 | Read in the paper's text (abstract and model section) | `ritard(x, q, w)` (in the engine; neither v3 song needed a ritard, since both loop) |
| Vibrato 5-7 Hz, extent up to about a semitone in operatic singing; rate and extent unstable at onset | Voice-science reviews (e.g. Lester-Smith 2021, search summaries) | Search summaries only | `vibrato()`, at 5.2-6 Hz and 0.08-0.32 semitones (a synth, not an opera singer) |

Not encoded: KTH Duration Contrast (the figure with its numbers did not survive PDF extraction), Leap Articulation, and
Harmonic Charge. They are candidates for a later round.

## Moments that move people (used deliberately in the v3 songs)

- **Sloboda 1991**, "Music structure and emotional response: some empirical findings", *Psychology of Music* 19:110-120:
  tears were most reliably linked to melodic appoggiaturas (and sequences); chills to new or unprepared harmony and
  sudden changes of texture or dynamics. Read through secondary write-ups, not the paper. The critique worth keeping:
  the study asked only which passages moved people, so appoggiaturas are associated with tears, not shown to cause
  them. Hence "sparingly": two or three per song, at chosen places.
- **Huron 2006**, *Sweet Anticipation* (ITPRA: imagination, tension, prediction, reaction, appraisal): a predicted
  event that is delayed and then delivered is felt more strongly. From memory of the book, not re-read this round.
  Used as the withheld tonic in city pop v3 and the late E6 lift in dark v3.

## Space

- **Freeverb** (Jezar at Dreampoint, public domain, 2000), from J. O. Smith, *Physical Audio Signal Processing*,
  "Freeverb" (dsprelated.com / CCRMA): eight lowpass-feedback combs, four Schroeder allpasses (gain 0.5), feedback
  `0.7 + 0.28 room`, damping `0.4 damp`, right channel +23 samples, tuning for 44.1 kHz. The comb and allpass
  lengths (1116 1188 1277 1356 1422 1491 1557 1617; 556 441 341 225) were confirmed in several ports' tuning lists.
  Written here from the equations, not copied.
- A tempo-synced ping-pong delay and tape wow are conventions (see `genres.md`, "a ping-pong 1/8-dotted delay on the
  lead"); their maths is simple enough to state in `src/effects.rs` and test directly.
