import sys, numpy as np
sys.path.insert(0, __file__.rsplit('\\', 1)[0].rsplit('/', 1)[0])
from analyze import load

# partials.py wav t0 t1 [fmin fmax floor_db]: peaks of one long-window spectrum, with parabolic
# interpolation, nearest equal-tempered note (A4 = 440) and cents off, level in dB re the max.
p, t0, t1 = sys.argv[1], float(sys.argv[2]), float(sys.argv[3])
fmin = float(sys.argv[4]) if len(sys.argv) > 4 else 40
fmax = float(sys.argv[5]) if len(sys.argv) > 5 else 6000
floor_db = float(sys.argv[6]) if len(sys.argv) > 6 else -50
x, sr = load(p)
seg = x[int(t0 * sr):int(t1 * sr)]
n = len(seg)
w = np.blackman(n)
N = 1 << int(np.ceil(np.log2(n * 4)))
S = np.abs(np.fft.rfft(seg * w, N))
f = np.fft.rfftfreq(N, 1 / sr)
db = 20 * np.log10(S / S.max() + 1e-12)
names = 'A A# B C C# D D# E F F# G G#'.split()
out = []
for i in range(2, len(S) - 2):
    if f[i] < fmin or f[i] > fmax: continue
    if db[i] > floor_db and db[i] >= db[i - 1] and db[i] > db[i + 1] and db[i] >= db[i - 2] and db[i] > db[i + 2]:
        a, b, c = db[i - 1], db[i], db[i + 1]
        d = 0.5 * (a - c) / (a - 2 * b + c)
        fr = (i + d) * sr / N
        semis = 12 * np.log2(fr / 440)
        k = int(round(semis))
        out.append((fr, b - 0.25 * (a - c) * d, names[k % 12] + str(4 + (k + 9) // 12), 100 * (semis - k)))
print(f'{len(out)} peaks above {floor_db} dB, {fmin}-{fmax} Hz, window {n / sr * 1000:.0f} ms')
for fr, lv, nm, c in out:
    print(f'  {fr:8.1f} Hz  {lv:6.1f} dB  {nm:4s} {c:+5.0f}c')
