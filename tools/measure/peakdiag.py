import sys, numpy as np
sys.path.insert(0, __file__.rsplit('\\', 1)[0].rsplit('/', 1)[0])
from analyze import load

# peakdiag.py wav...: the analyze.py peak metric (8192 frames, peaks within 40 dB of the frame max,
# 80 Hz - 8 kHz), broken down: median count per band, the median frame max's frequency, and the
# median level (dB re frame max) of the counted peaks per band.
edges = [80, 150, 500, 2000, 8000]
for p in sys.argv[1:]:
    x, sr = load(p); n = 8192
    per = []; fmax = []; lv = [[] for _ in edges[:-1]]
    for i in range(0, len(x) - n, n):
        S = np.abs(np.fft.rfft(x[i:i + n] * np.hanning(n))); f = np.fft.rfftfreq(n, 1 / sr)
        m = (f > 80) & (f < 8000); s = S[m]; ff = f[m]
        if s.max() < 1e-3: continue
        db = 20 * np.log10(s / s.max() + 1e-12)
        pk = np.where((db[1:-1] > db[:-2]) & (db[1:-1] > db[2:]) & (db[1:-1] > -40))[0] + 1
        fmax.append(ff[np.argmax(s)])
        row = []
        for b, (a, c) in enumerate(zip(edges, edges[1:])):
            sel = pk[(ff[pk] >= a) & (ff[pk] < c)]
            row.append(len(sel)); lv[b] += list(db[sel])
        per.append(row)
    per = np.array(per)
    print(p, 'total', int(np.median(per.sum(1))), 'per band', [int(v) for v in np.median(per, 0)],
          'frame max at median', int(np.median(fmax)), 'Hz; median peak level per band', [round(float(np.median(l)), 1) if l else None for l in lv])
