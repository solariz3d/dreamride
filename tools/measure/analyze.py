import sys, wave, numpy as np

def load(p):
    w = wave.open(p, 'rb'); n = w.getnframes(); ch = w.getnchannels(); sw = w.getsampwidth(); sr = w.getframerate()
    raw = w.readframes(n)
    a = np.frombuffer(raw, dtype=np.int16 if sw == 2 else np.int32).astype(np.float64)
    a = a.reshape(-1, ch).mean(axis=1)
    return a / (np.abs(a).max() + 1e-12), sr

def bands(x, sr):
    S = np.abs(np.fft.rfft(x * np.hanning(len(x)))) ** 2
    f = np.fft.rfftfreq(len(x), 1 / sr)
    edges = [0, 150, 500, 2000, 6000, 24000]
    tot = S.sum()
    return [100 * S[(f >= a) & (f < b)].sum() / tot for a, b in zip(edges, edges[1:])]

def peaks_per_frame(x, sr, n=8192):
    # in each frame, count spectral peaks within 40 dB of the frame's max, 80 Hz - 8 kHz
    counts, cents = [], []
    for i in range(0, len(x) - n, n):
        fr = x[i:i + n] * np.hanning(n)
        S = np.abs(np.fft.rfft(fr)); f = np.fft.rfftfreq(n, 1 / sr)
        m = (f > 80) & (f < 8000); s = S[m]
        if s.max() < 1e-3: continue
        db = 20 * np.log10(s / s.max() + 1e-12)
        pk = (db[1:-1] > db[:-2]) & (db[1:-1] > db[2:]) & (db[1:-1] > -40)
        counts.append(pk.sum())
        P = S ** 2; cents.append((f * P).sum() / P.sum())
    return np.median(counts), np.median(cents)

def kicks(x, sr, k=6):
    # low band envelope via FFT filter
    X = np.fft.rfft(x); f = np.fft.rfftfreq(len(x), 1 / sr)
    lo = np.fft.irfft(X * (f < 200), len(x))
    hop = int(0.005 * sr); env = np.array([np.sqrt(np.mean(lo[i:i + hop] ** 2)) for i in range(0, len(lo) - hop, hop)])
    d = np.diff(env, prepend=0); thr = env.max() * 0.35
    on = [i for i in range(1, len(env) - 1) if d[i] > thr * 0.5 and env[i] > thr and all(env[i] >= env[max(0, i - 20):i])]
    sel = []
    for o in on:
        if not sel or o - sel[-1] > 40: sel.append(o)
    out = []
    for o in sel[2:2 + k]:
        s0 = o * hop; tr = []
        for ms in range(0, 160, 10):
            a = s0 + int(ms / 1000 * sr); n = int(0.03 * sr)
            seg = lo[a:a + n]
            if len(seg) < n: break
            S = np.abs(np.fft.rfft(seg * np.hanning(n), 16 * n)); ff = np.fft.rfftfreq(16 * n, 1 / sr)
            m = (ff > 25) & (ff < 400)
            tr.append((ms, round(ff[m][np.argmax(S[m])]), round(20 * np.log10(np.sqrt(np.mean(seg ** 2)) + 1e-9), 1)))
        out.append((round(s0 / sr, 2), tr))
    return out

for p in (sys.argv[1:] if __name__ == '__main__' else []):
    x, sr = load(p)
    print('==', p.split('\\')[-1].split('/')[-1], f'{len(x)/sr:.1f}s')
    print('  band energy % (<150, 150-500, 500-2k, 2k-6k, >6k):', [round(b, 1) for b in bands(x, sr)])
    c, ce = peaks_per_frame(x, sr)
    print(f'  median spectral peaks per frame within 40 dB: {c:.0f}; median centroid {ce:.0f} Hz')
    for t, tr in kicks(x, sr, 3):
        print(f'  kick @{t}s  (ms, Hz, dB):', tr[:12])
