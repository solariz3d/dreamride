import sys, numpy as np
sys.path.insert(0, __file__.rsplit('\\', 1)[0].rsplit('/', 1)[0])
from analyze import load

# harm.py wav t0 t1 [fmin fmax]: find the strongest peak in fmin..fmax as f0 (refined), then the
# level of harmonics 1..12 relative to h1 (max within +-1% of k f0), and the deviation in cents.
p, t0, t1 = sys.argv[1], float(sys.argv[2]), float(sys.argv[3])
fmin = float(sys.argv[4]) if len(sys.argv) > 4 else 200
fmax = float(sys.argv[5]) if len(sys.argv) > 5 else 2000
x, sr = load(p)
seg = x[int(t0 * sr):int(t1 * sr)]
N = 1 << 18
S = np.abs(np.fft.rfft(seg * np.blackman(len(seg)), N)); f = np.fft.rfftfreq(N, 1 / sr)
m = (f > fmin) & (f < fmax)
f0 = f[m][np.argmax(S[m])]
row = []
for k in range(1, 13):
    mm = (f > k * f0 * 0.99) & (f < k * f0 * 1.01)
    if not mm.any(): break
    j = np.argmax(S[mm])
    row.append((k, 20 * np.log10(S[mm][j] / S[m].max() + 1e-12), 1200 * np.log2(f[mm][j] / (k * f0))))
print(f'{t0:.3f}-{t1:.3f}s f0 {f0:.1f} Hz: ' + ' '.join(f'h{k}:{lv:.0f}' for k, lv, c in row))
