# core_headers_deps.md · 核心头依赖扫描

> **审计依据**：`AUDIT_20260810.md` S9——实测共 **68 个 include 站点**（不止 Qt 驱动）
> **目的**：每个站点登记"读哪些符号、迁移后走哪条路径"，作为各 phase DoD 的"迁移确认"依据
> **扫描日期**：2026-08-10

---

## 1. 汇总

| 头文件 | 站点数 | 占比 |
|--------|------|------|
| `x6502.h` | 24 | 35.3% |
| `sound.h` | 17 | 25.0% |
| `ppu.h` | 16 | 23.5% |
| `bus.h` | 8 | 11.8% |
| `x6502struct.h` | 3 | 4.4% |
| **总计** | **68** | 100% |

> 这些站点**全部需要 Phase 6 末之前**要么走兼容垫片、要么显式迁移；
> 不是 Phase 8 才动——Phase 1-6 期间任何对 `X`/`PPU`/`ARead`/`::Wave` 等的
> 写入都要走兼容垫片，否则 vNESU11 接管时这些引用会读到"双份状态"。

---

## 2. `x6502.h`（24 站点）

| 文件 | 主要读取符号 | 迁移路径 | 负责 phase |
|------|------------|---------|-----------|
| `asm.cpp` | `FCEUI_DebugBreakpoint` 等 | FFI 调用 → 维持 | Phase 1 + Phase 6 |
| `bus.cpp` | `X` / `timestamp` / `RdMem`/`WrMem` | 走 vNESU11 cpu_read/write FFI | Phase 2 + Phase 6 |
| `cart.cpp` | mapper 注册路径 `X` 读取 | 改 `cart.cpp` 通过 `g_cpu.X()` 取值 | Phase 1 + Phase 5 |
| `cheat.cpp` | `X.PC` 等 | 走 FFI peek | Phase 1 + Phase 6 |
| `core_state.cpp` | `CpuView::reg()` 返 X 引用 | 改用 `vnesu11_cpu_peek_regs` | Phase 1 + Phase 6 |
| `cpu.cpp` | X 字段访问器 | 自身是 vNESU11 内部；**Phase 8 删除** | Phase 1 + Phase 8 |
| `debug.cpp` | `X.PC`/寄存器 / `MapIRQHook` | 走 FFI peek；hook 走 FFI | Phase 1 + Phase 6 |
| `fceu.cpp` | `X6502_Run` 等顶层入口 | 顶层改为 `vnesu11_emulate_frame` | Phase 6 |
| `fds.cpp` | `X6502_IRQBegin` (FCEU_IQEXT/EXT2) | 走 `vnesu11_set_external_irq` | Phase 4 + Phase 6 |
| `fds_sound.cpp` | X 读取 | 走 FFI peek | Phase 4 + Phase 6 |
| `filter.cpp` | X 字段 | 走 FFI peek | Phase 4 + Phase 6 |
| `ines_gi.cpp` | `X.PC` / 中断注入 | 走 FFI | Phase 5 + Phase 6 |
| `ines_load.cpp` | X 寄存器 | 走 FFI | Phase 5 + Phase 6 |
| `input.cpp` | X 寄存器 | 走 FFI | Phase 4 + Phase 6 |
| `kagami_bridge.cpp` | X 完整 blob | 走 FFI peek/poke（**关键 QA 路径**） | Phase 0 + Phase 6 |
| `lua-engine.cpp` | X 寄存器 + 内存 hook | hook 走 `vnesu11_set_lua_mem_hook` | Phase 1 + Phase 6 |
| `nsf_runtime.cpp` | X 寄存器 | 走 FFI | Phase 3（NSF）+ Phase 6 |
| `ppu.cpp` | X 字段读（vBlank/NMI 时序） | 走 FFI | Phase 3 + Phase 6 |
| `ppu_rendering.cpp` | X 字段读（new PPU 时序） | 走 FFI | Phase 3 + Phase 6 |
| `pputile_template.cpp` | X 字段读 | 走 FFI | Phase 3 + Phase 6 |
| `sound.cpp` | X 字段读（APU/DMC 时序） | 走 FFI | Phase 4 + Phase 6 |
| `state.cpp` | X 寄存器（savestate 序列化） | **走 SFORMAT tag 契约**（Phase 0 契约） | Phase 0 + Phase 6 |
| `vsuni.cpp` | X 寄存器 | 走 FFI | Phase 6 |
| `x6502.cpp` | X 自由函数（`X6502_Run` 等） | 自身是 vNESU11 内部；**Phase 8 删除** | Phase 1 + Phase 8 |

**总结**：
- `x6502.cpp` / `cpu.cpp` / `x6502.h` / `x6502struct.h`：Phase 8 整体删除
- `asm.cpp` / `cheat.cpp` / `core_state.cpp` / `debug.cpp` / `lua-engine.cpp` 等：
  Phase 1-6 期间改用 `vnesu11_cpu_peek_*` / `cpu_poke_*` FFI
- 关键时序路径（`bus.cpp` / `ppu.cpp` / `ppu_rendering.cpp`）：Phase 1/3 期间
  改成"读 vNESU11 CPU 状态"而非直接读 `X`

---

## 3. `sound.h`（17 站点）

| 文件 | 主要读取符号 | 迁移路径 | 负责 phase |
|------|------------|---------|-----------|
| `boards/legacy_expansion_audio.h` | 扩展音频声明 | mapper 端不动；**Phase 8 不删** | Phase 5 |
| `core_state.cpp` | `Wave` / `WaveFinal` / `WaveHi` | 改走 `vnesu11_apu_*` | Phase 4 + Phase 6 |
| `expansion_audio.cpp` | 扩展音频填充 | mapper 端不动 | Phase 5 |
| `fceu.cpp` | `FlushEmulateSound` / `WaveFinal` | 改走 vNESU11 FFI | Phase 4 + Phase 6 |
| `fds.cpp` | FDS 声音寄存器 | 走 FFI | Phase 4 + Phase 6 |
| `fds_sound.cpp` | FDS 音频 | 走 FFI | Phase 4 + Phase 6 |
| `filter.cpp` | `Wave` | 走 FFI | Phase 4 + Phase 6 |
| `input.cpp` | `sirq_stat` | 走 FFI | Phase 4 + Phase 6 |
| `kagami_bridge.cpp` | APU 寄存器（QA 路径） | 走 FFI（**关键 QA 路径**） | Phase 0 + Phase 6 |
| `lua-engine.cpp` | `sirq_stat` | 走 FFI | Phase 4 + Phase 6 |
| `nsf_ui.cpp` | NSF 音频输出 | 走 FFI | Phase 3 + Phase 6 |
| `ppu.cpp` | APU 时序耦合 | 走 FFI | Phase 4 + Phase 6 |
| `ppu_rendering.cpp` | APU 时序耦合 | 走 FFI | Phase 4 + Phase 6 |
| `sound.cpp` | APU 全局 | 自身是 vNESU11 内部；**Phase 8 不删（仍 C++ 旧 APU 兼容？）** | Phase 4 + Phase 8 |
| `state.cpp` | APU SFORMAT 序列化 | 走 tag 契约 | Phase 0 + Phase 6 |
| `wave.cpp` | 音频输出 | 走 FFI | Phase 4 + Phase 6 |
| `x6502.cpp` | DMC DMA 时序 | 走 FFI | Phase 4 + Phase 6 |

**注意**：`sound.cpp` 不完全删除——它包含 APU 注册入口、IRQ 路由、QTaI Hack 等
扩展音频逻辑。Phase 8 时评估哪些迁 Rust 哪些保留。

---

## 4. `ppu.h`（16 站点）

| 文件 | 主要读取符号 | 迁移路径 | 负责 phase |
|------|------------|---------|-----------|
| `bus.cpp` | PPU 寄存器访问 | 走 vNESU11 ppu_read/write FFI | Phase 2 + Phase 6 |
| `cart.cpp` | `PPU_hook` / `GameHBIRQHook` | 走 FFI | Phase 3 + Phase 5 + Phase 6 |
| `core_state.cpp` | `PPU` / `NTARAM` / `vnapage` / `VPage` | 改走 FFI peek（savestate 同步） | Phase 3 + Phase 6 |
| `debug.cpp` | PPU 寄存器 / PPU Viewer | 走 FFI；PPU Viewer 走 `ppu_peek` | Phase 3 + Phase 6 |
| `drivers/Qt/config.cpp` | `newppu` 全局 | **不动**（保留 GUI 切换） | 不变 |
| `fceu.cpp` | `FCEUPPU_Loop` / `FFCEUX_PPURead` | 顶层改 vNESU11 FFI；`newppu=0` 时保留 C++ 路径 | Phase 3 + Phase 6 |
| `ines_load.cpp` | CHR 银行信息 | 走 mapper 路径 | Phase 5 |
| `kagami_bridge.cpp` | `FFCEUX_PPURead` / `newppu_get_scanline/dot` | 走 FFI（**关键 QA 路径**） | Phase 0 + Phase 6 |
| `lua-engine.cpp` | `ppu_peek` 类调用 | 走 FFI | Phase 3 + Phase 6 |
| `ppu.cpp` | `PPU` 数组 / `NTARAM` 等 | 自身是 newppu=0 路径；**Phase 8 保留** | Phase 8 |
| `ppu_core.cpp` | newppu=0 时序 | 同上；**Phase 8 保留** | Phase 8 |
| `ppu_core.h` | newppu=0 API | 同上 | Phase 8 |
| `ppu_rendering.cpp` | new PPU 实现 | **Phase 8 删除**（vNESU11 接管） | Phase 3 + Phase 8 |
| `ppu_state.cpp` | 旧 PPU SFORMAT | 改走 tag 契约 | Phase 0 + Phase 8（部分保留） |
| `pputile_template.cpp` | tile fetcher | **Phase 8 删除** | Phase 3 + Phase 8 |
| `state.cpp` | 旧/新 PPU SFORMAT 路径 | 走 tag 契约 | Phase 0 + Phase 6 |

**总结**：
- `ppu_rendering.cpp` / `pputile_template.cpp` / `ppu_sprite_lut.cpp` / `ppu_state.cpp`（new PPU 部分）：
  Phase 8 删除（vNESU11 接管）
- `ppu.cpp` / `ppu_class.cpp` / `ppu_core.cpp` / 旧 `ppu_state.cpp`：Phase 8 **保留**（newppu=0 回退）
- `drivers/Qt/config.cpp`：`newppu` GUI 切换保留（不迁）

---

## 5. `bus.h`（8 站点）

| 文件 | 主要读取符号 | 迁移路径 | 负责 phase |
|------|------------|---------|-----------|
| `bus.cpp` | `ARead[]` / `AWrite[]` / `Page[]` | 自身；**Phase 8 部分删除**（保留 C++ 旧 PPU 的回退路径） | Phase 2 + Phase 8 |
| `bus.cpp` `SetReadHandler/SetWriteHandler` | 区间注册 | **[修订 2026-08-10]** 不做 vNESU11 转发——`readfunc=uint8(*)(uint32)` 无 ctx，无法直接 cast；正确适配在 Phase 5 MapperAdapter（mapper 实例作 ctx）。已从 bus.cpp 移除该转发 | Phase 5 |
| `cart.h` | `set_read_handler` / `set_write_handler` | 改用 `vnesu11_set_read_handler`（当 VNESU11_CORE=ON） | Phase 2 + Phase 5 + Phase 6 |
| `core_state.cpp` | `ARead` 别名 | 走 FFI | Phase 2 + Phase 6 |
| `fceu.cpp` | `SetReadHandler` / `SetWriteHandler` | 转发到 vNESU11 区间表 | Phase 2 + Phase 5 + Phase 6 |
| `fceu.h` | `ARead` 全局声明 | 保留（C++ 旧 PPU 仍需） | 不变 |
| `kagami_bridge.cpp` | `ARead[addr](addr)` 探针 | 走 FFI（QA 关键路径） | Phase 0 + Phase 6 |
| `pputile_template.cpp` | PPU 总线读 | 走 FFI | Phase 3 + Phase 6 |
| `x6502.cpp` | `ARead` / `AWrite` 内存访问 | 走 vNESU11 FFI（**核心 hot path**） | Phase 6 |

**总结（[修订 2026-08-10]）**：
- **[修订]** `x6502.cpp` 的 `RdMem`/`WrMem` **Phase 1 未改动**——Rust CPU 是独立
  解释器（`crates/vnesu11/src/cpu/`），通过自己的 `BusContext` 访问内存；C++ 的
  `x6502.cpp` 仍驱动现有模拟器（VNESU11_CORE=ON 时未接管 frame 循环）。真正的
  切换（C++ `Emulate` 改走 vNESU11）是 Phase 6 的事
- **[修订]** `fceu.cpp::SetReadHandler`/`SetWriteHandler` 是 mapper 注册唯一入口；
  Phase 5 的 MapperAdapter 在这里做 vNESU11 区间注册（不是 bus.cpp 转发，见上表）
- `bus.cpp` 内的 `ARead[]` / `AWrite[]` 表**保留**（C++ 旧 PPU 回退路径仍需）

---

## 6. `x6502struct.h`（3 站点）

| 文件 | 主要读取符号 | 迁移路径 | 负责 phase |
|------|------------|---------|-----------|
| `core_state.cpp` | `X6502` 引用 | 改走 `vnesu11_cpu_peek_regs` | Phase 1 + Phase 6 |
| `cpu.h` | `Cpu::layout_` 字段 | 自身是 vNESU11 内部镜像 | Phase 1 + Phase 8 |
| `x6502.h` | `X6502` 包含 | 改包含 `vnesu11/cpu_regs.h` | Phase 1 + Phase 6 |

**Phase 1 关键校验**：`CpuRegsLayout` 与 `X6502` 字节布局逐字段一致
（见 `savestate_tags.md` §3.1-3.2 与 `phase_0_foundation.md` §2.2）。

---

## 7. 迁移确认检查表（每个 phase DoD 必填）

| Phase | 必须确认迁移的站点 |
|-------|------------------|
| **Phase 0** | `kagami_bridge.cpp`（QA 探针已可走 FFI stub） |
| **Phase 1** | **[修订 2026-08-10]** 无 C++ 站点改动——Rust CPU 独立解释器，通过 `BusContext` 访问内存；C++ 侧 `x6502.cpp`/`core_state.cpp`/`debug.cpp`/`state.cpp` 的接管在 Phase 6 |
| **Phase 2** | `cart.h` / `fceu.cpp`（SetReadHandler/SetWriteHandler 转发）；`bus.cpp` / `core_state.cpp` / `kagami_bridge.cpp` |
| **Phase 3** | `ppu_rendering.cpp` / `pputile_template.cpp`（new PPU 路径删除）；`fceu.cpp`（FCEUPPU_Loop 顶层改）；`core_state.cpp`（PpuView）；`kagami_bridge.cpp`（PPU 探针）；`drivers/Qt/config.cpp`（newppu 保留） |
| **Phase 4** | `sound.cpp` / `fds.cpp` / `fds_sound.cpp` / `wave.cpp` / `filter.cpp`（APU 走 vNESU11） |
| **Phase 5** | `cart.cpp` / `boards/*.cpp`（mapper 注册自动走 vNESU11 区间表，通过转发机制实现） |
| **Phase 6** | `asm.cpp` / `cheat.cpp` / `ines_gi.cpp` / `ines_load.cpp` / `lua-engine.cpp` / `nsf_runtime.cpp` / `nsf_ui.cpp` / `vsuni.cpp`（外围站点改走 FFI） |
| **Phase 8** | `x6502.cpp` / `x6502.h` / `x6502struct.h` / `x6502abbrev.h` / `cpu.cpp` / `cpu.h` / `ops.inc` / `ops_table.inc` / `ppu_rendering.cpp` / `pputile_template.cpp` / `ppu_sprite_lut.cpp` / `pputile.inc` 删除（newppu=1 路径） |

---

## 8. 不在迁移范围（明确不动的站点）

| 文件 / 符号 | 不动原因 |
|-----------|---------|
| `drivers/Qt/config.cpp` `newppu` | GUI 配置项必须保留（ADR-009） |
| `fceu.h` `ARead` 全局 | C++ 旧 PPU 回退路径仍需 |
| `bus.cpp` `ARead[]` / `AWrite[]` | C++ 旧 PPU 回退路径仍需（双写策略） |
| `ppu.cpp` / `ppu_class.cpp` / `ppu_core.cpp` | 旧 PPU 保留（ADR-009） |
| `boards/*.cpp` 全部 mapper | 不迁（ADR-002），只走 FFI 适配 |
| `sound.cpp` 部分（如扩展音频） | 评估后保留 |

---

## 9. 更新日志

- **2026-08-10**：初版，基于 2026-08-10 grep 实测（68 站点）。Phase 0 开工前必须完成。
