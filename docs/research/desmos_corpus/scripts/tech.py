"""Tag each curated graph with the encoding techniques present in its LaTeX. Usage: tech.py RAWDIR
Every tag is a regex over the source; it says the technique is PRESENT, not that it carries the music.
Prints a matrix and per-technique counts (all curated, and the 'regarded' subset)."""
import json, os, re, sys
sys.path.insert(0, os.path.dirname(__file__))
from curated import C
D = sys.argv[1]
def src(h):
    s = json.load(open(os.path.join(D, h + '.json'), encoding='utf8'))
    ex = s.get('expressions', {})
    lat = [e.get('latex') or '' for e in ex.get('list', [])]
    for e in ex.get('list', []):
        if e.get('type') == 'table':
            lat += [c.get('latex') or '' for c in e.get('columns', [])]
    t = ex.get('ticker') or {}
    lat.append(t.get('handlerLatex') or '')
    lat.append(t.get('minStepLatex') or '')
    sliders = [e for e in ex.get('list', []) if (e.get('slider') or {}).get('isPlaying')]
    return '\n'.join(lat), bool(t.get('handlerLatex')), bool(sliders)
NUM = r'-?\d+(?:\.\d+)?'
T = {
 'step-list (list indexed by a time counter)': lambda x: re.search(r'\\left\[[^\]]{0,80}\\operatorname\{(?:floor|mod)\}|\[\\operatorname\{(?:floor|mod)\}|\\left\[t(?:_\{[^}]*\})?\\right\]|\\left\[T(?:_\{[^}]*\})?\\right\]|\\left\[a\\right\]', x),
 'event list (pitch,time) or time-window filter': lambda x: re.search(r'\\le t<|\\le t\\le|t_\{Start\}\\le t|\\left\(' + NUM + r',\s*' + NUM + r'\\right\),\\left\(' + NUM + r',' , x) or re.search(r'\.x\b.*\.y\b', x, re.S) and re.search(r'\\left\[[^\]]*\\le', x),
 'note-name variables / MIDI numbers': lambda x: re.search(r'2\^\{\\frac\{\\left\(x-69\\right\)\}\{12\}\}|\\frac\{x-69\}\{12\}|C_\{4\}|A_\{4\}=|c_\{s\}|e_\{4\}', x),
 'equal temperament 2^(n/12)': lambda x: re.search(r'2\^\{\\frac\{[^{}]*(?:\{[^{}]*\}[^{}]*)*\}\{12\}\}|\\sqrt\[12\]\{2\}|2\^\{\\left\(\\frac\{[^}]*\}\{12\}', x) or '{12}' in x,
 'ticker clock (actions advance time)': lambda x: bool(re.search(r'\\to', x)),
 'additive harmonics (frequency x list of multiples)': lambda x: re.search(r'\\left\[1(?:,|\.\.\.|,\.\.\.)\s*(?:2|3|\.\.\.)|\\left\[1\.\.\.|\[1\.\.\.|\\operatorname\{for\}\s*[a-z]=\\left\[1|\\sum_\{n=1\}', x),
 'spectral tilt 1/n^p (p tuned, not 1)': lambda x: re.search(r'\\frac\{[^{}]*\}\{[a-zA-Z](?:_\{[^}]*\})?\^\{?(?:[0-9.]+|[A-Za-z]_\{[^}]*\})\}?\}|\{1\.1\^\{P\}1\.6\^\{H\}\}|\^\{-d_\{amping\}\}', x),
 'smooth spectral rolloff / filter on partials': lambda x: re.search(r'butterworth|\\frac\{1\}\{1\+e\^|\\frac\{1\}\{\\sqrt\{1\+\\left\(\\frac\{f\}\{c\}|l_\{owpass\}|cutoff|c_\{utoff\}|L_\{r\}', x),
 'envelope: ADSR piecewise': lambda x: re.search(r'ADSR|A_\{DSR\}|E_\{nvelope\}|E_\{nv\}|s_\{ustain\}|v_\{sustain\}|\\frac\{v_\{max\}\}\{A\}', x),
 'envelope: power/linear decay (1-x)^c': lambda x: re.search(r'\\left\(1-\\frac\{[^}]*\}\{[^}]*\}\\right\)\^|\\left\(1-x\^|1-\\frac\{x\}\{h_\{ittime\}\}', x),
 'envelope: exponential decay': lambda x: re.search(r'e\^\{-|E_\{xpDecay\}|\\exp|3\^\{-n\}|0\.\d+\^\{', x),
 'pitch drop for kick (frequency times a decaying term)': lambda x: re.search(r'tone\}\\left\([^,]*(?:E_\{Kick\}|f_\{1\}\\left\(\\operatorname\{mod\}|\\left\(1-\\frac\{[^}]*\}\{[^}]*\}\\right\)|e\^\{-)', x),
 'noise from random() frequencies': lambda x: re.search(r'tone\}\\left\([^,]{0,60}random|\\operatorname\{random\}\\left\(\d', x) and 'random' in x,
 'echo/delay copy (same line, later, quieter)': lambda x: re.search(r'D\\left\(|R_\{ev\}|Rev\}|d_\{elai\}|P_\{arrot\}|\\cdot\\frac\{1\}\{3S_|delay|D_\{elay\}', x),
 'detune/chorus copies': lambda x: re.search(r'\\left\[0\.9\d*,1,1\.0\d*|h_\{arm3\}|74\.3|d_\{etune\}|D_\{etune\}|\\left\[1,1\.0\d', x),
 'vibrato (sin inside frequency)': lambda x: re.search(r'tone\}\\left\([^,]*\\sin', x) or re.search(r'v_\{ibrato\}|V_\{ibrato\}|v\\left\(x,', x),
 'glide/portamento (smoothstep between notes)': lambda x: re.search(r'G_\{liss|3\\left\([^)]*\\right\)\^\{2\}-2|smooth|l_\{0\}|glide|G_\{lide\}|\\tanh', x),
 'rest as undefined (0/0, 1/0) or zero': lambda x: re.search(r'\\frac\{0\}\{0\}|\\frac\{1\}\{0\}|R=\\frac', x),
 'sections gated by time windows': lambda x: re.search(r'S_\{ection|\\left\\\{[^:{}]{0,40}\\le [A-Za-z_{}]*<[^:]{0,40}:|s_\{ect\}=|\\left\\\{t<t_\{', x),
 'mod-cycled loop (mod of time)': lambda x: re.search(r'\\operatorname\{mod\}\\left\((?:t|T|B_\{eat\}|\\frac\{t|\\operatorname\{floor\}\\left\(t|[0-9.]*t)', x),
 'chord = root + interval list': lambda x: re.search(r'\\left\[0,\s*(?:3|4),\s*7|\+\\left\[0,4,7|C_\{hordMaj\}|C_\{hordMin\}|c_\{left\}', x),
 'per-voice voicing lists (one list per chord tone)': lambda x: len(re.findall(r'S_\{\dC?\}=\\left\[', x)) >= 3 or len(re.findall(r'A_\{note\d\}', x)) >= 3 or len(re.findall(r'c_\{\d+\}=\\left\[', x)) >= 3,
 'tempo ramp / swing (time warp)': lambda x: re.search(r'q_\{uadFormula\}|t_\{AtBeat\}|swing|S_\{wing\}', x),
 'audio-frame resynthesis (frequency/amplitude frames)': lambda x: len(x) > 200000,
 'generative rule (random/fractal/sequence/ratio)': lambda x: re.search(r'z\^\{2\}\+c|Fibonacci|f_\{ib\}|random\\left\(\d*\\right\)\\cdot|M_\{arkov\}|p_\{oly\}|b_\{alls\}|z\^\{2\}', x),
 'digit/bit tables (10^k, 2^k with floor/mod)': lambda x: re.search(r'\\operatorname\{floor\}\\left\(\\frac\{[^}]*\}\{10\^|\\operatorname\{floor\}\\left\(\\frac\{[^}]*\}\{2\^', x),
 'audio trace (curve height is the pitch; no tone())': lambda x: 'operatorname{tone}' not in x,
}
rows = []
for h, title, cat, reg in C:
    x, tick, sl = src(h)
    tags = {k: bool(f(x)) for k, f in T.items()}
    tags['ticker clock (actions advance time)'] = tick
    rows.append((h, title, cat, reg, tags))
regarded = lambda reg: bool(reg) and not reg.startswith('Desmos help') and not reg.startswith('cited') and not reg.startswith('linked') and reg not in ('r/desmos 2', 'r/desmos 3')
print('graphs', len(rows), 'regarded', sum(regarded(r[3]) for r in rows))
print()
for k in T:
    n_all = sum(r[4][k] for r in rows)
    n_reg = sum(r[4][k] for r in rows if regarded(r[3]))
    hs = [r[0] for r in rows if r[4][k] and regarded(r[3])]
    print(f'{n_reg:3d} {n_all:3d}  {k}  e.g. {", ".join(hs[:6])}')
json.dump([{'hash': r[0], 'title': r[1], 'cat': r[2], 'regard': r[3], 'tags': [k for k, v in r[4].items() if v]} for r in rows],
          open('tech_matrix.json', 'w', encoding='utf8'), indent=1)
