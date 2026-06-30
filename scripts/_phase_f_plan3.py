#!/usr/bin/env python3
"""Phase F: batch-add MapperEntryRegister to all unregistered board files."""
import re, glob, os

os.chdir(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

# Parse bmap from ines.cpp
with open('src/ines.cpp', encoding='utf-8', errors='replace') as f:
    ines = f.read()
bmap = {}  # func_name (with _Init) -> (mapper_number, display_name)
for m in re.finditer(r'^\s*\{"([^"]*)",\s*(\d+),\s*(\w+)', ines, re.MULTILINE):
    name, num, func = m.group(1), int(m.group(2)), m.group(3)
    if num > 0 or name:
        bmap[func] = (num, name or f'Mapper {num}')

# Find board files without MapperEntryRegister
changes = []
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
    
    # Find Init functions (capture full name including _Init)
    inits = re.findall(r'void\s+(\w+_Init)\s*\(\s*CartInfo\s*\*', content)
    if not inits:
        continue
    
    mapper_entries = []
    for func_name in inits:
        if func_name in bmap:
            num, display_name = bmap[func_name]
            mapper_entries.append((num, display_name, func_name))
    
    if not mapper_entries:
        continue
    
    changes.append({
        'file': cpp,
        'basename': basename,
        'entries': mapper_entries,
    })

print(f'Files to modify: {len(changes)}')
total_mappers = sum(len(c['entries']) for c in changes)
print(f'Total mappers to register: {total_mappers}')
print()
for c in changes:
    entries_str = ', '.join(f'{num}' for num, _, _ in c['entries'])
    print(f"  {c['basename']:30s} mappers: {entries_str}")
