import sys
from pathlib import Path
FILE = Path('src/ppu_rust_bridge.cpp')
text = FILE.read_text(encoding='utf-8')

OLD = (
    "    if (addr == 0x2002) {\n"
    "        const int16_t sl = fceux11_ppu_get_scanline(g_ppu_state);\n"
    "        const uint16_t dot = fceux11_ppu_get_dot(g_ppu_state);\n"
    "        if (sl == 240 && dot == 340) {\n"
    "            // 1 PPU dot before the VBL set at (241, 1): mark\n"
    "            // suppression so the upcoming (241, 1) tick skips the\n"
    "            // VBL flag set + NMI assert. Mirrors\n"
    "            // `fceu11_ppu_mark_vbl_set_suppressed()` in src/ppu.cpp:338.\n"
    "            fceux11_ppu_mark_vbl_set_suppressed(g_ppu_state);\n"
    "        } else if (sl == 241 && (dot == 1 || dot == 2)) {\n"
    "            // At the VBL set dot (241, 1) or 1 dot after (241, 2):\n"
    "            // the read returns VBL=1 (the set just happened) and\n"
    "            // clears the flag, AND we pull /NMI back up before the\n"
    "            // CPU samples it. Mirrors `X6502_IRQEnd(FCEU_IQNMI)` in\n"
    "            // src/ppu.cpp:630. take_nmi_pending returns 1 if a\n"
    "            // pending NMI was latched; we discard the value (the\n"
    "            // NMI is *cancelled*, not delivered) so the next\n"
    "            // `ppu_rust_bridge_take_nmi` poll returns 0.\n"
    "            (void)fceux11_ppu_take_nmi_pending(g_ppu_state);\n"
    "        }\n"
    "    }\n"
)
NEW = (
    "    // Session C (Phase 6.6.ter) follow-up: tighten A2002 windows.\n"
    "    // The previous (240, 340) suppression coordinate was a C++\n"
    "    // engine leftover (where VBL sets at the sl 240 -> 241\n"
    "    // boundary). The Rust PPU is VBL-first and sets VBL at (241, 1),\n"
    "    // so \"1 PPU dot before the set\" in this layout is (241, 0) \u2014\n"
    "    // exactly what `frame.rs:142-144` documents. Using (240, 340)\n"
    "    // here caused 281 batch-compat games to be suppressed at the\n"
    "    // last dot of the post-render line (a position where NMI\n"
    "    // handlers routinely read $2002), suppressing the NEXT frame's\n"
    "    // VBL set and stalling games that poll $2002 bit 7 in their\n"
    "    // main loop. The (241, 0) window is one PPU dot inside the VBL\n"
    "    // block, which the C++ engine's new PPU treats as the\n"
    "    // equivalent of its C++ (241, 0) window (1 dot after the\n"
    "    // C++ engine's boundary-set VBL flag).\n"
    "    if (addr == 0x2002) {\n"
    "        const int16_t sl = fceux11_ppu_get_scanline(g_ppu_state);\n"
    "        const uint16_t dot = fceux11_ppu_get_dot(g_ppu_state);\n"
    "        if (sl == 241 && dot == 0) {\n"
    "            // 1 PPU dot before the VBL set at (241, 1): mark\n"
    "            // suppression so the upcoming (241, 1) tick skips the\n"
    "            // VBL flag set + NMI assert. Mirrors the C++ engine's\n"
    "            // behavior at its (241, 0) read in VBL-first layout.\n"
    "            // See src/ppu.cpp:611-631 and frame.rs:142-144.\n"
    "            fceux11_ppu_mark_vbl_set_suppressed(g_ppu_state);\n"
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
)
if NEW in text:
    print('already applied')
    sys.exit(0)
if OLD not in text:
    print('ERROR: old text not found')
    sys.exit(1)
text = text.replace(OLD, NEW)
FILE.write_text(text, encoding='utf-8')
print('ok, new size:', FILE.stat().st_size)
