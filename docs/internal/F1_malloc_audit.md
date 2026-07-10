# PHASE F1 — malloc/free 审计与分类

> **状态**：✅ 完成（F1 任务 #1）
> **日期**：2026-07-09
> **方法**：`grep -rnE '\b(malloc|free|calloc|realloc)\b' --include='*.cpp' --include='*.h' --include='*.c' src/`，过滤 license 头注释
> **基线**：v1.13 Purify build plan §6.1

---

## 1. 全代码库扫描数据

| 范围 | 原始 grep 数 | 过滤 license 头 | 实际代码级 |
|------|------------|---------------|----------|
| 整个 `src/` | 587+ | — | — |
| 过滤 license/注释/字符串 | — | 292 | **~197**（排除 Lua 内嵌代码和 rust target 生成产物） |
| `src/lua/src/` (Lua 5.1 内嵌) | 36 | 36 | ⚠️ 将在 §10 中整体删除 |
| `src/rust/target/` (Rust 生成) | 8 | 8 | ⚠️ 不处理（自动生成） |

**计划估算 vs 实际**：
- 计划估算 414 处 → 实际代码级 ~197 处（去掉 Lua 36、rust target 8、license header 295+）

---

## 2. F2/F3 范围内的实际 malloc/free 清单

### 2.1 F2 核心 src/ 根目录 — 实际 malloc/free

| 文件 | 实际次数 | 分类 | 备注 |
|------|---------|------|------|
| `src/fds.cpp` | 8 | T_B (FCEU_malloc) → T_A/B/C 混合 | FDSBIOS/FDSRAM/CHRRAM/diskdata |
| `src/fceu.cpp` | 7 | T_A（filename/archiveFilename/GameInfo->name/AReadG/BWriteG） | 字符串/函数指针数组 |
| `src/cart.cpp` | 5 | T_A (GENIEROM) + T_E (注释) | Game Genie ROM |
| `src/debug.cpp` | 5 | T_B (watchpoint.condText/desc) | 调试器字符串 |
| `src/cheat.cpp` | 2 | T_B (fn) | 字符串 |
| `src/netplay.cpp` | 10 | T_A (buf/cbuf/tbuf) | 网络缓冲 |
| `src/nsf.cpp` | 1 | T_B (NSFDATA) | NSF 数据 |
| `src/ines_load.cpp` | 1 | T_B (VROM) | iNES 加载 |
| `src/oldmovie.cpp` | 3 | T_B (moviedata via realloc) | 旧版 movie |
| `src/unif_load.cpp` | 3 | T_B (UNIFchrrama/boardname/malloced) | UNIF 加载 |
| `src/palette.cpp` | 2 | T_B (grayscaled_palo) | 调色板缓存 |
| `src/state.cpp` | 0 | T_E（仅注释提及 malloc） | — |
| `src/video.cpp` | 0 | T_E（仅注释提及 free） | — |
| `src/utils/memory.cpp` | 5 | **T_D**（FCEU_malloc/FCEU_gmalloc/FCEU_realloc 实现内部） | §6.4 F4 范围 |
| `src/utils/memory.h` | 6 | **T_D**（声明）+ 注释 | §6.4 F4 范围 |

**F2 范围实际业务代码 malloc/free 调用：~48 处**（不含 T_D）

### 2.2 F3 Qt 驱动 — 实际 malloc/free

| 文件 | 实际次数 | 分类 | 备注 |
|------|---------|------|------|
| `src/drivers/Qt/AviRecord.cpp` | 4 | T_A (rawVideoBuf/rawAudioBuf) | 录制缓冲 |
| `src/drivers/Qt/AviRecordDiskThread.cpp` | 6 | T_A (rgb24/audioOut/videoOut) | 录制后台线程 |
| `src/drivers/Qt/AviRiffViewer.cpp` | 5 | **T_D** (data.malloc 是 gwavi_dataBuffer 成员函数) | 不需替换 |
| `src/drivers/Qt/CodeDataLogger.cpp` | 5 | T_A (cdloggerdata/cdloggervdata) | 代码日志 |
| `src/drivers/Qt/ConsoleDebugger.cpp` | 4 | T_B (watchpoint.condText/desc) | 调试器 |
| `src/drivers/Qt/ConsoleViewerGL.cpp` | 2 | T_A (localBuf) | 显示缓冲 |
| `src/drivers/Qt/ConsoleViewerQWidget.cpp` | 2 | T_A (localBuf) | 显示缓冲 |
| `src/drivers/Qt/ConsoleViewerSDL.cpp` | 2 | T_A (localBuf) | 显示缓冲 |
| `src/drivers/Qt/HexEditor.cpp` | 9 | T_A (data/modMem/entry->data/buf) | hex 编辑 |
| `src/drivers/Qt/LuaControl.cpp` | 2 | T_A (buf) | Lua 控件 |
| `src/drivers/Qt/TraceLogger.cpp` | 4 | T_A (recBuf/logBuf) | trace 缓冲 |
| `src/drivers/Qt/fceu_archive.cpp` | 2 | T_A (tmpMem) | 归档解压 |
| `src/drivers/Qt/iNesHeaderEditor.cpp` | 2 | T_A (iNesHdr) | iNES 编辑 |
| `src/drivers/Qt/nes_shm.cpp` | 2 | T_A (nes_shm) | 共享内存 |
| `src/drivers/Qt/sdl-sound.cpp` | 1 | T_A (s_Buffer) | SDL 音频 |
| `src/drivers/Qt/TasEditor/bookmarkPreviewPopup.cpp` | 6 | T_A (pixBuf/screenShotRaster) | 书签预览 |
| `src/drivers/Qt/avi/gwavi.cpp` | 5 | T_A (readBuf/buf/palette) + 1 类成员函数 | gwavi 第三方代码 |
| `src/drivers/Qt/TasEditor/TasEditorWindow.cpp` | 0 (实际) | `markersManager.free()` 等是类成员函数名 | 不需替换 |

**F3 范围实际业务代码 malloc/free 调用：~58 处**（不含 T_D / 类成员函数）

---

## 3. 分类法回顾

| 类别 | 描述 | 替换方案 |
|------|------|----------|
| **T_A** | 简单裸 `malloc(T)`/`free(p)` 配对，无复杂所有权 | 直接 `std::unique_ptr<T[]>` / `std::vector<T>` |
| **T_B** | 已通过 `FCEU_malloc`/`FCEU_gmalloc` 包装 | 改用 `FceuMallocPtr` RAII 或 `FCEU_gmalloc_unique()` |
| **T_C** | 跨文件/跨模块所有权，需审计配对 | 逐对替换 + 单元测试 |
| **T_D** | `FCEU_gmalloc`/`FCEU_gcalloc` 包装器内部实现 | §6.4 F4：内部 `new (std::nothrow)` 化 |
| **T_E** | 注释/字符串/lua 5.1 内嵌（待 §10 删除）/ rust target 生成 | 不处理（删除目录或自动生成） |

---

## 4. 迁移顺序确认（依据 §6.2/§6.3）

| 批次 | 文件 | 估算 malloc/free 数 | 风险 |
|------|------|-------------------|------|
| **F2a** | `fds.cpp` / `nsf.cpp` | 9 | 🟡（Rust FFI 边界） |
| **F2b** | `fceu.cpp` / `cart.cpp` / `state.cpp` / `ines_load.cpp` / `unif_load.cpp` / `oldmovie.cpp` | 19 | 🟡（核心生命周期） |
| **F2c** | `cheat.cpp` / `debug.cpp` / `conddebug.cpp` / `video.cpp` / `palette.cpp` | 9 | 🟢（辅助模块） |
| **F2d** | `x6502.cpp` / `ppu.cpp` / `sound.cpp` / `input.cpp` / `netplay.cpp` | 13 | 🔴（热路径/网络） |
| **F2e** | `utils/xstring.cpp` / `utils/unzip.cpp` / `utils/memory.h` | 0 + 2 | 🟢（utils，注释/宏） |
| **F3a** | `AviRecord.cpp` / `AviRecordDiskThread.cpp` / `avi/gwavi.cpp` | 15 | 🟡（录制冒烟） |
| **F3b** | `HexEditor.cpp` / `CodeDataLogger.cpp` / `ConsoleDebugger.cpp` | 18 | 🟢（调试/编辑） |
| **F3c** | `ConsoleViewerGL.cpp` / `ConsoleViewerQWidget.cpp` / `ConsoleViewerSDL.cpp` / `nes_shm.cpp` / `sdl-sound.cpp` / `fceu_archive.cpp` / `iNesHeaderEditor.cpp` / `LuaControl.cpp` / `TraceLogger.cpp` | 28 | 🟢（GUI） |
| **F3d** | `TasEditor/bookmarkPreviewPopup.cpp` / `TasEditor/TasEditorWindow.cpp` (无实际 malloc) | 6 | 🟢（TasEditor） |
| **F4** | `utils/memory.cpp` (FCEU_gmalloc 内部实现) | 5 | 🟡（包装器内部） |

---

## 5. 不纳入 F2/F3 范围（已声明或自动处理）

- `src/lua/src/*.c` (36 处) — §10 中整体删除 Lua 5.1 内嵌代码
- `src/rust/target/**` (8 处) — Rust 自动生成的 C 头，不修改
- `src/utils/scoped_ptr.h` (2 处) — §9 中整体移除
- `src/archived/fceux-server/server.cpp` (16 处) — 已归档代码（archived/）
- `src/drivers/common/hq2x.cpp` / `hq3x.cpp` (4 处) — 第三方 HQ2x/HQ3x 库，保留
- `src/drivers/common/scalebit.cpp` / `vidblit.cpp` / `config.cpp` / `args.cpp` — 第三方或 utils
- `src/boards/emu2413.c` — 第三方 emu2413 OPLL 模拟器

---

## 6. F1 结论

- **T_A/T_B 总数**：~106 处业务代码 malloc/free 需替换（F2+F3）
- **T_D 总数**：5 处包装器内部（F4 处理）
- **T_E 总数**：~50 处（注释/字符串/外部代码/自动生成），不处理
- **验证门禁**：§6.5 grep 门禁将核查业务代码零裸 `malloc/free/calloc/realloc`

**F1 完成**。下一步进入 F2 替换实施。