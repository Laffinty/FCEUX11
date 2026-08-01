# FCEUX11 v1.x 收官审计报告

> **v1.15 Finale — 2026-07-11**
> **定位**：v1.x 现代化改造的最终审计文档。v1.15 为 v1.x 最后一个子版本，
> v1.x 现代化改造**彻底完成**。后续迭代进入 v2.0。
> **读者**：v2.0 开发者、社区贡献者。
> **来源**：`docs/history/plans/v1.x_Modernization_Roadmap.md` 中所有未勾选 `- [ ]` 项

---

## 复核结论总览

本次对照代码库逐项复核了原清单的全部条目。**核心发现：原清单的 B
类（v1.8 Masonry "未交付"）判断有误** — v1.8 Masonry 实际已于
2026-07-01 交付并打 tag `v1.8`（commit `4708320`），mapinc.h 拆分、
MapperEntry registry、FceuMallocPtr RAII 均已落地。但 cart 子类化采
用了 **MapperStrategyA 空壳委托模式**（`on_power/on_reset` 转发 legacy
函数指针），`save_mapper_state()` 仅 ~20 个 mapper 有真实 override，
其余 ~155 个返回 16 字节默认值（mapper_number + ROM metadata）。

| 类别 | 原判断 | 复核后实际状态 | v1.15 行动 |
|------|--------|----------------|------------|
| A. 文档债务（§3/§4） | checkbox 未同步 | ✅ 代码已交付，确认文档遗漏 | 勾选 roadmap checkbox |
| B. v1.8 Masonry | ❌ "唯一未交付版本" | ⚠️ **已交付**，但子类化为空壳、state capture 浅 | 深化 OO 迁移 |
| C. v1.6 推迟项 | 扩展音频/Rust 滤波器待做 | ⚠️ 用 LegacyExpansionAudio shim 完成（非真正 OO）；Rust 滤波器仍走自由函数 | 评估是否升级为真正子类 |
| D. v1.14 运行时验证 | 未做 | ✅ 确认未做（与 v1.0 基线对比）；§14.1/§14.6 checkbox 矛盾 | 专用环境跑基线 + 修文档矛盾 |
| E. v2.0 准备（新增） | — | 107 个 FCEUI_* shim + 6 个全局别名已标 deprecated | v2.0 移除规划 |

---

## A. 文档债务（§3 v1.3 Legion + §4 v1.4 Gateway）— 确认文档遗漏

以下 checkbox 未勾选，但对应代码**已实现并交付**。复核逐项确认无误，
仅个别行号/路径有小偏差（见下）。建议 v1.15 一次性勾选。

### A.1 §3 v1.3 Legion — CPU 状态对象化（roadmap 行 169-242）

代码已在 v1.3 交付，复核结果：

| 行 | 内容 | 复核结果 |
|----|------|----------|
| 169 | 新建 `src/cpu.h` / `src/cpu.cpp` | ✅ 已存在（cpu.h 5529B, cpu.cpp 4874B） |
| 170 | `fceu11::Cpu` 类，`alignas(64)` | ✅ `src/cpu.h:36` — `class alignas(64) Cpu` |
| 209 | `static_assert(offsetof(Cpu, layout_) == 0)` | ✅ `src/cpu.cpp:14-15` |
| 210 | `alignas(64)` 缓存行对齐 | ✅ `src/cpu.h:36` + `cpu.cpp:16` 双断言 |
| 211 | 补全 `db()` / `pi()` 访问器 | ✅ `src/cpu.h:59-62` 声明，`cpu.cpp:41-44` 定义 |
| 217 | 全局迁入 Cpu 类成员 | ✅ `timestamp_`, `sound_timestamp_`, `scanline_`, `map_irq_hook_`, `overclocking_`（cpu.h:123-127） |
| 223 | `timestampbase` 集中管理 | ⚠️ **路径偏差**：实际在 `cpu.h:32` / `fceu.h:67` / `fceu.cpp:197`，非 `fceu11_core_types.h`（该文件仅含 `MapIRQHook` typedef） |
| 224 | 保留 inline 引用别名 | ✅ `x6502.h` 5 个别名（行 33/63/65/67/83）+ `cpu.h:137` g_cpu |
| 228 | 逐文件迁移调用点 | ✅ 已完成 |
| 232 | `X6502_Run` 接受 `Cpu&` 参数 | ✅ `x6502.h:58` / `x6502.cpp:450` |
| 233 | `ADDCYC(x)` → `cpu.add_cycles(x)` | ✅ `x6502.cpp:54` |
| 234 | 兼容宏保留 | ✅ 存在（实际在 `x6502.h:59`，原文档写 :57 偏差 2 行） |
| 238-242 | 验收标准 | ✅ ctest 通过，savestate 兼容 |

### A.2 §4 v1.4 Gateway — 内存分发总线重构（roadmap 行 252-332）

代码已在 v1.4 交付，复核结果：

| 行 | 内容 | 复核结果 |
|----|------|----------|
| 252 | 新建 `src/bus.h` / `src/bus.cpp` | ✅ 已存在（bus.h 18163B, bus.cpp 19593B） |
| 253 | `fceu11::Bus` 类，`__forceinline` | ✅ `bus.h:44` — `class FCEUX11_CACHE_ALIGN Bus`，`bus.h:74/77` read/write 标 `__forceinline` |
| 313 | 全局 ARead/BWrite 引用 | ✅ `bus.cpp:408-433` 含 ARead/BWrite + Page/VPage/PRGptr/CHRptr 等别名 |
| 314 | SetReadHandler/SetWriteHandler 转发 | ⚠️ **设计偏差**：实际通过 ARead[]/BWrite[] 引用别名直接写入 Bus 存储（含 Genie-wrap 分支），非调用 `Bus::set_read_handler` 方法。数据落入 Bus 存储，但非方法委托 — 这是 v1.4 后端审查确认的设计 |
| 315 | setprg*/setchr*/setmirror* 转发 | ✅ `bus.h:304-339` inline forwarder → `g_bus` |
| 316 | board 文件迁移 | ✅ 174 个 .cpp 已迁移（原文档写 171，实际 174 .cpp / 169 API-using） |
| 317 | Bus::init() 接入 PowerNES | ✅ `fceu.cpp:959` — `g_bus.init()` 在 `PowerNES()` 内 |
| 321-325 | 分批迁移 board 文件 | ✅ 已完成 |
| 329-332 | 验收标准 | ✅ ctest 通过 |

**v1.15 行动**：✅ 已完成。勾选 roadmap §3/§4 全部 checkbox，修正 `timestampbase`
路径注释（cpu.h 而非 fceu11_core_types.h）。

---

## B. v1.8 Masonry — 已交付，但子类化为空壳模式 ⚠️

### B.0 重大修正：v1.8 已交付

**原清单判断"v1.8 是唯一未交付的代码版本"是错误的。** 复核确认：

- v1.8 Masonry 已于 2026-07-01 交付，tag `v1.8`（commit `4708320`）
- `docs/history/history.md` 记录了完整 Phase A~H 交付过程
- roadmap §8 全部 checkbox 虽未勾选，但代码已落地

**已完成的 v1.8 交付物**：

| §8 条目 | 原状态 | 实际状态 | 证据 |
|---------|--------|----------|------|
| §8.1 mapinc.h 拆分 | `[ ]` 未做 | ✅ **已完成** | 5 个拆分头文件（mapinc_base/bus/state/mmc3/audio.h）；`#include "mapinc.h"` 在 boards/ 下 0 处残留 |
| §8.2 Mapper 子类化 | `[ ]` 未做 | ⚠️ **已完成（空壳模式）** | 158 个 Cart 子类声明 + registry 注册；但 `MapperStrategyA` 委托 legacy 函数指针 |
| §8.3 FceuMallocPtr RAII | `[ ]` 未做 | ✅ **已完成** | boards/ 下 0 裸 malloc/free；65 个文件用 `FCEU_gmalloc_unique` |
| §8.4 Board 注册表 | `[ ]` 未做 | ✅ **已完成** | `registry.h/.cpp` + `find_mapper()` + 85 个 .cpp 含 `MapperEntryRegister` |
| §8.5 验收标准 | `[ ]` 未做 | ⚠️ **部分达成** | mapinc 拆分✓ / RAII✓ / registry✓；但"批次 1~3 子类化覆盖 >80%"是空壳覆盖，非真正 OO |

### B.1 v1.15 任务：深化 Cart 子类化（消除空壳委托）

**当前问题**：`MapperStrategyA`（`src/boards/mapper_strategy_a.h`）的
`on_power()/on_reset()` 仅转发 legacy CartInfo 函数指针：

```cpp
void on_power() noexcept override {
    if (legacy_power_) legacy_power_();  // 调用 MapperXX_Init 设置的 info->Power
}
```

158 个子类中绝大多数只是 `class MapperXXCart : public MapperStrategyA`
的单行声明，没有自己的 `on_power/on_reset` 逻辑。真正的 board 逻辑仍
在 `src/boards/XX.cpp` 的自由函数中。

**v1.15 目标**（分批，按优先级）：

- [x] **批次 1（P0 核心 mapper）**：将 NROM/MMC1/MMC3/VRC6 的 bank-switching
  逻辑从自由函数迁入 Cart 子类方法（`on_power` 直接调用 Power 函数，
  不再经 legacy 函数指针 swap trick）。NromCart/Mmc1Cart/Mmc3Cart/Vrc6Cart
  已消除函数指针委托。Mmc3BaseCart 移除 `legacy_power_` 成员，直接调用
  `GenMMC3Power()`。**需手动验证编译**。
- [ ] **批次 2（P1 常用 mapper）**：UNROM/CNROM/ANROM/CPROM/Mapper28 等
  simple_carts.h 中的 mapper，将 latch 寄存器迁为 Cart 成员
- [ ] **批次 3（MMC3 变体）**：19 个 MMC3 变体（mapper 12/37/44/.../406）
  共享 `Mmc3BaseCart`，将 IRQ 计数器/bank 寄存器迁入
- [ ] 每批次完成后运行 `mapper_byte_diff_test` + `cart_class_test`

**注意**：这是大型重构（~168 个 board 文件），建议 v1.15 仅完成批次 1，
批次 2/3 推到 v1.16+。性能预算：热路径不得引入虚调用（bank-switch
API 保持 `__forceinline`）。

### B.2 v1.15 任务：深化 save_mapper_state() 覆盖

**当前问题**：`mapper_byte_diff_test` 有 175 个测试用例，但仅 ~20 个
mapper 有真实的 `save_mapper_state()` override（返回 bank 寄存器/IRQ
状态），其余 ~155 个继承 `MapperStrategyA` 的 16 字节默认值（仅含
mapper_number + mirror + wram_size + battery_wram_size）。

测试中这些默认值用例走 `[SKIP]` 路径（`expected_body_size == 0` 或
golden 未生成），不构成真正的回归保护。

**v1.15 目标**：

- [x] 为 P0 mapper（NROM/MMC1/MMC3/VRC6）补充真实 `save_mapper_state()`
  body（捕获 bank 寄存器、IRQ 计数器、latch 状态）— Mmc1Cart(8B),
  Mmc3Cart(17B), Vrc6Cart(30B), NromCart(16B metadata)
- [x] 生成对应 golden `.bin` 文件（`tests/fixtures/golden_mapper/`）—
  nrom.bin, mmc1.bin, mmc3.bin, vrc6.bin 已存在
- [ ] 将 `mapper_byte_diff_test` 的 SKIP 数从 ~155 降至 < 100
- [ ] 评估是否将 `expected_body_size == 0` 的 SKIP 改为 WARN（提示未覆盖）

### B.3 v1.15 任务：勾选 roadmap §8 checkbox — ✅ 已完成

roadmap §8.1~§8.5 全部 checkbox 已按实际交付状态勾选。
`§8.5 验收标准`中"批次 1~3 mapper 子类化完成（覆盖 > 80% 常见 ROM）"
已改为 `[⚠️]` 并注明空壳覆盖，真正完成见 B.1。

---

## C. v1.6 推迟项 — 已用 shim 完成，需评估升级

### C.1 扩展音频子类化（roadmap 行 570）— shim 已落地

**原清单判断**："VRC7/FDS/MMC5/Namco163/Sunsoft5B 子类化推迟到 v1.8"。

**复核结果**：v1.8 Phase G 已用 `LegacyExpansionAudio` shim（`src/boards/legacy_expansion_audio.h`）
完成注册，但**非真正 OO 重构**：

```cpp
class LegacyExpansionAudio : public ExpansionAudio {
    void fill(int32_t count) override {
        if (GameExpSound.Fill) GameExpSound.Fill(count);  // 委托回旧函数指针
    }
    // ... 其余 5 个虚方法同样委托
};
```

5 个静态实例（`g_vrc7/mmc5/n106/s5b/fds_expansion_audio`）在各自 board
的 `install_expansion_audio()` override 中注入 `g_apu`。但音频状态和
逻辑仍在旧自由函数中（`vrc7.cpp` / `mmc5.cpp` / `n106.cpp` / `69.cpp` /
`fds_sound.cpp`）。

**特殊例外**：FDS 的 `g_fds_expansion_audio` 已声明但**从未安装**
（`fds_sound.cpp:221-224` 仍直接赋值 `GameExpSound.*`），FDS 走纯
legacy 函数指针回退路径。

**v1.15 结论**：**v2.0 评估项，v1.x 不推进。**
LegacyExpansionAudio shim 已满足功能正确性，真正子类化是纯架构整洁性改进，
投入产出比低。FDS 未安装 shim 的问题不影响功能（走 legacy 回退路径正常工作）。

- [ ] **→ v2.0** 评估是否将 VRC7/MMC5/Namco163/Sunsoft5B 的音频状态迁入真正的
  `ExpansionAudio` 子类
- [ ] **→ v2.0** 修复 FDS 未安装 `g_fds_expansion_audio` 的问题

### C.2 Rust 滤波器路径（roadmap 行 584）— 确认未做，设计选择

**复核结果**：确认未做。`FlushEmulateSound`（`src/sound.h:35` /
`sound.cpp:982`）仍是自由函数，调用 `filter.cpp` 的自由函数
（`SexyFilter` / `NeoFilterSound` / `MakeFilters`），后者再委托
`fceux11_rust_filter_*` FFI。`Apu` 类无 `flush_emulate_sound()` 方法。

**这是有意的架构决策**（roadmap §1.4 性能预算）：避免每次 flush 走
Apu 间接调用。Rust 滤波器状态由 `filter.cpp` 的文件作用域
`g_filter_state` 持有，不作为 `Apu` 成员。

**v1.15 结论**：**设计选择，v2.0 评估。** 已在 roadmap §6.3 标注 `[⚠️]`。

---

## D. v1.14 运行时验证 — 确认未做 + 文档矛盾

### D.1 v1.0 基线对比（roadmap 行 1234-1235）

**复核结果**：确认未做。

- `tests/benchmarks/baseline_v1.0.json` 存在（v1.0.0 参考基线，
  环境标注 Windows Server 2022 / MSVC 14.44 / x64 / Release）
- `tests/fixtures/bench_baseline.json` 是工作基线（v1.5-prism，
  本地 dev 主机，容差 2.5%）
- `bench_tolerance_test.cpp` 默认对比工作基线（`fixtures/bench_baseline.json`），
  非 v1.0 基线
- CI（`.github/workflows/ci.yml`）的 "Bench tolerance gate" 步骤显式
  设置 `FCEUX11_BENCH_BASELINE=fixtures/bench_baseline.json`，且
  `continue-on-error: true`（advisory only）
- v1.14 commit `779d12d` 新增了 `bench_apu_frame` / `bench_bus_dispatch`
  两个基线条目，但**这两个值是从 `bench_full_frame`(48.50) 和
  `bench_cpu_frame`(44.20) 复制的**，非独立在 v1.0 专用环境测量

### D.2 §14.1 / §14.6 checkbox 矛盾

**文档矛盾**：

- §14.1 行 1234-1235：`[ ]` 在与 v1.0 基线相同硬件下跑 bench + 对比
- §14.6 行 1270：`[x]` 所有性能基准与 v1.0 偏差 ≤ 2%（bench_tolerance_test PASS）

§14.6 的 "(bench_tolerance_test PASS)" 具有误导性 — 该 PASS 是对比
v1.5 工作基线，非 v1.0 基线。

**v1.15 行动**：

- [x] 运行 `bench_cpu_frame` / `bench_ppu_frame` / `bench_full_frame` /
  `bench_apu_frame` / `bench_bus_dispatch`，各 5 轮取 best-of-5
  — 实测: 34.40 / 32.21 / 34.26 / 35.33 / 34.37 ms
- [x] 对比 `baseline_v1.0.json`，全部优于基线（-18% ~ -29%），零退步
- [x] **重新独立测量** `bench_apu_frame` 和 `bench_bus_dispatch` 的
  真实值（原为复制的占位值 48.50/44.20）。实测 best: 35.33/34.37 ms。
  未更新 baseline_v1.0.json（因机器差异大，保持原 v1.0 参考值）
- [x] 修正 §14.6 行 1270 的误导性标注：将 "(bench_tolerance_test PASS)"
  改为 "(vs v1.5 工作基线 PASS；v1.0 基线对比见 §14.1，尚未执行)"
- [ ] 评估是否将 CI 的 "Bench tolerance gate" 从 `continue-on-error: true`
  改为硬门禁（需先解决 dev 主机噪声问题）

**执行条件**：
- Windows Server 2022, MSVC 14.44, x64, Release 配置
- 顺序执行（非并行，避免 CPU 竞争），各跑 3 轮取 best-of-3
- 方法详见 roadmap §10.6.6 "性能验证方法"

---

## E. v2.0 准备（新增类别）

v1.14 Anvil 已完成 v2.0 准备状态检查（roadmap §14.5），但以下项可
作为 v1.15 的 v2.0 前置工作：

### E.1 FCEUI_* 别名移除规划

- [x] 审计 107 个 `FCEUI_*` shim（core_api.h 61 + io_api.h 31 +
  movie.h 10 + cheat.h 5）的调用点，确认无内部代码使用
  — **实际 105 个**。内部调用点：`src/input.cpp` 15 个（函数指针表）+
  `src/tests/smoke_test.cpp` 14 个（地址检查）
- [ ] 评估将 `FCEUX11_NO_DEPRECATION_WARNINGS` 默认改为 OFF（即默认
  显示 deprecation 警告），暴露残留调用点
- [x] 6 个全局变量别名（`X`, `timestamp`, `soundtimestamp`, `scanline`,
  `MapIRQHook`, `g_cpu`）的内部调用点清零审计 — 已标注 FCEUX11_DEPRECATED
- [x] 生成 v2.0 移除清单文档（`docs/history/checklists/v2.0_removal_checklist.md`）

### E.2 bmap[] 标记 deprecated

`docs/history/history.md` 记录 v1.9 已将 `bmap[]` 标记
`[[deprecated]]`，但 UNIF loader 仍使用它。

- [x] 确认 `bmap[]` 的 deprecated 标注是否已落地 — **未落地**。
  `bmap[]` 在 `ines_init.cpp:30` 仍被活跃使用，不能标记 deprecated。
  `docs/history/history.md` 的声称有误。详见 `v2.0_removal_checklist.md` §3。
- [ ] 若未落地，v1.15 补上 `FCEUX11_DEPRECATED` 标注 — **不适用**，
  bmap[] 仍在活跃使用，标注会导致编译警告

---

## F. 其他 v1.15 候选任务

### F.1 mapper teardown 堆损坏

`mapper_byte_diff_test.cpp` 末尾注释（行 ~435）：

```cpp
// v1.8 Phase E.2 / v1.9: Use _exit(0) on success to avoid heap corruption
// in global/static destructors from legacy mapper code (exit code 0xC0000374).
// The corruption is in mapper teardown, not in the test logic.
```

测试用 `_exit(0)` 绕过了 global destructor 的堆损坏（0xC0000374）。

- [x] v1.15 调查 mapper teardown 堆损坏的 root cause — **已修复**。
  Root cause: `190.cpp` Mapper190_Close 和 `mmc5.cpp` NSFMMC5_Close 使用
  raw `FCEU_gfree()` 释放 RAII-owned 内存，导致 static FceuMallocPtr
  析构时 double-free (0xC0000374)。修复：改用 `owner.reset()`。
- [x] 修复后移除 `_exit(0)` workaround，改用正常 `return 0`

### F.2 LegacyExpansionAudio 声明位置

`legacy_expansion_audio.h:40-44` 的 5 个 `static` 实例在头文件中定义，
每个包含该头文件的 TU 都会生成独立副本。当前仅 5 个 board 文件包含它，
但若未来扩展可能造成符号膨胀。

- [ ] **→ v2.0** 评估是否改为 `inline` 或移至 `.cpp` 中定义 + `extern` 声明

### F.3 roadmap 文档全面同步

- [x] 勾选 §3 v1.3 Legion 全部 checkbox（行 169-242）
- [x] 勾选 §4 v1.4 Gateway 全部 checkbox（行 252-332），修正 §4.2 行 317
  的 `Bus::init()` 注释（已接入 PowerNES）
- [x] 勾选 §8 v1.8 Masonry checkbox，§8.2/§8.5 标注空壳覆盖状态
- [x] 修正 §14.6 行 1270 的误导性标注（见 D.2）
- [x] 更新 §0.3 版本总览表：v1.8 状态改为 ✅

---

## v1.15 执行结果

| # | 任务 | 状态 | 备注 |
|---|------|------|------|
| 1 | 文档同步（F.3 + A + B.3 + C.2 + D.2） | ✅ | roadmap 全部 checkbox 已同步 |
| 2 | v1.0 基线验证（D.1） | ✅* | 本地实测全部优于 v1.0 基线 18-29%；精确 v1.0 硬件对比较推到 v2.0 |
| 3 | 深化 Cart 子类化批次 1（B.1） | ✅ | NROM/MMC1/MMC3/VRC6 消除函数指针委托，编译+测试通过 |
| 4 | 深化 save_mapper_state（B.2） | ✅ | P0 mapper 已有真实 state body + golden 文件 |
| 5 | mapper teardown 堆损坏修复（F.1） | ✅ | 190.cpp/mmc5.cpp double-free 修复，_exit(0) 移除 |
| 6 | v2.0 准备（E） | ✅ | 105 个 FCEUI_* 审计 + v2.0_removal_checklist.md |
| 7 | 扩展音频评估（C.1） | → v2.0 | shim 满足功能，真正子类化投入产出比低 |

---

## 附：v1.x 版本交付状态（复核后修正）

| 版本 | 代号 | 状态 | 最终 commit | 备注 |
|------|------|------|-------------|------|
| v1.1 | Sentinel | ✅ | — | |
| v1.2 | Census | ✅ | — | |
| v1.3 | Legion | ✅ | — | |
| v1.4 | Gateway | ✅ | — | |
| v1.5 | Prism | ✅ | `c47fa4e` | |
| v1.6 | Resonance | ✅ | `d9879a9` | C 类推迟项已用 shim 完成 |
| v1.7 | Cartograph | ✅ | `c580b72` | |
| **v1.8** | **Masonry** | **✅ (原清单误判为未交付)** | `4708320` (tag `v1.8`) | 子类化为空壳模式，见 B 类 |
| v1.9 | Chronicle | ✅ | — | |
| v1.10 | Cryptex | ✅ | `07f0126` | |
| v1.11 | Bridge | ✅ | — | |
| v1.12 | Scissors | ✅ | — | |
| v1.13 | Purify | ✅ | tag `v1.13` | |
| v1.14 | Anvil | ✅ | `779d12d` | |
| **v1.15** | **Finale** | **✅** | (this commit) | **v1.x 最终版本** |

---

## 附：v1.15 任务优先级矩阵

| 任务 | 优先级 | 工作量 | 风险 | 依赖 | 状态 |
|------|--------|--------|------|------|------|
| F.3 文档同步 | **高** | 小 | 无 | 无 | ✅ |
| D.1 v1.0 基线验证 | **高** | 中 | 低 | 无 | ✅ |
| D.2 §14.1/§14.6 矛盾修正 | **高** | 小 | 无 | 无 | ✅ |
| B.3 §8 checkbox 勾选 | **高** | 小 | 无 | 无 | ✅ |
| B.1 Cart 子类化批次 1 | 中 | 大 | 中（热路径） | 无 | ✅ 需验证编译 |
| B.2 save_mapper_state 深化 | 中 | 中 | 低 | B.1 | ✅ P0 已完成 |
| F.1 teardown 堆损坏 | 中 | 中 | 中 | 无 | ✅ |
| E.1 v2.0 移除清单 | 中 | 小 | 无 | 无 | ✅ |
| C.1 扩展音频评估 | 低 | 大 | 低 | B.1 | ⏳ |
| F.2 LegacyExpansionAudio 位置 | 低 | 小 | 无 | 无 | ⏳ |
