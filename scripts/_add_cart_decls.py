#!/usr/bin/env python3
"""Re-add Phase F Cart class declarations to simple_carts.h, excluding duplicates."""
import re, os, glob

os.chdir(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

# Find mappers declared in OTHER headers
other_headers = set()
for header in glob.glob('src/boards/*_carts.h'):
    if 'simple_carts.h' in header:
        continue
    with open(header, encoding='utf-8', errors='replace') as f:
        for line in f:
            m = re.match(r'class Mapper(\d+)Cart', line)
            if m:
                other_headers.add(int(m.group(1)))

# Parse bmap
with open('src/ines.cpp', encoding='utf-8', errors='replace') as f:
    ines = f.read()
bmap = {}
for m in re.finditer(r'^\s*\{"([^"]*)",\s*(\d+),\s*(\w+)', ines, re.MULTILINE):
    name, num, func = m.group(1), int(m.group(2)), m.group(3)
    if num > 0 or name:
        bmap[func] = (num, name)

# Find which mappers need Cart classes
needed = set()
for cpp in glob.glob('src/boards/*.cpp'):
    with open(cpp, encoding='utf-8', errors='replace') as f:
        content = f.read()
    if 'MapperEntryRegister' not in content:
        continue
    for m in re.finditer(r'Mapper(\d+)Cart', content):
        num = int(m.group(1))
        if num not in other_headers:
            needed.add(num)

# Check what's already in simple_carts.h
with open('src/boards/simple_carts.h', encoding='utf-8') as f:
    sc = f.read()
existing = set()
for m in re.finditer(r'class Mapper(\d+)Cart', sc):
    existing.add(int(m.group(1)))

# Add missing
missing = sorted(needed - existing)
print(f'Need to add {len(missing)} Cart classes to simple_carts.h')

if missing:
    decl = '// v1.8 Phase F: P2 mapper Cart subclasses.\n'
    for num in missing:
        decl += f'class Mapper{num}Cart  : public MapperStrategyA {{ public: explicit Mapper{num}Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {{}} }};\n'
    
    sc = sc.replace('} // namespace fceu11', decl + '\n} // namespace fceu11')
    with open('src/boards/simple_carts.h', 'w', encoding='utf-8') as f:
        f.write(sc)
    print(f'Added {len(missing)} declarations')
