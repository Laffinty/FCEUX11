import json, re
from collections import Counter
new = json.load(open(r'Z:/Project/FCEUX11/build/v2.1_phase6_session_c_batch_20260902_184955.json'))
old = json.load(open(r'Z:/Project/FCEUX11/build/v2.1_phase6_6_sessionA_batch.json'))
def breakdown(results):
    reasons = {}
    for r in results:
        if r.get('status') == 'PASS':
            continue
        reason = r.get('reason', 'unknown')
        m = re.match(r'([^=]+(?:_[^=]+)*)=', reason)
        key = m.group(1) if m else reason[:20]
        reasons[key] = reasons.get(key, 0) + 1
    return reasons
nb = breakdown(new['results'])
ob = breakdown(old['results'])
print('=== Failure reason comparison ===')
print('  Reason                         New    Old  Delta')
for key in sorted(set(nb) | set(ob)):
    n, o = nb.get(key, 0), ob.get(key, 0)
    print(f'  {key:<30} {n:>5} {o:>5} {n-o:>+5}')
print()
broken = [(r.get('mapper', 0), r['file']) for r in new['results'] if r.get('status') == 'FAIL']
print('=== BROKEN by mapper (top 10) ===')
c = Counter(m for m, _ in broken)
for mapper, count in c.most_common(10):
    print(f'  mapper {mapper:>3}: {count}')
