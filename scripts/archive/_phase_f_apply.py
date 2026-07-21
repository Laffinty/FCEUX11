#!/usr/bin/env python3
"""Phase F: auto-register all unregistered bmap mappers.
Adds: Cart subclass decls, #include, MapperEntryRegister, test entries."""
import re, glob, os

os.chdir(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

# --- Parse bmap ---
with open('src/ines.cpp', encoding='utf-8', errors='replace') as f:
    ines = f.read()
bmap = {}
for m in re.finditer(r'^\s*\{"([^"]*)",\s*(\d+),\s*(\w+)', ines, re.MULTILINE):
    name, num, func = m.group(1), int(m.group(2)), m.group(3)
    if num > 0 or name:
        bmap[func] = (num, name or f'Mapper {num}')

# --- Find unregistered mappers ---
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

all_mappers.sort(key=lambda x: x[0])
print(f'Total mappers to register: {len(all_mappers)}')

# --- Step 1: Add Cart subclass declarations to simple_carts.h ---
simple_carts_path = 'src/boards/simple_carts.h'
with open(simple_carts_path, encoding='utf-8') as f:
    sc_content = f.read()

# Find the closing } // namespace fceu11
# and insert before it
new_decls = []
for num, name, func, src in all_mappers:
    if f'class Mapper{num}Cart' not in sc_content:
        new_decls.append(f'class Mapper{num}Cart  : public MapperStrategyA {{ public: explicit Mapper{num}Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {{}} }};')

if new_decls:
    # Insert before the last } // namespace fceu11
    insert_text = '// v1.8 Phase F: remaining P2 mapper Cart subclasses.\n'
    for decl in new_decls:
        insert_text += decl + '\n'
    sc_content = sc_content.replace('} // namespace fceu11', insert_text + '\n} // namespace fceu11')
    with open(simple_carts_path, 'w', encoding='utf-8') as f:
        f.write(sc_content)
    print(f'Added {len(new_decls)} Cart subclass declarations to simple_carts.h')

# --- Step 2: Add #include + MapperEntryRegister to each board file ---
from collections import defaultdict
by_file = defaultdict(list)
for num, name, func, src in all_mappers:
    by_file[src].append((num, name, func))

modified_files = 0
for cpp, entries in by_file.items():
    with open(cpp, encoding='utf-8', errors='replace') as f:
        content = f.read()
    
    # Add #include "simple_carts.h" if not present
    if 'simple_carts.h' not in content:
        # Find the last #include line
        last_include = 0
        for i, line in enumerate(content.split('\n')):
            if line.strip().startswith('#include'):
                last_include = i
        lines = content.split('\n')
        lines.insert(last_include + 1, '#include "simple_carts.h"          // v1.8 Phase F')
        content = '\n'.join(lines)
    
    # Build registration block
    reg_block = '\n// v1.8 Masonry Phase F: MapperEntryRegister.\n'
    reg_block += 'namespace fceu11 {\nnamespace {\n'
    for num, name, func in entries:
        reg_block += f'static MapperEntryRegister kMapper{num}Register{{\n'
        reg_block += f'    MapperEntry{{{num}, "{name}", &{func},\n'
        reg_block += f'        [](Bus& bus) {{ return std::make_unique<Mapper{num}Cart>(bus); }}}}\n'
        reg_block += f'}};\n'
    reg_block += '}  // namespace\n}  // namespace fceu11\n'
    
    # Append to end of file
    content = content.rstrip() + '\n' + reg_block
    
    with open(cpp, 'w', encoding='utf-8') as f:
        f.write(content)
    modified_files += 1

print(f'Modified {modified_files} board files')

# --- Step 3: Add test ROM entries to generate_test_roms.py ---
gen_path = 'tests/fixtures/generate_test_roms.py'
with open(gen_path, encoding='utf-8') as f:
    gen_content = f.read()

# Find the closing ] of the mapper list
# Add new entries before it
new_entries = []
for num, name, func, src in all_mappers:
    if f'("mapper{num}"' not in gen_content:
        safe_name = name.replace('"', '\\"')
        new_entries.append(f'    ("mapper{num}", {num}),   # {safe_name}')

if new_entries:
    # Find the last entry before ]
    insert_text = '    # Phase F: remaining P2 mappers.\n'
    for entry in new_entries:
        insert_text += entry + '\n'
    gen_content = gen_content.replace(']\n\nPRG_SIZE', insert_text + ']\n\nPRG_SIZE')
    with open(gen_path, 'w', encoding='utf-8') as f:
        f.write(gen_content)
    print(f'Added {len(new_entries)} test ROM entries to generate_test_roms.py')

# --- Step 4: Add test entries to mapper_byte_diff_test.cpp ---
test_path = 'tests/core/mapper_byte_diff_test.cpp'
with open(test_path, encoding='utf-8') as f:
    test_content = f.read()

new_test_entries = []
for num, name, func, src in all_mappers:
    if f'mapper_mapper{num}.nes' not in test_content:
        safe_name = name.replace('"', '\\"').replace('\\', '')
        new_test_entries.append(
            f'    {{ "fixtures/mapper_mapper{num}.nes",     "mapper{num}",     60  }},  // {safe_name}'
        )

if new_test_entries:
    # Insert before the mapper 83 (YOKO VRC) line or the closing };
    insert_text = '    // Phase F: remaining P2 mappers.\n'
    for entry in new_test_entries:
        insert_text += entry + '\n'
    
    # Find the YOKO VRC line or closing };
    if 'mapper 83 (YOKO VRC)' in test_content:
        test_content = test_content.replace(
            '    // mapper 83 (YOKO VRC) placed last',
            insert_text + '    // mapper 83 (YOKO VRC) placed last'
        )
    else:
        test_content = test_content.replace('\n};\n\nstatic const int NUM_TESTS', insert_text + '\n};\n\nstatic const int NUM_TESTS')
    
    with open(test_path, 'w', encoding='utf-8') as f:
        f.write(test_content)
    print(f'Added {len(new_test_entries)} test entries to mapper_byte_diff_test.cpp')

print('\nDone. Run: python tests/fixtures/generate_test_roms.py && cmake --build build --config Release')
