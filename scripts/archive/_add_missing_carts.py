import re, os, glob
os.chdir('D:/Project/FCEUX11')

# Find registered mappers that need Cart classes
registered = set()
for cpp in glob.glob('src/boards/*.cpp'):
    with open(cpp, encoding='utf-8', errors='replace') as f:
        content = f.read()
    for m in re.finditer(r'MapperEntry\{(\d+),', content):
        registered.add(int(m.group(1)))

# Check what's in simple_carts.h
with open('src/boards/simple_carts.h', encoding='utf-8') as f:
    sc = f.read()
existing = set(int(m.group(1)) for m in re.finditer(r'class Mapper(\d+)Cart', sc))

# Check other headers
other = set()
for header in glob.glob('src/boards/*_carts.h'):
    if 'simple_carts.h' in header:
        continue
    with open(header, encoding='utf-8', errors='replace') as f:
        for line in f:
            m = re.match(r'class Mapper(\d+)Cart', line)
            if m:
                other.add(int(m.group(1)))

missing = sorted(registered - existing - other)
print(f'Missing Cart classes: {len(missing)}')

if missing:
    decl = '// v1.8 Phase F: remaining Cart classes.\n'
    for num in missing:
        decl += f'class Mapper{num}Cart  : public MapperStrategyA {{ public: explicit Mapper{num}Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {{}} }};\n'
    sc = sc.replace('} // namespace fceu11', decl + '\n} // namespace fceu11')
    with open('src/boards/simple_carts.h', 'w', encoding='utf-8') as f:
        f.write(sc)
    print(f'Added {len(missing)} Cart classes')
