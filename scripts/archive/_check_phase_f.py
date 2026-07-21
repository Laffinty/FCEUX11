import re, os, glob
os.chdir('D:/Project/FCEUX11')

# Find registered mappers
registered = set()
for cpp in glob.glob('src/boards/*.cpp'):
    with open(cpp, encoding='utf-8', errors='replace') as f:
        content = f.read()
    for m in re.finditer(r'MapperEntry\{(\d+),', content):
        registered.add(int(m.group(1)))

# Phase F mappers in test
phase_f = [14, 27, 30, 31, 35, 111, 116, 123, 125, 132, 133, 136, 137, 138, 139,
           141, 142, 143, 145, 146, 147, 148, 149, 150, 160, 162, 163, 164, 166,
           167, 168, 170, 172, 173, 175, 176, 181, 183, 185, 186, 187, 188, 189,
           190, 193]

for n in phase_f:
    status = "OK" if n in registered else "MISSING"
    if status == "MISSING":
        print(f'  {n}: {status}')
