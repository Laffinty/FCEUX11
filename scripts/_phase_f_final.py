#!/usr/bin/env python3
"""Phase F: comprehensive mapper registration. Handles all edge cases."""
import re, os, glob
from collections import defaultdict

os.chdir(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

SKIP = {178, 20, 8, 39, 100, 101, 102, 104, 109, 110, 122, 124, 126, 127, 128, 129, 130, 131, 135, 158, 161, 169, 179, 182, 196, 197, 199, 200, 201, 202, 203, 204, 209, 211, 212, 213, 214, 217, 218, 223, 224, 227, 229, 231, 236, 237, 239, 247, 248, 251}  # Not in bmap or crashing

# Parse bmap: number -> (name, func)
with open('src/ines.cpp', encoding='utf-8', errors='replace') as f:
    ines = f.read()
bmap = {}
for m in re.finditer(r'^\s*\{"([^"]*)",\s*(\d+),\s*(\w+)', ines, re.MULTILINE):
    name, num, func = m.group(1), int(m.group(2)), m.group(3)
    if num > 0 or name:
        bmap[num] = (name, func)

# Find mappers declared in other headers
other_headers = set()
for header in glob.glob('src/boards/*_carts.h'):
    if 'simple_carts.h' in header:
        continue
    with open(header, encoding='utf-8', errors='replace') as f:
        for line in f:
            m = re.match(r'class Mapper(\d+)Cart', line)
            if m:
                other_headers.add(int(m.group(1)))

# Find all Init functions in board files
func_to_file = {}
for cpp in sorted(glob.glob('src/boards/*.cpp')):
    with open(cpp, encoding='utf-8', errors='replace') as f:
        content = f.read()
    for m in re.finditer(r'void\s+(\w+_Init)\s*\(\s*CartInfo', content):
        func_to_file[m.group(1)] = cpp

# Build list of mappers to register
to_add = []
for num, (name, func) in bmap.items():
    if num in SKIP:
        continue
    if func not in func_to_file:
        continue
    cpp = func_to_file[func]
    basename = os.path.basename(cpp)
    if basename.startswith('_') or basename in ('registry.cpp', 'mmc3_base_cart.cpp'):
        continue
    # Check if already registered
    with open(cpp, encoding='utf-8', errors='replace') as f:
        content = f.read()
    if 'MapperEntryRegister' in content:
        continue
    to_add.append((num, name, func, cpp))

to_add.sort(key=lambda x: x[0])
print(f'Mappers to register: {len(to_add)}')

# Step 1: Add Cart classes to simple_carts.h
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

# Step 2: Add registrations to board files
by_file = defaultdict(list)
for num, name, func, cpp in to_add:
    by_file[cpp].append((num, name, func))

modified = 0
for cpp, entries in by_file.items():
    with open(cpp, encoding='utf-8', errors='replace') as f:
        content = f.read()
    
    # Determine which include is needed
    needs_mmc3 = any(n in other_headers for n, _, _ in entries)
    
    # Add includes
    if needs_mmc3 and 'mmc3_variants_carts.h' not in content:
        last_include = 0
        for i, line in enumerate(content.split('\n')):
            if line.strip().startswith('#include'):
                last_include = i
        lines = content.split('\n')
        lines.insert(last_include + 1, '#include "mmc3_variants_carts.h"  // v1.8 Phase F')
        content = '\n'.join(lines)
    
    if 'simple_carts.h' not in content:
        last_include = 0
        for i, line in enumerate(content.split('\n')):
            if line.strip().startswith('#include'):
                last_include = i
        lines = content.split('\n')
        lines.insert(last_include + 1, '#include "simple_carts.h"          // v1.8 Phase F')
        content = '\n'.join(lines)
    
    # Build registration block
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

# Step 3: Add test entries
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

# Step 4: Add ROM entries
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
