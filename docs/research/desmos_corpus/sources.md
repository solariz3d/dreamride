# Desmos music corpus: sources

64 public Desmos graphs that play a song, a beat, or self-playing generative music, fetched 2026-11-03. Each raw state
is `raw/<hash>.json`, byte for byte as served. The analysis is in `../desmos_corpus.md`.

**Public, and what that allows.** Every graph here was reached through a public share link (a desmos.com/calculator,
/3d or /geometry URL) posted on the page in the "found on" column, or listed in a public Desmos gallery. None states a
licence, so Desmos's terms of service govern them; most covers also carry the original song's copyright. **They are
kept to learn from, not to copy:** no example song or test fixture in this repository may be a translation of one of
them (see `../desmos_corpus.md`, "Calibration pieces").

**How a state was fetched.** The graph page `https://www.desmos.com/<calculator|3d|geometry>/<hash>` names its state URL
(`calc-states/production/version/<hash>/<version>`, or `calc-3d-states/...`); older graphs are served from
`https://saved-work.desmos.com/calc-states/production/<hash>`. The title is the page's `<title>`. Two geometry graphs
(`hforvza4lz`, `plnmywzex1`) could not be fetched and are not in the corpus.

**Categories:** *cover* (a known piece, transcribed), *original* (composed by the graph's author), *beat* (a drum loop),
*generative* (a rule makes the notes), *resynth* (a recording's frequency and amplitude frames, resynthesised; not
notes). *Audio trace* marks graphs from before `tone()`, played by Desmos's audio trace of a curve's height.

**Regard** is the outside signal of how it was received, as found: an r/desmos post score (from the Arctic Shift
archive of r/desmos, read 2026-11-03), YouTube views (the watch page, 2026-11-03), a finalist place in the 2026 Desmos
Music Challenge run by the YouTube channel Mathemagic Land (its three public result galleries), or a place in the
official Desmos Art Contest galleries (desmos.com/art-2023, -2024, -2025; the gallery bundles list featured graphs,
and a graph can appear in more than one year's bundle).

| hash | title | category | author (as credited on the graph or post) | found on | regard |
|---|---|---|---|---|---|
| e2iix1hhkk | silent road | cover | nyansci (port); arranged by bermei.inazawa | [Music Challenge, Song](https://www.desmos.com/gallery/4208356a-27d0-4092-a3fc-07f4e324a630) | challenge finalist |
| jvagomcuy3 | My Way (Frank Sinatra) | resynth | "total" | Music Challenge, Song | challenge finalist |
| agcq6pwjzr | snake eyes cover | cover | nyansci (port); arranged by Alexander Panos | Music Challenge, Song | challenge finalist |
| gaowmi4hi0 | supersaw sequencer | original | nyansci | Music Challenge, Song | challenge finalist |
| qeth5bpf0u | Oliver Richman - Bunnies (Reprise) | cover | DragonFire28 (@df28) | Music Challenge, Song | challenge finalist |
| pdhikcv2zr | Bad Apple!! | resynth | Soup (the-can-of-soup) | Music Challenge, Song | challenge finalist |
| h8y6e2aavi | Phase Callibration Radio | cover | Satindra Ramalingam (saturn.255) | Music Challenge, Song | challenge finalist |
| a9jlvp2yl8 | Transmission (Sploinky Doinky) | original | u/tomate_22_ | Music Challenge, Song | challenge finalist |
| vcldbih0wa | family mart theme | cover | not credited on the graph; r/desmos post of the same song by Biansci | Music Challenge, Song | challenge finalist |
| wcq51fhx0m | Looping The Rooms with Music Video | cover | Askalotl | Music Challenge, Song | challenge finalist |
| 3mcvjtyd4q | A Mathematical Ballad | original | CodePlanTB | Music Challenge, Song | challenge finalist |
| tzfakerrgm | Friday Night Funkin' | cover | gimikER | Music Challenge, Song | challenge finalist |
| lonntptn3d | Never Gonna Give You Up | cover | DevDoors | Music Challenge, Song | challenge finalist |
| sjdmjwxcsf | Giant Steps - leoo_c2 | cover | leoo_c2 | Music Challenge, Song | challenge finalist |
| 2sil5zcnun | TETRIS | cover | pan Waja | Music Challenge, Song | challenge finalist |
| 34gdqnbihn | Bach Cello Suite No. 1 in G Major | cover | Talentsonofpoela | Music Challenge, Song | challenge finalist |
| cjcu7govza | The Fractal Domain | generative | PhoenixAD09 | [Music Challenge, Sound Effect](https://www.desmos.com/gallery/6a4d8ec9-4137-49db-829f-199ef13c4322) | challenge finalist (sound effect) |
| 0tlexdfyjb | Fractal Sounds | generative | Warthog71129 | Music Challenge, Sound Effect | challenge finalist (sound effect) |
| w07779vzrr | polyrhythms | generative | crinchcrunch | Music Challenge, Sound Effect | challenge finalist (sound effect) |
| bcjl2hgbgh | Shepards Tone | generative | not credited | Music Challenge, Sound Effect | challenge finalist (sound effect) |
| idy42pofff | Lead Melody | cover | DIXERION | [Music Challenge, Instrument](https://www.desmos.com/gallery/365424d6-adcf-49f4-8053-2f6194c1eaa9); [r/desmos](https://www.reddit.com/r/desmos/comments/1tk09xt/) | challenge finalist; r/desmos 21 |
| 8kxhmzypm8 | Mini Desmos Song | original | daireks | Desmos Art Contest gallery (art-2025 bundle) | art contest featured |
| 4fe020e052 | Clair de Lune | cover | Audrey Chiang | Desmos Art Contest gallery (art-2023, art-2025 bundles) | art contest featured |
| 0b41b463f4 | Glitch Symphony (Swan Lake) | cover | Dheer Mukund Om | Desmos Art Contest gallery (art-2025 bundle) | art contest featured |
| wdu9e4eovg | "Little" Fugue in G minor BWV578 | cover | "Y." | Desmos Art Contest gallery (art-2024, art-2025 bundles) | art contest featured |
| 0196ae61eb | music box | cover | zboy | Desmos Art Contest gallery (art-2023, art-2025 bundles) | art contest featured |
| f5t0narvnf | Starman | cover | Jack Stormes | Desmos Art Contest gallery (art-2024, art-2025 bundles) | art contest featured |
| uuxpmvvjz7 | MIDIJam Bass & Guitar | cover | Jasper Jahnig | Desmos Art Contest gallery (art-2024, art-2025 bundles) | art contest featured |
| mdhcdhnqbx | Composition VIII (Mussorgsky's Promenade) | cover | Avi Rosenberg | Desmos Art Contest gallery (art-2025 bundle) | art contest featured |
| fuyhyjnedt | MUSIC IN DESMOS WHAT | original (a loop) | SzefRon | [YouTube](https://www.youtube.com/watch?v=xdwn2F9pZyg) | YouTube 121k views |
| bqrffiubis | Overtime 2.0 (Desmix) with Notes | cover (reversed and re-arranged) | DeeFeeCee | [YouTube](https://www.youtube.com/watch?v=El5LEivPHlc); v1 on [r/desmos](https://www.reddit.com/r/desmos/comments/1iasqpq/) | YouTube 7.9k; r/desmos 14 (v1) |
| ug51lg43xu | rickroll ("I taught Desmos how to sing") | resynth | beatbox4108 | [r/desmos](https://www.reddit.com/r/desmos/comments/1qr50ov/) | r/desmos 401 |
| mbokvmxbzr | Music Engine w/ Drums (rickroll) | cover | Emblem32 | [r/desmos](https://www.reddit.com/r/desmos/comments/1jxyi40/) | r/desmos 241 |
| dmsixrvsee | Believer (Imagine Dragons) | resynth | random-tomato | [r/desmos](https://www.reddit.com/r/desmos/comments/1k1i0x3/) | r/desmos 546 |
| jbfiikq4mx | Better Off Alone | cover | DIXERION | [r/desmos, "The tone function is fun"](https://www.reddit.com/r/desmos/comments/1jvo7my/) | r/desmos 259 |
| notnmptv6n | The Spectre | cover | DIXERION | same post | r/desmos 259 |
| rxm7hwvl0o | Astronomia | cover | DIXERION | same post | r/desmos 259 |
| dpavflxnmf | Rush E | resynth | random-tomato | [r/desmos](https://www.reddit.com/r/desmos/comments/1ewbwd5/) | r/desmos 174 |
| 4tca7a3nay | Polyrhythms | generative | Gallium-Gonzollium | [r/desmos](https://www.reddit.com/r/desmos/comments/1hrwbi9/) | r/desmos 193 |
| e8vwkgezij | MANDELBROT music | generative | Rotcehhhh | [r/desmos](https://www.reddit.com/r/desmos/comments/1rstbms/) | r/desmos 68 |
| d60jdhrsl0 | Tetris? (game with music) | cover | DanielTheManiel- | [r/desmos](https://www.reddit.com/r/desmos/comments/1dsitpf/) | r/desmos 62 |
| h20c0nnwix | Christmas Music Box | cover | kingofblasphemy | [r/desmos](https://www.reddit.com/r/desmos/comments/1hne96s/) | r/desmos 54 |
| 7ielw63oyk | Blasphemian (Intro) cover | cover | Legitimate_Animal796 | [r/desmos](https://www.reddit.com/r/desmos/comments/1ndpbdj/) | r/desmos 37 |
| 8vvg5m9zfr | Mountain of Faith (game with music) | cover | Mark_Ma_ | [r/desmos](https://www.reddit.com/r/desmos/comments/1htzgj0/) | r/desmos 30 |
| c3en3v0uzb | Polyrhythms visualization | generative | Numerous_Judgment980 | [r/desmos](https://www.reddit.com/r/desmos/comments/1jas9ky/) | r/desmos 35 |
| c6cspwifej | Prime polyrhythm visualization | generative | Numerous_Judgment980 | [r/desmos](https://www.reddit.com/r/desmos/comments/1jbeivg/) | r/desmos 27 |
| petadzc4sb | Song ("Song I made in this silly calculator") | original | Pitiful_Dealer5685 | [r/desmos](https://www.reddit.com/r/desmos/comments/1vxhju7/) | r/desmos 23 |
| j5x56bdn6c | aloboi styled desmos music | original | TylevGD | [r/desmos](https://www.reddit.com/r/desmos/comments/1khu80d/) | r/desmos 24 |
| cyd1baqnyg | We'll Be Fine (first minute) | cover | Yawcty | [r/desmos](https://www.reddit.com/r/desmos/comments/1w5m5eb/) | r/desmos 20 |
| 1bikdj5g4c | randomMusic | generative | sargos7 | [r/desmos](https://www.reddit.com/r/desmos/comments/1dualqz/) | r/desmos 23 |
| r20eqqtxwl | Fibonacci music | generative | sargos7 | [r/desmos](https://www.reddit.com/r/desmos/comments/1e226ed/) | r/desmos 2 |
| krs6vorqns | Markov Chain | generative | rimuru_tempest_slima | [r/desmos](https://www.reddit.com/r/desmos/comments/1n3f0yu/) | r/desmos 3 |
| d51lat4mbh | fallen down (Undertale) | cover | rolling_watermelon | [r/desmos](https://www.reddit.com/r/desmos/comments/1ij0lbu/) | r/desmos 15 |
| y5tldfmcxx | Beatuds | beat | Berrynote | [YouTube](https://www.youtube.com/watch?v=Z62C4l_hqus) | YouTube 7.9k |
| gj3xpo0ykm | Testing graph ("Making beats in Desmos") | beat | Berrynote | [YouTube](https://www.youtube.com/watch?v=60iq3VJM924) | YouTube 32k |
| 3bwlb8y8di | amen break | beat | not credited (linked by Guzman_Tierno) | [Desmos forum](https://cl.desmos.com/t/making-drums-percussion-in-desmos/8095) | the forum's answer to "how to make drums" |
| trfe0iapq9 | Your New Home (The Amazing Digital Circus) | cover | Brain of a Penguin | [YouTube](https://www.youtube.com/watch?v=Do2DNjkgZok); [GitHub README](https://github.com/Herobrine692/Desmos-Music-Maker) | YouTube 51k |
| vapr9l1wxk | Nyan Cat | cover, audio trace | James Cozens | [YouTube](https://www.youtube.com/watch?v=fC33nZc21Z0) | YouTube 250k |
| lsnyus8jox | Rush E | cover, audio trace | James Cozens | [YouTube](https://www.youtube.com/watch?v=MxnZqemF3Fc) | YouTube 196k |
| nixzh3xzyr | among us | cover, audio trace | Yacka Desmos | [YouTube](https://www.youtube.com/watch?v=UV_uEr7OvLU) | YouTube 416k |
| gf2xwv8oo6 | Simple Gifts | cover, audio trace | Eric Tao | [YouTube](https://www.youtube.com/watch?v=kZq-V8oHz84) | YouTube 9.4k |
| z2oofcw6tx | Desmos Song 4 | cover, audio trace | Mason Hirschi | cited in `docs/research/prior_art.md` §2 | none found |
| 0ntuvwjibi | SynthWave | original, audio trace | Connor Criss | [YouTube](https://www.youtube.com/watch?v=hCD1M_G1leQ) | YouTube 34k |
| 05h9f2smtg | Playing a melody | cover (the official example) | Desmos | [Desmos help, "Tone"](https://help.desmos.com/hc/en-us/articles/21373904717197-Tone) | the official example |

## Where else was searched, and what was left out

- **r/desmos:** every post from 2024-05-28 to 2026-11-03 (14,000 posts, from the Arctic Shift archive; Reddit itself
  returned 403), filtered by music words in the title, then the comments of the 256 matching posts read for graph links
  posted by the original poster. Of the 192 graphs linked, 190 were fetched; most are instruments, sound effects or
  sine-wave art, not songs.
- **YouTube:** 27 searches (watch-page descriptions read for graph links), including the channels Eric Tao, Berrynote,
  Yacka Desmos, James Cozens, DeeFeeCee and Twopie Math.
- **Desmos community forum** (cl.desmos.com): the music threads "Play music in Desmos!", "Desmos Song Player", "Making
  drums/percussion in Desmos" and the Desmos Music Lab thread. Its search API rate-limited after a few queries.
- **GitHub:** repository search "desmos music" and the READMEs of 15 Desmos-music repositories (mostly MIDI-to-Desmos
  converters).
- **Not in the corpus but fetched** (kept in the harvest folder, not here): 189 other graphs that use `tone()`:
  pianos, DAWs and sequencers without a song, single sound effects, the "Cursed Desmos Sounds" videos' graphs (1.3M
  views for one, but each is a single strange sound, not music), and art pieces with a sound effect. The harvest folder
  was the session scratchpad and is not kept.
