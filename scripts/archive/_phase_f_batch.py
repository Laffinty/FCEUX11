#!/usr/bin/env python3
"""Add Phase F mappers in a specific batch. Usage: _phase_f_add_batch.py <start> <end>"""
import re, os, glob, sys

os.chdir(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

start = int(sys.argv[1]) if len(sys.argv) > 1 else 0
end = int(sys.argv[2]) if len(sys.argv) > 2 else 999

# Parse bmap
with open('src/ines.cpp', encoding='utf-8', errors='replace') as f:
    ines = f.read()
bmap = {}
for m in re.finditer(r'^\s*\{"([^"]*)",\s*(\d+),\s*(\w+)', ines, re.MULTILINE):
    name, num, func = m.group(1), int(m.group(2)), m.group(3)
    if num > 0 or name:
        bmap[func] = (num, name)

# Find unregistered mappers
to_add = []
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
            num, name = bmap[func_name]
            to_add.append((num, name, func_name, cpp))

to_add.sort(key=lambda x: x[0])
batch = to_add[start:end]
print(f'Adding mappers {start}-{end} ({len(batch)} mappers)')

# Add Cart classes
with open('src/boards/simple_carts.h', encoding='utf-8') as f:
    sc = f.read()
existing_carts = set(int(m.group(1)) for m in re.finditer(r'class Mapper(\d+)Cart', sc))
new_decls = []
for num, name, func, cpp in batch:
    if num not in existing_carts:
        new_decls.append(f'class Mapper{num}Cart  : public MapperStrategyA {{ public: explicit Mapper{num}Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {{}} }};')
if new_decls:
    decl = f'// v1.8 Phase F batch {start}-{end}.\n'
    for d in new_decls:
        decl += d + '\n'
    sc = sc.replace('} // namespace fceu11', decl + '\n} // namespace fceu11')
    with open('src/boards/simple_carts.h', 'w', encoding='utf-8') as f:
        f.write(sc)
    print(f'Added {len(new_decls)} Cart classes')

# Add registrations
from collections import defaultdict
by_file = defaultdict(list)
for num, name, func, cpp in batch:
    by_file[cpp].append((num, name, func))

modified = 0
for cpp, entries in by_file.items():
    with open(cpp, encoding='utf-8', errors='replace') as f:
        content = f.read()
    if 'MapperEntryRegister' in content:
        continue
    if 'simple_carts.h' not in content:
        last_include = 0
        for i, line in enumerate(content.split('\n')):
            if line.strip().startswith('#include'):
                last_include = i
        lines = content.split('\n')
        lines.insert(last_include + 1, '#include "simple_carts.h"          // v1.8 Phase F')
        content = '\n'.join(lines)
    reg = f'\n// v1.8 Phase F batch {start}-{end}.\nnamespace fceu11 {{\nnamespace {{\n'
    for num, name, func in entries:
        reg += f'static MapperEntryRegister kMapper{num}Register{{\n'
        reg += f'    MapperEntry{{{num}, "{name}", &{func},\n'
        reg += f'        [](Bus& bus) {{ return std::make_unique<Mapper{num}Cart>(bus); }}}}\n'
        reg += f'}};\n'
    reg += '}  // namespace\n}  // namespace fceu11\n'
    content = content.rstrip() + '\n' + reg
    with open(cpp, 'w', encoding='utf-8') as f:
        f.write(content)
    modified += 1

print(f'Modified {modified} board files')
