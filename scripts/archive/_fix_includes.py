#!/usr/bin/env python3
"""Fix includes for mappers whose Cart classes are in mmc3_variants_carts.h."""
import re, os, glob

os.chdir(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

# Find mappers with Cart classes in mmc3_variants_carts.h
mmc3_mappers = set()
with open('src/boards/mmc3_variants_carts.h', encoding='utf-8') as f:
    for line in f:
        m = re.match(r'class Mapper(\d+)Cart', line)
        if m:
            mmc3_mappers.add(int(m.group(1)))

# Find mappers with Cart classes in smb2j_carts.h
smb2j_mappers = set()
for header in glob.glob('src/boards/*_carts.h'):
    if 'simple_carts.h' in header or 'mmc3_variants' in header:
        continue
    with open(header, encoding='utf-8', errors='replace') as f:
        for line in f:
            m = re.match(r'class Mapper(\d+)Cart', line)
            if m:
                smb2j_mappers.add(int(m.group(1)))

print(f'MMC3 variant mappers: {sorted(mmc3_mappers)}')

# Fix board files that use MMC3 variant Cart classes
fixed = 0
for cpp in glob.glob('src/boards/*.cpp'):
    with open(cpp, encoding='utf-8', errors='replace') as f:
        content = f.read()
    
    if 'MapperEntryRegister' not in content:
        continue
    if 'simple_carts.h' not in content:
        continue
    
    # Check if this file uses any MMC3 variant Cart class
    uses_mmc3 = False
    for m in mmc3_mappers:
        if f'Mapper{m}Cart' in content:
            uses_mmc3 = True
            break
    
    if uses_mmc3:
        # Replace simple_carts.h with mmc3_variants_carts.h
        content = content.replace(
            '#include "simple_carts.h"          // v1.8 Phase F',
            '#include "boards/mmc3_variants_carts.h"  // v1.8 Phase F'
        )
        with open(cpp, 'w', encoding='utf-8') as f:
            f.write(content)
        fixed += 1
        print(f'  Fixed: {os.path.basename(cpp)} -> mmc3_variants_carts.h')

print(f'\nFixed {fixed} files')
