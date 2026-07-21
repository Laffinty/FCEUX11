import re, os, glob
os.chdir('D:/Project/FCEUX11')

# Find registered mappers
registered = set()
for cpp in glob.glob('src/boards/*.cpp'):
    with open(cpp, encoding='utf-8', errors='replace') as f:
        content = f.read()
    for m in re.finditer(r'MapperEntry\{(\d+),', content):
        registered.add(int(m.group(1)))

# Check what's in the test (by number and by name)
with open('tests/core/mapper_byte_diff_test.cpp', encoding='utf-8') as f:
    test = f.read()
tested = set()
for m in re.finditer(r'"mapper(\d+)"', test):
    tested.add(int(m.group(1)))
# Named mappers
named = {0:'nrom',1:'mmc1',2:'uxrom',3:'cnrom',4:'mmc3',5:'mmc5',7:'axrom',
         9:'mmc2',10:'mmc4',11:'colordreams',13:'cprom',16:'bandai',
         26:'vrc6',66:'gnrom',85:'vrc7'}
for num, name in named.items():
    if f'"{name}"' in test:
        tested.add(num)

gap = sorted(registered - tested)
print(f'Gap: {len(gap)} mappers')

# Add to generate_test_roms.py
with open('tests/fixtures/generate_test_roms.py', encoding='utf-8') as f:
    gen = f.read()
new_entries = ''
for num in gap:
    if f'("mapper{num}"' not in gen:
        new_entries += f'    ("mapper{num}",  {num}),\n'
if new_entries:
    gen = gen.replace(']\n\nPRG_SIZE', '    # Phase F remaining.\n' + new_entries + ']\n\nPRG_SIZE')
    with open('tests/fixtures/generate_test_roms.py', 'w', encoding='utf-8') as f:
        f.write(gen)
    print(f'Added {len(gap)} entries to generate_test_roms.py')

# Add to mapper_byte_diff_test.cpp
new_test = '    // Phase F remaining.\n'
for num in gap:
    if f'mapper_mapper{num}.nes' not in test:
        new_test += f'    {{ "fixtures/mapper_mapper{num}.nes",     "mapper{num}",     60  }},\n'
test = test.replace('    // mapper 83 (YOKO VRC) placed last', new_test + '    // mapper 83 (YOKO VRC) placed last')
with open('tests/core/mapper_byte_diff_test.cpp', 'w', encoding='utf-8') as f:
    f.write(test)
print('Added test entries')
