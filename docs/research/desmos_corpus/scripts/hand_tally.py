"""Hand tally: techniques I verified by reading each graph's sound expressions (not regex).
One row per graph family (the three DIXERION songs share one engine and count once).
Prints the count per technique and which graphs, sorted by count."""
HAND = {
 'jbfiikq4mx (+notnmptv6n, rxm7hwvl0o) Better Off Alone / Spectre / Astronomia, r/desmos 259':
   ['step lists', 'separate gate list', 'additive harmonics', 'spectral tilt or filter on partials', 'per-note filter sweep',
    'kick = falling pitch', 'sidechain ducking', 'mod-cycled loop', 'glide/slide'],
 'bqrffiubis Overtime 2.0, YouTube 7.9k':
   ['step lists', 'note names', 'additive harmonics', 'decay envelope per note', 'kick = falling pitch', 'noise drums (random partials)',
    'echo copy', 'glide/slide', 'sections gate layers', 'mod-cycled loop', 'ticker clock', 'rest as undefined', 'section transposition'],
 'fuyhyjnedt SzefRon, YouTube 121k':
   ['step lists', 'arpeggio indexes a chord table', 'additive harmonics', 'decay envelope per note', 'kick = falling pitch',
    'snare = falling pitch', 'vibrato', 'sections gate layers', 'mod-cycled loop'],
 'mbokvmxbzr Music Engine w/ Drums, r/desmos 241':
   ['event list', 'decay envelope per note', 'additive harmonics', 'spectral tilt or filter on partials', 'kick = falling pitch',
    'noise drums (random partials)', 'drum hit-time lists'],
 '8kxhmzypm8 Mini Desmos Song, art contest':
   ['event list', 'decay envelope per note', 'additive harmonics', 'spectral tilt or filter on partials', 'noise drums (random partials)',
    'ticker clock', 'chords as stacked voices'],
 'a9jlvp2yl8 Transmission, challenge':
   ['event list', 'decay envelope per note', 'additive harmonics', 'spectral tilt or filter on partials', 'noise drums (random partials)',
    'echo copy', 'tempo change', 'sections gate layers', 'glide/slide', 'drum hit-time lists'],
 '3mcvjtyd4q A Mathematical Ballad, challenge':
   ['step lists', 'arpeggio indexes a chord table', 'chord = root + intervals', 'additive harmonics', 'sections gate layers',
    'rest as undefined', 'ticker clock', 'mod-cycled loop'],
 '2sil5zcnun TETRIS, challenge':
   ['step lists', 'additive harmonics', 'ticker clock', 'mod-cycled loop'],
 '34gdqnbihn Bach Cello Suite, challenge':
   ['step lists', 'note names', 'ticker clock'],
 'e2iix1hhkk silent road, challenge':
   ['step lists', 'chords as stacked voices', 'additive harmonics', 'decay envelope per note', 'glide/slide', 'sections gate layers',
    'mod-cycled loop', 'noise drums (random partials)'],
 'gaowmi4hi0 supersaw sequencer, challenge':
   ['step lists', 'chords as stacked voices', 'additive harmonics', 'spectral tilt or filter on partials', 'detune/unison',
    'mod-cycled loop', 'ticker clock', 'separate gate list'],
 'vcldbih0wa family mart, challenge':
   ['step lists', 'separate gate list', 'additive harmonics', 'mod-cycled loop', 'chords as stacked voices'],
 'agcq6pwjzr snake eyes, challenge':
   ['step lists', 'separate gate list', 'additive harmonics', 'glide/slide', 'mod-cycled loop'],
 'lonntptn3d Never Gonna Give You Up, challenge':
   ['event list', 'decay envelope per note', 'additive harmonics', 'spectral tilt or filter on partials', 'inharmonic partials'],
 'idy42pofff Lead Melody, challenge':
   ['step lists', 'additive harmonics', 'spectral tilt or filter on partials', 'detune/unison'],
 'qeth5bpf0u Bunnies, challenge':
   ['event list', 'chord = root + intervals', 'sections gate layers', 'ticker clock'],
 'sjdmjwxcsf Giant Steps, challenge':
   ['step lists', 'chords as stacked voices', 'ticker clock'],
 'wcq51fhx0m Looping The Rooms, challenge':
   ['step lists', 'chords as stacked voices', 'ticker clock', 'variable step duration'],
 '4tca7a3nay Polyrhythms, r/desmos 193':
   ['ratio rhythm = harmonic-series pitch', 'spectral tilt or filter on partials', 'ticker clock'],
 'w07779vzrr polyrhythms, challenge':
   ['ratio rhythm = harmonic-series pitch', 'ticker clock'],
 'y5tldfmcxx Beatuds, Berrynote':
   ['chords as stacked voices', 'additive harmonics', 'spectral tilt or filter on partials', 'detune/unison'],
 '7ielw63oyk Blasphemian cover, r/desmos 37':
   ['continuous pitch curve', 'glide/slide', 'vibrato', 'additive harmonics', 'ticker clock'],
 'petadzc4sb Song (original), r/desmos 23':
   ['continuous pitch curve', 'additive harmonics', 'spectral tilt or filter on partials', 'mod-cycled loop', 'chords as stacked voices'],
 '8vvg5m9zfr Mountain of Faith, r/desmos 30':
   ['step lists', 'mod-cycled loop', 'intro then loop', 'ticker clock'],
 'cyd1baqnyg We\'ll Be Fine, r/desmos 20':
   ['event list', 'decay envelope per note'],
 'd60jdhrsl0 Tetris?, r/desmos 62':
   ['step lists', 'ticker clock'],
 '3bwlb8y8di amen break, forum':
   ['drum hit-time lists', 'noise drums (random partials)', 'decay envelope per note'],
}
from collections import Counter
c = Counter(t for v in HAND.values() for t in v)
print('graphs (families):', len(HAND))
for t, n in c.most_common():
    print(f'{n:3d}  {t}')
