#!/usr/bin/env python3
"""Phase F: add a small batch of mappers. Usage: _add_small_batch.py <mapper_numbers...>"""
import re, os, sys, glob

os.chdir(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
mappers = [int(x) for x in sys.argv[1:]]

# Parse bmap
with open('src/ines.cpp', encoding='utf-8', errors='replace') as f:
    ines = f.read()
bmap = {}
for m in re.finditer(r'^\s*\{"([^"]*)",\s*(\d+),\s*(\w+)', ines, re.MULTILINE):
    name, num, func = m.group(1), int(m.group(2)), m.group(3)
    if num > 0 or name:
        bmap[num] = (name, func)

# Find which source file has each Init function
func_to_file = {}
for cpp in sorted(glob.glob('src/boards/*.cpp')):
    with open(cpp, encoding='utf-8', errors='replace') as f:
        content = f.read()
    for m in re.finditer(r'void\s+(\w+_Init)\s*\(\s*CartInfo', content):
        func_to_file[m.group(1)] = cpp

# Add Cart classes to simple_carts.h
with open('src/boards/simple_carts.h', encoding='utf-8') as f:
    sc = f.read()
existing = set(int(m.group(1)) for m in re.finditer(r'class Mapper(\d+)Cart', sc))

new_decls = []
for num in mappers:
    if num not in existing and num in bmap:
        new_decls.append(f'class Mapper{num}Cart  : public MapperStrategyA {{ public: explicit Mapper{num}Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {{}} }};')

if new_decls:
    decl = '// v1.8 Phase F incremental.\n'
    for d in new_decls:
        decl += d + '\n'
    sc = sc.replace('} // namespace fceu11', decl + '\n} // namespace fceu11')
    with open('src/boards/simple_carts.h', 'w', encoding='utf-8') as f:
        f.write(sc)
    print(f'Added {len(new_decls)} Cart classes')

# Add registrations to board files
from collections import defaultdict
by_file = defaultdict(list)
for num in mappers:
    if num in bmap:
        name, func = bmap[num]
        if func in func_to_file:
            by_file[func_to_file[func]].append((num, name, func))

for cpp, entries in by_file.items():
    with open(cpp, encoding='utf-8', errors='replace') as f:
        content = f.read()
    if 'MapperEntryRegister' in content and 'Phase F incremental' in content:
        # Already has Phase F registration, append
        pass
    elif 'MapperEntryRegister' in content:
        # Has registration from Phase E, skip
        continue
    
    if 'simple_carts.h' not in content:
        last_include = 0
        for i, line in enumerate(content.split('\n')):
            if line.strip().startswith('#include'):
                last_include = i
        lines = content.split('\n')
        lines.insert(last_include + 1, '#include "simple_carts.h"          // v1.8 Phase F')
        content = '\n'.join(lines)
    
    reg = '\n// v1.8 Phase F incremental.\nnamespace fceu11 {\nnamespace {\n'
    for num, name, func in entries:
        reg += f'static MapperEntryRegister kMapper{num}Register{{\n'
        reg += f'    MapperEntry{{{num}, "{name}", &{func},\n'
        reg += f'        [](Bus& bus) {{ return std::make_unique<Mapper{num}Cart>(bus); }}}}\n'
        reg += f'}};\n'
    reg += '}  // namespace\n}  // namespace fceu11\n'
    content = content.rstrip() + '\n' + reg
    with open(cpp, 'w', encoding='utf-8') as f:
        f.write(content)
    print(f'  {os.path.basename(cpp)}: mappers {[n for n,_,_ in entries]}')

# Add test entries
with open('tests/core/mapper_byte_diff_test.cpp', encoding='utf-8') as f:
    test = f.read()
new_test = ''
for num in mappers:
    if f'mapper_mapper{num}.nes' not in test:
        new_test += f'    {{ "fixtures/mapper_mapper{num}.nes",     "mapper{num}",     60  }},\n'
if new_test:
    test = test.replace('    // mapper 83 (YOKO VRC) placed last', new_test + '    // mapper 83 (YOKO VRC) placed last')
    with open('tests/core/mapper_byte_diff_test.cpp', 'w', encoding='utf-8') as f:
        f.write(test)
    print(f'Added test entries')

# Add to generate_test_roms.py
with open('tests/fixtures/generate_test_roms.py', encoding='utf-8') as f:
    gen = f.read()
new_gen = ''
for num in mappers:
    if f'("mapper{num}"' not in gen:
        new_gen += f'    ("mapper{num}",  {num}),\n'
if new_gen:
    gen = gen.replace(']\n\nPRG_SIZE', new_gen + ']\n\nPRG_SIZE')
    with open('tests/fixtures/generate_test_roms.py', 'w', encoding='utf-8') as f:
        f.write(gen)
    print(f'Added ROM entries')

print('Done')
