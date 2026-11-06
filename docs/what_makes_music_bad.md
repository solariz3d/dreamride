# What makes music bad: research, and a prediction registered before the first bad mark (2026-11-05)

The keeper, 2026-11-05: *"the whole point is to not have any bad rolls … all should be good"*, and *"just look up and
do research of what makes music bad, that is a good signal"*. What he hears going wrong: sections that *"lose its
coherence and begin to unwind in ways that just sound bad"*.

The B key (world app) now marks a stretch as bad, into `judged\bad\`. S already saves the good ones into `examples\`.
This note says what the research predicts will separate the two, before any bad stretch exists to look at.

## What the research says (sources below; how strong each one is is stated)

1. **Pleasure peaks at intermediate predictability, and it depends on context.**
   - Gold et al. 2019 (J. Neurosci.) measured each note's information content (surprise, −log₂ P(note | what came
     before)) and the entropy (uncertainty) of what might come next, using the IDyOM expectation model. Listeners
     liked intermediate values best.
   - The two interacted: when the context was more uncertain, listeners wanted more predictable events.
   - Cheung et al. 2019 (Current Biology), 80,000 chords from 745 Billboard songs: pleasure came from a surprise
     after a confident expectation, or from a confirmation after an uncertain one. It did not come from either
     extreme alone.
   - **Caveat, from Gold et al. themselves:** of 57 studies, only 15 found the inverted-U (Wundt) curve. It is a real
     pattern, not a law.
2. **Songs repeat themselves, and generated music does not.**
   - Dai, Zhang & Dannenberg 2022, "What is missing in deep music generation?":
     - human pop songs have multi-level structure;
     - they reuse a small vocabulary of their own, not the statistics of music in general;
     - their repetition follows a trend over the song (measured by cross-entropy);
     - generated music shows "striking differences" in exactly this.
   - Margulis (Music Perception 2016, and *On Repeat*): repeating a random tone sequence makes listeners rate it as
     more musical. The authors call the effect small. Related work (looped environmental sounds; a rondo study) points
     the same way, but I read it only in press summaries.
   - Wu & Yang's structureness indicator and inter-phrase similarity are the usual measures of this.
3. **Groove peaks at medium syncopation.** Witek et al. 2014 (PLoS ONE), funk drum breaks: medium syncopation made
   listeners most want to move and gave them the most pleasure. Too little and too much both lost it. Entropy of the
   audio did not predict this; syncopation did.
4. **Sensory roughness: partials beating within a critical band** (Plomp & Levelt; Sethares's formula). Two partials
   at f₁ < f₂ with amplitudes a₁, a₂ give roughness a₁a₂(e^(−3.5·s·Δf) − e^(−5.75·s·Δf)), where s scales Δf to the
   critical band. The roughness of a whole mix is the sum over all pairs. Measured from audio this needs peak-picking.
   **This engine knows every partial exactly**, from each voice's `partials` and its pitch at each moment, so here
   roughness can be computed exactly.

**Correction, same night (~03:45):** two of these four are already in the engine. `src/compose/critic.rs` computes
Sethares roughness from the exact partials (`pair_roughness`, `chord_roughness`) and Longuet-Higgins–Lee syncopation
per bar (`lhl_per_bar`). Both feed every stage's critic score, which picks among candidates. This note first described
roughness as if it were new. It is not. That also changes what to expect: a property the composer already selects for
is less likely to separate bad from good. Measure 2 below uses the critic's own `chord_roughness`, unchanged. What
nothing in the engine measures is the song against **itself**: self-surprise, repetition of its own bars, its pitch
classes and its centre over time, and whether a phrase closes.

## What this means for Dreamride's "unwinding"

The keeper's word, *unwind*, fits finding 2 more closely than any other: a song that stops sounding like itself. The
composer changes modules, progressions and species as it goes, and each stage is judged against the one before it
(audibility, the critic). No stage is judged against **the song's own vocabulary so far**. A stage can be audible,
score well, and still belong to a different song.

## Registered prediction (2026-11-05 ~03:30, before any stretch has been marked bad)

When at least 8 stretches have been marked bad (B) and compared with the keeper's good saves (S, version 3 and
earlier):

1. **Self-surprise.** The lead's notes in bad stretches are more surprising **given the song's own earlier stages**
   than in good stretches. Measure: mean information content of each lead note under a variable-order pitch-interval
   model trained only on that song's preceding stages, in bits. Bad median above good median.
2. **Roughness.** Bad stretches are rougher: Sethares roughness of the dressed mix per 16th, from the exact partials,
   with the median over the stretch. Bad median above good median.
3. **Repetition.** Bad stretches repeat less: the share of bars in the stretch whose lead and bass match a bar heard
   earlier in the song. Bad median below good median.

**How each measure is computed** (written 2026-11-05 ~04:15, before the instrument has run on any stretch and before
any bad mark exists; `src/compose/measure.rs`):
- **The stretch and its history.** A judged stretch (S or B) is regenerated from its seed file, at its own version,
  from stage 1 to its last stage. Everything before its first stage is history. A saved file alone has no history,
  which is why B saves the seed too.
- **Lead notes.** The critic's events of the voice named `lead` (onsets, and settled new pitches inside held notes).
- **Self-surprise.** Intervals between consecutive lead notes, rounded and clamped to ±24 semitones. The model blends
  orders 0 to 3: each order's estimate is (count + 0.5) / (context count + 0.5 · 49), and each is weighted by its
  context count, with order 0 always weighted 1. It is updated after every note, history included. The measure is the
  mean −log₂ P over the stretch's notes.
  **Corrected ~04:25, before any real stretch was measured.** The smoothing first written here,
  (count + 0.5) / (context count + 0.5 · 49), adds 24.5 pseudo-counts per context. The known-case test
  (`tests/measure.rs`) showed what that does: after twenty identical intervals the model gave that interval only 0.44,
  and a loop heard a hundred times still scored 1.5 bits. A model that cannot learn a loop cannot measure a song
  against itself. The estimate is now (count + 1/49) / (context count + 1): one pseudo-count per context, spread over
  the 49 intervals. The blending, the orders and everything else are unchanged.
- **Roughness.** The critic's own procedure (tonal, non-percussive voices; the critic's `chord_roughness` of their
  exact spectra), sampled every 16th, with the median over the stretch.
- **Repetition.** A bar's signature is the lead's and the bass's rounded pitch at each 16th, or a rest; a missing
  voice counts as rests. It is the share of the stretch's bars whose signature equals that of any earlier bar of the
  song, history included.
- **Macroharmony and centricity.** Windows of two stages, stepping by one stage, inside the stretch. Pitch-class
  weight is gain × amp, summed over the open 16ths of the tonal, non-percussive voices. Macroharmony counts the pitch
  classes with at least 5% of the weight. Centricity is the best over the second-best Pearson correlation with the 24
  rotations of the Krumhansl–Kessler major and minor profiles. Both are the median over the windows.
- **Conjunct.** The share of consecutive lead-note moves of at most 2 semitones.
- **AUC.** The probability that a random bad stretch is further in the predicted direction than a random good one,
  with ties counting half.
- **Known weakness, stated before the run:** in oneline the lead is the four-register curve, so its moves are mostly
  leaps by design, in good and bad stretches alike. That makes conjunct motion noisier there, not biased.

**Amended ~04:35, after the first bad marks were seen and before anything was scored.** The keeper's first B marks
were four overlapping stretches of one song (automaton, seed 85749, marked at stages 2, 3, 4 and 5; *"a legendary
horrible pull on first go"*). Two defects showed up, and neither change touches any measure's formula, direction or
threshold:
- **Silence counted as repetition.** In an opening with no lead and no bass, every bar was all rests, so every bar
  "repeated" (0.94). A bar where neither voice sounds is now left out. If no bar sounds, the measure is NaN. Test:
  `silence_is_not_repetition`, which fails on the old code.
- **Stretches are not independent judgments.** One song marked four times would count as four. The registration is
  now scored per **song**: each song counts once, as the median of its stretches, on both sides, with at least 8 songs
  on each side. On the good side, seed 885243 has three saved stretches.

Asked what made seed 85749 horrible (rhythm, chords or sound?), the keeper: *"it just doesnt sound good to me or
musically correct in a way that is fullfilling"*. Recorded as said. It is not used in scoring.

Second bad song (~05:12): bounce, seed 187301, marked at stage 1 (the lead alone) and at stage 3. The keeper,
before anything was said about it: *"you will think its the inherent family of seed or function, but its more the
values that then form within the species, not the species itself"*. The record agrees, and the comparison is
uncurated:
- **Species.** Seed 152637, the keeper's first liked save, is automaton too, the same species as bad seed 85749.
- **Progression.** Bad seed 187301 (bounce) opens on the same progression as good seed 152637,
  `Hb = 1166442200553322`, `Hm = 7`.

So neither the species nor the progression alone is bad. What differs is what forms: which layers open, which modules
meet which progression, and what the pitch does. In 85749 that is pads and hat on the quintuplet grid with no melody,
over the chromatic lament. In 187301 it is a lead sprung two semitones sharp on every note, then a bass that slides
seven semitones and back every bar. **Consequence for the floor:** it cannot ban species or modules. It has to judge
what a candidate sounds like, which is what the composer change (version 4) proposes and what these measures read.

**The rating batch (~05:35, at the keeper's "ok").** The marks were arriving only from songs the keeper happened to
stop on. So:
- **The sample.** 16 species seeds drawn by `crypto.randomInt(1, 1000000)`, at version 3. Each clip is stages 1 to 3,
  exactly as S or B would save them (`ab/src/bin/batch.rs` in the session scratchpad).
- **The clips.** Rendered by `luminesce-render` and matched in RMS to the quietest (−20.76 dBFS, only ever scaled
  down). Shuffled by crypto into `Downloads\dreamride_ab\batch\clip_01.wav` … `clip_16.wav`, with no species or seed
  in any name.
- **The key** is outside the repo, sealed before any listening:

      sha256 a116b7d70be64bf1061b5f9fc6474f5ba553c85d9df972fb0ec6a079ae4ff709

- **How the ratings are used.** The keeper rates each clip good, bad or skip. Each rated clip joins its side, one song
  each. The measures are not run on the batch until the ratings are in.

**The keeper's ratings (~05:39), recorded verbatim before the key was opened:**
1. "trash"
2. "trash"
3. "could have went hard but was just garbo"
4. "good, but i wish the bass was more aware of itself and sticking to a single tone sometimes when it needs to for a
   moment"
5. "good until 35 seconds when the pitch shifts down into an ugly phase of tone"
6. "i like it, but the high hat needs to some how have a different sound … could be better as well with being aware of
   progression"
7. "kicks are weird position, the rest is interesting, but wouldnt say good at all, same problem as the rest"

Clips 8 to 16 were not rated. Read by the rule set before rating (only clear good and clear bad count):
- **bad:** 1, 2, 3, 7. Clip 7 is "wouldnt say good at all, same problem as the rest", read as bad. That is a judgment
  call, open to the keeper's correction.
- **good:** 4, 6.
- **left out:** 5, which is good and then bad inside one clip.

### A third hypothesis, from clip 5, registered ~05:50 before it was checked on any other song

Key opened (`sha256 a116b7d7…`, matching):
- clip 1 = oneline 235665
- clip 2 = dialogue 707207
- clip 3 = chaos 546123
- clip 4 = chaos 104389
- clip 5 = chaos 344178
- clip 6 = oneline 518643
- clip 7 = bounce 160892

Species split again across good and bad (chaos 3 bad and 4 good; oneline 1 bad and 6 good).

**Clip 5.** "Good until 35 seconds when the pitch shifts down into an ugly phase". At 111 bpm stage 3 begins at 34.6 s,
where the stage summary reads "a new progression, chaos/prog.minor_climb". The best Krumhansl–Kessler key of each
stage alone was:
- stage 1: G♯ minor, 0.81;
- stage 2: E major, 0.82;
- stage 3: **E minor, 0.61**.

That is the parallel minor, three scale degrees down a semitone at once, and a much weaker fit. Registered measure 5
(centricity, best over second-best) did not see it: 1.037 to 1.028. The two good clips moved only to closely related
keys (4: B♭ to F, a fifth; 6: A♭ minor to B major, the relative major), and their fit rose.

**Hypothesis K (from clip 5 alone, so clip 5 is not evidence for it).**
- **Each stage's key** is its best Krumhansl–Kessler key over the stage alone (pitch classes as in measure 4).
- **Key distance** between consecutive stages inside a judged stretch is the distance on the circle of fifths between
  their key signatures, with a minor key taking its relative major's signature: E major (4♯) to E minor (1♯) is 3.
- **A song fails** if any consecutive stages in its judged stretches are 2 or more apart, or if any stage's best fit
  is below 0.65.
- **Prediction:** among the other rated songs (bad: 85749, 187301 and batch clips 1, 2, 3, 7; good: batch clips 4 and
  6 and the S saves), the share of bad songs that fail is at least 0.5, and the share of good songs that fail is at
  most 0.25.
- **Falsified if** bad songs fail no more often than good songs do.
- **Small numbers, stated in advance:** about 6 bad and 9 good songs. A pass would justify registering the rule for
  new marks; it would not establish it.

**Result (~05:55): hypothesis K is FALSIFIED.** It was run exactly as registered (`ab/src/bin/keyrule.rs`):
- **Bad songs failed 2 of 6 (0.33):** 85749 and 187301. Clips 1, 2, 3 and 7 passed.
- **Good songs failed 7 of 9 (0.78):** only 104389 and 191747 passed.

Liked songs make the same parallel-mode moves that clip 5 made:
- 23421: B minor to B major;
- 497865: E minor, A major, A minor, E minor;
- 598575: F♯ minor, B major, G major.

The fit threshold has a defect of its own: a stage with drums only has no pitch classes and a fit of 0 (152637
stage 1). With key distance alone it is still falsified: bad 2 of 6, good 4 of 9.
- **What this rules out:** "a move to a distant key" is not what the keeper hears as bad. Whatever made clip 5 ugly
  at 34.6 s is specific to that move, not a property of key changes.
- **What it does not rule out:** the registered measures. They have not been scored.

**Amended 2026-11-06 ~03:25, before anything was scored.** New marks arrived on the 6th:
- good: 647878 and 239;
- bad: 352849, 456771, 81735, 990347, and **239 stages 8 to 11**.

Seed 239 is the first song on both sides: saved good at 1–4, 4–7 and 6–9, marked bad at 8–11. The keeper: *"the last
good one was the best one, and even better, there was a stage of it where it went bad that I also marked"*.

The registration did not foresee a song on both sides. The rule, set before scoring: a song counts once on each side
it was judged on, with only the stretches judged that way. The batch 1 ratings join their sides (bad: clips 1, 2, 3
and 7; good: 4 and 6), copied from `research/2026-11-05/batch1/` into `judged/bad/` and `judged/good/`.
`luminesce-measure` reads `judged/good/` as well as `examples/`. Nothing else changes: the measures, directions,
thresholds and families are as registered.

### SCORED 2026-11-06 ~03:28: neither family holds; neither is falsified

`cargo run --release --bin luminesce-measure -- .`: 16 good stretches from 11 songs, 15 bad stretches from 11 songs
(five older saves skipped, because no version reproduces them as saved). AUC, the probability that a bad song lies
further in the registered direction than a good one:

| measure | direction | AUC |
|---|---|---|
| self-surprise | bad higher | 0.636 |
| roughness | bad higher | 0.345 |
| repetition | bad lower | 0.616 |
| macroharmony | bad higher | **0.814** |
| centricity | bad lower | 0.372 |
| conjunct | bad lower | 0.368 |

- **First family: 0 of 3 at AUC ≥ 0.70.** Self-surprise and repetition are at least 0.60, so it is not falsified. It
  does not hold.
- **Theory family: 1 of 3** (macroharmony). Not falsified, does not hold.
- **Read with the warning registered before the run.** Conjunct motion was named in advance as noisy for oneline,
  whose lead leaps by design. Oneline is 5 of the 11 good songs, so "bad songs move more by step" mostly says "bad
  songs are more often not oneline". The three measures below 0.40 are not evidence of the opposite rule: they were not
  predicted, and a species confound explains at least one of them.
- **Seed 239, the same song on both sides.** Good stretches 6, 7 and 6 pitch classes; the stretch that went bad, 9.
  Self-surprise 3.36 against 1.30, 1.87 and 3.15.

**New hypothesis M, registered now, from this scoring (so this data is not evidence for it).** Bad stretches use more
pitch classes than good ones. The measure is macroharmony as defined above, scored on judgments made after
2026-11-06 03:28 only.
- **Holds** if AUC ≥ 0.70 over at least 6 new bad and 6 new good songs.
- **Falsified** if AUC < 0.60.

**The prediction holds** if at least two of the three separate bad from good in the stated direction, each with
bad-over-good AUC at least 0.70.

**Falsified if** none of the three separates them (every AUC below 0.60). Then the keeper is hearing something these
three measures do not capture, and the next step is to ask him what, not to add measures until one fits.

**Abuse condition:** no measure may be added, reversed or re-thresholded after the bad marks have been seen and still
count as part of this registration. Anything found afterwards is a new hypothesis and needs new marks to test it.

**What a pass would not show:** that rejecting such stages makes songs good. It would show that the measure sees what
he hears. A floor built on it needs its own blind test.

## Music theory: what it adds (2026-11-05 ~03:40, also before any bad mark)

The perceptual research says *what* listeners like. Music theory names the structure that delivers it, and most of
it can be measured in this engine directly, because every voice is a function the engine can evaluate.

5. **Tymoczko's five components of tonality** (*A Geometry of Music*, 2011). Tonal music shows:
   - conjunct melodic motion (mostly steps);
   - acoustic consonance (consonant sonorities, especially at points of stability);
   - harmonic consistency (the harmonies in a passage are structurally alike);
   - **limited macroharmony** (a small set of pitch classes over a span of a few seconds);
   - **centricity** (one pitch class heard as home).

   He calls three of them (conjunct motion, limited macroharmony, centricity) common to virtually all human music.
   *Unwinding* fits the last two: a stretch that keeps adding pitch classes and loses its centre stops sounding like
   one piece.
6. **Key clarity over time (Krumhansl–Schmuckler).**
   - Correlate the pitch-class distribution with the 24 major and minor key profiles (Krumhansl & Kessler's
     listener ratings).
   - The best correlation is the key strength. The ratio of the best two correlations is tonal clarity (Müllensiefen).
   - Rolled over a window, it shows whether a song holds a centre.
   - Caveats: the window length changes how well it matches listeners a great deal (MTG–UPF), and untrained listeners
     track it less well (Frankland & Cohen 1996).
7. **Melodic shape, as it actually is in corpora.**
   - Huron and von Hippel: melodies regress to the middle of their range. A leap is usually followed by a turn back,
     mostly because leaps reach the edges of the range.
   - A line that keeps leaping without returning, or that drifts out of its own range, breaks the most robust melodic
     statistic there is.
8. **Phrase and cadence (Caplin).**
   - A sentence: a basic idea, its repetition, then a continuation that fragments and accelerates into a cadence.
   - A period: an antecedent ending on a weaker cadence, answered by a consequent that closes.
   - Both are usually 4 or 8 bars.
   - Caplin's *Cadence* (2024): closure is made by melody, harmony and form together, not by the sound stopping.
   - A stage of this engine is 8 bars. Nothing in it is required to close the phrase it opens.
9. **Tension over time (Farbood 2012, Music Perception 29(4)).**
   - Perceived tension follows harmony, pitch height, melodic expectation, loudness, onset frequency, tempo and
     syncopation.
   - A model with a moving window and *trend salience* matched continuous listener ratings, where plain linear models
     did not.
   - The composer already plans arcs (intro, build, peak, strip, turn, return), but by layer count, not by measured
     tension.
10. **Counterpoint** (Fux onwards; standard). Independent voices avoid parallel perfect fifths and octaves, avoid
    crossing, and keep their spacing. Version 3's imitation an octave below can produce parallel octaves by design.
    Whether that sounds like listening or like doubling is something the dialogue A/B can tell.

### Registered: the theory family (before any bad mark)

Measured over each stretch, from the played song evaluated by the engine:

4. **Macroharmony.** The number of pitch classes that carry at least 5% of the sounding (gate-open, gain-weighted)
   time of the tonal voices, over each 2-stage window. Bad median above good median.
5. **Centricity.** The Krumhansl–Schmuckler tonal clarity (best correlation over second-best) of the same windows.
   Bad median below good median.
6. **Conjunct motion.** The share of lead note-to-note moves of at most 2 semitones. Bad median below good median.

This family **holds** if at least two of the three separate bad from good in the stated direction with AUC at least
0.70. It is **falsified** if all three are below 0.60. The first family's bar and the abuse condition above apply
unchanged, and the families are scored separately, never pooled to rescue one another.

## Sources

- Gold, Pearce, Mas-Herrero, Dagher & Zatorre (2019), *Predictability and Uncertainty in the Pleasure of Music: A
  Reward for Learning?*, J. Neurosci. https://pmc.ncbi.nlm.nih.gov/articles/PMC6867811
- Cheung et al. (2019), *Uncertainty and Surprise Jointly Predict Musical Pleasure
  …*, Current Biology. https://www.sciencedaily.com/releases/2019/11/191107111744.htm (press summary; the paper was not
  read in full)
- Dai, Zhang & Dannenberg (2022), *What is missing in deep music generation? A study of repetition and structure in
  popular music*. https://arxiv.org/abs/2209.00182 (abstract read; the full text would not convert)
- Margulis & Simchy-Gross (2016), repetition and the musicality of random tone sequences.
  https://sonicfield.org/library/repetition-enhances-the-musicality-of-randomly-generated-tone-sequences ; Margulis,
  Aeon essay: https://aeon.co/essays/why-repetition-can-turn-almost-anything-into-music
- Witek, Clarke, Wallentin, Kringelbach & Vuust (2014), *Syncopation, body-movement and pleasure in groove music*, PLoS
  ONE 9(4) e94446. https://doi.org/10.1371/journal.pone.0094446
- Sethares's roughness model, as implemented in MIRtoolbox (`mirroughness.m`) and a Python port:
  https://gist.github.com/3066664 . The constants 3.5 and 5.75 are from those sources, and the critical-band scale s
  must be checked against *Tuning, Timbre, Spectrum, Scale* before use.
- Survey of music-generation metrics (2025): https://arxiv.org/pdf/2509.00051
- Tymoczko (2011), *A Geometry of Music*. Summary used: https://en.wikipedia.org/wiki/Dmitri_Tymoczko ; the five
  components were checked against search excerpts of the book, not the book itself.
- Key finding and tonal clarity: https://arxiv.org/pdf/2608.19061 ; window effects: https://mtg.upf.edu/node/2500
- Von Hippel & Huron (2000), skips and regression to the mean:
  https://labs.sonicfield.org/library/why-do-skips-precede-reversals-the-effect-of-tessitura-on-melodic-structure ;
  https://shanahdt.github.io/mus-345-445/emp_methods_workshop/regression_in_music.html
- Caplin, the sentence and the period: https://pressbooks.nebraska.edu/openmusictheory/?p=296 ; *Cadence* (2024),
  as reviewed in https://storage.gmth.de/zgmth/pdf/1237
- Farbood (2012), *A Parametric, Temporal Model of Musical Tension*:
  https://bpb-us-e1.wpmucdn.com/wp.nyu.edu/dist/f/11865/files/2020/08/Farbood_2012_Musical_Tension.pdf (abstract and
  summaries; the full model weights were not read)
- Not found: Herremans & Chew's tension ribbons. The search returned only physics papers, so they are not cited.
