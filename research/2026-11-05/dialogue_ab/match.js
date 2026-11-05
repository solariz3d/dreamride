const fs = require('fs');
const dir = process.argv[2];
function load(p) {
  const b = fs.readFileSync(p);
  let o = 12;
  while (b.toString('ascii', o, o + 4) !== 'data') o += 8 + b.readUInt32LE(o + 4);
  return { b, start: o + 8, len: b.readUInt32LE(o + 4) };
}
function rms(w) { let s = 0, n = w.len / 2; for (let i = 0; i < n; i++) { const x = w.b.readInt16LE(w.start + 2 * i); s += x * x; } return Math.sqrt(s / n); }
const a = load(dir + '/v2.wav'), b = load(dir + '/v3.wav');
const ra = rms(a), rb = rms(b);
console.log('rms dB re full scale: v2', (20 * Math.log10(ra / 32768)).toFixed(2), 'v3', (20 * Math.log10(rb / 32768)).toFixed(2));
const [loud, quiet, name] = ra > rb ? [a, rb / ra, 'v2'] : [b, ra / rb, 'v3'];
for (let i = 0; i < loud.len / 2; i++) { const p = loud.start + 2 * i; loud.b.writeInt16LE(Math.round(loud.b.readInt16LE(p) * quiet), p); }
fs.writeFileSync(dir + '/' + name + '_matched.wav', loud.b);
fs.copyFileSync(dir + '/' + (name === 'v2' ? 'v3' : 'v2') + '.wav', dir + '/' + (name === 'v2' ? 'v3' : 'v2') + '_matched.wav');
const c = load(dir + '/v2_matched.wav'), d = load(dir + '/v3_matched.wav');
console.log('scaled', name, 'by', (20 * Math.log10(quiet)).toFixed(2), 'dB; matched rms: v2', (20 * Math.log10(rms(c) / 32768)).toFixed(3), 'v3', (20 * Math.log10(rms(d) / 32768)).toFixed(3));
