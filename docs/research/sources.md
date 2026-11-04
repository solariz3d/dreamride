# Knowledge sources for the composer

What we could load as reference, with each licence recorded as precisely as I could confirm it on 2026-11-02.
**(unverified)** marks a licence or figure I could not confirm from the source itself. Nothing here has been
downloaded into the repo.

**Verdicts:**
- **COPY:** text or data can be vendored into the repo, with the attribution the licence asks for.
- **CITE:** read it and cite it; write our own rules from it; do not paste its text.
- **USE:** process it as data at development time (derive statistics, tables, weights) and vendor only what we derive.
  Whether derived numbers are free of the source licence is a judgement call; for share-alike or non-commercial sources,
  treat it as the author's call.

A note on copyleft: this repo's own licence is not yet decided (`DESIGN.md`, "Open"). Vendoring CC BY-SA text puts
share-alike on those files; GPL / AGPL code would put it on the program. Keep share-alike material in its own folder,
and do not copy GPL / AGPL code.

---

## 1. Texts and documentation

| Source | Licence | Verdict | What it gives the composer |
|---|---|---|---|
| **Open Music Theory, v2** (Gotham, Gullings, Hamm, Hughes, Jarvis, Lavengood, Peterson) [viva.pressbooks.pub/openmusictheory](https://viva.pressbooks.pub/openmusictheory/) | CC BY-SA 4.0 "except where otherwise noted" (the copyright line on its introduction page, confirmed via search; the page itself returned 403 to the fetcher). Stable since 1 July 2021. | **COPY** extracts (share-alike, attributed, own folder) or **CITE** | Voice-leading rules, diatonic and chromatic harmony (secondary dominants, modal mixture), and chapters on popular music: pop schemas, loops, form. The best single theory reference to vendor. |
| **Sonic Pi tutorial** [github.com/sonic-pi-net/sonic-pi](https://github.com/sonic-pi-net/sonic-pi/blob/dev/LICENSE.md) | `etc/doc/` (the tutorial and articles) and `etc/examples/`: CC BY-SA 4.0, copyright Sam Aaron unless an author is named. Code: MIT. **Bundled samples and wavetables: CC0 1.0.** All confirmed from `LICENSE.md` (dev branch). | Tutorial: **COPY** with care (share-alike) or **CITE**. Samples: **COPY** freely. | A live-coding tutorial written around rhythm, rings, ticks and synth parameters; close to "music as code". The CC0 drum samples could serve as a reference to tune our synthesised drums against. |
| **Strudel docs** [strudel.cc](https://strudel.cc/learn/faq/) | Code AGPL-3.0. The FAQ says contributed examples and tracks are "separately licensed" and "must not e.g. be used to train AI models without permission". No licence for the doc text itself found **(unverified)**. | **CITE** only | Mini-notation (`"bd*2 [~ sd] hh*4"`), Euclidean rhythms `(3,8)`, `every`, `sometimesBy`, `off`, `jux`: a vocabulary of pattern transformations that maps one-to-one onto functions of `t`. Read for ideas; write our own. |
| **TidalCycles docs** (`tidal-doc`, now at [codeberg.org/uzu/tidal-doc](https://codeberg.org/uzu/tidal-doc)) | GPL-3.0 (the repo's `LICENSE` file). The Tidal library is GPL-3.0 too ([Hackage](https://hackage.haskell.org/package/tidal)). | **CITE** only | Same as Strudel, and the source of its ideas: patterns as functions from time spans to events, which is this program's model with events instead of curves. |
| **Hooktheory** (TheoryTab, Trends) [hooktheory.com](https://www.hooktheory.com/terms) | Terms §4: no third party may "copy, scrape, bulk-download, text-and-data mine, or redistribute", including "to train, fine-tune, or evaluate models", except as authorised by a written data licence or API terms. | **CITE** articles; do **not** scrape | Its *Hooktheory I/II* books and blog explain pop harmony well. See the API row in section 2. |
| **Wikipedia: Royal road progression, UK garage, chord progression articles** | CC BY-SA 4.0 (Wikipedia's standard text licence) | **COPY** extracts (attributed, share-alike) | Progression names and variants, as used in `genres.md`. |
| **Genre production guides** (Native Instruments blog, MusicRadar, Attack Magazine, EDMProd, Mode Audio) | All rights reserved (the publishers' default) | **CITE** only | The tempos, swing values and patterns cited inline in `genres.md`. |

## 2. Datasets

| Source | Licence | Verdict | What it gives the composer |
|---|---|---|---|
| **Groove MIDI Dataset** (Magenta / Google) [magenta.tensorflow.org/datasets/groove](https://magenta.tensorflow.org/datasets/groove) | CC BY 4.0 ("made available by Google LLC under a Creative Commons Attribution 4.0 International (CC BY 4.0) License", confirmed on the page). | **USE** (and COPY a subset if wanted, attributed) | 13.6 h, 1,150 MIDI files (503 beats, 647 fills), 22,000+ bars, 10 drummers on a Roland TD-11, each file tagged with style and tempo, played to a metronome. Gives **real microtiming and velocity per step, per style**: compute the mean offset and velocity for each of the 16 steps × instrument × style and vendor that table (`grooves.json`). Paper: Gillick et al., "Learning to Groove with Inverse Sequence Transformations", ICML 2019 ([arXiv:1905.06118](https://arxiv.org/abs/1905.06118)). |
| **McGill Billboard Project** (Burgoyne, Wild, Fujinaga) [ddmal.music.mcgill.ca](https://ddmal.music.mcgill.ca/research/The_McGill_Billboard_Project_(Chord_Analysis_Dataset)) | CC0 **(unverified at the source**: the official page returned 404 to the fetcher; CC0 is stated by the [mirdata loader](https://mirdata.readthedocs.io/en/latest/_modules/mirdata/datasets/billboard.html) and a GitHub mirror quoting the project page, which also asks that users cite the ISMIR 2011 paper). | **USE**, and COPY if CC0 holds | Expert chord annotations with section labels for ~890 chart slots (the count quoted by search; not re-counted), drawn from the Billboard Hot 100 of roughly 1958–1991 (date range from memory, unverified). Gives chord-transition probabilities and harmonic rhythm for pop/soul/AOR, the era city pop grew from. |
| **Chordonomicon** (Kantarelis et al. 2024) [huggingface.co/datasets/ailsntua/Chordonomicon](https://huggingface.co/datasets/ailsntua/Chordonomicon) | **Conflicting.** The current dataset card's YAML says `cc-by-nc-4.0`; a search snippet of the card said MIT. I read the raw card, so CC BY-NC 4.0 is the better-supported reading. The chords appear to come from user-submitted tabs (Ultimate Guitar) **(unverified; the card does not say)**, so their provenance is a further risk. | **USE** for research only; do not vendor | 666,000+ songs of chord progressions with genre, sub-genre, release date and section labels ([arXiv:2410.22046](https://arxiv.org/abs/2410.22046)). Gives per-genre progression frequencies at scale. Non-commercial terms make it unsuitable for shipping derived tables without the author's decision. |
| **Hooktheory Trends API** [hooktheory.com/api/trends/docs](https://www.hooktheory.com/api/trends/docs) | No API terms on the docs page; the site terms (above) forbid bulk download and model use unless an API or data licence authorises it. API: login → bearer token (~1 h), 10 requests per 10 s; "Can I get a full dump … ? Not right now." | **CITE** / ask Hooktheory before USE | Next-chord probabilities given a chord path (`trends/nodes?cp=4,5`): exactly a Markov table of pop harmony. Valuable, but needs written permission to store. |
| **Lakh MIDI Dataset** (Raffel) [colinraffel.com/projects/lmd](https://colinraffel.com/projects/lmd/) | CC BY 4.0 for the dataset as distributed (stated on the page). The MIDI files themselves are transcriptions of copyrighted songs collected from the web, so the underlying works are not CC **(a provenance caveat, not a licence statement from the page)**. | **USE** for statistics only | 176,581 unique MIDI files (45,129 matched to the Million Song Dataset). Gives drum-pattern, bass-rhythm and melody-interval statistics by genre (via the MSD match). Large and noisy; a few thousand files may be corrupt. |
| **Rolling Stone 200 rock corpus** (de Clercq & Temperley 2011) | **(unverified)**: not checked | **CITE** / USE if licence permits | Harmonic and melodic analyses of 200 rock songs; a smaller, cleaner progression corpus. |

## 3. Claude skills and agent prompts for composition

| Source | Licence | Verdict | What it gives |
|---|---|---|---|
| **PyTheory** + its Claude Code plugin [github.com/kennethreitz/pytheory](https://github.com/kennethreitz/pytheory) (skills in `kennethreitz/pytheory-skill`) | MIT (library, confirmed on the repo page). The skill repo's own licence **(unverified)**. | **CITE** / COPY if the skill repo is MIT | Six skills teaching Claude to compose, analyse and notate with PyTheory's Python API. Useful as a model of how to structure a composition skill; its content is about the library, not about genre. |
| **Build Music** skill (lens-studio-devs, `ls-extensions/plugins/ls-clad/skills/build-music/SKILL.md`) [claudskills.com listing](https://claudskills.com/skills/build-music/) | Apache-2.0 (the SKILL.md SPDX header, per the listing; repo not opened) | **COPY** possible (Apache-2.0, keep the notice) | Scale-degree composition, pattern-based arrangement and frequency-separated mixing rules for offline WAV synthesis: the closest in spirit to this program. Not read in full; worth reading before writing our own composer skill. |
| **Music Composer** (ellmos-ai/skills), **generative-music-composer** (4444j99/a-i--skills) | Listed as MIT on an aggregator **(unverified)** / unknown | Read first | Seen only on aggregator sites (claudskills.com, vibeindex.ai); contents not verified. |
| **anthropics/skills** | (not checked for this task) | Reference for the SKILL.md format | No music skill known; useful only for format. |

I found no public skill that encodes genre recipes like `genres.md`; this file is closer to that than anything seen.

## 4. Papers directly usable by an algorithmic composer

### Groove, syncopation and microtiming

| Paper | Licence | Verdict | What it gives |
|---|---|---|---|
| Witek, Clarke, Wallentin, Kringelbach, Vuust, "Syncopation, Body-Movement and Pleasure in Groove Music", *PLOS ONE* 9(4) e94446, 2014 ([PMC](https://pmc.ncbi.nlm.nih.gov/articles/PMC3989225/)) | CC BY (confirmed on PMC) | **COPY** figures/tables with attribution; CITE | The inverted U between syncopation and groove; a syncopation index for polyphonic drum patterns (supplement Text S2) the composer can compute to keep patterns in the sweet spot. 66 participants, 50 breaks. |
| Matthews, Witek, Heggli, Penhune, Vuust, "The sensation of groove is affected by the interaction of rhythmic and harmonic complexity", *PLOS ONE* 2019 ([PMC](https://pmc.ncbi.nlm.nih.gov/articles/PMC6328141/)) | CC BY 4.0 (PLOS ONE's standard licence; not read on the page) | CITE | Rhythmic complexity: inverted U. Harmonic complexity: low and medium chords rated similarly. Tells the composer where complexity pays. |
| Senn, Kilchenmann, von Georgi, Bullerjahn, "The Effect of Expert Performance Microtiming on Listeners' Experience of Groove in Swing or Funk Music", *Frontiers in Psychology* 7:1487, 2016 ([Frontiers](https://www.frontiersin.org/journals/psychology/articles/10.3389/fpsyg.2016.01487/full)) | CC BY 4.0 **(unverified**; Frontiers' standard, footer not read) | CITE | Quantised ≈ original microtiming for groove; exaggerated microtiming is worse. A rule against random humanisation. |
| Frühauf, Kopiez, Platz, "Music on the timing grid", *Musicae Scientiae* 17(2), 2013; Davies, Madison, Silva, Gouyon, "The effect of microtiming deviations on the perception of groove in short rhythms", *Music Perception*, 2013 | Publisher copyright | CITE | Both reported (via secondary sources) as finding microtiming lowers groove. Original findings not read. |
| Danielsen (ed.), *Musical Rhythm in the Age of Digital Reproduction*, Ashgate 2010; the RADR project report ([UiO](https://hf.uio.no/imv/english/research/projects/rhythm/sluttrapport.pdf)) | Book: publisher copyright | CITE | The "beat bin": a beat as a zone with width and shape. Justifies layered, slightly offset onsets (laid-back snare, flam-like kick+bass). |
| Roger Linn interview on swing, Attack Magazine ([link](https://attackmagazine.com/features/interview/roger-linn-swing-groove-magic-mpc-timing/)) | All rights reserved | CITE | The swing-percentage definition and its useful range (50–70%; 54% loosens without swinging; 66% = triplet). |
| Toussaint, "The Euclidean Algorithm Generates Traditional Musical Rhythms", *BRIDGES* 2005 | **(unverified)** | CITE (the algorithm is free to implement) | E(k, n): spread k hits as evenly as possible over n steps. One formula generates a large family of idiomatic patterns (E(3,8) tresillo, E(5,8), E(7,16)); ideal for functions of `t`. |
| Gillick et al., "Learning to Groove with Inverse Sequence Transformations", ICML 2019 ([arXiv](https://arxiv.org/abs/1905.06118)) | arXiv licence **(unverified)** | CITE | Defines groove as velocity + microtiming over a quantised score; the model for how to separate a pattern from its feel. |

### Arrangement, tension and repetition

| Paper | Licence | Verdict | What it gives |
|---|---|---|---|
| Solberg & Jensenius, "Pleasurable and Intersubjectively Embodied Experiences of Electronic Dance Music", *Empirical Musicology Review* 11(3–4), 2016/2017 ([UiO](https://duo.uio.no/handle/10852/55312)) | Open access; exact licence **(unverified)** | CITE | Breakdown → build-up → drop measured on 16 dancers: movement drops in breakdowns, rises at drops; build-ups and drops rated most pleasurable. |
| Cameron et al., "Undetectable very-low frequency sound increases dancing at a live concert", *Current Biology* 32(21), 2022 | Publisher; OA status **(unverified)** | CITE | About 12% more movement with inaudible sub-bass (press figures; original not read). |
| Hove, Marie, Bruce, Trainor, PNAS 111(28), 2014 ([PMC](https://pmc.ncbi.nlm.nih.gov/articles/PMC4104866)); Wojtczak, Mehta, Oxenham, PNAS 2017 ([PMC](https://pmc.ncbi.nlm.nih.gov/articles/PMC5293018)) | PMC open access; licence **(unverified)** | CITE | The low-voice timing advantage, and a failed replication. Use as a caution, not a rule. |
| Huron, *Sweet Anticipation: Music and the Psychology of Expectation*, MIT Press 2006 | Publisher copyright | CITE | ITPRA: imagination, tension, prediction, reaction, appraisal. The theory behind risers and set-up-then-deliver. |
| Margulis, *On Repeat: How Music Plays the Mind*, Oxford UP 2014 | Publisher copyright | CITE | Why repetition makes music; supports "repeat more than feels safe". |
| Salimpoor et al., "Anatomically distinct dopamine release during anticipation and experience of peak emotion to music", *Nature Neuroscience* 14, 2011 | Publisher copyright | CITE **(details from memory; verify before quoting)** | Anticipation and peak are separate rewards. |

### Harmony, voice leading and melody generation

| Paper | Licence | Verdict | What it gives |
|---|---|---|---|
| Rohrmeier, "Towards a generative syntax of tonal harmony", *Journal of Mathematics and Music* 5(1):35–53, 2011 | Publisher; OA **(unverified)** | CITE | Context-free rewrite rules for functional harmony (T → T D, D → S D, secondary-dominant insertion). Directly implementable as a progression grammar that produces turnarounds and secondary dominants by rule. |
| Steedman, "A generative grammar for jazz chord sequences", *Music Perception* 2(1), 1984 | Publisher copyright | CITE | A handful of rewrite rules for 12-bar blues / jazz substitutions; a smaller grammar to start from. |
| Pachet & Roy, "Markov constraints: steerable generation of Markov sequences", *Constraints* 16(2):148–172, 2011 ([Springer](https://link.springer.com/article/10.1007/s10601-010-9101-4)) | Listed as open access by Springer per search **(unverified licence)** | CITE | Markov melody/chord generation with hard constraints (end on the tonic, hit a given note on a given beat): the right tool for "a Markov melody that still cadences". |
| Tymoczko, *A Geometry of Music*, Oxford UP 2011 | Publisher copyright | CITE | Voice leading as minimal distance between chords; the cost function to minimise when voicing. |
| Temperley, *The Cognition of Basic Musical Structures*, MIT Press 2001 | Publisher copyright | CITE | Preference-rule models of metre, key and harmony; useful as scoring rules for generated output. |

### Consonance and roughness

| Source | Licence | Verdict | What it gives |
|---|---|---|---|
| Plomp & Levelt, "Tonal consonance and critical bandwidth", *JASA* 38, 1965 | Publisher copyright | CITE | Roughness of two pure tones peaks at about a quarter of a critical band. The basis of everything below. |
| Sethares, *Tuning, Timbre, Spectrum, Scale* (Springer) and his `disscurve2.cpp` ([link](https://sethares.engr.wisc.edu/files/disscurve2.cpp)) | Book: publisher. Code: **no licence notice** on the file | CITE; re-implement the formula ourselves | The parametrised Plomp–Levelt curve (constants read from his code): `s = 0.24 / (0.0207·f_min + 18.96)`, `d = min(a_i, a_j) · (5·e^(−3.51·s·Δf) − 5·e^(−5.75·s·Δf))`, summed over all partial pairs. Computable directly on our synthesised spectra, so the composer can score a voicing's roughness *for the timbre it will actually play with*. |
| Hutchinson & Knopoff 1978; Vassilakis 2001 (roughness models) | Publisher copyright | CITE **(not checked for this task)** | Alternative roughness models, if Sethares' proves too crude. |

---

## 5. What I could not verify

- The official pages for McGill Billboard (404) and Open Music Theory (403) could not be fetched; their licences come from
  mirrors and search excerpts.
- Chordonomicon's licence conflicts (CC BY-NC 4.0 in the current card vs MIT in a search excerpt) and its data
  provenance is not stated on the card.
- No API-specific terms exist on Hooktheory's API docs page; the site terms are restrictive.
- The Strudel / TidalCycles doc *text* licences (as opposed to code) are not stated anywhere I found, except tidal-doc's
  GPL-3.0 `LICENSE`.
- I could not find @measure_plan / "Stereo Drift" online at all; everything about the reference comes from
  `DESIGN.md`.
- Third-party skill contents (Build Music, Music Composer, generative-music-composer) were seen only through aggregator
  listings.
- Paper licences marked (unverified) above, and Salimpoor 2011 details, were not checked against the papers.
