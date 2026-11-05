const fs = require('fs'), crypto = require('crypto'), path = require('path');
const [batch, aware, dst] = process.argv.slice(2);
function load(p) { const b = fs.readFileSync(p); let o = 12; while (b.toString('ascii', o, o + 4) !== 'data') o += 8 + b.readUInt32LE(o + 4); return { b, start: o + 8, len: b.readUInt32LE(o + 4) }; }
function rms(w) { let s = 0, n = w.len / 2; for (let i = 0; i < n; i++) { const x = w.b.readInt16LE(w.start + 2 * i); s += x * x; } return Math.sqrt(s / n); }
function scale(w, g) { for (let i = 0; i < w.len / 2; i++) { const p = w.start + 2 * i; w.b.writeInt16LE(Math.round(w.b.readInt16LE(p) * g), p); } }
fs.mkdirSync(dst, { recursive: true });
let key = '';
for (const [n, seed] of [['4', '104389'], ['6', '518643']]) {
  const orig = load(path.join(batch, `stream_s${seed}_1-3.wav`)), aw = load(path.join(aware, `clip0${n}_aware.wav`));
  const [ro, ra] = [rms(orig), rms(aw)], t = Math.min(ro, ra);
  scale(orig, t / ro); scale(aw, t / ra);
  const flip = crypto.randomInt(2);
  const [a, b] = flip ? [aw, orig] : [orig, aw];
  fs.writeFileSync(path.join(dst, `clip${n}_A.wav`), a.b);
  fs.writeFileSync(path.join(dst, `clip${n}_B.wav`), b.b);
  key += `clip${n}: A = ${flip ? 'aware' : 'original'}, B = ${flip ? 'original' : 'aware'} (rms matched to ${(20 * Math.log10(t / 32768)).toFixed(2)} dBFS)\n`;
}
key += 'nonce ' + crypto.randomBytes(8).toString('hex') + '\n';
fs.writeFileSync(path.join(aware, 'key.txt'), key);
console.log('sha256 ' + crypto.createHash('sha256').update(key).digest('hex'));
