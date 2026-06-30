#!/usr/bin/env python3
"""Phase F: batch-add MapperEntryRegister to all unregistered board files."""
import re, glob, os, sys

os.chdir(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

# Parse bmap from ines.cpp to get mapper number -> Init function mapping
with open('src/ines.cpp', encoding='utf-8', errors='replace') as f:
    ines = f.read()
bmap = {}  # func_name -> (mapper_number, display_name)
for m in re.finditer(r'^\s*\{["\']([^"\']*)["\'],\s*(\d+),\s*(\w+)', ines, re.MULTILINE):
    name, num, func = m.group(1), int(m.group(2)), m.group(3)
    if num > 0 or name:
        bmap[func] = (num, name or f"Mapper {num}")

# Parse existing registrations to avoid duplicates
registered_funcs = set()
for cpp in glob.glob('src/boards/*.cpp'):
    with open(cpp, encoding='utf-8', errors='replace') as f:
        content = f.read()
    # Find what's already registered by looking for MapperEntry{ patterns
    # (both single-line and multi-line formats)
    for m in re.finditer(r'MapperEntry\{(\d+)', content):
        pass  # number-based
    # Also check for legacy_init= patterns
    for m in re.finditer(r'legacy_init[= ]*&(\w+)', content):
        registered_funcs.add(m.group(1))
    # And &XxxInit patterns in MapperEntry blocks
    for m in re.finditer(r'MapperEntry\{[^}]*&(\w+_Init)', content):
        registered_funcs.add(m.group(1))

# Find which board files need registration
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
    
    # Find Init functions
    inits = re.findall(r'void\s+(\w+)_Init\s*\(\s*CartInfo\s*\*', content)
    if not inits:
        continue
    
    # Map Init functions to bmap entries
    mapper_entries = []
    for func_name in inits:
        if func_name in bmap:
            num, display_name = bmap[func_name]
            # Create a safe class name
            safe_name = re.sub(r'[^a-zA-Z0-9]', '', func_name.replace('_Init', ''))
            mapper_entries.append((num, display_name, func_name, safe_name))
        else:
            # Not in bmap, skip for now
            pass
    
    if not mapper_entries:
        continue
    
    # Check if file includes simple_carts.h
    has_simple_carts = 'simple_carts.h' in content
    
    # Check what include the file uses
    has_mapinc_bus = 'mapinc_bus.h' in content
    has_mapinc_audio = 'mapinc_audio.h' in content
    has_mapinc_mmc3 = 'mapinc_mmc3.h' in content
    
    changes.append({
        'file': cpp,
        'basename': basename,
        'entries': mapper_entries,
        'has_simple_carts': has_simple_carts,
        'has_mapinc_bus': has_mapinc_bus or has_mapinc_audio or has_mapinc_mmc3,
    })

print(f'Files to modify: {len(changes)}')
total_mappers = sum(len(c['entries']) for c in changes)
print(f'Total mappers to register: {total_mappers}')
print()

# Generate the script output
for c in changes:
    entries_str = ', '.join(f'{num}' for num, _, _, _ in c['entries'])
    print(f"  {c['basename']:30s} mappers: {entries_str}")
