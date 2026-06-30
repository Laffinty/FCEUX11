import re
with open('src/ines.cpp', encoding='utf-8', errors='replace') as f:
    ines = f.read()
bmap = {}
for m in re.finditer(r'^\s*\{"([^"]*)",\s*(\d+),\s*(\w+)', ines, re.MULTILINE):
    name, num, func = m.group(1), int(m.group(2)), m.group(3)
    if num > 0 or name:
        bmap[func] = (num, name or f'Mapper {num}')
print(f'bmap entries: {len(bmap)}')
for i, (k,v) in enumerate(list(bmap.items())[:5]):
    print(f'  {k}: {v}')
print()

# Check a specific file
with open('src/boards/103.cpp', encoding='utf-8', errors='replace') as f:
    content = f.read()
inits = re.findall(r'void\s+(\w+)_Init\s*\(\s*CartInfo\s*\*', content)
print(f'103.cpp Init functions: {inits}')
for func_name in inits:
    if func_name in bmap:
        print(f'  {func_name} -> bmap[{func_name}] = {bmap[func_name]}')
    else:
        print(f'  {func_name} -> NOT in bmap')
