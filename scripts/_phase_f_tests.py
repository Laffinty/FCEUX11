#!/usr/bin/env python3
"""Add test entries for Phase F mappers."""
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

# Find which mappers are registered but not in test
import glob
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

# Find gap
to_test = sorted(registered - tested)
print(f'Mappers registered but not tested: {len(to_test)}')

# Add test ROM entries
with open('tests/fixtures/generate_test_roms.py', encoding='utf-8') as f:
    gen = f.read()
gen_new = gen.rstrip().rstrip(']')
gen_new += '\n    # Phase F: remaining mappers.\n'
for num in to_test:
    if f'("mapper{num}"' not in gen:
        gen_new += f'    ("mapper{num}",  {num}),\n'
gen_new += ']\n'
with open('tests/fixtures/generate_test_roms.py', 'w', encoding='utf-8') as f:
    f.write(gen_new)
print(f'Added {len(to_test)} test ROM entries')

# Add test entries
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
print(f'Added test entries')

print('Done')
