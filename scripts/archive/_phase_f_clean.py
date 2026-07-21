#!/usr/bin/env python3
"""Phase F: add all working mappers (excluding 178 which crashes)."""
import re, os, glob
from collections import defaultdict

os.chdir(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

SKIP = {178}  # crashes with ACCESS_VIOLATION

# Parse bmap
with open('src/ines.cpp', encoding='utf-8', errors='replace') as f:
    ines = f.read()
bmap = {}
for m in re.finditer(r'^\s*\{"([^"]*)",\s*(\d+),\s*(\w+)', ines, re.MULTILINE):
    name, num, func = m.group(1), int(m.group(2)), m.group(3)
    if num > 0 or name:
        bmap[num] = (name, func)

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
        for num, (name, bmap_func) in bmap.items():
            if bmap_func == func_name and num not in SKIP:
                to_add.append((num, name, func_name, cpp))
                break

to_add.sort(key=lambda x: x[0])
print(f'Mappers to add: {len(to_add)}')

# Add Cart classes (skip those in other headers)
other_headers = set()
for header in glob.glob('src/boards/*_carts.h'):
    if 'simple_carts.h' in header:
        continue
    with open(header, encoding='utf-8', errors='replace') as f:
        for line in f:
            m = re.match(r'class Mapper(\d+)Cart', line)
            if m:
                other_headers.add(int(m.group(1)))

with open('src/boards/simple_carts.h', encoding='utf-8') as f:
    sc = f.read()
existing_carts = set(int(m.group(1)) for m in re.finditer(r'class Mapper(\d+)Cart', sc))

new_decls = []
for num, name, func, cpp in to_add:
    if num not in existing_carts and num not in other_headers:
        new_decls.append(f'class Mapper{num}Cart  : public MapperStrategyA {{ public: explicit Mapper{num}Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {{}} }};')

if new_decls:
    decl = '// v1.8 Phase F: P2 mapper Cart subclasses.\n'
    for d in new_decls:
        decl += d + '\n'
    sc = sc.replace('} // namespace fceu11', decl + '\n} // namespace fceu11')
    with open('src/boards/simple_carts.h', 'w', encoding='utf-8') as f:
        f.write(sc)
    print(f'Added {len(new_decls)} Cart classes')

# Add registrations
by_file = defaultdict(list)
for num, name, func, cpp in to_add:
    by_file[cpp].append((num, name, func))

modified = 0
for cpp, entries in by_file.items():
    with open(cpp, encoding='utf-8', errors='replace') as f:
        content = f.read()
    if 'MapperEntryRegister' in content:
        continue
    
    include_needed = 'simple_carts.h'
    # Check if any mapper needs mmc3_variants_carts.h
    for num, _, _ in entries:
        if num in other_headers:
            include_needed = 'mmc3_variants_carts.h'
            break
    
    if include_needed not in content:
        last_include = 0
        for i, line in enumerate(content.split('\n')):
            if line.strip().startswith('#include'):
                last_include = i
        lines = content.split('\n')
        lines.insert(last_include + 1, f'#include "{include_needed}"          // v1.8 Phase F')
        content = '\n'.join(lines)
    
    if include_needed == 'mmc3_variants_carts.h' and 'simple_carts.h' not in content:
        last_include = 0
        for i, line in enumerate(content.split('\n')):
            if line.strip().startswith('#include'):
                last_include = i
        lines = content.split('\n')
        lines.insert(last_include + 1, '#include "simple_carts.h"          // v1.8 Phase F (registry)')
        content = '\n'.join(lines)
    
    reg = '\n// v1.8 Phase F: MapperEntryRegister.\nnamespace fceu11 {\nnamespace {\n'
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

# Add test entries
with open('tests/core/mapper_byte_diff_test.cpp', encoding='utf-8') as f:
    test = f.read()
new_test = '    // Phase F: P2 mappers.\n'
for num, _, _, _ in to_add:
    if f'mapper_mapper{num}.nes' not in test:
        new_test += f'    {{ "fixtures/mapper_mapper{num}.nes",     "mapper{num}",     60  }},\n'
test = test.replace('    // mapper 83 (YOKO VRC) placed last', new_test + '    // mapper 83 (YOKO VRC) placed last')
with open('tests/core/mapper_byte_diff_test.cpp', 'w', encoding='utf-8') as f:
    f.write(test)
print('Added test entries')

# Add ROM entries
with open('tests/fixtures/generate_test_roms.py', encoding='utf-8') as f:
    gen = f.read()
new_gen = '    # Phase F: P2 mappers.\n'
for num, _, _, _ in to_add:
    if f'("mapper{num}"' not in gen:
        new_gen += f'    ("mapper{num}",  {num}),\n'
gen = gen.replace(']\n\nPRG_SIZE', new_gen + ']\n\nPRG_SIZE')
with open('tests/fixtures/generate_test_roms.py', 'w', encoding='utf-8') as f:
    f.write(gen)
print('Added ROM entries')

print('Done')
