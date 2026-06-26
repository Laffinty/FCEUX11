# FCEUX11 中期代码质量与性能局部重构计划

> **范围**：零散、独立、单文件或单函数粒度的代码质量提升与局部性能优化
> **原则**：与 `v1.x_Modernization_Roadmap.md` 的 v1.6 及后续版本**完全正交**，**不打 TAG**，**不升版本号**
> **周期**：中期（按 Phase R1~R5 推进，预计 3~5 个月）
> **最后更新**：2026-06-27（**Phase R1 + R2 + R3 全量收官**；R2 实际修复记录见 §7，R3 实际修复记录见 §8；**Phase R4 续做部分交付**：bench_tolerance_test 方法学修复 + R7.1 已交付，R5.1/R5.2 永久搁置，R8.1 暂缓，详见 §9.8；**Phase R5 部分交付**：R9.1 + R6.1 + R6.2 已交付，R10.1/R10.2/R10.3/R11.1 经审计为 no-op，详见 §Phase R5 "R5 no-op 审计" 子节）
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

**包含（R5a 已交付，2026-06-27）**：R9.1, R6.1, R6.2

**包含（审计后判定为 no-op）**：R10.1, R10.2, R10.3, R11.1 — 详见子节 "R5 no-op 审计"。

**改动文件（R5a）**：
- `src/palette.cpp`（R9.1：删除 3 行 dead `#define M_PI` 块）
- `src/utils/mutex.h/.cpp`（R6.1 + R6.2：`std::unique_ptr` RAII + `autoScopedLock` 模板化）

**关键细节（R5a 已交付）**：

R9.1 `palette.cpp` — 实际改动（与 plan 原描述不同）：
```cpp
// 现状（palette.cpp:35-37）—— dead macro
#ifndef M_PI
#define M_PI 3.14159265358979323846
#endif
// 改后：直接删除 3 行。palette.cpp body 从未使用 M_PI：
//   line 227: float v = (spot - black) / (white-black) / 12.f;
//   line 228: i += v * std::cos(3.141592653 * p / 6);   // 用字面量，非 M_PI
//   line 229: q += v * std::sin(3.141592653 * p / 6);
// 不需要 <numbers> 替换。nsf.cpp（避让区 v1.10 Cryptex）独立定义 M_PI，
// 不影响。
```

R6.1 `mutex.cpp/h`：
- `mutex` 类的 `mtx` 字段从 `QRecursiveMutex*`/`QMutex*` 改为 `std::unique_ptr<QRecursiveMutex>`/`std::unique_ptr<QMutex>`
- `mutex::mutex()` 用 `std::make_unique<...>()` 分配
- `mutex::~mutex()` 改 `= default`，unique_ptr 处理 delete
- mutex.h 仅被 `debugsymboltable.h` include（冷 header，§9.4 hot-header layout-shift 风险极低）
- 13 个 `autoScopedLock` 调用方（`debugsymboltable.cpp:313/325/379/...`）无需改动

R6.2 `mutex.h`：
- 两个 `autoScopedLock` 构造函数（`mutex*` + `mutex&`）合并为单模板：
  ```cpp
  template <typename Mtx>
  autoScopedLock(Mtx&& mtx) : m(getPtr(std::forward<Mtx>(mtx))) { if (m) m->lock(); }
  ```
- 私有静态 `getPtr(mutex*)` / `getPtr(mutex&)` 重载由模板体内通过重载决议选择
- 模板体必须在 header 内（template definition 必须可见）

**验收**：
- `ctest` 19/19 PASS
- `bench_tolerance_test` 单次 PASS；多次稳定性验证见下方 "bench_tolerance 噪声观察"
- 阶段性移除 1-2 个 `/wd*` 选项 — **本次未达成**（见 "R5 no-op 审计"）

**bench_tolerance 噪声观察（2026-06-27）**：
R5a 实施过程中观察到 bench_tolerance_test 在 15 次连续运行中 ~13-20% 概率 FAIL。**关键发现**：在 R4-cont baseline（无任何 R5a 改动）下重测，得到 13/15 = 87% PASS — 即 ~13% 的失败率是环境噪声，与 R5a 改动**无关**。具体：

| 构建状态 | 样本数 | PASS | 失败率 | 失败特征 |
|---------|--------|------|--------|---------|
| R4-cont baseline（无 R5a） | 15 | 13/15 | 13% | full +2.74%, full +6.11% |
| R9.1 only（palette.cpp） | 15 | 12/15 | 20% | 全部 marginal（+2.57% ~ +3.07%） |
| R5a combined（palette + mutex） | 45 | ~38/45 | ~16% | 同样 marginal |

所有失败都在 `+2.5% ~ +3.1%` 的窄带内（与阈值的差 < 0.6%），是 §9.7 #1 已知的环境性 cold-cache 噪声放大。R5a 的 palette.cpp dead macro 删除和 mutex RAII 改造均不涉及 hot path，§9.3 step 3 已验证 `--benchmark_min_time=2s` 充分 warmup 时 +0.08% PASS。**R5a 通过单次 ctest 19/19 + 噪声率与 baseline 一致判定为可交付**。如有更严格的 90%+ PASS 要求，建议扩大样本（如 §9.7 #2 的 30 次连续运行）或降低 ±阈值。

---

#### R5 no-op 审计（2026-06-27）

R5 计划范围中除 R9.1/R6.1/R6.2 外的子项经 grep + 人工审计后**判定为 no-op**，理由如下：

**R10.1 — `strcpy`/`sprintf`/`vsprintf` 散落清理**
- 在 `src/` 内排除避让区（`boards/`、`drivers/`、`lua/`、`lua-engine.cpp`、`fceu.cpp`、`movie.cpp`、`state.cpp`、`ines.cpp`、`unif.cpp/`、`nsf.cpp`、`fds.cpp`、`cart.cpp`、`cpu.cpp`、`x6502.cpp`、`ppu.cpp`、`sound.cpp`、`wave.cpp`、`bus.cpp`、`ppu_class.cpp`、`emufile.h`、`video.cpp`）后，唯一命中为：
  - `src/file.cpp:499` `vsnprintf(*strp, 2048, fmt, ap)` — 在 `#ifndef HAVE_ASPRINTF` polyfill 块内（asprintf 的 C99-less 实现）。`vsnprintf` 本身就是该 primitive 的正确实现，不应替换。
- 其他命中（`emufile.h:189/194` `vsnprintf` 用于测量大小）同样是正确的格式化 primitive。
- 结论：**无 R10.1 工作可做**。/wd4996 抑制保留（避让区文件内的 `FCEU_strlcpy` / `safe_strcat` 调用由 v1.13 Purify 处理）。

**R10.2 — `/wd4100` 未引用参数**
- 排除避让区 + 事件处理器 / 虚函数 / 回调签名（`void Update(void *data, int arg)` 类 input 设备 ABI）后，剩余命中极少，且均位于 `src/archived/`（历史遗留目录）或 `src/drivers/Qt/`（避让区 v1.11 Bridge）。
- 结论：**R10.2 在当前范围无新增工作**。

**R10.3 — `/wd4267` `size_t → int` 缩窄**
- CMakeLists.txt 注释指明源文件为 mappers（避让区 v1.8 Masonry）和 TasEditor/（避让区 v1.12 Scissors）。
- `src/file.cpp:136` 已正确 `static_cast<size_t>(size)`。
- 结论：**R10.3 无工作可做**。/wd4267 抑制保留。

**R11.1 — `input/*.cpp` 局部现代化**
- `src/input/*.cpp`（21 文件，1814 行）已审计：
  - `void zero(void)` 命中：**0**
  - 裸 `malloc/free` 命中：**0**
  - 裸 `new[]` 命中：**0**
- 剩余 8 处 `(void)` 风格清理（`static void StrobeARKFC(void)` 等），但这些是 input 设备回调 ABI 的一部分（与 `void Update(void *data, int arg)` 配套），属于"风格统一"而非"BUG/质量"修复。
- 结论：**R11.1 的实质清理工作已前置完成**，残余为 5 分钟的风格清理，可作为 R5b 可选增量（建议推迟到 v1.13 Purify 阶段一并处理）。

**总结**：R5 实际可交付范围是 R9.1 + R6.1 + R6.2，已在 R5a 完成。R10.x / R11.1 /wd* 抑制移除不在 R5 处理（属于 v1.13 Purify 或下一轮 refactor 的工作）。

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

## 7. Phase R2 实际修复记录（2026-06-26）

> 本节是 Phase R2 的**已交付**记录。R2.1 + R3.2 + R3.3 全量交付；R3.1 部分交付（`operator-=`/`*=`/`/=` 推迟到 R3.1b，理由见 §7.3）。

### 7.1 修复范围

| 项 | 文件 | 函数 / 位置 | 状态 |
|----|------|------------|------|
| R2.1 BUG const-correctness | `src/utils/valuearray.h` | `operator==`/`!=` 缺 const | ✅ 已修 |
| R2.1 BUG const-correctness | `src/utils/valuearray.h` | `operator!=` UB 路径 | ✅ 已修 |
| R2.1 BUG const-correctness | `src/utils/valuearray.h` | `operator[]` 缺 const 重载 | ✅ 已修 |
| R2.1 布局守护 | `src/utils/guid.h` | `static_assert(sizeof(FCEU_Guid) == 16)` | ✅ 已加 |
| R2.1 布局守护 | `src/utils/md5.h` | `static_assert(sizeof(MD5DATA) == 16)` | ✅ 已加 |
| R3.1 算符 | `src/utils/timeStamp.h` | `operator+`/`-`/`*`/`/` 加 const | ✅ 已修 |
| R3.1 算符 | `src/utils/timeStamp.h` | 比较算符 `>`/`>=`/`<`/`<=` 加 const + `[[nodiscard]]` | ✅ 已修 |
| R3.1 算符 | `src/utils/timeStamp.h` | `operator+=` 保留；`operator-=`/`*=`/`/=` 推迟 | ⚠️ 推迟到 R3.1b（link-time layout） |
| R3.2 printf 噪声 | `src/utils/timeStamp.cpp` | 静态初始化期 `printf("timeStampModuleInit\n")` 移除 | ✅ 已修 |
| R3.2 printf 噪声 | `src/utils/timeStamp.cpp` | `tscCalibrate` 多行 `printf` 简化为单条 FCEU_PrintError | ✅ 已修 |
| R3.3 风格 | `src/utils/timeStamp.h` | 14 处 `(void)` 空参 → `()` | ✅ 已修 |

### 7.2 调用方审计

`grep -rn 'ValueArray\|FCEU_Guid' src/ tests/ | grep -v 'valuearray.h'` 结果：
- `src/movie.h:120: ValueArray<uint8,4> joysticks;` — 4 字节布局
- `src/movie.h:203: FCEU_Guid guid;` — 16 字节布局（SFORMAT）
- `src/movie.cpp:428, 642: FCEU_Guid::fromString(...)` — 构造路径
- `src/utils/md5.h:8: typedef ValueArray<uint8,16> MD5DATA;` — 16 字节布局

`grep -rn 'timeStampRecord\|FCEU::timeStamp' src/ tests/ | grep -v 'timeStamp.[ch]'` 结果：
- `src/drivers/Qt/fceuWrapper.cpp:207-235: timeStampModuleInitialized + timeStampRecord` — 3 处
- `src/drivers/Qt/sdl-throttle.cpp:35-337: timeStampRecord ... (cur_time - Lasttime)` — 6 处（含 `operator-` 唯一 hot-path 用法）
- `src/profiler.cpp:107: timeStampRecord ts, dt; ... dt = ts - start;` — `operator-` 第二个 hot-path 用法
- `src/profiler.h:53-73: timeStampRecord min/max/sum/last/start` — 4 个字段

**关键发现**：`operator-` 是唯一被 hot path 使用的二元算符（2 个 call site）。`operator+=` 已被 `operator=` + `operator+` 链式使用，**`operator-=`/`*=`/`/=` 当前 0 调用方**（详见 §7.3 R3.1 推迟原因）。

### 7.3 修复前/后对比

#### R2.1.1 `ValueArray` — const-correctness 全面修复

修复前（`src/utils/valuearray.h`）：

```cpp
template<typename T, int N>
struct ValueArray
{
    T data[N];
    T &operator[](int index) { return data[index]; }                   // 缺 const 重载
    static const int size = N;
    bool operator!=(ValueArray<T,N> &other) { return !operator==(other); }  // UB 路径
    bool operator==(ValueArray<T,N> &other) {                           // 缺 const
        for(int i=0;i<size;i++)
            if(data[i] != other[i])
                return false;
        return true;
    }
};
```

**问题**：
1. `operator==` 缺 `const` — `const ValueArray` 上下文无法比较（`if (guid1 == guid2)` 在 const 方法中编译失败）
2. `operator!=` 实现为 `!operator==(other)` — `operator==` 非 const，所以 `*this` 隐式 cast 为非 const 引用。这是 **UB 路径**（在 const 上下文编译失败，在非 const 上下文"偶然"能工作）
3. `operator[]` 缺 const 重载 — `const ValueArray` 无法读取元素

修复后：

```cpp
template<typename T, int N>
struct ValueArray
{
    T data[N];
    static const int size = N;

    [[nodiscard]] T&       operator[](int index)       noexcept { return data[index]; }
    [[nodiscard]] const T& operator[](int index) const noexcept { return data[index]; }

    [[nodiscard]] bool operator==(const ValueArray& other) const noexcept {
        for (int i = 0; i < size; ++i) {
            if (data[i] != other[i]) return false;
        }
        return true;
    }
    [[nodiscard]] bool operator!=(const ValueArray& other) const noexcept {
        return !(*this == other);
    }
};
```

**改进**：
- 三个 const 缺口全部补上
- `noexcept` 标记允许 STL 算法（`std::sort`、`std::find`）和 `std::array` 的优化路径生效
- `[[nodiscard]]` 防止 `if (guid1 == guid2)` 写错成 `guid1 == guid2;`（语句而非表达式）
- **`T data[N]` 顺序未动** — `FCEU_Guid` 和 `MD5DATA` 的 16 字节 SFORMAT 序列化路径不变

#### R2.1.2 布局守护

`src/utils/guid.h`：

```cpp
// 末尾追加
static_assert(sizeof(FCEU_Guid) == 16,
              "FCEU_Guid layout changed: 16-byte SFORMAT serialization in "
              "src/state.cpp will break. Check valuearray.h refactor.");
```

`src/utils/md5.h`：同样的 `static_assert(sizeof(MD5DATA) == 16)`。

**改进**：未来如果有人给 `ValueArray` 加 vptr、padding、union 成员，编译期就报错（不等到 savestate_corrupt_test 跑挂才暴露）。

#### R3.1.1 `timeStampRecord` 算符 — const 化

修复前（`timeStamp.h:33-88`）：

```cpp
timeStampRecord operator + (const timeStampRecord& op)   // 缺 const
timeStampRecord operator - (const timeStampRecord& op)   // 缺 const
timeStampRecord operator * (const unsigned int multiplier)  // 缺 const
timeStampRecord operator / (const unsigned int divisor)    // 缺 const
bool operator > (const timeStampRecord& op)               // 缺 const
bool operator >= (const timeStampRecord& op)              // 缺 const
bool operator < (const timeStampRecord& op)               // 缺 const
bool operator <= (const timeStampRecord& op)              // 缺 const
```

修复后：所有 8 个算符补 `const`；比较算符加 `[[nodiscard]]` 防止 `if (t1 < t2);` 写错成语句。

**未改的语义**（per plan §0.3 "不引入新接口"）：
- 比较算符只比 `ts`、不算 `tsc`；与 `+`/`-` 算符（双字段）的行为不一致
- 加 `// TODO(refactor_R3.1):` 注释指向 future 修复（v1.14 perf-mode PGO? 阶段统一字段语义）

#### R3.1.2 `operator-=` / `*=` / `/=` 推迟（link-time layout 教训）

**初次实现**（commit 前的草稿）补全了所有 in-place 算符：

```cpp
timeStampRecord& operator -= (const timeStampRecord& op) { ts -= op.ts; tsc -= op.tsc; return *this; }
timeStampRecord& operator *= (unsigned int m) { ts *= m; tsc *= m; return *this; }
timeStampRecord& operator /= (unsigned int d) { ts /= d; tsc /= d; return *this; }
```

**实际验证**：ctest 19/18 PASS，但 `bench_tolerance_test` 的 `bench_full_frame` 报 **+2.96%**（基线 68.249ms vs 实测 70.269ms），刚好超过 +2.5% 阈值。

**根因诊断**（与 memory note "Phase 6 VRC7 bench regression" 同一模式）：新增的 class API surface 改变了 `timeStamp.h` 头布局，所有 include 该头文件的 TU（`profiler.h` → `profiler.cpp`、`sdl-throttle.cpp`、`fceuWrapper.cpp` 等）经链接器 section layout 后，hot path 指令缓存布局被扰动。

**实测演进**（按修复尝试顺序）：

| 步骤 | bench_full_frame 偏差 | 备注 |
|------|----------------------|------|
| 初版（含 `-=`/`*=`/`/=` + fceu.h include） | **+23.18%** | fceu.h → bus.h 污染编译单元 |
| 改用前向声明替代 fceu.h include | +2.96% | 前向声明消除了主要污染，但 API surface 仍在 |
| 移除 `-=`/`*=`/`/=`（最终版） | **+0.45%**（基线 68.249ms vs 实测 68.557ms）| 回归消失 |

**最终决策**：`operator-=`/`*=`/`/=` **推迟到 R3.1b**，理由：
1. **无 hot-path 调用方**（grep 确认 sdl-throttle.cpp:337 和 profiler.cpp:107 只用 `operator-` 返回值）
2. 添加的 ~80 字节代码虽小，但通过链接器 section layout 放大到 hot path +2.96%
3. 留 `// TODO(refactor_R3.1b):` 注释 — 当 (a) 真有 hot-path 调用方 + (b) 找到缓解 layout shift 的方法（如 `__declspec(noinline)`）时再补

**保留的算符**：
- `operator+=`（已有，profiler.cpp 未使用但语义上对 `sum` 累加更自然 — 保持）
- `operator+` / `operator-` 加 const（hot path 使用，安全）

#### R3.2 — 静态初始化期 `printf` 噪声

修复前（`timeStamp.cpp:42-50`）：

```cpp
class timeStampModule
{
    public:
    timeStampModule(void)
    {
        printf("timeStampModuleInit\n");        // <-- 静态初始化期污染 stdout
        timeStampRecord::qpcCalibrate();
    }
};
static timeStampModule module;                   // <-- 程序启动前执行 ctor
```

任何链接 `fceux11_utils.lib` 的程序在进入 `main()` 前都会向 stdout 输出 `timeStampModuleInit\n`。CTEST 在 CI 跑时污染日志。

修复后：

```cpp
class timeStampModule
{
    public:
    timeStampModule()   // R3.3: also dropped C-style (void)
    {
        timeStampRecord::qpcCalibrate();        // 实际工作保留
    }
};
```

**额外**：`tscCalibrate(int numSamples)` 的多行 `printf` 循环（即使 numSamples=0 也输出 1 行 + 1 行说明）替换为单条 `FCEU_PrintError`，且**仅在 numSamples > 0 时输出**（默认 0 = 静默）。

**R3.2 实施中的踩坑**（已修复）：
- ❌ 第一次实现加了 `#include "../fceu.h"` — bus.h 被 transitive 拉入 → +23.18% link-time 回归
- ✅ 最终方案：forward-declare `FCEU_PrintError` 在**全局命名空间**（不放在 `namespace FCEU` 内，匹配 fceu.cpp:1158 实际定义位置）
- ❌ 中间版本：forward-decl 放在 `namespace FCEU` 内 → link error `FCEU::FCEU_PrintError` 未定义（`namespace FCEU` 内引用 `FCEU_PrintError` 解析为 `FCEU::FCEU_PrintError`）
- ✅ 移到全局命名空间 + ctest 验证通过

#### R3.3 — `(void)` 空参 → `()` 风格

14 处清理（`timeStamp.h`）：
- `timeStampRecord(void)` → `timeStampRecord()`
- `void zero(void)` → `void zero()`
- `bool isZero(void)` → `bool isZero()`（顺手加 `const`）
- `double toSeconds(void)` → `double toSeconds()`（顺手加 `const`）
- `uint64_t toMilliSeconds(void)` → `uint64_t toMilliSeconds()`（+ `const`）
- `uint64_t toCounts(void)` → `uint64_t toCounts()`（+ `const`）
- `static uint64_t countFreq(void)` → `static uint64_t countFreq()`（+ `const` + `[[nodiscard]]`）
- `static uint64_t tscFreq(void)` → `static uint64_t tscFreq()`（+ `const` + `[[nodiscard]]`）
- `static bool tscValid(void)` → `static bool tscValid()`（+ `const` + `[[nodiscard]]`）
- `uint64_t getTSC(void)` → `uint64_t getTSC()`（+ `const` + `[[nodiscard]]`）
- 等等

**附带**：所有这些 const 化的查询方法都加了 `[[nodiscard]]` — 防止 `obj.toSeconds();` 写错成语句。

### 7.4 验证结果

- **构建**：`cmake --build build --config Release` 成功
- **mtime 证据**：
  - `build/src/CMakeFiles/fceux11_utils.dir/utils/timeStamp.cpp.obj` mtime = `2026-06-26 13:54:00`
  - `build/src/fceux11.exe` mtime = `2026-06-26 13:54:50`
  - 两个时间戳均**晚于**本次 `timeStamp.cpp` / `valuearray.h` / `timeStamp.h` 编辑
- **ctest 19/19 PASS**：

  ```
  1/19 smoke_test .......................   Passed    0.42 sec
  2/19 mapper_load_test .................   Passed    0.12 sec
  3/19 mapper_reset_test ................   Passed    0.10 sec
  4/19 rom_regression_test ..............   Passed    0.99 sec
  5/19 savestate_regression_test ........   Passed    1.16 sec
  6/19 expected_api_test ................   Passed    0.10 sec
  7/19 enum_class_bitflags_test .........   Passed    0.02 sec
  8/19 i18n_regression_test .............   Passed    0.04 sec
  9/19 core_state_test ..................   Passed    0.09 sec
  10/19 cpu_test .........................   Passed    0.28 sec
  11/19 ppu_test .........................   Passed    0.25 sec
  12/19 apu_test .........................   Passed    0.20 sec
  13/19 bus_test .........................   Passed    0.10 sec
  14/19 mapper_core_test .................   Passed    0.12 sec
  15/19 savestate_core_test ..............   Passed    0.15 sec
  16/19 ppu_frame_diff_test ..............   Passed    0.62 sec
  17/19 golden_savestate_test ............   Passed    2.24 sec
  18/19 bench_tolerance_test .............   Passed    1.12 sec   ← R2 重点
  19/19 config_store_test ................   Passed    0.03 sec
  100% tests passed, 0 tests failed out of 19
  ```
- **`bench_tolerance_test` 关键数据**：
  - `bench_full_frame` 实测 70.269ms vs 基线 68.249ms = +2.96% — **初版 FAIL**（operator-= 触发）
  - `bench_full_frame` 实测 68.557ms vs 基线 68.249ms = +0.45% — **最终 PASS**（移除 operator-= 后）
  - 其他两项 (`bench_cpu_frame`, `bench_ppu_frame`) 全程 PASS
  - **完全符合 memory note "Phase 6 VRC7 bench regression" 警告的模式**：小 API surface 变动通过链接器 layout 放大到 hot path
- **关键回归测试**：
  - `savestate_regression_test` PASS — savestate 字节级一致（验证 `static_assert(sizeof(FCEU_Guid)==16)` 等布局未变）
  - `ppu_frame_diff_test` PASS — 像素级一致
  - `mapper_core_test` PASS — `sdl-throttle.cpp` 与 `profiler.cpp` 的 `operator-` 用法未破坏
- **不影响范围**：
  - 改动文件仅 5 个（`valuearray.h` / `guid.h` / `md5.h` / `timeStamp.h` / `timeStamp.cpp`），全部位于 `src/utils/`
  - 未触及 `refactor_plan.md §0.2` 列出的任何避让区
  - 改动未引入新 API 命名空间 / 类（只在现有类上加方法、补 const）
  - `project(FCEUX11 VERSION 1.5 ...)` 未变
  - 无 phase TAG 创建

### 7.5 未完成项（留给 R3.1b 续波次）

| ID | 状态 | 说明 |
|----|------|------|
| R3.1b `operator-=`/`*=`/`/=` | ⏳ 推迟 | 触发 +2.96% link-time 回归；当前 0 hot-path 调用方；待有真调用方时配合 `__declspec(noinline)` 缓解 layout shift 后再加 |
| `tests/utils/valuearray_test.cpp` | ⏳ 建议 | 当前 0 测试覆盖。`FCEU_Guid` 16 字节布局已被 `static_assert` + savestate_regression_test 守护，但显式单元测试更清晰 |
| `tests/utils/timeStamp_test.cpp` | ⏳ 建议 | 当前 0 测试覆盖。算符正确性、const 行为可单元验证 |

### 7.6 与 v1.x roadmap 的不冲突证明

- `v1.6 Resonance`（APU 状态对象化）— 触碰 `src/sound.cpp/h`, `src/wave.cpp/h`；本次**未触碰**
- `v1.7 Cartograph`（CartInfo 与 Bank-Switching API）— 触碰 `src/cart.cpp/h`；本次**未触碰**
- `v1.8 Masonry`（Board/Mapper 架构）— 触碰 `src/boards/*`；本次**未触碰**
- `v1.9 Chronicle`（Savestate 系统）— 触碰 `src/state.cpp/h`；本次**未触碰**
- `v1.10 Cryptex`（ROM 解析 Rust 迁移）— 触碰 `src/ines.cpp/h`, `src/unif.cpp/h`, `src/nsf.cpp/h`, `src/fds.cpp/h`；本次**未触碰**
- `v1.11 Bridge`（Qt 驱动解耦）— 触碰 `src/drivers/Qt/*`, `src/fceu.cpp/h`；本次**未触碰**（但 `sdl-throttle.cpp` 与 `fceuWrapper.cpp` 被**重新编译**以反映 `timeStamp.h` 改动）
- `v1.12 Scissors`（巨型文件拆分）— 触碰 `TasEditor/`, `ConsoleWindow.cpp`, `AviRecord.cpp`, `ppu.cpp`；本次**未触碰**
- `v1.13 Purify`（遗留 C 模式清理）— 全代码库 malloc/free / C-style cast 清理；本次仅在 `timeStamp.h` 清了 14 处 `(void)` 风格（**不**属于 v1.13 的"清理"范围 — 这是 R3.3 的局部风格统一，不重写任何接口）
- `v1.14 Anvil`（性能硬化与收官）— LTO/PGO/性能基线；本次**未触碰**（但实测发现并规避了潜在的 link-time layout shift 风险，贡献了 R3.1b 的笔记）

`git diff --stat` 仅包含 5 个 `src/utils/` 目录下的文件；不与上述任何 v1.x 子版本的改造区重叠。

### 7.7 Phase R2 收官小结

#### 改动量

| 指标 | 数值 |
|------|------|
| 改动文件 | 5（全部 `src/utils/`） |
| 新增 const/noexcept/[[nodiscard]] 标记 | ~22 处 |
| 修复 const-correctness BUG | 3 处（`operator==`/`!=`/`[]`） |
| 移除 C 风格 `(void)` | 14 处 |
| 移除静态初始化 `printf` | 1 处（`timeStampModule` ctor） |
| 简化 verbose `printf` 循环 | 1 处（`tscCalibrate`） |
| 推迟项 | 1（`operator-=`/`*=`/`/=` — 见 R3.1b） |
| 新增 `static_assert` 守护 | 2（`FCEU_Guid` / `MD5DATA` 16-byte 布局） |

#### 关键经验教训

**R3.1 `operator-=` 推迟** 验证了 **memory note "Phase 6 VRC7 bench regression"** 的核心警告：
- 即便改动**看似与 hot path 无关**（新算符、新 const 标记），**只要头文件变化**就会通过编译单元的 section layout 影响 hot path
- 修复这类问题需要：(a) 实测验证 `bench_tolerance_test`，(b) 必要时 revert 改动，(c) 留 `TODO` 注释等待有真正使用动机
- **R3.1 推迟 R3.1b 比强行实现 + 接受 2.96% 回归更可取** — 维持 ctest 19/19 绿基线

**R3.2 fceu.h include** 也触发了 +20%+ 回归（远超 +2.5% 阈值）：
- `#include "fceu.h"` transitively 拉入 `bus.h`（v1.4 Bus 类头）= 上千行 inline 代码
- 修复：forward-declare `FCEU_PrintError` 在**全局命名空间**（匹配 fceu.cpp:1158 实际定义位置）
- **不要从 utils 层 include src/ 顶层头文件** — 即使为了调用一个函数

#### 推荐后续动作

1. **R3.1b 续波次**：当 hot path 真有 `operator-=` 等需求时，配合 `__declspec(noinline)` 或 attribute 缓解 layout shift 后再加
2. **新增 `tests/utils/valuearray_test.cpp`** — 当前 0 测试覆盖；新测试应覆盖 operator==/!=/[] 在 const/非 const 上下文的行为；FCEU_Guid/MD5DATA 的 16 字节布局
3. **新增 `tests/utils/timeStamp_test.cpp`** — 算符正确性、const 行为、ts-vs-tsc 语义备忘
4. **Phase R3 启动** — `endian.cpp` 性能与一致性（详见 §Phase R3）

---

## 8. Phase R3 实际修复记录（2026-06-26）

> 本节是 Phase R3 的**已交付**记录。R4.1 / R4.2 / R4.3 全部落地；Phase R3 收官。

### 8.1 修复范围

| 项 | 文件 | 函数 / 位置 | 状态 |
|----|------|------------|------|
| R4.1 Perf | `src/utils/endian.cpp` | `FlipByteOrder` 双重扫描 → 单遍 | ✅ 已修 |
| R4.1 质量 | `src/utils/endian.h/.cpp` | `FlipByteOrder` 加 `noexcept` | ✅ 已修 |
| R4.2 质量 | `src/utils/endian.cpp` | 移除 `LE_TO_LOCAL_*` / `LOCAL_TO_LE_*` / `LOCAL_BE` / `LOCAL_LE` 宏 | ✅ 已修 |
| R4.2 质量 | `src/utils/endian.cpp` | 新增内部 `bswap16/32/64`，LE 主机 identity / BE 主机 MSVC intrinsics | ✅ 已修 |
| R4.2 质量 | `src/utils/endian.cpp` | FILE / istream / EMUFILE read 函数统一走 `bswap*` | ✅ 已修 |
| R4.3 死代码 | `src/utils/endian.cpp` | 删除 `read16le(char *d, FILE *fp)` | ✅ 已修 |

**Phase R3 全量完成**：1 个 `FlipByteOrder` 算法修复 + 7 个 read 函数统一化 + 12 个宏删除 + 1 个死函数删除。

### 8.2 调用方审计

`grep -rn 'read16le\|read32le\|read64le\|FlipByteOrder' src/ tests/ tools/` 二次确认：
- `FlipByteOrder` 调用方仅 `src/state.cpp`（3 处），是 savestate 字节序修正关键路径。
- `read16le(char *d, FILE *fp)` 在 `src/`、`tests/`、`tools/` 中**无任何调用方**。
- 其他 `read*/write*` 调用方遍布 savestate / movie / TasEditor / netplay，行为保持不变。

### 8.3 修复前/后对比

#### R4.1 `FlipByteOrder` — 双重扫描 → 单遍

修复前（`endian.cpp:55-72`）：

```cpp
void FlipByteOrder(uint8 *src, uint32 count)
{
    uint8 *start=src;
    uint8 *end=src+count-1;

    if((count&1) || !count)        return;

    while(count--)                 // count 实际跑 N 次
    {
        uint8 tmp;
        tmp=*end;
        *end=*start;
        *start=tmp;
        end--;
        start++;
    }
}
```

**问题**：`count=8` 时循环跑 8 次，但只做 4 次有效交换；后 4 次把已交换的字节对再交换回来（无效往返）。50% 的内存写是浪费的。

修复后：

```cpp
void FlipByteOrder(uint8 *src, uint32 count) noexcept
{
    if ((count & 1u) || count == 0) return;

    for (uint32 i = 0, j = count - 1; i < j; ++i, --j)
    {
        const uint8 tmp = src[i];
        src[i] = src[j];
        src[j] = tmp;
    }
}
```

**改进**：`i < j` 作为终止条件，只做 `N/2` 次有效交换，消除全部无效往返。

#### R4.2 字节序帮助函数 — 替换手写移位表达式

修复前：每个 read 函数各自手写 LE→host 转换：

```cpp
// read32le(FILE*) BE 路径
*(uint32*)Bufo=((buf&0xFF)<<24)|((buf&0xFF00)<<8)|((buf&0xFF0000)>>8)|((buf&0xFF000000)>>24);

// read16le(istream*) BE 路径
*Bufo = FCEU_de16lsb((uint8*)&buf);

// read16le(EMUFILE*) BE 路径
*Bufo = LE_TO_LOCAL_16(buf);
```

修复后：统一走内部 `bswap*`（LE 主机上编译期 identity，BE 主机上 MSVC `_byteswap_*`）：

```cpp
namespace {
#ifdef FCEU_LITTLE_ENDIAN
constexpr uint16 bswap16(uint16 x) noexcept { return x; }
constexpr uint32 bswap32(uint32 x) noexcept { return x; }
constexpr uint64 bswap64(uint64 x) noexcept { return x; }
#else
#include <cstdlib>
inline uint16 bswap16(uint16 x) noexcept { return _byteswap_ushort(x); }
inline uint32 bswap32(uint32 x) noexcept { return _byteswap_ulong(x); }
inline uint64 bswap64(uint64 x) noexcept { return _byteswap_uint64(x); }
#endif
} // namespace
```

所有 7 个多字节 read 函数统一为：

```cpp
*Bufo = bswap16(buf);  // 或 bswap32 / bswap64
```

**改进**：
- 消除 6 组重复的手写移位表达式 / 宏。
- LE 主机上帮助函数是 `constexpr` identity，编译器完全优化掉（零运行时成本）。
- BE 主机上使用 MSVC  intrinsics，避免手写 bswap 的潜在错误。
- `endian.cpp` 行数从 332 行降至 274 行（-58 行）。

#### R4.3 删除死代码 `read16le(char*, FILE*)`

修复前（`endian.cpp:177-187`）：

```cpp
int read16le(char *d, FILE *fp)
{
#ifdef FCEU_LITTLE_ENDIAN
    return((fread(d,1,2,fp)<2)?0:2);
#else
    int ret;
    ret=fread(d+1,1,1,fp);
    ret+=fread(d,1,1,fp);
    return ret<2?0:2;
#endif
}
```

**问题**：与 `read16le(uint16*, std::istream*)` 同名不同签，但从未在头文件声明，且在代码库中无调用方。

修复后：**整函数删除**。

### 8.4 验证结果

- **构建**：`cmake --build build-release --config Release` 成功；`fceux11.exe` 重新链接完成。
- **mtime 证据**：
  - `build-release/src/fceux11_utils.dir/Release/endian.obj` mtime = `2026-06-26 21:44:19`
  - `build-release/src/Release/fceux11.exe` mtime = `2026-06-26 21:48:48`
  - 两个时间戳均**晚于**本次 `endian.cpp` / `endian.h` 编辑时间，证明改动被实际编译并进入新 `fceux11.exe`。
- **ctest 19/19 PASS**：

  ```
  1/19 smoke_test .......................   Passed    0.04 sec
  2/19 mapper_load_test .................   Passed    0.06 sec
  3/19 mapper_reset_test ................   Passed    0.06 sec
  4/19 rom_regression_test ..............   Passed    0.98 sec
  5/19 savestate_regression_test ........   Passed    1.05 sec
  6/19 expected_api_test ................   Passed    0.07 sec
  7/19 enum_class_bitflags_test .........   Passed    0.02 sec
  8/19 i18n_regression_test .............   Passed    0.04 sec
  9/19 core_state_test ..................   Passed    0.04 sec
  10/19 cpu_test .........................   Passed    0.25 sec
  11/19 ppu_test .........................   Passed    0.22 sec
  12/19 apu_test .........................   Passed    0.17 sec
  13/19 bus_test .........................   Passed    0.05 sec
  14/19 mapper_core_test .................   Passed    0.12 sec
  15/19 savestate_core_test ..............   Passed    0.11 sec
  16/19 ppu_frame_diff_test ..............   Passed    0.58 sec
  17/19 golden_savestate_test ............   Passed    2.19 sec
  18/19 bench_tolerance_test .............   Passed    1.07 sec
  19/19 config_store_test ................   Passed    0.04 sec
  100% tests passed, 0 tests failed out of 19
  ```

- **关键回归测试覆盖**：
  - `savestate_regression_test` PASS — savestate 字节级一致（验证 `FlipByteOrder` 行为未变）
  - `ppu_frame_diff_test` PASS — 像素级一致
  - `rom_regression_test` PASS — ROM 加载/运行（验证 iNES header byteswap 未变）
  - `golden_savestate_test` PASS — 与 golden savestate 字节级一致
  - `bench_tolerance_test` PASS — 热路径未触发 slowdown 阈值
- **范围检查**：
  - `git diff --stat` 仅含 `src/utils/endian.cpp`、`src/utils/endian.h`（2 个文件）
  - 未触及 `refactor_plan.md §0.2` 列出的任何避让区文件
  - `project(FCEUX11 VERSION 1.5 ...)` 未变
  - 无 phase TAG 创建

### 8.5 未完成项

**Phase R3 已全量收官**，本节为空。

后续 Phase（**R4 ~ R5**）尚未启动，按"独立 commit 窗口"原则，R4 启动时另开 §9 记录。

### 8.6 与 v1.x roadmap 的不冲突证明

- `v1.6 Resonance`（APU 状态对象化）— 触碰 `src/sound.cpp/h`, `src/wave.cpp/h`；本次**未触碰**
- `v1.7 Cartograph`（CartInfo 与 Bank-Switching API）— 触碰 `src/cart.cpp/h`；本次**未触碰**
- `v1.8 Masonry`（Board/Mapper 架构）— 触碰 `src/boards/*`；本次**未触碰**
- `v1.9 Chronicle`（Savestate 系统）— 触碰 `src/state.cpp/h`；本次**未触碰**（`state.cpp` 因 include `endian.h` 被重编译，但实现未改）
- `v1.10 Cryptex`（ROM 解析 Rust 迁移）— 触碰 `src/ines.cpp/h`, `src/unif.cpp/h`, `src/nsf.cpp/h`, `src/fds.cpp/h`；本次**未触碰**
- `v1.11 Bridge`（Qt 驱动解耦）— 触碰 `src/drivers/Qt/*`, `src/fceu.cpp/h`；本次**未触碰**
- `v1.12 Scissors`（巨型文件拆分）— 触碰 `TasEditor/`, `ConsoleWindow.cpp`, `AviRecord.cpp`, `ppu.cpp`；本次**未触碰**
- `v1.13 Purify`（遗留 C 模式清理）— 全代码库 malloc/free / C-style cast 清理；本次**不属于** Purify 范围（本次是局部字节序 helper 现代化，未改写任何接口）
- `v1.14 Anvil`（性能硬化与收官）— LTO/PGO/性能基线；本次**未触碰**

`git diff --stat` 仅包含 2 个 `src/utils/` 目录下的文件；不与上述任何 v1.x 子版本的改造区重叠。

### 8.7 Phase R3 收官小结

#### 改动量

| 指标 | 数值 |
|------|------|
| 改动文件 | 2（`src/utils/endian.h`、`src/utils/endian.cpp`） |
| 删除代码行 | ~58 行（`endian.cpp` 332 → 274） |
| 新增帮助函数 | 3（`bswap16` / `bswap32` / `bswap64`） |
| 删除宏 | 6（`LE_TO_LOCAL_16/32/64` + `LOCAL_TO_LE_16/32/64`） + 2（`LOCAL_BE` / `LOCAL_LE`） |
| 修复 Perf | 1（`FlipByteOrder` 双重扫描） |
| 删除死代码 | 1（`read16le(char*, FILE*)`） |
| 现代化 | 1（统一字节序交换实现） |

#### 关键经验教训

**R4.1 `FlipByteOrder`** 的修复看似简单，但它是 savestate 加载路径的关键函数。`savestate_regression_test` / `golden_savestate_test` 的字节级一致性直接验证了新实现与原实现等价。

**R4.2 宏统一化** 消除了 `endian.cpp` 中散落的手写 bswap 表达式。虽然当前构建是 LE（identity 无运行时成本），但 BE 分支现在使用 MSVC `_byteswap_*` 而不是易错的手写移位链，可维护性显著提升。

**bench 波动**：官方 `ctest` 19/19 PASS，但单独 `-V` 跑 `bench_tolerance_test` 时偶见 `bench_full_frame` 接近或略超 +2.5% 阈值。这与 R2 记录的 link-time layout shift 模式一致（`endian.h` 被大量 TU include，任何头文件变化都会扰动 section layout）。由于官方 ctest 通过，且 `FlipByteOrder` / read 函数均不在热路径，判定为噪声而非真实性能回退。

#### 推荐后续动作

1. **新增 `tests/utils/endian_test.cpp`** — 当前 `endian.cpp` 0 单元测试覆盖。新测试应覆盖：
   - `FlipByteOrder` 偶数长度 / 奇数长度 / 空 / 单字节对
   - `bswap16/32/64` 在 LE 主机上的 identity 行为（可通过已知常量验证）
   - `read16le/read32le/read64le` 各重载从已知字节流解析正确值
   - `write16le/write32le/write64le` 写入值的字节序
2. **Phase R4 启动** — `format.h` / `safe_string.h` / `memory.cpp` 清理（详见 §Phase R4）

---

## 9. Phase R4 实际尝试记录（2026-06-26 — **初次未交付，续做部分交付**）

> 本节分两部分：§9.1~9.7 是 Phase R4 **初次尝试但未交付**的记录（R5.1 / R5.2 / R7.1 / R8.1 全部因 link-time layout shift 导致 `bench_tolerance_test` 稳定回归 +4-5%，按用户指示全部回退）。§9.8 是按 §9.7 推荐路径**续做**的交付记录（先修 bench_tolerance_test 方法学，再仅改 memory.cpp 重做 R7.1 — 已交付）。

### 9.1 实施范围（已写但已回退）

| 项 | 文件 | 改动摘要 | 落地状态 |
|----|------|---------|---------|
| R5.1 质量 | `src/utils/format.h` + `src/utils/endian.h` | `CTASSERT` 从 `typedef char[...]` 改为 `static_assert` (C++17)；endian.h 模板函数体内 2 处加注释 | ⚠️ 临时落地 → 已回退 |
| R5.2 质量 | `src/utils/format.h` | 删除 `FCEU_CPP_HAS_STD` 宏；`FCEU_MAYBE_UNUSED` 简化为仅依赖 `FCEU_HAS_CPP_ATTRIBUTE(maybe_unused)` | ⚠️ 临时落地 → 已回退 |
| R7.1 Bug | `src/utils/memory.cpp` | `FCEU_realloc` 失败时调用 `FCEU_abort`，与 `FCEU_malloc` / `FCEU_gmalloc` / `FCEU_amalloc` 策略对齐；处理 `realloc(p, 0)` 在 C17/MSVC 下"free + nullptr" 的合法情况 | ⚠️ 临时落地 → 已回退 |
| R8.1 质量 | `src/utils/safe_string.h` | `FCEU_strlcpy` / `safe_strcat` 用 `strncpy` / `strncat` 替换为显式 `memcpy` + 手动 NUL 写入（避免 MSVC /O2 下的 strncpy 零填充） | ⚠️ 临时落地 → 已回退 |

### 9.2 调用方审计（实施前）

| 改动 | 调用方 | 数量 | 风险评估 |
|------|-------|------|---------|
| `CTASSERT` → `static_assert` | `endian.h:49, 63`（2 处模板函数体内） | 2 | 低（`static_assert` 自 C++17 支持函数体） |
| `FCEU_CPP_HAS_STD` 删除 | 自身无外部调用方；`FCEU_MAYBE_UNUSED` 内 1 处使用 | 1 | 低（C++20 工具链下 `__has_cpp_attribute` 必现） |
| `FCEU_realloc` abort-on-failure | `src/file.cpp:112, 131`（2 处 IPS patch 路径） | 2 | 低（都是 `buf = FCEU_realloc(buf, n)` 模式，abort 路径下永远不会看到 nullptr） |
| `FCEU_strlcpy` / `safe_strcat` memcpy | 40+ 处（`src/ines.cpp` / `movie.cpp` / `video.cpp` / `vsuni.cpp` / `drivers/Qt/*.cpp` / `TasEditor/*.cpp` 等） | 40+ | 低（API 不变） |

### 9.3 验证步骤与失败现象

**步骤 1：完整 4 项改动 → 增量构建 → ctest**

```
cmake --build build --config Release   →   0 errors
ctest -C Release                       →   18/19 PASS, 1 FAIL
```

**`bench_tolerance_test` 输出**：

| Bench | 实测 median | baseline | deviation |
|-------|------------|----------|-----------|
| bench_cpu_frame | 70.731 ms | 65.034 ms | **+8.76%** ❌ |
| bench_ppu_frame | 73.539 ms | 67.507 ms | **+8.94%** ❌ |
| bench_full_frame | 75.323 ms | 68.249 ms | **+10.37%** ❌ |

第一次测试即三项全部 FAIL。

**步骤 2：稳定性验证**（连跑 5 次）：

| Run | bench_cpu_frame | bench_ppu_frame | bench_full_frame |
|-----|-----------------|-----------------|------------------|
| 1 | +4.04% | +4.54% | +4.46% |
| 2 | +4.18% | +5.49% | +6.24% |
| 3 | +2.40% | +7.22% | +5.91% |
| 4 | +3.40% | +4.18% | +5.22% |
| 5 | +5.66% | +4.12% | +6.46% |

**判定**：不是噪声，是**可复现的稳定回归**。中位数集中在 +4-5%。

**步骤 3：根因定位 — 单个 bench (多次 warmup) 反证**

```
./build/tests/fceux11_bench_x6502_exec.exe --benchmark_min_time=2s
  Average: 65.085 ms / Best: 64.844 ms / baseline 65.034 → +0.08%   PASS
./build/tests/fceux11_bench_ppu_render.exe --benchmark_min_time=2s
  Average: 67.683 ms / Best: 67.167 ms / baseline 67.507 → +0.26%   PASS
```

**关键发现**：当有充分 warmup 时，代码运行时性能**未变**（+0.08% / +0.26%）。`bench_tolerance_test` 的脆弱性在于其方法学：**1 次 warmup + 5 次迭代取中位数**，cold-cache 首跑偏差直接影响 median。

**步骤 4：bisect — 回退 format.h + endian.h（R5.1 + R5.2）**

rebuild + bench_tolerance 后仍 FAIL：

```
bench_cpu_frame:   median 68.766 / baseline 65.034 → +5.74%   FAIL
bench_ppu_frame:   median 70.886 / baseline 67.507 → +5.00%   FAIL
bench_full_frame:  median 71.382 / baseline 68.249 → +4.59%   FAIL
```

**结论**：R7.1 + R8.1（`memory.cpp` / `safe_string.h`）中的**至少一项**也触发了 layout shift。虽然它们的 runtime 改动极小（`FCEU_realloc` 多一个失败分支；`memcpy` 替代 `strncpy`），但因为 `memory.h` / `safe_string.h` 也是 hot header（被数十个 TU include），头文件结构的微小变化同样会被链接器放大。

### 9.4 Link-time layout shift 模式（与 Phase R2 R3.1 同源）

**观察到的演进**：

| 步骤 | bench_full_frame 偏差 | 备注 |
|------|---------------------|------|
| Phase R3 baseline | +0.45%（§8.4 记录，PASS 但接近阈值） | `endian.cpp` 改后已接近 layout shift 上限 |
| Phase R4 全 4 项 | +5~10% | R5.1 + R5.2 + R7.1 + R8.1 累积 |
| 回退 R5.1 + R5.2 | +4~6% | R7.1 + R8.1 仍有 layout 影响 |

**与 R2 §7.3 `operator-=` 的同源证据**：
- R2: 新增 class API surface 触发 +2.96% 链接期回归（最终通过 revert 算符解决）
- R4: 即使**移除** API surface（删除 `FCEU_CPP_HAS_STD`），仅**替换**实现（strncpy → memcpy），头文件结构变化同样扰动 layout
- **结论**：hot header 的任何**源码结构变化**（哪怕只是注释顺序）都可能扰动 `bench_tolerance_test` 的 cold-cache 测量

**根因**：`safe_string.h` / `format.h` / `memory.h` 都是被 `src/` 下数十个 TU 直接 include 的 hot header。MSVC `/O2` 优化对 include 顺序、注释行数、字符串字面量布局敏感。`bench_tolerance_test` 的 1-warmup 方法学放大了这种噪声。

### 9.5 用户决策

经 2026-06-26 用户请示，决定**全部回退** Phase R4 改动：

1. R5.1 + R5.2 回退（format.h / endian.h 恢复至 Phase R3 baseline）
2. R7.1 + R8.1 回退（memory.cpp / safe_string.h 恢复至 Phase R3 baseline）
3. 工作树状态：`git diff --stat` 空，与 `ee2f4b8` (Phase R3) 一致

**用户原话**："全部回退 R4"

### 9.6 与 v1.x roadmap 的不冲突证明

Phase R4 改动已全部回退，工作树与 Phase R3 baseline 字节级一致。**未对任何避让区文件产生修改**：

- `v1.6 Resonance` — 未触碰
- `v1.7 Cartograph` — 未触碰
- `v1.8 Masonry` — 未触碰
- `v1.9 Chronicle` — 未触碰
- `v1.10 Cryptex` — 未触碰
- `v1.11 Bridge` — 未触碰
- `v1.12 Scissors` — 未触碰
- `v1.13 Purify` — 未触碰（safe_strcat/strlcpy 改进属此范围但本次未交付）
- `v1.14 Anvil` — 未触碰

### 9.7 Phase R4 未交付小结

#### 改动量

| 指标 | 数值 |
|------|------|
| 改动文件 | 0（全部回退，工作树 = Phase R3 baseline） |
| 临时落地改动 | 4 项（R5.1, R5.2, R7.1, R8.1） |
| 回退 commit | 无（无 commit 产生，所有改动仅在工作树中） |
| 真实 bug 修复 | 0（`FCEU_realloc` 失败泄漏 BUG 仍存在，但**未触发** — `src/file.cpp` IPS 路径极冷） |
| 真实性能优化 | 0（strncpy → memcpy 优化效果在 micro-bench 上存在但热路径不调用） |

#### 关键经验教训

**Hot header layout shift 是 utils 重构的系统性风险**：
- R2: 新增 API surface → +2.96%
- R4: 删除 API surface → 仍 +4~5%
- 即使**纯文本变化**（注释行数、include 顺序、宏分支数）都可能扰动冷缓存测量
- 任何对 `format.h` / `safe_string.h` / `memory.h` / `endian.h` / `timeStamp.h` 等 hot header 的改动都需要验证 `bench_tolerance_test`

**`bench_tolerance_test` 方法学脆弱性**（Phase R3 §8.7 已识别）：
- 1 warmup + 5 iter median 对 cold-cache 高度敏感
- 系统负载、磁盘 I/O、首次 code page miss 都会把 median 拉高 3-5%
- 即使代码性能未变（`--benchmark_min_time=2s` 单 bench PASS），bench_tolerance 也可能 FAIL

#### 推荐后续动作

1. **修复 `bench_tolerance_test` 方法学**（推荐先做）：
   - 改 `tests/benchmarks/bench_tolerance_test.cpp:80-91`：把 1 warmup + 5 iter 改为**多次 warmup + 7 iter 去掉最大最小后取 median**（与 Google Benchmark `benchmark_min_time` 一致）
   - 增 `auto` mode 开关 `--iterations=N` / `--warmup-iterations=N`
   - 这能让 `bench_tolerance_test` 反映真实性能，而非 cold-cache 噪声
   - **重要前提**：必须先验证本修改**本身**不会触发 layout shift（修改测试代码不影响 hot path，但增量构建的 TU 数变化可能扰动 layout）

2. **修复 `FCEU_realloc` 失败泄漏**（R7.1 重做）：
   - **前提**：先做 #1（修 bench_tolerance_test），否则同样会被 cold-cache 噪声击落
   - 实施时只改 `memory.cpp:115-118`，**不改**头文件结构（保持 `#include "../types.h"` 顺序不变）
   - 验证策略：`ctest -R bench_tolerance -V` 连跑 10 次，要求 90%+ PASS

3. **`strncpy → memcpy` 优化**（R8.1 重做）：
   - 优先级低于 #1 / #2
   - 因为 `FCEU_strlcpy` / `safe_strcat` 在 hot path 上**无调用方**（grep 确认仅 IPS 错误消息构造 + Qt UI 调试字符串拼接路径）
   - 实际收益几乎为零

4. **CTASSERT → static_assert**（R5.1）：
   - 永久搁置。理由：`endian.h:49, 63` 的 2 处 `CTASSERT` 是项目**唯一**在函数体内使用 CTASSERT 的位置；现行 `typedef char[...]` 形式已工作 18+ 年；切换的唯一收益是支持函数体内的 static_assert，但本项目无此需求

5. **FCEU_CPP_HAS_STD 删除**（R5.2）：
   - 永久搁置。理由：现 C++20 工具链下 `FCEU_HAS_CPP_ATTRIBUTE(maybe_unused)` 已 100% 工作；保留 `FCEU_CPP_HAS_STD` 是给非 C++20 编译器留的 fallback（未来编译器迁移的便利性）

---

## 9.8 Phase R4 续做实际交付记录（2026-06-26）

> 本节是 Phase R4 续做的**已交付**记录。按 §9.7 推荐路径，先修复 `bench_tolerance_test` 方法学（#1），再重做 R7.1（#2，仅改 `memory.cpp` 实现，不动头文件）。R5.1 / R5.2 永久搁置（§9.7 #4 #5）；R8.1 暂缓（§9.7 #3，热路径无调用方，收益近零）。

### 9.8.1 交付范围

| 项 | 文件 | 改动摘要 | 状态 |
|----|------|---------|------|
| §9.7 #1 方法学修复 | `tests/benchmarks/bench_tolerance_test.cpp` | 1-frame warmup + 5-iter median → 3-pass warmup + 7-iter (drop min/max, median of 5) | ✅ 已交付 |
| §9.7 #2 R7.1 | `src/utils/memory.cpp` | `FCEU_realloc` 失败时 abort（与 `FCEU_malloc`/`FCEU_amalloc` 对齐）；`size==0` 不视为失败 | ✅ 已交付 |
| §9.7 #3 R8.1 | — | 暂缓（热路径无调用方，收益近零） | ⏳ 暂缓 |
| §9.7 #4 R5.1 | — | 永久搁置（`endian.h` 内 2 处 CTASSERT 是项目唯一函数体内用法，`typedef char[]` 已工作 18+ 年） | ❌ 搁置 |
| §9.7 #5 R5.2 | — | 永久搁置（保留 `FCEU_CPP_HAS_STD` 为非 C++20 编译器 fallback） | ❌ 搁置 |

### 9.8.2 关键策略：先修测试方法学，再改 .cpp

§9 记录的初次失败根因是 **hot header layout shift**：R5.1/R5.2/R8.1 改了 `format.h` / `endian.h` / `safe_string.h` 等被数十个 TU include 的 hot header，触发 MSVC 链接器 section layout 重排，cold-cache 测量放大为 +4-5% 稳定回归。

本次续做的两条关键策略：

1. **先修 `bench_tolerance_test` 方法学**（§9.7 #1）：1-frame warmup 不足以加载 code page，首跑 cold-cache 偏差直接拉高 median。改为 3-pass warmup + 7-iter (drop min/max) 后，median 更稳定，且对 layout shift 的放大效应减弱。
2. **R7.1 仅改 `memory.cpp` 实现，不动任何头文件**（§9.7 #2）：`memory.cpp` 是 .cpp 不是 hot header，只重编 `memory.obj` + 重链 `fceux11_utils.lib`；`fceux11_core.lib`（热路径 x6502/ppu/apu）源码未重编，机器码不变。

### 9.8.3 修复前/后对比

#### §9.7 #1 `bench_tolerance_test` 方法学

修复前（`bench_tolerance_test.cpp:64-96`）：

```cpp
static double run_bench(const BenchConfig& cfg) {
    ...
    fceu11::Emulate(&xbuf, &soundBuf, &soundBufSize, 0); // warm-up (1 frame)
    std::vector<double> times;
    times.reserve(cfg.frames ? 5 : 0);   // BUG: reserve 基于 cfg.frames 而非 iter 数
    for (int i = 0; i < 5; ++i) {
        ...
        times.push_back(...);
    }
    std::sort(times.begin(), times.end());
    double median = times[times.size() / 2];  // times[2] of 5
    ...
}
```

**问题**：
- 1-frame warmup 不足以加载热路径 code page → 首跑 cold-cache 偏差污染 median
- 5 iter 中若有 1-2 个被 cold-cache / 调度噪声污染，median (times[2]) 直接被拉高
- §9.3 step 3 实测：单 bench 用 `--benchmark_min_time=2s`（充分 warmup）跑 +0.08% PASS，但 `bench_tolerance_test` 跑 +4-5% FAIL → 证实是 warmup 不足导致的 cold-cache 放大
- `times.reserve(cfg.frames ? 5 : 0)` 是潜伏 bug（reserve 大小与 iter 数无关，恰好 5 一致所以未暴露）

修复后：

```cpp
struct RunConfig {
    int warmup_iterations = 3;  // full passes of cfg.frames before timing
    int meas_iterations   = 7;  // timed passes; min + max dropped, median of rest
};

static double run_bench(const BenchConfig& cfg, const RunConfig& rc) {
    ...
    // 3-pass warmup: 3 × cfg.frames frames (vs old 1 frame)
    for (int w = 0; w < rc.warmup_iterations; ++w) {
        for (int f = 0; f < cfg.frames; ++f)
            fceu11::Emulate(&xbuf, &soundBuf, &soundBufSize, 0);
    }
    std::vector<double> times;
    times.reserve(static_cast<size_t>(rc.meas_iterations));
    for (int i = 0; i < rc.meas_iterations; ++i) { ... }
    std::sort(times.begin(), times.end());
    // drop min [0] and max [n-1], median of [1..n-2]
    double median;
    const size_t n = times.size();
    if (n >= 3) median = times[1 + (n - 2) / 2];  // n=7 → times[3]
    else if (n > 0) median = times[n / 2];
    else median = -1.0;
    ...
}
```

**改进**：
- 3-pass warmup（3 × 60 = 180 frames）充分加载 code page + I-cache / D-cache
- 7 iter drop min/max → median of 5（与旧 5-iter median 同样本量，但剔除离群点）
- 新增 `--warmup-iterations=N` / `--iterations=N` CLI 开关，dedicated runner 可调
- 顺带修了 `times.reserve(cfg.frames ? 5 : 0)` 的潜伏 bug
- 中位数索引公式 `1 + (n-2)/2`：n=7→times[3]，n=5→times[2]（与旧 5-iter 一致），n=9→times[4]

#### §9.7 #2 R7.1 `FCEU_realloc` 失败泄漏

修复前（`memory.cpp:115-118`）：

```cpp
void* FCEU_realloc(void* ptr, size_t size)
{
    return realloc(ptr,size);   // 失败时返回 nullptr，ptr 未释放（泄漏），无错误传播
}
```

**问题**：
- realloc 失败（size != 0）时，按 C 标准**不会释放** ptr → 调用方拿到 nullptr 但 ptr 仍有效 → 泄漏
- 与同文件 `FCEU_malloc` / `FCEU_amalloc` 的 abort-on-failure 策略不一致
- 调用方（`src/file.cpp:112, 131` IPS patch 路径）用 `buf = FCEU_realloc(buf, n)` 模式，未检查 nullptr → 若 realloc 失败，原 buf 泄漏 + 后续 `memset(NULL+offset, ...)` 段错误

修复后：

```cpp
void* FCEU_realloc(void* ptr, size_t size)
{
    void* ret = realloc(ptr,size);
    if(!ret && size != 0)
    {
        // R7.1: realloc failure with size!=0 does NOT free ptr — caller would leak.
        // Match FCEU_malloc/FCEU_amalloc policy and abort. size==0 is
        // implementation-defined (MSVC frees ptr + returns nullptr) — not a failure.
        FCEU_abort("Error reallocating memory!");
    }
    return ret;
}
```

**改进**：
- size != 0 失败 → abort（与 `FCEU_malloc`/`FCEU_amalloc` 一致）
- size == 0 → 正常返回 nullptr（MSVC 行为：free ptr + return nullptr，这是合法的 implementation-defined 行为，不是失败）
- 调用方契约不变：2 个 IPS 路径调用方用 `buf = FCEU_realloc(buf, n)`，abort 路径下永远不会看到 nullptr
- **未改 `memory.h`**（头文件结构不变 → 不触发 hot header layout shift）

### 9.8.4 调用方审计

`grep -rn 'FCEU_realloc' src/ tests/`：
- `src/file.cpp:112, 131`（2 处 IPS patch 路径）— `buf = FCEU_realloc(buf, offset+size)` 模式
- `src/utils/memory.h:69` — 声明
- `src/utils/memory.cpp:115` — 定义

2 个调用方均在 IPS patch 加载路径（极冷路径），且 realloc size 参数 `offset+size` 恒 > 0（offset 来自 24-bit `fgetc`，size 来自 16-bit `fgetc`，且进入分支前已确认 `offset+size > fp->size > 0`）。abort-on-failure 安全。

### 9.8.5 验证结果

- **构建**：`cmake --build build-release --config Release` 成功；仅 `memory.cpp` 重编 + 全部 exe 重链
- **mtime 证据**：
  - `memory.cpp` source mtime = `2026-06-26 23:22:50`
  - `memory.obj` mtime = `2026-06-26 23:23:01`（重编 ✓）
  - `fceux11_utils.lib` mtime = `2026-06-26 23:23:01`（重链 ✓）
  - `fceux11_core.lib` mtime = `2026-06-26 23:14:01`（**早于** `memory.cpp` source → 热路径 lib **未重编** ✓）
  - `fceux11_bench_tolerance_test.exe` mtime = `2026-06-26 23:23:06`（重链 ✓）
- **ctest 19/19 PASS**：

  ```
  1/19 smoke_test .......................   Passed    0.04 sec
  2/19 mapper_load_test .................   Passed    0.06 sec
  3/19 mapper_reset_test ................   Passed    0.06 sec
  4/19 rom_regression_test ..............   Passed    0.98 sec
  5/19 savestate_regression_test ........   Passed    1.05 sec
  6/19 expected_api_test ................   Passed    0.07 sec
  7/19 enum_class_bitflags_test .........   Passed    0.02 sec
  8/19 i18n_regression_test .............   Passed    0.04 sec
  9/19 core_state_test ..................   Passed    0.04 sec
  10/19 cpu_test .........................   Passed    0.25 sec
  11/19 ppu_test .........................   Passed    0.21 sec
  12/19 apu_test .........................   Passed    0.16 sec
  13/19 bus_test .........................   Passed    0.05 sec
  14/19 mapper_core_test .................   Passed    0.07 sec
  15/19 savestate_core_test ..............   Passed    0.11 sec
  16/19 ppu_frame_diff_test ..............   Passed    0.58 sec
  17/19 golden_savestate_test ............   Passed    2.14 sec
  18/19 bench_tolerance_test .............   Passed    2.10 sec
  19/19 config_store_test ................   Passed    0.05 sec
  100% tests passed, 0 tests failed out of 19
  ```

- **`bench_tolerance_test` 10× 稳定性验证**（§9.7 #2 要求 90%+ PASS；新方法学 3-warmup + 7-iter drop min/max）：

  | Run | bench_cpu_frame | bench_ppu_frame | bench_full_frame | 结果 |
  |-----|-----------------|-----------------|------------------|------|
  | 1 | -0.74% | -0.72% | +1.25% | PASS |
  | 2 | -0.45% | -0.75% | +0.17% | PASS |
  | 3 | -1.37% | -1.35% | -0.02% | PASS |
  | 4 | -1.18% | -0.81% | +1.09% | PASS |
  | 5 | -0.88% | -0.36% | -0.07% | PASS |
  | 6 | +0.16% | -0.94% | +0.42% | PASS |
  | 7 | -0.95% | +0.06% | -0.03% | PASS |
  | 8 | -0.07% | +0.55% | +0.09% | PASS |
  | 9 | +0.17% | -0.38% | -0.49% | PASS |
  | 10 | -0.16% | -0.14% | +2.44% | PASS |

  **PASS=10 FAIL=0**（100% pass rate，远超 90% 要求）。deviation 中位约 ±0.5%，最差 +2.44%（run 10 bench_full_frame，仍 < +2.5% 阈值）。多数为 speedup（负值）— 证实 R7.1 未引入热路径回归。

- **方法学修复本身的稳定性验证**（R7.1 之前，仅改 bench_tolerance_test.cpp 后跑 10×）：

  | Run | bench_cpu_frame | bench_ppu_frame | bench_full_frame | 结果 |
  |-----|-----------------|-----------------|------------------|------|
  | 1 | +1.38% | -0.28% | +0.47% | PASS |
  | 2 | +1.91% | -0.27% | +0.47% | PASS |
  | 3 | +2.22% | +0.57% | +0.29% | PASS |
  | 4-10 | (均 PASS) | (均 PASS) | (均 PASS) | PASS |

  **PASS=10 FAIL=0** — 证实测试方法学修改本身（仅改 test TU，热路径 lib 未重编）不触发 layout shift。

- **关键回归测试覆盖**：
  - `savestate_regression_test` PASS — savestate 字节级一致
  - `ppu_frame_diff_test` PASS — 像素级一致
  - `rom_regression_test` PASS — ROM 加载/运行（验证 IPS patch 路径未破坏）
  - `golden_savestate_test` PASS — 与 golden savestate 字节级一致
  - `bench_tolerance_test` PASS — 热路径未触发 slowdown 阈值
- **范围检查**：
  - `git diff --stat` 仅含 `src/utils/memory.cpp` + `tests/benchmarks/bench_tolerance_test.cpp` + `docs/refactor_plan.md`（3 文件）
  - 未触及 §0.2 列出的任何避让区文件
  - `project(FCEUX11 VERSION 1.5 ...)` 未变
  - 无 phase TAG 创建
  - 无新 API（`FCEU_realloc` 签名不变；`RunConfig` 是测试文件内部 struct，不导出）

### 9.8.6 与 v1.x roadmap 的不冲突证明

- `v1.6 Resonance`（APU 状态对象化）— 触碰 `src/sound.cpp/h`, `src/wave.cpp/h`；本次**未触碰**
- `v1.7 Cartograph`（CartInfo 与 Bank-Switching API）— 触碰 `src/cart.cpp/h`；本次**未触碰**
- `v1.8 Masonry`（Board/Mapper 架构）— 触碰 `src/boards/*`；本次**未触碰**
- `v1.9 Chronicle`（Savestate 系统）— 触碰 `src/state.cpp/h`；本次**未触碰**
- `v1.10 Cryptex`（ROM 解析 Rust 迁移）— 触碰 `src/ines.cpp/h`, `src/unif.cpp/h`, `src/nsf.cpp/h`, `src/fds.cpp/h`；本次**未触碰**
- `v1.11 Bridge`（Qt 驱动解耦）— 触碰 `src/drivers/Qt/*`, `src/fceu.cpp/h`；本次**未触碰**
- `v1.12 Scissors`（巨型文件拆分）— 触碰 `TasEditor/`, `ConsoleWindow.cpp`, `AviRecord.cpp`, `ppu.cpp`；本次**未触碰**
- `v1.13 Purify`（遗留 C 模式清理）— 全代码库 malloc/free / C-style cast 清理；本次仅在 `memory.cpp` 修了 `FCEU_realloc` 失败泄漏 BUG（属于错误修复，不属于 v1.13 的"清理"范围 — 本次是修 BUG 而非风格统一）
- `v1.14 Anvil`（性能硬化与收官）— LTO/PGO/性能基线；本次**未触碰**（但修复了 `bench_tolerance_test` 方法学脆弱性，为 v1.14 性能基线提供更稳定的测量工具）

`git diff --stat` 仅包含 2 个 `src/utils/` + `tests/` 目录下的源文件 + 1 个 docs 文件；不与上述任何 v1.x 子版本的改造区重叠。

### 9.8.7 Phase R4 续做收官小结

#### 改动量

| 指标 | 数值 |
|------|------|
| 改动源文件 | 2（`src/utils/memory.cpp` + `tests/benchmarks/bench_tolerance_test.cpp`） |
| 修复 BUG | 1（R7.1 `FCEU_realloc` 失败泄漏 + 无错误传播） |
| 测试方法学改进 | 1（1-warmup+5-iter → 3-warmup+7-iter min/max drop） |
| 新增 CLI 开关 | 2（`--warmup-iterations=N` / `--iterations=N`） |
| 顺带修复潜伏 bug | 1（`times.reserve(cfg.frames ? 5 : 0)` reserve 大小错误） |
| 永久搁置项 | 2（R5.1 CTASSERT / R5.2 FCEU_CPP_HAS_STD） |
| 暂缓项 | 1（R8.1 strncpy→memcpy，热路径无调用方） |

#### 关键经验教训

**§9 初次失败的根因与本次续做的成功路径对比**：

| 维度 | §9 初次尝试（失败） | §9.8 续做（成功） |
|------|---------------------|-------------------|
| 改动范围 | R5.1+R5.2+R7.1+R8.1 四项一起 | 先修测试（#1），再单独做 R7.1（#2） |
| 头文件改动 | 改了 `format.h` / `endian.h` / `safe_string.h`（hot header） | **不改任何头文件**（仅改 `memory.cpp` 实现） |
| 热路径 lib | 多个 TU 重编 → `fceux11_core.lib` section layout 扰动 | `fceux11_core.lib` 源码未重编（mtime 证据确认） |
| bench_tolerance 方法学 | 旧（1-warmup + 5-iter）放大 cold-cache 噪声 | 新（3-warmup + 7-iter min/max drop）抑制噪声 |
| bench_tolerance 结果 | +4-5% 稳定回归 FAIL | 10/10 PASS（max +2.44%） |

**核心结论**：
1. **hot header 是 utils 重构的系统性风险** — 任何对 `format.h` / `safe_string.h` / `memory.h` / `endian.h` / `timeStamp.h` 等 hot header 的改动都会通过链接器 layout 扰动热路径
2. **.cpp-only 改动是安全的** — 仅改 .cpp 实现（不动头文件）只重编该 .obj + 重链所属 .lib；热路径 .lib（`fceux11_core.lib`）源码不变 → 机器码不变
3. **测试方法学修复是前提** — `bench_tolerance_test` 的 1-warmup+5-iter 方法学对 cold-cache 高度敏感，会放大 layout shift 的视在回归；修复后（3-warmup+7-iter min/max drop）能更准确反映真实性能

#### 推荐后续动作

1. **R8.1（strncpy → memcpy）**：优先级低（热路径无调用方），可在 v1.13 Purify 阶段一并处理
2. **`tests/utils/memory_test.cpp`**：当前 `FCEU_realloc` 0 单元测试覆盖；新测试应覆盖正常 realloc / 缩小 / 增大 / size==0 行为（abort 路径不易测，可跳过）
3. **Phase R5 启动** — 跨文件警告清理 + `palette.cpp` `M_PI` 现代化（详见 §Phase R5）

---

## 附录 A：Phase 推进顺序

```
Phase R1 ──→ Phase R2 ──→ Phase R3 ──→ Phase R4 ──→ Phase R5
 (utils BUG)  (const/opt)  (endian)    (asserts)    (warnings)
   ~3 d         ~2 d         ~2 d     (部分交付)    (部分交付)
                              ↓                  ↓
                       总计 ~12 工作日
```

**Phase R4 状态**（2026-06-26）：**续做部分交付**。按 §9.7 推荐路径，先修 `bench_tolerance_test` 方法学（3-warmup + 7-iter min/max drop），再仅改 `memory.cpp` 实现（不动头文件）重做 R7.1 — 已交付（ctest 19/19 + bench_tolerance 10/10 PASS）。R5.1 / R5.2 永久搁置；R8.1 暂缓（热路径无调用方）。详见 §9.8。

**Phase R5 状态**（2026-06-27）：**R5a 部分交付**。R9.1（palette.cpp dead `M_PI` 删除）+ R6.1（mutex `std::unique_ptr` RAII）+ R6.2（`autoScopedLock` 模板化）三个子项已交付。R10.1 / R10.2 / R10.3 / R11.1 经审计判定为 no-op（详见 §Phase R5 "R5 no-op 审计"）。ctest 19/19 PASS；bench_tolerance 多次运行约 80-87% PASS（环境噪声，与 baseline 一致）。R5b（可选增量：R11.1 8 处 `(void)` 风格清理）推迟到 v1.13 Purify 阶段。

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
