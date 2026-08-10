# Phase 8 · Cleanup（清理收尾）

> **目标**：删除已无用的 C++ **newppu=1 路径** CPU/PPU 源、移除兼容垫片、收尾文档。**让 vNESU11 成为 newppu=1 的唯一实现**。
>
> **2026-08-10 修订**：本文件按 `AUDIT_20260810.md` S5 修订——
> **旧 PPU（newppu=0 路径）不删除**，保留为 movie 兼容回退（ADR-009）。
> 清理范围 = newppu=1 路径的 C++ 渲染代码。

## 工期：1 周

---

## 1. 范围

### 1.1 ✅ 在范围内（newppu=1 路径）
- 移除 `src/x6502.cpp`、`src/x6502.h`、`src/x6502struct.h`、`src/x6502abbrev.h`
- 移除 `src/cpu.cpp`、`src/cpu.h`
- 移除 `src/ppu_rendering.cpp/.h`（newppu=1 渲染）
- 移除 `src/ppu_sprite_lut.cpp/.h`（newppu=1 精灵 LUT）
- 移除 `src/pputile_template.cpp/.h`（newppu=1 tile fetcher）
- 移除 `src/ops.inc`、`src/ops_table.inc`、`src/pputile.inc`
- 移除 `src/CMakeLists.txt` 中对应条目
- 移除 `VNESU11_CORE=OFF` 编译路径
- 移除 `FCEUI_*` 兼容垫片（已无意义）
- 移除 shadow run 代码（vNESU11 已是默认）
- 更新 `kagami_bridge.cpp` 引用（已无 C++ newppu CPU/PPU）
- 收尾文档（标记本目录全部 phase 完成）

### 1.2 ❌ 不在范围内（[修订] S5——保留）
- **`src/ppu.cpp`（旧 PPU 主路径）保留**——newppu=0 时仍走 C++ 旧 PPU
- **`src/ppu_class.cpp` 等旧 PPU 支撑代码保留**
- ~250 个 C++ mapper（永远保留）
- Qt 驱动 / 调试器 / TAS Editor（保留）
- Lua 引擎（保留）

> **注意**：`ppu.cpp` 同时包含旧 PPU 主循环和部分共享基础设施（SPRBUF 等）。
> 清理时必须**只删 newppu=1 专属**部分，共享部分保留。以 `grep newppu` 逐处核对。

---

## 2. 任务清单

### 2.1 删除 C++ newppu=1 CPU/PPU 源（[修订] S5）

```bash
git rm src/x6502.cpp src/x6502.h src/x6502struct.h src/x6502abbrev.h
git rm src/cpu.cpp src/cpu.h
git rm src/ppu_rendering.cpp src/ppu_rendering.h
git rm src/ppu_sprite_lut.cpp src/ppu_sprite_lut.h
git rm src/pputile_template.cpp src/pputile_template.h
git rm src/ops.inc src/ops_table.inc src/pputile.inc
# 注意：ppu.cpp / ppu_class.cpp 等【保留】（旧 PPU newppu=0 路径 + 共享基础设施）
```

**验证**：删除前先 `grep -r "x6502\|cpu\.h\|ppu_rendering\|ppu_sprite_lut\|pputile"` 确认所有引用都迁移完毕；`grep -r "newppu"` 确认 newppu=0 路径仍完整。

### 2.2 移除 CMake 条目

```cmake
# src/CMakeLists.txt
set(SRC_CORE
    # ... 保留 fceu.cpp / state.cpp / debug.cpp / mapper / etc
    # 删除所有 x6502 / cpu / ppu 相关
)
```

### 2.3 移除兼容垫片

`FCEUI_*` 函数从 `core_api.h` / `io_api.h` / `net_api.h` / `diag_api.h` / `movie.h` 中删除：

- 直接迁移到 vNESU11 直接调用（Qt 驱动 / TAS Editor / 调试器）
- 保留**部分**必要的（如 `FCEUI_LoadGame` 仍然是入口，由 `vnesu11_attach_mapper` 接管）

### 2.4 移除 vNESU11 bridge / shadow（[修订] 保留 newppu=0 路径）

```bash
git rm src/vnesu11_shadow.cpp src/vnesu11_shadow.h
# vnesu11_bridge 视情况保留（newppu=0 时 CPU/APU 仍走 vNESU11，PPU 走 C++ 旧路径）
# 若 newppu=0 也切到 vNESU11 CPU/APU，bridge 需保留部分函数
```

`kagami_bridge.cpp` 简化为：直接调 vNESU11 FFI（newppu=1 路径）。

### 2.5 收尾文档

- 本目录每个 phase 文件标记 ✅ 完成
- `README.md` 状态表填入完成
- `docs/ChangeLog.md` 增补 `[2.0.1]`（v2.0.0 后清理条目）
- 移除本目录（移到 `docs/completed/v2.0/` 或保留作为历史）

---

## 3. 验证策略

### 3.1 编译验证

- [ ] `cmake -B build`：无 warning（`-Werror`）
- [ ] Release 构建通过
- [ ] 所有 CTest 通过
- [ ] kagami-qa-runner 全绿

### 3.2 行为验证

- [ ] 启动 + 加载 ROM + 跑帧：行为与 Phase 7 完全一致
- [ ] Shadow run 代码已删，无法回退（C++ CPU/PPU 源已删）
- [ ] TAS movie round-trip：通过
- [ ] savestate round-trip：通过

### 3.3 性能对比

- [ ] Release 帧时间 = Phase 7 测量值（无显著变化）
- [ ] 二进制大小：减少（移除 C++ CPU/PPU 源）

---

## 4. 关键技术决策

### 4.1 [修订] 旧 PPU（newppu=0）保留（S5，ADR-009）

- `ppu.cpp` 旧路径保留，`newppu=0` 时 vNESU11 不接管 PPU
- movie `PPUflag=0` 的存档继续用 C++ 旧 PPU 播放
- v2.1 是否把旧 PPU 也迁 Rust = v2.1 决策，不在 v2.0 范围

### 4.2 不删除整个 C++ 代码库

C++ 还保留：
- ~250 个 mapper
- **旧 PPU（newppu=0）路径**
- Qt 驱动
- 调试器
- Lua 引擎绑定
- TAS Editor
- video / drawing / sound 输出

**Phase 8 只删 newppu=1 的 CPU + PPU 渲染**，不动其他。

### 4.3 二进制兼容性

savestate 文件格式**不变**（SFORMAT tag 语义不变）——这是 Phase 0 已经定下的
约束（按 S2 修订后的 tag 驱动方式），Phase 8 必须遵守。

### 4.4 不做"全 Rust 重写"

v2.0 不重写 mapper、Qt 驱动、调试器。Phase 8 的目标是**newppu=1 路径的
CPU/PPU 单一所有权**，不是**全栈 Rust**。

---

## 5. 风险

| 风险 | 严重度 | 缓解 |
|------|--------|------|
| 某个隐藏引用未清理，链接错 | 🟠 中 | `grep -r` 全面扫描；CI 强制编译 |
| 删除兼容垫片破坏 Qt 驱动 | 🟠 中 | 迁移每个调用点到直接 FFI 调用 |
| savestate 兼容破坏 | 🔴 高 | Phase 0 已验证，Phase 8 不再修改布局 |
| 性能回退 | 🟡 低 | Phase 7 已 baseline，Phase 8 不改代码 |

---

## 6. DoD

- [ ] 所有 C++ CPU/PPU 源删除
- [ ] CMake 构建通过（无 warning）
- [ ] CTest 全绿
- [ ] kagami-qa Oracle A + B 全 47 + 177 通过（与 v2.0.0 一致）
- [ ] TAS movie + savestate round-trip 通过
- [ ] 性能 = v2.0.0 基线
- [ ] 本目录所有 phase 文件标记完成
- [ ] CHANGELOG `[2.0.1]` 条目
- [ ] Release tag：`v2.0.1`

---

## 7. 关键文件交付

```
删除：
  src/x6502.*, src/cpu.*, src/ppu*.cpp/.h
  src/ops.inc, src/ops_table.inc, src/pputile.inc
  src/vnesu11_bridge.*, src/vnesu11_shadow.*
  src/CMakeLists.txt 中相应条目

修改：
  src/CMakeLists.txt
  src/core_api.h / io_api.h / net_api.h / diag_api.h / movie.h  # 删除兼容垫片
  src/kagami_bridge.cpp
  docs/ChangeLog.md
  docs/wip_2.0_plan/  # 标记完成，可移至 docs/completed/
```

---

## 8. v2.0 完成总览

| 指标 | v1.17 (基线) | v2.0 (目标) | 状态 |
|------|--------------|-------------|------|
| C++ newppu=1 CPU/PPU 渲染源 | ~4 000 行 | 0 行 | ✅ |
| C++ 旧 PPU（newppu=0） | ~1 000 行 | 保留（回退） | ✅ |
| Rust vNESU11 | ~5 000 行 | ~6 000 行（预估） | ✅ |
| blargg 177 PASS（newppu=1） | 144 | ≥ 144 | ✅ |
| Oracle A 47 PASS | 39 | ≥ 39 | ✅ |
| SMB1 帧时间 | X ms | ≤ X × 1.05 | ✅ |
| 二进制大小 | ~X MB | 略减（newppu=1 C++ 源删除） | ✅ |

**v2.0 完成的客观标志**：
- vNESU11 crate 1.0 release
- `VNESU11_CORE` 移除（不再可选）
- C++ newppu=1 CPU/PPU 源从主分支删除（旧 PPU 保留）
- 文档齐全
