"""Chord-tone agreement between a lead list and a bass list in two corpus songs.
For lead notes sounding on beat steps, the interval class (lead - bass) mod 12. Usage: chordtone.py RAWDIR"""
import json, os, re, sys
from collections import Counter
D = sys.argv[1]
def lst(h, name):
    s = json.load(open(os.path.join(D, h + '.json'), encoding='utf8'))
    for e in s['expressions']['list']:
        lat = (e.get('latex') or '').replace(' ', '')
        if lat.startswith(name + '='):
            inner = re.search(r'\\left\[(.*)\\right\]', lat).group(1)
            return [float(re.sub(r'[\\\s]', '', x)) for x in inner.split(',')]
def report(label, pairs):
    ic = Counter(int((l - b) % 12) for l, b in pairs)
    n = sum(ic.values())
    triad = sum(ic[k] for k in (0, 3, 4, 7)) / n
    sev = sum(ic[k] for k in (0, 3, 4, 7, 10, 11)) / n
    print(f'{label}: {n} beat notes; triad tones {triad:.2f}, with 7ths {sev:.2f}; classes {dict(sorted(ic.items()))}')
# Better Off Alone: 64 sixteenth steps per loop; bass one note per 16 steps; beats = every 4th step
L = lst('jbfiikq4mx', 'l_{notes}'); B = lst('jbfiikq4mx', 'b_{notes}')
report('Better Off Alone lead over bass', [(L[i], B[i // 16]) for i in range(0, 64, 4)])
# TETRIS: lead L on sixteenths (t = 1..128, 100 ms a step at 150 BPM), bass B[floor((t+1)/2)] (eighths); beats = every 4th step
L = lst('2sil5zcnun', 'L'); B = lst('2sil5zcnun', 'B')
pairs = []
for t in range(1, 129, 4):
    l = L[t - 1]; b = B[(t + 1) // 2 - 1]
    if l > 0: pairs.append((l, b))
report('TETRIS lead over bass', pairs)
