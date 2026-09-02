import sys
from pathlib import Path
FILE = Path('docs/plans/v2.1_ppu_rust_refactor_plan.md')
text = FILE.read_text(encoding='utf-8')

OLD = (
    "| **Phase 6** | 批量兼容与性能 | 🟢 **进行中**（6.0-6.4 + 6.1.e follow-up 已收尾；§6.5 Rust PPU 同引擎 baseline 83.95% PASS（+8.23% vs §6.0 75.72%）,90% 目标未达——BROKEN 178 主因 sprite0 hit per-pixel 精度（27 个 Super Mario @ pc=0x8153）+ 各 mapper IRQ 时序；6.6/6.7 待做,2026-09-01 快照） | `v2.1_phase6_batch_compat.md` |\n"
    "| **Phase 7** | 默认启用 + C++ PPU 退役 | ⚪ 未开工 | — |\n"
)

NEW = (
    "| **Phase 6** | 批量兼容与性能 | 🟢 **进行中**（§6.0-§6.5 + §6.6 Session A + §6.6.ter Session C 已收尾；§6.5 Rust PPU 同引擎 83.95% PASS baseline（+8.23% vs §6.0 75.72%），§6.6.ter A2002 port 暂退到 83.3%（281 ROM PASS→FAIL 净 −25，但修 132 no_video→cpu_stuck 视频改善；§6.6.ter.10 锁定 281 BROKEN 根因 = Rust PPU VBL 块 per-dot 交错 vs C++ engine VBL 块不交错的 PPU/CPU 相位差异，**决策：架构层修复 Option A**——VBL 块改为非交错、可见 + pre-render 保持 per-dot；下次开工按 staged plan 推进） | `v2.1_phase6_batch_compat.md` §6.0-§6.6.ter.10 |\n"
    "| **Phase 7** | 默认启用 + C++ PPU 退役 | ⚪ 未开工 | — |\n"
    "\n"
    "### \u00a70.5 关键决策记录（Session C, 2026-09-02）\n"
    "\n"
    "**决策：§6.6.ter 281 BROKEN 走 Option A 架构修复，不走 Option B/C 退让。**\n"
    "\n"
    "项目原理（2026-09-02 锁定，owner 批准）：\n"
    "1. **Rust 优先**——v2.1 的目标是 Rust PPU 替代 C++ PPU（Phase 7），不是 Rust PPU 模仿 C++ PPU 的过渡壳\n"
    "2. **不为测试改逻辑**——A2002 logic 是 NES 硬件真实行为（NESdev PPU frame timing 文档化），不是测试要求；不能为了 batch_compat 数字放弃硬件正确性\n"
    "3. **修复根因**——281 BROKEN 暴露的是 Rust PPU 架构层偏差（per-dot 交错在 VBL 块内与硬件不符），不是测试 artifact；C++ engine 的 VBL 块不交错是隐性的硬件真实实现\n"
    "4. **A2002 logic 必须保留**——suppress@(240,340→VBL-first 0) + NMI cancel@(VBL set dot) 是 NES PPU 真实窗口；B2000 rising-edge NMI enable 时机也是硬件真实\n"
    "\n"
    "**Staged Plan（§6.6.ter → §7.0 transition）**：\n"
    "\n"
    "1. **阶段 1（半天）**：\u8bfb Phase 5.1 commit history，定位 3-dot chunking 破坏 nestest 的根因\n"
    "   - 关键 commit：`ppu_rendering.cpp:1196-1201` 注释引用\n"
    "   - 确认 VBL 块非交错与 3-dot chunking 是否同一件事（**应该不是**）\n"
    "2. **阶段 2（半天）**：实施 VBL 块非交错 in `scheduler.rs::tick_one_ppu_dot`\n"
    "   - VBL 块（sl 241-260）整体 PPU-only 推进；可见 + pre-render 保持 per-dot\n"
    "   - A2002 port windows 可能需要 re-tune（VBL set dot 在 VBL-first 布局下的实际位置）\n"
    "3. **阶段 3（半天）**：验证\n"
    "   - `mapper_mmc1_frame0_byte_diff` 必须 PASS（Phase 5.3 §6.1.f 警告的核心 gate）\n"
    "   - `ppu_frame_diff_test` 5/5 ROM 必须 PASS（Phase 4 门槛）\n"
    "   - `batch_compat 3451`：预期 83.3% → 84.0%+，281 BROKEN 大部分恢复\n"
    "   - `02-vbl_set_time`：本 session 已知与本决策无关（VBL set dot 本身问题，§6.6.ter.5 候选清单）\n"
    "4. **阶段 4（下下个 session）**：如果阶段 3 顺利，深挖 02-vbl_set_time 的 VBL set dot 根因\n"
    "5. **阶段 5**：合并 §6.5/§6.6/§6.6.ter 全部 \u00b7 §6.6 profile + perf 报告 → Phase 7 准备\n"
    "\n"
    "**§5.1 历史教训（避免重复）**：\n"
    "- Phase 5.1 试过 3-dot chunking（全扫描线），破坏 nestest frames 3-7 的 $2006/$3Fxx 写窗口\n"
    "- **本次方案不是 3-dot chunking**——VBL 块非交错 + 可见/Pre-render 保持 per-dot，sub-instruction 精度不丢\n"
    "- 唯一需注意：VBL 块边界（sl 240→241 和 sl 260→−1）的 PPU/CPU 同步点需小心处理\n"
    "\n"
    "**§6.6.ter A2002 port 当前状态**：保留在代码里（`src/ppu_rust_bridge.cpp:671-693`），session C v2 优化（suppress@(241,0) + cancel@(241,1)）\u3002Phase 7 阶段 2 实施 VBL 块非交错后，A2002 port 才会\"正确\"生效；当前状态是\"配置好了但 PPU/CPU 相位错位使它误触发\"，不是\"该删除\"\u3002\n"
)

if 'A2002 port Option A 架构修复' in text:
    print('already applied')
    sys.exit(0)
if OLD not in text:
    print('ERROR: old not found')
    sys.exit(1)
text = text.replace(OLD, NEW)
FILE.write_text(text, encoding='utf-8')
print('ok, new size:', FILE.stat().st_size)
