// generate.js: a drum groove for a style, sampled from what real drummers in that style play (model from learn.js).
//   node generate.js <model.json> <style> <seed> [band.txt]  -> prints the song (the band with the groove's drums added)
//   require('./generate.js').groove(model, style, seed)        -> { bpm, bars: [{ K: [vel|0 x16], ... }], off, timekeeper }
// Nothing here is a hand-written pattern. Every hit, loudness and timing offset is drawn from the learned odds:
//   - each instrument walks the 16 steps with P(hit | hit on the step before), learned per step;
//   - a bar is kept only when every instrument's density and the kick+snare syncopation sit inside the 5th-95th
//     percentile of real bars of the style (rejection sampling, at most 400 tries, then the closest try);
//   - bars 2-8 repeat bar 1 with each step re-drawn at the style's learned bar-to-bar change rate;
//   - bars 4 and 8 end with a fill drawn from the style's fill bars (their last two beats);
//   - the time is kept by hats or ride, chosen by how often the style's drummers use each;
//   - timing is each step's learned mean offset, with the style's overall lean removed (systematic, never jitter).
const fs = require('fs');
const INST = ['K', 'S', 'H', 'O', 'R', 'T', 'C'];
const W = [0, -4, -3, -4, -2, -4, -3, -4, -1, -4, -3, -4, -2, -4, -3, -4];
function sync(on) { let s = 0; for (let i = 0; i < 16; i++) if (on[i]) for (let j = i + 1; j <= i + 4; j++) { const jj = j % 16; if (on[jj]) break; if (W[jj] > W[i]) { s += W[jj] - W[i]; break; } } return s; }
function rng(seed) { let a = seed >>> 0; return () => { a = (a + 0x6D2B79F5) >>> 0; let t = a; t = Math.imul(t ^ (t >>> 15), t | 1); t ^= t + Math.imul(t ^ (t >>> 7), t | 61); return ((t ^ (t >>> 14)) >>> 0) / 4294967296; }; }
function gauss(r) { return Math.sqrt(-2 * Math.log(r() || 1e-9)) * Math.cos(2 * Math.PI * r()); }
function walk(m, r, from = 0, prevLast = false, base = null) {
  const out = base ? base.slice() : Array(16).fill(0);
  let prev = from ? !!out[from - 1] : prevLast;
  for (let i = from; i < 16; i++) {
    const hit = r() < m.p[+prev][i];
    out[i] = hit ? Math.max(1, Math.min(127, Math.round(m.v[i][0] + m.v[i][1] * gauss(r)))) : 0;
    prev = hit;
  }
  return out;
}
function groove(model, style, seed) {
  const M = model[style]; if (!M) throw new Error(`no style ${style}; have ${Object.keys(model).join(', ')}`);
  const B = M.beat, F = M.fill, r = rng(seed);
  // the timekeeper: hats or ride, by how often this style's drummers play each
  const keep = r() < B.inst.H.hits / (B.inst.H.hits + B.inst.R.hits) ? 'H' : 'R';
  const use = INST.filter(k => k !== (keep === 'H' ? 'R' : 'H') && k !== 'C');
  const inBand = bar => use.every(k => { const n = bar[k].filter(Boolean).length, d = B.dens[k]; return n >= d.p5 && n <= d.p95; })
    && (() => { const s = sync(bar.K.map((v, i) => v || bar.S[i])); return s >= B.sync.p5 && s <= B.sync.p95; })()
    && bar.K.some(Boolean) && bar.S.some(Boolean);
  let first = null, best = null, bestErr = Infinity;
  for (let tries = 0; tries < 400 && !first; tries++) {
    const bar = {}; for (const k of INST) bar[k] = use.includes(k) ? walk(B.inst[k], r, 0, r() < B.inst[k].p[0][15]) : Array(16).fill(0);
    if (inBand(bar)) first = bar;
    else { const err = use.reduce((e, k) => e + Math.abs(bar[k].filter(Boolean).length - B.dens[k].p50), 0); if (err < bestErr) { bestErr = err; best = bar; } }
  }
  first = first || best;
  const bars = [first];
  for (let b = 1; b < 8; b++) {
    const bar = {};
    for (const k of INST) {
      const m = B.inst[k], src = bars[0][k].slice();
      if (use.includes(k)) for (let i = 0; i < 16; i++) if (r() < m.change) { const prev = i ? !!src[i - 1] : !!bars[b - 1][k][15]; src[i] = r() < m.p[+prev][i] ? Math.max(1, Math.min(127, Math.round(m.v[i][0] + m.v[i][1] * gauss(r)))) : 0; }
      bar[k] = src;
    }
    if (b === 3 || b === 7) for (const k of ['K', 'S', 'T', keep, 'O', 'C']) bar[k] = walk(F.inst[k], r, 8, !!bar[k][7], bar[k]);
    bars.push(bar);
  }
  if (B.inst.C.hits) bars[0].C[0] = Math.max(bars[0].C[0], Math.round(B.inst.C.v[0][0] || 90)); // a crash where the loop comes round
  // timing: each step's learned mean offset, minus the style's overall lean (the mean over the beats' own steps)
  const off = {};
  for (const k of INST) { const o = B.inst[k].o, lean = [0, 4, 8, 12].reduce((s, i) => s + o[i], 0) / 4; off[k] = o.map(x => Math.max(-0.4, Math.min(0.4, x - lean))); }
  const bpm = Math.round((B.bpm.p5 + B.bpm.p50) / 2 + r() * ((B.bpm.p50 + B.bpm.p95) / 2 - (B.bpm.p5 + B.bpm.p50) / 2));
  return { style, seed, bpm, keep, bars, off };
}
// ---- the song language
const digit = v => v ? Math.max(1, Math.min(9, Math.round(v / 127 * 9))) : 0;
// a bar as two 8-digit halves: a 16-digit table that starts with 9 can exceed a double's exact range (2^53)
const half = (a, h) => a.slice(8 * h, 8 * h + 8).map(digit).join('');
const SOUND = {
  K: 'kind = dash\n  from = 450\n  to = 58\n  dash = 23\n  hold = 40\n  decay = 90\n  length = 380\n  partials = 0, -24, -12, -20, -19\n  rolloff = 660\n  gain = 0.68',
  S: 'kind = snare\n  gain = 0.55\n  pan = -0.05\n  reverb = 0.25',
  H: 'kind = dash\n  from = 7600\n  to = 5400\n  dash = 5\n  decay = 12\n  length = 70\n  partials = 0, -10\n  rolloff = 660\n  gain = 2.4\n  pan = 0.3',
  O: 'kind = dash\n  from = 7000\n  to = 5200\n  dash = 6\n  decay = 85\n  length = 320\n  partials = 0, -8, -14\n  rolloff = 660\n  gain = 1.7\n  pan = 0.35',
  R: 'kind = dash\n  from = 5400\n  to = 4600\n  dash = 8\n  decay = 170\n  length = 650\n  partials = 0, -6, -11, -15\n  rolloff = 660\n  gain = 1.1\n  pan = -0.3',
  T: 'kind = dash\n  from = 300\n  to = 140\n  dash = 35\n  hold = 20\n  decay = 130\n  length = 380\n  partials = 0, -14, -18\n  rolloff = 660\n  gain = 0.5\n  pan = 0.15',
  C: 'kind = dash\n  from = 6800\n  to = 4800\n  dash = 10\n  decay = 420\n  length = 1600\n  partials = 0, -5, -9, -13\n  rolloff = 660\n  gain = 0.9\n  pan = -0.2\n  reverb = 0.2',
};
const NAME = { K: 'kick', S: 'snare', H: 'hat', O: 'ohat', R: 'ride', T: 'tom', C: 'crash' };
function songText(g, band) {
  const used = INST.filter(k => g.bars.some(b => b[k].some(Boolean)));
  const defs = [
    `# a ${g.style} groove, seed ${g.seed}: sampled from the Groove MIDI Dataset's ${g.style} drummers (Magenta, CC BY 4.0); timekeeper ${g.keep}`,
    'Gn = floor(4t + 0.5)', 'Gu = 4t - Gn', 'Gs = mod(Gn, 16)', 'Gb = mod(floor(Gn / 16), 8)',
    'Gt(v, o) = (v > 0)(Gu >= o)(0.3 - Gu + o)',
    ...used.flatMap(k => [
      ...[0, 1].map(h => `G${k}${h} = ${g.bars.map((b, i) => i < 7 ? `if(Gb == ${i}, ${half(b[k], h)}, ` : half(b[k], h)).join('')}${')'.repeat(7)}`),
      `G${k}o = (hd(${g.off[k].map(x => Math.round(x * 10) + 4).join('')}, Gs) - 4) / 10`,
      `G${k}v = if(Gs < 8, mod(floor(G${k}0 / 10^(7 - Gs)), 10), mod(floor(G${k}1 / 10^(15 - Gs)), 10))`]),
  ].join('\n');
  let s = band.replace(/^bpm \d+$/m, `bpm ${g.bpm}`).replace(/^beats \d+$/m, 'beats 64').replace(/^(K = .*)$/m, `${defs}\n$1`);
  s += '\n' + used.map(k => `drum ${NAME[k]} {\n  trigger = Gt(G${k}v, G${k}o)\n  amp = G${k}v / 9\n  ${SOUND[k]}\n}`).join('\n\n') + '\n';
  return s;
}
module.exports = { groove, songText, sync };
if (require.main === module) {
  const [mf, style, seed, band] = process.argv.slice(2);
  const g = groove(JSON.parse(fs.readFileSync(mf, 'utf8')), style, +seed);
  process.stdout.write(band ? songText(g, fs.readFileSync(band, 'utf8')) : JSON.stringify(g));
}
