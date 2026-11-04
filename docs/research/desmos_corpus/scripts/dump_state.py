"""Print a Desmos state's expressions compactly: id, type, folder, latex (truncated). Usage: dump_state.py FILE [maxlen]"""
import json, sys
d = json.load(open(sys.argv[1], encoding='utf8'))
ml = int(sys.argv[2]) if len(sys.argv) > 2 else 220
ex = d.get('expressions', {})
print('ticker:', json.dumps(ex.get('ticker'))[:300])
folders = {e['id']: e.get('title', '') for e in ex.get('list', []) if e.get('type') == 'folder'}
for e in ex.get('list', []):
    t = e.get('type')
    if t == 'folder':
        print(f'## FOLDER {e.get("title","")}'); continue
    if t == 'text':
        print('  #', (e.get('text') or '')[:ml].replace('\n', ' ')); continue
    lat = e.get('latex') or ''
    if t == 'table':
        cols = e.get('columns', [])
        print('  TABLE', [(c.get('latex'), len(c.get('values', []))) for c in cols][:6]); continue
    extra = ''
    if e.get('slider'): extra += ' slider' + json.dumps({k: e['slider'].get(k) for k in ('min', 'max', 'step', 'isPlaying', 'animationPeriod', 'loopMode') if k in e['slider']})
    if e.get('hidden'): extra += ' hidden'
    s = lat if len(lat) <= ml else lat[:ml] + f'...[{len(lat)}]'
    print(f'  {s}{extra}')
