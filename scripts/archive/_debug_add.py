import re, os, glob
os.chdir('D:/Project/FCEUX11')

SKIP = {178}

with open('src/ines.cpp', encoding='utf-8', errors='replace') as f:
    ines = f.read()
bmap = {}
for m in re.finditer(r'^\s*\{"([^"]*)",\s*(\d+),\s*(\w+)', ines, re.MULTILINE):
    name, num, func = m.group(1), int(m.group(2)), m.group(3)
    if num > 0 or name:
        bmap[num] = (name, func)

to_add = []
for cpp in sorted(glob.glob('src/boards/*.cpp')):
    basename = os.path.basename(cpp)
    if basename.startswith('_') or basename.startswith('__'):
        continue
    if basename in ('registry.cpp', 'mmc3_base_cart.cpp'):
        continue
    with open(cpp, encoding='utf-8', errors='replace') as f:
        content = f.read()
    if 'MapperEntryRegister' in content:
        continue
    inits = re.findall(r'void\s+(\w+_Init)\s*\(\s*CartInfo\s*\*', content)
    for func_name in inits:
        # Look up by func name in bmap values
        for num, (name, bmap_func) in bmap.items():
            if bmap_func == func_name and num not in SKIP:
                to_add.append((num, name, func_name, cpp))
                break

to_add.sort(key=lambda x: x[0])
print(f'Mappers to add: {len(to_add)}')
for num, name, func, cpp in to_add[:10]:
    print(f'  {num}: {name} ({func}) in {os.path.basename(cpp)}')
