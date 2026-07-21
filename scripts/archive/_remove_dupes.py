#!/usr/bin/env python3
"""Remove duplicate Cart class declarations from simple_carts.h."""
import re, os, glob

os.chdir(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

# Get mappers already declared in mmc3_variants_carts.h
mmc3_variants = set()
with open('src/boards/mmc3_variants_carts.h', encoding='utf-8') as f:
    for line in f:
        m = re.match(r'class Mapper(\d+)Cart', line)
        if m:
            mmc3_variants.add(int(m.group(1)))

# Also check smb2j_carts.h and other headers
for header in glob.glob('src/boards/*_carts.h'):
    if header == 'src/boards/simple_carts.h':
        continue
    with open(header, encoding='utf-8', errors='replace') as f:
        for line in f:
            m = re.match(r'class Mapper(\d+)Cart', line)
            if m:
                mmc3_variants.add(int(m.group(1)))

print(f'Mappers already declared in other headers: {sorted(mmc3_variants)}')

# Remove duplicates from simple_carts.h
with open('src/boards/simple_carts.h', encoding='utf-8') as f:
    lines = f.readlines()

new_lines = []
removed = 0
for line in lines:
    m = re.match(r'class Mapper(\d+)Cart', line)
    if m and int(m.group(1)) in mmc3_variants:
        removed += 1
        continue
    new_lines.append(line)

with open('src/boards/simple_carts.h', 'w', encoding='utf-8') as f:
    f.writelines(new_lines)

print(f'Removed {removed} duplicate declarations from simple_carts.h')
