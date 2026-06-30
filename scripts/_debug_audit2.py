import re
with open('src/ines.cpp', encoding='utf-8', errors='replace') as f:
    ines = f.read()
bmap = {}
for m in re.finditer(r'^\s*\{"([^"]*)",\s*(\d+),\s*(\w+)', ines, re.MULTILINE):
    name, num, func = m.group(1), int(m.group(2)), m.group(3)
    if num > 0 or name:
        bmap[func] = (num, name or f'Mapper {num}')
print(f'bmap entries: {len(bmap)}')

# Check: bmap key for mapper 103
for k, v in bmap.items():
    if v[0] == 103:
        print(f'  bmap key for 103: {k}')

# Fix: the Init regex captures "Mapper103" but bmap key is "Mapper103_Init"
# Let me check what the actual function signature looks like
with open('src/boards/103.cpp', encoding='utf-8', errors='replace') as f:
    content = f.read()
for line in content.split('\n'):
    if 'Mapper103_Init' in line and 'void' in line:
        print(f'  103.cpp: {line.strip()}')
