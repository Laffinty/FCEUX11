#!/usr/bin/env python3
"""Phase F audit: find which board files have Init functions but no MapperEntryRegister."""
import re, glob, os

os.chdir(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

results = []
for cpp in sorted(glob.glob('src/boards/*.cpp')):
    with open(cpp, encoding='utf-8', errors='replace') as f:
        content = f.read()
    
    # Skip helper/utility files
    basename = os.path.basename(cpp)
    if basename.startswith('_') or basename.startswith('__'):
        continue
    if basename in ('registry.cpp', 'mmc3_base_cart.cpp'):
        continue
    
    # Find Init functions (void Xxx_Init(CartInfo*))
    inits = re.findall(r'void\s+(\w+)_Init\s*\(\s*CartInfo\s*\*', content)
    if not inits:
        continue
    
    # Check if already has MapperEntryRegister
    has_reg = 'MapperEntryRegister' in content
    
    if not has_reg:
        results.append((basename, inits))

print(f'Board files with Init functions but NO MapperEntryRegister: {len(results)}')
print()
for basename, inits in results:
    print(f'  {basename:30s} -> {", ".join(inits)}')
