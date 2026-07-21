#!/usr/bin/env python3
"""Remove duplicate Cart class declarations from simple_carts.h.
Only removes classes already declared in OTHER headers (mmc3_variants_carts.h, etc.)."""
import re, os, glob

os.chdir(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

# Get mappers declared in OTHER headers (not simple_carts.h)
other_headers = set()
for header in glob.glob('src/boards/*_carts.h'):
    if 'simple_carts.h' in header:
        continue
    with open(header, encoding='utf-8', errors='replace') as f:
        for line in f:
            m = re.match(r'class Mapper(\d+)Cart', line)
            if m:
                other_headers.add(int(m.group(1)))

print(f'Mappers declared in other *_carts.h headers: {sorted(other_headers)}')

# Read simple_carts.h
with open('src/boards/simple_carts.h', encoding='utf-8') as f:
    lines = f.readlines()

new_lines = []
removed = []
for line in lines:
    m = re.match(r'class Mapper(\d+)Cart', line)
    if m and int(m.group(1)) in other_headers:
        removed.append(int(m.group(1)))
        continue
    new_lines.append(line)

with open('src/boards/simple_carts.h', 'w', encoding='utf-8') as f:
    f.writelines(new_lines)

print(f'Removed {len(removed)} duplicate declarations: {sorted(removed)}')
