import json
v1 = json.load(open(r'Z:/Project/FCEUX11/build/v2.1_phase6_session_c_batch_20260902_184955.json'))
v2 = json.load(open(r'Z:/Project/FCEUX11/build/v2.1_phase6_session_c_v2_batch_20260902_194834.json'))
v1p = {r['file']: r for r in v1['results']}
v2p = {r['file']: r for r in v2['results']}
v1_pass = {f for f, r in v1p.items() if r.get('status') == 'PASS'}
v2_pass = {f for f, r in v2p.items() if r.get('status') == 'PASS'}
v1_fail = {f for f, r in v1p.items() if r.get('status') == 'FAIL'}
v2_fail = {f for f, r in v2p.items() if r.get('status') == 'FAIL'}
print('v1 PASS:', len(v1_pass), 'v2 PASS:', len(v2_pass), 'Delta:', len(v2_pass) - len(v1_pass))
print('v1 FAIL:', len(v1_fail), 'v2 FAIL:', len(v2_fail), 'Delta:', len(v2_fail) - len(v1_fail))
print()
print('FIXED (v1 FAIL -> v2 PASS):', len(v1_fail & v2_pass))
print('BROKEN (v1 PASS -> v2 FAIL):', len(v1_pass & v2_fail))
print('STILL FAIL:', len(v1_fail & v2_fail))
print('STILL PASS:', len(v1_pass & v2_pass))
