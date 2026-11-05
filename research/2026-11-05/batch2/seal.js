const fs = require('fs'), crypto = require('crypto'), path = require('path');
const [src, dst] = process.argv.slice(2);
function load(p) { const b = fs.readFileSync(p); let o = 12; while (b.toString('ascii', o, o + 4) !== 'data') o += 8 + b.readUInt32LE(o + 4); return { b, start: o + 8, len: b.readUInt32LE(o + 4) }; }
function rms(w) { let s = 0, n = w.len / 2; for (let i = 0; i < n; i++) { const x = w.b.readInt16LE(w.start + 2 * i); s += x * x; } return Math.sqrt(s / n); }
const files = fs.readdirSync(src).filter(f => /^(phrase|now)_s\d+_1-3\.wav$/.test(f));
const waves = files.map(f => ({ f, w: load(path.join(src, f)) }));
waves.forEach(x => x.r = rms(x.w));
const target = Math.min(...waves.map(x => x.r));
// shuffle (Fisher-Yates, crypto)
for (let i = waves.length - 1; i > 0; i--) { const j = crypto.randomInt(i + 1); [waves[i], waves[j]] = [waves[j], waves[i]]; }
let key = '';
waves.forEach((x, k) => {
  const g = target / x.r;
  for (let i = 0; i < x.w.len / 2; i++) { const p = x.w.start + 2 * i; x.w.b.writeInt16LE(Math.round(x.w.b.readInt16LE(p) * g), p); }
  const name = 'clip_' + String(k + 1).padStart(2, '0') + '.wav';
  fs.writeFileSync(path.join(dst, name), x.w.b);
  key += `${name} = ${x.f.replace('.wav', '')} (scaled ${(20 * Math.log10(g)).toFixed(2)} dB)\n`;
});
key += 'nonce ' + crypto.randomBytes(8).toString('hex') + '\n';
fs.writeFileSync(path.join(src, 'key.txt'), key);
console.log('clips', waves.length, 'matched to rms', (20 * Math.log10(target / 32768)).toFixed(2), 'dBFS');
console.log('sha256 ' + crypto.createHash('sha256').update(key).digest('hex'));
