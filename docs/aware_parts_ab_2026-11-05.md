# Parts that read the structure: blind A/B (registered 2026-11-05 ~06:30, before listening)

The keeper, on the rating batch: clip 4 "good, but i wish the bass was more aware of itself and sticking to a single
tone sometimes when it needs to"; clip 6 "the high hat needs to some how have a different sound … could be better as
well with being aware of progression"; clip 7 "kicks are weird position". Then: *"but how do we make a function
understand itself"*, and *"do it"*.

The answer given: in this engine a part is aware of whatever its formula reads. Most parts read only `t`. These read
the chord, the chord changing, and the phrase.

## What was built (outside the repo; the engine and the book are unchanged)

Each pair is the same song, stages 1 to 3, differing only in the one part. The originals are the batch clips as rated.
The edit script is `aware/make.js` in the session scratchpad.

- **Clip 4 (chaos, seed 104389): the pedal bass.**
  - `Pp(i) = Hr(4 floor(i / 4))` is the root at the start of every two bars.
  - `Ph(p, i)` counts the chord tones of half bar i (the root and the three voiced notes) that share p's pitch class.
  - The bass plays `lw(Pp(i))` while `Ph(Pp(i), i) > 0`, and otherwise its old line. The rhythm is unchanged.
  - Counted over the clip: the pedal holds 39 of 48 half bars, 12 of them on a note that is not the chord's root, and
    every held half bar loses the old line's chaotic octave jumps.
- **Clip 6 (oneline, seed 518643): the hat.**
  - The hat was a `dash` drum, a 9 kHz pitch blip (9083 to 8908 Hz). It is now a noise hat, with the reference song's
    own settings (decay 44.66 ms, length 97.36 ms).
  - Added: an open hat (noise, decay 260 ms) that sounds once at each chord change,
    `(Hr(floor(t / 2)) != Hr(floor(t / 2) - 1))` on the half bar's first eighth. That is 20 changes in the clip.
  - **Confound, stated:** this is two changes, the sound the keeper asked for and the awareness. This pair cannot say
    which one did it.
- **Clip 7 is not built.** The planned fix, anchoring the kick to the downbeat, was checked first and is already true.
  `bounce/kick.half` hits beat 1 of every bar, plus the and of 3 in the second bar. Building it would have been an A/B
  of no change. What is weird about it is still open.

## The blind test

- **The assignment.** Each pair was matched in RMS to its quieter clip, and each was assigned to `A`/`B` by its own
  `crypto.randomInt(2)`, in `Desktop\Dreamride aware AB\clip4_A.wav` … `clip6_B.wav`.
- **The key** is outside the repo, sealed:

      sha256 daac263d36f1f2351bf6b14a5d46bc53cd3d9ddb30e85a1e4912098a6bf9732c

- **The question, per pair:** which one is better? "Same" is allowed.
- **Prediction:** the keeper picks the aware version in both pairs.
- **Falsified if** he picks the original, or hears no difference, in either pair. Each pair is scored on its own.
- **What a pass would not show:** that awareness in general is right. It is one listener, one song per part, and in
  clip 6 the sound and the awareness are not separated.

## Result

2026-11-05 ~06:20. The keeper, blind: *"clip 6 b is better. clip 4 a is better but still bad"*.

The key was re-checked before it was opened: `sha256 daac263d36f1f2351bf6b14a5d46bc53cd3d9ddb30e85a1e4912098a6bf9732c`,
identical. Contents: clip 4 A = aware, B = original; clip 6 A = aware, B = original; nonce `e27942a42043afcb`.

- **Clip 4: the prediction HELD.** He picked the pedal-aware bass, and added "still bad": better is not good.
- **Clip 6: the prediction is FALSIFIED.** He picked the original, the 9 kHz dash hat with no open hat, although he
  had asked for a different hat sound. The confound named in advance means this does not say whether the noise sound,
  the open hat at every chord change (20 in 35 s at 166 bpm), or both made it worse. Separating them takes a further
  pair: the original against the noise hat alone.
