# 全局状态清查报告 (v1.2 Census)

> **基线**：v1.0.0 (LTS, 基于 v0.3.16)
> **目标版本**：v1.2 Census
> **生成日期**：2026-06-18
> **生成工具**：人工 + `grep -E "^extern " src/`
> **范围**：`src/` 下所有头文件的 file-scope `extern` 声明（不含 `extern "C"` 块、函数原型 extern、不含 drivers/ Qt 驱动层）

---

## 0. 摘要

| 分类 | 数量 | 计划迁入 |
|------|------|----------|
| A — CPU 所有 | 8 | v1.3 `Cpu` |
| B — PPU 所有 | 18 | v1.5 `Ppu` |
| C — APU 所有 | 11 | v1.6 `Apu` |
| D — Bus 所有 | 6  | v1.4 `Bus` |
| E — Cart 所有 | 24 | v1.7 `Cart` |
| F — Config | 9  | 仅保留全局配置，v1.11 收敛到 State |
| G — Debug | 25 | 仅调试构建可见，不进入 State |
| **总计 (核心 file-scope extern)** | **101** | **v1.14 目标 ≤ 6** |

> 实际数量略多于 v1.x §0.1 表中的"~100"估值；驱动器层 `extern`（如 `fceuWrapper.h` 中 19 个）属于 v1.11 Bridge 范畴，未在本表统计。

---

## 1. 分类 A — CPU 所有（8 项）

| 名称 | 类型 | 声明位置 | 写入文件数 | 读取文件数 | 备注 |
|------|------|----------|-----------|-----------|------|
| `X` | `X6502` | `x6502.h:26` | 1 (x6502.cpp) | ~30 (含 boards) | 主 CPU 状态 |
| `timestamp` | `uint32` | `x6502.h:53` | 3 (ppu/sound/x6502) | ~13 (含 Rust FFI) | CPU 主时钟 |
| `soundtimestamp` | `uint32` | `x6502.h:54` | 1 (sound.cpp) | 6 | APU 子时钟 |
| `scanline` | `int` | `x6502.h:55` | 3 | ~13 | 当前扫描线 |
| `MapIRQHook` | `fceu11::MapIRQHook` | `x6502.h:70` | 35 (boards) | 1 (x6502.cpp) | 已类型化 (v0.3.8) |
| `timestampbase` | `uint64` | `fceu.h:55` | 2 | 4 | 录像基准 |
| `normalscanlines` | `int` | `fceu.h:14` | 1 (config) | 5 | 视频系统参数 |
| `totalscanlines` | `int` | `fceu.h:15` | 1 (config) | 4 | 同上 |

**v1.3 迁移计划**：`X`/`timestamp`/`soundtimestamp`/`scanline`/`MapIRQHook` 迁入 `Cpu::layout_` 或 `Cpu::*_` 成员，保留 `extern` 别名供 35 个 board 文件逐批过渡。

---

## 2. 分类 B — PPU 所有（18 项）

| 名称 | 类型 | 声明位置 | 写入文件数 | 读取文件数 | 备注 |
|------|------|----------|-----------|-----------|------|
| `PPU[4]` | `uint8[]` | `ppu.h:42` / `debug.h:125` | 1 (ppu.cpp) | ~10 | PPU 寄存器镜像 |
| `NTARAM[0x800]` | `uint8[]` | `ppu.h:17` | 1 (ppu.cpp) | ~12 | 名称表 RAM |
| `vnapage[4]` | `uint8*[]` | `ppu.h:17` | 1 (ppu.cpp) | ~5 | 名称表指针 |
| `PPUNTARAM` | `uint8` | `ppu.h:18` | 1 | 1 | 标记 NTARAM 是否启用 |
| `PPUCHRRAM` | `uint8` | `ppu.h:19` | 1 | 1 | CHR RAM 标记 |
| `ppuphase` | `PPUPHASE` | `ppu.h:50` | 1 (ppu.cpp) | 3 | 渲染阶段 |
| `PPU_hook` | `void(*)(uint32)` | `ppu.h:9` | 0 | 1 | mapper 钩子 |
| `GameHBIRQHook[2]` | `void(*)()` | `ppu.h:10` | 1 (ppu.cpp) | 2 | HBlank IRQ |
| `FFCEUX_PPURead` | `uint8(*)(uint32)` | `ppu.h:36` | 1 | 4 | PPU 总线读 |
| `FFCEUX_PPUWrite` | `void(*)(uint32,uint8)` | `ppu.h:37` | 1 | 4 | PPU 总线写 |
| `g_rasterpos` | `int` | `ppu.h:41` | 1 (ppu.cpp) | 1 | 调试器使用 |
| `VPage[8]` | `uint8*[]` | `debug.h:124` | 1 (ppu.cpp) | ~5 | PPU 地址页表 |
| `SPRAM[0x100]` | `uint8[]` | `debug.h:125` | 1 | 4 | OAM |
| `VRAMBuffer` | `uint8` | `debug.h:125` | 1 (ppu.cpp) | 3 | VRAM 读缓冲 |
| `PPUGenLatch` | `uint8` | `debug.h:125` | 1 (ppu.cpp) | 3 | 总线复用 |
| `XOffset` | `uint8` | `debug.h:125` | 1 (ppu.cpp) | 2 | 精灵 X 偏移 |
| `PALRAM[0x20]` | `std::array<uint8_t>` | `debug.h:126` | 1 (ppu.cpp) | 3 | 调色板 RAM |
| `UPALRAM[3]` | `std::array<uint8_t>` | `debug.h:127` | 1 (ppu.cpp) | 1 | 调色板亮度 |

**v1.5 迁移计划**：全部迁入 `Ppu` 类（保持 savestate 二进制兼容）。

---

## 3. 分类 C — APU 所有（11 项）

| 名称 | 类型 | 声明位置 | 写入文件数 | 读取文件数 | 备注 |
|------|------|----------|-----------|-----------|------|
| `Wave[2560]` | `int32[]` | `sound.h:49` | 1 (sound.cpp) | 2 | LQ 输出缓冲 |
| `WaveFinal[2560]` | `int32[]` | `sound.h:50` | 1 | 2 | 滤波后 LQ |
| `WaveHi[]` | `int32[]` | `sound.h:51` | 1 | 2 | HQ 输出 |
| `nesincsize` | `int32` | `sound.h:43` | 1 | 1 | 时钟增量 |
| `soundtsinc` | `uint32` | `sound.h:52` | 1 | 1 | 时间戳增量 |
| `soundtsoffs` | `uint32` | `sound.h:60` | 1 | 2 | 采样偏移 |
| `GameExpSound` | `EXPSOUND` | `sound.h:41` | 5 (boards) | 1 (sound.cpp) | 扩展音频钩子 |
| `PSG[0x10]` | `uint8[]` | `debug.h:179` | 1 (sound.cpp) | 2 | APU 寄存器镜像 |
| `DMCFormat/RawDALatch/DMCAddressLatch/DMCSizeLatch/InitialRawDALatch` | 5×`uint8` | `sound.cpp` / `debug.h:180-183` | 1 | ~3 | DMC 状态 |
| `EnabledChannels/SpriteDMA/RawReg4016/IRQFrameMode` | 4×`uint8` | `debug.h:184-187` | 1 | ~3 | APU 寄存器 |
| `swapDuty` | `bool` | `sound.h:61` | 1 | 2 | NTSC/PAL 占空比翻转 |

> 注：`ENVUNIT` 是 typedef 而非变量；`ENVUNIT env_units[]` 数组为 sound.cpp 文件作用域，不进入 extern 清单。

**v1.6 迁移计划**：全部迁入 `Apu` 类 + `ExpansionAudio` 虚接口（`EXPSOUND` 兼容层保留至 v2.0）。

---

## 4. 分类 D — Bus 所有（6 项）

| 名称 | 类型 | 声明位置 | 写入文件数 | 读取文件数 | 备注 |
|------|------|----------|-----------|-----------|------|
| `ARead[0x10000]` | `readfunc[]` | `fceu.h:85` | ~5 (cart/mapper) | ~50 (热路径) | CPU 读分发表 |
| `BWrite[0x10000]` | `writefunc[]` | `fceu.h:86` | ~5 | ~30 (含 mapper) | CPU 写分发表 |
| `Page[32]` | `uint8*[]` | `cart.h:89` | 1 (cart.cpp) | ~30 | CPU 页表 |
| `VPage[8]` | `uint8*[]` | `cart.h:89` / `debug.h:124` | 1 | ~25 | PPU 页表 |
| `MMC5SPRVPage[8]` | `uint8*[]` | `cart.h:89` | 1 (mmc5) | 1 | MMC5 精灵页表 |
| `MMC5BGVPage[8]` | `uint8*[]` | `cart.h:89` | 1 (mmc5) | 1 | MMC5 背景页表 |

**v1.4 迁移计划**：6 个数组迁入 `Bus` 类，cache-line 对齐 (`FCEUX11_CACHE_ALIGN`)。

---

## 5. 分类 E — Cart 所有（24 项）

| 名称 | 类型 | 声明位置 | 写入文件数 | 读取文件数 | 备注 |
|------|------|----------|-----------|-----------|------|
| `currCartInfo` | `CartInfo*` | `cart.h:83` | 1 (ines/unif) | ~20 | 当前卡带 |
| `PRGram[32]` | `uint8[]` | `cart.h:100` | 1 | 1 | PRG RAM 注册表 |
| `CHRram[32]` | `uint8[]` | `cart.h:101` | 1 | 1 | CHR RAM 注册表 |
| `PRGptr[32]` | `uint8*[]` | `cart.h:103` | 1 | ~50 (boards) | PRG 指针表 |
| `CHRptr[32]` | `uint8*[]` | `cart.h:104` | 1 | ~30 (boards) | CHR 指针表 |
| `PRGsize[32]` / `CHRsize[32]` | `uint32[]` | `cart.h:106-107` | 1 | 5 | 大小表 |
| `PRGmask{2,4,8,16,32}[32]` | 5×`uint32[]` | `cart.h:109-113` | 1 | ~20 (boards) | 寻址掩码 |
| `CHRmask{1,2,4,8}[32]` | 4×`uint32[]` | `cart.h:115-118` | 1 | ~15 (boards) | 同上 |
| `ROM` | `uint8*` | `ines.h:43` | 1 | ~30 | ROM 数据 |
| `VROM` | `uint8*` | `ines.h:44` | 1 | ~5 | CHR ROM |
| `VROM_size` | `uint32` | `ines.h:45` | 1 | ~5 | CHR 大小 |
| `ROM_size` | `uint32` | `ines.h:46` | 1 | ~30 | ROM 大小 |
| `ExtraNTARAM` | `uint8*` | `ines.h:47` | 1 | 1 | MMC5 额外 NTARAM |
| `VPageR[8]` | `uint8**` | `ines.h:48` | 1 | 1 | 只读 PPU 页表 |
| `head` | `iNES_HEADER` | `ines.h:88` | 1 | ~5 | ROM 头 |
| `geniestage` | `int` | `cart.h:151` | 1 (cheat) | 1 | Game Genie 状态 |
| `QTAINTRAM[2048]` | `uint8[]` | `fceu.h:72` | 1 (VRCV mapper) | 1 | VRCV 内部 RAM |
| `QTAIHack` | `int` | `fceu.h:71` | 1 | 1 | VRCV 标志 |
| `qtaintramreg` | `uint8` | `fceu.h:73` | 1 | 1 | VRCV 寄存器 |
| `LoadedRomFName[4096]` | `char[]` | `ines.h:51` | 1 | ~3 | 加载路径 |
| `MasterRomInfo` | `TMasterRomInfo*` | `ines.h:54` | 1 | 1 | Bad ROM 信息 |
| `MasterRomInfoParams` | `TMasterRomInfoParams` | `ines.h:55` | 1 | 1 | Bad ROM 参数 |

**v1.7 迁移计划**：`CartInfo*` 与 bank 表迁入 `Cart`/`Bus`；`ROM`/`VROM`/`head` 保留（v1.10 Cryptex 转为 Rust）。

---

## 6. 分类 F — Config（9 项）

| 名称 | 类型 | 声明位置 | 写入文件数 | 读取文件数 | 备注 |
|------|------|----------|-----------|-----------|------|
| `PAL` | `uint8` | `fceu.h:103` | 2 | ~15 | PAL/NTSC 标志 |
| `dendy` | `int` | `fceu.h:104` | 1 | 2 | Dendy 制式 |
| `movieSubtitles` | `bool` | `fceu.h:105` | 1 | 1 | 字幕开关 |
| `FSettings` | `FCEUS` | `fceu.h:141` | 1 (config) | ~30 | 主配置结构 |
| `RAMInitOption` | `int` | `fceu.h:80` | 2 | ~10 | 内存初始化模式 |
| `RAM` | `uint8*` | `fceu.h:77` | 1 | ~30 | 主内存指针 |
| `EmulationPaused` | `int` | `fceu.h:78` | 1 (fceuWrapper) | ~5 | 暂停标志 |
| `frameAdvance_Delay` | `int` | `fceu.h:79` | 1 | 2 | 帧步进延迟 |
| `GameAttributes` | `int` | `fceu.h:101` | 1 | 3 | 游戏属性 |

**策略**：保留为全局直至 v1.11 Bridge 收拢到 `fceu11::State::config()`。

---

## 7. 分类 G — Debug / 诊断（25 项）

| 名称 | 类型 | 声明位置 | 写入文件数 | 读取文件数 | 备注 |
|------|------|----------|-----------|-----------|------|
| `fceuindbg` | `int` | `fceu.h:6` | 1 | 3 | 调试模式标志 |
| `watchpoint[65]` | `watchpointinfo[]` | `debug.h:65` | 1 (debug) | ~10 | 监视点 |
| `debuggerPageSize` | `unsigned int` | `debug.h:67` | 1 | 1 | 调试页大小 |
| `codecount/datacount/undefinedcount` | 3×`volatile int` | `debug.h:82` | 1 (debug) | ~5 | CD Logger 计数 |
| `cdloggerdata/Size` | `unsigned char*` / `unsigned int` | `debug.h:83-84` | 1 | 2 | CD Logger 数据 |
| `debug_loggingCD` | `int` | `debug.h:86` | 1 (fceuWrapper) | 2 | 开关 |
| `cdloggervdata` | `unsigned char*` | `ppu.h:52` | 1 (ppu.cpp) | 1 | 视频日志 |
| `cdloggerVideoDataSize` | `unsigned int` | `ppu.h:53` | 1 | 1 | 视频日志大小 |
| `rendercount/vromreadcount/undefinedvromcount` | 3×`volatile int` | `ppu.h:54` | 1 | 1 | PPU 诊断 |
| `iaPC` | `int` | `debug.h:99` | 1 | 1 | IA PC |
| `iapoffset` | `uint32` | `debug.h:100` | 1 | 1 | IA 偏移 |
| `break_asap` | `bool` | `debug.h:105` | 1 | 3 | 即时中断 |
| `break_on_unlogged_{code,data}` | 2×`bool` | `debug.h:106-107` | 1 | 1 | 未记录中断 |
| `total_cycles_base/delta_cycles_base` | 2×`uint64` | `debug.h:108-109` | 1 | 1 | 周期计数 |
| `break_on_cycles/break_cycles_limit` | `bool`/`uint64` | `debug.h:110-111` | 1 | 1 | 周期断点 |
| `total_instructions/delta_instructions` | 2×`uint64` | `debug.h:112-113` | 1 | 1 | 指令计数 |
| `break_on_instructions/break_instructions_limit` | `bool`/`uint64` | `debug.h:114-115` | 1 | 1 | 指令断点 |
| `numWPs` | `int` | `debug.h:130` | 1 | 1 | 监视点数 |
| `NSFHeader` | `NSF_HEADER` | `debug.h:177` | 1 | 2 | NSF 头 |
| `DMC_7bit` | `bool` | `ppu.h:43` | 1 | 1 | DMC 7bit 模式 |
| `paldeemphswap` | `bool` | `ppu.h:44` | 1 | 1 | PAL 去加重交换 |
| `debugSymbolTable` | `debugSymbolTable_t` | `debugsymboltable.h:171` | 1 | ~3 | 符号表 |
| `ROM_size` (重复计数 E) | `uint32` | `ines.h:46` | — | — | — |

**策略**：仅 `#ifdef FCEUDEF_DEBUGGER` 可见，不进入 `fceu11::State` 主入口；Release 构建零开销。

---

## 8. 文件级 `using namespace std;` 清查

`grep -nE '^using namespace std;' src/` 结果（**v1.2 仅 6 处 file-scope 命中**）：

| 文件 | 行号 | 说明 |
|------|------|------|
| `src/cheat.cpp` | 52 | file-scope |
| `src/fceu.cpp` | 96 | file-scope |
| `src/file.cpp` | 54 | file-scope |
| `src/movie.cpp` | 61 | file-scope |
| `src/oldmovie.cpp` | 22 | file-scope |
| `src/state.cpp` | 70 | file-scope |

> 计划提到的 `xstring.cpp` 不需修改 — 其 `using namespace std;` 位于 `tokenize_str()` 函数体内（`utils/xstring.cpp:363`），已限定作用域，不属于 file-scope 反模式。

---

## 9. 进度追踪（v1.2 → v1.14）

| 分类 | v1.0 数量 | v1.2 归档 | v1.3 | v1.4 | v1.5 | v1.6 | v1.7 | v1.14 目标 |
|------|-----------|-----------|------|------|------|------|------|-----------|
| A — CPU | 8 | ✅ | 5 → Cpu | — | — | — | — | 0 |
| B — PPU | 18 | ✅ | — | — | 18 → Ppu | — | — | 0 |
| C — APU | 11 | ✅ | — | — | — | 11 → Apu | — | 0 |
| D — Bus | 6 | ✅ | — | 6 → Bus | — | — | — | 0 |
| E — Cart | 24 | ✅ | — | — | — | — | 24 → Cart | 0 |
| F — Config | 9 | ✅ | — | — | — | — | — | ≤ 6 |
| G — Debug | 25 | ✅ | — | — | — | — | — | 仅调试构建可见 |
| **总计** | **101** | **100%** | **5** | **6** | **18** | **11** | **24** | **≤ 6** |

---

## 10. 与 v1.3+ 的衔接

`fceu11::State`（v1.2 §2.2）当前为"全局变量引用聚合视图"。v1.3 起逐步替换：

```
v1.2:  class State { auto& cpu(); ... }   // 引用 ::X, ::timestamp, ...
v1.3:  class Cpu  { X6502 layout_; ... }   // ::X 变为 alias
v1.4:  class Bus  { alignas(64) ...; }     // ::Page 变为 alias
v1.5:  class Ppu  { uint8 regs_[4]; ... }   // ::PPU 变为 alias
...
```

每个子版本的兼容别名（`inline auto& PPU = state.ppu().regs_ref()`）允许 100+ 调用点按文件分批迁移，避免一次性 break-all。

---

## 11. 验收（v1.2）

- [x] **101/101** file-scope extern 声明已分类到 A-G
- [x] 每个全局标记写入/读取文件数（基于 `grep -c` 采样）
- [x] `using namespace std` 6 处 file-scope 命中确认
- [x] 输出至 `docs/internal/global_state_audit.md`