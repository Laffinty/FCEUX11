import json
new = json.load(open(r'Z:/Project/FCEUX11/build/v2.1_phase6_session_c_batch_20260902_184955.json'))
old = json.load(open(r'Z:/Project/FCEUX11/build/v2.1_phase6_6_sessionA_batch.json'))
new_p = {r['file']: r for r in new['results']}
old_p = {r['file']: r for r in old['results']}
broken = [f for f in old_p if old_p[f].get('status') == 'PASS' and new_p.get(f, {}).get('status') == 'FAIL']
# 抽 5 个 BROKEN mapper 4 看 PC stuck 模式
print('=== BROKEN mapper 4 PC stuck (first 10) ===')
n = 0
for f in broken:
    r = new_p[f]
    if r.get('mapper') == 4:
        print('  ' + f + ': ' + str(r.get('reason')))
        n += 1
        if n >= 10: break
print()
print('=== BROKEN mapper 0 PC stuck (first 10) ===')
n = 0
for f in broken:
    r = new_p[f]
    if r.get('mapper') == 0:
        print('  ' + f + ': ' + str(r.get('reason')))
        n += 1
        if n >= 10: break
