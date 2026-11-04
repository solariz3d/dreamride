"""Melody and bass statistics from note lists in corpus graphs, to calibrate the critic against human songs.
Each entry: (hash, list name, unit, kind). Rests (0, 0/0, R) are dropped for interval stats but kept for density.
Prints per list: steps, onsets/step, share of moves that are repeats / 1-2 st / 3-4 st / 5+ st, range p5-p95,
post-leap reversal share, and the best bar repetition (16-step bars) as the critic computes it."""
import json, os, re, sys
D = sys.argv[1]
SEMI = {'C': 0, 'D': 2, 'E': 4, 'F': 5, 'G': 7, 'A': 9, 'B': 11}
def getdef(h, name):
    s = json.load(open(os.path.join(D, h + '.json'), encoding='utf8'))
    for e in s['expressions']['list']:
        lat = (e.get('latex') or '').replace(' ', '')
        if lat.startswith(name + '='):
            return lat[len(name) + 1:]
    raise KeyError(name)
def items(body):
    body = re.sub(r'^[-\d.]*\+?', lambda m: m.group(0) if False else '', body) if False else body
    m = re.search(r'\\left\[(.*)\\right\]', body) or re.search(r'\[(.*)\]', body)
    inner = m.group(1)
    off = 0
    pre = body[:body.find('[') if '[' in body else 0]
    mo = re.match(r'^(-?\d+)\+', body)
    if mo: off = int(mo.group(1))
    return [x for x in re.split(r',(?![^{]*\})', inner)], off
def note(tok, off, scheme):
    tok = tok.strip()
    if not tok.startswith('\\frac'):
        tok = re.sub(r'[\\\s]', '', tok)
    if tok in ('R', '0', '\\frac{0}{0}') or tok.startswith('\\frac{') or tok == '':
        return None
    if scheme == 'num':
        v = float(tok); return v + off
    m = re.match(r'([A-Ga-g])_\{(s?)(\d)\}', tok)
    if not m: return None
    L, sh, octv = m.groups()
    v = SEMI[L.upper()] + (1 if sh else 0) - (1 if L.islower() else 0) + 12 * (int(octv) + 1)
    return v + off
def stats(seq, steps_per_bar=16):
    notes = [x for x in seq if x is not None]
    # onsets: a new value or a value after a rest
    onsets = sum(1 for i, x in enumerate(seq) if x is not None and (i == 0 or seq[i - 1] != x))
    mv = [b - a for a, b in zip(notes, notes[1:])]
    distinct = [b - a for a, b in zip(notes, notes[1:]) if b != a]
    n = len(mv) or 1
    rep = sum(1 for d in mv if d == 0) / n
    st = sum(1 for d in distinct if 1 <= abs(d) <= 2) / (len(distinct) or 1)
    th = sum(1 for d in distinct if 3 <= abs(d) <= 4) / (len(distinct) or 1)
    lp = sum(1 for d in distinct if abs(d) >= 5) / (len(distinct) or 1)
    rev = [1 if (b * a < 0) else 0 for a, b in zip(distinct, distinct[1:]) if abs(a) >= 5]
    srt = sorted(notes)
    p = lambda q: srt[min(len(srt) - 1, int(q * len(srt)))]
    bars = [seq[i:i + steps_per_bar] for i in range(0, len(seq) - steps_per_bar + 1, steps_per_bar)]
    best = []
    for i in range(1, len(bars)):
        best.append(max(sum(1 for a, b in zip(bars[i], bars[j]) if a == b) / steps_per_bar for j in range(i)))
    return dict(steps=len(seq), onsets_per_step=round(onsets / len(seq), 2), repeats=round(rep, 2), steps12=round(st, 2),
                thirds=round(th, 2), leaps5=round(lp, 2), post_leap_reversal=round(sum(rev) / len(rev), 2) if rev else None,
                range_p5_p95=p(0.95) - p(0.05), bar_repeat=round(sum(best) / len(best), 2) if best else None)
LISTS = [
    ('jbfiikq4mx', 'l_{notes}', 'num', 'lead, Better Off Alone (16ths)', 16),
    ('jbfiikq4mx', 'b_{notes}', 'num', 'bass, Better Off Alone (one per bar)', 4),
    ('2sil5zcnun', 'L', 'num', 'lead, TETRIS (16ths)', 16),
    ('2sil5zcnun', 'B', 'num', 'bass, TETRIS (8ths)', 8),
    ('3mcvjtyd4q', 'r_{notes1}', 'num', 'melody, A Mathematical Ballad (original; one note per 300 ms tick)', 6),
    ('bqrffiubis', 'M_{elody6}', 'name', 'melody, Overtime 2.0 bridge (half notes)', 4),
    ('bqrffiubis', 'L_{owBass1}', 'name', 'bass, Overtime 2.0 (half notes)', 4),
    ('bqrffiubis', 'A_{rpeg1}', 'name', 'arpeggio, Overtime 2.0 (8ths)', 8),
    ('gaowmi4hi0', 'S_{0}', 'num', 'root line, supersaw sequencer (original)', 16),
    ('e2iix1hhkk', 'S_{0}', 'num', 'bass, silent road', 16),
]
for h, name, scheme, label, bar in LISTS:
    try:
        body = getdef(h, name)
    except KeyError:
        print('missing', h, name); continue
    toks, off = items(body)
    seq = [note(t, off, scheme) for t in toks]
    print(f'{label} [{h} {name}]', json.dumps(stats(seq, bar)))
# Bach: join of measures M_1..M_20 (16 sixteenths each)
s = []
for k in range(1, 21):
    try:
        toks, off = items(getdef('34gdqnbihn', f'M_{{{k}}}'))
        s += [note(t, off, 'name') for t in toks]
    except KeyError:
        pass
print('Bach Cello Suite 1 prelude, 20 bars of 16ths [34gdqnbihn M_k]', json.dumps(stats(s, 16)))
