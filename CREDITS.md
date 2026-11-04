# Carried alongside

Nothing here was invented from nothing. These are the people whose work taught this program what it knows. We copied
none of their songs or code; we learned from what they found, and we carry it alongside them. Sources and licences are
in `docs/research/`.

- **Alan (@measure_plan, Stereo Drift, Montreal)**: graphing-calculator city pop. The formulas that started this
  project: scale degrees by `floor((12d+2)/7)`, chord roots stored as digits, voicings folded by `mod`, and swing as a
  time warp whose two branches meet at exactly 58%.
- **Desmos**: `tone()`, the sine that graph music is built from, and its documented gain cap, `min(10, 660/f)`, which
  is why high notes are quiet in graph music. The v4 instruments are modelled on both.
- **@phras_22**: a public Desmos synth graph whose voices are sums of `tone()` sines, which showed how graph music makes
  its richer sounds.
- **viznut (Ville-Matias Heikkilä)**: bytebeat, 2011, the discovery that one line of arithmetic played as audio can be
  music.
- **0b5vr**: live sets performed as GPU shaders, whole songs as one function of time.
- **The Dollchan bytebeat community**: thousands of formula songs, each one someone's find.
- **The demoscene** (4klang, Oidos, Sointu, WaveSabre): whole soundtracks from a few kilobytes, and arrangement as
  order lists.
- **The live-coders** (Strudel, TidalCycles, Sonic Pi): pattern maths as groove.
- **Godfried Toussaint**: Euclidean rhythms.
- **Maria Witek and colleagues**: a medium amount of syncopation is where groove lives.
- **Reinier Plomp, Willem Levelt and William Sethares**: consonance and roughness, measured.
- **Google Magenta's Groove MIDI Dataset**: real drummers' timing, kept so it could be learned from.
- **The McGill Billboard project and Open Music Theory**: harmony, annotated and taught in the open.
- **Anders Friberg, Johan Sundberg, Roberto Bresin and Lars Frydén** (KTH, the Director Musices rule system): that a
  performance is a set of small, named, measurable rules. High Loud (3 dB per octave), melodic charge and its table, the
  Phrase rule's arch and final lengthening, articulation; and Friberg and Sundberg's final ritard, measured from runners
  stopping (1999).
- **Neil P. McAngus Todd**: the phrase arch as one motion, tempo and loudness rising and falling together ("The dynamics
  of dynamics", 1992).
- **John Sloboda**: the passages that move people to tears and chills, and the appoggiatura among them (1991).
- **David Huron**: *Sweet Anticipation* (2006), expectation as the engine of feeling, and why a withheld tonic lands.
- **Jezar at Dreampoint**: Freeverb (2000), given to the public domain; and **Julius O. Smith III**, whose *Physical Audio
  Signal Processing* explains it, Schroeder's and Moorer's reverberators beneath it, so it could be written again here.
- **Manfred Schroeder and James A. Moorer**: the comb-and-allpass reverberator, and its lowpass-feedback comb.
- The singers and players whose vibrato research gives its rate and onset (5-7 Hz, starting late).
- **Ada Lovelace**, who wrote in 1843 that an engine "might compose elaborate and scientific pieces of music of any
  degree of complexity or extent".
- **Omar Cornut and the Dear ImGui contributors**: Dear ImGui, which draws the world's menu overlay. This one is code,
  not only a lesson: it is vendored in `world/src/imgui` under its MIT licence (`world/src/imgui/LICENSE.txt`), taken
  from the copy in the author's Dreamzone-RT.
- **For the composer** (`docs/composer.md`), whose grammar and critic are built from what these people found:
  - **H. Christopher Longuet-Higgins and Christopher Lee** (1984), the syncopation measure, with **W. Tecumseh Fitch and
    Andrew Rosenfeld** (2007), whose table of its metrical weights the critic uses.
  - **Piet Vos and Jim Troost** (1989), that melodies move mostly by small steps; **Paul von Hippel and David Huron**
    (2000), why leaps turn back; and Huron again (1996), the melodic arch.
  - **Elizabeth Hellmuth Margulis**, *On Repeat* (2014): repetition is what makes sound heard as music.
  - **Trevor de Clercq and David Temperley** (2011, rock harmony) and **Martin Rohrmeier and Ian Cross** (2008, Bach's
    chorales), how chord roots move.
  - **Ragnhild Torvanger Solberg and Alexander Refsum Jensenius** (2017), dancers measured through build-up, breakdown and
    drop.
  - **Teiji Takagi** (1903), the blancmange curve, and **Georg Landsberg**, its weighted family; **Jules Antoine
    Lissajous** and **Nathaniel Bowditch**, the figures; **Jacques Binet**, the closed form of Fibonacci, and
    **D. D. Wall** (1960), its periods modulo m; **Lothar Collatz**; **Stephen Wolfram**, rule 30; **Joseph L. Walsh**,
    the square-wave functions.
  - **Ralph Bradley and Milton Terry** (1952), paired comparisons: the plan for learning the critic's weights from the
    author's ratings.
  - **Guy Steele, Doug Lea and Christopher Flood**, SplitMix64; **James Cooley and John Tukey**, the FFT; **Peter Welch**,
    averaged spectra; **Howard Hinnant**, the civil-date algorithm that names each run's folder.

- **For stream mode**:
  - **Alan (@measure_plan)** again: his videos set the equation under the graph with a stage counter, and each stage
    edits or adds a term while the music plays on. Stream mode is that, with the program making the edits.
  - **Kenneth E. Iverson**, who gave floor and ceiling their brackets, ⌊x⌋ and ⌈x⌉ (*A Programming Language*, 1962): the
    live equation draws them sized to what they hold.
  - **Donald Knuth**, whose TeX taught how math is laid out (a fraction's bar on the axis, scripts raised and smaller); the
    overlay's markup borrows its command names (`\frac`, `\sqrt`). Written here from those ideas, not from TeX's code.
  - **Microsoft's Cambria**, by **Jelle Bosma** with **Steve Matteson** and **Robin Nicholas** (2004), which ships with
    Windows and sets the equation.

- **For the move book** (`src/compose/book.txt`):
  - **DIXERION** (Better Off Alone) and **pan Waja** (TETRIS), whose Desmos graphs gave the measured rate of chord tones
    under a melody's beats (0.72-0.75) that the book's melodies are held to; **CodePlanTB** (A Mathematical Ballad) and
    **DeeFeeCee** (Overtime 2.0), whose lists set the step and repetition bands.
  - **Karl Weierstrass**, the sum of cosines at many scales; **Edward Lorenz**, whose drifting system the Lorenz-like
    sum of sines only gestures at.
  - **Roland's TR-808**, whose gliding bass taught a generation what portamento under a kick can do.
  - The author, whose saved stretch (a blancmange curve read at three time scales, gliding) is the model for
    four of the leads.

When this program learns from someone new, their name goes here.

- **For the song species** (`src/compose/species/`), each a different universe of constraints:
  - **Harry Partch** (*Genesis of a Music*, 1949) and **William Sethares** (*Tuning, Timbre, Spectrum, Scale*, 1998):
    just intonation, harmony as whole-number ratios, the seventh harmonic and the undertone chord, and why a timbre
    should match its tuning (the `ratios` species).
  - **Robert May** ("Simple mathematical models with very complicated dynamics", *Nature* 261, 1976): the logistic
    map; **Mitchell Feigenbaum** (1978), where its period doubling runs out; **Yves Pomeau and Paul Manneville** (1980),
    intermittency at the edge of the period-3 window (the `chaos` species).
  - **Stephen Wolfram** (1983; *A New Kind of Science*, 2002): elementary cellular automata and their numbering, rules
    30 and 90; **Matthew Cook** (2004), rule 110's universality (the `automaton` species and the `ca` built-in).
  - **Brian Eno**: *Music for Airports* (1978), loops of incommensurable lengths that drift and rarely realign, and his
    talks on generative music; **Joseph Yasser** (*A Theory of Evolving Tonality*, 1932) and **Easley Blackwood**, on
    19-tone equal temperament (the `loops` species).
  - **Heinz Bohlen** (1978), **Kees van Prooijen** and **John R. Pierce** (1984), with **Max Mathews**: the
    Bohlen-Pierce scale, thirteen steps to the tritave, and its odd-harmonic triads (the `tritave` species).
  - The theorists and players of **maqam**, and the **Cairo Congress of Arab Music** (1932): quarter tones and neutral
    thirds; and classical mechanics' coefficient of restitution, which shapes the bouncing phrases (the `bounce`
    species).
  - **Albert S. Bregman** (*Auditory Scene Analysis*, 1990) and **Leon van Noorden** (1975): auditory streaming, why
    one fast line splits into a band; the chiptune composers who made chords from one channel's arpeggio; and **J. S.
    Bach**'s solo suites and partitas, implied polyphony (the `oneline` species).
  - Call and response from West African and African-American music (**Samuel A. Floyd Jr.**, *The Power of Black
    Music*, 1995), and **Neil Todd** and **Anders Friberg and Johan Sundberg** again for the breathing tempo (the
    `dialogue` species).
