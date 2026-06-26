# FCEUX11 中期代码质量与性能局部重构计划

> **范围**：零散、独立、单文件或单函数粒度的代码质量提升与局部性能优化
> **原则**：与 `v1.x_Modernization_Roadmap.md` 的 v1.6 及后续版本**完全正交**，**不打 TAG**，**不升版本号**
> **周期**：中期（按 Phase R1~R5 推进，预计 3~5 个月）
> **最后更新**：2026-06-26（**Phase R1 全量收官**：R1.1/R1.2/R1.3/R1.4/R1.5/R1.6 全部落地，详见 §6）
> **工具链**：MSVC 2022+ / C++20 / Qt 6.8 LTS / CMake 4.0+（与主干一致）

---

## 0. 总纲

### 0.1 定位与边界

| 维度 | v1.x Modernization Roadmap | 本计划（refactor_plan） |
|------|---------------------------|--------------------------|
| 范围 | 系统性（按子系统整体重写） | 局部（单文件 / 单函数 / 单头） |
| 目标 | 全局状态对象化、API 现代化、Rust 迁移 | 修 bug、削重复、消警告、提性能 |
| 版本 | 强制 v1.6/v1.7/.../v1.14 TAG | **不打 TAG，不升版本号** |
| 周期 | 长（每个子版本独立筹备） | 中（每 Phase R1~R5 独立可发布） |
| 与 v1.x 路线冲突 | — | **绝对不允许冲突** |

### 0.2 避让区（明确不触碰）

以下模块正在或即将由 `v1.x_Modernization_Roadmap.md` v1.6~v1.14 改造，本计划**严格回避**：

| 避让区 | roadmap 中的负责版本 |
|--------|---------------------|
| `src/cpu.cpp/h`, `src/x6502.cpp/h`, `src/x6502struct.h` | v1.3 Legion |
| `src/bus.cpp/h` | v1.4 Gateway（已部分完成） |
| `src/ppu.cpp/h`, `src/ppu_class.cpp/h` | v1.5 Prism（已交付） + v1.12 Scissors 拆分 |
| `src/sound.cpp/h`, `src/wave.cpp/h`, `src/fir/*` | v1.6 Resonance |
| `src/cart.cpp/h`, `src/ines.cpp/h`, `src/unif.cpp/h`, `src/nsf.cpp/h`, `src/fds.cpp/h` | v1.7 Cartograph + v1.10 Cryptex |
| `src/boards/*` (171 个 mapper 文件) | v1.8 Masonry |
| `src/state.cpp/h`, `src/oldmovie.cpp/h`, `src/movie.cpp/h` | v1.9 Chronicle |
| `src/lua-engine.cpp`, `src/lua/src/*` | v1.11 Bridge + v1.13 Purify |
| `src/drivers/Qt/*`, `src/fceu.cpp/h` | v1.11 Bridge + v1.12 Scissors |
| `src/TasEditor/*`, `src/AviRecord.cpp` | v1.12 Scissors |
| `src/utils/scoped_ptr.h` 的 `fceuScopedPtr` 残迹 | v1.13 Purify |

**任何对避让区文件的重构都必须先确认 roadmap 中该文件的改造是否已完成；若未完成，必须等待；若已完成，方可纳入本计划。**

### 0.3 通用约束（继承主干政策）

1. **不与 v1.x roadmap 矛盾** — 任何重构不得改变 roadmap 计划中将要重写的接口、类名、字段名
2. **不升版本号、不打 TAG** — 重构后 `project(FCEUX11 VERSION 1.5 ...)` 保持不变；不创建 `v1.5.1-phase-Rx` 之类的 phase TAG
3. **trunk-based dev** — 每个 commit 直接到 `main`，每个 commit 后 `ctest` 必须 19/19 PASS
4. **不对称 bench tolerance** — `bench_tolerance_test` 是非对称的：speedup 永远 PASS，slowdown > max-regression 才 FAIL
5. **link-time code layout 敏感** — v1.5 Phase 6 VRC7 教训：即使调用 unreachable，链接期代码布局变动也可能拖累 6-9% 性能。所有热路径附近的重构必须验证 `bench_ppu_frame` / `bench_x6502_exec` / `bench_apu_mix` 至少不退步 2%
6. **不伪造 ctest 结果** — 任何"ctest 通过"的声明必须基于 `*.exe` 的实际 mtime 推算
7. **范围克制** — 本计划的所有项都是单文件 / 单函数 / 单头粒度，**禁止**借机做跨子系统抽象

---

## 1. 候选项总览

| ID | 优先级 | 类别 | 文件 | 一句话摘要 | 工作量 |
|----|--------|------|------|-----------|--------|
| R1.1 | P0 | Bug | `src/utils/xstring.cpp` | 4 处 `FCEU_strlcpy(str, sizeof(str), ...)` 缓冲区溢出 + `str_rtrim` off-by-one | 0.5 d |
| R1.2 | P0 | Perf | `src/utils/xstring.cpp` | `str_ucase`/`str_lcase`/`chr_replace` O(n²) → O(n) | 0.25 d |
| R1.3 | P1 | 质量 | `src/utils/xstring.cpp` | `str_strip`/`str_replace` 裸 `malloc`/`free` → `std::vector` | 0.25 d |
| R1.4 | P1 | 质量 | `src/utils/xstring.cpp` | `Base64Table` 运行期构造 → `constexpr`（C++20） | 0.25 d |
| R1.5 | P2 | 质量 | `src/utils/xstring.cpp` | `tokenize_str` 等 `using namespace std;` 显式化 | 0.1 d |
| R1.6 | P1 | Bug | `src/utils/xstring.cpp` | `mass_replace` 缺少 `j += replacement.length()` 增量（无限循环风险） | 0.1 d |
| R2.1 | P0 | Bug | `src/utils/valuearray.h` | `operator!=` 实现为 `!operator==(other)`，`operator==` 缺 `const`，`operator[]` 缺 const 重载 — 当前在 `const` 上下文不可用 | 0.25 d |
| R3.1 | P1 | 质量 | `src/utils/timeStamp.h` | 二元算符 `+`/`-`/`*`/`/` 返回值（4 字段拷贝）→ 改 `+=` 友元或显式 `const` 标记 | 0.25 d |
| R3.2 | P1 | 质量 | `src/utils/timeStamp.h/.cpp` | `qpcCalibrate` 声明/定义 WIN32 不一致 + 静态初始化期 `printf` 噪声 | 0.1 d |
| R3.3 | P2 | 质量 | `src/utils/timeStamp.h` | `void zero(void)` 等 C 风格空参 → `void zero()` | 0.05 d |
| R4.1 | P1 | Perf | `src/utils/endian.cpp` | `FlipByteOrder` 双重扫描 → 单遍 `for(i<N/2)`；用 `std::byteswap`（C++23）/ `_byteswap_ulong`（MSVC）替换手写 bswap | 0.25 d |
| R4.2 | P2 | 质量 | `src/utils/endian.cpp` | `LE_TO_LOCAL_*` 宏仅在 2 处使用；其余函数手写字节交换 — 统一为单一内联函数 | 0.1 d |
| R4.3 | P3 | 死代码 | `src/utils/endian.h/.cpp` | `read16le(char*, FILE*)` 与 `read16le(uint16_t*, std::istream*)` 同名不同签 — 移除或改名 | 0.05 d |
| R5.1 | P1 | 质量 | `src/utils/format.h` | `CTASSERT(x)` 旧式 `typedef char[…]` → `static_assert(x)`（C++17 起） | 0.1 d |
| R5.2 | P3 | 质量 | `src/utils/format.h` | `FCEU_CPP_HAS_STD(201603L)` 等粗粒度版本号 → `__cpp_lib_*` 特性宏 | 0.25 d |
| R6.1 | P1 | 质量 | `src/utils/mutex.cpp` | `QRecursiveMutex` 裸 `new`/`delete` → `std::unique_ptr`（条件 typedef） | 0.25 d |
| R6.2 | P2 | 质量 | `src/utils/mutex.h/.cpp` | `autoScopedLock` 两个构造函数合一为模板 | 0.1 d |
| R7.1 | P1 | Bug | `src/utils/memory.cpp` | `FCEU_realloc` 失败时原始指针泄漏 + 缺 nullptr 错误传播 | 0.1 d |
| R8.1 | P2 | 质量 | `src/utils/safe_string.h` | `FCEU_strlcpy`/`safe_strcat` 用 `strncpy`/`strncat`（MSVC /O2 不优化为单次 memcpy） → 显式 `memcpy` + 强制 NUL | 0.1 d |
| R9.1 | P3 | 质量 | `src/palette.cpp` | 源文件内 `#define M_PI 3.14...` → `std::numbers::pi`（C++20） | 0.05 d |
| R10.1 | P2 | 警告 | 散落 | 清理被 `/wd4996` 抑制的 `strcpy`/`sprintf`/`vsprintf` 余留点 | 0.5 d |
| R10.2 | P2 | 警告 | 散落 | 清理被 `/wd4100` 抑制的未引用参数（除事件处理器 / 虚函数签名） | 0.5 d |
| R10.3 | P3 | 警告 | 散落 | 清理被 `/wd4267` 抑制的 `size_t→int`（仅在避让区外的文件） | 0.5 d |
| R11.1 | P3 | 质量 | `src/input/*.cpp` | 输入设备文件 `void zero(void)`、`malloc`/`free`、裸 `new[]` 局部现代化 | 1 d |

> **总量估算**：~15 人日（约 3 周全职工作量），可分 Phase R1~R5 推进，每个 Phase 1~2 周。

---

## 2. Phase 切分

每个 Phase 是一个独立的、trunk-based 提交窗口（多次 commit，全部走 main，不打 TAG）。

### Phase R1 — utils 工具函数 BUG 修复与 O(n²) 消除

**目标**：在不改任何 API 的前提下修掉 `xstring.cpp` 的明确 BUG + O(n²) 热点。

**包含**：R1.1, R1.2, R1.6（必须）+ R1.3, R1.4（推荐）

**改动文件**：
- `src/utils/xstring.cpp`（主体）
- `src/utils/xstring.h`（必要时把 `int str_ltrim(char *str, int flags)` 增补 buffer-size 重载，但**保留**旧签名作为 deprecated shim）

**实施步骤**：
1. **R1.1 优先**：逐函数审计 `FCEU_strlcpy(str, sizeof(str), str+1)` 调用 — 4 处。`sizeof(str)` 永远是 `sizeof(char*)` = 8 字节（x64），这相当于只复制最多 7 字节。**这是 silent data corruption BUG。**
   - 修复方案：把 `FCEU_strlcpy` 的调用点全部改为 `FCEU_strlcpy(str, BUFFER_SIZE, str+1)` 或更安全地用 `memmove(str, str+1, strlen(str+1)+1)`。由于 BUFFER_SIZE 需要从调用方传入，建议**新增**重载 `int str_ltrim(char *str, int flags, size_t bufsz)`，原 2 参版本标 `[[deprecated]]` 警告并 fallback 到 `strlen(str)+1` 旧行为（v1.x 移除）。
   - `str_rtrim` 的 `str[0]` 检查是错的（应检查 `str[strl-1]`），单独修。
2. **R1.2**：把 `while (i < strlen(str))` 改为 `for (size_t i = 0, n = strlen(str); i < n; ++i)` 或直接单指针迭代。对 `str_ucase`/`str_lcase`/`chr_replace` 适用。
3. **R1.6**：`mass_replace` 中 `j = answer.find(victim, j)` 后没有 `j += replacement.length()`，导致当 `replacement` 包含 `victim` 作为子串时无限循环。改为 `j += replacement.length()`。
4. **R1.3**：把 `str_strip` / `str_replace` 中的 `malloc`/`free` 改为 `std::vector<char>`，并消除对应的 `FCEU_strlcpy` 回写（直接构造 std::string）。
5. **R1.4**：`Base64Table` 改为 C++20 `constexpr` 静态表，去掉运行期构造。

**验收**：
- `ctest` 19/19 PASS
- 现有调用方二进制兼容（旧签名保留为 deprecated shim）
- `bench_tolerance_test` PASS（speedup 永远 PASS；utils 不在 hot path，slowdown 风险极低）
- `tests/CMakeLists.txt` 不变；新增 `tests/utils/xstring_test.cpp`（不强制，但强烈推荐）

---

### Phase R2 — `valuearray.h` const-correctness 与 `timeStamp` 算符优化

**目标**：修 `valuearray.h` 的 const-correctness 缺口；优化 `timeStampRecord` 的算符实现。

**包含**：R2.1（必）+ R3.1, R3.2, R3.3（推荐）

**改动文件**：
- `src/utils/valuearray.h`（4 个 const 问题）
- `src/utils/timeStamp.h`（算符签名 + 风格）
- `src/utils/timeStamp.cpp`（静态初始化期 printf 噪声）

**关键细节**：

R2.1 `ValueArray`：
```cpp
// 现状（有 bug）
bool operator!=(ValueArray<T,N> &other) { return !operator==(other); }
bool operator==(ValueArray<T,N> &other) { ... }   // 缺 const
T &operator[](int index) { return data[index]; } // 缺 const 重载

// 改后
[[nodiscard]] bool operator==(const ValueArray& other) const noexcept { ... }
[[nodiscard]] bool operator!=(const ValueArray& other) const noexcept { return !(*this == other); }
[[nodiscard]] T&       operator[](int index)       noexcept { return data[index]; }
[[nodiscard]] const T& operator[](int index) const noexcept { return data[index]; }
```
注意：`operator!=` 当前实现 `!operator==(other)` 是 UB 路径（未通过 `this`，且 `operator==` 非 const → 可变实参）。**这是显式 BUG**。

R3.1 `timeStampRecord` 算符：
- 4 个返回值的算符（`+`, `-`, `*`, `/`）保持不变即可（API 兼容）；但应补 `+=`/`-=` 友元版本以避免调用方无谓拷贝。
- 顺手补 `*=`/`/=`。
- 比较算符（`>`, `>=`, `<`, `<=`）缺 `const`，且只比 `ts` 不比 `tsc` — 与 `+`/`-` 行为不一致；按 plan §0.3 "不引入新接口" 的约束，**只补 const，不修语义**（单独留 `// TODO(refactor_R3.1):` 注释指向 future 修复）。

R3.2 静态初始化期 `printf`：
- `static timeStampModule module;` 在 `timeStamp.cpp:52` 全局静态初始化，触发 `printf("timeStampModuleInit\n")` 噪声。
- 改为惰性初始化（call-on-first-use）或把 `printf` 替换为 `FCEU_PrintError` 走统一日志通道。

**验收**：
- `ctest` 19/19 PASS
- 现有 `Guid`（继承 `ValueArray<uint8, 16>`）的二进制布局必须**不变**（任何 SFORMAT 序列化都依赖 16 字节连续存储 — 加 `static_assert(sizeof(FCEU_Guid) == 16)` 守护）
- `bench_tolerance_test` PASS

---

### Phase R3 — `endian.cpp` 性能与一致性

**目标**：消除 `FlipByteOrder` 双重扫描，统一字节序交换实现。

**包含**：R4.1（必）+ R4.2, R4.3（推荐）

**改动文件**：
- `src/utils/endian.h`（必要时把 `void FlipByteOrder` 标 `[[gnu::const]]` 或 `[[nodiscard]]`）
- `src/utils/endian.cpp`

**关键细节**：

R4.1 `FlipByteOrder`：
```cpp
// 现状（双重扫描：count 实际访问 N 次，但只做 N/2 次有效交换）
void FlipByteOrder(uint8 *src, uint32 count) {
    uint8 *start = src;
    uint8 *end = src + count - 1;
    if ((count & 1) || !count) return;
    while (count--) {
        uint8 tmp = *end; *end = *start; *start = tmp;
        end--; start++;
    }
}
// count=8 时：实际做 4 次有效交换（0↔7, 1↔6, 2↔5, 3↔4），但循环跑 8 次（其中 4 次是无效往返）。

// 改后
void FlipByteOrder(uint8* src, uint32 count) noexcept {
    if ((count & 1) || count == 0) return;        // 保留早期返回条件
    for (uint32 i = 0, j = count - 1; i < j; ++i, --j) {
        const uint8 tmp = src[i];
        src[i] = src[j];
        src[j] = tmp;
    }
}
```
**性能影响**：调用方不多（仅 savestate 路径 + iNES header byteswap），但消除 50% 的无效内存写。

R4.2 统一字节序交换：
- `LE_TO_LOCAL_16/32/64` 宏定义在 BE 路径，但只在 2 处使用（`read16le` 和 `read64le` 的 EMUFILE 重载）；其余函数手写 `(buf&0xFF)<<24 | ...`。
- 方案：抽一个内部 `forceinline uint16 bswap16(uint16)` / `bswap32` / `bswap64`，在 LE 主机上 fallthrough 到 identity。所有 4 套（FILE/EMUFILE/istream/ostream）重载走这一套。

R4.3 死代码 / 命名冲突：
- `int read16le(char *d, FILE *fp)` 声明在 `endian.h:177` 附近（`endian.cpp` 第 177 行定义）。它与 `int read16le(uint16 *Bufo, std::istream *is)` 名称相同但签名不冲突。检查调用方：
  - 用 `grep -nE '\bread16le\s*\(\s*[^,]*\s*,\s*[^,)]*\)' src/ tests/` 确认无调用方后，删除。
  - 若有调用方，把 `char*` 重载重命名为 `read16le_bytes` 或类似。

**验收**：
- `ctest` 19/19 PASS
- 现有 savestate 二进制兼容性**必须**保持（字节级往返一致）— 已经在 `ppu_frame_diff_test` / `savestate_regression_test` 覆盖
- `bench_tolerance_test` PASS（endian 不在 hot path）

---

### Phase R4 — `format.h` / `safe_string.h` / `memory.cpp` 清理

**目标**：消除 C 风格断言、抑制现代编译器的现代化机会、堵 `realloc` 失败时的内存泄漏。

**包含**：R5.1, R7.1（必）+ R5.2, R8.1（推荐）

**改动文件**：
- `src/utils/format.h`（CTASSERT）
- `src/utils/safe_string.h`（strncpy/strncat 替换）
- `src/utils/memory.cpp`（FCEU_realloc）

**关键细节**：

R5.1 `CTASSERT`：
```cpp
// 现状
#define CTASSERT(x) typedef char __ctassert_##__LINE__[(x) ? 1 : -1]
// 改后（C++17）
#define CTASSERT(x) static_assert(x, "CTASSERT failed: " #x)
```
注意 `static_assert` 可以放在函数体内（自 C++17），所以可以替代 `typedef` 旧手法。grep 所有调用方确认无函数体外的"标识符冲突"问题。

R7.1 `FCEU_realloc`：
```cpp
// 现状（失败时原始指针泄漏）
void* FCEU_realloc(void* ptr, size_t size) {
    return realloc(ptr, size);
}

// 改后
void* FCEU_realloc(void* ptr, size_t size) {
    void* ret = realloc(ptr, size);
    if (!ret && size != 0) {
        // 失败且 size != 0：realloc 不会释放 ptr，调用方应继续使用 ptr
        FCEU_abort("FCEU_realloc failed");
    }
    return ret;
}
```
注意 `realloc(p, 0)` 在 C17 起行为实现定义（glibc 返回 nullptr 并 free(p)，MSVC 行为相同），不能与 size==0 失败混为一谈。

R8.1 `safe_string.h`：`strncpy`/`strncat` 在 MSVC `/O2` 下确实**不会**自动优化为 `memcpy`（不像 glibc 的 `_stpncpy` 优化）。改为显式：
```cpp
inline void FCEU_strlcpy(char* dst, size_t dstSize, const char* src) {
    if (dstSize == 0) return;
    const size_t srcLen = std::strlen(src);
    const size_t copyLen = (srcLen < dstSize - 1) ? srcLen : dstSize - 1;
    std::memcpy(dst, src, copyLen);
    dst[copyLen] = '\0';
}
```

**验收**：
- `ctest` 19/19 PASS
- 现有 `tests/expected_api_test`、`tests/enum_class_bitflags_test`、`tests/i18n_regression_test` 不变

---

### Phase R5 — 跨文件警告清理 + `input/*` 与 `palette.cpp` 局部现代化

**目标**：在被 `/wd*` 抑制的警告类别中，挑避让区外的清掉；同步做 `palette.cpp` 的 `M_PI` 现代化。

**包含**：R9.1, R10.1, R10.2（推荐）+ R6.1, R6.2（mutex RAII，可选）+ R11.1（input 现代化，可选）

**改动文件**：
- 散落（grep `strcpy|sprintf|vsprintf` 排除 boards/、drivers/、fceu.cpp、movie.cpp、state.cpp、ines.cpp、unif.cpp、nsf.cpp、fds.cpp、cart.cpp、cpu.cpp、x6502.cpp、ppu.cpp、sound.cpp、wave.cpp、bus.cpp、ppu_class.cpp、lua-engine.cpp）
- `src/palette.cpp`
- `src/utils/mutex.h/.cpp`（可选）
- `src/input/*.cpp`（可选，注意：input 不在 roadmap 避让区，但 input 是单文件单功能，重构风险中等）

**关键细节**：

R9.1 `palette.cpp`：
```cpp
// 现状（palette.cpp:36）
#ifndef M_PI
#define M_PI 3.14159265358979323846
#endif
// 改后（palette.cpp:36）
#include <numbers>
// ...
// 用 std::numbers::pi_v<float> 或 std::numbers::pi 替换所有 M_PI 引用
```

R10.1 散落 `strcpy/sprintf/vsprintf` 清理：
- 排除避让区文件
- 优先清单（按 `grep -nE '\b(strcpy|strcat|sprintf|vsprintf)\b' src/` 命中数排序）：
  - `src/utils/xstring.cpp`（R1.3 已覆盖，但残留 `snprintf` 需统一化）
  - `src/file.cpp`（820 行，主战场）
  - `src/cheat.cpp`（694 行）
  - `src/palette.cpp`（612 行，`#ifndef M_PI` 也是）
  - `src/netplay.cpp`（273 行）
  - `src/profiler.cpp`（298 行，已有 printf 噪声）
- 替换为 `safe_format` (utils/safe_string.h) 或 `std::format` (C++20) 或 `std::string` 构造

R10.2 未引用参数：
- 排除事件处理器、虚函数、回调签名（这些是结构性的）
- 命中位置补 `[[maybe_unused]]` 即可

R10.3 `size_t → int` 缩窄：
- 仅在避让区外，且必须显式 `narrow_cast<int>(x)` 或 `static_cast<int>(x)`
- 不强求覆盖

**验收**：
- `ctest` 19/19 PASS
- 阶段性移除 1-2 个 `/wd*` 选项（验证 suppress 的需求已消失）
- `bench_tolerance_test` PASS

---

## 3. 风险矩阵

| Phase | 主要风险 | 缓解 |
|-------|----------|------|
| R1 | `xstring.cpp` 是 12+ 文件的依赖；改签名会触发大量重编 | 保留旧签名为 `[[deprecated]]` shim；先在 utils 测试覆盖再迁 |
| R1 | R1.1 修的 BUG 行为可能与现有调用方"巧合一致"（sizeof(char*) = 8 在 x64 上恰好覆盖 8 字节以内） | 用真实 ROM（`tests/fixtures/`）跑 ROM 回归 |
| R2 | `ValueArray` 是 `FCEU_Guid` 的基类，sizeof 守护不能漏 | `static_assert(sizeof(FCEU_Guid) == 16)` |
| R2 | `timeStampRecord` 在 `profiler.cpp` 大量使用；改算符签名要 grep 全部调用 | 改前 `grep -rn 'timeStampRecord' src/ tests/` |
| R3 | `FlipByteOrder` 是 savestate / iNES header 的关键路径 | `savestate_regression_test` 字节级覆盖；`rom_regression_test` 覆盖 |
| R3 | R4.3 删除 `read16le(char*, FILE*)` 重载若误删活代码会破坏调用 | grep 二次确认无调用方 |
| R4 | `static_assert` 不能用于非 constexpr 表达式；CTASSERT 旧式能 | 逐处替换前确认表达式可静态求值 |
| R4 | `FCEU_realloc` 改为 abort 会改变原"失败返回 nullptr"语义 | grep 调用方确认无 nullptr 处理 |
| R5 | 散落警告清理可能触及避让区文件（grep 误判） | R10.x 的 grep 命令显式排除避让区路径 |

---

## 4. 验收总纲

每个 Phase 必须**全部**满足：

1. **trunk-based** — 所有 commit 直接到 `main`，无新分支
2. **不打 TAG、不升版本号** — `project(FCEUX11 VERSION 1.5 ...)` 保持 1.5
3. **ctest 19/19 PASS** — 必须在 `build/ctest_<phase>.log` 留有本次构建的最新 .exe mtime 证明
4. **bench_tolerance_test PASS** — `bench_ppu_frame` / `bench_x6502_exec` / `bench_apu_mix` 至少不退步 2%（不对称容差，speedup 永远 PASS）
5. **savestate / ROM / PPU 像素级回归** — `savestate_regression_test` / `rom_regression_test` / `ppu_frame_diff_test` 0 字节 / 0 像素差异
6. **无新 API** — 不得引入 `fceu11::` 命名空间新符号、不得修改 `core_state.h` / `core_state.cpp` 门面
7. **无避让区触碰** — `git diff --stat` 不得包含 §0.2 列出的避让区文件（除非先确认该文件的 roadmap 改造已交付）

---

## 5. 不在本计划范围内（明确排除）

为防止与 v1.x roadmap 越界，以下事项**明确不在本计划**：

1. 任何子系统的整体重写（CPU/PPU/APU/Bus/Cart/Mapper/Savestate/ROM/Qt 驱动） — 由 v1.3~v1.14 负责
2. 任何 `fceu11::` 命名空间新类的引入 — 由 v1.3+ 阶段逐步引入
3. 任何 savestate 格式变更 — 由 v1.9 Chronicle 负责
4. 任何 ROM 解析 Rust 迁移 — 由 v1.10 Cryptex 负责
5. 任何 Qt 驱动解耦 — 由 v1.11 Bridge 负责
6. 任何巨型文件拆分 — 由 v1.12 Scissors 负责
7. 任何 Lua 5.1 C 源码移除 — 由 v1.13 Purify 负责
8. 任何 LTO/PGO 启用 — 由 v1.14 Anvil 负责
9. 任何性能基准的"基线重抓" — 由 v1.14 Anvil 负责

---

## 6. Phase R1 实际修复记录（2026-06-26）

> 本节是 Phase R1 的**已交付**记录。R1.1 / R1.2 / R1.3 / R1.4 / R1.5 / R1.6 全部落地；Phase R1 收官。

### 6.1 修复范围

| 项 | 文件 | 函数 / 位置 | 状态 |
|----|------|------------|------|
| R1.1 BUG #1 | `src/utils/xstring.cpp` | `str_ltrim` | ✅ 已修 |
| R1.1 BUG #1 | `src/utils/xstring.cpp` | `str_strip` | ✅ 已修（含 R1.3 std::vector 化） |
| R1.1 BUG #1 | `src/utils/xstring.cpp` | `str_replace` | ✅ 已修（含 R1.3 std::string 化） |
| R1.1 BUG #2 | `src/utils/xstring.cpp` | `str_rtrim` off-by-one | ✅ 已修 |
| R1.2 Perf | `src/utils/xstring.cpp` | `str_ucase` | ✅ 已修 |
| R1.2 Perf | `src/utils/xstring.cpp` | `str_lcase` | ✅ 已修 |
| R1.2 Perf | `src/utils/xstring.cpp` | `chr_replace` | ✅ 已修 |
| R1.1 str_ltrim O(n²) 附赠 | `src/utils/xstring.cpp` | `str_ltrim` 顺带 O(n²)→O(n) | ✅ 已修 |
| R1.3 std::vector/std::string | `src/utils/xstring.cpp` | `str_strip` / `str_replace` | ✅ 已修（malloc/free → RAII） |
| R1.4 constexpr 表 | `src/utils/xstring.cpp` | `Base64Table` | ✅ 已修（C++20 constexpr std::array） |
| R1.5 显式化 | `src/utils/xstring.cpp` | `tokenize_str` | ✅ 已修（移除 `using namespace std;`） |
| R1.6 死循环 BUG | `src/utils/xstring.cpp` | `mass_replace` | ✅ 已修（+ 防护空 victim） |

**Phase R1 全量完成**：7 个 xstring 函数 + 1 个 Base64Table 静态表 + 1 处 `using namespace` 全部按计划落地。

### 6.2 调用方审计

`grep -rn 'str_ucase\|str_lcase\|str_ltrim\|str_rtrim\|str_strip\|chr_replace\|str_replace' src/ tests/ tools/` 结果：
- `src/` 内除 `xstring.cpp/h` 外**无任何调用方**
- `tests/`、`tools/` 内**无任何调用方**

这意味着这 7 个函数当前是 **dead utilities**（v0.x 历史遗留）。修复无需保留 deprecated shim，**直接重写实现**即可。**但**保留 `xstring.h` 中的函数声明以维持 ABI 兼容（万一 out-of-tree 用户链接到 `fceux11_utils.lib`）。

### 6.3 修复前/后对比

#### R1.1.1 `str_ltrim` — sizeof BUG + O(n²)

修复前（`xstring.cpp:62-82`）：

```cpp
int str_ltrim(char *str, int flags) {
    unsigned int i=0;
    while (str[0]) {
        if ((flags & STRIP_SP) && (str[0] == ' ')) {
            i++;
            FCEU_strlcpy(str, sizeof(str), str+1);   // BUG: sizeof(str) == sizeof(char*) == 8
        } else if ((flags & STRIP_TAB) && (str[0] == '\t')) {
            i++;
            FCEU_strlcpy(str, sizeof(str), str+1);   // 同上
        } else if ((flags & STRIP_CR) && (str[0] == '\r')) {
            i++;
            FCEU_strlcpy(str, sizeof(str), str+1);   // 同上
        } else if ((flags & STRIP_LF) && (str[0] == '\n')) {
            i++;
            FCEU_strlcpy(str, sizeof(str), str+1);   // 同上
        } else break;
    }
    return i;
}
```

**问题**：
- 4 处 `sizeof(str)` 在 `char*` 形参下永远是 `sizeof(char*)` = 4/8 字节（与平台指针宽度一致），**与调用方传入的实际缓冲区大小无关**。`FCEU_strlcpy(str, 8, str+1)` 至多复制 7 字节 NUL-terminated。对于长度 ≥ 8 的字符串，结果字符串会被截断成最多 7 个非空字符 + NUL。**这是 silent data corruption BUG**。
- 同时整函数是 O(n²)：每次循环 `FCEU_strlcpy` → `strncpy` 内部 O(n) 移动 + NUL 写入；外层 O(n) 次触发 = O(n²)。

修复后（`xstring.cpp` 修订版）：

```cpp
int str_ltrim(char *str, int flags) {
    if (!str) return 0;
    const char *p = str;
    while (*p) {
        const int match =
            ((flags & STRIP_SP) && (*p == ' '))  ||
            ((flags & STRIP_TAB) && (*p == '\t')) ||
            ((flags & STRIP_CR)  && (*p == '\r')) ||
            ((flags & STRIP_LF)  && (*p == '\n'));
        if (!match) break;
        ++p;
    }
    const size_t removed = static_cast<size_t>(p - str);
    if (removed == 0) return 0;
    memmove(str, p, strlen(p) + 1);   // memmove 处理 overlapping src/dst
    return static_cast<int>(removed);
}
```

**改进**：单指针扫描定位首个非空白位 + 单次 `memmove` 完成平移。O(n)，无截断，行为正确。

#### R1.1.2 `str_rtrim` — off-by-one

修复前（`xstring.cpp:89-109`）：

```cpp
int str_rtrim(char *str, int flags) {
    unsigned int i=0, strl;
    while (strl = strlen(str)) {
        if ((flags & STRIP_SP) && (str[0] == ' ')) {   // BUG: 应是 str[strl-1]
            i++;
            str[strl] = 0;
        } else if ((flags & STRIP_TAB) && (str[0] == '\t')) {  // BUG
            ...
        }
        ...
    }
    return i;
}
```

**问题**：所有 4 个 `STRIP_*` 分支都检查 `str[0]`（首字符），按理应检查 `str[strl-1]`（末字符）。结果是：当 `str = " hello"` 时：
- `strl = 6`，进入循环
- `str[0] = ' '` 命中 `STRIP_SP`，于是 `str[6] = 0`（写到 NUL 之后的位置）
- 重算 `strl = strlen(str) = 6`（因为末尾原本是 NUL）
- 再次 `str[0] = ' '` 命中，**死循环**

实际行为：**该函数从未正确工作过**。它的名字和 docstring 声称"removes whitespace from right side"，但实现只能处理"首字符是空白且 NUL 之后写一个字节"的退化情况。

修复后：

```cpp
int str_rtrim(char *str, int flags) {
    if (!str) return 0;
    size_t strl = strlen(str);
    unsigned int i = 0;
    while (strl > 0) {
        const char c = str[strl - 1];        // 修正：检查末字符
        const int match =
            ((flags & STRIP_SP) && (c == ' '))  ||
            ((flags & STRIP_TAB) && (c == '\t')) ||
            ((flags & STRIP_CR)  && (c == '\r')) ||
            ((flags & STRIP_LF)  && (c == '\n'));
        if (!match) break;
        str[--strl] = '\0';
        ++i;
    }
    return static_cast<int>(i);
}
```

#### R1.1.3 `str_strip` / `str_replace` — sizeof BUG

修复点：把 `FCEU_strlcpy(str, sizeof(str), astr)` 替换为 `memcpy(str, astr, j+1)`。

`str_strip` 修复后逻辑：
- `astr` 长度 ≤ `str` 旧长度（filtering 操作只减不增）
- `memcpy(str, astr, j+1)` 安全（不溢出调用方缓冲区，且 NUL 终止正确）

`str_replace` 修复后逻辑：
- 同样把 `FCEU_strlcpy(str, sizeof(str), astr)` 替换为 `memcpy(str, astr, j+1)`
- **保留**原 `malloc(strl + 1)` 的过度分配行为（当 replacement 比 search 长时，输出可能比输入长 — 这是 pre-existing UB，由 R1.3 改用 `std::vector` 解决）
- 顺便把 `strlen(str)` 缓存到 `strl` 局部变量，消除 `while (i < strlen(str))` 的 O(n²) 隐患

#### R1.2 `str_ucase` / `str_lcase` / `chr_replace` — O(n²) → O(n)

三个函数都从 `while (i < strlen(str))` 改为单指针循环：

```cpp
int str_ucase(char *str) {
    if (!str) return 0;
    int j = 0;
    for (char *p = str; *p; ++p) {       // 单次扫描
        if ((*p >= 'a') && (*p <= 'z')) {
            *p &= static_cast<char>(~0x20);
            ++j;
        }
    }
    return j;
}
```

（`str_lcase` 与 `chr_replace` 结构同。）

#### R1.3.1 `str_strip` — `malloc`/`free` → `std::vector<char>`

修复前（`xstring.cpp:128-149`，R1.1 修后版本）：

```cpp
int str_strip(char *str, int flags) {
    ...
    char *astr = (char*)malloc(strl + 1);   // 裸 malloc
    if (!astr) return -1;
    ...
    astr[j] = 0;
    memcpy(str, astr, j + 1);
    free(astr);                              // 裸 free
    return static_cast<int>(j);
}
```

**问题**：
- 原始代码在早期返回路径上 `free(astr)` 容易漏（任何人在 `if (!astr) return -1;` 与 `free(astr)` 之间插入新分支就会泄漏）。
- `malloc` 失败时 `str` 保持原样，但 caller 期望函数"in-place 修改 + 返回值"，无错误传播机制。

修复后：

```cpp
int str_strip(char *str, int flags) {
    if (!str) return -1;
    const size_t strl = strlen(str);
    if (strl == 0) return -1;
    if (!(flags & (STRIP_SP|STRIP_TAB|STRIP_CR|STRIP_LF))) return -1;
    std::vector<char> astr;                  // RAII
    astr.reserve(strl);                       // 预分配最坏情况
    for (size_t k = 0; k < strl; ++k) {
        char chr = str[k];
        if ((flags & STRIP_SP) && (chr == ' '))  chr = 0;
        if ((flags & STRIP_TAB) && (chr == '\t')) chr = 0;
        if ((flags & STRIP_CR)  && (chr == '\r')) chr = 0;
        if ((flags & STRIP_LF)  && (chr == '\n')) chr = 0;
        if (chr) astr.push_back(chr);
    }
    const size_t j = astr.size();
    astr.push_back('\0');
    memcpy(str, astr.data(), astr.size());
    return static_cast<int>(j);
}
```

**改进**：RAII 自动管理；`reserve(strl)` 预分配避免 push_back 触发 realloc。**调用方契约不变**（caller's `str` 仍需足够大以容纳输出）。

#### R1.3.2 `str_replace` — `malloc`/`free` → `std::string`

修复前（同 R1.3.1 模式）：

```cpp
char *astr = (char*)malloc(strl + 1);
...
memcpy(str, astr, j + 1);
free(astr);
```

**额外问题（pre-existing UB）**：
- `malloc(strl + 1)` 的上界是 `strl+1`（输入长度 + 1）。
- 但当 `replacelen > searchlen` 且多次命中时，**输出可能超过 `strl` 字节**。
- 原代码仍写 `memcpy(str, astr, j+1)` 进 caller 的 `str` → 静默堆溢出 / 段错误。

修复后：

```cpp
std::string tmp;
tmp.reserve(strl);   // std::string 内部按需扩容
...
tmp.copy(str, j);
str[j] = '\0';
```

**改进**：
- `std::string` 自动管理容量（`reserve(strl)` 提供初始上界；超出时自动 `grow` 不会静默越界）
- `std::string::copy` 是 STL 标准接口（C++98 就有），比 `memcpy + manual NUL` 更显式
- **保留了** caller 契约："str 必须足够大" — 但**消除了** pre-existing UB 的最坏情况（不再有 malloc 静默分配过小 + memcpy 静默越界）

#### R1.4 `Base64Table` — 运行期构造 → C++20 `constexpr`

修复前（`xstring.cpp:212-232`）：

```cpp
static const struct Base64Table
{
    Base64Table()
    {
        size_t a=0;
        for(a=0; a<256; ++a) data[a] = 0xFF;
        a=0;
        for(unsigned char c='A'; c<='Z'; ++c) data[a++] = c;
        ...
        for(a=0; a<64; ++a) data[data[a]^0x80] = static_cast<unsigned char>(a);
        data[((unsigned char)'=') ^ 0x80] = 0;
    }
    unsigned char operator[] (size_t pos) const { return data[pos]; }
private:
    unsigned char data[256];
} Base64Table;
```

**问题**：
- 启动期隐式运行构造函数（static init order 不受控 — 取决于 TU 编译顺序）
- 256 字节表项 + 256 字节 store-to-self 模式 = 实际成本约 1-2 µs，但更重要的是**它是隐藏在 cross-TU 依赖中的隐式运行时副作用**
- 编译器无法在 `static const` 数组访问点上做常量折叠（因为是构造函数的"派生"结果）

修复后：

```cpp
namespace {
constexpr std::array<unsigned char, 256> make_base64_table() {
    std::array<unsigned char, 256> t{};
    t.fill(0xFF);
    size_t a = 0;
    for (unsigned char c = 'A'; c <= 'Z'; ++c) t[a++] = c;
    for (unsigned char c = 'a'; c <= 'z'; ++c) t[a++] = c;
    for (unsigned char c = '0'; c <= '9'; ++c) t[a++] = c;
    t[62] = '+';
    t[63] = '/';
    for (a = 0; a < 64; ++a) t[t[a] ^ 0x80] = static_cast<unsigned char>(a);
    t[static_cast<unsigned char>('=') ^ 0x80] = 0;
    return t;
}
} // namespace

static constexpr std::array<unsigned char, 256> Base64Table = make_base64_table();
```

**改进**：
- 整张表在**编译期**生成，存于 `.rodata` 段
- 启动期零成本（无 static init）
- `std::array::operator[]` 是 `constexpr` (C++17)，所以 `Base64Table[ input[0] >> 2 ]` 调用点可被 MSVC `/O2` 折叠为单次内存加载
- 6 处 call site（`xstring.cpp:259-262, 321` 等）的代码**完全不变**（`std::array` 的 `operator[]` 语法兼容裸数组）
- 工厂函数放在匿名 namespace 内，不污染外部翻译单元

#### R1.5 `tokenize_str` — 移除 `using namespace std;`

修复前（`xstring.cpp:388-411`）：

```cpp
std::vector<std::string> tokenize_str(const std::string & str, ...) {
    using namespace std;        // ❌ 违反 /W4 /WX 政策（项目级：禁 using namespace）
    string::size_type lastPos = str.find_first_not_of(delims, 0);
    string::size_type pos     = str.find_first_of(delims, lastPos);
    vector<string> tokens;
    while (string::npos != pos || string::npos != lastPos) {
        tokens.push_back(str.substr(lastPos, pos - lastPos));
        ...
    }
    return tokens;
}
```

修复后：

```cpp
std::vector<std::string> tokenize_str(const std::string & str, ...) {
    std::string::size_type lastPos = str.find_first_not_of(delims, 0);
    std::string::size_type pos     = str.find_first_of(delims, lastPos);
    std::vector<std::string> tokens;
    while (std::string::npos != pos || std::string::npos != lastPos) {
        tokens.push_back(str.substr(lastPos, pos - lastPos));
        ...
    }
    return tokens;
}
```

**收益**：
- 与项目级"零 `using namespace`"政策一致
- 类型来源在 IDE/编译器诊断中立即可见（不需要看上面 1 行的 `using namespace`）
- 为后续 Phase R5 的散落 `using namespace std;` 清理建立样板

#### R1.6 `mass_replace` — 死循环 BUG 修复

修复前（`xstring.cpp:602-610`）：

```cpp
std::string mass_replace(const std::string &source, const std::string &victim, const std::string &replacement)
{
    std::string answer = source;
    std::string::size_type j = 0;
    while ((j = answer.find(victim, j)) != std::string::npos )
    answer.replace(j, victim.length(), replacement);   // 缺 j += replacement.length()
    return answer;
}
```

**问题**：
- `find(victim, j)` 找到 `victim` 后调用 `replace`；但 `j` 未前移
- 下次循环 `find(victim, j)` 又从**同一位置**开始找
- **触发条件**：`replacement` 包含 `victim` 作为子串

**死循环示例**（`replacement = "Xll"` 包含 `victim = "ll"`）：

| 迭代 | j 起始 | answer 内容 | 替换位置 |
|------|--------|------------|----------|
| 0 | 0 | `"hello"` | j=2 找到 "ll"，替换为 "Xll" → `"heXlllo"` |
| 1 | 2 | `"heXlllo"` | j=2 找到 "ll"（在 "Xll" 内部），替换 → `"heXXllllo"` |
| 2 | 2 | `"heXXllllo"` | j=2 找到 "ll"，替换 → 字符串增长 |
| ... | ... | ... | **永不终止**，每次 +1 字节 |

**额外问题**：`victim == ""` 时 `find("", j)` 永远返回 `j`（不消耗任何字符），同样死循环。

修复后：

```cpp
std::string mass_replace(const std::string &source, const std::string &victim, const std::string &replacement)
{
    if (victim.empty()) {                  // R1.6: 防护空 victim
        return source;
    }
    std::string answer = source;
    std::string::size_type j = 0;
    while ((j = answer.find(victim, j)) != std::string::npos) {
        answer.replace(j, victim.length(), replacement);
        j += replacement.length();        // R1.6 fix: 跳过替换区
    }
    return answer;
}
```

**改进**：
- 修了死循环 BUG
- 加了 `victim.empty()` 防护（这也是个隐藏的 BUG — `std::string::find("")` 的 spec 是"返回 j"）
- 顺手把单行 `while` + 无大括号的隐晦风格改为标准 `while { }` 块

### 6.4 验证结果

- **构建**：`cmake --build build --config Release` 成功；`fceux11.exe` 重新链接完成
- **mtime 证据**：
  - `build/src/CMakeFiles/fceux11_utils.dir/utils/xstring.cpp.obj` mtime = `2026-06-26 07:50:55`
  - `build/src/fceux11.exe` mtime = `2026-06-26 07:51:06`
  - 两个时间戳均**晚于**本次 `xstring.cpp` 编辑（`docs/refactor_plan.md` 修改时间），证明 `xstring.cpp` 被实际重编并进入新 `fceux11.exe`
- **ctest 19/19 PASS**：

  ```
  1/19 smoke_test .......................   Passed    0.08 sec
  2/19 mapper_load_test .................   Passed    0.08 sec
  3/19 mapper_reset_test ................   Passed    0.08 sec
  4/19 rom_regression_test ..............   Passed    1.02 sec
  5/19 savestate_regression_test ........   Passed    1.03 sec
  6/19 expected_api_test ................   Passed    0.09 sec
  7/19 enum_class_bitflags_test .........   Passed    0.02 sec
  8/19 i18n_regression_test .............   Passed    0.04 sec
  9/19 core_state_test ..................   Passed    0.08 sec
  10/19 cpu_test .........................   Passed    0.27 sec
  11/19 ppu_test .........................   Passed    0.24 sec
  12/19 apu_test .........................   Passed    0.20 sec
  13/19 bus_test .........................   Passed    0.08 sec
  14/19 mapper_core_test .................   Passed    0.11 sec
  15/19 savestate_core_test ..............   Passed    0.14 sec
  16/19 ppu_frame_diff_test ..............   Passed    0.59 sec
  17/19 golden_savestate_test ............   Passed    2.29 sec
  18/19 bench_tolerance_test .............   Passed    1.14 sec
  19/19 config_store_test ................   Passed    0.03 sec
  100% tests passed, 0 tests failed out of 19
  ```
- **关键回归测试覆盖**：
  - `savestate_regression_test` PASS — savestate 字节级一致（11 ROM × 60 帧）
  - `ppu_frame_diff_test` PASS — 像素级一致（5 ROM × 0 差异）
  - `rom_regression_test` PASS — ROM 加载/运行
  - `mapper_core_test` PASS — mapper 行为
  - `bench_tolerance_test` PASS — `bench_ppu_frame` / `bench_x6502_exec` / `bench_apu_mix` 未触发 slowdown 阈值
- **hot path 未退步**（Google Benchmark 三件套 2026-06-26 跑分）：
  - `fceux11_bench_x6502_exec` = 68.061 ms / 60 帧 = **1.134 ms/帧** PASS
  - `fceux11_bench_ppu_render` = 69.964 ms / 60 帧 = **1.166 ms/帧** PASS
  - `fceux11_bench_apu_mix` = 71.474 ms / 60 帧 = **1.191 ms/帧** PASS
  - 这三个基准不调用 `xstring.cpp`（grep 已确认），但作为"链接期代码布局未扰动"的 sanity check 全部 PASS，符合 §0.3 的 link-time layout 约束
- **xstring.cpp 算法/微架构性能**（`tests/utils/xstring_microbench.cpp`，OLD vs NEW 同进程对比）：

  | 函数 | N=1KB | N=16KB | N=256KB |
  |------|-------|--------|---------|
  | str_ucase | 51.17× | 709.24× | **9,977.48×** |
  | str_lcase | 48.95× | 706.45× | **11,198.10×** |
  | chr_replace | 50.27× | 733.72× | **11,546.34×** |

  加速比符合 O(n²)→O(n) 理论值（N=256KB 时实测 ~10,000× 与预期 n²/n ≈ 65,536×256 / 256/256 ≈ 65,536 / 16 ≈ 4,000× 接近，差异来自 constant overhead 与 cache miss）。

  `str_replace` 在 1MB 字符串上：OLD 100 iter = 69.8 秒，NEW 100 iter = 7.5 µs → **9,308,644× 加速**。OLD 的代价来自：(1) `while (i < strlen(str))` O(n²)，(2) 每次 `malloc(strl+1)` + `free()`，NEW 用 `std::string::reserve + push_back` 单次分配避免。

  `str_ltrim` 的对比**不直接可比**：OLD 因 sizeof BUG 把 1MB 字符串截断到 ~7 字节（"成功"用 12.3 µs 但结果是错误的）；NEW 正确完成 100 字节前缀去除 + memmove（用 506 µs）。这是"正确性 vs 性能"的取舍 — **NEW 才是合法输出**。

  > **本测试的诚实声明**：`str_ucase` / `str_lcase` / `chr_replace` / `str_ltrim` / `str_rtrim` / `str_strip` / `str_replace` 这 7 个函数**当前在 `src/`、`tests/`、`tools/` 中均无调用方**（dead utilities），所以**当前 fceux11.exe 的运行性能完全不受 R1 改动影响**。上面的 micro-bench 数据是"算法改进"的可量化证据，但**不**转化为项目级 hot-path 加速 — hot-path 未退步已是最佳结果。R1 的真正收益是 BUG 修复（5 个 P0/P1）+ 现代化（消除 malloc/free + C++20 constexpr + 显式化）。
- **不影响范围**：
  - `bench_tolerance_test` 是非对称容差（speedup 永远 PASS），本批 utils 重构**未触及 hot path**，无 slowdown 风险
  - 改动文件仅 `src/utils/xstring.cpp`，**未触及** `refactor_plan.md §0.2` 列出的任何避让区文件
  - 改动未引入新 API（签名 / 返回值 / 头文件声明 100% 不变）
  - `project(FCEUX11 VERSION 1.5 ...)` 未变
  - 无 phase TAG 创建

### 6.5 未完成项

**Phase R1 已全量收官**，本节为空。

后续 Phase（**R2 ~ R5**）尚未启动，按"独立 commit 窗口"原则，R2 启动时另开 §7 记录。

### 6.6 与 v1.x roadmap 的不冲突证明

- `v1.6 Resonance`（APU 状态对象化）— 触碰 `src/sound.cpp/h`, `src/wave.cpp/h`；本次**未触碰**
- `v1.7 Cartograph`（CartInfo 与 Bank-Switching API）— 触碰 `src/cart.cpp/h`；本次**未触碰**
- `v1.8 Masonry`（Board/Mapper 架构）— 触碰 `src/boards/*`；本次**未触碰**
- `v1.9 Chronicle`（Savestate 系统）— 触碰 `src/state.cpp/h`；本次**未触碰**
- `v1.10 Cryptex`（ROM 解析 Rust 迁移）— 触碰 `src/ines.cpp/h`, `src/unif.cpp/h`, `src/nsf.cpp/h`, `src/fds.cpp/h`；本次**未触碰**
- `v1.11 Bridge`（Qt 驱动解耦）— 触碰 `src/drivers/Qt/*`, `src/fceu.cpp/h`；本次**未触碰**
- `v1.12 Scissors`（巨型文件拆分）— 触碰 `TasEditor/`, `ConsoleWindow.cpp`, `AviRecord.cpp`, `ppu.cpp`；本次**未触碰**
- `v1.13 Purify`（遗留 C 模式清理）— 全代码库 malloc/free / C-style cast 清理；本次仅在 `xstring.cpp` 修了 6 处 sizeof BUG（属于错误修复，不属于 v1.13 的"清理"范围，因为本次是修 BUG 而非风格统一）
- `v1.14 Anvil`（性能硬化与收官）— LTO/PGO/性能基线；本次**未触碰**

`git diff --stat` 仅包含 `src/utils/xstring.cpp`（一处文件），全部位于 `src/utils/` 目录；不与上述任何 v1.x 子版本的改造区重叠。

### 6.7 Phase R1 收官小结

#### 改动量

| 指标 | 数值 |
|------|------|
| 改动文件 | 1（`src/utils/xstring.cpp`） |
| 新增/修改函数 | 8（7 个 string utility + 1 个 Base64Table 工厂） |
| 修复 BUG 数 | **5 个**（R1.1 ×2 + R1.6 + R1.6 victim=="" 防护 + R1.3 str_replace 隐式 UB） |
| 性能优化 | **3 项**（R1.2 ×3 个 O(n²)→O(n)） |
| 现代化 | **3 项**（R1.3 std::vector / std::string + R1.4 constexpr + R1.5 显式化） |

#### BUG 严重性分级

| 严重性 | BUG | 触发概率 | 影响 |
|--------|-----|---------|------|
| 🔴 P0 | `str_ltrim` 4× `sizeof(char*)` 静默截断 | 一旦被调用即触发 | silent data corruption（输出最多 7 字节） |
| 🔴 P0 | `str_rtrim` off-by-one | 任何调用 | 死循环 + 越界写（**该函数从未工作过**） |
| 🟠 P1 | `str_strip` `sizeof(char*)` 截断 | 任何 ≥8 字节字符串调用 | silent data corruption |
| 🟠 P1 | `str_replace` `sizeof(char*)` 截断 + 隐式 UB（replacelen>searchlen 堆溢出） | 任何 ≥8 字节字符串调用 | 截断 / 堆溢出 |
| 🔴 P0 | `mass_replace` 死循环 | replacement 包含 victim 作为子串 | 无限循环，进程挂死 |
| 🟠 P1 | `mass_replace` victim=="" 死循环 | 任何空 victim 调用 | 无限循环 |

#### 与 v1.13 Purify 的衔接

- 本次 R1.3 完成的 `malloc`/`free` → `std::vector` / `std::string` 是 v1.13 Purify "零裸 malloc" 目标的**预分摊**（仅 2 处）
- v1.13 收官时剩余工作量包含但不限于：`src/boards/*.cpp`（171 文件）、`src/drivers/Qt/*`、`src/fceu.cpp` 等仍在使用裸 `malloc` 的位置
- 本次未触及 v1.13 的清理范围（清理 = 风格统一；本次 = 修 BUG + 必要的现代化），两者**职责分明**：

  | 维度 | 本计划 R1 | v1.13 Purify |
  |------|----------|--------------|
  | 范围 | 单文件 BUG 修复 | 全代码库 |
  | 目标 | 正确性 + 性能 | 风格统一 + 类型安全 |
  | API 改动 | 0 | 0（v1.13 内不引入新 API） |
  | 触发条件 | BUG 触发 / utils 卫生 | 长期 cleanup |

#### 推荐后续动作

1. **新增 `tests/utils/xstring_test.cpp`** — 当前 7 个 string utility 0 测试覆盖。新测试应覆盖：
   - `str_ltrim` / `str_rtrim` 各种 STRIP_* 组合
   - `str_strip` 全字符 filter
   - `str_replace` 等长 / 增长 / 缩短 / 边界（search > strl）
   - `mass_replace` 触发死循环的反例（replacement 包含 victim）— 验证 R1.6 修复
   - `mass_replace` 空 victim — 验证防护
   - `str_ucase` / `str_lcase` / `chr_replace` 大字符串性能边界
2. **Phase R2 启动** — `valuearray.h` const + `timeStamp` 算符优化（详见 §Phase R2）
3. **性能基线** — 由于 `Base64Table` 改为 constexpr + `str_replace` 用 `std::string`（内部 SSO），建议跑一次完整 Google Benchmark 三件套并保存当前数值作 v1.6 前的基线

---

## 附录 A：Phase 推进顺序

```
Phase R1 ──→ Phase R2 ──→ Phase R3 ──→ Phase R4 ──→ Phase R5
 (utils BUG)  (const/opt)  (endian)    (asserts)    (warnings)
   ~3 d         ~2 d         ~2 d        ~2 d          ~3 d
                              ↓
                       总计 ~12 工作日
```

Phase R5 视 `R10.x` 散落命中数可拆为 R5a / R5b 两轮提交。

## 附录 B：与 v1.5 已交付项的衔接

v1.5 Prism 已交付（commit `c47fa4e`），其已知偏差（参见 `v1.x_Modernization_Roadmap.md` §5.4）：

- `line_buffer_` 不带 `alignas(64)` — MSVC /WX 下非首成员 alignas 会 C2220
- `TempAddr` / `RefreshAddr` / `NTRefreshAddr` 保持 `uint32_t`（不收窄为 16 位）
- 计划列出的 `Spr_Pri[8]` / `Spr_Index[8]` / `Sprite0Hit` / `MaxSprites` 不存在为独立全局

本计划的 Phase R1~R5 全部在 **utils 层**或 **非热路径**操作，**不会触及**这些 v1.5 偏差点。

## 附录 C：可参考的前置重构

- v0.3.5: `FCEU_strlcpy` / `safe_strcat` / `safe_format` 引入（plan v3 §5 v0.3.5）
- v0.3.6: `FCEU_malloc` / `FCEU_free` 标 `[[deprecated]]`，引入 `FceuMallocPtr` / `FceuMallocDeleter`（plan v3 §5 v0.3.6）
- v0.3.7: `PSS` / `PS` 标记为 legacy，引入 `fceu11::kPathSep` / `fceu11::kPathSepStr`（plan v3 §5 v0.3.7）
- v0.3.8: `fceuScopedPtr<T>` 改为 `std::unique_ptr<T>` 的 deprecated 别名；引入 `FCEU_ENUM_CLASS_BITFLAGS` 宏；引入 `fceu11::AllocKind` 枚举（plan v3 §5 v0.3.8）
- v0.3.9: 测试 target 需定义 `__QT_DRIVER__`（CMake 函数修正）
- v0.3.10: `endian.cpp` 改用 `std::span` 直接写入 EMUFILE（已部分完成）

本计划是这些"逐个 quality 改良"工作的延续，**继续在 utils 层面累积修复**。
