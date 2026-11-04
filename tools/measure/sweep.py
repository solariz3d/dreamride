import sys, numpy as np
sys.path.insert(0, __file__.rsplit('\\', 1)[0].rsplit('/', 1)[0])
from analyze import load

# sweep.py wav onset [n] [fmin] [fmax]: around one onset, every 4 ms from -10 to +118 ms, the
# frequency of the largest rise over the mean spectrum 40-15 ms before it (fmin-fmax, default
# 35-4000 Hz), and its rise level in dB. Use n = 1024 to compare tracks with the README's numbers.
p, on = sys.argv[1], float(sys.argv[2])
n = int(sys.argv[3]) if len(sys.argv) > 3 else 384
x, sr = load(p)
win = np.hanning(n)
def spec(t):
    a = int(t * sr) - n // 2
    return np.abs(np.fft.rfft(x[a:a + n] * win, 32 * n))
f = np.fft.rfftfreq(32 * n, 1 / sr)
base = np.mean([spec(on - 0.040 + k * 0.005) for k in range(6)], 0)
fmin = float(sys.argv[4]) if len(sys.argv) > 4 else 35
fmax = float(sys.argv[5]) if len(sys.argv) > 5 else 4000
m = (f > fmin) & (f < fmax)
out = []
for ms in range(-10, 122, 4):
    d = spec(on + ms / 1000) - base
    d[~m] = 0
    j = np.argmax(d)
    out.append(f'{ms}:{int(f[j])}({20*np.log10(max(d[j],1e-9)):.0f})')
print(f'{on:.3f}s n={n}:', ' '.join(out))
