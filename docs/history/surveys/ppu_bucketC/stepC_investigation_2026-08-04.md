# FCEUX11 v1.16 — PPU 桶调查 (Phase 3 Step 3.2 桶 C)

> **编制日期**: 2026-08-04
> **承接文档**: `docs/FCEUX11-1.16_P2-精度收敛三阶段构建方案.md` §5 / Step 3.2
> **承接报告**: `docs/history/reports/FCEUX11-1.16_P3-Step31_harness_cleanup_2026-08-04.md`
> **基线 commit**: `60136cc` (Step 3.1 完成) + 桶 A (`f4a072a`) + 桶 B (`57d3e88`)
> **2026-08-05 修订**: `ppu_open_bus` 0x03 → 0x00 PASS（`23b0cdd`，见
> `opendecay_probe_2026-08-05.md`），桶 C 2/4 收敛。
> **状态**: ✅ **2/4 收敛 (`ppu_read_buffer` + `ppu_open_bus` PASS)，2/4 已知限制（`oam_stress` + `ppu_vbl_nmi` 深模型族）**

---

## 0. 摘要

桶 C (PPU 真实精度) 含 **4 项 ROM FAIL**（Step 3.1 后期 Oracle B）：

| 错误码 | ROM | 状态 |
|---|---|---|
| 0x0E | `ppu_read_buffer` | ✅ **已收敛 → 0x00 PASS**（本桶首个，`863e9d7`） |
| 0x03 | `ppu_open_bus` | ✅ **已收敛 → 0x00 PASS**（`23b0cdd`，2026-08-05） |
| 0x01 | `oam_stress` | 🚧 已知限制（深模型族，PPU 隐式 OAM 改写） |
| 0x01 | `ppu_vbl_nmi` | 🚧 已知限制（CPU 侧读采样量化，Phase 1 vbl_02 同族） |

**核心成果**：
- `ppu_read_buffer` 0x0E → 0x00 PASS（`863e9d7`，2026-08-04，3 处 PPU bug：vnapage
  reset 保留、A2007 newppu 读更新 PPUGenLatch、palette 索引用 RefreshAddr）
- 顺带 `cpu_dummy_writes_ppu` 0x09 → 0x00 PASS（palette 索引同族）
- `ppu_open_bus` 0x03 → 0x00 PASS（`23b0cdd`，2026-08-05，3 处 PPU bug：
  600ms 时间衰减 + A2007 palette 高 2 位 + A2004 整字节刷新）

Oracle B 141 → 143 → 144 PASS，34 → 33 FAIL 与基线逐一一致，**零回归**。

---

## 1. 修复 1：Ppu::reset() 不重置 vnapage（mirroring 保留）

**文件**: `src/ppu_class.cpp` `Ppu::reset()`

### 根因
`FCEUPPU_Reset()` → `g_ppu.reset()` 把 `vnapage_[0..3]` 全重置为 `ntaram_`
（1-nametable）。PowerNES 顺序：`FCEUPPU_Reset` 重置 → `GameInterface(GI_POWER)`
→ CNROM `LatchPower` 只调 `CNROMSync`（setchr8/setprg32/setprg8r，**不碰
mirroring**）→ header 声明的 vertical mirroring 丢失，vnapage 停留在 1-nametable。

### 测试证据
`ppu_read_buffer.nes`（mapper 3 CNROM，header vertical）`TEST_NTA_MIRRORING_FAIL_1NTA`：
- 写 $23FF=$AA / $27FF=$BB / $2BFF=$CC / $2FFF=$DD
- 1-nametable 下 4 地址同块 → 读 $23FF = $DD → FAIL #14 (0x0E)
- 期望 vertical 读 $CC（$23FF/$2BFF 共享）→ 通过

env-gated 探针 `FCEUX11_PPUMIRROR_PROBE=1`（`ines_load.cpp` iNES_Init 后）确认：
`Mirroring=1 MapperNo=3` 正确解析，vnapage 显示 vertical；但运行时（PowerNES 后）
vnapage 退化为 1-nametable → 实测读 $DD。

### 修复
`Ppu::reset()` 不再覆盖 vnapage_。mirroring 是卡带硬件属性（iNES header 声明），
reset 不应清除。需要重设的 mapper（4-screen / 可切换 UNROM / MMC5 等）在自身
Power() 中调 `SetupCartMirroring` 覆盖，不受影响。

### 安全性
`Ppu` 构造函数（ppu_class.cpp:47-51）已设 `vnapage_[0..3] = ntaram_`，进程启动后
vnapage 永不 null。原先 reset() 的 null 兜底（hotfix3 C-1）不再需要。

---

## 2. 修复 2：A2007 newppu 读更新 PPUGenLatch（open bus）

**文件**: `src/ppu.cpp` `A2007` newppu 路径

### 根因
$2007 读把返回值放上 PPU 数据总线，后续读取无读功能的寄存器（$2000-$2006，
open bus）应重复该值。旧 PPU 路径有 `PPUGenLatch = VRAMBuffer`，newppu 路径缺失
→ $2000 读返回陈旧 PPUGenLatch。

### 测试证据
`TEST_PPU_OPENBUS_MUST_NOT_COPY_READBUFFER`（0x13=19）：
```asm
set_ppuaddr $2D00
:ldy PPUDATA      ; 读 $2007 → buffer 值 V
 cpy PPUCTRL      ; 读 $2000 → 应 = V（最后传输的总线值）
 bne @failed
```
FCEUX 的 $2000 读返回旧 PPUGenLatch ≠ V → FAIL。

### 修复
newppu 的 A2007 在 `return ret` 前加 `PPUGenLatch = ret;`（镜像旧 PPU 路径）。

---

## 3. 修复 3：A2007 palette 索引用 RefreshAddr 而非 tmp

**文件**: `src/ppu.cpp` `A2007` palette 分支

### 根因
A2007 开头 `tmp = RefreshAddr & 0x3FFF`（进入时全局值），然后
`RefreshAddr = ppur.get_2007access()`（ppur 寄存器真值）。palette 分支用 **陈旧
tmp** 做 READPAL 索引：`ret = READPAL(tmp & 0x1F)`。当 RefreshAddr 全局与 ppur
寄存器不同步（前次 $2007 读自增了 ppur，中间 open-bus/$2005 访问留陈旧全局值），
读到错误的 PALRAM 槽。

### 测试证据
`TEST_PALETTE_READS_UNRELIABLE`（0x30=48）：
- 写 $3F0F=$0E 后循环 255 次读 $3F0F 期望每次 $0E
- env-gated 探针 `FCEUX11_PALPROBE=1`（A2007 palette 分支）确认：
  - `tmp=3F0F ra=3F0F ret=0E`（前 41 次成功）
  - `tmp=0113 ra=3F0F ret=22`（之后失败：tmp 陈旧 → READPAL(0x13)=$22）
  - `tmp=2F0F ra=3F0F ret=0E` / `tmp=011B ra=3F0F ret=3C` 等
- B2006 探针确认：`V=3F tg=0->1 taddr=3F0D raddr=0113 gt7=3F0F` ——
  RefreshAddr 全局(=0113) 与 ppur 寄存器(=3F0F) 不同步

### 修复
palette 分支改用 `RefreshAddr & 0x1F`（get_2007access 更新后的真值）索引 PALRAM。

---

## 4. 顺带修复：cpu_dummy_writes_ppu

`cpu_dummy_writes_ppu`（0x09）在本次修复后转 PASS。根因同族——palette 读索引
（修复 3）+ open bus latch（修复 2）影响该测试的 PPU open bus / read buffer
验证。

---

## 5. 验证

| 项 | 结果 |
|---|---|
| `ppu_read_buffer` × 3000 frames | ✅ 0x00 PASS（基线 0x0E） |
| `cpu_dummy_writes_ppu` × 300 frames | ✅ 0x00 PASS（基线 0x09） |
| Oracle A `ctest -LE perf` | ✅ 34/34（构建时全量） |
| Oracle B 全量 177 ROMs | ✅ **143 PASS / 34 FAIL**（基线 141/36，+2 PASS） |
| 零 PASS→FAIL 回归 | ✅ 34 项 FAIL 与基线逐一一致 |

**剩余 3 项**（桶 C 未收敛）：
- `ppu_open_bus` 0x03 "Decay value should become zero by one second"——PPU open
  bus 电容放电衰减（PPUGenLatch 需随时间衰减到 0），许多模拟器不实现或简化
- `oam_stress` 0x01——OAM 压力测试，与桶 B 发现的 PPU 隐式 OAM 改写可能同族
- `ppu_vbl_nmi` 0x01——NMI 触发时序（manifest frames=300 不足，需 3000；Oracle B
  3000 frames 下为 0x01，测试 2 of 10 vbl_set_time 失败，与 Phase 1 vbl_02 同族）

---

## 6. 调查探针（已清理）

本次调查使用 env-gated 探针（`FCEUX11_PPUMIRROR_PROBE` / `FCEUX11_PALPROBE`），
已随修复提交清理（`git diff` 仅含 3 处修复）。如需复现调查，参考本文件 §1-3 的
探针描述。

---

## 7. 后续

桶 C 剩余 3 项评估：
- ~~`ppu_open_bus` 0x03：open bus decay 是有据精度限制（需 PPUGenLatch 电容衰减
  模型，与 deep model 族相关），建议记录为有据已知限制或独立小 PR~~
  → **✅ 已收敛**（2026-08-05，`23b0cdd`，见
  `docs/history/surveys/ppu_bucketC/opendecay_probe_2026-08-05.md`）。
  600ms 时间衰减 + A2007 palette per-bit + A2004 整字节刷新，
  三处小改，零回归。
- `oam_stress` 0x01：与桶 B `cpu_dummy_writes_oam` 的 PPU 隐式 OAM 改写同族，
  需深模型
- `ppu_vbl_nmi` 0x01：与 Phase 1 vbl_02 同族（CPU 侧读采样量化），已知限制

**桶 C 当前状态（2026-08-05）**：2/4 收敛（`ppu_read_buffer` + `ppu_open_bus` +
顺带 `cpu_dummy_writes_ppu`），2/4 已知限制（`oam_stress` + `ppu_vbl_nmi`）。
**桶 C 余项建议**：记入有据已知限制（与桶 A/B 模式一致）。

---

*调查完。桶 C 2/4 收敛（ppu_read_buffer + ppu_open_bus），2/4 已知限制待
Step 3.2 其余桶或深模型族突破。*
