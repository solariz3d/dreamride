"""Print a state's tone() expressions plus the definitions they reference (one level), and its notes/text.
Usage: tones_only.py FILE [maxlen]"""
import json, re, sys
d = json.load(open(sys.argv[1], encoding='utf8'))
ml = int(sys.argv[2]) if len(sys.argv) > 2 else 300
L = d.get('expressions', {}).get('list', [])
print('ticker:', json.dumps(d.get('expressions', {}).get('ticker'))[:300])
defs = {}
for e in L:
    lat = e.get('latex') or ''
    m = re.match(r'^\s*\\?([A-Za-z](?:_\{[A-Za-z0-9]+\})?)(?:\\left\(([^)]*)\\right\))?\s*=', lat)
    if m: defs.setdefault(m.group(1), lat)
def short(s): return s if len(s) <= ml else s[:ml] + f'...[{len(s)}]'
seen = set()
def show(lat, depth):
    for name in re.findall(r'(?<![A-Za-z\\])([A-Za-z]_\{[A-Za-z0-9]+\}|[A-Za-z])(?![A-Za-z])', lat):
        if name in defs and name not in seen and name not in ('x', 'y', 'i', 'e'):
            seen.add(name); print('   ' * depth + '= ' + short(defs[name]))
            if depth < 2: show(defs[name], depth + 1)
for e in L:
    if e.get('type') == 'text':
        t = (e.get('text') or '').strip()
        if t: print('# ' + short(t.replace('\n', ' ')))
for e in L:
    lat = e.get('latex') or ''
    if 'operatorname{tone}' in lat:
        print('TONE ' + short(lat)); show(lat, 1)
