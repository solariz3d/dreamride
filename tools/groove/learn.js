// learn.js: per-style drum statistics from the Groove MIDI Dataset (Magenta, CC BY 4.0, https://magenta.tensorflow.org/datasets/groove).
//   node learn.js > model.json
// Reads every 4/4 file, quantises onsets to a 16-step bar, and learns per style group and instrument:
//   p[prev][i]   P(hit at step i | hit at the step before)      v[i]  mean and sd of velocity at step i
//   o[i]         mean timing offset at step i, in 16ths          change  share of steps that differ from the bar before
// plus each real bar's density and kick+snare syncopation, so a generated bar can be checked against real ones.
const fs = require('fs'), path = require('path');
const ROOT = 'groove';
const MAP = { K: [35, 36], S: [37, 38, 39, 40], H: [22, 42, 44], O: [26, 46], R: [51, 53, 59], T: [41, 43, 45, 47, 48, 50, 58], C: [49, 52, 55, 57] };
const CLS = {}; for (const [k, ns] of Object.entries(MAP)) for (const n of ns) CLS[n] = k;
const GROUPS = { citypop: ['funk', 'soul', 'pop', 'dance', 'disco'], lofi: ['hiphop'], rock: ['rock'], latin: ['latin', 'afrocuban'], jazz: ['jazz'], reggae: ['reggae'] };
function readMidi(buf) {
  let o = 0; const u32 = () => { const v = buf.readUInt32BE(o); o += 4; return v; }, u16 = () => { const v = buf.readUInt16BE(o); o += 2; return v; };
  if (buf.toString('ascii', 0, 4) !== 'MThd') throw new Error('not a MIDI file');
  o = 8; u16(); const ntr = u16(), tpq = u16(); if (tpq & 0x8000) throw new Error('SMPTE time not supported'); o = 14;
  const notes = [];
  for (let t = 0; t < ntr; t++) {
    if (buf.toString('ascii', o, o + 4) !== 'MTrk') throw new Error('bad track'); o += 4; const end = u32() + o; let tick = 0, run = 0;
    while (o < end) {
      let d = 0, b; do { b = buf[o++]; d = (d << 7) | (b & 127); } while (b & 128); tick += d;
      let st = buf[o]; if (st & 128) o++; else st = run;
      if (st === 0xff) { o++; let l = 0; do { b = buf[o++]; l = (l << 7) | (b & 127); } while (b & 128); o += l; continue; }
      if (st === 0xf0 || st === 0xf7) { let l = 0; do { b = buf[o++]; l = (l << 7) | (b & 127); } while (b & 128); o += l; continue; }
      run = st; const hi = st & 0xf0, a = buf[o++], c = (hi === 0xc0 || hi === 0xd0) ? 0 : buf[o++];
      if (hi === 0x90 && c > 0) notes.push({ tick, note: a, vel: c });
    }
    o = end;
  }
  return { tpq, notes };
}
// Longuet-Higgins & Lee style syncopation for one 16-step bar of onsets: a note on a weak step followed by silence on a stronger one
const W = [0, -4, -3, -4, -2, -4, -3, -4, -1, -4, -3, -4, -2, -4, -3, -4];
function sync(on) { let s = 0; for (let i = 0; i < 16; i++) if (on[i]) for (let j = i + 1; j <= i + 4 && j < 16 + i; j++) { const jj = j % 16; if (on[jj]) break; if (W[jj] > W[i]) { s += W[jj] - W[i]; break; } } return s; }
const info = fs.readFileSync(path.join(ROOT, 'info.csv'), 'utf8').trim().split(/\r?\n/).slice(1).map(l => l.split(','));
const model = {};
for (const [g, styles] of Object.entries(GROUPS)) {
  const acc = { beat: newAcc(), fill: newAcc() }; let files = 0;
  for (const r of info) {
    const style = r[3].split('/')[0], type = r[5], ts = r[6], file = r[7];
    if (!styles.includes(style) || ts !== '4-4') continue;
    if (process.env.SPLIT && r[10] !== process.env.SPLIT) continue;
    const { tpq, notes } = readMidi(fs.readFileSync(path.join(ROOT, file)));
    const bars = new Map();
    for (const n of notes) { const k = CLS[n.note]; if (!k) continue; const p = n.tick / tpq * 4, s = Math.round(p), b = Math.floor(s / 16), i = ((s % 16) + 16) % 16;
      if (!bars.has(b)) bars.set(b, {}); const bar = bars.get(b); bar[k] = bar[k] || Array(16).fill(null);
      if (!bar[k][i] || n.vel > bar[k][i].vel) bar[k][i] = { vel: n.vel, off: p - s }; }
    add(acc[type], [...bars.entries()].sort((a, b) => a[0] - b[0]).map(e => e[1]), +r[4]); files++;
  }
  model[g] = { styles, files, beat: finish(acc.beat), fill: finish(acc.fill) };
}
function newAcc() { const z = () => Object.fromEntries(Object.keys(MAP).map(k => [k, { n: [Array(16).fill(0), Array(16).fill(0)], h: [Array(16).fill(0), Array(16).fill(0)], vs: Array(16).fill(0), vq: Array(16).fill(0), os: Array(16).fill(0), c: Array(16).fill(0), diff: 0, cmp: 0 }])); return { inst: z(), bars: [], bpm: [] }; }
function add(a, bars, bpm) {
  let prev = null;
  for (const bar of bars) {
    const on = k => (bar[k] || Array(16).fill(null)).map(x => !!x);
    for (const k of Object.keys(MAP)) {
      const x = a.inst[k], cur = on(k), last = prev ? on.call(null, k) && (prev[k] || Array(16).fill(null)).map(y => !!y) : null;
      for (let i = 0; i < 16; i++) { const pv = i ? cur[i - 1] : (last ? last[15] : false); x.n[+pv][i]++; if (cur[i]) { x.h[+pv][i]++; const e = bar[k][i]; x.vs[i] += e.vel; x.vq[i] += e.vel * e.vel; x.os[i] += e.off; x.c[i]++; } }
      if (last) { for (let i = 0; i < 16; i++) if (cur[i] !== last[i]) x.diff++; x.cmp += 16; }
    }
    const ks = on('K'), ss = on('S'); a.bars.push({ dens: Object.fromEntries(Object.keys(MAP).map(k => [k, on(k).filter(Boolean).length])), sync: sync(ks.map((v, i) => v || ss[i])) });
    prev = bar;
  }
  a.bpm.push(bpm);
}
function pct(xs, p) { const s = [...xs].sort((a, b) => a - b); return s[Math.min(s.length - 1, Math.floor(p * (s.length - 1)))]; }
function finish(a) {
  const inst = {};
  for (const [k, x] of Object.entries(a.inst)) inst[k] = {
    p: [0, 1].map(pv => x.n[pv].map((n, i) => n ? +(x.h[pv][i] / n).toFixed(3) : 0)),
    v: x.c.map((c, i) => c ? [+(x.vs[i] / c).toFixed(1), +Math.sqrt(Math.max(0, x.vq[i] / c - (x.vs[i] / c) ** 2)).toFixed(1)] : [0, 0]),
    o: x.c.map((c, i) => c ? +(x.os[i] / c).toFixed(3) : 0), change: x.cmp ? +(x.diff / x.cmp).toFixed(3) : 0, hits: x.c.reduce((s, c) => s + c, 0) };
  const band = q => ({ p5: pct(q, 0.05), p50: pct(q, 0.5), p95: pct(q, 0.95) });
  return { bars: a.bars.length, bpm: a.bpm.length ? band(a.bpm) : null, sync: band(a.bars.map(b => b.sync)),
    dens: Object.fromEntries(Object.keys(MAP).map(k => [k, band(a.bars.map(b => b.dens[k]))])), inst };
}
process.stdout.write(JSON.stringify(model));
