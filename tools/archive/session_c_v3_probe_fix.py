import sys
from pathlib import Path
FILE = Path('src/ppu_rust_bridge.cpp')
text = FILE.read_text(encoding='utf-8')

OLD = (
    "    // Session C v3 (PPU/CPU phase investigation): dump every $2002\n"
    "    // read with (PC, addr, PPU sl, dot, CPU cycle). Env-gated by\n"
    "    // FCEUX11_PPU_PHASE_TRACE=1; silent otherwise. This is the\n"
    "    // trace that exposes whether the BROKEN 281 games are hitting\n"
    "    // the A2002 windows at positions the C++ engine never would.\n"
    "    if (addr == 0x2002) {\n"
    "        static const bool on = []() {\n"
    "            const char* e = std::getenv(\"FCEUX11_PPU_PHASE_TRACE\");\n"
    "            return e && e[0] == '1' && e[1] == '\\0';\n"
    "        }();\n"
    "        if (on) {\n"
    "            extern uint16_t X6502_GetReg(int reg);\n"
    "            const int16_t sl = fceux11_ppu_get_scanline(g_ppu_state);\n"
    "            const uint16_t dot = fceux11_ppu_get_dot(g_ppu_state);\n"
    "            const uint16_t pc = X6502_GetReg(0); // reg 0 = PC\n"
    "            const uint64 now = fceu11::cpu_instance().timestamp_base()\n"
    "                + static_cast<uint64>(fceu11::cpu_instance().timestamp_ref());\n"
    "            std::fprintf(stderr,\n"
    "                \"R3 P2002_READ abs=%llu sl=%d dot=%d pc=0x%04X\\n\",\n"
    "                (unsigned long long)now, (int)sl, (int)dot,\n"
    "                (unsigned)pc);\n"
    "        }\n"
    "    }\n"
)
NEW = (
    "    // Session C v3 (PPU/CPU phase investigation): dump every $2002\n"
    "    // read with (last PC, PPU sl, dot, CPU cycle). Env-gated by\n"
    "    // FCEUX11_PPU_PHASE_TRACE=1; silent otherwise. fceu11_e1_last_pc()\n"
    "    // gives the PC of the last instruction that was dispatched by\n"
    "    // the Cpu facade (so the $2002 read in this instruction is the\n"
    "    // next CPU op to run). The trace lets us compare the Rust PPU's\n"
    "    // (sl, dot) landing for the same instruction against the C++\n"
    "    // engine's E1 P2002_READ stream.\n"
    "    if (addr == 0x2002) {\n"
    "        static const bool on = []() {\n"
    "            const char* e = std::getenv(\"FCEUX11_PPU_PHASE_TRACE\");\n"
    "            return e && e[0] == '1' && e[1] == '\\0';\n"
    "        }();\n"
    "        if (on) {\n"
    "            const int16_t sl = fceux11_ppu_get_scanline(g_ppu_state);\n"
    "            const uint16_t dot = fceux11_ppu_get_dot(g_ppu_state);\n"
    "            const uint16_t pc = fceu11_e1_last_pc();\n"
    "            const uint64 now = fceu11::cpu_instance().timestamp_base()\n"
    "                + static_cast<uint64>(fceu11::cpu_instance().timestamp_ref());\n"
    "            std::fprintf(stderr,\n"
    "                \"R3 P2002_READ abs=%llu sl=%d dot=%d pc=0x%04X\\n\",\n"
    "                (unsigned long long)now, (int)sl, (int)dot,\n"
    "                (unsigned)pc);\n"
    "        }\n"
    "    }\n"
)

if NEW in text:
    print('already applied')
    sys.exit(0)
if OLD not in text:
    print('ERROR: old not found')
    sys.exit(1)
text = text.replace(OLD, NEW)
FILE.write_text(text, encoding='utf-8')
print('ok, new size:', FILE.stat().st_size)
