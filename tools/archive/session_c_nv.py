import json
new = json.load(open(r'Z:/Project/FCEUX11/build/v2.1_phase6_session_c_batch_20260902_184955.json'))
old = json.load(open(r'Z:/Project/FCEUX11/build/v2.1_phase6_6_sessionA_batch.json'))
old_nv = set()
for r in old['results']:
    reason = r.get('reason') or ''
    if 'no_video' in reason:
        old_nv.add(r['file'])
new_p_pass = {r['file'] for r in new['results'] if r.get('status') == 'PASS'}
fixed_from_nv = old_nv & new_p_pass
print('Old no_video:', len(old_nv))
print('Fixed from no_video -> new PASS:', len(fixed_from_nv))
print('Sample:')
for f in sorted(fixed_from_nv)[:5]:
    print(' ', f)
