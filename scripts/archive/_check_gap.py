import re, glob, os
os.chdir('D:/Project/FCEUX11')
registered = set()
for cpp in glob.glob('src/boards/*.cpp'):
    with open(cpp, encoding='utf-8', errors='replace') as f:
        content = f.read()
    for m in re.finditer(r'MapperEntry\{(\d+),', content):
        registered.add(int(m.group(1)))
with open('tests/core/mapper_byte_diff_test.cpp', encoding='utf-8') as f:
    test = f.read()
tested = set(int(m.group(1)) for m in re.finditer(r'"mapper(\d+)"', test))
gap = sorted(registered - tested)
print(f'Registered: {len(registered)}, Tested: {len(tested)}, Gap: {len(gap)}')
print('Gap mappers:', gap)
