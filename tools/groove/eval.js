// eval.js: is the generator aware of style? A classifier built ONLY from the validation split scores
//   (a) real bars from the held-out test split, and (b) bars the generator makes from the train split's model.
// Registered before the run: generated bars are classified to their own style no less often than real test bars
// are, minus 15 points, for each style with at least 30 real test bars. Otherwise the generator is not aware.
const fs = require('fs'), path = require('path'), { groove } = require('./generate.js');
const val = require('./model_val.json'), train = require('./model_train.json');
const STY = Object.keys(val).filter(s => val[s].beat.bars >= 40), USE = ['K', 'S', 'H', 'R', 'O'];
function ll(style, bar) { const m = val[style].beat.inst; let s = 0;
  for (const k of USE) { const on = bar[k].map(Boolean); for (let i = 0; i < 16; i++) { const pv = i ? on[i - 1] : on[15]; const p = Math.min(0.98, Math.max(0.02, m[k].p[+pv][i])); s += Math.log(on[i] ? p : 1 - p); } } return s; }
const classify = bar => STY.reduce((b, s) => ll(s, bar) > ll(b, bar) ? s : b, STY[0]);
// real test bars, read the same way learn.js reads them
const MAP = { K: [35, 36], S: [37, 38, 39, 40], H: [22, 42, 44], O: [26, 46], R: [51, 53, 59], T: [41, 43, 45, 47, 48, 50, 58], C: [49, 52, 55, 57] };
const CLS = {}; for (const [k, ns] of Object.entries(MAP)) for (const n of ns) CLS[n] = k;
const src = fs.readFileSync('learn.js', 'utf8'); eval(src.slice(src.indexOf('function readMidi'), src.indexOf('// Longuet')));
const info = fs.readFileSync('groove/info.csv', 'utf8').trim().split(/\r?\n/).slice(1).map(l => l.split(','));
const real = {}; for (const s of STY) real[s] = [];
for (const r of info) { if (r[10] !== 'test' || r[5] !== 'beat' || r[6] !== '4-4') continue; const st = STY.find(s => val[s].styles.includes(r[3].split('/')[0])); if (!st) continue;
  const { tpq, notes } = readMidi(fs.readFileSync(path.join('groove', r[7]))); const bars = new Map();
  for (const n of notes) { const k = CLS[n.note]; if (!k) continue; const s16 = Math.round(n.tick / tpq * 4), b = Math.floor(s16 / 16), i = ((s16 % 16) + 16) % 16;
    if (!bars.has(b)) bars.set(b, Object.fromEntries(Object.keys(MAP).map(x => [x, Array(16).fill(0)]))); bars.get(b)[k][i] = n.vel; }
  real[st].push(...bars.values()); }
console.log('style      real test bars   real classified own   generated classified own (200 seeds x 8 bars)   verdict');
for (const s of STY) {
  const rb = real[s], ra = rb.length ? rb.filter(b => classify(b) === s).length / rb.length : NaN;
  let gn = 0, gt = 0; for (let seed = 1; seed <= 200; seed++) for (const b of groove(train, s, seed).bars) { gt++; if (classify(b) === s) gn++; }
  const ga = gn / gt, verdict = rb.length < 30 ? 'too few real test bars to judge' : (ga >= ra - 0.15 ? 'PASS' : 'FAIL');
  console.log(`${s.padEnd(10)} ${String(rb.length).padStart(14)}   ${(100 * ra).toFixed(0).padStart(18)}%   ${(100 * ga).toFixed(0).padStart(28)}%                    ${verdict}`);
}
