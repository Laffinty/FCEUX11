#!/usr/bin/env python3
"""Phase F: add mappers in small batches to isolate crash."""
import re, os

os.chdir(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

# Parse bmap
with open('src/ines.cpp', encoding='utf-8', errors='replace') as f:
    ines = f.read()
bmap = {}
for m in re.finditer(r'^\s*\{"([^"]*)",\s*(\d+),\s*(\w+)', ines, re.MULTILINE):
    name, num, func = m.group(1), int(m.group(2)), m.group(3)
    if num > 0 or name:
        bmap[func] = (num, name)

# Simple mappers to add (known safe: single file, no complex deps)
batch = [
    ('src/boards/103.cpp', 'Mapper103_Init'),
    ('src/boards/106.cpp', 'Mapper106_Init'),
    ('src/boards/108.cpp', 'Mapper108_Init'),
    ('src/boards/112.cpp', 'Mapper112_Init'),
    ('src/boards/117.cpp', 'Mapper117_Init'),
    ('src/boards/120.cpp', 'Mapper120_Init'),
    ('src/boards/121.cpp', 'Mapper121_Init'),
    ('src/boards/151.cpp', 'Mapper151_Init'),
    ('src/boards/156.cpp', 'Mapper156_Init'),
    ('src/boards/177.cpp', 'Mapper177_Init'),
]

# Add Cart class declarations to simple_carts.h
with open('src/boards/simple_carts.h', encoding='utf-8') as f:
    sc = f.read()

new_decls = []
for cpp, func in batch:
    if func in bmap:
        num, name = bmap[func]
        if f'class Mapper{num}Cart' not in sc:
            new_decls.append(f'class Mapper{num}Cart  : public MapperStrategyA {{ public: explicit Mapper{num}Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {{}} }};')

if new_decls:
    decl = '// v1.8 Phase F batch 1.\n'
    for d in new_decls:
        decl += d + '\n'
    sc = sc.replace('} // namespace fceu11', decl + '\n} // namespace fceu11')
    with open('src/boards/simple_carts.h', 'w', encoding='utf-8') as f:
        f.write(sc)
    print(f'Added {len(new_decls)} Cart classes')

# Add includes + registrations to board files
for cpp, func in batch:
    if func not in bmap:
        continue
    num, name = bmap[func]
    
    with open(cpp, encoding='utf-8', errors='replace') as f:
        content = f.read()
    
    if 'MapperEntryRegister' in content:
        continue
    
    # Add include
    if 'simple_carts.h' not in content:
        last_include = 0
        for i, line in enumerate(content.split('\n')):
            if line.strip().startswith('#include'):
                last_include = i
        lines = content.split('\n')
        lines.insert(last_include + 1, '#include "simple_carts.h"          // v1.8 Phase F')
        content = '\n'.join(lines)
    
    # Add registration
    reg = f'\n// v1.8 Phase F: MapperEntryRegister.\nnamespace fceu11 {{\nnamespace {{\n'
    reg += f'static MapperEntryRegister kMapper{num}Register{{\n'
    reg += f'    MapperEntry{{{num}, "{name}", &{func},\n'
    reg += f'        [](Bus& bus) {{ return std::make_unique<Mapper{num}Cart>(bus); }}}}\n'
    reg += f'}};\n}}  // namespace\n}}  // namespace fceu11\n'
    
    content = content.rstrip() + '\n' + reg
    with open(cpp, 'w', encoding='utf-8') as f:
        f.write(content)
    print(f'  {os.path.basename(cpp)}: mapper {num}')

print('Done')
