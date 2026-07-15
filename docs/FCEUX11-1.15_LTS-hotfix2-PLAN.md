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

## 九、PR 清单与优先级

按 **严重度 + 修复成本 + 收益** 综合打分，按可独立合入的 PR 编号。**§〇-§八** 的诊断条目 (ARCH-/DS-/MASK-/ALIAS-/INLINE-/MAP-/VECTOR-/MICRO-) 在本节作为"诊断 ID"被映射到 PR。Phase 详细实施方案见 §十，时间表见 §十三，风险矩阵见 §十四，验收清单见 §十五。

### 9.1 总览表

| PR ID | 诊断 ID | 标题 | Phase | 风险 | 预期收益 | 工时 |
|-------|--------|------|-------|------|----------|------|
| **P0-1** | ARCH-2 | `RefreshSprites` 8 像素全展开 LUT 解码 | A | 中 | 15-25% | 3 天 |
| **P0-2** | ARCH-3 | 精灵 X-bucket 预分类 + shift 预计算 | A | 中 | 8-15% | 2 天 |
| **P0-3** | ARCH-1a | `pputile.inc` → `template<uint8_t Flags>` | A | 中 | 5-12% | 2 天 |
| **P0-4** | ARCH-1b | `RefreshLine` dispatcher 与 4 个 TU 特化 | A | 低 | 1-3% | 1 天 |
| **P0-5** | VECTOR-1 | AVX2 精灵 8-pixel decode（runtime gated） | A | 中 | 4-6% | 2 天 |
| **P1-1** | DS-3 | `BGData` 中 `ppu1[]` 拆 SoA | B | 低 | 2-4% | 1 天 |
| **P1-2** | MASK-1 | `pal_mask` 入口 hoist | B | 低 | 1-3% | 0.5 天 |
| **P1-3** | MICRO-4 | `cycle` 取模改为 wrap-around | B | 低 | 1-3% | 0.5 天 |
| **P1-4** | INLINE-1 | `runppu1_inline` 显式内联 | B | 低 | <1% | 0.5 天 |
| **P1-5** | INLINE-2 | `PPU_hook` 入口去虚化 | B | 低 | <1% | 0.5 天 |
| **P1-6** | MAP-1 | `RefreshKind` 枚举分发 | B | 中 | 2-5% | 1.5 天 |
| **P1-7** | MAP-4 | `scanline_ref` 改 value-return + 寄存器缓存 | B | 低 | <1% | 0.25 天 |
| **P2-1** | ALIAS-1 | `FCEU_dwmemset` 重写 + AVX2 `fill32` | C | 中 | 3-5% | 1 天 |
| **P2-2** | DS-2 | `PALRAM[0/4/8/0xC]` 4 字节合并读写 | C | 低 | <1% | 0.25 天 |
| **P2-3** | DS-4 | `SPRBUF` 类型改 `SPRB[64]` | C | 低 | 1-2% | 0.5 天 |
| **P2-4** | MICRO-5 | `runppu` 移除冗余 modulo | C | 低 | 1-2% | 0.25 天 |
| **P2-5** | MICRO-7 | `runppu` 重批（恢复 batched 调用） | C | 中 | 1-2% | 1 天 |
| **P2-6** | DS-1 | `pshift[2]` 本地化（去除 `uint32&` 引用） | C | 低 | <1% | 0.5 天 |
| **P3-1** | DS-5 | `bitrevlut` → `constexpr std::array` | D | 低 | <1% | 0.25 天 |
| **P3-2** | MICRO-1 | `ppudead` 期间 `XBuf` memset 单次 | D | 低 | <1% | 0.25 天 |
| **P3-3** | MICRO-3 | `InputScanlineHook` `[[unlikely]]` 守卫 | D | 低 | <1% | 0.25 天 |
| **P3-4** | MAP-2 | `norecurse` 守卫移至 hook-only 内部 | D | 低 | <1% | 0.25 天 |
| **P3-5** | MAP-3 | `vnapage` 缓存复用与预取 review | D | 低 | <1% | 0.5 天 |

合计 **23 个 PR**（P0=5，P1=7，P2=6，P3=5），按 Phase A→D 顺序推进。

### 9.2 收益叠加预估

按 P0→P1→P2→P3 顺序合入，预期累计帧时间下降（vs. hotfix1 基线）：

| 阶段 | 累计收益 | 累计帧时间 | NTSC 帧预算下 headroom |
|------|---------|----------|----------------------|
| hotfix1（基线） | — | 7.5 ms | -20%（已超 16.67 ms 限制的对应比例） |
| + Phase A 全部 | 18-35% | 6.1-4.9 ms | 跳至 <70% 预算 |
| + Phase B 全部 | 22-42% | 5.85-4.35 ms | 进一步 headroom |
| + Phase C 全部 | 24-45% | 5.7-4.1 ms | 同上 |
| + Phase D 全部 | 24-45% | 5.7-4.1 ms | 同上（清理类无显著性能影响） |

> **方法学声明**: 收益数字来自 hotfix1 同类 PR 在同类硬件（Intel i5-8250U / Ryzen 5 3500U）单线程 60 秒 perf_ppu.cpp 测得上下界。**最终数据以 hotfix2 各自 PR 内的实测为准。**

---

## 十、Phase 详细实施方案

每个 PR 包含：文件定位、当前代码节选、修复方案（含代码骨架）、风险评估、验证步骤、PR 标题、关联条目。

---

### Phase A — P0 算法核心

#### P0-1【ARCH-2】`RefreshSprites` 8 像素全展开 LUT 解码

**文件**: `src/ppu_rendering.cpp:880-955`（`RefreshSprites` 内 4 路 H/V 翻转，每路 8 个 `if (J & mask)` 分支 + 8 次 `pixdata >>= 4`）

**背景**: 每帧每扫描线 8 sprites × 8 pixels × 4 paths = 256 个**数据相关**条件分支；`pixdata >>= 4` 链形成 8-cycle serial dependency。`READPAL(VB | ...)` 内部又重算 `GRAYSCALE ? 0x30 : 0xFF`。**这是 hotfix2 性价比最高的一项**。

**当前代码**（节选 H_FLIP=false / SP_BACK=false 分支, ppu_rendering.cpp:940-954）：
```cpp
if (J & 0x80) C[0] = READPAL(VB | (pixdata & 3));
pixdata >>= 4;
if (J & 0x40) C[1] = READPAL(VB | (pixdata & 3));
pixdata >>= 4;
// ... 8 次
if (J & 0x01) C[7] = READPAL(VB | pixdata);
```

**修复方案**:
1. 在 `RefreshLine` 入口构建两张 256×4-entry LUT（按 H_FLIP / !H_FLIP × GRAYSCALE / !GRAYSCALE 预生成 4 张），每张返回 8 字节 packed 输出：
   ```cpp
   // pputile_sprite_lut.h
   struct alignas(8) SpritePixelRow { uint8_t c[8]; };
   // 索引: ca[0] (8b) | ca[1]<<8 | atr_lo<<16  (atr_lo = VB_lo | (SP_BACK<<3) | (H_FLIP<<4))
   extern const SpritePixelRow kSpriteLUT[256*256*8];
   ```
   构造时机：进程启动一次（`constexpr` 或 `std::array` 静态初始化）。PAL 模式 / NTSC 模式 / 灰度 4 种各一份，全局 4 张。
2. 内循环改写为：
   ```cpp
   const uint16_t ca = ca[0] | (ca[1] << 8);
   const uint8_t atr_lo = (VB & 0xC) | (atr & SP_BACK ? 0x10 : 0) | (atr & H_FLIP ? 0x20 : 0);
   const auto& row = kSpriteLUT[atr_lo][ca];
   std::memcpy(C, &row, 8);   // 或分支: C[0]=row.c[0]; ... C[7]=row.c[7];
   ```
   分支数从 8/sprite 降到 0，shift 链消失。
3. `kSpriteLUT` 构造期间调一次 `READPAL`（4 张 × 256×256 = 256 KiB），内存成本 ~1-2 MiB，可接受（占 32 KB L1 是太多，但只在 startup 一次）。如果内存敏感，可缩为 `kSpriteLUT_gray` / `kSpriteLUT_color` 各 128 KiB，运行时按 `GRAYSCALE` 二选一。
4. 保留 `RefreshSprites` 外层 `J` 计算（`ppulut1 / ppulut2`），仅替换 8 像素解码。

**风险**:
- **正确性**：LUT 必须覆盖 `atr & SP_BACK`（前景/背景两路）与 `atr & H_FLIP`（水平翻转）共 4 路 × GRAYSCALE 2 路 = 8 张；少一张就是 silent corruption。**所有 8 张 LUT 必须由单测覆盖**。
- **可读性**：单测能立即发现 LUT 项错配，肉眼难。
- **回滚**：单 PR revert 即可恢复原宏代码。

**验证步骤**:
1. 编译：Windows / Linux / macOS 三平台 `cmake --build build` 全过
2. 单测（新增 `tests/ppu_rendering_lut_test.cpp`）：对所有 `ca ∈ [0, 65535]`、`atr_lo ∈ [0, 7]` 验证 LUT 输出 ≡ 原宏代码输出
3. 静态分析：`cppcheck --enable=all` 无新警告
4. 动态：ASan + UBSan 构建，Ubsan 启用 `-fsanitize=integer` 验证 LUT 索引无 OOB
5. 功能：
   - SMB1（精灵密集）/ Contra（横向滚动）/ Batman（多精灵重叠）/ Kirby（OAM 边界）/ Micro Machines（窗口分割）5 个手动测试场景
   - 帧时间 vs hotfix1 基线：降幅 ≥ 12%（保守目标）
6. Profile：`scripts/perf_ppu.cpp` 60 秒跑 + `tracy` 验证 `RefreshSprites` 在 profile top10 中位置后移 ≥ 5 名

**PR 标题**: `perf(ppu): replace RefreshSprites 8-pixel decode with precomputed LUT (15-25%)`

**关联**: 配合 P0-5（AVX2）作为进阶可选；为 P1-2（PALMASK hoist）做铺垫（LUT 一次性 hoist `pal_mask`）。

---

#### P0-2【ARCH-3】精灵 X-bucket 预分类 + shift 预计算

**文件**: `src/ppu_rendering.cpp:1420-1463`（`FCEUX_PPU_Loop` 内层 `for s = 0..oamcount`）

**背景**: 当前每像素遍历全部 8 个 sprite（`oams[renderslot][s]`），命中分支不可预测。`oam[4] >>= 1` / `oam[5] >>= 1` 在循环里每像素各做 8 次（覆盖同一缓存行 8 字节，cache-line 边界随机）。

**当前代码**（节选 ppu_rendering.cpp:1418-1431）：
```cpp
bool havepixel = false;
for (int s = 0; s < oamcount; s++) {
    uint8* oam = oams[renderslot][s];
    int x = oam[3];
    if (rasterpos >= x && rasterpos < x + 8) {
        uint8 spixel = oam[4] & 1;
        spixel |= (oam[5] & 1) << 1;
        oam[4] >>= 1;
        oam[5] >>= 1;
        // ...
```

**修复方案**:
1. **扫描线开始时预分桶**（32 桶，`bucket_idx = x >> 3`，范围 0..31；扫描线最多 32 个 8 像素 tile）：
   ```cpp
   // FCEUX_PPU_Loop 入口，scanline 开始时
   struct alignas(64) SpriteBucket {
       uint8_t count;
       uint8_t idx[8];   // sprite indices in this bucket
   };
   SpriteBucket buckets[32] = {};
   for (int s = 0; s < oamcount; s++) {
       uint8_t x = oams[renderslot][s][3];
       uint8_t bi = x >> 3;
       buckets[bi].idx[buckets[bi].count++] = s;
   }
   ```
2. **像素循环改扫桶**（多数像素命中 0 或 1 个 sprite）：
   ```cpp
   const uint8_t bi = rasterpos >> 3;
   for (int k = 0; k < buckets[bi].count; k++) {
       int s = buckets[bi].idx[k];
       // 原 sprite 处理逻辑
   }
   ```
3. **shift 预计算**：把 `oam[4] >>= 1` × 8 与 `oam[5] >>= 1` × 8 在**扫描线开始时**一次性做完（每 sprite 8 次 shift），存为 `oams_shift[renderslot][s][2][8]`：
   ```cpp
   // scanline 开始
   for (int s = 0; s < oamcount; s++) {
       uint8_t v0 = oams[renderslot][s][4];
       uint8_t v1 = oams[renderslot][s][5];
       for (int k = 0; k < 8; k++) {
           oams_shift[s][0][k] = v0 & 1; v0 >>= 1;
           oams_shift[s][1][k] = v1 & 1; v1 >>= 1;
       }
   }
   // 像素循环
   int sub = rasterpos & 7;
   uint8_t spixel = oams_shift[s][0][sub] | (oams_shift[s][1][sub] << 1);
   ```
4. 删除 `oam[4] >>= 1; oam[5] >>= 1;`（移至扫描线入口）。

**风险**:
- **正确性**：sprite 0 hit 检测（`oam[6] == 0`）逻辑不变；`havepixel` 早退不变；V_FLIP 路径仍用 `oam[4]/[5]` 直接解码（不预 shift），需要为 V_FLIP sprite 走原路径（`if (atr & V_FLIP) use_oam_else use_shift_table`）。**V_FLIP 路径单测覆盖**。
- **缓存**：buckets[32] × 9 字节 = 288 字节，跨 5 个 cache line，可接受。
- **回滚**：单 PR revert。

**验证步骤**:
1. 编译三平台通过
2. 单测（新增 `tests/ppu_x_bucket_test.cpp`）：对比旧 256×8 扫描 vs 新分桶扫描，每像素 `pixelcolor` / `PPU_status` / `havepixel` / sprite-0 hit 一致
3. ASan / UBSan 通过
4. 功能：5 个手动场景 + 帧时间降幅 ≥ 6%（保守目标）
5. Profile：`tracy` 验证 `FCEUX_PPU_Loop` 内层 `for s` 在 top10 中下降

**PR 标题**: `perf(ppu): pre-bucket sprites by X tile and precompute per-pixel shift (8-15%)`

**关联**: 与 P0-1 互补（前者消除条件分支，后者消除扫描冗余）。

---

#### P0-3【ARCH-1a】`pputile.inc` → `template<uint8_t Flags>`

**文件**: `src/pputile.inc`（完整文件，`RefreshLine` 4 处 `#include`）、`src/ppu_rendering.cpp:340-413`（`RefreshLine` 内 dispatcher）、`src/ppu_rendering.cpp:888-955`（`RefreshSprites` 不在内层循环，但 `pputile.inc` 也含 sprite 渲染宏）

**背景**: 5 个 `#define` 维度 (`PPUT_MMC5` / `PPUT_MMC5SP` / `PPUT_HOOK` / `PPU_BGFETCH` / `PPU_VRC5FETCH`) × 多个分支 = 编译器最多看到 8 份不同特化。每个特化都是 30+ 行宏展开，函数体膨胀，I-cache 压力高，**跨 mapper 切换 I-cache 抖动**。

**当前代码**（节选 ppu_rendering.cpp:340-413）：
```cpp
#define PPUT_MMC5
if (MMC5Hack && geniestage != 1) {
    // 4 个 PPUT_MMC5* 分支
}
#undef PPUT_MMC5
else if (PPU_hook) { ... }
// ...
```

**修复方案**:
1. 把 `pputile.inc` 整体重写为 `template <uint8_t Flags> inline FCEU_HOT void FetchAndDrawTile(int X1, ...)`，函数体用 `if constexpr ((Flags & PPUT_MMC5) != 0)` 隔离冷路径：
   ```cpp
   template <uint8_t Flags>
   __attribute__((hot)) inline void FetchAndDrawTile(/* args */) noexcept {
       if constexpr ((Flags & PPUT_MMC5) != 0) {
           // MMC5 路径
       } else if constexpr ((Flags & PPUT_HOOK) != 0) {
           // hook 路径
       } else {
           // 普通路径
       }
       // ... 共用 fetch + draw 逻辑
   }
   ```
2. 4 个 dispatcher 实例化：
   ```cpp
   template void FetchAndDrawTile<kFlagNormal>(/* ... */);
   template void FetchAndDrawTile<kFlagMMC5SP>(/* ... */);
   template void FetchAndDrawTile<kFlagMMC5CHR1>(/* ... */);
   template void FetchAndDrawTile<kFlagHook>(/* ... */);
   ```
3. dispatcher 改 switch：
   ```cpp
   switch (kind) [[likely]] {
       case RefreshKind::Normal:    FetchAndDrawTile<kFlagNormal>(...); break;
       case RefreshKind::MMC5SP:    FetchAndDrawTile<kFlagMMC5SP>(...); break;
       // ...
   }
   ```
4. `pputile.inc` 文件保留作为**单测参考**（逐步迁移，最终删除或挪到 `tests/golden/pputile.inc`）。

**风险**:
- **正确性**：每个特化须保持与原宏代码逐字节一致的输出（pixel 输出 + PPU 状态 + palette 副作用）。
- **模板实例化代码膨胀**：4 个特化各 ~5KB = 20KB I-cache，比原 8 份宏的 ~25KB 略小。
- **调试**：模板报错信息更难读；需在源码注释中保留与原宏代码的映射。

**验证步骤**:
1. 编译：4 个特化均实例化（`nm` / `objdump` 验证 4 个符号存在）
2. 单测（新增 `tests/pputile_template_test.cpp`）：对所有 `Flags` 组合，5 个手动场景画面 byte-for-byte 一致
3. ASan / UBSan 通过
4. 功能：5 个手动场景 + MMC5 mapper（Castlevania III / Just Breed）+ VRC5（Madara / Shin Megami Tensei）
5. Profile：I-cache miss 计数（`perf stat -e L1-icache-load-misses`）降幅 ≥ 5%

**PR 标题**: `refactor(ppu): convert pputile.inc to template<uint8 Flags> (5-12%)`

**关联**: 与 P0-4（dispatcher 提取）配套实施；为 P1-6（RefreshKind 枚举）做铺垫。

---

#### P0-4【ARCH-1b】`RefreshLine` dispatcher 与 4 个 TU 特化

**文件**: `src/ppu_rendering.cpp:340-413`（dispatcher 主体）、新建 `src/ppu_refresh_normal.cpp` / `pputile_mmc5sp.cpp` / `pputile_mmc5chr1.cpp` / `pputile_hook.cpp`（4 个 TU）

**背景**: P0-3 把内层循环改为模板后，仍可在 dispatcher 处做一次 mapper 选择，把外层 mapper 分支从**每 tile 32 次**降到**每扫描线 1 次**。

**修复方案**:
1. 4 个新 TU，每个导出 `RefreshLineKind_*` 函数签名：
   ```cpp
   // pputile_normal.cpp
   void RefreshLineKind_Normal(int firsttile, int lasttile, /* ... */);
   // pputile_mmc5sp.cpp / mmc5chr1.cpp / hook.cpp 类似
   ```
2. dispatcher 改 enum-driven：
   ```cpp
   enum class RefreshKind { Normal, MMC5SP, MMC5CHR1, PEC586, QTAI, Hook };
   static RefreshKind select_refresh_kind() noexcept {
       if (MMC5Hack && geniestage != 1) {
           if (MMC5HackCHRMode == 0 && (MMC5HackSPMode & 0x80)) return RefreshKind::MMC5SP;
           if (MMC5HackCHRMode == 1 && (MMC5HackSPMode & 0x80)) return RefreshKind::MMC5CHR1;
           if (MMC5HackCHRMode == 1) return RefreshKind::MMC5CHR1;
           return RefreshKind::Normal;
       }
       if (PPU_hook) return RefreshKind::Hook;
       if (PEC586Hack) return RefreshKind::PEC586;
       if (QTAIHack) return RefreshKind::QTAI;
       return RefreshKind::Normal;
   }
   ```
3. `RefreshLine` 入口一次 select，循环内零 mapper 分支。

**风险**:
- **正确性**：6 种 `RefreshKind` 必须与原宏分支 1:1 对应；**全 mapper 矩阵单测**。
- **I-cache**：4 个独立 TU 使 I-cache 占用更稳定（只载入当前 kind 的代码），但总代码量略增（~10KB）。
- **回滚**：dispatcher 改回 macro 分支，单 PR revert。

**验证步骤**:
1. 编译：`nm` 验证 4 个新 TU 符号导出
2. 单测：mapper 矩阵（NROM / MMC3 / MMC5 / VRC5 / VRC6 / Mapper 4 / Mapper 19 / Mapper 69 / Mapper 85）全部输出 byte-for-byte 一致
3. 功能：5 个手动场景 + 上述 9 个 mapper 各 1 款代表性游戏
4. Profile：mapper 切换时 I-cache miss 抖动下降

**PR 标题**: `refactor(ppu): split RefreshLine into per-kind TU for I-cache stability (1-3%)`

**关联**: 紧随 P0-3 提交（必须 P0-3 先合入，模板才能被 dispatcher 调用）。

---

#### P0-5【VECTOR-1】AVX2 精灵 8-pixel decode（runtime gated）

**文件**: `src/ppu_rendering.cpp:880-955`、`src/cpu_detect_x86.h`（新增 CPUID 检测头）

**背景**: P0-1 消除分支后，仍有 8 次 `READPAL` 调用 + 8 次 store。AVX2 可用 `_mm256_i32gather_epi32` 一次性 gather 8 个 palette 值。

**修复方案**:
1. 检测 CPUID `__AVX2__`：
   ```cpp
   // cpu_detect_x86.h
   namespace fceu11::simd {
   bool have_avx2() noexcept;
   extern const bool kHaveAVX2;
   }
   ```
2. 在 `RefreshSprites` 入口选择路径：
   ```cpp
   if (fceu11::simd::kHaveAVX2) [[likely]] {
       sprite_decode_avx2(C, &kSpriteLUT_AVX2[atr_lo][ca]);
   } else {
       sprite_decode_scalar(C, &kSpriteLUT[atr_lo][ca]);
   }
   ```
3. AVX2 实现：
   ```cpp
   __m256i indices = _mm256_set_epi32(0,0,0,0, pixdata_7, ..., pixdata_0); // 8 个 4-bit chunk
   __m256i pal = _mm256_i32gather_epi32(palette_base, indices, 1);
   _mm256_storeu_si256((__m256i*)C, _mm256_or_si256(pal, mask));
   ```
4. 运行时检测失败时 fallback 到 P0-1 的 LUT 路径，**保证非 AVX2 CPU 仍能工作**。

**风险**:
- **可移植性**：AVX2 仅 x86-64；ARM / 老 x86 自动 fallback。runtime dispatch gate 必须正确。
- **正确性**：gather index 顺序、H_FLIP / V_FLIP 排列、SP_BACK mask 与 P0-1 LUT 完全一致。
- **MSVC / Clang / GCC 三家编译**：`_mm256_*` 头包含路径略有差异；用 `<immintrin.h>` 统一。

**验证步骤**:
1. 编译三平台 + 三编译器（MSVC 19.40 / Clang 17 / GCC 13）
2. 单测：CPUID 检测返回正确（手动 mock）；AVX2 路径与 scalar 路径输出 byte-for-byte 一致
3. ASan / UBSan 通过；**额外**：`valgrind --tool=memcheck` 在无 AVX2 机器上验证 fallback 路径
4. 功能：5 个手动场景，AVX2 路径帧时间下降 ≥ 4%

**PR 标题**: `perf(ppu): AVX2 8-pixel sprite decode with runtime dispatch (4-6%)`

**关联**: 必须在 P0-1 后合入（依赖 LUT 数据结构）。

---

### Phase B — P1 微结构

#### P1-1【DS-3】`BGData` 中 `ppu1[]` 拆 SoA

**文件**: `src/ppu_rendering.cpp:1153-1238`（`BGData::Record` 定义 + `Read()`）、`src/ppu_rendering.cpp:1241`（`bgdata.main[34]` 实例化）

**背景**: `BGData::Record` 是 14 字节 + padding；`ppu1[8]` 与 `nt / at / pt[2] / qtnt / pecnt` 混排，外层 32 次循环访问相邻 record 时 `ppu1[0..7]` 与 `pt[0..1]` 跨 cache line 边界。

**修复方案**:
```cpp
struct alignas(64) BGDataSoA {
    // 主结构：14 字节，去 ppu1
    struct Record {
        uint8_t nt, pecnt, at, pt[2], qtnt;
        uint8_t _pad[8];  // 对齐填充，原 ppu1 占位
    } main[34];
    // 拆分出去的 SoA：8 字节（每 record 8 像素）
    uint8_t ppu1[34][8] alignas(64);
};
```
`Read()` 内 `ppu1[i] = PPU[1]` 改为 `bgdata.ppu1[xt+2][xp] = PPU[1]`；调用方读 `bgdata.ppu1[xt+2][xp]` 同步替换。

**风险**: 单 PR 全局替换所有 `bgdata.main[*].ppu1[*]` 引用；grep 验证无遗漏。

**验证步骤**: 编译三平台；功能测试与 P0-1 一致；profile 验证 cache miss 下降。

**PR 标题**: `perf(ppu): split BGData::ppu1[8] into SoA for cache locality (2-4%)`

---

#### P1-2【MASK-1】`pal_mask` 入口 hoist

**文件**: `src/ppu_rendering.cpp:149`（`READPAL` 宏）、`src/ppu_rendering.cpp:311, 427, 442, 890-955`（调用点）

**当前代码**（节选 ppu_rendering.cpp 宏）：
```cpp
#define READPAL(ofs) \
    (PALRAM[(ofs) & 0x1F] & (GRAYSCALE ? 0x30 : 0xFF))
```

**修复方案**:
```cpp
// RefreshLine 入口（以及 RefreshSprites 入口）
const uint8_t pal_mask = (PPU[1] & 0x01) ? 0x30 : 0xFF;
// 替换所有宏调用：
#define READPAL(ofs) (PALRAM[(ofs) & 0x1F] & pal_mask)
```
对 `RefreshSprites`，因 4 路 SP_BACK / H_FLIP 各独立，可让 LUT 一次性按 `pal_mask` 二选一（见 P0-1 LUT 设计）。

**风险**: `GRAYSCALE` 在 mid-frame 内切换（如 FF1 "polygon" effect）— 当前实现按 PPU[1] 实时算。**新方案在 RefreshLine / RefreshSprites 入口固定，可能在帧中段切换时少几个像素**。

**缓解**: 保留 `RefreshLine` / `RefreshSprites` 入口处检测 `PPU[1] & 0x01` 在循环内是否变化（mapper 写 PPU[1] 触发）；若变化则回到原宏。**单测必须覆盖 mid-frame GRAYSCALE toggle**。

**验证步骤**: FF1 polygon effect（灰度模式切换）手动测试；5 个场景 + ASan/UBSan 通过。

**PR 标题**: `perf(ppu): hoist pal_mask computation out of hot loop (1-3%)`

---

#### P1-3【MICRO-4】`cycle` 取模改为 wrap-around

**文件**: `src/ppu_rendering.cpp:1146`

**当前代码**:
```cpp
void runppu(int x) {
    ppur.status.cycle = (ppur.status.cycle + x) % ppur.status.end_cycle;
    // ...
}
```

**修复方案**:
```cpp
void runppu(int x) {
    int c = ppur.status.cycle + x;
    if (c >= ppur.status.end_cycle) c -= ppur.status.end_cycle;
    ppur.status.cycle = c;
    // ...
}
```
**预期**: `x=1, end_cycle=341`，每帧调用 ~89 000 次 `DIV` → ~89 000 次高度可预测 `cmp+jmp`，预测率接近 100%（341 次中只有 1 次为 true）。

**风险**: 低。`end_cycle` 始终 = 341（`kLineTime`）常量，但保留通用性。

**验证步骤**: 5 个手动场景帧时间；与 P2-4 配套（避免双重 modulo）。

**PR 标题**: `perf(ppu): replace cycle modulo with wrap-around branch (1-3%)`

---

#### P1-4【INLINE-1】`runppu1_inline` 显式内联

**文件**: `src/ppu_rendering.cpp:1145-1151`

**修复方案**:
```cpp
[[gnu::always_inline]] inline void runppu1_inline() noexcept {
    int c = ppur.status.cycle + 1;
    if (c >= ppur.status.end_cycle) c -= ppur.status.end_cycle;
    ppur.status.cycle = c;
    if (!new_ppu_reset) X6502_RunDebug(g_cpu, 1);
}
// 原 runppu 改为单 x=1 包装；多 x 调用另起 runppu_batch
```

**风险**: 显式 `always_inline` 抑制编译器决策；体积膨胀 <2KB，可接受。

**验证步骤**: 编译产出大小（`size --totals`）+ 5 个场景。

**PR 标题**: `perf(ppu): always_inline runppu1 for hot BGData::Read (micro)`

---

#### P1-5【INLINE-2】`PPU_hook` 入口去虚化

**文件**: `src/pputile.inc:56, 102, 140`（3 处 `PPU_hook(vadr)`）

**修复方案**:
```cpp
// pputile.inc 改：
#ifdef PPUT_HOOK
    if (PPU_hook) [[unlikely]] {
        PPU_hook(0x2000 | (RefreshAddr & 0xfff));
    }
#endif
```
更好的做法：P0-4 dispatcher 已在入口按 `kind == Hook` 选路径，**hook 路径才进入此宏**，非 hook 路径完全跳过此判断。

**风险**: 仅有 hook 的极少数场景（debugger 内存观察、代码搜索插件）受影响。**debugger 测试矩阵必须覆盖**。

**验证步骤**: 启用 debugger + memory search；与未启用对比帧时间。

**PR 标题**: `perf(ppu): de-virtualize PPU_hook in non-hooked paths (micro)`

---

#### P1-6【MAP-1】`RefreshKind` 枚举分发

**文件**: `src/ppu_rendering.cpp:340-413`（dispatcher）

**修复方案**: 与 P0-4 共用 dispatcher（直接合并实现）。本 PR 主要负责**清理** `MMC5Hack` / `PEC586Hack` / `QTAIHack` / `PPU_hook` 4 个全局变量在循环内的重读，把判定上提到入口一次。

**风险**: 同 P0-4。

**验证步骤**: 同 P0-4。

**PR 标题**: `perf(ppu): hoist RefreshKind selection out of inner loop (2-5%)`

**关联**: 与 P0-4 合并或紧随。

---

#### P1-7【MAP-4】`scanline_ref` 改 value-return + 寄存器缓存

**文件**: `src/ppu.h` (`Cpu::scanline_ref()`)、`src/ppu_rendering.cpp:1091-1109`

**当前代码**:
```cpp
int& Cpu::scanline_ref() noexcept { return scanline_; }
// 调用点:
for (g_cpu.scanline_ref() = 0; g_cpu.scanline_ref() < totalscanlines; ) {
    // ...
}
```

**修复方案**:
```cpp
// 改 value-return
int Cpu::scanline() const noexcept { return scanline_; }
int& Cpu::scanline_ref() noexcept { return scanline_; }  // 保留向后兼容
// 新增 setter 显式:
void Cpu::set_scanline(int v) noexcept { scanline_ = v; }
// 调用点:
int sl = 0;
for (; sl < totalscanlines; ) {
    // ... DoLine 内 set_scanline(++sl) 显式
}
```

**风险**: DoLine 多处 `g_cpu.scanline_ref()` 调用须全替换；grep 验证。

**验证步骤**: 5 个手动场景帧时间。

**PR 标题**: `perf(ppu): make scanline_ref value-return to enable register caching (micro)`

---

### Phase C — P2 微观

#### P2-1【ALIAS-1】`FCEU_dwmemset` 重写 + AVX2 `fill32`

**文件**: `src/utils/memory.h:30`、`src/utils/simd_fill.h`（新增）

**当前代码**（节选 utils/memory.h:30）：
```cpp
#define FCEU_dwmemset(d,c,n) {int _x; for(_x=n-4;_x>=0;_x-=4) *(uint32 *)&(d)[_x]=c;}
```

**修复方案**:
```cpp
// utils/memory.h
inline void FCEU_dwmemset(void* dst, uint32_t pattern, size_t bytes) noexcept {
    uint32_t* p = reinterpret_cast<uint32_t*>(dst);
    for (size_t i = 0; i < bytes; i += 4) p[i >> 2] = pattern;
}

// utils/simd_fill.h
#ifdef __AVX2__
inline void fill32_avx2(void* dst, uint8_t pattern, size_t bytes) noexcept {
    __m256i v = _mm256_set1_epi8(pattern);
    uint8_t* p = static_cast<uint8_t*>(dst);
    for (size_t i = 0; i < bytes; i += 32) _mm256_storeu_si256((__m256i*)(p+i), v);
}
#endif
// 宏定义替换为内联函数:
#define FCEU_dwmemset(d, p, n) fceu11::FCEU_dwmemset((d), (p), (n))
#define FCEU_memset32(d, p, n) fceu11::fill32_avx2((d), (p), (n))
```

**风险**: `FCEU_dwmemset` 严格别名 UB 修复需全工程 grep 确认无 `*(uint32*)&byte_array[...]` 残留；AVX2 路径要求 runtime 检测。

**验证步骤**: ASan/UBSan（严格别名）、帧时间降幅 ≥ 3%。

**PR 标题**: `perf(ppu): replace FCEU_dwmemset macro with type-safe AVX2 fill (3-5%)`

---

#### P2-2【DS-2】`PALRAM[0/4/8/0xC]` 4 字节合并读写

**文件**: `src/ppu_rendering.cpp:332-335`（`|= 64`）、`419-422`（`&= 63`）

**修复方案**:
```cpp
// 替换 4 次单字节写为 1 次 4 字节写
uint32_t pal;
std::memcpy(&pal, &PALRAM[0], 4);
const uint32_t saved = pal;
pal |= 0x40404040;
std::memcpy(&PALRAM[0], &pal, 4);
// RefreshLine 末尾
std::memcpy(&PALRAM[0], &saved, 4);  // 或直接 & 0x3F3F3F3F
```

**风险**: 字节序假设（小端）；PALRAM 是 uint8[]，memcpy 安全。

**验证步骤**: 5 个手动场景帧时间；perf `perf stat -e L1-dcache-store-misses`。

**PR 标题**: `perf(ppu): batch PALRAM[0/4/8/C] writes into 32-bit store (micro)`

---

#### P2-3【DS-4】`SPRBUF` 类型改 `SPRB[]`

**文件**: `src/ppu.h:90`（`SPRBUF` 声明）、`src/ppu_rendering.cpp:756-758, 821-823`（memcpy 写入）

**修复方案**:
```cpp
// ppu.h
struct alignas(4) SPRB { uint8_t y, no, atr, x; };   // 4 字节 packed
alignas(64) SPRB SPRBUF[64];   // 替代 uint8_t SPRBUF[0x100]

// ppu_rendering.cpp
// 原：memcpy(&SPRBUF[ns<<2], &tmp, 4);
// 新：
SPRBUF[ns] = SPRB{ .y = ..., .no = ..., .atr = ..., .x = ... };
// 或对 packed 结构直接 store:
SPRBUF[ns] = *reinterpret_cast<SPRB*>(&tmp);   // 类型明确，编译器可生成 movdqu
```

**风险**: `SPRB` 内存布局须与原 `uint8[4]` 字节级一致（packed 4 字节，无 padding）；`sizeof(SPRB) == 4` 静态断言。

**验证步骤**: 5 个场景；静态断言；FFI 兼容（v1.14 引入的 `fceux11_core_types.h` 不引用 SPRB，OK）。

**PR 标题**: `perf(ppu): type SPRBUF as SPRB[64] to drop memcpy (1-2%)`

---

#### P2-4【MICRO-5】`runppu` 移除冗余 modulo

**文件**: `src/ppu_rendering.cpp:1146`、`1159-1238`（`BGData::Record::Read` 8 次 `runppu(1)`）

**修复方案**:
- 若 P1-3 已合入：`runppu(x)` 内做 wrap-around，无 modulo。
- `BGData::Record::Read` 内 8 次 `runppu(1)` 各自 wrap-around，**消除双重 modulo**（之前是 8 次 %end_cycle + caller 又 %）。
- 验证 `runppu(delay)` 多次大值仍正确（如 P2-5 启用）。

**风险**: caller 可能仍依赖 modulo 行为；grep `ppur.status.cycle =` 验证所有赋值走 wrap-around。

**验证步骤**: 帧时间、5 个场景。

**PR 标题**: `perf(ppu): eliminate redundant cycle modulo in BGData::Read (1-2%)`

---

#### P2-5【MICRO-7】`runppu` 重批（恢复 batched 调用）

**文件**: `src/ppu_rendering.cpp:1302-1310`（VBlank 区双重 for 循环）

**当前代码**（节选 ppu_rendering.cpp:1304-1310）：
```cpp
//formerly: runppu(20 * (kLineTime) - delay);
for(int S=0;S<sltodo;S++)
{
    for(int dot=(S==0?delay:0);dot<kLineTime;dot++)
        runppu(1);
    ppur.status.sl++;
}
```

**修复方案**:
1. 重新审视 timing correctness（P1-3 之后 `runppu(delay)` 的 wrap-around 已正确）。
2. 恢复 batched：
   ```cpp
   // S=0: runppu(kLineTime - delay)
   runppu(kLineTime - delay);
   ppur.status.sl++;
   // S=1..sltodo-1: runppu(kLineTime)
   for (int S = 1; S < sltodo; S++) {
       runppu(kLineTime);
       ppur.status.sl++;
   }
   ```
3. **先开 PR with timing instrumentation**（在 `runppu` 入口插 timestamp），对比改前/改后 PPU 时序，确认无回归后再合。

**风险**: VBlank 区 timing 微调可能影响特定 mapper（如 MMC3 IRQ 在 VBlank 内的精确位置）；先单 PR 提 instrumentation，第二个 PR 改逻辑。

**验证步骤**: mapper 矩阵测试（含 MMC3 IRQ）；帧时间降幅 ≥ 1%。

**PR 标题**: `perf(ppu): restore batched runppu() in VBlank loop (1-2%)`

**关联**: 拆分两个 PR：① P2-5a 加 instrumentation；② P2-5b 改逻辑。

---

#### P2-6【DS-1】`pshift[2]` 本地化（去除 `uint32&` 引用）

**文件**: `src/ppu_rendering.cpp:275-276`

**修复方案**:
```cpp
// 替换:
uint32 (&pshift)[2] = fceu11::g_ppu.bg_latch();
uint32 &atlatch     = fceu11::g_ppu.bg_latch_h();
// 改为:
uint32_t pshift_local[2] = { g_ppu.bg_latch()[0], g_ppu.bg_latch()[1] };
uint32_t atlatch_local   = g_ppu.bg_latch_h();
// pputile.inc 内引用替换为 pshift_local / atlatch_local
// 循环结束写回:
g_ppu.bg_latch()[0] = pshift_local[0];
g_ppu.bg_latch()[1] = pshift_local[1];
g_ppu.bg_latch_h()  = atlatch_local;
```

**风险**: `pshift` 在 `pputile.inc` 内多处取址（`&pshift[0] <<= 8`）；新方案必须把所有 `pshift[...]` 替换为 `pshift_local[...]`。

**验证步骤**: 5 个场景；编译 warning `-Waddress-of-packed-member`；ASan。

**PR 标题**: `perf(ppu): localize pshift[2] to enable register allocation (micro)`

---

### Phase D — P3 清理

#### P3-1【DS-5】`bitrevlut` → `constexpr std::array`

**文件**: `src/ppu_rendering.cpp:628-655`

**修复方案**:
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

**风险**: 移除 `BITREVLUT<T,BITS>` 模板的全局 `new[]`，需要确保所有 caller（`oam[4]` / `oam[5]` 反向）都迁移。

**验证步骤**: 5 个场景；ASan 检测无泄漏。

**PR 标题**: `chore(ppu): replace bitrevlut with constexpr std::array (cleanup)`

---

#### P3-2【MICRO-1】`ppudead` 期间 `XBuf` memset 单次

**文件**: `src/ppu_rendering.cpp:999`

**修复方案**:
```cpp
static bool s_ppudead_cleared = false;
if (ppudead) {
    if (!s_ppudead_cleared) {
        memset(XBuf, 0x80, 256 * 240);
        s_ppudead_cleared = true;
    }
    // ... continue 渲染（全 0x80）
    return;
}
s_ppudead_cleared = false;  // ppudead 退出时复位
```

**风险**: `ppudead` 期间 XBuf 本来就全 0x80（无渲染），但 Game Genie / 调试器仍可能修改 XBuf 局部像素。**保持每帧清零**（仅在 ppudead 持续期间跳过）需保守权衡。

**替代方案**: 仅在 ppudead **首次进入** 时清，后续帧复用（前提：调试器不在 ppudead 改 XBuf）。**与调试团队确认**。

**验证步骤**: Game Genie 路径；5 个场景。

**PR 标题**: `perf(ppu): skip XBuf memset after first ppudead frame (micro)`

---

#### P3-3【MICRO-3】`InputScanlineHook` `[[unlikely]]` 守卫

**文件**: `src/ppu_rendering.cpp:326, 464`

**修复方案**:
```cpp
if (InputScanlineHook) [[unlikely]] {
    InputScanlineHook(Plinef, spork ? sprlinebuf : 0, linestartts, lasttile * 8 - 16);
}
```

**风险**: 无（仅代码风格）；编译器可能已自动优化。

**验证步骤**: TAS 录制 / 回放场景；帧时间。

**PR 标题**: `perf(ppu): mark InputScanlineHook as unlikely branch (cleanup)`

---

#### P3-4【MAP-2】`norecurse` 守卫移至 hook-only 内部

**文件**: `src/ppu_rendering.cpp:287, 380, 394`

**修复方案**:
```cpp
// RefreshLine 入口去掉:
static int norecurse = 0;
if (norecurse) return;   // 删除
// 改为在 RefreshLine_Hooked 版本内检查（仅 hook 路径使用）
```

**风险**: 99% 路径不查 norecurse，收益微小；需确认 `norecurse` 仅 hook 路径设置。

**验证步骤**: hook path（debugger 内存观察）；5 个场景。

**PR 标题**: `perf(ppu): move norecurse guard to hook-only path (cleanup)`

---

#### P3-5【MAP-3】`vnapage` 缓存复用与预取 review

**文件**: `src/pputile.inc:47`（`vnapage[(RefreshAddr >> 10) & 3]`）

**修复方案**:
- 选项 A：保持现状，依赖编译器优化（实测多已 CSE）。
- 选项 B：mapper 设置时填充紧凑 `nt_ptrs[4]`，与当前 `vnapage` 共享语义。
- 选项 C：在 `RefreshLine` 入口 `_mm_prefetch` 4 个 nametable（仅当 mapper 切换时）。

**修复方向**: 先 A/B 二选一（A 优先，最小侵入）；C 仅作为未来方向。

**验证步骤**: 帧时间、5 个场景。

**PR 标题**: `chore(ppu): review vnapage caching for cache miss reduction (cleanup)`

---

## 十一、测试与验证策略

### 11.1 通用基线（每个 PR 必跑）

1. **正确性回归**: hotfix1 的所有 PPU 测试 + SMB1 / SMB3（精灵密集）/ Contra（横向滚屏）/ Batman（多 sprite 重叠）/ Kirby（OAM 边界）/ Micro Machines（窗口分割）6 个手动测试场景必须 100% pass
2. **性能基线**:
   - 用 `scripts/perf_ppu.cpp`（v1.14 已引入）跑 60 秒、记录平均帧时间
   - hotfix1 基线：~7.5 ms/frame；hotfix2 目标：单线程 PPU 主导负载降到 ≤ 4.5 ms/frame（40-50% 降幅）
3. **静态分析**:
   - `cppcheck --enable=all,style` 无新增警告
   - `clang-tidy` 检查 `ppu_rendering.cpp`（默认开）
4. **动态分析**（按 PR 选）：
   - AddressSanitizer：所有 PR
   - UndefinedBehaviorSanitizer：所有 PR（特别是 P2-1 严格别名修复）
   - ThreadSanitizer：仅在跨线程字段变更时（P1-7 不需要，但需 NES 主线程 / GUI 线程 smoke）
5. **Profile 工具**:
   - `tracy`（跨平台，0.10+）：嵌入 `BUILD_TRACY=1` 构建
   - Linux：`perf record -g -e cycles`
   - Windows：`Very Sleepy` 或 `Superluminal`
6. **FFI 兼容**: 不得改变 `extern "C"` ABI。`fceux11_core_types.h`（v1.14 引入）依赖字节级布局，P2-3 须静态断言。
7. **存档兼容**: 用 v1.15 LTS / v1.15.1 LTS hotfix1 生成的存档能被 hotfix2 新代码加载；新代码生成的存档能被旧版读取（标签不变）。
8. **存档兼容粒度**: P0-1 / P0-2 / P0-3 / P0-4 影响 SFORMAT 时，标签必须保留（与 hotfix1 一致）。

### 11.2 per-Phase 重点

| Phase | 重点验证 | 必跑 mapper 矩阵 | 必跑手动场景 | 性能目标 |
|-------|---------|----------------|-------------|---------|
| **A**（算法） | LUT byte-for-byte 一致 / X-bucket 等价 | NROM + MMC3 + MMC5 + VRC5 + VRC6 + Mapper 19/69/85 | 6 场景 | 帧时间下降 ≥ 18% |
| **B**（微结构） | mid-frame GRAYSCALE toggle / SoA 边界 / scanline_ref 同步 | 同上 + Mapper 4 | 6 场景 + FF1 polygon | 累计 ≥ 22% |
| **C**（微观） | 严格别名 / VBlank timing / 双 modulo 消除 | 同上 + Mapper 1 (MMC1 IRQ) | 6 场景 | 累计 ≥ 24% |
| **D**（清理） | constexpr 初始化 / ppudead 边界 | 同上 | 6 场景 + Game Genie | 清理类不影响帧时间 |

### 11.3 性能门槛（合并门槛）

- 单 PR 不得**回退**已有 PR 的性能；新 PR 测得帧时间高于 hotfix1 基线即视为失败，必须优化或回滚
- 性能测试用 3 次测量取中位数，单次 < ±5% 噪声
- "NTSC 帧预算 headroom" 必须 ≥ 30%（即帧时间 ≤ 11.6 ms）

---

## 十二、不在本 PLAN 范围

- mapper dispatch (`src/boards/*.cpp`) 优化：单次成本低，开支不足以触发算法级重构
- APU 优化：v1.14 Anvil 已实施 LTO/PGO，本轮不重复
- CPU 解释器（`x6502.cpp`）：dynarec 重写是大工程，不属 hotfix2 范畴（应进 v1.16 计划）
- 视频后处理（`drivers/sdl/`、`video.cpp`）：上层，依赖外部 lib
- Rust FFI 侧 `fceux11_rust.h` 变更：本轮不修改（除非 hotfix1 P1-3/4/6 等已涉及）
- 调试器 UX 改造：性能优化不触及 debugger 路径正确性

---

## 十三、执行时间表

| 阶段 | PR 范围 | 预计 PR 数 | 周期 | 关键里程碑 |
|------|--------|-----------|------|-----------|
| **Phase 1 — P0 算法核心** | P0-1, P0-2, P0-3, P0-4, P0-5 | 5 | 2.5 周 | 单测 LUT 等价 + mapper 矩阵全过 + 帧时间降幅 ≥ 18% |
| **Phase 2 — P1 微结构** | P1-1 ~ P1-7 | 7 | 2 周 | SoA + dispatcher + wrap-around 落地，FF1 polygon / MMC3 IRQ 不回归 |
| **Phase 3 — P2 微观** | P2-1 ~ P2-6 | 6 | 1.5 周 | 严格别名 UB 全部清除，AVX2 runtime gate 验证 |
| **Phase 4 — P3 清理** | P3-1 ~ P3-5 | 5 | 1 周 | constexpr LUT 化 + 守卫收紧 + 文档同步 |
| **总计** | — | **23 PR** | **7 周** | 帧时间 ≤ 4.5 ms/frame；FFI / 存档 / mapper 矩阵 100% 兼容 |

> **预留缓冲**: 实际周期可能因 P0-1 LUT 单测、P0-5 AVX2 三编译器验证等节点延长 0.5-1 周；hotfix2 发布目标不晚于 2026-08-31。

---

## 十四、风险矩阵

### 14.1 高风险 PR（需双 reviewer + 专项回归）

| PR | 主要风险 | 缓解措施 |
|----|---------|---------|
| P0-1 RefreshSprites LUT | 8 张 LUT 漏 1 张 = silent corruption | 全 256×256×8 单测覆盖；与原宏代码 byte-for-byte 对比 |
| P0-3 pputile.inc template | 模板报错信息难读；4 特化语义偏离 | 保留 pputile.inc 作为单测 golden；4 mapper 矩阵回归 |
| P0-5 AVX2 dispatch | runtime gate 失效 / MSVC 头路径差异 | 三编译器（MSVC 19.40 / Clang 17 / GCC 13）各跑一遍 |
| P2-5 runppu 重批 | VBlank timing 微调影响 MMC3 IRQ | 拆 P2-5a (instrumentation) + P2-5b (逻辑)；mapper 矩阵 timing 对比 |

### 14.2 中风险 PR

| PR | 主要风险 | 缓解措施 |
|----|---------|---------|
| P0-2 X-bucket | V_FLIP 路径误覆盖 | V_FLIP 单测分支显式 |
| P0-4 dispatcher 提取 | RefreshKind 6 路遗漏 | mapper 矩阵（含 PEC586 / QTAI hack）单测 |
| P1-2 pal_mask hoist | mid-frame GRAYSCALE toggle | FF1 polygon 单测；hoist 失败回退宏 |
| P1-6 RefreshKind | 与 P0-4 合并冲突 | 紧随 P0-4 提交或合并 PR |
| P2-1 FCEU_dwmemset | 严格别名 UB 残留 | grep 验证；Ubsan 必跑 |
| P2-3 SPRB 类型化 | SPRB 内存布局 | `static_assert(sizeof(SPRB) == 4)` |

### 14.3 低风险 PR（单 reviewer + 标准回归）

P0-4（与 P0-3 合并时）、P1-1、P1-3、P1-4、P1-5、P1-7、P2-2、P2-4、P2-6、P3-1 ~ P3-5。

### 14.4 跨 PR 风险（须全局回归）

- **FFI ABI 兼容**: P2-3 涉及 struct 布局，须 `fceux11_core_types.h` 同步验证
- **存档兼容**: P0-3 模板 + P0-4 dispatcher 不影响 SFORMAT 标签；用 v1.15.1 存档 smoke
- **mapper 矩阵**: 23 PR 全合入后必须 9 款 mapper 全过；任一回归即 block release
- **三平台编译**: 每周一次完整三平台 `cmake --build` 验证，避免最后一周集中返工

---

## 十五、PR 验收清单（每个 PR 必填）

每个 PR 必须包含以下条目，缺一项即视为不合格：

- [ ] 修改文件清单（含新增 / 删除 / 重命名）
- [ ] **§十一.1** 全部基线通过（编译 / 静态 / 动态 / 功能 / FFI / 存档）
- [ ] per-Phase 重点验证（§十一.2 对应行）
- [ ] 单测覆盖（新增 / 修改文件 ≥ 80% 行覆盖；hotfix1 覆盖率基线不下降）
- [ ] AddressSanitizer 输出无错
- [ ] UndefinedBehaviorSanitizer 输出无错（涉及 alias / 取模 PR）
- [ ] ThreadSanitizer（如涉及跨线程字段）
- [ ] 三平台编译通过（Windows MSVC / Linux GCC / macOS Clang 或 Apple Clang）
- [ ] `scripts/perf_ppu.cpp` 60 秒测量；帧时间 vs hotfix1 基线
- [ ] `tracy` 或 `perf` profile 截图（标出 hot path 位置变化）
- [ ] 性能门槛：本次 PR 帧时间不得高于上一 PR 状态
- [ ] CHANGELOG 条目（指向本 PR）
- [ ] PR 描述引用本 PLAN 的对应 PR ID（如 `ref: P0-1`）
- [ ] 至少 1 个 approve + CI 全过

### 15.1 每个 Phase 完成时额外验收

- [ ] Phase 全量回归通过（所有 PR 合并后跑全套 §十一.1 + §十一.2）
- [ ] mapper 矩阵 9 款全过
- [ ] 帧时间累计降幅 ≥ §十一.2 对应行
- [ ] `CHANGELOG.md` Phase 段更新
- [ ] `readme.md` "1.15(hotfix2)" 版本号同步
- [ ] Phase 完成报告（新增 `docs/history/v1.15_hotfix2_phase_*.md`）

---

## 十六、PR 实施纪律

### 16.1 顺序约束

- **P0-3 必须先于 P0-4**：模板实例化后才能被 dispatcher 调用
- **P0-5 必须后于 P0-1**：依赖 LUT 数据结构
- **P1-3 必须先于 P2-4**：wrap-around 落地后才能消除双重 modulo
- **P1-6 必须紧随 P0-4**：dispatcher 已就位才能 hoist 选择
- **P3-* 必须最后**：清理类不在算法路径上受其他 PR 干扰

### 16.2 分支策略

- 每个 PR 一个独立 feature 分支（`hotfix2/p0-1-sprite-lut`、`hotfix2/p1-3-cycle-wrap` 等）
- 合并前必须 `git rebase hotfix2` 避免 merge commit
- 冲突解决须由 PR 作者本人执行；reviewer 仅给建议

### 16.3 Review 节奏

- 高风险 PR：双 reviewer（一位算法 / 一位 mapper），review 时长 ≥ 24h
- 中风险 PR：单 reviewer，review 时长 ≥ 4h
- 低风险 PR：单 reviewer，CI 全过即合并
- 单 reviewer 不能 review 自己的 PR

### 16.4 回滚预案

- 任一 PR 引入性能回退 > 5% 立即 revert
- 任一 PR 引入 mapper 回归立即 revert
- Phase 边界（如 Phase A → B 切换）必须先 `git tag hotfix2-phase-a-done`，回滚时整 phase 撤回
- 每个 PR 必须可独立 revert（不允许跨 PR 强耦合）

### 16.5 性能预算使用规则

- 单个 PR 申请 "性能预算"（即帧时间降幅目标）≤ 10%；超额完成不累积
- P0-5（AVX2）等向量化 PR 可一次性申请最多 6%（跨 ISA 复杂）
- 性能预算累计达成进度须在 Phase 完成报告里汇总

---

**PLAN END** — hotfix2 算法级 REVIEW 与性能优化方案 v2（2026-07-15）

*本版较 v1（490797d）新增：§九按 P0/P1/P2/P3 编号的 23 PR 清单、§十逐 PR 文件定位 / 当前代码 / 修复方案 / 风险 / 验证 / PR 标题、§十一按 Phase 区分重点验证、§十三时间表、§十四风险矩阵、§十五 PR 验收清单、§十六实施纪律。v1 的 §〇-§八 review 章节与 §十二 out-of-scope 保留作为背景。*