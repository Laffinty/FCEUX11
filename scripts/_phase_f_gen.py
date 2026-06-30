#!/usr/bin/env python3
"""Phase F: batch-add MapperEntryRegister + Cart subclass declarations."""
import re, glob, os

os.chdir(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

# Parse bmap from ines.cpp
with open('src/ines.cpp', encoding='utf-8', errors='replace') as f:
    ines = f.read()
bmap = {}
for m in re.finditer(r'^\s*\{"([^"]*)",\s*(\d+),\s*(\w+)', ines, re.MULTILINE):
    name, num, func = m.group(1), int(m.group(2)), m.group(3)
    if num > 0 or name:
        bmap[func] = (num, name or f'Mapper {num}')

# Collect all mappers to register
all_mappers = []  # (number, display_name, init_func, source_file)
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
    inits = re.findall(r'void\s+(\w+_Init)\s*\(\s*CartInfo\s*\*', content)
    for func_name in inits:
        if func_name in bmap:
            num, display_name = bmap[func_name]
            all_mappers.append((num, display_name, func_name, cpp))

# Sort by mapper number
all_mappers.sort(key=lambda x: x[0])

# --- Step 1: Generate Cart subclass declarations for simple_carts.h ---
cart_decls = []
for num, name, func, src in all_mappers:
    cart_decls.append(f'class Mapper{num}Cart  : public MapperStrategyA {{ public: explicit Mapper{num}Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {{}} }};')

print(f'=== Cart subclass declarations for simple_carts.h ({len(cart_decls)} classes) ===')
for decl in cart_decls:
    print(f'  {decl}')

# --- Step 2: Generate registration blocks per file ---
print()
print(f'=== Registration blocks ({len(all_mappers)} mappers across {len(set(src for _,_,_,src in all_mappers))} files) ===')

# Group by file
from collections import defaultdict
by_file = defaultdict(list)
for num, name, func, src in all_mappers:
    by_file[src].append((num, name, func))

for cpp in sorted(by_file.keys()):
    entries = by_file[cpp]
    print(f'\n  --- {os.path.basename(cpp)} ---')
    for num, name, func in entries:
        print(f'    static MapperEntryRegister kMapper{num}Register{{')
        print(f'        MapperEntry{{{num}, "{name}", &{func},')
        print(f'            [](Bus& bus) {{ return std::make_unique<Mapper{num}Cart>(bus); }}}}')
        print(f'    }};')
