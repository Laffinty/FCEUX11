#!/usr/bin/env python3
"""Add ALL Phase F test entries to generate_test_roms.py and mapper_byte_diff_test.cpp."""
import re, os, glob

os.chdir(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

# Parse bmap
with open('src/ines.cpp', encoding='utf-8', errors='replace') as f:
    ines = f.read()
bmap = {}
for m in re.finditer(r'^\s*\{"([^"]*)",\s*(\d+),\s*(\w+)', ines, re.MULTILINE):
    name, num, func = m.group(1), int(m.group(2)), m.group(3)
    if num > 0 or name:
        bmap[func] = (num, name)

# Find registered mappers
registered = set()
for cpp in glob.glob('src/boards/*.cpp'):
    with open(cpp, encoding='utf-8', errors='replace') as f:
        content = f.read()
    for m in re.finditer(r'MapperEntry\{(\d+),', content):
        registered.add(int(m.group(1)))

# Check what's in the test
with open('tests/core/mapper_byte_diff_test.cpp', encoding='utf-8') as f:
    test_content = f.read()
tested = set()
for m in re.finditer(r'"mapper(\d+)"', test_content):
    tested.add(int(m.group(1)))

to_test = sorted(registered - tested)
print(f'Mappers to add to test: {len(to_test)}')

# --- Fix generate_test_roms.py ---
with open('tests/fixtures/generate_test_roms.py', encoding='utf-8') as f:
    gen = f.read()

# Find the closing ] and PRG_SIZE line
# The list ends with ] followed by PRG_SIZE
gen = gen.replace(
    '    ("mapper59",    59),   # active in bmap[], audit gap\n]\n\nPRG_SIZE',
    '    ("mapper59",    59),   # active in bmap[], audit gap\n'
    '    # Phase F: remaining mappers.\n'
    + ''.join(f'    ("mapper{n}",  {n}),\n' for n in to_test)
    + ']\n\nPRG_SIZE'
)

with open('tests/fixtures/generate_test_roms.py', 'w', encoding='utf-8') as f:
    f.write(gen)
print(f'Added {len(to_test)} entries to generate_test_roms.py')

# --- Fix mapper_byte_diff_test.cpp ---
with open('tests/core/mapper_byte_diff_test.cpp', encoding='utf-8') as f:
    test = f.read()

new_entries = '    // Phase F: remaining mappers.\n'
for num in to_test:
    if f'mapper_mapper{num}.nes' not in test:
        new_entries += f'    {{ "fixtures/mapper_mapper{num}.nes",     "mapper{num}",     60  }},\n'

test = test.replace(
    '    // mapper 83 (YOKO VRC) placed last',
    new_entries + '    // mapper 83 (YOKO VRC) placed last'
)

with open('tests/core/mapper_byte_diff_test.cpp', 'w', encoding='utf-8') as f:
    f.write(test)
print(f'Added test entries to mapper_byte_diff_test.cpp')

print('Done')
