# Instruments drawn per song (2026-11-04)

The keeper: "the instrumentation is also always the same, even if the songs are different". Every module carried one
fixed sound, so every lens song wore the same lead, bass and pads. `src/compose/timbre.rs` draws a song's own
instrument for each tonal role from its seed and keeps it for the song. Drums are untouched: they are generated
already (`docs/groove.md`).

## How

- **Roles.** Each role draws from its own ranges.
  - The bass has 4-8 harmonics on a steep slope, a filter at 280-950 Hz with an envelope, and a tight decay.
  - Pads have 10-16 soft harmonics, a 60-500 ms attack and 4-14 cents of detune.
  - The arp is a pluck.
  - A lead or counter is a pluck, a held lead or a bell (the counter darker by 2 dB an octave).
  - Every spectrum is a slope, an odd/even balance (saw-like to square-like) and one resonant bump.
- **Balance.** Each instrument's gain is rescaled so it is as loud as the written instrument it replaces: the summed
  harmonic power with the voice's rolloff, at the role's usual pitch.
- **References.** `timbre::choose` renders the full band, 16 beats at 16 kHz, with the drawn instruments, and scores
  its five-band energy against the two reference recordings (the critic's "sound bands"). A set more than 0.1 below
  the written instruments' score is refused and another is drawn, up to 6 times; otherwise the best is kept.
- **Scope.** Every book species takes them except the Explore tunings, whose sound is part of the tuning's idea. Only
  the text a stage plays is dressed; the stream composes from its undressed equation, so how it chooses moves is
  unchanged.

## Registered before the first run

**Over the Book seeds 1 to 20:**
1. at least 18 of 20 songs find a set within 0.1 of the written instruments' band score without falling back to the
   best of six;
2. the 20 songs' leads are 20 different instruments (no two equal), and they use at least two of the three lead
   kinds.

**Falsified if** fewer than 18 find a set within the margin. That would mean the role ranges themselves pull the mix
away from the reference balance, and the ranges, not the seeds, are wrong.

**What a pass would not show:** that the instruments sound good. The band score measures balance against two
recordings, not taste; that check is the keeper's ears.

## Second registration: the keeper heard no difference, and the hats tweet (2026-11-04, before the run)

The keeper, after the first version: *"i dont think i notice a difference for instrumentation"*, then *"the same
tweeting hi hats are getting so stale"*.

**Measured, not guessed.** Every module sets `rolloff = 660` (the graph calculator's cap). At a lead's usual pitch
(about 830 Hz) every harmonic is already above 660 Hz, so the cap flattens all of them alike. Over 8 songs, the drawn
leads' brightness was:
- 1.09 to 2.81 harmonics as drawn (the spectral centroid);
- **1.02 to 1.24 as heard**, against 1.03 to 1.17 for the written leads.

All 8 of those leads glide, so the gliding rule had made every one of them a held note. The hats are one sound in
every groove: a dash that falls 7600 to 5400 Hz in 5 ms, a 40% sweep, which is a chirp.

**Changed:**
- The rolloff is drawn per instrument: 660 about a third of the time, else 1500 to 6000 Hz.
- Held leads may open a filter at each note.
- Detune and drive are drawn more often.
- **A kit is drawn per song:**
  - kick: a dash with drawn punch, drop, body and length;
  - clap slot: a noise snare or a dash clap;
  - hat: the engine's noise hat or a metallic dash that sweeps at most 5%;
  - perc: an open noise hat or a metallic ring.

  Each drum's gain is matched to one hit's energy of the sound it replaces. The engine's noise hat and snare now take
  `decay` and `length` (their built-in values when absent, so every existing song sounds the same).

**Registered:**
1. Over Book seeds 1 to 20, the heard brightness of the songs' leads (centroid with the instrument's own rolloff and
   filter, at 830 Hz) spans at least 1.0 harmonic.
2. No drawn hat or open hat sweeps more than 5%, over 200 palettes. Over seeds 1 to 20, the hats are 20 different
   sounds and at least two kinds (noise and metal).
3. The first registration still holds with the kit in the mix check: at least 18 of 20 songs within 0.1 of the
   written band score.

**Falsified if** 1 or 3 fails. If 1 fails, the drawn range is still too narrow to hear. If 3 fails, the wider range
breaks the reference balance, and the two goals pull against each other.

**Second registration, result** (2026-11-04, `cargo test --release --test timbre -- --nocapture`):

| registered | result |
|---|---|
| leads' heard brightness spans at least 1.0 harmonic | **FAILED: 0.70** (from 0.22 before). Values: 1.02 to 1.72; half below 1.10 |
| no hat sweeps more than 5% (200 palettes) | held |
| 20 different hats, at least two kinds | held: 20 of 20, noise and metal |
| at least 18 of 20 within the reference margin, kit included | held: 20 of 20 |

A dressed drum hit also carries the written hit's energy: kick, hat and perc 0.00 dB, clap +0.12 dB.

**The falsifier fired on brightness: the drawn range is still too narrow to measure as different.** A first reading,
not yet checked: the pluck's filter cutoff (900 to 2600 Hz) sits just above a lead's usual pitch, so a pluck is dark
once its filter envelope has closed. Part of the gap is the measure. A pluck is bright at its onset, and a static
centroid cannot see that. The measure is not changed after seeing the result; a corrected one gets its own
registration.

## Result

2026-11-04, `cargo test --release --test timbre -- --nocapture`
(`registered_eighteen_of_twenty_songs_find_instruments_within_the_reference_margin`):

- **20 of 20 within the margin (registered: 18): it held.** 18 found theirs on the first draw and two on the second
  (seeds 1 and 15). There was no fallback.
- **The 20 leads are 20 different instruments, and all three kinds were drawn:** pluck, held and bell.
- **Most drawn sets score nearer the references than the written instruments do.** Seed 2: 0.431 written, 0.623 drawn.
  Seed 5: 0.349 against 0.596. That does not say the written sounds are worse. It says the references were never
  what they were tuned to.

**Wrong on the way, kept so the fix is legible.**

- The first loudness model counted harmonics and the rolloff only. Over 20 draws on one lead, dressed levels ran from
  -12.5 to +5.6 dB against the written one, failing the test's 9 dB bar.
- Adding the filter and the sustain narrowed it to -9.0 to +4.2, still failing.
- Printing the level by kind showed why: every bell was quiet (-1.4 to -9.0 dB). The test lead was
  `lead.blancmange`, a line that holds one gate open for four bars and glides, and a bell rings once and fades under
  that. That is a musical mismatch, not a calibration error.
- So the rule became aware of it: **a gliding line (its module tagged `glide`) keeps the drawn tone but holds its
  notes** (sustain at least 0.65, decay at least 300 ms).
- The test was rebuilt to measure each of 20 real book songs' own lead in its song's own instrument. Result: -3.1 to
  +4.4 dB, all within the bar.
