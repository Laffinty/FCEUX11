import sys
from pathlib import Path
FILE = Path('docs/history/v2.1_phase6_batch_compat.md')
text = FILE.read_text(encoding='utf-8')

OLD = (
    "### §6.6.ter.10 深挖 PPU/CPU 相位（Session C v3, 2026-09-02）\n"
)

NEW = (
    "### §6.6.ter.11 决策记录：281 BROKEN 走 Option A 架构修复（2026-09-02，owner 批准）\n"
    "\n"
    "**决策**：§6.6.ter.10 确认的 281 BROKEN 根因（Rust PPU VBL 块 per-dot 交错 vs C++ engine VBL 块不交错的 PPU/CPU 相位差异），**走架构层修复 Option A**——Rust PPU 的 VBL 块改为非交错（与 C++ engine 行为一致、与硬件真实行为一致），可见 + pre-render 保持 per-dot 交错精度。\n"
    "\n"
    "**项目原理（2026-09-02 锁定）**：\n"
    "1. **Rust 优先**——v2.1 的目标是 Rust PPU 替代 C++ PPU（Phase 7），不是 Rust PPU 模仿 C++ PPU 的过渡壳\n"
    "2. **不为测试改逻辑**——A2002 logic 是 NES 硬件真实行为（NESdev PPU frame timing 文档化），不是测试要求；不能为了 batch_compat 数字放弃硬件正确性\n"
    "3. **修复根因**——281 BROKEN 暴露的是 Rust PPU 架构层偏差（per-dot 交错在 VBL 块内与硬件不符），不是测试 artifact\n"
    "4. **A2002 logic 必须保留**——suppress@(VBL set 前 1 dot) + NMI cancel@(VBL set dot + 1 dot after) 是 NES PPU 真实窗口\n"
    "\n"
    "**否决方案及理由**：\n"
    "- **Option B（A2002 port 加 `newppu` gate）**：倒退信号——明确告诉后人 \"Rust PPU 还没准备好做硬件真实行为\"，Phase 7 退役 C++ PPU 时 A2002 还是死路，281 BROKEN 根因（per-dot 交错）还在下次会重新爆。\n"
    "- **Option C（默认关闭 + env var 开启）**：同 B 的所有问题 + 复杂度上升（CI 跑两套行为，dev 工作流要记两套）。\n"
    "\n"
    "**Staged Plan（下次开工 checklist）**：\n"
    "\n"
    "- [ ] **阶段 1**：读 Phase 5.1 commit history，定位 3-dot chunking 破坏 nestest 的根因\n"
    "  - 关键 commit：`ppu_rendering.cpp:1196-1201` 注释引用\n"
    "  - 确认 VBL 块非交错与 3-dot chunking 是否同一件事（**应该不是**——3-dot 是全扫描线 chunking，VBL 块非交错只影响 sl 241-260）\n"
    "- [ ] **阶段 2**：实施 VBL 块非交错 in `scheduler.rs::tick_one_ppu_dot`\n"
    "  - VBL 块（sl 241-260）整体 PPU-only 推进（一次 20 行不交错）\n"
    "  - 可见（sl 0-240）+ pre-render（sl -1）保持 per-dot 交错\n"
    "  - A2002 port windows 验证（如果 VBL 块相位变了，窗口可能需要 re-tune）\n"
    "- [ ] **阶段 3**：验证（按顺序）\n"
    "  - [ ] `cargo test -p fceux11-ppu` 100% PASS（基础）\n"
    "  - [ ] `mapper_mmc1_frame0_byte_diff` PASS（Phase 5.3 §6.1.f 警告的核心 gate）\n"
    "  - [ ] `ppu_frame_diff_test` 5/5 ROM PASS（Phase 4 门槛）\n"
    "  - [ ] `nestest` rom_regression 780/780 帧 PASS（不要 nestest golden 重生——VBL 块不参与 nestest 的 $2006/$3Fxx 写窗口）\n"
    "  - [ ] `batch_compat 3451`：预期 83.3% → 84.0%+，281 BROKEN 大部分恢复\n"
    "  - [ ] `kagami_qa_direct_smoke` 12/12 PASS（vs §6.6 的 5P/7F baseline）\n"
    "- [ ] **阶段 4**（下下个 session）：如果阶段 3 顺利，深挖 02-vbl_set_time 的 VBL set dot 根因（§6.6.ter.5 候选 1-4）\n"
    "- [ ] **阶段 5**：合并 §6.5/§6.6/§6.6.ter 全部 → §6.6 profile + perf 报告 → Phase 7 准备\n"
    "\n"
    "**当前代码状态**（下次开工前请确认这些 commit 都已落）：\n"
    "- `src/ppu_rust_bridge.cpp:671-693` §6.6.ter A2002 port（v2 优化：suppress@(241,0) + cancel@(241,1)）\n"
    "- `src/rust/crates/fceux11-ppu/src/ffi.rs:428-446` Session C v3 trace probe（env-gated `FCEUX11_PPU_PHASE_TRACE`，默认关闭，可保留作为 Phase 7 诊断工具）\n"
    "- `src/rust/crates/fceux11-ppu/src/frame.rs:152-160` VBL set at `(sl 241, dot 1)`（保持不变，VBL set dot 调整是阶段 4 范畴）\n"
    "- 所有 doc 已落：`docs/plans/v2.1_ppu_rust_refactor_plan.md` §0.5 决策记录，`docs/history/v2.1_phase6_batch_compat.md` §6.6.ter.1-11 完整 session 日志\n"
    "\n"
    "**§5.1 历史教训（避免重复）**：\n"
    "- Phase 5.1 试过 3-dot chunking（全扫描线），破坏 nestest frames 3-7 的 $2006/$3Fxx 写窗口\n"
    "- **本次方案不是 3-dot chunking**——VBL 块非交错 + 可见/Pre-render 保持 per-dot，sub-instruction 精度不丢\n"
    "- 唯一需注意：VBL 块边界（sl 240→241 和 sl 260→-1）的 PPU/CPU 同步点需小心处理\n"
    "\n"
    "### §6.6.ter.10 深挖 PPU/CPU 相位（Session C v3, 2026-09-02）\n"
)

if '§6.6.ter.11 决策记录' in text:
    print('already applied')
    sys.exit(0)
if OLD not in text:
    print('ERROR: old not found')
    sys.exit(1)
text = text.replace(OLD, NEW)
FILE.write_text(text, encoding='utf-8')
print('ok, new size:', FILE.stat().st_size)
