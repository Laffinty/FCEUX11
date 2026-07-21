import re, os
os.chdir('D:/Project/FCEUX11')
with open('src/ines.cpp', encoding='utf-8', errors='replace') as f:
    ines = f.read()
bmap = {}
for m in re.finditer(r'^\s*\{"([^"]*)",\s*(\d+),\s*(\w+)', ines, re.MULTILINE):
    name, num, func = m.group(1), int(m.group(2)), m.group(3)
    if num > 0 or name:
        bmap[func] = (num, name)
print(f'bmap size: {len(bmap)}')
print(f'UNL22211_Init in bmap: {"UNL22211_Init" in bmap}')
print(f'Mapper172_Init in bmap: {"Mapper172_Init" in bmap}')
print(f'Mapper173_Init in bmap: {"Mapper173_Init" in bmap}')
