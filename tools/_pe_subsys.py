import struct, sys

for path in [r"build-rust-cpu\tests\kagami_qa_cycle_trace.exe",
             r"build-off\tests\kagami_qa_cycle_trace.exe"]:
    with open(path, "rb") as f:
        data = f.read()
    pe_off = struct.unpack_from("<I", data, 0x3C)[0]
    subsys = struct.unpack_from("<H", data, pe_off + 0x5C)[0]
    machine = struct.unpack_from("<H", data, pe_off + 4)[0]
    print(f"{path}: subsystem={subsys} (2=GUI,3=CUI) machine={machine:04X}")
