# PHASE F 接力文档 — v1.13 Purify malloc/free 根除

> **状态**：🟡 部分完成（部分子阶段进行中）
> **基线**：v1.13 Purify build plan §6.1-§6.5
> **当前分支**：`wip`
> **已提交进度**：F1 完整；F2 全部 5 批完成；F3 进行中遇到阻塞
> **日期**：2026-07-09

---

## 1. 已完成（已提交）

### ✅ F1 — 审计与分类
- 文档：`docs/internal/F1_malloc_audit.md`（123 行）
- 分类：~106 处 T_A/T_B 业务代码 + 5 处 T_D 包装器内部 + ~50 处 T_E 不处理
- 提交：850d348 "F1 audit + F2c/F3b watchpoint condText/desc to std::string"

### ✅ F2c — 核心辅助模块
- `src/cheat.cpp` — strdup+free → std::string
- `src/debug.h` + `src/debug.cpp` — watchpointinfo.condText/desc: char*→std::string
- `src/palette.cpp` — grayscaled_palo: pal*→std::unique_ptr<pal[]>
- 提交：753d8f7, 850d348

### ✅ F3b (部分) — watchpoint 跨文件 RAII
- `src/drivers/Qt/ConsoleDebugger.cpp` — 8 处 wp->condText/desc → .empty()/.c_str()
- `src/drivers/Qt/TraceLogger.cpp` — 3 处 → .empty()/.c_str()
- 提交：850d348

### ✅ F2b — 核心生命周期（fceu/cart/ines_load/unif_load/oldmovie）
- `src/git.h` — FCEUGI.filename/archiveFilename: char*→std::string
- `src/fceu.cpp` — AReadG/BWriteG: FCEU_malloc+free→std::vector；2 处 strdup→std::string
- `src/cart.cpp` — GENIEROM: FCEU_malloc+free→FceuMallocPtr
- `src/ines_load.cpp` — free(VROM)→FCEU_free（匹配 FCEU_malloc）
- `src/unif_load.cpp` — 3 处 free→FCEU_free
- `src/oldmovie.cpp` — moviedata: realloc+free→std::vector + try/catch
- 提交：979fea5, f05b4ac, 32bb6a2

### ✅ F2a — FFI 边界（fds/nsf）
- `src/nsf.cpp` — free(NSFDATA)→FCEU_free
- `src/fds.cpp` — 4 处 free→FCEU_free/FceuMallocPtr owner.reset()；2 处 strdup→std::string
- 提交：9d54045

### ✅ F2d — 热路径（netplay）
- `src/netplay.cpp` — 9 处 free→FCEU_free；FCEU_dmalloc→FCEU_malloc；1 处 raw malloc→FCEU_malloc
- 提交：0865210

---

## 2. F3 进行中 — Qt 驱动文件

### 🟡 F3a — 进行中遇到阻塞
**已完成**：
- `src/drivers/Qt/AviRecord.cpp` — rawVideoBuf/rawAudioBuf: malloc+free→std::unique_ptr<T[]>（编辑已保存，编译失败）
- `src/drivers/Qt/AviRecordDiskThread.cpp` — rgb24/audioOut/videoOut: malloc+free→std::unique_ptr<T[]>（编辑已保存，未编译）

**当前阻塞错误**：
```
D:\Project\FCEUX11\src\drivers\Qt\AviRecord.cpp(70): error C2040:
  "rawVideoBuf":"std::unique_ptr<uint32_t [],...>" 与 "uint32_t *" 的间接寻址级别不同
D:\Project\FCEUX11\src\drivers\Qt\AviRecord.cpp(71): error C2040: rawAudioBuf 同上
D:\Project\FCEUX11\src\drivers\Qt\AviRecord.cpp(503/506): error C2440:
  "=": 无法从 "std::unique_ptr<...>" 转换为 "uint32_t *"
```

**可能原因**：
- `AviRecord.cpp` 70 行的 `std::unique_ptr` 声明与某处**前向声明**或**外部声明**冲突
- 可能是 `AviRecordDiskThread.cpp` 等其他 TU 仍把 `rawVideoBuf` 视为 `uint32_t*`（extern 隐式）
- 已删除 `.obj` 强制重建仍失败 → 排除缓存
- 已确认 `AviRecord.h` 不声明 `rawVideoBuf`

**下一步排查**：
1. 用 `grep -rn "rawVideoBuf\|rawAudioBuf" src/` 找所有引用
2. 可能是 `AviRecordDiskThread.cpp` 第 115/181 行仍用 `rawVideoBuf[head]` 隐式转换
3. 解决方案：要么在 `AviRecord.cpp` 用 `extern std::unique_ptr<uint32_t[]>` 声明；要么在消费者加 `.get()`
4. **实际更可能**：`AviRecordDiskThread.cpp` 是另一个 TU，链接时符号冲突。改为 `AviRecord.cpp` 头里用 extern 暴露

### ⏳ F3b — 待办
- `src/drivers/Qt/HexEditor.cpp` — 9 处 free/malloc
- `src/drivers/Qt/CodeDataLogger.cpp` — 5 处
- `src/drivers/Qt/ConsoleDebugger.cpp` — 还有 4 处 watchpoint free（已在 F3b 部分处理）

### ⏳ F3c — 待办
- `src/drivers/Qt/ConsoleViewerGL.cpp` — 2 处
- `src/drivers/Qt/ConsoleViewerQWidget.cpp` — 2 处
- `src/drivers/Qt/ConsoleViewerSDL.cpp` — 2 处
- `src/drivers/Qt/nes_shm.cpp` — 2 处
- `src/drivers/Qt/sdl-sound.cpp` — 1 处
- `src/drivers/Qt/fceu_archive.cpp` — 2 处
- `src/drivers/Qt/iNesHeaderEditor.cpp` — 2 处
- `src/drivers/Qt/LuaControl.cpp` — 2 处
- `src/drivers/Qt/TraceLogger.cpp` — 还有 4 处 TraceLogger 自己的 malloc（已修过 watchpoint 部分）

### ⏳ F3d — 待办
- `src/drivers/Qt/TasEditor/bookmarkPreviewPopup.cpp` — 6 处
- `src/drivers/Qt/avi/gwavi.cpp` — 5 处（注意：含 `data.malloc()` 第三方 API）
- `src/drivers/Qt/AviRiffViewer.cpp` — 5 处 `data.malloc()`（类成员函数）

---

## 3. 关键文件清单（按优先级）

| 优先级 | 文件 | malloc/free 数 | 备注 |
|--------|------|---------------|------|
| 🔴 阻塞中 | `src/drivers/Qt/AviRecord.cpp` | 4 | unique_ptr 编译失败 |
| 🟡 同 TU | `src/drivers/Qt/AviRecordDiskThread.cpp` | 6 | unique_ptr 已编 |
| 🟢 简单 | `src/drivers/Qt/ConsoleViewerGL.cpp` | 2 | localBuf: unique_ptr |
| 🟢 简单 | `src/drivers/Qt/ConsoleViewerQWidget.cpp` | 2 | localBuf: unique_ptr |
| 🟢 简单 | `src/drivers/Qt/ConsoleViewerSDL.cpp` | 2 | localBuf: unique_ptr |
| 🟢 简单 | `src/drivers/Qt/nes_shm.cpp` | 2 | nes_shm: unique_ptr |
| 🟢 简单 | `src/drivers/Qt/sdl-sound.cpp` | 1 | s_Buffer: unique_ptr |
| 🟢 简单 | `src/drivers/Qt/fceu_archive.cpp` | 2 | tmpMem: unique_ptr |
| 🟢 简单 | `src/drivers/Qt/iNesHeaderEditor.cpp` | 2 | iNesHdr: unique_ptr |
| 🟢 简单 | `src/drivers/Qt/LuaControl.cpp` | 2 | buf: unique_ptr |
| 🟡 中等 | `src/drivers/Qt/HexEditor.cpp` | 9 | 多个不同生命周期 |
| 🟡 中等 | `src/drivers/Qt/CodeDataLogger.cpp` | 5 | cdloggerdata/vdata |
| 🟡 中等 | `src/drivers/Qt/TraceLogger.cpp` | 4 | recBuf/logBuf |
| 🟡 中等 | `src/drivers/Qt/TasEditor/bookmarkPreviewPopup.cpp` | 6 | pixBuf/screenShotRaster |
| 🔴 复杂 | `src/drivers/Qt/avi/gwavi.cpp` | 5 | 含第三方类 API |

---

## 4. 已有工具（必读）

### FceuMallocPtr RAII
文件：`src/utils/memory.h`
```cpp
struct FceuMallocDeleter {
    void operator()(uint8_t* p) const noexcept { if (p) FCEU_gfree(p); }
};
using FceuMallocPtr = std::unique_ptr<uint8_t[], FceuMallocDeleter>;
inline FceuMallocPtr FCEU_gmalloc_unique(size_t size) {
    return FceuMallocPtr(static_cast<uint8_t*>(FCEU_gmalloc(size)));
}
```
用途：FCEU_gmalloc 包装的 RAII 等价物；和 FCEU_malloc 不同（FCEU_malloc 用 _aligned_malloc）

### 标准模式
- 简单字节缓冲：```std::unique_ptr<T[]> ptr = std::make_unique<T[]>(n);```
- FCEU_malloc/FCEU_gmalloc 缓冲：```FceuMallocPtr ptr = FCEU_gmalloc_unique(n);```
- 字符串：```std::string s = FCEU_MakeFName(...);```

### 重要保留习惯
- 保留 `fceuScopedPtr` typedef（§9 范围，但还没实施）
- 不动 Lua 5.1 内嵌代码（§10 删除整个 src/lua/src/）
- 不动 rust/target 生成文件
- 不动第三方：hq2x/hq3x/scalebit/emu2413/nes_ntsc
- 不动 `archived/` 目录

---

## 5. 构建系统

### Windows 构建
- Generator: NMake Makefiles
- MSVC: VS BuildTools 18, vcvars64.bat
- 工具脚本（在 `scripts/`）：
  - `scripts/build_check.bat` — 单 target 构建并重定向到 `D:\tmp_build.log`
  - `scripts/cmake_regen.bat` — 重新生成 CMake 配置
- 调用方法（必须用 `//c` 转义）：
  ```bash
  cmd //c "D:\Project\FCEUX11\scripts\build_check.bat" 2>&1
  ```
  错误用 `grep -E "error C|error:" /d/tmp_build.log | head -10`

### 构建命令变体
- 完整：`nmake /f Makefile fceux11 > D:\tmp_build.log 2>&1`
- 仅 core：`nmake /f Makefile fceux11_core`
- 跑测试：`ctest -C Release -LE perf`

### 已知坑
- 删除 PCH 后必须 `cmake . -G "NMake Makefiles"` 重新生成 `cmake_pch.cxx`
- 用 `cmd /c` 在 MSYS bash 中会被吞输出；必须用 `cmd //c` 或 `cmd.exe /c`
- MSVC `/W4 /WX` 默认被 PCH 重写为 `/WX-`（这是历史原因，不需修复）

---

## 6. 验收门禁（§6.5 + §12.2）

PHASE F5 完成时必须验证：
```bash
# 1. 零业务代码裸 malloc/free（仅允许 T_D 包装器内部、T_E 注释/字符串）
git grep -nE '\b(malloc|free|calloc|realloc)\b' --include='*.cpp' --include='*.h' src/ \
  | grep -vE 'free software|free of charge|Permission is hereby|lua/src|rust/target|src/boards/emu2413|drivers/common/(hq2x|hq3x|scalebit|vidblit|config|args)|archived/|scoped_ptr.h|memory\.(h|cpp)|gwavi|free alternative'

# 2. 编译门禁
cmd //c "D:\Project\FCEUX11\scripts\build_check.bat" 2>&1  # ExitCode=0

# 3. 回归测试
ctest -C Release -LE perf
```

---

## 7. 接力任务清单

> 下一个 agent 接手后建议按以下顺序：

1. **🔴 解决 AviRecord.cpp unique_ptr 编译错误**
   - 查 `git grep rawVideoBuf src/` 找出所有 TU
   - 可能需要在 `AviRecord.cpp` 用 `static` 限定为内部符号，或在 `AviRecordDiskThread.cpp` 用 `.get()`
2. **🟢 完成 F3c/F3d**（简单 unique_ptr 替换）
3. **🟡 完成 F3b**（HexEditor/CodeDataLogger/TraceLogger）
4. **🟡 实施 F4** — `src/utils/memory.cpp` FCEU_gmalloc 内部改 `new (std::nothrow) T[]`（计划 §6.4）
5. **✅ 跑 F5 验收门禁**（grep + ctest + CHANGELOG 更新）

### 续接起点命令
```bash
cd "D:/Project/FCEUX11"
git log --oneline -10  # 查看进度
git status -s           # 查看未提交修改（含 AviRecord/AviRecordDiskThread）
cat docs/internal/F1_malloc_audit.md  # 复习审计数据
cat docs/internal/PHASE_F_HANDOFF.md  # 本文档
```

---

## 8. 关键提交 hash

```
850d348 F1 audit + F2c/F3b watchpoint condText/desc to std::string
753d8f7 F2c palette.cpp grayscaled_palo -> std::unique_ptr<pal[]>
979fea5 F2b fceu.cpp + FCEUGI fields -> std::string/std::vector
f05b4ac F2b cart.cpp GENIEROM -> FceuMallocPtr RAII
32bb6a2 F2b ines_load/unif_load/oldmovie.cpp free() modernization
9d54045 F2a nsf.cpp + fds.cpp free() modernization (FFI boundary)
0865210 F2d netplay.cpp free() -> FCEU_free() (FCEU_malloc allocations)
```

7 commits ahead of `origin/wip`. F2 + F3a（部分） + F3b（部分）已落地。
