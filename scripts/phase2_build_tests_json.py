"""
Phase 2 — tests.json v1.8 生成器（v1）
数据驱动：47 项 v1.17 迁移 + 65 项 rom-suite + 8 项其他 = 120 项
"""
import json
import sys
import os
import subprocess
from copy import deepcopy

# 镜像源第一个 tag (决策 8b) — 默认所有 rom-suite 引用
DEFAULT_MIRROR_REF = 'v1.8.0-mirror'
MIRROR_REPO = 'https://github.com/Laffinty/f11qa-rom-mirror'

# policy 默认值（v1.8 §六 §6.1）
POLICY = {
    'license_allowed': ['PD', 'CC0', 'zlib', 'GPL-2.0', 'GPL-2.0-only',
                        'GPL-2.0-or-later', 'GPL-3.0', 'GPL-3.0-only',
                        'GPL-3.0-or-later', 'MIT', 'BSD', 'Apache', 'CC-BY'],
    'mirror_repo': MIRROR_REPO,
    'r4_gate_thresholds': {
        'total_min': 120,
        'fail_to_pass_max': 0,
        'mirror_snapshot_check_required': True,
    },
}

# ============================================================================
# A. C++ 单元 (kgmqa-001 ~ 019) — 19 项
# ============================================================================
# v1.17 license 字段全部填 GPL-2.0（FCEUX11 主仓库许可证）
KG_CPP_UNIT = [
    # (kgmqa_id, legacy_id, title, kind, layer, spec_source, input_binary, input_args, working_dir, timeout, tags, failure_means, expected_extra)
    ('kgmqa-001-smoke-cpp', 'smoke_test',
     '核心符号 + fceu11::Initialize() 烟雾',
     ['unit-cpp', 'smoke'], 'core', 'internal',
     'fceux11_smoke_test', None, 'tests', 30, ['unit', 'smoke'], 'blocking', None),
    ('kgmqa-002-cpu', 'cpu_test',
     'CPU 单元 ≥10 case',
     ['unit-cpp'], 'core', 'internal',
     'fceux11_cpu_test', None, 'tests', 60, ['unit', 'cpu'], 'blocking', None),
    ('kgmqa-003-ppu', 'ppu_test',
     'PPU 单元',
     ['unit-cpp'], 'core', 'internal',
     'fceux11_ppu_test', None, 'tests', 60, ['unit', 'ppu'], 'blocking', None),
    ('kgmqa-004-apu', 'apu_test',
     'APU 单元',
     ['unit-cpp'], 'core', 'internal',
     'fceux11_apu_test', None, 'tests', 60, ['unit', 'apu'], 'blocking', None),
    ('kgmqa-005-bus', 'bus_test',
     'Bus 单元',
     ['unit-cpp'], 'core', 'internal',
     'fceux11_bus_test', None, 'tests', 30, ['unit', 'bus'], 'blocking', None),
    ('kgmqa-006-mapper-core', 'mapper_core_test',
     'Mapper core 单元',
     ['unit-cpp'], 'boards', 'internal',
     'fceux11_mapper_core_test', None, 'tests', 60, ['unit', 'mapper'], 'blocking', None),
    ('kgmqa-007-cart-class', 'cart_class_test',
     'Cart class 表面',
     ['unit-cpp'], 'boards', 'internal',
     'fceux11_cart_class_test', None, 'tests', 30, ['unit', 'cart'], 'blocking', None),
    ('kgmqa-008-driver-callbacks', 'driver_callbacks_test',
     'g_driver + register_driver POD 契约',
     ['unit-cpp'], 'driver', 'internal',
     'fceux11_driver_callbacks_test', None, 'tests', 30, ['unit', 'driver'], 'blocking', None),
    ('kgmqa-009-core-driver-boundary', 'core_driver_boundary_test',
     'core/driver 边界 mutex pImpl 不变量',
     ['unit-cpp'], 'driver', 'internal',
     'fceux11_core_driver_boundary_test', None, 'tests', 30, ['unit', 'boundary'], 'blocking', None),
    ('kgmqa-010-savestate-core', 'savestate_core_test',
     'FCEUSS_SaveMS / LoadMS 字段完整性',
     ['unit-cpp'], 'core', 'internal',
     'fceux11_savestate_core_test', None, 'tests', 30, ['unit', 'savestate'], 'blocking', None),
    ('kgmqa-011-fds-load', 'fds_load_test',
     'FDS 加载 + 坏 ROM 检测',
     ['unit-cpp'], 'core', 'internal',
     'fceux11_fds_load_test', None, 'tests', 30, ['unit', 'fds'], 'blocking', None),
    ('kgmqa-012-ppu-rendering-lut', 'ppu_rendering_lut_test',
     'LUT 两阶段 sprite 解码 vs 8x 移位链',
     ['unit-cpp'], 'core', 'internal',
     'fceux11_ppu_rendering_lut_test', None, 'tests', 30, ['unit', 'ppu', 'lut'], 'blocking', None),
    ('kgmqa-013-ppu-phase-c', 'ppu_phase_c_test',
     '原子字节填充 + SPRB + pshift',
     ['unit-cpp'], 'core', 'internal',
     'fceux11_ppu_phase_c_test', None, 'tests', 30, ['unit', 'ppu'], 'blocking', None),
    ('kgmqa-014-ppu-phase-d', 'ppu_phase_d_test',
     'constexpr bitrev LUT + InputScanlineHook + vnapage',
     ['unit-cpp'], 'core', 'internal',
     'fceux11_ppu_phase_d_test', None, 'tests', 30, ['unit', 'ppu'], 'blocking', None),
    ('kgmqa-015-expected-api', 'expected_api_test',
     'tl::expected<T,E> API 覆盖',
     ['unit-cpp'], 'core', 'internal',
     'fceux11_expected_api_test', None, 'tests', 30, ['unit', 'api'], 'blocking', None),
    ('kgmqa-016-mapper-load', 'mapper_load_test',
     '遍历所有内置 mapper iNES 加载',
     ['unit-cpp'], 'boards', 'internal',
     'fceux11_mapper_load_test', None, 'tests', 60, ['unit', 'mapper'], 'blocking', None),
    ('kgmqa-017-mapper-reset', 'mapper_reset_test',
     '加载+reset+power-cycle+reload',
     ['unit-cpp'], 'boards', 'internal',
     'fceux11_mapper_reset_test', None, 'tests', 60, ['unit', 'mapper'], 'blocking', None),
    ('kgmqa-018-config-store', 'config_store_test',
     'TypedConfig<T>',
     ['unit-cpp'], 'core', 'internal',
     'fceux11_config_store_test', None, 'tests', 30, ['unit', 'config'], 'blocking', None),
    ('kgmqa-019-pixbuf-pool', 'pixbuf_pool_test',
     'PixBufPool resize/clear/bytes',
     ['unit-cpp'], 'core', 'internal',
     'fceux11_pixbuf_pool_test', None, 'tests', 30, ['unit', 'pixbuf'], 'blocking', None),
]

# ============================================================================
# B. unit-hdr (kgmqa-020) — 1 项
# ============================================================================
KG_HDR = [
    ('kgmqa-020-enum-class-bitflags', 'enum_class_bitflags_test',
     'FCEU_ENUM_CLASS_BITFLAGS 宏',
     ['unit-hdr'], 'core', 'internal',
     'fceux11_enum_class_bitflags_test', None, 'tests', 30, ['unit', 'header'], 'blocking', None),
]

# ============================================================================
# C. C++ Harness (kgmqa-021 ~ 026) — 6 项
# 注意：v1.17 中部分已迁 Rust runner；这里 kgmqa-021/022/025 是 Rust harness (kgmqa-027/028/029)，
# 但 v1.8 §三 §3.3 C 节仍保留 cpp 版 kgmqa-021/022/025 作为 fallback / 字节级差分的可重实现
# ============================================================================
KG_CPP_HARNESS = [
    ('kgmqa-021-rom-regression-cpp', 'rom_regression_test',
     '12 ROM × 60 帧 → CRC32 vs golden_hashes.json',
     ['harness-cpp'], 'core', 'internal',
     'fceux11_rom_regression_runner', None, 'tests', 120,
     ['regression', 'rom', 'ppu'], 'blocking',
     {'golden_hashes': 'tests/fixtures/golden_hashes.json'}),
    ('kgmqa-022-savestate-regression-cpp', 'savestate_regression_test',
     '12 ROM × 60 帧 → MD5 vs golden_savestate_hashes.json',
     ['harness-cpp'], 'core', 'internal',
     'fceux11_savestate_regression_runner', None, 'tests', 300,
     ['regression', 'savestate'], 'blocking',
     {'golden_hashes': 'tests/fixtures/golden_savestate_hashes.json'}),
    ('kgmqa-023-ppu-frame-diff', 'ppu_frame_diff_test',
     '256×240 XBuf 整帧字节 vs golden_frames/*.xbuf',
     ['harness-cpp'], 'core', 'internal',
     'fceux11_ppu_frame_diff_test', None, 'tests', 60,
     ['ppu', 'diff'], 'blocking', None),
    ('kgmqa-024-apu-wav-diff', 'apu_wav_diff_test',
     '16-bit mono vs golden_wav/*.wav',
     ['harness-cpp'], 'core', 'internal',
     'fceux11_apu_wav_diff_test', None, 'tests', 60,
     ['apu', 'wav', 'diff'], 'blocking', None),
    ('kgmqa-025-mapper-byte-diff-cpp', 'mapper_byte_diff_test',
     'mapper 状态字节 vs golden_mapper/*.bin (169 mapper)',
     ['harness-cpp'], 'boards', 'internal',
     'fceux11_mapper_byte_diff_runner', None, 'tests', 60,
     ['mapper', 'diff'], 'blocking', None),
    ('kgmqa-026-golden-savestate', 'golden_savestate_test',
     '加载 .fc0 → 1 帧 → MD5 vs golden_index.json',
     ['harness-cpp'], 'core', 'internal',
     'fceux11_golden_savestate_test', None, 'tests', 30,
     ['savestate', 'golden'], 'blocking', None),
]

# ============================================================================
# D. Rust harness (kgmqa-027 ~ 043) — 17 项
# 每个 rom-suite 用例在末尾追加 (mirror_path, vendor_state)
# ============================================================================
KG_RUST_HARNESS = [
    # (kgmqa_id, legacy_id, title, kind[], layer, spec_source, binary, args, working_dir, timeout, tags[], failure_means, provenance, mirror_path?, vendor_state?)
    ('kgmqa-027-rom-regression-rust', None,
     'Rust harness: CRC32 视频帧 (Task1-C2 迁移自 cpp)',
     ['harness-rust'], 'core', 'internal',
     'f11qa_rom_regression_runner', None, 'tests', 120,
     ['regression', 'rom', 'ppu', 'rust'], 'blocking',
     'Task1-C2 rust migration, parity 9488773; Rust fixes C++ first-entry parser miss (nrom now verified, 13 ROMs)',
     None, None),
    ('kgmqa-028-savestate-regression-rust', None,
     'Rust harness: MD5 savestate (Task1-C3 迁移自 cpp)',
     ['harness-rust'], 'core', 'internal',
     'f11qa_savestate_regression_runner', None, 'tests', 300,
     ['regression', 'savestate', 'rust'], 'blocking',
     'Task1-C3 rust migration, parity 9488773 (12/12 MD5 identical)',
     None, None),
    ('kgmqa-029-mapper-byte-diff-rust', None,
     'Rust harness: mapper 字节差 (Task1-C3 迁移自 cpp)',
     ['harness-rust'], 'boards', 'internal',
     'f11qa_mapper_byte_diff_runner', None, 'tests', 60,
     ['mapper', 'diff', 'rust'], 'blocking',
     'Task1-C3 rust migration; mapper 字节差 Rust harness',
     None, None),
    ('kgmqa-030-blargg-runner', 'blargg_suite',
     '$6000 协议 runner (180 ROM 全量入口)',
     ['harness-rust'], 'core', 'internal',
     'f11qa_blargg_runner', None, 'tests', 600,
     ['blargg', 'runner'], 'blocking',
     'Task1-C1 blargg harness Rust parity 177/177 + reset semantics fix',
     None, None),
    ('kgmqa-031-blargg-smoke', 'blargg_smoke',
     'nestest.nes 烟雾',
     ['rom-suite', 'harness-rust', 'smoke'], 'core', 'internal',
     'f11qa_blargg_runner', ['--rom', 'tests/fixtures/nestest/nestest.nes',
                              '--frames', '300'], 'tests', 60,
     ['blargg', 'smoke', 'nestest'], 'blocking',
     'Task1-C1 nestest smoke',
     'nestest/nestest.nes', 'vendored'),
    ('kgmqa-032-cpu-instrs-blargg', 'blargg_cpu_instrs',
     'instr_test-v5 all_instrs (blargg CPU 全指令)',
     ['rom-suite', 'harness-rust'], 'core', 'nesdev.org/wiki/Emulator_tests#instr_test_v5',
     'f11qa_blargg_runner', ['--rom', 'tests/fixtures/blargg/cpu/instr_v5_all.nes',
                              '--frames', '3000'], 'tests', 60,
     ['blargg', 'cpu'], 'blocking',
     'Task1-C1 rust migration, parity 9488773 (177/177)',
     'blargg/cpu/instr_test_v5_all.nes', 'vendored'),
    ('kgmqa-033-cpu-timing-blargg', 'blargg_cpu_timing',
     'cpu_timing_test6',
     ['rom-suite', 'harness-rust'], 'core', 'nesdev.org/wiki/Emulator_tests#cpu_timing_test6',
     'f11qa_blargg_runner', ['--rom', 'tests/fixtures/blargg/cpu/cpu_timing_test6.nes',
                              '--frames', '300'], 'tests', 60,
     ['blargg', 'cpu', 'timing'], 'blocking', None,
     'blargg/cpu/cpu_timing_test6.nes', 'vendored'),
    ('kgmqa-034-ppu-vbl-nmi-blargg', 'blargg_ppu_vbl_nmi',
     'ppu_vbl_nmi (advisory 已知限制)',
     ['rom-suite', 'harness-rust'], 'core', 'nesdev.org/wiki/Emulator_tests#ppu_vbl_nmi',
     'f11qa_blargg_runner', ['--rom', 'tests/fixtures/blargg/ppu/ppu_vbl_nmi.nes',
                              '--frames', '600'], 'tests', 60,
     ['blargg', 'ppu', 'vbl', 'nmi'], 'advisory',
     'advisory: VBL/NMI 精度属于 PPU 子系统已记录限制',
     'blargg/ppu/ppu_vbl_nmi.nes', 'pending-vendor'),
    ('kgmqa-035-mmc3-4-scanline-blargg', 'blargg_mmc3_4_scanline_timing',
     'mmc3_test_4 (bucket A)',
     ['rom-suite', 'harness-rust'], 'core', 'nesdev.org/wiki/Emulator_tests#mmc3_test',
     'f11qa_blargg_runner', ['--rom', 'tests/fixtures/blargg/mmc3/mmc3_test_4.nes',
                              '--frames', '300'], 'tests', 60,
     ['blargg', 'mmc3', 'scanline'], 'blocking',
     'mmc3 scanline (bucket A — 已通过)',
     'blargg/mmc3/mmc3_test_4.nes', 'pending-vendor'),
    ('kgmqa-036-mmc3-v2-4-scanline-blargg', 'blargg_mmc3_v2_4_scanline_timing',
     'mmc3_test_v2_4',
     ['rom-suite', 'harness-rust'], 'core', 'nesdev.org/wiki/Emulator_tests#mmc3_test',
     'f11qa_blargg_runner', ['--rom', 'tests/fixtures/blargg/mmc3/mmc3_test_2_4.nes',
                              '--frames', '300'], 'tests', 60,
     ['blargg', 'mmc3', 'scanline', 'v2'], 'blocking', None,
     'blargg/mmc3/mmc3_test_2_4.nes', 'pending-vendor'),
    ('kgmqa-037-cpu-int-2-nmi-brk-blargg', 'blargg_cpu_int_2_nmi_brk',
     'cpu_int_2 (bucket B)',
     ['rom-suite', 'harness-rust'], 'core', 'nesdev.org/wiki/Emulator_tests#cpu_interrupts_v2',
     'f11qa_blargg_runner', ['--rom', 'tests/fixtures/blargg/cpu/cpu_interrupts_v2_2.nes',
                              '--frames', '300'], 'tests', 60,
     ['blargg', 'cpu', 'interrupt'], 'blocking', 'cpu interrupt (bucket B)',
     'blargg/cpu/cpu_interrupts_v2_2.nes', 'vendored'),
    ('kgmqa-038-instr-misc-blargg', 'blargg_instr_misc',
     'instr_misc (bucket B)',
     ['rom-suite', 'harness-rust'], 'core', 'nesdev.org/wiki/Emulator_tests#instr_misc',
     'f11qa_blargg_runner', ['--rom', 'tests/fixtures/blargg/cpu/instr_misc.nes',
                              '--frames', '600'], 'tests', 60,
     ['blargg', 'cpu', 'misc'], 'blocking', 'instr_misc (bucket B)',
     'blargg/cpu/instr_misc.nes', 'vendored'),
    ('kgmqa-039-oam-stress-blargg', 'blargg_oam_stress',
     'oam_stress (bucket C)',
     ['rom-suite', 'harness-rust'], 'core', 'nesdev.org/wiki/Emulator_tests#oam_stress',
     'f11qa_blargg_runner', ['--rom', 'tests/fixtures/blargg/ppu/oam_stress.nes',
                              '--frames', '600'], 'tests', 120,
     ['blargg', 'ppu', 'oam'], 'blocking', 'oam_stress (bucket C)',
     'blargg/ppu/oam_stress.nes', 'pending-vendor'),
    ('kgmqa-040-vbl-05-nmi-timing-blargg', 'blargg_vbl_05_nmi_timing',
     'vbl_05_nmi_timing (PASS monitor)',
     ['rom-suite', 'harness-rust'], 'core', 'nesdev.org/wiki/Emulator_tests#vbl_nmi_timing',
     'f11qa_blargg_runner', ['--rom', 'tests/fixtures/blargg/vbl_nmi_timing/5.nmi_suppression.nes',
                              '--frames', '600'], 'tests', 60,
     ['blargg', 'vbl', 'nmi', 'timing'], 'blocking', None,
     'blargg/vbl_nmi_timing/5.nmi_suppression.nes', 'vendored'),
    ('kgmqa-041-ppu-read-buffer-blargg', 'blargg_ppu_read_buffer',
     'ppu_read_buffer (PASS monitor)',
     ['rom-suite', 'harness-rust'], 'core', 'nesdev.org/wiki/Emulator_tests#ppu_read_buffer',
     'f11qa_blargg_runner', ['--rom', 'tests/fixtures/blargg/ppu/blargg_ppu_2005_vram_access.nes',
                              '--frames', '600'], 'tests', 60,
     ['blargg', 'ppu', 'read-buffer'], 'blocking', None,
     'blargg/ppu/blargg_ppu_2005_vram_access.nes', 'pending-vendor'),
    ('kgmqa-042-sprdma-dmc-dma-blargg', 'blargg_sprdma_dmc_dma',
     'sprdma_dmc_dma (bucket D)',
     ['rom-suite', 'harness-rust'], 'core', 'nesdev.org/wiki/Emulator_tests#sprdma',
     'f11qa_blargg_runner', ['--rom', 'tests/fixtures/blargg/sprdma/sprdma_and_dmc_dma.nes',
                              '--frames', '600'], 'tests', 120,
     ['blargg', 'sprdma', 'dmc', 'dma'], 'blocking', 'sprdma_dmc_dma (bucket D)',
     'blargg/sprdma/sprdma_and_dmc_dma.nes', 'pending-vendor'),
    ('kgmqa-043-blargg-suite', 'blargg_suite',
     '全套 180 ROM batch',
     ['rom-suite', 'harness-rust'], 'core', 'nesdev.org/wiki/Emulator_tests',
     'f11qa_blargg_runner', ['--batch', 'tests/fixtures/blargg_manifest.json',
                              '--frames', '600'], 'tests', 1800,
     ['blargg', 'suite'], 'blocking', '全套 180 ROM batch runner',
     'blargg/cpu/instr_test_v5_all.nes', 'vendored'),  # placeholder; suite 由 manifest 决定
]

# ============================================================================
# E. Lua 绑定 (kgmqa-044 ~ 047) — 4 项
# ============================================================================
KG_LUA = [
    ('kgmqa-044-lua-bit', 'lua_bit_test',
     'bit 库 11 函数',
     ['lua-api', 'harness-rust'], 'lua', 'internal',
     'f11qa_lua_runner', ['tests/lua_scripts/test_bit.lua', '--frames', '60'], '.', 30,
     ['lua', 'integration', 'bit'], 'blocking',
     'fceux11-lua bit library binding tests'),
    ('kgmqa-045-lua-emu', 'lua_emu_test',
     'emu 库',
     ['lua-api', 'harness-rust'], 'lua', 'internal',
     'f11qa_lua_runner', ['tests/lua_scripts/test_emu.lua', '--frames', '60'], '.', 30,
     ['lua', 'integration', 'emu'], 'blocking', 'fceux11-lua emu library binding tests'),
    ('kgmqa-046-lua-memory', 'lua_memory_test',
     'memory 库 9 函数',
     ['lua-api', 'harness-rust'], 'lua', 'internal',
     'f11qa_lua_runner', ['tests/lua_scripts/test_memory.lua', '--frames', '60'], '.', 30,
     ['lua', 'integration', 'memory'], 'blocking', 'fceux11-lua memory library binding tests'),
    ('kgmqa-047-lua-joypad', 'lua_joypad_test',
     'joypad 库 get/set + mask1/mask2',
     ['lua-api', 'harness-rust'], 'lua', 'internal',
     'f11qa_lua_runner', ['tests/lua_scripts/test_joypad.lua', '--frames', '60'], '.', 30,
     ['lua', 'integration', 'joypad'], 'blocking',
     'fceux11-lua | 2026-07-30 demoted to advisory; Task1 lua rust migration (f11qa_lua_runner): '
     'does not surface joypad.get/joypad.set to Lua state (mask1/mask2 override logic unimplemented in '
     'Rust bindings). 2026-08-05 Phase 4.1: bindings verified present; root cause of failure = '
     'joypad.get() returned a TABLE (bitmask_to_table) while the FCEUX-compatible contract (and the test '
     "script's first assertion) requires a NUMBER bitmask. Fixed in src/rust/crates/fceux11-lua/bindings/"
     'joypad.rs (return state as i32). Re-enabled as blocking.'),
]

def make_case(kgmqa_id, legacy_id, title, kind, layer, spec_source,
              binary, args, working_dir, timeout, tags, failure_means, provenance_or_input_extra,
              is_rom_suite=False, mirror_path=None, vendor_state=None, license='GPL-2.0'):
    """通用 case 构造函数"""
    # input
    if isinstance(provenance_or_input_extra, dict):
        # C++ harness 的 expected_extra (golden_hashes 等)
        input_extra = provenance_or_input_extra
        provenance = 'F11QA-recast-2026-09-21; mirror-ref-2026-09-24'
    else:
        input_extra = None
        provenance = (provenance_or_input_extra or '') + '; F11QA-recast-2026-09-21; mirror-ref-2026-09-24'
        provenance = provenance.lstrip('; ').rstrip()

    input_obj = {'binary': binary, 'working_dir': working_dir}
    if args:
        input_obj['args'] = args
    if input_extra:
        input_obj.update(input_extra)

    case = {
        'kgmqa_id': kgmqa_id,
        'legacy_id': legacy_id,
        'title': title,
        'kind': kind,
        'layer': layer,
        'spec_source': spec_source,
        'license': license,
        'input': input_obj,
        'expected': {'exit_code': 0},
        'timeout_seconds': timeout,
        'tags': tags,
        'failure_means': failure_means,
        'provenance': provenance,
    }

    if is_rom_suite:
        case['mirror_ref'] = DEFAULT_MIRROR_REF
        case['mirror_path'] = mirror_path
        case['vendor_state'] = vendor_state
        # rom-suite 用例：failure_means 应由 vendor_state 决定（advisory 不 blocking）
        if vendor_state in ('advisory', 'pending-vendor'):
            case['failure_means'] = 'advisory'

    return case

def build_cases_a_e():
    """生成 kgmqa-001 ~ kgmqa-047（v1.17 迁移 + 升级）"""
    cases = []
    # A
    for (kgid, legacy, title, kind, layer, src, binary, args, wdir, to, tags, fm, extra) in KG_CPP_UNIT:
        cases.append(make_case(kgid, legacy, title, kind, layer, src, binary, args, wdir, to, tags, fm, extra))
    # B
    for (kgid, legacy, title, kind, layer, src, binary, args, wdir, to, tags, fm, extra) in KG_HDR:
        cases.append(make_case(kgid, legacy, title, kind, layer, src, binary, args, wdir, to, tags, fm, extra))
    # C
    for (kgid, legacy, title, kind, layer, src, binary, args, wdir, to, tags, fm, extra) in KG_CPP_HARNESS:
        cases.append(make_case(kgid, legacy, title, kind, layer, src, binary, args, wdir, to, tags, fm, extra))
    # D (15 元 tuple: 含 mirror_path, vendor_state)
    for row in KG_RUST_HARNESS:
        (kgid, legacy, title, kind, layer, src, binary, args, wdir, to, tags, fm, prov,
         mirror_path, vendor_state) = row
        is_rom = 'rom-suite' in kind
        cases.append(make_case(kgid, legacy, title, kind, layer, src, binary, args, wdir, to, tags, fm, prov,
                                is_rom_suite=is_rom, mirror_path=mirror_path, vendor_state=vendor_state))
    # E (13 元 tuple: 无 mirror 字段)
    for row in KG_LUA:
        (kgid, legacy, title, kind, layer, src, binary, args, wdir, to, tags, fm, prov) = row
        cases.append(make_case(kgid, legacy, title, kind, layer, src, binary, args, wdir, to, tags, fm, prov))
    return cases

# ============================================================================
# F. 第三方 ROM 套件 (kgmqa-048 ~ 112) — 65 项
# ============================================================================
# 字段: (kgmqa_id, title, layer, spec_source, binary, args, working_dir, timeout, tags, failure_means, provenance, mirror_path, vendor_state, license)
# mirror_path 是相对 Laffinty/f11qa-rom-mirror 根的路径
KG_ROM_SUITE = [
    # F.1 kevtris (kgmqa-048)
    ('kgmqa-048-nestest', 'kevtris nestest CPU trace vs Nintendulator 黄金日志',
     'core', 'nesdev.org/wiki/Emulator_tests#nestest',
     'f11qa_nestest_runner', ['--rom', 'tests/fixtures/nestest/nestest.nes',
                              '--log', 'tests/fixtures/nestest/nestest.log',
                              '--frames', '255'], 'tests', 60,
     ['kevtris', 'cpu', 'trace', 'TASVideos-required'], 'blocking',
     'v1.8 新增: kevtris nestest CPU 全指令 trace vs Nintendulator 参考日志',
     'nestest/nestest.nes', 'vendored', 'PD'),

    # F.2 bisqwit (kgmqa-049 ~ 052)
    ('kgmqa-049-ppu-read-buffer-bisqwit', 'bisqwit ppu_read_buffer ($2007 读缓冲怪兽级测试)',
     'core', 'bisqwit.iki.fi/src/nes_tests',
     'f11qa_bisqwit_runner', ['--rom', 'tests/fixtures/bisqwit/test_ppu_read_buffer.nes',
                              '--frames', '600'], 'tests', 60,
     ['bisqwit', 'ppu', 'read-buffer'], 'blocking',
     'bisqwit ppu_read_buffer',
     'bisqwit/test_ppu_read_buffer.nes', 'vendored', 'zlib'),
    ('kgmqa-050-cpu-dummy-writes-bisqwit', 'bisqwit cpu_dummy_writes',
     'core', 'bisqwit.iki.fi/src/nes_tests',
     'f11qa_bisqwit_runner', ['--rom', 'tests/fixtures/bisqwit/cpu_dummy_writes_oam.nes',
                              '--frames', '300'], 'tests', 60,
     ['bisqwit', 'cpu', 'dummy-writes'], 'blocking',
     'bisqwit cpu_dummy_writes (oam)',
     'bisqwit/cpu_dummy_writes_oam.nes', 'vendored', 'zlib'),
    ('kgmqa-051-cpu-exec-space-bisqwit', 'bisqwit cpu_exec_space',
     'core', 'bisqwit.iki.fi/src/nes_tests',
     'f11qa_bisqwit_runner', ['--rom', 'tests/fixtures/bisqwit/test_cpu_exec_space_apu.nes',
                              '--frames', '300'], 'tests', 60,
     ['bisqwit', 'cpu', 'exec-space'], 'blocking',
     'bisqwit cpu_exec_space (apu)',
     'bisqwit/test_cpu_exec_space_apu.nes', 'vendored', 'zlib'),
    ('kgmqa-052-cpu-flag-concurrency-bisqwit', 'bisqwit cpu_flag_concurrency (zip 内待解析)',
     'core', 'bisqwit.iki.fi/src/nes_tests',
     'f11qa_bisqwit_runner', ['--rom', 'tests/fixtures/bisqwit/cpu_flag_concurrency.nes',
                              '--frames', '300'], 'tests', 60,
     ['bisqwit', 'cpu', 'flag-concurrency'], 'advisory',
     '⏸ advisory: bisqwit zip-aware 解压分支尚未实现；镜像源中 zip 内多 ROM 待解析',
     'bisqwit/cpu_flag_concurrency.nes', 'advisory', 'zlib'),

    # F.3 blargg 扩展 (kgmqa-053 ~ 067)
    ('kgmqa-053-branch-timing-tests-blargg', 'blargg branch_timing_tests',
     'core', 'nesdev.org/wiki/Emulator_tests',
     'f11qa_blargg_runner', ['--rom', 'tests/fixtures/blargg/cpu/branch_timing_1.Branch_Basics.nes',
                              '--frames', '600'], 'tests', 60,
     ['blargg', 'branch', 'timing'], 'blocking', 'blargg branch timing',
     'blargg/cpu/branch_timing_1.Branch_Basics.nes', 'vendored', 'PD'),
    ('kgmqa-054-cpu-interrupts-v2-blargg', 'blargg cpu_interrupts_v2',
     'core', 'nesdev.org/wiki/Emulator_tests#cpu_interrupts_v2',
     'f11qa_blargg_runner', ['--rom', 'tests/fixtures/blargg/cpu/cpu_interrupts_v2_1-cli_latency.nes',
                              '--frames', '300'], 'tests', 60,
     ['blargg', 'cpu', 'interrupt'], 'blocking', 'cpu_interrupts_v2 cli_latency',
     'blargg/cpu/cpu_interrupts_v2_1-cli_latency.nes', 'vendored', 'PD'),
    ('kgmqa-055-cpu-reset-blargg', 'blargg cpu_reset',
     'core', 'nesdev.org/wiki/Emulator_tests#cpu_reset',
     'f11qa_blargg_runner', ['--rom', 'tests/fixtures/blargg/cpu/cpu_reset_registers.nes',
                              '--frames', '300'], 'tests', 60,
     ['blargg', 'cpu', 'reset'], 'blocking', 'cpu_reset registers',
     'blargg/cpu/cpu_reset_registers.nes', 'vendored', 'PD'),
    ('kgmqa-056-instr-timing-blargg', 'blargg instr_timing',
     'core', 'nesdev.org/wiki/Emulator_tests#instr_timing',
     'f11qa_blargg_runner', ['--rom', 'tests/fixtures/blargg/cpu/instr_timing_instr_timing.nes',
                              '--frames', '300'], 'tests', 60,
     ['blargg', 'cpu', 'instr-timing'], 'blocking', 'instr_timing',
     'blargg/cpu/instr_timing_instr_timing.nes', 'vendored', 'PD'),
    ('kgmqa-057-instr-test-v3-blargg', 'blargg instr_test_v3 (vs v5 互补)',
     'core', 'nesdev.org/wiki/Emulator_tests#instr_test',
     'f11qa_blargg_runner', ['--rom', 'tests/fixtures/blargg/cpu/instr_test_v3_all_instrs.nes',
                              '--frames', '3000'], 'tests', 60,
     ['blargg', 'cpu', 'instr-test', 'v3'], 'blocking', 'instr_test_v3',
     'blargg/cpu/instr_test_v3_all_instrs.nes', 'vendored', 'PD'),
    ('kgmqa-058-ppu-sprite-hit-blargg', 'blargg ppu_sprite_hit',
     'core', 'nesdev.org/wiki/Emulator_tests#sprite_hit',
     'f11qa_blargg_runner', ['--rom', 'tests/fixtures/blargg/ppu/sprite_hit_01.basics.nes',
                              '--frames', '600'], 'tests', 60,
     ['blargg', 'ppu', 'sprite-hit'], 'advisory',
     '❌ pending-vendor: mirror 中 blargg/ppu/ 子目录待补',
     'blargg/ppu/sprite_hit_01.basics.nes', 'pending-vendor', 'PD'),
    ('kgmqa-059-sprite-overflow-blargg', 'blargg sprite_overflow_tests',
     'core', 'nesdev.org/wiki/Emulator_tests#sprite_overflow',
     'f11qa_blargg_runner', ['--rom', 'tests/fixtures/blargg/ppu/sprite_overflow_1.Basics.nes',
                              '--frames', '600'], 'tests', 60,
     ['blargg', 'ppu', 'sprite-overflow'], 'advisory',
     '❌ pending-vendor: mirror 中 blargg/ppu/ 子目录待补',
     'blargg/ppu/sprite_overflow_1.Basics.nes', 'pending-vendor', 'PD'),
    ('kgmqa-060-ppu-open-bus-blargg', 'blargg ppu_open_bus',
     'core', 'nesdev.org/wiki/Emulator_tests#ppu_open_bus',
     'f11qa_blargg_runner', ['--rom', 'tests/fixtures/blargg/ppu/ppu_open_bus.nes',
                              '--frames', '600'], 'tests', 60,
     ['blargg', 'ppu', 'open-bus'], 'advisory',
     '❌ pending-vendor: mirror 中 blargg/ppu/ 子目录待补',
     'blargg/ppu/ppu_open_bus.nes', 'pending-vendor', 'PD'),
    ('kgmqa-061-nmi-sync-blargg', 'blargg nmi_sync',
     'core', 'nesdev.org/wiki/Emulator_tests#nmi_sync',
     'f11qa_blargg_runner', ['--rom', 'tests/fixtures/blargg/ppu/04-nmi_control.nes',
                              '--frames', '600'], 'tests', 60,
     ['blargg', 'ppu', 'nmi-sync'], 'advisory',
     '❌ pending-vendor',
     'blargg/ppu/04-nmi_control.nes', 'pending-vendor', 'PD'),
    ('kgmqa-062-oam-read-blargg', 'blargg oam_read',
     'core', 'nesdev.org/wiki/Emulator_tests#oam_read',
     'f11qa_blargg_runner', ['--rom', 'tests/fixtures/blargg/ppu/oam_read.nes',
                              '--frames', '600'], 'tests', 60,
     ['blargg', 'ppu', 'oam-read'], 'advisory',
     '❌ pending-vendor',
     'blargg/ppu/oam_read.nes', 'pending-vendor', 'PD'),
    ('kgmqa-063-apu-test-blargg', 'blargg apu_test',
     'core', 'nesdev.org/wiki/Emulator_tests#apu_test',
     'f11qa_blargg_runner', ['--rom', 'tests/fixtures/blargg/apu/apu_test.nes',
                              '--frames', '300'], 'tests', 60,
     ['blargg', 'apu'], 'advisory',
     '❌ pending-vendor: blargg apu/ 子目录待补',
     'blargg/apu/apu_test.nes', 'pending-vendor', 'PD'),
    ('kgmqa-064-apu-mixer-blargg', 'blargg apu_mixer',
     'core', 'nesdev.org/wiki/Emulator_tests#apu_mixer',
     'f11qa_blargg_runner', ['--rom', 'tests/fixtures/blargg/apu/apu_mixer.nes',
                              '--frames', '300'], 'tests', 60,
     ['blargg', 'apu', 'mixer'], 'advisory',
     '❌ pending-vendor',
     'blargg/apu/apu_mixer.nes', 'pending-vendor', 'PD'),
    ('kgmqa-065-dmc-tests-blargg', 'blargg dmc_tests',
     'core', 'nesdev.org/wiki/Emulator_tests#dmc',
     'f11qa_blargg_runner', ['--rom', 'tests/fixtures/blargg/apu/dmc_tests_buffer_retained.nes',
                              '--frames', '300'], 'tests', 60,
     ['blargg', 'apu', 'dmc'], 'advisory',
     '❌ pending-vendor',
     'blargg/apu/dmc_tests_buffer_retained.nes', 'pending-vendor', 'PD'),
    ('kgmqa-066-dmc-dma-during-read-blargg', 'blargg dmc_dma_during_read4',
     'core', 'nesdev.org/wiki/Emulator_tests#dmc_dma_during_read',
     'f11qa_blargg_runner', ['--rom', 'tests/fixtures/blargg/apu/dmc_dma_during_read4_dma_2007_read.nes',
                              '--frames', '300'], 'tests', 60,
     ['blargg', 'apu', 'dmc', 'dma'], 'advisory',
     '❌ pending-vendor',
     'blargg/apu/dmc_dma_during_read4_dma_2007_read.nes', 'pending-vendor', 'PD'),
    ('kgmqa-067-square-timer-div2-blargg', 'blargg square_timer_div2',
     'core', 'nesdev.org/wiki/Emulator_tests#square_timer',
     'f11qa_blargg_runner', ['--rom', 'tests/fixtures/blargg/apu/square_timer_div2.nes',
                              '--frames', '300'], 'tests', 60,
     ['blargg', 'apu', 'square', 'timer'], 'advisory',
     '❌ pending-vendor',
     'blargg/apu/square_timer_div2.nes', 'pending-vendor', 'PD'),

    # F.4 Damian Yerrick (kgmqa-068)
    ('kgmqa-068-volume-tests-damianyerrick', 'Damian Yerrick volume_tests (音频通道混音)',
     'core', 'github.com/christopherpow/nes-test-roms/tree/master/volume_tests',
     'f11qa_volume_runner', ['--rom', 'tests/fixtures/damianyerrick/volume_tests/volumes.nes',
                              '--frames', '600'], 'tests', 120,
     ['damianyerrick', 'pinobatch', 'audio', 'volume'], 'blocking',
     'v1.8 新增: 音频通道混音',
     'damianyerrick/volume_tests/volumes.nes', 'vendored', 'zlib'),

    # F.5 Mapper-specific (kgmqa-069 ~ 080)
    ('kgmqa-069-mmc3ir-test-nk', 'N-K mmc3irqtest (MMC3 scanline IRQ + $C000 glitch)',
     'boards', 'forums.nesdev.org/viewtopic.php?p=261236',
     'f11qa_mmc3_runner', ['--rom', 'tests/fixtures/nk/mmc3irqtest/mmc3irqtest.nes',
                            '--frames', '600'], 'tests', 60,
     ['nk', 'mmc3', 'irq', 'scanline'], 'blocking',
     'N-K mmc3irqtest',
     'nk/mmc3irqtest/mmc3irqtest.nes', 'vendored', 'PD'),
    ('kgmqa-070-mmc5test-drag', 'Drag mmc5test (MMC5 scanline)',
     'boards', 'forums.nesdev.org/viewtopic.php?t=11028',
     'f11qa_mmc5_runner', ['--rom', 'tests/fixtures/drag/mmc5test/mmc5test.nes',
                            '--frames', '600'], 'tests', 60,
     ['drag', 'mmc5', 'scanline'], 'blocking',
     'Drag mmc5test',
     'drag/mmc5test/mmc5test.nes', 'vendored', 'PD'),
    ('kgmqa-071-mmc5test-v2-awj', 'AWJ mmc5test_v2',
     'boards', 'forums.nesdev.org/viewtopic.php?t=11028',
     'f11qa_mmc5_runner', ['--rom', 'tests/fixtures/awj/mmc5test_v2/mmc5test.nes',
                            '--frames', '600'], 'tests', 60,
     ['awj', 'mmc5', 'v2'], 'blocking',
     'AWJ mmc5test_v2',
     'awj/mmc5test_v2/mmc5test.nes', 'vendored', 'PD'),
    ('kgmqa-072-vrc24test-awj', 'AWJ vrc24test (VRC2/4 全变体)',
     'boards', 'forums.nesdev.org/viewtopic.php?t=11028',
     'f11qa_vrc24_runner', ['--rom', 'tests/fixtures/awj/vrc24test/vrc24test.nes',
                              '--frames', '600'], 'tests', 60,
     ['awj', 'vrc2', 'vrc4'], 'advisory',
     '❌ pending-vendor: AWJ 余项',
     'awj/vrc24test/vrc24test.nes', 'pending-vendor', 'PD'),
    ('kgmqa-073-vrc6test-natt', 'natt vrc6test',
     'boards', 'forums.nesdev.org/viewtopic.php?t=11028',
     'f11qa_vrc6_runner', ['--rom', 'tests/fixtures/natt/vrc6test/vrc6test.nes',
                            '--frames', '600'], 'tests', 60,
     ['natt', 'vrc6', 'mirroring'], 'advisory',
     '❌ pending-vendor: natt 整 suite 未 vendor',
     'natt/vrc6test/vrc6test.nes', 'pending-vendor', 'PD'),
    ('kgmqa-074-mmc1atest-tepples', 'tepples mmc1atest (MMC1A vs MMC1B)',
     'boards', 'github.com/christopherpow/nes-test-roms',
     'f11qa_mmc1_runner', ['--rom', 'tests/fixtures/tepples/mmc1atest/mmc1atest.nes',
                            '--frames', '600'], 'tests', 60,
     ['tepples', 'mmc1'], 'advisory',
     '❌ pending-vendor: tepples mmc1atest 未在 mirror',
     'tepples/mmc1atest/mmc1atest.nes', 'pending-vendor', 'PD'),
    ('kgmqa-075-mmc3bigchrram-tepples', 'tepples mmc3bigchrram (32KB CHR RAM)',
     'boards', 'github.com/christopherpow/nes-test-roms',
     'f11qa_mmc3_runner', ['--rom', 'tests/fixtures/tepples/mmc3bigchrram/mmc3bigchrram.nes',
                            '--frames', '600'], 'tests', 60,
     ['tepples', 'mmc3', 'chr-ram'], 'blocking',
     'tepples mmc3bigchrram',
     'tepples/mmc3bigchrram/mmc3bigchrram.nes', 'vendored', 'PD'),
    ('kgmqa-076-test28-tepples', 'tepples test28 (mapper 28 / Action 53)',
     'boards', 'github.com/christopherpow/nes-test-roms',
     'f11qa_test28_runner', ['--rom', 'tests/fixtures/tepples/test28/test28.nes',
                              '--frames', '600'], 'tests', 60,
     ['tepples', 'mapper-28', 'action-53'], 'blocking',
     'tepples test28',
     'tepples/test28/test28.nes', 'vendored', 'PD'),
    ('kgmqa-077-holy-mapperel-tepples', 'Holy Mapperel (13 mapper 自动识别, 47 ROM aggregate)',
     'boards', 'github.com/pinobatch/holy-mapperel',
     'f11qa_holy_mapperel_runner',
     ['--batch', 'tests/fixtures/holy_mapperel_manifest.json',
      '--frames', '600'], 'tests', 1800,
     ['pinobatch', 'tepples', 'mapper', 'auto-identify', 'aggregate'], 'blocking',
     'v1.8 新增: Holy Mapperel aggregate (47 ROM 在 runner 内部循环, 全 PASS 才 PASS, kgmqa-077 不拆)',
     'holy_mapperel/M0_P32K_C8K_V.nes', 'vendored', 'zlib'),
    ('kgmqa-078-serom-lidnariq', 'lidnariq serom (MMC1 SEROM/SHROM 约束)',
     'boards', 'forums.nesdev.org/viewtopic.php?f=3&t=9350',
     'f11qa_mmc1_runner', ['--rom', 'tests/fixtures/lidnariq/serom/serom.nes',
                            '--frames', '600'], 'tests', 60,
     ['lidnariq', 'mmc1', 'serom', 'shrom'], 'blocking',
     'lidnariq serom',
     'lidnariq/serom/serom.nes', 'vendored', 'PD'),
    ('kgmqa-079-exram-quietust', 'Quietust exram (MMC5 ExRAM)',
     'boards', 'qmtpro.com/~nes',
     'f11qa_mmc5_runner', ['--rom', 'tests/fixtures/quietust/mmc5exram.nes',
                            '--frames', '600'], 'tests', 60,
     ['quietust', 'mmc5', 'exram'], 'blocking',
     'Quietust MMC5 ExRAM',
     'quietust/mmc5exram.nes', 'vendored', 'PD'),
    ('kgmqa-080-scanline-quietust', 'Quietust scanline (PPU 扫描线精度)',
     'core', 'qmtpro.com/~nes',
     'f11qa_scanline_runner', ['--rom', 'tests/fixtures/quietust/scanline.nes',
                                '--frames', '600'], 'tests', 60,
     ['quietust', 'ppu', 'scanline'], 'blocking',
     'Quietust scanline',
     'quietust/scanline.nes', 'vendored', 'PD'),
]

def build_cases_f_batch1():
    """kgmqa-048 ~ 080 (33 项)"""
    cases = []
    for row in KG_ROM_SUITE:
        (kgid, title, layer, src, binary, args, wdir, to, tags, fm, prov,
         mirror_path, vendor_state, license) = row
        cases.append(make_case(kgid, None, title, ['rom-suite'], layer, src,
                                binary, args, wdir, to, tags, fm, prov,
                                is_rom_suite=True,
                                mirror_path=mirror_path, vendor_state=vendor_state,
                                license=license))
    return cases

# ============================================================================
# F.6 FDS 子系统 (kgmqa-081 ~ 085) — 5 项
# ============================================================================
KG_ROM_FDS = [
    ('kgmqa-081-fds-irq-tests-sour', 'Sour FdsIrqTests v7',
     'core', 'github.com/Sour',
     'f11qa_fds_runner', ['--rom', 'tests/fixtures/sour/fdsirqtests/fdsirqtestsV7_patched.fds',
                            '--frames', '600'], 'tests', 120,
     ['sour', 'fds', 'irq'], 'blocking',
     'Sour FdsIrqTests v7 (patched)',
     'sour/fdsirqtests/fdsirqtestsV7_patched.fds', 'vendored', 'MIT'),
    ('kgmqa-082-fds-mirroring-takuikaninja', 'TakuikaNinja FDS-Mirroring-Test',
     'core', 'github.com/TakuikaNinja/FDS-Mirroring-Test',
     'f11qa_fds_runner', ['--rom', 'tests/fixtures/takuikaninja/FDS-Mirroring-Test/mirroring-test.fds',
                            '--frames', '600'], 'tests', 60,
     ['takuikaninja', 'fds', 'mirroring'], 'advisory',
     '⏸ advisory: TakuikaNinja 上游 4 repo 均无 LICENSE; 联系上游补 LICENSE 后升级',
     'takuikaninja/FDS-Mirroring-Test/mirroring-test.fds', 'advisory', 'PD'),
    ('kgmqa-083-fds-audio-registers-takuikaninja', 'TakuikaNinja FDS-Audio-Registers',
     'core', 'github.com/TakuikaNinja/FDS-Audio-Registers',
     'f11qa_fds_runner', ['--rom', 'tests/fixtures/takuikaninja/FDS-Audio-Registers/audio-registers.fds',
                            '--frames', '600'], 'tests', 60,
     ['takuikaninja', 'fds', 'audio'], 'advisory',
     '⏸ advisory: 等 LICENSE',
     'takuikaninja/FDS-Audio-Registers/audio-registers.fds', 'advisory', 'PD'),
    ('kgmqa-084-fds-4030d1-addr-takuikaninja', 'TakuikaNinja FDS-4030D1-Addr (DRAM 刷新 IRQ)',
     'core', 'github.com/TakuikaNinja/FDS-4030D1-Addr',
     'f11qa_fds_runner', ['--rom', 'tests/fixtures/takuikaninja/FDS-4030D1-Addr/4030d1-addr.fds',
                            '--frames', '600'], 'tests', 60,
     ['takuikaninja', 'fds', 'dram-refresh'], 'advisory',
     '⏸ advisory: 等 LICENSE',
     'takuikaninja/FDS-4030D1-Addr/4030d1-addr.fds', 'advisory', 'PD'),
    ('kgmqa-085-fds-4023-test-takuikaninja', 'TakuikaNinja FDS-4023-Test',
     'core', 'github.com/TakuikaNinja/FDS-4023-Test',
     'f11qa_fds_runner', ['--rom', 'tests/fixtures/takuikaninja/FDS-4023-Test/4023-test.fds',
                            '--frames', '600'], 'tests', 60,
     ['takuikaninja', 'fds', '4023'], 'advisory',
     '⏸ advisory: 等 LICENSE',
     'takuikaninja/FDS-4023-Test/4023-test.fds', 'advisory', 'PD'),
]

# ============================================================================
# F.7 NES 2.0 + mapper 边缘 (kgmqa-086 ~ 098) — 13 项
# ============================================================================
KG_ROM_NES2 = [
    ('kgmqa-086-nes2-submapper-2-test', 'rainwarrior 2_test (UxROM submapper 0/1/2)',
     'boards', 'forums.nesdev.org/viewtopic.php?t=12085',
     'f11qa_submapper_runner', ['--rom', 'tests/fixtures/rainwarrior/submapper/2_test.nes',
                                  '--frames', '300'], 'tests', 60,
     ['rainwarrior', 'nes2', 'submapper', 'uxrom'], 'advisory',
     '⏸ advisory: rainwarrior 4 项 submapper 暂搁（沙箱拿不到 .nes 实体）',
     'rainwarrior/submapper/2_test.nes', 'advisory', 'PD'),
    ('kgmqa-087-nes2-submapper-3-test', 'rainwarrior 3_test (CNROM)',
     'boards', 'forums.nesdev.org/viewtopic.php?t=12085',
     'f11qa_submapper_runner', ['--rom', 'tests/fixtures/rainwarrior/submapper/3_test.nes',
                                  '--frames', '300'], 'tests', 60,
     ['rainwarrior', 'nes2', 'submapper', 'cnrom'], 'advisory',
     '⏸ advisory: 同上',
     'rainwarrior/submapper/3_test.nes', 'advisory', 'PD'),
    ('kgmqa-088-nes2-submapper-7-test', 'rainwarrior 7_test (AxROM)',
     'boards', 'forums.nesdev.org/viewtopic.php?t=12085',
     'f11qa_submapper_runner', ['--rom', 'tests/fixtures/rainwarrior/submapper/7_test.nes',
                                  '--frames', '300'], 'tests', 60,
     ['rainwarrior', 'nes2', 'submapper', 'axrom'], 'advisory',
     '⏸ advisory: 同上',
     'rainwarrior/submapper/7_test.nes', 'advisory', 'PD'),
    ('kgmqa-089-nes2-submapper-34-test', 'rainwarrior 34_test (BNROM)',
     'boards', 'forums.nesdev.org/viewtopic.php?t=12085',
     'f11qa_submapper_runner', ['--rom', 'tests/fixtures/rainwarrior/submapper/34_test.nes',
                                  '--frames', '300'], 'tests', 60,
     ['rainwarrior', 'nes2', 'submapper', 'bnrom'], 'advisory',
     '⏸ advisory: 同上',
     'rainwarrior/submapper/34_test.nes', 'advisory', 'PD'),
    ('kgmqa-090-mmc5ramsize-rainwarrior', 'rainwarrior mmc5ramsize (MMC5 PRG-RAM)',
     'boards', 'forums.nesdev.org/viewtopic.php?p=244062',
     'f11qa_mmc5_runner', ['--rom', 'tests/fixtures/rainwarrior/mmc5ramsize/mmc5ramsize.nes',
                            '--frames', '300'], 'tests', 60,
     ['rainwarrior', 'mmc5', 'prg-ram'], 'blocking',
     'rainwarrior mmc5ramsize',
     'rainwarrior/mmc5ramsize/mmc5ramsize.nes', 'vendored', 'PD'),
    ('kgmqa-091-n163-soundram-rainwarrior', 'rainwarrior n163_soundram (Namco 163 音频 RAM 读回)',
     'core', 'forums.nesdev.org/viewtopic.php?p=284414',
     'f11qa_n163_runner', ['--rom', 'tests/fixtures/rainwarrior/n163_soundram/n163_soundram.nes',
                            '--frames', '300'], 'tests', 60,
     ['rainwarrior', 'n163', 'sound-ram'], 'advisory',
     '⏸ advisory: rainwarrior 暂搁，wayback 无 .nes 实体',
     'rainwarrior/n163_soundram/n163_soundram.nes', 'advisory', 'PD'),
    ('kgmqa-092-n163-soundram-init-rainwarrior', 'rainwarrior n163_soundram_init (通电初值)',
     'core', 'forums.nesdev.org/viewtopic.php?p=284414',
     'f11qa_n163_runner', ['--rom', 'tests/fixtures/rainwarrior/n163_soundram/n163_soundram_init.nes',
                            '--frames', '300'], 'tests', 60,
     ['rainwarrior', 'n163', 'sound-ram', 'init'], 'advisory',
     '⏸ advisory: 同上',
     'rainwarrior/n163_soundram/n163_soundram_init.nes', 'advisory', 'PD'),
    ('kgmqa-093-bntest', 'tepples BNTest (BxROM / BNROM 边界)',
     'boards', 'forums.nesdev.org/viewtopic.php?p=79826',
     'f11qa_bntest_runner', ['--rom', 'tests/fixtures/tepples/bntest/bntest-aorom.nes',
                                '--frames', '300'], 'tests', 60,
     ['tepples', 'bntest', 'bxrom', 'bnrom'], 'blocking',
     'tepples BNTest (aorom 变体; mirror bnxest_aorom.nes 由 tepples 与 bntest/ 子目录同名, 需消歧)',
     'tepples/bntest/bntest-aorom.nes', 'vendored', 'PD'),
    ('kgmqa-094-bxrom-512k-test-rainwarrior', 'rainwarrior bxrom_512k_test',
     'boards', 'forums.nesdev.org/viewtopic.php?f=3&t=12085',
     'f11qa_bxrom_runner', ['--rom', 'tests/fixtures/rainwarrior/bxrom_512k_test/bxrom_512k_test.nes',
                              '--frames', '300'], 'tests', 60,
     ['rainwarrior', 'bxrom', '512k'], 'blocking',
     'rainwarrior bxrom_512k_test',
     'rainwarrior/bxrom_512k_test/bxrom_512k_test.nes', 'vendored', 'PD'),
    ('kgmqa-095-31-test', 'rainwarrior 31_test (mapper 31 子 mapper)',
     'boards', 'forums.nesdev.org/viewtopic.php?f=3&t=13120',
     'f11qa_mapper31_runner', ['--rom', 'tests/fixtures/rainwarrior/31_test/31_test.nes',
                                '--frames', '300'], 'tests', 60,
     ['rainwarrior', 'mapper-31'], 'advisory',
     '⏸ advisory: rainwarrior 暂搁',
     'rainwarrior/31_test/31_test.nes', 'advisory', 'PD'),
    ('kgmqa-096-fme7acktest-tepples', 'tepples fme7acktest-r1 (FME-7 IRQ ack)',
     'boards', 'github.com/christopherpow/nes-test-roms',
     'f11qa_fme7_runner', ['--rom', 'tests/fixtures/tepples/fme7/fme7acktest.nes',
                              '--frames', '300'], 'tests', 60,
     ['tepples', 'fme7', 'irq', 'ack'], 'blocking',
     'tepples fme7acktest',
     'tepples/fme7/fme7acktest.nes', 'vendored', 'PD'),
    ('kgmqa-097-fme7ramtest-tepples', 'tepples fme7ramtest-r1 (FME-7 WRAM)',
     'boards', 'github.com/christopherpow/nes-test-roms',
     'f11qa_fme7_runner', ['--rom', 'tests/fixtures/tepples/fme7/fme7ramtest.nes',
                              '--frames', '300'], 'tests', 60,
     ['tepples', 'fme7', 'wram'], 'blocking',
     'tepples fme7ramtest',
     'tepples/fme7/fme7ramtest.nes', 'vendored', 'PD'),
    ('kgmqa-098-famicom-audio-swap-tests', 'rainwarrior famicom_audio_swap_tests (扩展音频互换)',
     'core', 'rainwarrior.ca/projects/nes/famicom_audio_swap_tests.zip',
     'f11qa_famicom_audio_runner', ['--rom', 'tests/fixtures/rainwarrior/famicom_audio_swap_tests/famicom_audio_swap.nes',
                                       '--frames', '300'], 'tests', 60,
     ['rainwarrior', 'famicom', 'audio', 'swap'], 'advisory',
     '⏸ advisory: zip 内多文件, zip-aware 解压分支待实现',
     'rainwarrior/famicom_audio_swap_tests/famicom_audio_swap.nes', 'advisory', 'PD'),
]

# ============================================================================
# F.8 TV 显示 (kgmqa-099 ~ 100) — 2 项
# ============================================================================
KG_ROM_TV = [
    ('kgmqa-099-240pee-damianyerrick', 'Damian Yerrick 240pee (NTSC/PAL/Dendy 时序 + TV)',
     'core', 'github.com/pinobatch/240p-test-mini',
     'f11qa_240pee_runner', ['--rom', 'tests/fixtures/240pee/240pee.nes',
                              '--frames', '600'], 'tests', 60,
     ['damianyerrick', 'pinobatch', 'tv', 'ntsc', 'pal', 'dendy'], 'blocking',
     '240pee',
     '240pee/240pee.nes', 'vendored', 'GPL-2.0'),
    ('kgmqa-100-tvpassfail-tepples', 'tepples tvpassfail (NTSC 色彩 + NTSC-PAL pixel aspect ratio)',
     'core', 'github.com/christopherpow/nes-test-roms',
     'f11qa_tvpassfail_runner', ['--rom', 'tests/fixtures/tepples/tvpassfail/tv.nes',
                                    '--frames', '600'], 'tests', 60,
     ['tepples', 'tv', 'ntsc', 'pal', 'color'], 'blocking',
     'tepples tvpassfail',
     'tepples/tvpassfail/tv.nes', 'vendored', 'PD'),
]

# ============================================================================
# F.9 输入 (kgmqa-101 ~ 111) — 11 项
# ============================================================================
KG_ROM_INPUT = [
    ('kgmqa-101-allpads-tepples', 'tepples allpads (多控制器综合, 等价 porttest)',
     'core', 'github.com/christopherpow/nes-test-roms',
     'f11qa_allpads_runner', ['--rom', 'tests/fixtures/tepples/porttest/porttest.nes',
                                '--frames', '600'], 'tests', 60,
     ['tepples', 'input', 'controller'], 'blocking',
     'tepples porttest (等价 allpads)',
     'tepples/porttest/porttest.nes', 'vendored', 'PD'),
    ('kgmqa-102-zap-ruder-tepples', 'tepples Zap Ruder (光枪 / 双持)',
     'core', 'github.com/christopherpow/nes-test-roms',
     'f11qa_zap_ruder_runner', ['--rom', 'tests/fixtures/tepples/zap_ruder/zap_ruder.nes',
                                    '--frames', '600'], 'tests', 60,
     ['tepples', 'input', 'zapper', 'dual-hold'], 'advisory',
     '❌ pending-vendor: tepples 余项',
     'tepples/zap_ruder/zap_ruder.nes', 'pending-vendor', 'PD'),
    ('kgmqa-103-spadtest-tepples', 'tepples spadtest-nes (SNES 手柄)',
     'core', 'github.com/christopherpow/nes-test-roms',
     'f11qa_spad_runner', ['--rom', 'tests/fixtures/tepples/spadtest/spadtest.nes',
                            '--frames', '600'], 'tests', 60,
     ['tepples', 'input', 'snes-pad'], 'advisory',
     '❌ pending-vendor',
     'tepples/spadtest/spadtest.nes', 'pending-vendor', 'PD'),
    ('kgmqa-104-powerpad-tepples', 'tepples POWERPAD.NES / powerpadgesture',
     'core', 'github.com/christopherpow/nes-test-roms',
     'f11qa_powerpad_runner', ['--rom', 'tests/fixtures/tepples/powerpad/POWERPAD.NES',
                                '--frames', '600'], 'tests', 60,
     ['tepples', 'input', 'power-pad'], 'advisory',
     '❌ pending-vendor',
     'tepples/powerpad/POWERPAD.NES', 'pending-vendor', 'PD'),
    ('kgmqa-105-paddle-test3-3gengames', '3gengames PaddleTest3 (Arkanoid 旋转电位器)',
     'core', 'forums.nesdev.org/viewtopic.php?t=11028',
     'f11qa_paddle_runner', ['--rom', 'tests/fixtures/3gengames/PaddleTest3/PaddleTest.nes',
                              '--frames', '600'], 'tests', 60,
     ['3gengames', 'input', 'paddle'], 'blocking',
     'PaddleTest3',
     '3gengames/PaddleTest3/PaddleTest.nes', 'vendored', 'PD'),
    ('kgmqa-106-vaus-test-lidnariq', 'lidnariq vaus / Damian Yerrick vaus-test (Arkanoid 9-bit)',
     'core', 'forums.nesdev.org/viewtopic.php?t=23801',
     'f11qa_vaus_runner', ['--rom', 'tests/fixtures/lidnariq/vaus/vaus.nes',
                              '--frames', '600'], 'tests', 60,
     ['lidnariq', 'damianyerrick', 'input', 'vaus', 'arkanoid'], 'blocking',
     'vaus + vaus-test (lidnariq 与 damianyerrick/ 各一 ROM 共用 kgmqa-106)',
     'lidnariq/vaus/vaus.nes', 'vendored', 'PD'),
    ('kgmqa-107-mset-rainwarrior', 'rainwarrior mset (SNES 鼠标)',
     'core', 'forums.nesdev.org/viewtopic.php?p=231608',
     'f11qa_mset_runner', ['--rom', 'tests/fixtures/rainwarrior/mset/mset.nes',
                            '--frames', '600'], 'tests', 60,
     ['rainwarrior', 'input', 'mouse'], 'blocking',
     'mset',
     'rainwarrior/mset/mset.nes', 'vendored', 'PD'),
    ('kgmqa-108-mict-rainwarrior', 'rainwarrior mict (Famicom 麦克风)',
     'core', 'forums.nesdev.org/viewtopic.php?p=231608',
     'f11qa_mict_runner', ['--rom', 'tests/fixtures/rainwarrior/mict/mict.nes',
                            '--frames', '600'], 'tests', 60,
     ['rainwarrior', 'input', 'microphone'], 'blocking',
     'mict',
     'rainwarrior/mict/mict.nes', 'vendored', 'PD'),
    ('kgmqa-109-telling-lys-tepples', 'tepples Telling LYs? (输入扫描线精度)',
     'core', 'github.com/christopherpow/nes-test-roms',
     'f11qa_tellinglys_runner', ['--rom', 'tests/fixtures/tepples/tellinglys/tellinglys.nes',
                                    '--frames', '600'], 'tests', 60,
     ['tepples', 'input', 'scanline'], 'blocking',
     'tellinglys',
     'tepples/tellinglys/tellinglys.nes', 'vendored', 'PD'),
    ('kgmqa-110-dma-sync-test-v2-rahsennor', 'Rahsennor dma_sync_test_v2 (DMC DMA 读取损坏)',
     'core', 'forums.nesdev.org/viewtopic.php?t=14319',
     'f11qa_dma_runner', ['--rom', 'tests/fixtures/rahsennor/dma_sync_test_v2/dma_sync_test_v2.nes',
                            '--frames', '600'], 'tests', 60,
     ['rahsennor', 'dma', 'dmc'], 'blocking',
     'dma_sync_test_v2',
     'rahsennor/dma_sync_test_v2/dma_sync_test_v2.nes', 'vendored', 'PD'),
    ('kgmqa-111-read-joy3-blargg', 'blargg read_joy3 (手柄 + DMC DMA 损坏)',
     'core', 'nesdev.org/wiki/Emulator_tests#read_joy3',
     'f11qa_blargg_runner', ['--rom', 'tests/fixtures/blargg/read_joy3/test_buttons.nes',
                              '--frames', '600'], 'tests', 60,
     ['blargg', 'input', 'joy3', 'dmc'], 'blocking',
     'blargg read_joy3',
     'blargg/read_joy3/test_buttons.nes', 'vendored', 'PD'),
]

# ============================================================================
# F.10 综合压力 (kgmqa-112) — 1 项
# ============================================================================
KG_ROM_STRESS = [
    ('kgmqa-112-nesstress-flubba', 'Flubba NEStress (综合压力)',
     'core', 'nesdev wiki',
     'f11qa_nesstress_runner', ['--rom', 'tests/fixtures/nesstress/NEStress/NEStress.NES',
                                  '--frames', '600'], 'tests', 60,
     ['flubba', 'stress'], 'blocking',
     'NEStress',
     'nesstress/NEStress/NEStress.NES', 'vendored', 'PD'),
]

def build_cases_f_rest():
    """kgmqa-081 ~ 112 (32 项)"""
    cases = []
    all_rom = KG_ROM_FDS + KG_ROM_NES2 + KG_ROM_TV + KG_ROM_INPUT + KG_ROM_STRESS
    for row in all_rom:
        (kgid, title, layer, src, binary, args, wdir, to, tags, fm, prov,
         mirror_path, vendor_state, license) = row
        cases.append(make_case(kgid, None, title, ['rom-suite'], layer, src,
                                binary, args, wdir, to, tags, fm, prov,
                                is_rom_suite=True,
                                mirror_path=mirror_path, vendor_state=vendor_state,
                                license=license))
    return cases

# ============================================================================
# G/H/I/J 杂项 (kgmqa-113 ~ 120) — 8 项
# ============================================================================
KG_OTHER = [
    # G. static-analysis (kgmqa-113 / 114)
    ('kgmqa-113-i18n-regression', 'i18n_regression_test', None,
     'i18n .ts 覆盖率 >90% + simp/trad 污染 + retranslateUi() 完备',
     ['static-analysis'], 'core', 'internal',
     'fceux11_i18n_regression_test', None, 'tests', 30,
     ['i18n', 'static-analysis'], 'blocking',
     'v0.3.15', None, None, 'GPL-2.0'),
    ('kgmqa-114-menu-slot-check', 'menu_slot_check', None,
     'Qt SLOT 静态分析 (Python scripts/check_menu_slots.py)',
     ['static-analysis'], 'driver', 'internal',
     'fceux11_menu_slot_check', None, 'tests', 30,
     ['qt', 'static-analysis'], 'blocking',
     'v1.17 menu_slot_check; Python static analysis', None, None, 'GPL-2.0'),
    # H. perf (kgmqa-115)
    ('kgmqa-115-bench-tolerance', 'bench_tolerance_test', None,
     'R4 协议 (warmup 3 + timed 7, drop extremes, median vs baseline)',
     ['perf'], 'benchmark', 'internal',
     'fceux11_bench_tolerance_test', None, 'tests', 600,
     ['perf', 'benchmark'], 'blocking',
     'v1.17 bench_tolerance_test; R4 gate 性能门禁', None, None, 'GPL-2.0'),
    # I. smoke (kgmqa-116) — legacy_id 是 null（v1.17 没有 headless_smoke_test）
    ('kgmqa-116-headless-smoke', None, None,
     'null driver 编译/链接/启动',
     ['smoke'], 'driver', 'internal',
     'fceux11_headless_smoke_test', None, 'tests', 30,
     ['smoke', 'headless', 'null-driver'], 'blocking',
     'v1.8 新增: headless smoke (null driver 编译/链接/启动)', None, None, 'GPL-2.0'),
    # J. static-license (kgmqa-117) — v1.8 新增（Python 实现避免引入 nlohmann/json）
    ('kgmqa-117-mirror-snapshot-check', None, None,
     'mirror snapshot 一致性 + license accepted set 校验',
     ['static-license'], 'core', 'docs/plans/FCEUX11-v1.8_F11QA-构建计划.md',
     'python', ['tests/f11qa/mirror_snapshot_check.py',
               '--manifest', 'tests/fixtures/f11qa_mirror_pin.json'], 'tests', 60,
     ['license', 'mirror', 'snapshot'], 'blocking',
     'v1.8 新增: mirror_snapshot_check (Python 实现); 取代 v1.17 license_manifest_check', None, None, 'GPL-2.0'),
    # unit-rust (kgmqa-118/119/120) — v1.8 新增
    ('kgmqa-118-config-store-rust', None, None,
     'TypedConfig<T> Rust 端 (对偶 kgmqa-018)',
     ['unit-rust'], 'core', 'internal',
     'cargo_test_config_store', ['-p', 'f11qa', '--test', 'config_store'], '.', 120,
     ['rust', 'config', 'unit'], 'blocking',
     'v1.8 新增: Rust crate 内 TypedConfig<T> 对偶 C++ 测试', None, None, 'GPL-2.0'),
    ('kgmqa-119-pixbuf-pool-rust', None, None,
     'PixBufPool Rust 端 (对偶 kgmqa-019)',
     ['unit-rust'], 'core', 'internal',
     'cargo_test_pixbuf_pool', ['-p', 'f11qa', '--test', 'pixbuf_pool'], '.', 60,
     ['rust', 'pixbuf', 'unit'], 'blocking',
     'v1.8 新增: PixBufPool Rust 对偶 C++ 测试', None, None, 'GPL-2.0'),
    ('kgmqa-120-state-facade-rust', None, None,
     'fceux11::State Rust 端 (对偶 kgmqa-009)',
     ['unit-rust'], 'driver', 'internal',
     'cargo_test_state_facade', ['-p', 'f11qa', '--test', 'state_facade'], '.', 60,
     ['rust', 'state', 'unit'], 'blocking',
     'v1.8 新增: fceux11::State Rust 对偶 C++ 测试', None, None, 'GPL-2.0'),
]

def build_cases_g_j():
    """kgmqa-113 ~ 120 (8 项)"""
    cases = []
    for row in KG_OTHER:
        (kgid, legacy_id, _legacy_id_ignored, title, kind, layer, src, binary, args, wdir, to, tags, fm, prov,
         mirror_path, vendor_state, license) = row
        cases.append(make_case(kgid, legacy_id, title, kind, layer, src,
                                binary, args, wdir, to, tags, fm, prov,
                                is_rom_suite=False,
                                mirror_path=mirror_path, vendor_state=vendor_state,
                                license=license))
    return cases

# ============================================================================
# 主入口
# ============================================================================

def main():
    cases = []
    cases.extend(build_cases_a_e())        # 001-047 (47 项)
    cases.extend(build_cases_f_batch1())   # 048-080 (33 项)
    cases.extend(build_cases_f_rest())      # 081-112 (32 项)
    cases.extend(build_cases_g_j())         # 113-120 (8 项)
    # 验证 kgmqa_id 编号 1..120 连续
    for i, c in enumerate(cases, 1):
        expected_prefix = f'kgmqa-{i:03d}-'
        actual = c['kgmqa_id']
        if not actual.startswith(expected_prefix):
            print(f'WARN: position {i} has kgmqa_id {actual} (expected {expected_prefix}*)')
    output = {
        'schema_version': '1.8',
        'suite_id': 'f11qa-v1.8',
        'generated_at': '2026-09-24T19:30:00Z',
        'policy': POLICY,
        'cases': cases,
    }
    out_path = sys.argv[1] if len(sys.argv) > 1 else 'tests/tests.json'
    with open(out_path, 'w', encoding='utf-8') as f:
        json.dump(output, f, indent=2, ensure_ascii=False)
    print(f'Wrote {len(cases)} cases to {out_path}')

if __name__ == '__main__':
    main()
