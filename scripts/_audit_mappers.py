#!/usr/bin/env python3
"""Audit: find active bmap[] mappers not yet registered in MapperEntry registry."""
import re, glob, os

os.chdir(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

# Parse active bmap entries from ines.cpp
with open('src/ines.cpp', encoding='utf-8', errors='replace') as f:
    ines = f.read()
bmap = {}
for m in re.finditer(r'^\s*\{["\']([^"\']*)["\'],\s*(\d+),\s*(\w+)', ines, re.MULTILINE):
    name, num, func = m.group(1), int(m.group(2)), m.group(3)
    if num > 0 or name:  # skip sentinel {0, NULL}
        bmap[num] = (name, func)

# Parse registered mappers from board files
registered = set()
for cpp in glob.glob('src/boards/*.cpp'):
    with open(cpp, encoding='utf-8', errors='replace') as f:
        content = f.read()
    for m in re.finditer(r'MapperEntry\{(\d+),', content):
        registered.add(int(m.group(1)))
# Special registration in registry.cpp (mapper 406 fallback)
with open('src/boards/registry.cpp', encoding='utf-8', errors='replace') as f:
    reg = f.read()
for m in re.finditer(r'mapper_number\s*=\s*(\d+)', reg):
    registered.add(int(m.group(1)))

# Find gap
gap = sorted(set(bmap.keys()) - registered)
print(f'Active bmap entries: {len(bmap)}')
print(f'Registered mappers:  {len(registered)}')
print(f'GAP (need registration): {len(gap)}')
print()
for n in gap:
    name, func = bmap[n]
    print(f'  {n:3d}: {name or "(unnamed)":30s} -> {func}')
