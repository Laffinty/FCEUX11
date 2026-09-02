import re
from pathlib import Path
FILE = Path('src/rust/crates/fceux11-ppu/src/ffi.rs')
text = FILE.read_text(encoding='utf-8')

OLD = (
    "    ret\n"
    "}\n"
    "\n"
    "/// Phase 5.1: take-and-clear the PPU NMI latch. Returns 1 when the\n"
)
NEW = (
    "    // Session C v3 (PPU/CPU phase investigation): env-gated trace of\n"
    "    // every $2002 read with the current (sl, dot). Compares against\n"
    "    // the C++ engine's E1 P2002_READ stream to detect PPU/CPU phase\n"
    "    // drift between the two engines. Activated by FCEUX11_PPU_PHASE_TRACE=1.\n"
    "    if reg == 2 {\n"
    "        static ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);\n"
    "        static INIT: std::sync::Once = std::sync::Once::new();\n"
    "        INIT.call_once(|| {\n"
    "            if let Ok(v) = std::env::var(\"FCEUX11_PPU_PHASE_TRACE\") {\n"
    "                if v == \"1\" { ON.store(true, std::sync::atomic::Ordering::Relaxed); }\n"
    "            }\n"
    "        });\n"
    "        if ON.load(std::sync::atomic::Ordering::Relaxed) {\n"
    "            eprintln!(\n"
    "                \"R3 P2002_READ sl={} dot={}\",\n"
    "                sb.state.scanline, sb.state.dot\n"
    "            );\n"
    "        }\n"
    "    }\n"
    "    ret\n"
    "}\n"
    "\n"
    "/// Phase 5.1: take-and-clear the PPU NMI latch. Returns 1 when the\n"
)

if 'R3 P2002_READ' in text:
    print('already applied')
    import sys; sys.exit(0)
if OLD not in text:
    print('ERROR: old not found')
    import sys; sys.exit(1)
text = text.replace(OLD, NEW)
FILE.write_text(text, encoding='utf-8')
print('ok, new size:', FILE.stat().st_size)
