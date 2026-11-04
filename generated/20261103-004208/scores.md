# Composer run, seed 7

200 candidates written and scored from their functions; 200 rendered and given a sound score; 5 kept. 112.5 s on 16 threads.

Scores are 0..1. The critic, its components, weights and citations are in `docs/composer.md`. The function score leaves out the sound (which needs a render); the total includes it.

Function scores over all candidates: min 0.58, median 0.77, 80th percentile 0.82, max 0.94.

## The kept songs

| rank | song | family | total | function | melody range | melody steps | melody contour | melody repetition | rhythm syncopation | rhythm density | harmony roughness | harmony roots | structure contrast | structure return | structure energy | sound bands | sound peaks |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | s7-c60 | squares | 0.90 | 0.94 | 0.80 | 0.93 | 1.00 | 1.00 | 1.00 | 1.00 | 0.91 | 0.66 | 1.00 | 1.00 | 1.00 | 0.66 | 0.70 |
| 2 | s7-c62 | fibonacci | 0.87 | 0.89 | 1.00 | 0.86 | 0.78 | 0.95 | 1.00 | 0.70 | 0.90 | 0.73 | 1.00 | 0.91 | 0.89 | 0.85 | 0.74 |
| 3 | s7-c178 | squares | 0.85 | 0.87 | 1.00 | 0.00 | 1.00 | 1.00 | 1.00 | 1.00 | 0.78 | 0.66 | 1.00 | 1.00 | 1.00 | 0.93 | 0.57 |
| 4 | s7-c174 | fractal | 0.84 | 0.91 | 1.00 | 0.96 | 0.93 | 0.90 | 1.00 | 1.00 | 0.93 | 0.69 | 1.00 | 0.72 | 0.82 | 0.65 | 0.16 |
| 5 | s7-c38 | squares | 0.84 | 0.89 | 1.00 | 1.00 | 1.00 | 0.96 | 1.00 | 1.00 | 0.78 | 0.67 | 1.00 | 0.87 | 0.59 | 0.61 | 0.41 |

## The hand-written songs, scored the same way

| | song | | total | function | melody range | melody steps | melody contour | melody repetition | rhythm syncopation | rhythm density | harmony roughness | harmony roots | structure contrast | structure return | structure energy | sound bands | sound peaks |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| | graph_takagi | hand | 0.78 | 0.75 | 1.00 | 0.34 | 1.00 | 1.00 | 0.89 | 0.61 | 0.99 | 0.69 | 0.91 | 0.29 | 0.22 | 0.97 | 1.00 |
| | graph_lissajous | hand | 0.72 | 0.68 | 0.78 | 0.80 | 0.50 | 0.16 | 1.00 | 1.00 | 0.64 | 0.74 | 0.58 | 0.00 | 1.00 | 0.98 | 1.00 |
| | graph_pisano | hand | 0.78 | 0.76 | 1.00 | 1.00 | 1.00 | 0.86 | 1.00 | 1.00 | 0.73 | 0.66 | 0.66 | 0.33 | 0.00 | 0.97 | 0.87 |

Where each hand-written song ranks among this run's random grammar samples (the fraction of samples it scores above):

- **graph_takagi**: above 44% by function score, above 76% by total (every sample rendered)
- **graph_lissajous**: above 10% by function score, above 39% by total (every sample rendered)
- **graph_pisano**: above 46% by function score, above 76% by total (every sample rendered)

## Why each was kept

### 1. s7-c60 (total 0.90)

Strongest: melody contour 1.00 (measured 1.00); melody repetition 1.00 (measured 0.53); rhythm syncopation 1.00 (measured 6.00). Weakest: sound bands 0.66 (measured 28.03); harmony roots 0.66 (measured 0.66).

Composed by luminesce-compose, seed 7 candidate 60: a squares song, grown from one generator through a scale lens.  sq(v) is a square wave, 0 for the first half of each cycle and 1 for the second: sums of them at time scales an octave apart are binary counting, read as music (Walsh functions). The chord roots are two square waves 4 and 8 bars long. The melody is the root plus 4 square waves: 1 x period 4, -2 x period 16, 1 x period 64, 3 x period 32. The lens h is the Ionian mode throughout. The lead is h(M) on every eighth, held legato and gliding into each step. A harmony doubles the lead a diatonic fifth above. 3 pads stack thirds on the root, struck each bar. An arpeggio walks the chord's tones by the generator, every sixteenth. The bass walks the chord's root, third and fifth by the generator, one a beat. The kick is drawn from the generator's own sequence. The clap is a pitch line dashing down on 2 and 4. The hats are eighths, their loudness read from the generator. Form: the hand songs' shape: all in, the lead from bar 5. 7 definitions and 7 time names.

Parameters: family = squares; bpm = 104; key = 1; lens = Fixed(5); lead steps per beat = 2; legato = true; glide = Some(0.75); rest = None; harmony = Some(4); arp = true; pads = 3; contour = false; bass = Walk; kick = Seq; clap = BackbeatTone; hats = eighths; form = the hand songs' shape: all in, the lead from bar 5; root periods (bars) = 4, 8; melody terms = 1 x period 4; -2 x period 16; 1 x period 64; 3 x period 32

### 2. s7-c62 (total 0.87)

Strongest: melody range 1.00 (measured 15.00); rhythm syncopation 1.00 (measured 3.00); structure contrast 1.00 (measured 0.40). Weakest: rhythm density 0.70 (measured 13.08); harmony roots 0.73 (measured 0.73).

Composed by luminesce-compose, seed 7 candidate 62: a fibonacci song, grown from one generator through a scale lens.  F(k) is Binet's formula, exact to k = 70, read modulo 8, where it repeats every 12 (its Pisano period). The chord roots are F of the bar number mod 8, and the melody adds F of the step mod 8, as signed degrees. Other voices read F modulo other numbers, whose periods (3, 6, 8, 12, 20) cycle against each other. The lens h is the Dorian mode throughout. The lead is h(M) on every eighth, held legato and gliding into each step. A harmony doubles the lead a diatonic fifth above. 4 pads stack thirds on the root, struck each bar. An arpeggio walks the chord's tones by the generator, every sixteenth. The bass plays the root on every beat. The kick is on every beat. The clap is a dash on 2 and 4. The hats are sixteenths, their loudness read from the generator. Form: the hand songs' shape: all in, the lead from bar 5. 9 definitions and 7 time names.

Parameters: family = fibonacci; bpm = 93; key = -2; lens = Fixed(3); lead steps per beat = 2; legato = true; glide = Some(0.75); rest = None; harmony = Some(4); arp = true; pads = 4; contour = false; bass = Beats; kick = Four; clap = BackbeatDash; hats = sixteenths; form = the hand songs' shape: all in, the lead from bar 5; modulus = 8; pisano period = 12

### 3. s7-c178 (total 0.85)

Strongest: melody range 1.00 (measured 14.00); melody contour 1.00 (measured 0.98); melody repetition 1.00 (measured 0.68). Weakest: melody steps 0.00 (measured 0.00); sound peaks 0.57 (measured 83.00).

Composed by luminesce-compose, seed 7 candidate 178: a squares song, grown from one generator through a scale lens.  sq(v) is a square wave, 0 for the first half of each cycle and 1 for the second: sums of them at time scales an octave apart are binary counting, read as music (Walsh functions). The chord roots are two square waves 4 and 2 bars long. The melody is the root plus 4 square waves: -3 x period 2, 2 x period 16, 1 x period 8, 2 x period 32. The lens brightens one sharp a section, Dorian to Lydian. The lead is h(M) on every eighth, plucked, gliding into the next note. A harmony doubles the lead a diatonic fifth above. 3 pads stack thirds on the root, struck each bar. The bass plays the root on every beat. The kick is a Euclidean E(3, 16) rotated by 3. The clap is a dash on 2 and 4. The hats are sixteenths, their loudness read from the generator. Form: build: one layer more each section. 7 definitions and 7 time names.

Parameters: family = squares; bpm = 104; key = -1; lens = Brighten(3); lead steps per beat = 2; legato = false; glide = Some(0.75); rest = None; harmony = Some(4); arp = false; pads = 3; contour = false; bass = Beats; kick = Euclid(3, 3); clap = BackbeatDash; hats = sixteenths; form = build: one layer more each section; root periods (bars) = 4, 2; melody terms = -3 x period 2; 2 x period 16; 1 x period 8; 2 x period 32

### 4. s7-c174 (total 0.84)

Strongest: melody range 1.00 (measured 13.00); rhythm syncopation 1.00 (measured 7.58); rhythm density 1.00 (measured 6.08). Weakest: sound peaks 0.16 (measured 67.00); sound bands 0.65 (measured 17.26).

Composed by luminesce-compose, seed 7 candidate 174: a fractal song, grown from one generator through a scale lens.  T(u) sums 5 tents at halving sizes, each 0.6 the height of the last (the Takagi-Landsberg curve), so every stretch looks like the whole. The chord roots are T across the whole song, one sample a bar, times 8. The melody adds T at 3 time scales (32, 16, 2 lead steps), so each short figure has the shape of the longer ones. The lens h is the Aeolian mode throughout. The lead is h(M) on every eighth, plucked, sounding on a Euclidean E(4, 8), gliding into the next note. A harmony doubles the lead a diatonic third above. 4 pads stack thirds on the root, struck each bar. A contour voice draws the generator across the whole song, held on scale tones by the glide lens g. The bass jumps octaves on the root in eighths. The kick is on every beat. The clap is a pitch line dashing down on 2 and 4. The hats are eighths, their loudness read from the generator. Form: build, break, return: the drums and bass drop out for section 3. 9 definitions and 6 time names.

Parameters: family = fractal; bpm = 99; key = 0; lens = Fixed(2); lead steps per beat = 2; legato = false; glide = Some(0.75); rest = Euclid(4, 0); harmony = Some(2); arp = false; pads = 4; contour = true; bass = Octaves; kick = Four; clap = BackbeatTone; hats = eighths; form = build, break, return: the drums and bass drop out for section 3; takagi weight = 0.6; takagi terms = 5; root scale = 8; melody scales = [32, 16, 2]; melody amplitudes = [5, 5, 4]

### 5. s7-c38 (total 0.84)

Strongest: melody range 1.00 (measured 10.00); melody steps 1.00 (measured 0.74); melody contour 1.00 (measured 1.00). Weakest: sound peaks 0.41 (measured 77.50); structure energy 0.59 (measured 0.85).

Composed by luminesce-compose, seed 7 candidate 38: a squares song, grown from one generator through a scale lens.  sq(v) is a square wave, 0 for the first half of each cycle and 1 for the second: sums of them at time scales an octave apart are binary counting, read as music (Walsh functions). The chord roots are two square waves 2 and 4 bars long. The melody is the root plus 3 square waves: 1 x period 8, 2 x period 16, 4 x period 2. The lens brightens one sharp a section, Aeolian to Ionian. The lead is h(M) on every sixteenth, plucked, sounding on a Euclidean E(4, 8), gliding into the next note. A harmony doubles the lead a diatonic fifth above. 4 pads stack thirds on the root, struck each bar. The bass jumps octaves on the root in eighths. The kick is drawn from the generator's own sequence. The clap is a dash on 2 and 4. The hats are sixteenths, their loudness read from the generator. Form: an arch: thin, full, full, thinning. 8 definitions and 7 time names.

Parameters: family = squares; bpm = 106; key = -1; lens = Brighten(2); lead steps per beat = 4; legato = false; glide = Some(0.75); rest = Euclid(4, 7); harmony = Some(4); arp = false; pads = 4; contour = false; bass = Octaves; kick = Seq; clap = BackbeatDash; hats = sixteenths; form = an arch: thin, full, full, thinning; root periods (bars) = 2, 4; melody terms = 1 x period 8; 2 x period 16; 4 x period 2

