# Drums sampled from real drummers (2026-11-04)

The keeper, listening to the stream: the hats were "way too basic", and the generation should be "aware" and random,
not genres written by hand. The written drum modules (four kicks, three hats, two claps, two percs) were fixed patterns.
They are replaced, in the lens book, by grooves sampled from statistics learned from Magenta's **Groove MIDI Dataset**
(about 13 hours of drummers playing to a click, labelled by style; CC BY 4.0; credited in `CREDITS.md`).

## The pieces

| file | what it does |
|---|---|
| `tools/groove/learn.js` | reads every 4/4 performance, quantises it to 16 steps a bar, and learns per style group and instrument: P(hit at step i given the step before), velocity mean and sd per step, mean timing offset per step, bar-to-bar change rate; and per style the 5th to 95th percentile of real bars' hit counts and kick+snare syncopation. `node learn.js > model.json` (with `SPLIT=train` to learn from one split) |
| `src/compose/groove_model.txt` | that model as plain text (exported from `model.json`), read by the engine without a parser crate |
| `src/compose/groove.rs` | the generator, and `replace_drums`, which swaps the lens book's written kick, hat, clap and perc modules for 24 generated grooves when the book loads |
| `tools/groove/generate.js`, `tools/groove/eval.js` | the JavaScript prototype the Rust port was made from, and the style-awareness test below |
| `tests/groove.rs` | determinism, the band, every rendered hit against its table, the lens book, the families |

The dataset itself is not in the repository. `learn.js` expects it unpacked as `groove/` beside it:
<https://storage.googleapis.com/magentadata/datasets/groove/groove-v1.0.0-midionly.zip>.

## How a groove is made

- Each instrument walks the 16 steps with P(hit | the step before), learned per step and per style.
- A bar is kept only when every instrument's hit count and the kick+snare syncopation lie inside the 5th to 95th
  percentile of real bars of the style. That is rejection sampling, at most 400 tries, then the try closest to the
  medians.
- Bars 2 to 8 repeat bar 1, with each step re-drawn at the style's learned bar-to-bar change rate.
- Bars 4 and 8 end with two beats drawn from the style's fills.
- Time is kept by the hats or the ride, by how often the style's drummers play each.
- Timing is each step's learned mean offset with the style's overall lean removed: systematic, never random jitter.
  Senn et al. 2016 found exaggerated microtiming lowers groove (`docs/research/genres.md` 3.1).
- In the song language, a groove is a pair of 8-digit tables per bar (a 16-digit table that starts with a 9 can
  pass a double's exact range), an offset table, and `Gt(v, o)`, a hit `o` sixteenths from the grid.

Style groups learned: `citypop` (funk, soul, pop, disco/dance), `lofi` (hip hop), `rock`, `latin` (latin and
afro-cuban), `jazz` and `reggae`. The lens book uses `citypop`.

## Registered, then run: is it aware of style?

Registered before the run: a classifier built only from the dataset's **validation** split scores real bars from its
**test** split, and bars the generator makes from a model learned from its **train** split. The generator is
style-aware for a style if its bars are classified to that style no less often than real test bars are, minus 15
points (styles with at least 40 validation bars, judged where there are at least 30 real test bars).
`node tools/groove/eval.js`:

| style | real test bars | real classified own | generated classified own (200 seeds x 8 bars) | verdict |
|---|---|---|---|---|
| citypop | 904 | 64% | 62% | PASS |
| rock | 394 | 30% | 55% | PASS |
| latin | 303 | 84% | 28% | **FAIL** |
| jazz | 68 | 21% | 68% | PASS |

- **City pop**, the style the lens uses, passes cleanly.
- **Latin fails.** Its grooves interlock (clave against kick and hats), and this generator samples each instrument
  alone with one step of memory, so it cannot make that structure. Sampling the instruments jointly is the next design
  change, and any interlocking style will need it.
- **Rock and jazz** pass but read as more typical than real players: less varied than real drummers.
- **Lo-fi (hip hop) and reggae** were not judged: the validation split had too few bars of them.
- **Trap and house** are not in the dataset (they are programmed, not played), so this method cannot learn them.

## What changed in the stream because of it

`Composer::closure_parts` counted the novelty a song had spent as modules heard over half the book. Twenty-four
generated grooves per drum slot made the book about 85 modules bigger, so novelty could not fill and songs ended by the
clock: `every_song_ends_within_its_bounds_by_its_own_closure` fell to 3 of 10 ending by closure (the bar is 5). At the
keeper's choice (2026-11-04, option B), novelty now counts **families**: a slot's generated grooves are takes of one
idea (`groove::family`), and every written module is its own family. The test then gave 7 of 10.
