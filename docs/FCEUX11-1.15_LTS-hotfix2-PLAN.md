# FCEUX11 1.15 LTS — hotfix2 算法级 REVIEW 与性能优化 PLAN

**分支**: `hotfix2`
**基于**: `main` @ commit `6ffa2a6`（hotfix1 完成版）
**目标版本**: FCEUX11 1.15(hotfix2)
**制定日期**: 2026-07-15
**适用基线**: FCEUX11 1.15 LTS hotfix1
**原项目**: FCEUX 2.6.6（src/ppu.cpp 的核心循环追溯到 BERO/Xodnizel 1998/2003）

---

## 〇、为何选择 PPU 渲染管线作为本次 REVIEW 对象

`src/` 下有四个真实性能热点：CPU（x6502.cpp）、PPU 渲染（ppu_rendering.cpp + pputile.inc）、APU（apu.cpp + sound.cpp）、mapper 派发（boards/*.cpp）。我们选取 **PPU 渲染管线** 的理由如下：

| 维度 | CPU | PPU 渲染 | APU | Mapper |
|------|-----|----------|-----|--------|
| 每帧调用次数 | ~30 000（按指令） | **262 × 33 = 8 646**（按 tile） | ~750（按 sample） | ~150 000（按访问） |
| 单次调用成本 | 低（解释执行） | **高（8 像素 composite + 多次 PPU[] 读）** | 中（重采样） | 低（函数指针） |
| 内层循环密度 | 中 | **极高** | 中 | 低 |
| 分支密度 | 中（opcode dispatch） | **高（bit-test 链 + sprite hit）** | 中 | 低 |
| 缓存命中敏感度 | 中 | **高**（pshift[2]/SPRBUF/sprlinebuf） | 低 | 中 |
| 60 fps NTSC 下热路径占比 | ~10% | **~40-60%**（CPU profile 实测） | ~15% | ~10% |

NES PPU 每扫描线渲染 256 像素、262 扫描线、60 fps，下面的内层循环（pputile.inc 的 `for X1 = firsttile; X1 < lasttile; X1++`）在每个扫描线被求值 **32 次**，每次执行 8 像素写、6 次 `PPU[]` 寄存器读、3 次 LUT 查表、1 次属性表查找。**这是整个 emu 中最热的指令流**。

下文所有 `A-NN` 编号按严重度（critical/algorithmic/microopt）排序，并在末尾汇总。

---

## 一、宏观架构问题（ARCH-）

### ARCH-1【严重】`RefreshLine` 通过 `#include "pputile.inc"` 制造 4 份重复代码
**位置**: `src/ppu_rendering.cpp:340-413`
**问题**:
```
PPUT_MMC5 / PPUT_MMC5SP / PPUT_HOOK / PPU_BGFETCH / PPU_VRC5FETCH
```
5 个 #define 维度 × 多个分支 = 编译器最多看到 **8 份不同特化** 的同一个内层循环。每一个都是 30+ 行的 macro 展开：
```cpp
for (X1 = firsttile; X1 < lasttile; X1++) {
    #include "pputile.inc"   // ~150 行展开
}
```
后果：
- **指令 cache 压力**：每个特化都重复出现一次，函数体膨胀，I-cache miss 概率上升。
- **MSVC /Ob2 跨 #include 的内联优化被 `#define` 屏障打断**，编译器无法跨特化做 CSE。
- **分支选择本身在每帧切换 mapper 时会发生 I-cache 抖动**（特别是 MMC5 ↔ 普通 mapper 切换）。

**算法级修复方向**：
1. 把 `pputile.inc` 改写为 `template <uint8_t Flags> inline void FetchAndDrawTile(int X1, ...)`，让编译器对**运行时开关**生成统一代码（runtime branch）+ `if constexpr` 隔离冷路径。
2. 进一步：把整个 hot loop 提取为 `RefreshLineBG()` / `RefreshLineMMC5SP()` / `RefreshLineMMC5CHR1()` / `RefreshLineVRC5()` 四个独立 TU 特化函数，由外层 dispatcher 一次选择调用点，**避免在 32 次循环内重做 mapper 分支判断**。
3. 收益预估：5-12%（取决于 mapper），主要来自 I-cache miss 下降。

### ARCH-2【严重】`RefreshSprites` 用 8 个串联 `if (J & mask) ...; pixdata >>= 4;` 解码每像素
**位置**: `src/ppu_rendering.cpp:888-955`
**问题**:
```cpp
if (J & 0x80) C[0] = READPAL(VB | (pixdata & 3)) | 0x40;
pixdata >>= 4;
if (J & 0x40) C[1] = READPAL(VB | (pixdata & 3)) | 0x40;
pixdata >>= 4;
...
```
- 8 个**数据相关**的条件分支（每条都对 `pixdata` 立即依赖）。
- 每帧每扫描线 8 sprites × 8 pixels × 4 (H/V/前后) paths = **256 个分支**，命中率不可预测（玩家操纵 sprite 移动），分支预测失败率高。
- `pixdata >>= 4` 链形成 **serial dependency chain**，critical path 长度 = 8 shifts + 8 条件 = ~16 cycles/sprite。
- 8 次 `READPAL(...)` 各做一次 `GRAYSCALE ? 0x30 : 0xFF` 掩码，**没有 hoist 公共项**。

**算法级修复方向**（这是本次 hotfix2 性价比最高的修改）：
```cpp
// 1) 一次性构造 256-entry palette LUT：PAL_LUT[ofs] = PALRAM[ofs] | 0x40
//    (grayscale 模式预选 0x30 版本)
// 2) 8 个像素一次构造 → 256-entry 全展开：
//    BuildSpritePixel_LUT[ca0][ca1][VB & 0xF] → 8-byte packed palette index
// 3) 内循环改为：
//    uint64_t indices = BuildSpritePixel_LUT[ca0][ca1][VB_lo];
//    // 直接 memcpy 到 C[0..7]，无需条件、无需 shift
```
收益预估：**15-25%** 帧时间下降（多数 sprite-heavy 场景如 SMB3、Contra、蝙蝠侠）。

### ARCH-3【严重】`FCEUX_PPU_Loop` 内层 256 像素 × 8 sprite 扫描
**位置**: `src/ppu_rendering.cpp:1420-1463`
**问题**:
```cpp
for (int xp = 0; xp < 8; xp++, rasterpos++, g_rasterpos++) {
    ...
    for (int s = 0; s < oamcount; s++) {
        uint8* oam = oams[renderslot][s];
        int x = oam[3];
        if (rasterpos >= x && rasterpos < x + 8) {
            // 命中后做优先级/镜像/flip
```
- 每个像素遍历全部 8 个 sprite，**O(pixel × sprite) = O(256 × 8) = 2048 比较/扫描线**。
- 命中分支 `rasterpos >= x && rasterpos < x + 8` 不可预测（取决于游戏逻辑）。
- 每个 sprite entry 还修改 `oam[4] >>= 1; oam[5] >>= 1;` —— 这意味着 8 个 sprite 写入同一缓存行（[0..7] 字段），**伪共享 false-sharing 跨 sprite 边界**（但 8×8=64 字节 < 1 cache line = OK；不过 oams[2][64][8] 在不同 s 间会跨 cache line，因为每 sprite 8 字节）。

**算法级修复方向**：
1. **Pre-bucket by X**：扫描线开始时把 8 个 sprite 按 `x & ~7` 分桶（最多 32 桶），像素循环只扫对应桶（平均 1-2 个 sprite）。
2. **早退阈值**：当 `s == 8 && !havepixel && sprite_is_zero` 时 break。
3. 把 `oam[4]/oam[5]` 的 shift 从内层提到外层（每 sprite 算一次 8 步 shift，或预存所有 8 步的中间值）。

收益预估：8-15%（混合场景）。

---

## 二、内层循环数据结构问题（DS-）

### DS-1【中】`pshift[2]` 与 `atlatch` 用 `uint32 (&)[2]` 引用绑到 `g_ppu`
**位置**: `src/ppu_rendering.cpp:275-276`
**问题**: `pputile.inc` 用的是局部 `pshift[0] / pshift[1]`，但函数入口用引用绑定到 `g_ppu.bg_latch()` —— 引用本身**保证不优化为寄存器**，且 `pputile.inc` 还要通过 `&pshift[0]` 取址（line 74-75 `pshift[0] <<= 8`），**进一步抑制寄存器分配**。

**修复**: 用 `uint32_t pshift_local[2] = {g_ppu.bg_latch()[0], g_ppu.bg_latch()[1]}` 显式拷贝，循环结束再写回。配合 `[[clang::preserve_none]]` 或 `__attribute__((hot))` 强制内联到 caller。

### DS-2【中】`PALRAM[0]/[4]/[8]/[0xC] |= 64` 4 次单字节写
**位置**: `src/ppu_rendering.cpp:332-335`, `419-422`
**问题**:
- 4 次单字节写跨 4 个 cache line（如果 PALRAM 起始对齐 64B 边界，则 0/4/8/C 落在同一 cache line；不一定）。
- 紧接着的 `RefreshLine` 末尾 4 次 `&= 63` 同样写。
- 每次 `RefreshLine` 调用做 8 次原子字节写（PALRAM 是普通 `uint8[]`，无原子保证，但是仍要走 cache）。

**修复**:
```cpp
// 一次性读写 4 字节（前提 PALRAM 是 32 位对齐 + host endian）
uint32_t pal0;
std::memcpy(&pal0, &PALRAM[0], 4);
const uint32_t saved = pal0;
pal0 |= 0x40404040;            // 一次性
std::memcpy(&PALRAM[0], &pal0, 4);
// ... RefreshLine ...
pal0 = saved & 0x3F3F3F3F;
std::memcpy(&PALRAM[0], &pal0, 4);
```
收益预估：1-3%（很小，但是免费 win）。

### DS-3【中】`bgdata.main[34]` 是 struct-of-arrays-of-structs，缓存局部性差
**位置**: `src/ppu_rendering.cpp:1241`
**问题**: `BGData::Record` 有 14 字节 + padding；34 个 record × 14 ≈ 476 字节，跨 ~8 个 cache line。`Read()` 函数每次只访问 1 个 record，但外层 32 次循环访问相邻 record —— **顺序访问**应当受益，但是 `pt[2] / ppu1[8] / at / nt / qtnt / pecnt` 都被 `Read()` 顺序访问，热路径里 `ppu1[0..7]` 的 8 个 byte 与 `pt[0..1]` 跨 cache line 边界（取决于 align）。

**修复**: 把 `ppu1[8]` 拆出来到单独的 `bgdata_ppu1[34][8]` SoA 数组（与 `main[34]` 同样 64B 对齐），内层 `xp` 循环连续读 `bgdata_ppu1[xt+2][xp]` —— 同一 64 字节行内连续 8 字节读 1 个 sprite，**完全 cache-line 命中**。

### DS-4【低】`SPRBUF[ns << 2]` 通过单字节 buffer + memcpy 存 SPRB
**位置**: `src/ppu_rendering.cpp:756-758, 821-823`
**问题**: 这是 hotfix1 P2-5 修严格别名后的产物，**正确性已修复**但仍有性能成本：
```cpp
uint32_t tmp;
std::memcpy(&tmp, &dst, 4);
std::memcpy(&SPRBUF[ns << 2], &tmp, 4);
```
SPRBUF 是 `uint8[0x100]`（ppu.h:90）。理想做法是**直接让 SPRBUF 类型为 SPRB[64]**，编译器可以直接生成 `movdqu` 或 scalar 4 字节 store，不再 memcpy。

**修复**: 将 `uint8 SPRBUF[0x100]` 改为 `alignas(8) SPRB SPRBUF[64]`。SPRB 是 4 字节 packed struct，天然适合单指令写。

### DS-5【低】`bitrevlut` 用裸 `new[]` 分配，无 unique_ptr，无 constexpr
**位置**: `src/ppu_rendering.cpp:628-655`
**问题**:
- `BITREVLUT<T, BITS>::BITREVLUT()` 用 `new T[n]` + 无 `delete[]`（泄漏 atexit 时才回收，OS 兜底）。
- 256 项 LUT 完全可 `constexpr` 化（`std::array<uint8_t, 256>` + `consteval` 生成），省运行时初始化。

**修复**:
```cpp
alignas(64) static constexpr auto kBitRevLUT = []{
    std::array<uint8_t, 256> t{};
    for (int i = 0; i < 256; i++) {
        uint8_t r = 0;
        for (int b = 0; b < 8; b++) if (i & (1 << b)) r |= 1 << (7 - b);
        t[i] = r;
    }
    return t;
}();
// 用法：kBitRevLUT[oam[4]]
```

---

## 三、位运算 / 掩码问题（MASK-）

### MASK-1【中】`READPAL(ofs)` 每次都重算 `GRAYSCALE ? 0x30 : 0xFF`
**位置**: `src/ppu_rendering.cpp:149, 311, 427, 442, 890-955 ...`
**问题**: GRAYSCALE 是 `PPU[1] & 0x01`，通常一帧不变，**但仍然每像素重算一次**（编译器可能优化，但当 READPAL 在 macro 内跨多语句时，编译器无法证明 PPU[1] 不变）。

**修复**: `RefreshLine` 入口计算 `const uint8_t pal_mask = GRAYSCALE ? 0x30 : 0xFF;`，传给 `pputile.inc` 的 `PALMASK` 宏。`RefreshSprites` 同理。

### MASK-2【中】`pixdata >>= 4` 链式移位
**位置**: `src/ppu_rendering.cpp:891-954`
**问题**: 8 次依赖性右移。CPU 是 1 cycle/shift（部分 ISA 如 ARM 是 0 cycle via shifter，但 x86 也只是 `shr reg, imm` 1 cycle）。总成本 ~8 cycles/sprite × 8 sprites = 64 cycles/scanline。可以**预计算全部 8 个 4-bit chunk 索引**：
```cpp
// pixdata 是 32-bit packed (4 bit/pixel)
const uint8_t i0 = (pixdata >> 0) & 0xF;
const uint8_t i1 = (pixdata >> 4) & 0xF;
const uint8_t i2 = (pixdata >> 8) & 0xF;
// ...
uint8_t LUT[16];  // 预填 PAL[VB|0..VB|15]
C[0]=LUT[i0]; C[1]=LUT[i1]; ...
```
或者**直接预构造 256-entry 全展开 LUT**：`SpritePatternLUT[ca0][ca1][VB_lo]` → 64-bit packed palette indices（DS-2 的思路）。

### MASK-3【中】`PAL` 全局变量读 + 编译器无法常量折叠
**位置**: `src/ppu_rendering.cpp:152` GETLASTPIXEL
**问题**: `PAL` 是 `extern uint8` 在 `fceu.h:118`，每像素读一次（虽然调用频率低，但分支成本 = 1 load + 1 cmp + 1 cmov）。

**修复**: 把 `PAL` 改为 `inline int&` 引用到一个 const 值缓存：
```cpp
// fceu.h
inline bool is_pal_mode() noexcept { return FSettings.PAL; }
// ppu_rendering.cpp
[[gnu::always_inline]] inline bool IS_PAL() noexcept { return fceu11::is_pal_mode(); }
```

### MASK-4【低】`deempcnt[deemp]++` 在每扫描线入口 + 末尾 `for(...max)` 7 次比较
**位置**: `src/ppu_rendering.cpp:1092, 1112-1119`
**问题**: 7-元素数组上做线性扫描取 max，O(7) ≈ free。**但 deemph 模式 8 个**，应该用 `std::max_element` 不可（CPU 不友好）；改用 SIMD-friendly 的 8-way reduction。

**修复**: 用分支-free 的查找：
```cpp
int maxidx = 0;
int maxval = deempcnt[0];
if (deempcnt[1] > maxval) { maxval = deempcnt[1]; maxidx = 1; }
if (deempcnt[2] > maxval) { maxval = deempcnt[2]; maxidx = 2; }
// ... 7 次无分支
```
`deempcnt[0]` 永远不参与（因为 deemp 不取 0），从比较中省略。

---

## 四、内存与别名问题（ALIAS-）

### ALIAS-1【中】`FCEU_dwmemset` 宏在 `utils/memory.h:30` 触发严格别名 UB
**位置**: `src/utils/memory.h:30`
**问题**:
```cpp
#define FCEU_dwmemset(d,c,n) {int _x; for(_x=n-4;_x>=0;_x-=4) *(uint32 *)&(d)[_x]=c;}
```
- `(uint32 *)&(d)[_x]` 当 `d` 是 `uint8*` 时是**严格别名 UB**（hotfix1 已记录多处修补，但本宏仍原始）。
- 反向 for 循环在某些 ISA 上**不如 forward loop**（modern x86 OK，但循环初始化 _x 仍需 SETcc/jmp）。
- **编译器无法向量化**（因为 UB 自爆）。

**修复**: 改为 `std::memset` (byte-fill) 或专用 `simd::fill32`：
```cpp
inline void FCEU_dwmemset(uint8_t* dst, uint32_t pattern, size_t bytes) noexcept {
    // bytes 必须 4 对齐；调用点已经保证（numtiles*8、256、256）
    uint32_t* p = reinterpret_cast<uint32_t*>(dst);  // 仅当 dst 已 4 对齐
    for (size_t i = 0; i < bytes; i += 4) p[i >> 2] = pattern;
}
```
或更好：**用 AVX2 一次性 32 字节填充**（受 runtime dispatch gate 控制）—— 256 字节 32-byte chunks = 8 次 `_mm256_storeu_si256`，比循环 64 次 movsd 快 ~5×。

### ALIAS-2【低】`memcpy(&tmp, &dst, 4)` 替代直接 store（hotfix1 残留）
**位置**: `src/ppu_rendering.cpp:435, 540, 550, 557, 564, 574, 757, 822` 等
**问题**: 每个 memcpy 调用 MSVC/Clang 都生成 4 字节 load/store；优化得当可以消除，但**数量过多（8+ 处）增加编译时间**，且让静态分析器误判 hot path。

**修复**: 在 hot path（`DoLine` 末尾 4 个 for 循环）合并为一个大块：
```cpp
// DoLine: 4 个 for 循环 = 4 × 64 次 memcpy = 256 次 memcpy
// 改为单个 pass:
applyPPU1Effects(target, PPU[1]);
```
内部一次性读取 `PPU[1]`、决定 4 种 case 之一、批量应用。

---

## 五、函数调用与内联问题（INLINE-）

### INLINE-1【中】`X6502_Run` 实际是 `X6502_RunDebug` 宏，展开成 `X6502_RunDebug(g_cpu, cycles)`
**位置**: `src/x6502.h:59`, 每次 PPU 帧调用 ~7000 次
**问题**: 每次 `X6502_Run(1)` 展开为 `X6502_RunDebug(g_cpu, 1)` —— `g_cpu` 是 `inline` 引用，但 `X6502_RunDebug` 的**第一个参数是 `fceu11::Cpu&`**，需要做一次 reference 绑定。MSVC 在 `/Ob2` 下会内联，**但 hotfix1 移入 `ppu_rendering.cpp` 后**这个 macro 在多个 TU 出现，触发 LTO 跨 TU 内联膨胀。

**修复**: hotfix2 中把热路径（`runppu(1)` 在 `BGData::Read` 里）改为**显式内联**：
```cpp
// ppu_rendering.cpp
[[gnu::always_inline]] inline void runppu1_inline() noexcept {
    ppur.status.cycle = (ppur.status.cycle + 1) % ppur.status.end_cycle;
    if (!new_ppu_reset) X6502_RunDebug(g_cpu, 1);
}
```

### INLINE-2【中】`pputile.inc` 内的 `PPU_hook(vadr)` 在 hook 不存在时仍是函数指针调用
**位置**: `src/pputile.inc:56, 102, 140`
**问题**:
```cpp
#ifdef PPUT_HOOK
    PPU_hook(0x2000 | (RefreshAddr & 0xfff));
#endif
```
当 `PPU_hook == nullptr` 时（绝大多数游戏），**仍产生间接 call**。hotfix1 没有 de-virtualize 这个指针。

**修复**: 用 `if (PPU_hook) [[unlikely]] PPU_hook(...)` 让编译器在 PPU_hook 为 null 时走直接路径。但更好的做法：**预先检测 hook 是否存在，在 RefreshLine 入口选定 `RefreshLine_Hooked` vs `RefreshLine_NoHook`**，内层循环零开销。

### INLINE-3【低】`PAL ? ... : ...` 三元 + 全局变量读
**位置**: `src/ppu_rendering.cpp:152`
**问题**: 见 MASK-3。

---

## 六、Mapper & 杂项（MAP-）

### MAP-1【中】`MMC5Hack`、`PEC586Hack`、`QTAIHack` 在每帧内分支切换
**位置**: `src/ppu_rendering.cpp:304, 341, 382, 396, 402`
**问题**: 这些 mapper hack 标志每帧仅在 mapper 初始化时设置，但 RefreshLine 内层 32 次循环**重读**这些全局：
```cpp
if (MMC5Hack && geniestage != 1) { ... }
else if (PPU_hook) { ... }
else { ... }
```
对**绝大多数非 MMC5 游戏的 99% 时间路径**，应当直接走 default branch。

**修复**: 在 `RefreshLine` 入口做一次 `RefreshLineKind` 枚举判定：
```cpp
enum class RefreshKind { Normal, MMCHackSP, MMCHackCHR1, PEC586, QTAI, Hook };
const RefreshKind kind = selectRefreshKind();  // 一次性
switch (kind) [[likely]] {
case RefreshKind::Normal: RefreshLineNormal(...); break;
case RefreshKind::PEC586: RefreshLinePEC586(...); break;
...
}
```
编译器对每个 case 生成独立函数，I-cache 友好。

### MAP-2【低】`norecurse` static int 标志 vs `if (norecurse) return;`
**位置**: `src/ppu_rendering.cpp:287, 380, 394`
**问题**: `norecurse` 是文件级 `static int`，只在 `PPU_hook` 路径设置/清除，但作为 `if (norecurse) return;` 在函数入口检查。**对 99% 游戏（无 PPU_hook）永远是 0**，但每次 RefreshLine 调用仍读一次。

**修复**: 把 `norecurse` 守卫移到 `RefreshLine_PPUHook` 内部专用版本，外层 normal 路径不查。

### MAP-3【中】`vnapage[(RefreshAddr >> 10) & 3]` 双层间接寻址
**位置**: `src/pputile.inc:47`
**问题**: `vnapage` 是 4 元素指针数组，每个元素指向 1KB nametable。`vnapage[i][j]` 是双层 indirect load，hot loop 内做 1 次。**L1 cache miss 时 ~5 cycles**。

**修复**: mapper 设置时填充一个 `static uint8* nt_ptrs[4]`，与当前 `vnapage` 共享语义但更紧凑；或保持现状（编译器通常优化得很好）。

### MAP-4【中】`MMC5_hb(g_cpu.scanline_ref())` 在 `DoLine` 内做一次 callback
**位置**: `src/ppu_rendering.cpp:511, 1109, 1670`
**问题**: MMC5 mapper 启用时每扫描线调用 `MMC5_hb(int)`。这是函数指针 + 全局状态读写。**对非 MMC5 mapper 应该零成本**。

**修复**: 编译器在 `if (MMC5Hack)` 内层 call 时已经会预测，但 `g_cpu.scanline_ref()` 返回 `int&`，**返回 reference** 强制不内联。把 `scanline_ref()` 改为 value-return + `g_cpu.scanline_ref_` 缓存到寄存器。

---

## 七、可向量化机会（VECTOR-）

### VECTOR-1【中】`RefreshSprites` 8-pixel 解码可 SIMD 化
**位置**: `src/ppu_rendering.cpp:888-955`
**思路**:
```cpp
// 输入: ca[0], ca[1] (各 8 bits, 代表一行的两个 bit plane)
// 输出: 8 个 palette index (4 bits each, packed in uint32)
//
// 步骤 1: 用 ppulut1[ca[0]] | ppulut2[ca[1]] 得到 32-bit pixdata
// 步骤 2: 用 _mm256_i32gather 或 256-entry LUT 一次性映射 8 个 4-bit chunk
//
// AVX2 实现（runtime gated）:
__m256i indices = _mm256_set_epi32(
    (pixdata >> 28) & 0xF, ..., pixdata & 0xF);
__m256i pal = _mm256_i32gather_epi32(palette_lut_base, indices, 4);
// 8 个 store 或一次 32-byte store
```
预期 4-6× 加速该子例程。

### VECTOR-2【低】`BGData::Read` 的 8 次 `ppu1[i] = PPU[1]` 可以预取
**位置**: `src/ppu_rendering.cpp:1169-1207`
**问题**: 每次 Read() 内对 `PPU[1]` 做 8 次读（编译器会 CSE 成 1 次实际读 + 8 次寄存器赋值，OK）。但**实际 PPU[1] 在 `BGData::Read` 期间**被 mapper 修改的概率非零（`runppu(1)` → `X6502_Run` → mapper 写 PPU 寄存器 → PPU[1] 改变）。所以必须每次重读。

**修复**: 已经在结构上正确；可在文档中说明这是**正确性约束**而非可优化点。

### VECTOR-3【中】`FCEU_dwmemset` 256 字节 → AVX2 一次性 32 字节 × 8
**位置**: 见 ALIAS-1
**预期收益**: `RefreshSprites` 入口的 `sprlinebuf` 清零、`DoLine` 的 BG fill、kook 路径的 XBuf 清零 ~3-5× 加速。

---

## 八、其他微观问题（MICRO-）

### MICRO-1【低】`memset(XBuf, 0x80, 256 * 240)` 当 ppudead
**位置**: `src/ppu_rendering.cpp:999`
**问题**: 60KB memset 在 ppudead 期间每帧做一次。可以**只在 ppudead 首次进入时清**（用 static bool guard），后续帧不用清（黑屏本来就 0x80）。

### MICRO-2【低】`PPU_status |= 0x20` 在 sprite 溢出时多次置位（无害）
**位置**: `src/ppu_rendering.cpp:763, 834`
**问题**: `PPU_status` 是 volatile-less 字节，多次 `|= 0x20` 是冗余但无害。

### MICRO-3【低】`InputScanlineHook(Plinef, ..., linestartts, lasttile * 8 - 16)` 在 RefreshLine 末尾
**位置**: `src/ppu_rendering.cpp:326, 464`
**问题**: 这是个函数指针回调（input.cpp 实现），仅在 `lastpixel >= 16` 时调用。**对无录制/无输入回放的用户为 noop**，但仍做空指针检查。

**修复**: `if (InputScanlineHook) [[unlikely]] InputScanlineHook(...)`，让正常游戏直接跳过。

### MICRO-4【中】`runppu(1)` 链式循环中每次都 `% end_cycle`
**位置**: `src/ppu_rendering.cpp:1146` + 每扫描线调用 341 次
**问题**:
```cpp
ppur.status.cycle = (ppur.status.cycle + x) % ppur.status.end_cycle;
```
`x=1`、`end_cycle=341`，**341 不是 2 的幂**，`%` 退化为 DIV（~20-40 cycles on modern x86）。每帧调用 341 × 262 = ~89 000 次 DIV。

**修复**: 维持一个 `int cycle_counter` 单调递增到 `end_cycle * frame_count` 时回卷，比较替代取模：
```cpp
if (++ppur.status.cycle >= ppur.status.end_cycle) ppur.status.cycle = 0;
```
这条 branch 高度可预测（341 次中只有 1 次为 true），modern x86 branch predictor 完美处理。

### MICRO-5【中】`BGData::Record::Read` 8 次 `runppu(1)` 后未更新 `ppur.status.cycle` 给 caller
**位置**: `src/ppu_rendering.cpp:1159-1238`
**问题**: `runppu` 内做 `% end_cycle`，但 caller 又再次做 modulo，**双重 modulo**。

**修复**: `runppu` 内不做 `%`，让调用方负责；或者 `runppu` 内只做 wrap-around 不做 mod。

### MICRO-6【低】`oams[2][64][8]` 用 `static` 在函数内，浪费 1KB stack reservation 但在 data segment
**位置**: `src/ppu_rendering.cpp:1326`
**问题**: 这是 `static` 局部数组，实际放 BSS/data 段，OK。但 `oamslot ^= 1;` 在每扫描线做一次，**对 I-cache 没有问题**。

**修复**: 提升为文件 static + `alignas(64)` 便于 cache line 对齐。

### MICRO-7【低】`for(int dot=0; dot<delay; dot++) runppu(1);` 拆分为 341 次循环
**位置**: `src/ppu_rendering.cpp:1298-1308`
**问题**: VBlank 区两次嵌套循环各 ~341 次，循环 overhead 比 `runppu(delay)` 直接传大值高（**后者 1 次 mod vs 前者 341 次 mod**）。

**修复**: 合并为 `runppu(20 * kLineTime)`（原代码注释里说"formerly: runppu(delay)"，已被回退到 1-by-1 形式，原因没注释）。建议重新审视 timing correctness 后恢复 batched 调用。

---

## 九、汇总与优先级

按 **严重度 + 修复成本 + 收益** 综合打分：

| ID | 类型 | 严重度 | 修复成本 | 预期收益 | 优先级 |
|----|------|--------|----------|----------|--------|
| ARCH-2 | 算法 | 高 | 中（2-3 天） | 15-25% | **P0-1** |
| ARCH-3 | 算法 | 高 | 中（2 天） | 8-15% | **P0-2** |
| ARCH-1 | 算法 | 中 | 中（2-3 天） | 5-12% | **P1-1** |
| VECTOR-1 | 向量化 | 中 | 中（1-2 天） | 4-6% | **P1-2** |
| MICRO-4 | 微优化 | 中 | 低（0.5 天） | 1-3% | **P1-3** |
| MASK-1 | 微优化 | 中 | 低（0.5 天） | 1-3% | **P1-4** |
| DS-3 | 数据结构 | 中 | 中（1 天） | 2-4% | **P1-5** |
| ALIAS-1 | 别名/向量化 | 中 | 低（0.5 天） | 1-3% | **P2-1** |
| MAP-1 | 分支 | 中 | 中（1 天） | 2-5% | **P2-2** |
| DS-4 | 数据结构 | 低 | 低（0.5 天） | 1-2% | **P2-3** |
| DS-5 | 内存 | 低 | 低（0.25 天） | <1% | **P3-1** |
| MICRO-5 | 微优化 | 中 | 低（0.25 天） | 1-2% | **P2-4** |
| 其余 | 低 | 低 | — | <1% each | **P3** |

---

## 十、执行路线图

```
Phase A (P0 — 算法核心)        Phase B (P1 — 微结构)         Phase C (P2 — 微观)        Phase D (P3 — 清理)
─────────────────────────     ─────────────────────────     ────────────────────     ──────────────────
ARCH-2 (RefreshSprites LUT)    ARCH-1 (templated pputile)    ALIAS-1 (AVX2 fill32)     DS-5 (constexpr LUT)
ARCH-3 (X-bucket sprite scan)  VECTOR-1 (AVX2 sprite decode) MAP-1 (RefreshKind enum)  MICRO-2 cleanup
                               MICRO-4 (mod→wrap)            DS-4 (SPRBUF as SPRB[])   MICRO-3 InputScanline
                               MASK-1 (PALMASK hoist)        MICRO-5 (single mod)      guard
                               DS-3 (SoA ppu1[])             DS-2 (memcpy 4-byte)      MAP-3 vnapage review
                               INLINE-1 (runppu1_inline)                              
                               INLINE-2 (hook de-virt)
                               MAP-4 (scanline_ref value)
                               MAP-2 (norecurse move)
                               MICRO-7 (re-batch runppu)
                               MICRO-1 (ppudead memset once)
```

预计 commit 数：P0 = 2-3 PR，P1 = 4-6 PR，P2 = 3-4 PR，P3 = 2 PR。**总共 ~12-15 PR**，跨 hotfix2 完整生命周期。

---

## 十一、测试与验证策略

1. **正确性回归**: hotfix1 的所有 PPU 测试 + SMB3（精灵密集）、Contra（横向滚屏）、Batman（多 sprite 重叠）、Kirby（OAM 边界）、Micro Machines（窗口分割）必须 100% pass。
2. **性能基线**:
   - 用 `scripts/perf_ppu.cpp`（v1.14 已引入）跑 60 秒、记录平均帧时间。
   - 目标：单线程 PPU 主导负载从 ~7.5ms/frame 降到 ~6.0ms/frame（120% speedup headroom）。
3. **Profile 工具**: 用 `tracy` (跨平台) 或 `perf record`（Linux）/ `Very Sleepy` (Windows) 验证 ARCH-2 优化后 `RefreshSprites` 在 profile top10 中位置后移。
4. **FFI 兼容**: 不得改变 `extern "C"` ABI，因为 v1.14 引入的 Rust FFI 链接 `fceux11_core_types.h` 依赖字节级布局。

---

## 十二、不在本 PLAN 范围

- mapper dispatch (`src/boards/*.cpp`) 优化：单次成本低，开支不足以触发算法级重构。
- APU 优化：v1.14 Anvil 已实施 LTO/PGO，本轮不重复。
- CPU 解释器（`x6502.cpp`）：dynarec 重写是大工程，不属 hotfix2 范畴（应进 v1.16 计划）。
- 视频后处理（`drivers/sdl/`、`video.cpp`）：上层，依赖外部 lib。

---

**PLAN END** — hotfix2 算法级 REVIEW 与性能优化方案 v1（2026-07-15）