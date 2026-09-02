import sys
from pathlib import Path
FILE = Path('src/ppu_rust_bridge.cpp')
text = FILE.read_text(encoding='utf-8')

# Find the location: just before `return fceux11_ppu_cpu_read(g_ppu_state, ...);`
TARGET = (
    "        } else if (sl == 241 && dot == 1) {\n"
    "            // At the VBL set dot (241, 1): the read returns\n"
    "            // VBL=1 (the set just happened) and clears the flag,\n"
    "            // AND we pull /NMI back up before the CPU samples it.\n"
    "            // Mirrors `X6502_IRQEnd(FCEU_IQNMI)` in src/ppu.cpp:630.\n"
    "            // (241, 2) read at \"1 dot after set\" is intentionally\n"
    "            // NOT included \u2014 the C++ engine's rcy <= 1 maps to\n"
    "            // (241, 0) and (241, 1) in C++ layout, which translate to\n"
    "            // (241, 1) and (241, 2) in Rust VBL-first. We narrow to\n"
    "            // (241, 1) only because the batch-compat regression in\n"
    "            // Session C's first attempt showed (241, 2) was being\n"
    "            // triggered by NMI handler $2002 reads at the start of\n"
    "            // the VBL block, cancelling in-flight NMIs and stalling\n"
    "            // games whose NMI handler returns 1 PPU dot after the\n"
    "            // set dot.\n"
    "            (void)fceux11_ppu_take_nmi_pending(g_ppu_state);\n"
    "        }\n"
    "    }\n"
    "    return fceux11_ppu_cpu_read(g_ppu_state, static_cast<uint16_t>(addr));\n"
)

# Insert a temporary trace just before the return
PROBE_INSERT = (
    "        }\n"
    "    }\n"
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
    "    return fceux11_ppu_cpu_read(g_ppu_state, static_cast<uint16_t>(addr));\n"
)

if 'FCEUX11_PPU_PHASE_TRACE' in text:
    print('already applied')
    sys.exit(0)
if TARGET not in text:
    print('ERROR: target not found')
    sys.exit(1)
text = text.replace(TARGET, PROBE_INSERT)
FILE.write_text(text, encoding='utf-8')
print('ok, new size:', FILE.stat().st_size)
