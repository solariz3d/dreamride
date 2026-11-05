const fs = require('fs'), path = require('path');
const [batch, out] = process.argv.slice(2);
function edit(src, pairs) {
  let s = fs.readFileSync(path.join(batch, src), 'utf8');
  for (const [a, b] of pairs) { const n = s.split(a).length - 1; if (n !== 1) throw new Error(src + ': ' + n + ' for ' + a.slice(0, 60)); s = s.replace(a, b); }
  return s;
}
// clip 4: the pedal-aware bass
const four = edit('stream_s104389_1-3.txt', [
  ['T_0 = t - 0\n', `Pv(m, i) = K + 6m - 16 + hd(if(m == 0, Hx, if(m == 1, Hy, Hz)), i)
Ph(p, i) = (mod(p - Hr(i), 12) == 0) + (mod(p - Pv(0, i), 12) == 0) + (mod(p - Pv(1, i), 12) == 0) + (mod(p - Pv(2, i), 12) == 0)
Pp(i) = Hr(4 floor(i / 4))
Pd(i) = Ph(Pp(i), i) > 0
T_0 = t - 0
`],
  ['  pitch = if(Q == 0, Ob2_0, if(Q == 1, Ob2_1, Ob2_2))',
   '  pitch = if(Q == 0, if(Pd(floor(T_0 / 2)), lw(Pp(floor(T_0 / 2))), Ob2_0), if(Q == 1, if(Pd(floor(T_1 / 2)), lw(Pp(floor(T_1 / 2))), Ob2_1), if(Pd(floor(T_2 / 2)), lw(Pp(floor(T_2 / 2))), Ob2_2)))'],
]);
fs.writeFileSync(path.join(out, 'clip04_aware.txt'), four);
// clip 6: a noise hat, and an open hat at each chord change
const six = edit('stream_s518643_1-3.txt', [
  [`  kind = dash
  from = 9082.65
  to = 8907.61
  dash = 2.38
  hold = 0`, `  kind = hat
  decay = 44.66
  length = 97.36`],
]);
const ohat = `
drum ohat {
  trigger = (Hr(floor(t / 2)) != Hr(floor(t / 2) - 1)) (frac(t / 2) < 0.5) (0.5 - frac(2t))
  gain = 2.2
  pan = -0.2
  kind = hat
  decay = 260
  length = 420
}
`;
fs.writeFileSync(path.join(out, 'clip06_aware.txt'), six.replace(/\s*$/, '\n') + ohat);
console.log('ok');
