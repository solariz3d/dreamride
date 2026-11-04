# Dreamride: design

A desktop program that writes music as mathematical functions, plays it procedurally, shows it as a lit
structure in a 3D world, and exports it for a DAW. Settled 2026-11-02. (The crate and binaries are still named `luminesce*`, the working name.)

## What it does

- **Voices are functions of time.** Bass, arp, lead, harmony, kick, snare and hats are each a formula in `t`
  (beats). The program evaluates them, synthesises sound, and draws them.
- **It writes its own functions.** The composer produces songs as function sets; it is not a library of
  finished songs.
- **The composer understands what a song needs:** form (sections), an energy curve, harmonic rhythm, cadences,
  motif statement, repetition and variation, density per section, fills at transitions, tension and release.
- **Per-track control:** Auto (composer writes it), Lock (keep it while re-rolling the rest), Off.
- **Export for further editing in a DAW:** the full mix as WAV, a stem per track, MIDI per track, tempo and key
  in the file names, loop-exact lengths. Adding your own elements happens in the DAW, not in this app.

## The world (the window)

- **The graph stands upright like a wall:** pitch goes up, time goes along. Each voice is a glowing line on it
  (bass low, lead high, chords as stacked bands). The Guitar Hero highway, stood on its edge.
- **The camera travels forward while looking back,** at a three-quarter angle, off to the side of the wall. It
  never looks at the wall straight on.
- **You never see the future.** The present is the **draw-head**, beside the camera, where the newest note
  blooms brightest. Everything already played streams away behind it into the distance, smaller and dimmer,
  fading into fog. Nothing ahead is drawn, which is literally true: the composer has not written it yet.
- **The graph is an object in the world, not a picture:** the lines are tubes and ribbons with thickness, they
  light their surroundings (glow on the floor, colour on nearby surfaces), parallax against the far background
  tells the eye it is really there, and markers (floor grid, bar lines) give it scale.
- **Pull-back:** the camera can lift up and back to show the whole trail, the song as the shape it left.
- **The composer's energy curve can drive the camera:** closer at quiet parts, pulled back at a drop.
- Fog distance follows tempo: a slow song leaves a long clear trail, a fast one dissolves quickly.
- The formulas can be shown as a typeset panel on demand; the world is the main view. Built for stream mode
  (2026-11-02): the playing stage's equation, typeset clear of the draw-head, its changed terms lit at each seam.
  Since 2026-11-03 ("the function needs to be not taking up so much space") it is a quiet strip along the bottom edge
  by default, at most a fifth of the screen high, keeping the changed lines and the generators first; E cycles
  compact, the full panel, hidden.

**Stream mode (added 2026-11-02).** Music as a chain of eight-bar equations, each an evolution of
the one before, composed and rendered a stage ahead of the playhead and joined without a seam. This makes "the composer
has not written it yet" literal: the next equation exists only a stage ahead, and is never drawn. A good stretch can be
saved as an ordinary song. **v2 (2026-11-03)**, after listening found v1 changed too little: every stage is a change
you can hear (a layer, a new generator, a groove, a progression, a register, a feel), measured from the functions and
required to clear a threshold; stages run in arcs (intro, build, peak, breakdown, turn, return); a budget caps the
layers and definitions so it stays simple; and the default stream is a new grammar song each launch, with G kept as
"Stream from G". **The move book (2026-11-03)**, after listening found v2's content flat and MIDI-like: the brain
writes the material (`src/compose/book.txt`, modules of a few graph-native definitions each, tagged by role, energy,
density and mood) and the arc planner only arranges it. A book stream also **ends**, when its own closure score says it
has said what it has to say (never before 2.5 minutes, always by 7), with a composed coda and a ring-out. **Species
(2026-11-03)**, after every stream's function was found to look the same: a layer above the book, each species its own universe of
constraints (tuning, meter, generator, form, timbre, tempo) written as its own book with its own definitions, so the
equation itself differs: the lens, just ratios in 3/4, the logistic map in 7/8, cellular automata in 5/4 whole tone,
Eno loops in 19-TET, Bohlen-Pierce in 12/8, bouncing balls in quarter tones, one curve heard as a band, and a call and
response on a breathing clock. A seed picks one. See README, "Stream mode", and docs/composer.md. **Written harmony
(2026-11-03)**, after a progression made by arithmetic was found to be what made a stream sound bad: harmony is
written, never generated. The 12-tone species play hand-written progressions (`src/compose/harmony.txt`), voice-led
into digit tables, with a cadence at every phrase end and every seam, and their maths writes only what plays on top;
the strange tunings moved behind an Explore option.

**Themes come after the bones.** First the original theme (the plain void: the wall, the draw-head, the trail
and the fog), done well. Stylised themes (nebula, night city, and others) are built from it later.

## Architecture

- **Rust audio engine:** the expression evaluator, synthesis and rendering. Playback and export use the same
  code, so what you hear is what you export.
- **The window is native (changed 2026-11-02, the author's decision):** C++, Direct3D 12 and DXR hardware
  raytracing, from scratch like the author's Dreamzone-RT, in `world/`. It replaces the earlier plan of Tauri and
  WebGL/Three.js for the window, so Three.js is no longer needed. The engine is loaded as `luminesce.dll` through a
  C ABI (`src/ffi.rs`, header `world/src/luminesce.h`). One call renders the song and records what the renderer
  used for every voice (gate, pitch, amp, level every millisecond; every drum onset). The host plays that buffer
  through XAudio2, with the playback position as the master clock, and draws from the record, so the picture
  cannot drift from the sound.
- **W1 render pipeline** (`world/src/main.cpp`, `world/shaders/world.hlsl`, compiled at launch):
  1. The TLAS: two copies of the song's single BLAS, this loop and the one before, moved every frame so the present
     is always at x = 0.
  2. The floor's light: a compute pass integrating every recent tube as a line light into a floor texture.
  3. The floor's reflection: a near-mirror (W2), four inline ray queries per pixel at fixed sub-pixel offsets against
     the TLAS, rejecting hits ahead of the present during traversal, bent by kick ripples, then a gloss blur sized
     by hit distance that fades out below a pixel.
  4. The main pass at 8x MSAA HDR: sky, floor, tubes, draw-heads, dust (additive). Every surface adds the
     closed-form in-scattering of the present's light and of drum flashes.
  5. A luminance-weighted resolve, a 13-tap bloom chain, ACES and dither.
  6. Present with vsync.
- **W2, the look as a theme** (`Theme` in `world/src/main.cpp`, `VoidTheme()` the original). The picture answers the
  music within the void:
  - note onsets flare;
  - loudness is tube thickness, and colour is tinted by pitch class on the circle of fifths;
  - drum dashes stand at their true pitch height;
  - drum hits flash light into the fog, and kicks ring the floor;
  - the camera mixes a quiet and a dense pose by the song's smoothed energy;
  - dust drifts in the graph's light.

  A stylised theme is a new `Theme` value; `LM_THEME` overrides the numbers for a run.
- **The composer and the UI** will need their own window plan (W2 onward): Tauri for the panels beside the native
  world, or panels drawn in the native window. Not decided. What is built (2026-11-02): the player's navigation
  (song library, transport with seeking, view, live theme sliders, keys card) is an overlay drawn in the native
  window with Dear ImGui, under the program's name, **Dreamride**. The composer's panels are still open; the overlay
  shows that panels in the native window are cheap (its GPU cost is within the bench's noise).
- **Composer:** its own module, which writes function sets: whole songs (`luminesce-compose`), or a stream of eight-bar
  stages (`src/compose/stream.rs`) rendered stage by stage with every voice's and effect's state carried across the seam
  (`src/stream.rs`), which the world plays through `lm_stream_*` and draws through a window over the latest stages.

## The reference: @measure_plan's city-pop formulas (Stereo Drift)

- `h(d) = floor((12d + 2) / 7)`: scale degree to semitone. As written this gives the natural MINOR scale
  (0 2 3 5 7 8 10 12); `floor((12d + 5) / 7)` gives major (0 2 4 5 7 9 11 12). Found by the engine's tests,
  2026-11-02; the earlier note here called the +2 form major, which was wrong.
- `root(i) = floor(342258473 / 10^(8 - i)) mod 10 - 5`: chord roots stored as digits.
- `fold(z) = (z + 9) mod 12 - 9`: keeps voicings in range.
- `sw = (floor(2x) + max(frac(2x) / 1.16, (frac(2x) - 0.16) / 0.84)) / 2`: swing as a time warp.
- `w = u^2 (3 - 2u)`: smoothstep, for chord glides.
- Leads are 16-digit integers, one per section; harmony is a third above.

## Build order

1. **Engine, headless** (prove-it tier: the maths and the export): expression parser and evaluator, synthesis,
   WAV render. No external dependencies.
2. **Live playback** (needs `cpal`, pending the author's yes).
3. **The window, original theme:** the wall, the draw-head, the backward-looking camera, the fading trail.
   W1 (the bones, native D3D12 + DXR) and W2 (sharp reflections, a picture that answers the music, the look as a
   theme) are built: see README, "The world".
4. **The composer.**
5. **Stems and MIDI export.**
6. **Stylised themes.**

## Open

- The final name, and whether the GitHub repo is public or private. Nothing is pushed until the author decides.
- `cpal` as a dependency (to ask). Three.js is no longer planned: the world is native.
