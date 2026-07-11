# FCEUX11 v1.15 空指针解引用审计报告

> **报告性质**：v1.15 Finale 发布前 CODE REVIEW 发现项，建议 v2.0 修复  
> **编制日期**：2026-07-11  
> **版本**：v1.0  
> **状态**：已知问题，正常使用场景触发概率极低，暂不修复（v1.15 LTS 无计划做大修改）

---

## 摘要

在 v1.15 Finale 发布前 CODE REVIEW 中，审查了 `src/` 目录下核心路径的空指针安全性，发现 **5 处** 空指针解引用风险。这些缺陷的共同特征为：全局指针（`GameInfo`、`currCartInfo`、`XBackBuf`）在程序初始化前或 ROM 加载前的空窗期内被无条件解引用。

当前正常使用场景下（模拟器启动 → 加载 ROM → 运行），这些缺陷的触发条件不满足，因此不影响 v1.15 LTS 版本的日常使用。建议在 v2.0 大版本中统一加固。

---

## 缺陷列表

### 缺陷 1：`currCartInfo` 空指针解引用 — 录像回放/录制

| 项目 | 详情 |
|------|------|
| **文件** | `src/movie_io.cpp` |
| **行号** | 167, 173, 177, 183, 189, 201, 202, 204, 209, 210 |
| **严重度** | HIGH |
| **函数** | `MovieData::loadSaveramFrom()` / `MovieData::dumpSaveramTo()` |

**描述**：`currCartInfo` 是全局 `CartInfo*` 指针，仅在两处赋值（`src/ines.cpp:63`、`src/unif.cpp:49`），ROM 加载前为 `nullptr`。录像回放代码无条件解引用该指针访问 `SaveGame` 向量和 `battery` 字段，无 null 守卫。

**触发条件**：在未加载 ROM 时执行录像回放/录制（movie playback/record）。正常 UI 流程下，录像操作前必须先加载 ROM，因此触发概率极低。

**建议修复**：
```cpp
if (!currCartInfo) return false;
```

---

### 缺陷 2：`XBackBuf` 空指针解引用 — 即时存档

| 项目 | 详情 |
|------|------|
| **文件** | `src/state.cpp` |
| **行号** | 361 |
| **严重度** | HIGH |
| **函数** | `FCEUSS_SaveMS()` |

**描述**：`XBackBuf` 是全局 `uint8*`，由 `FCEU_InitVirtualVideo()` 分配、`FCEU_KillVirtualVideo()` 释放。`FCEUSS_SaveMS()` 中的 `memcpy(buf.data(), XBackBuf, 256*256)` 无条件拷贝 64KB 数据，未检查指针是否为 null。

同样的问题也存在于：
- `src/fceu.cpp:851` — `ResetNES()` 中的 `memset(XBackBuf, 0, 256*256)`
- `src/fceu.cpp:1002` — `PowerNES()` 中的 `memset(XBackBuf, 0, 256*256)`

**触发条件**：在视频初始化前或在 `KillVirtualVideo` 后触发存档操作。正常流程下模拟器初始化 → 加载 ROM → 运行，此时 `XBackBuf` 已分配且有效，触发概率极低。

**建议修复**：
```cpp
if (XBackBuf) {
    memcpy(buf.data(), XBackBuf, 256 * 256);
}
```

---

### 缺陷 3：`GameInfo->type` 空指针解引用 — Code/Data Logger (CDL)

| 项目 | 详情 |
|------|------|
| **文件** | `src/debug.cpp` |
| **行号** | 248 |
| **严重度** | MEDIUM |
| **函数** | `GetPRGAddress()` |

**描述**：`GetPRGAddress()` 直接访问 `GameInfo->type` 无 null 守卫。该函数被音频模拟路径调用：`sound.cpp:130` 中的 `LogDPCM()` → `GetPRGAddress()`。如果 CDL 功能在 ROM 加载前启用，此链式调用将崩溃。

**触发条件**：启用 Code/Data Logger 功能时未加载 ROM。正常使用场景下 CDL 功能依赖 ROM 存在。

**建议修复**：
```cpp
if (!GameInfo) return -1;
```

---

### 缺陷 4：`GameInfo->type` 空指针解引用 — Debug 构建

| 项目 | 详情 |
|------|------|
| **文件** | `src/debug.cpp` |
| **行号** | 917 |
| **严重度** | MEDIUM |
| **函数** | `DebugCycle()` |

**描述**：Debug 构建中 (`types.h:45` 将 `DEBUG(X)` 展开为 `X;`)，`DebugCycle()` 在**每条 CPU 指令**上被调用。该函数无条件访问 `GameInfo->type` 无 null 守卫。

**触发条件**：Debug 构建中启动调试器时未加载 ROM。

**建议修复**：
```cpp
if (!GameInfo) return;
```

---

### 缺陷 5：`GameInfo->type` 空指针解引用 — Qt GUI 调试回调

| 项目 | 详情 |
|------|------|
| **文件** | `src/drivers/Qt/CodeDataLogger.cpp:805`、`src/drivers/Qt/ppuViewerContext.cpp:65`、`src/drivers/Qt/HexEditor.cpp:258` |
| **严重度** | MEDIUM |
| **函数** | `InitCDLog()` / `getPPU()` / 匿名函数 |

**描述**：Qt GUI 调试工具的回调函数可在用户点击 UI 控件时被调用，其中多个函数直接访问 `GameInfo->type` 无 null 守卫。与同文件中已有正确处理形成对比——如 `CodeDataLogger.cpp:572` 处有 `if (!GameInfo) return;`，但 805 行遗漏。

**触发条件**：未加载 ROM 时打开调试工具窗口（如 Code/Data Logger、PPU Viewer、Hex Editor）。

**建议修复**：参照同文件中已有守卫模式，在函数入口添加 `if (!GameInfo) return;`。

---

## 风险评估

| 缺陷 | 正常使用触发 | Debug 构建触发 | 恶意触发 | 建议修复版本 |
|------|:----------:|:------------:|:------:|:----------:|
| 1 - currCartInfo | 极低 | 极低 | 可能 | v2.0 |
| 2 - XBackBuf | 极低 | 极低 | 否 | v2.0 |
| 3 - GetPRGAddress | 低 | 低 | 否 | v2.0 |
| 4 - DebugCycle | 否 | 低 | 否 | v2.0 |
| 5 - Qt GUI 回调 | 低 | 低 | 否 | v2.0 |

**结论**：5 处缺陷在正常使用场景下触发概率均极低，v1.15 LTS 版本无需立即修复。建议在 v2.0 版本中统一采用 RAII 或 null 守卫模式加固全局指针访问。
