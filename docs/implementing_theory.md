# From the research to the graph music: how each finding can live in this engine (2026-11-05 ~03:50)

The keeper: *"now lets take all of that research, and research how to implement it into our graph music"*. The
research is in `docs/what_makes_music_bad.md`. This note maps it onto the engine as it is: songs are functions of time
`t`, melodies are functions of a step index `k`, harmony is written digit tables (`harmony.txt`, read by `hd(N, i)`),
and a song is 8-bar stages chosen one after another by the composer.

Nothing here is built yet. Everything in it is a proposal, ranked at the end.

## What the engine does now, read from its own source

- **Melodies are fractals of a step number, read at every scale at once.**
  - `lead.blancmange`: `Lm(k) = hs(floor(1 + 3 T(k / 64) + 4 T(k / 16) + 3 T(k / 8) + 2 T(k / 4)), k, 4)`.
  - oneline: the ruler sequence `ru(m)` climbing the chord's ladder.

  Self-similar, so related at small scales. But because the slowest term reads `k / 64`, **no two bars of a stage
  are the same.** There is no motif: nothing is stated, then repeated, then developed.
- **Harmony is a written progression per stage**, as digit tables, one digit a half bar. Inside a stage the
  pitch-class set is limited. Across a song it changes whenever a move takes "a new progression". In the keeper's
  four saves from tonight, a new progression or a new melody arrives in 7 of 16 stage summaries (recounted from
  the saved files' `# Stage` lines: 410229 1, 497865 1, 23421 2, 598575 3).
- **Each stage is judged against the one before it** (audibility, the critic: roughness, syncopation, sound), and
  arcs are planned by layer count. **Nothing judges a stage against the song so far.** That is the gap the research
  points at (Dai, Zhang & Dannenberg 2022; MorpheuS: "may sound good over a short time span … lack long-term
  structure").

## The mechanisms, by scale

### Inside a stage: the phrase (8 bars = one Caplin phrase)

1. **Motif by periodicity.** In a function language, repetition is periodicity in the argument. Read the fractal on
   the position inside a 2-bar span, `mod(k, 16)` at eighths, instead of on `k`. The 2-bar idea then recurs. The slow
   terms that now make every bar new become **the development**: a transposition or a variant chosen per span. The
   graph character stays (the motif is still a blancmange); it gains a memory.
2. **Form as a digit table**, the way harmony already is. `Fo = 0 0 1 1 2 2 3 3`, one digit per bar, chooses each
   bar's treatment:
   - 0: the basic idea;
   - 1: the idea again, moved onto the next chord (`sc` already snaps to it);
   - 2: fragmentation, the first half of the idea twice (Caplin's continuation, which speeds the surface up);
   - 3: the cadence.

   A sentence is `0 0 1 1 2 2 3 3` read in half-bar pairs. A period is an antecedent ending on a weak close and a
   consequent ending on a strong one. Tymoczko's *harmonic consistency* comes free: every bar is a version of one
   idea. This is the motif-variant set the composition literature uses (repetition, transposition, inversion,
   fragmentation, expansion: MeloTrans 2024, Computoser), written as a table instead of code.
3. **The cadence as an override at the phrase end.** For the last beats of bar 8 (and bar 4 for a weak close), the
   lead takes the chord root or third on the downbeat and holds it (gate open, no glide). `if(…, cadence, line)` is
   already the language's idiom. Caplin: closure is melody, harmony and form arriving together. The progression
   tables decide the harmony, and this makes the melody agree with it.
4. **Range and return** (von Hippel & Huron). After a leap, prefer a step back toward the middle of the line's range.
   The `T` sums are bounded already, so this is a check more than a change: measure it before building anything.
   **Not for oneline's curve.** Its leaps are the streaming illusion, so this applies to its melodic stream (`ru` on
   `Hc`), not to the curve.

### Across stages: the song (the composer)

5. **A song memory, and a coherence term.** The composer keeps what the song has played: its lead intervals, its
   pitch-class weights, its modules. Each candidate stage gets three numbers, computed exactly by evaluating its
   functions:
   - **self-surprise:** mean information content of its lead under a small variable-order model (orders 1 to 3 over
     pitch intervals, blended) trained only on this song's earlier stages, a minimal IDyOM;
   - **centricity:** Krumhansl–Schmuckler clarity of the song's pitch classes with the candidate added;
   - **macroharmony:** the pitch classes it adds that the song has not used.

   **The target is a band, not a minimum.** Gold et al. found intermediate predictability is liked best, and Gold's
   own review found the inverted U in only 15 of 57 studies. A coherence term pushed to its extreme writes a loop that
   never changes, the opposite failure. So the band's edges must come from data: the keeper's good saves and the
   reference song, split so that the band is never set and tested on the same songs.
6. **A target tension curve instead of layer counts** (MorpheuS; Farbood 2012).
   - Per stage, tension from parameters the engine already has exactly:
     - onset density;
     - pitch height;
     - loudness (the audible profile);
     - roughness (the critic's own);
     - harmonic distance from the key, from the progression table.
   - The arc planner then chooses moves that follow a curve, for example rise to the peak, fall in the breakdown and
     resolve at the return, rather than counting layers.
   - Farbood's caveat: plain linear sums fit listeners badly, and her model needed a moving window and trend salience.
     Start with the curve's **direction** per stage (up, down, flat), not its height.
7. **Return as recall.** The composer has `homing` and a Return role. Make a return bring back the song's own
   material (its first motif, its first progression), as the coda already does for the reference song. "Return" then
   means the song remembering itself, not a different idea arriving loudly.
8. **Dialogue in every species** (version 3 is lens only). Oneline and dialogue are what the keeper hears most. The
   same literal listening, written per species.

## What to build first, and how each step is checked

1. **The measuring instrument** (`tools` or a test helper): the six registered measures plus tension direction, over
   any stretch or whole song.
   - It is needed before anything else can be shown to work.
   - It is checked on known cases: a stage that literally repeats has self-surprise near zero, and a chromatic random
     line has maximal macroharmony.
   - It runs on the good saves today. The B marks then test it against the registration.
2. **Song memory and the coherence band in the composer**, as generator version 4. It is aimed most directly at
   "unwinding", and it changes only which candidate stage is chosen, not the book. Its band comes from the good
   saves. It is tested by a blind A/B: the same seeds at versions 3 and 4, whole songs, and **whether the keeper marks
   fewer stretches bad**.
3. **Motif and form leads** (items 1 to 3): new book modules beside the old ones, so nothing that exists is changed.
   They get a blind A/B against the module each one replaces, as the phrased lead got on 2026-11-04 and won.
4. **Tension arcs** (item 6), after the instrument shows what tension does in good and bad stretches.

**What would show this map is wrong:** the B marks fail both registered families. Then the song-against-itself gap is
not what the keeper hears, and items 2 and 5 are built on the wrong diagnosis.

## Sources (beyond `docs/what_makes_music_bad.md`)

- Herremans & Chew (2018), *MorpheuS: generating structured music with constrained patterns and tension*,
  https://arxiv.org/abs/1812.04832 . Abstract read. The tension model's components are not named in it, and are not
  claimed here.
- Pachet & Roy, Markov constraints: https://research.gold.ac.uk/id/eprint/9519 ; meter in finite-length Markov
  sequences (AAAI 2013): https://ojs.aaai.org/index.php/AAAI/article/view/8661 . Strong on local constraints and weak
  on global ones by the authors' own account, which is why the coherence term here is a score on whole candidates and
  not a constraint on each note.
- Motif variants: MeloTrans, https://arxiv.org/pdf/2410.13419 ; Computoser, https://arxiv.org/pdf/1412.3079 ; Spiegel,
  *Manipulations of musical patterns*,
  https://continuum-hypothesis.com/music/spiegel_manipulations_of_musical_patterns.pdf
- Patterns as functions of time with transformations scheduled per cycle (`every`, `rev`, `fast`), the nearest living
  practice to this engine: Tidal Cycles, https://tidalcycles.org/docs/reference/workshop ; Strudel vs Tidal,
  https://patterns.slab.org/learn/strudel-vs-tidal
