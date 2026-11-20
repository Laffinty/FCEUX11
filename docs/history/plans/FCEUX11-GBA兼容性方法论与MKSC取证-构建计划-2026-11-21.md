# FCEUX11 — GBA 兼容性方法论与 MKSC 兼容性取证（构建计划）

> 本文是 **GBA 兼容性工作的方法论裁定** + **MKSC（美版 AMKE）取证批次 W1–W5** 的执行计划。
> 开工前先读 `AGENTS.md`（接手须知，尤其「GBA 与本体的关系」一节的硬约束与「门禁」节）；
> 探针触发方式见 `docs/tech/gba_probe_howto.md`。
> §四 是事实底座——取证已于 2026-11-21 完成，**内容冻结，实施期不再重查**（坐标失效以源码为准）。
> 在途本文放 `docs/plans/`，收口即移 `docs/history/plans/`。

---

## §〇 状态与业主裁决（2026-11-21）

| 项 | 内容 |
|---|---|
| 立项 | 2026-11-21，业主两项裁决：① 兼容性方法论裁定（下文「裁决一」）；② MKSC 取证批次 W1–W5 立项 |
| 版本归属 | **未定**——本文档刻意不用版本号命名；**不占用 v2.0.6**（其内容已冻结为发布流水线，tag/Release 待业主 dispatch）。收口时按业主分配的版本号落 CHANGELOG |
| 工作分支 | **尚未建立**（分支动作须业主逐次明确指示——AGENTS.md 长尾节口径）。W1 / W5 为研究区与文档动作，可先行；W2–W4 触及仓库前再问 |
| 工作区 | `Research_only/mksc_study/`（gitignored）：`scan1.py / scan2.py / scan3.py`（capstone 反汇编脚本）、`run_probe.bat / run_build.bat`（MSVC 环境 + cargo，全 ASCII）、`rom/mksc.gba`（ROM ASCII 副本，sha256 `aadc8b9f…`）、`build_probe.log` |

### 裁决一：兼容性方法论（约束所有 GBA 兼容性工作，永久有效）

**通用机制优化是唯一主线；不做针对特定卡带的 hack。「类型」按硬件变体切（存档芯片家族 / RTC-GPIO / 镜像例外），永远不按游戏切。**

| 层次 | 内容 | 做法 | 本仓先例 |
|---|---|---|---|
| **A 通用机制** | CPU/DMA/PPU/定时器/SWI/音频路径的精度 | 唯一主线；报告驱动的根因修复 | r56（halt 唤醒返回地址）、v2.0.1 P0（ARM SWI 编号位段）、v2.0.3 S3（SWI 实现/EEPROM 动态容量/mode5 翻页） |
| **B 硬件变体建模** | SRAM / Flash64 / Flash128 / EEPROM / RTC-GPIO、ROM 镜像例外、非 2 幂 ROM | 按**变体**实现通用模型 + 检测，不按游戏 | v2.0.3 S3-b EEPROM D1 动态容量；长尾⑦（mGBA 两例外） |
| **C 逐卡 override** | 最后手段 | **数据表**（game code 键控）+ 显式守卫，不写核心代码分叉 | 无先例，目标永远没有 |

**Tier C 四道门槛**（缺一不进）：① 根因调查证明该行为**真·逐卡**（坏 dump / 依赖未定义总线 / 检测不到的硬件配置），无任何通用修法；② 症状用户可感且显著；③ 实现为数据表 + 守卫，非散落分叉；④ known limitations 登记、可被上层配置覆盖。存档类型误判这类最常见需求走 `gba_set_save_type` 手动覆盖（长尾②）——**用户侧配置，不算 Tier C**。

**两条推论**：
- **单卡故障是通用机制缺口的探测器，不是 hack 的入场券**——每个兼容性报告按 r53/r56 铁律（探针+差分）处理，根因修在机制层。
- **诚实的代价**：个别卡会修得慢（FireRed 至今是「越过」init 竞争而非修好）。这与 GBA 长期 Alpha、不阻塞发版（r54 裁决）自洽。

### 裁决二：批次

W1–W5 全部立项（§三），建议顺序 **W1→W2→W3→W4→W5**；W3 可用「改 Option 存档」作为比赛存档受阻时的替代触发。

---

## §一 背景与目标

《Mario Kart: Super Circuit》是 GBA 首发软件。业主 2026-11-21 提出研究「完美兼容需要什么条件」。当日完成的四路取证（§四）结论：

1. **本仓 GBA 侧对该作的全部结构性需求已经满足**（halt+HDMA、逐像素 PPU、affine 内部参考点、Flash64 建模、FIFO DMA 定时器选择、BIOS SWI 面）——包括外部已知的最著名「MKSC 模拟器杀手」（GBADotnet #78：CPU Halt 时 HDMA 必须继续触发），本仓 `arm7tdmi.rs` 的通用 halt 语义（r56）直接满足，**不需要一行 MKSC 专属代码**。
2. **权威信源证实 mGBA/VBA-M 对 AMKE 零 override、零游戏专属 hack**——不存在需要照抄的魔法。
3. 剩余工作是**证据闭环**（boot→比赛→存档三阶段的动态取证）+ 若干**引擎级精度缺口**的登记（不修）。

本计划把上述结论可执行化。**「完美兼容」的判定口径（防过度承诺）**：通用机制满足 + 三阶段证据闭环 + 业主人工验收（画面/声音/存档）。**不承诺 100%**——GBA 长期 Alpha 是红线。

## §二 非目标

1. 不做任何 MKSC 专属代码分叉（裁决一）。
2. 不建模 Atmel page-write 闪存命令：本仓报 Panasonic 假 ID `(0x32,0x1B)`=1B32（mGBA 同款技巧），SDK 库按 ID 分支走**标准擦写路径**（0x80+0x30 擦 / 0xA0 写）——已实现路径恰好是被选中的路径。真卡 Atmel AT29LV512（ID 0x1F3D）仅作事实登记。
3. 不做 SIO/联机（单卡；RCNT/SIO 代码在 ROM 中存在但激活需要联机线）。
4. 不做音频保真度调优（M4A 路径结构已通；听感归业主人工验收）。
5. 不做 60 FPS / 宽屏等改行为增强。
6. GBA 不进 F11QA / ctest 门禁；新增锁测试一律 `#[cfg(test)]`，逐帧哈希基线只存研究区。
7. NES 侧零改动；任何 `src/rust/**` 变更后 `ctest -LE perf`（49 项，以 `ctest -N -LE perf` 为准）必须全绿（不变式 1）。

## §三 批次

### W1 基线入档（≈0.5 天，零仓库改动）

**目标**：固化 MKSC 当前行为基线，供后续任何改动对照。

**步骤**：
1. 重跑并保存探针报告到 `Research_only/mksc_study/baseline/`（Git Bash，cwd=`Research_only/mksc_study`）：
   ```bash
   export GBA_PROBE_ROM='D:\Project\FCEUX11\Research_only\mksc_study\rom\mksc.gba'
   export FCEUX11_GBA_PROBE_DIR='D:\Project\FCEUX11\Research_only\mksc_study\rom'
   export FCEUX11_DIAG_LOG='D:\Project\FCEUX11\Research_only\mksc_study\diag.log'
   cmd //c "call run_probe.bat probe_cartridges_for_wedges" > baseline/cartprobe_boot.txt 2>&1
   cmd //c "call run_probe.bat probe_a_real_cartridge"          > baseline/probe_a.txt       2>&1
   ```
2. **澄清 `dump_frames_like_the_reference` 落盘语义（只读）**：2026-11-21 实测——不带 `GBA_PROBE_TRACE` 时打印 "wrote 600 frames" 但文件仅首帧+终态两行；`GBA_PROBE_TRACE=1` 则切换为逐步 trace 模式。读 `src/rust/src/gba/frame.rs` 该探针 doc comment，把语义写成研究区 README 一段；**不改探针行为**（若确认是缺陷 → 登记 §六，不修）。
3. 研究区写 `README.md`：工件清单 + 复跑命令 + 基线读数摘要（§四⑧）。

**验收**：三个文件存在，README 能让下一个人独立复跑。
**风险**：无（零代码改动）。

### W2 注入推进到比赛（1–2 天，预期零仓库改动）

**目标**：把取证从 boot/标题推进到**比赛玩法**，运行期验证 MKSC 的标志性机制——**DMA0 HDMA 逐扫描线刷 BG2 仿射参数**（§四④）。

**现状坐标**：boot 窗口 DMA0 未武装（cartprobe 实测）；HDMA 装载函数 0x0804AF74 静态解码：CNT_H=`0xA660`（timing=**2** HBlank + 32 位 + repeat + 源递增 + 目的递增/回卷）、CNT_L=4（4×32 位 = 16 字节参数块 BG2PA..BG3PB）、目的 `0x04000020`。
**探针能力**：输入注入 `GBA_PROBE_INPUT` + `GBA_PROBE_ONLY`（v2.0.3 r6 落地，Dragon Ball 真卡注入先例）——**注入值格式以 `src/rust/src/gba/frame.rs` / `cartprobe.rs` doc comment 为准，本文不凭记忆写格式**。

**步骤**：
1. 读探针 doc comment 确定注入格式；设计键位序列（标题→档案/菜单→GrandPrix→比赛），注入序列按实跑迭代。
2. 注入跑 cartprobe 窗口；**判据：窗口内 DMA0 armed 且 timing=2**（与静态 0xA660 印证）+ 赛道画面段帧哈希持续变化。
3. 产出 `baseline/cartprobe_race.txt`；「boot 窗口未武装 vs 比赛窗口 armed」作为一对证据保存。

**验收**：`baseline/cartprobe_race.txt` 含 DMA0 armed timing=2 + verdict running；README 登记注入序列。
**反向守卫**：W1 的 boot 基线必须保持原读数——两份报告对照即守卫。
**止损线**：注入走不进比赛超过 ~2 个工作日 → 降级：业主在 GUI 发布构建人工跑一场（透视是否正确 = 用户可感证据），DMA0 运行期观测转 §六，降级动作记 §八。**不阻塞收口**。
**风险**：菜单导航注入不稳定（先例证明可行但非 trivial）。

### W3 存档端到端（1–2 天，预期零仓库改动）

**目标**：闭环 MKSC 的 **Flash64 存档**通路：触发存档 → flash 命令流 → `.srm` 字节 → 重载读回。

**现状坐标**：`FLASH_V124` → `BackupType::detect` → Flash64（`internal_memory.rs:78-85`）；命令状态机 0x90/0xF0/0x80+0x10/0x30/0xA0/0xB0（`:841-940`）；flash ID 报 `(0x32,0x1B)`（`flash_id()`）；`.srm` 读写导出面已有（S3-1，r46）；读档 redetect（v2.0.3 apply_state 修复）。

**步骤**：
1. **观察通道先定**（不凭空设计）：优先用既有 `.srm` 导出/读回面做前后对照（存档动作前后各导一次，比对字节）；命令级 trace 视 `tracing::debug` 在 cargo test 下有无 subscriber 而定——没有就不强求，命令级覆盖交给「若 W4 风格锁测试缺失则补一条 Flash64 命令序列锁」的备选步骤。
2. 触发存档：改 Option（菜单内可达，不依赖比赛）；若必须赛后记录，复用 W2 注入成果。
3. 校验：`.srm` 尺寸/内容合理；重载后读回一致（apply_state 路径）；存档→重启→记录画面呈现已存数据。
4. 若实测暴露 flash 状态机缺陷 → **另立修复子项**：修根因 + `#[cfg(test)]` 锁 + 双向变异 + `cargo test -p gba-core --lib` 全绿，不混入本批次收口。

**验收**：`.srm` 前后对照记录 + 重载读回一致 + README 登记。
**反向守卫**：存档前的 boot 基线不变（W1）。
**风险**：Option 保存是否落 flash 未实测（可能延迟到退出才写）；触发点找不准 → 比赛记录路径兜底。
**非目标**：不建模 Atmel；不验 EEPROM/SRAM（MKSC 用不上）。

### W4 OAM 优先级锁（≈1 天，纯测试改动）

**目标**：把 GBADotnet #64 场景（**同优先级的仿射与非仿射精灵**的绘制顺序）固化为合成夹具锁，防 re-vendor 静默回退。**不依赖 MKSC ROM。**

**现状坐标**：vendor OBJ/优先级合成在 `gba-core/src/cpu/hardware/lcd.rs:299-358`（逐像素、层排序）+ `lcd/object_attributes.rs`；测试放 **root crate 夹具区** `src/rust/src/gba/fixtures/`（公共底座 `fixtures.rs`；`fixtures/lcd.rs` 是 v2.0.3 r17 的注册流程先例）——**不放 vendor**，避免 vendor 树与 ATTRIBUTION 记账膨胀。

**步骤**：
1. 新夹具 `fixtures/oam_priority.rs`：经 bus 写 OAM 构造「同 BG 优先级、一个 affine 一个普通、坐标重叠」双精灵，步进一帧，读帧缓冲断言遮挡关系。**期望值从权威来源独立推导，不调实现重算**（纪律 5；测试与实现同错已发生三次，不许第四次）。
2. **同一个提交**内注册进 G2 夹具集合与守卫扫描模块清单（`fixtures/lcd.rs` 流程先例）。
3. **变异验证**（纪律 3）：**先提交夹具**，再变异 vendor 优先级排序 → 锁转红 → 还原 → 转绿。还原用 `git checkout -- <file>` 或显式刷 mtime（**禁止 Copy-Item 还原**——保留时间戳 ⇒ 假红，AGENTS.md r15）。
4. 更新根 crate 锁测试计数（当前 GBA 根 319 = 299 passed / 20 ignored；夹具集合 14 → 15），收口时改 AGENTS.md。

**验收**：夹具全绿 + 变异腿记录（红点与预登记预期一致）。
**风险**：优先级规则与权威来源对不上 → **停下来记录，不写**（纪律 4）。

### W5 观察项登记与收口（≈0.5 天）

**步骤**：
1. §六 观察项表定稿。
2. 收口动作：本文移 `docs/history/plans/`；AGENTS.md 长尾节加一条指针（方法论裁定 + MKSC 结论各一行）；CHANGELOG 归属按业主分配版本号在发版时落。
3. 若 W2 实测出注入格式的增量知识，补 `docs/tech/gba_probe_howto.md` 一行。

**验收**：仓库 diff 只含文档与（若做了）W4 测试；AGENTS.md 计数同步。

---

## §四 事实底座（2026-11-21 取证冻结）

> 四路交叉取证：ROM 静态反汇编（capstone，scan1–3.py）/ 权威信源（联网检索）/ 本仓实现实读 / 动态探针（release）。

### ① ROM 基本信息
4 MiB（0x400000）· CRC32 `ED316E37` · sha256 `aadc8b9f…` · header：title `MARIO KART\0\0` / code `AMKE` / maker `01` / ver 0 / 补码校验 OK · Gekkio gbhwdb「AGB-AMKE-0」同卡（真卡 flash = Atmel AT29LV512，ID `0x1F3D`）。

### ② 代码形态
- **Thumb 主体 + ARM crt0/分发器**。crt0（0xC0–0xF0）把 **0x08000100（IE&IF 位扫描器）** 写进 0x03007FFC = 游戏 IRQ 分发器；表基址 0x03004F80（pool 0x1D4）；分发器另有 IWRAM 拷贝迹象（pool @0x62608：0x080000FC / 0x03002400 / 0x03004F80，精确角色未追尽，不影响结论）。
- **BIOS 调用全部 Thumb SWI 低 8 位**（0x61350–0x61390 包装）：`0xDF0B` CpuSet / `0xDF06` Div(含 Mod) / `0xDF12` LZ77UnCompVram / `0xDF11` LZ77UnCompWram / `0xDF25` MultiBoot / `0xDF01` RegisterRamReset / `0xDF00` SoftReset / `0xDF05` VBlankIntrWait。ARM 模式 `num<<16` 约定仅 crt0（`svc 0x010000` @0x118）。软复位函数 0x8061370：IME=0 → SP=0x03007F00 → SWI 1(0xFF) → SWI 0。
- ⚠️ 取证工具坑：scan3.py 对 Thumb 窗口的 svc 注释会错标「bits16-23」——Thumb SWI 是低 8 位；复用脚本时注意这个假象。

### ③ 存档
`FLASH_V124` @0x1E9778 = **64 KB flash**（非 SRAM/EEPROM）。SDK flash 驱动区 0x613xx–0x627xx：ID 序列（AA→0x0E005555 / 55→0x0E002AAA / 0x90）+ 延时循环 + 0xF0 退出。命令池：0x0E005555 ×9 @0x613B8、0x0E002AAA ×8 @0x613BC。

### ④ DMA0 HDMA（MKSC 标志性机制，外部对应 GBADotnet #78）
装载函数 **0x0804AF74**（Thumb）：拷 8 字节结构入 IWRAM → `DMA0SAD`=运行期 IWRAM 参数块指针（GBADotnet 报告引用 0x03005060）→ `DMA0DAD`=**0x04000020（BG2PA）**（pool 0x4AFC8）→ CNT 字 **`0xA6600004`**（pool 0x4AFD0：CNT_L=4，CNT_H=0xA660 = 使能 + timing=**2** HBlank + 32 位 + repeat(bit9) + 源递增(bits7-8=00) + 目的递增/回卷(bits6-5=11)）→ 读回校验 → ack IF bit1（写 2 到 0x04000202，pool 0x4AFD4）→ **IE |= 2**（pool 0x4AFE0=0x04000200）。即：CPU 在 IWRAM 维护 16 字节参数块（BG2PA..BG3PB），每个 HBlank 由 DMA0 整块刷进仿射寄存器，伪造赛道透视。帧侧 ISR（0x2D2BC，pool 0x2D318=0x040000B0）：DMA0CNT_H &= 0xC5FF 再 &= 0x7FFF（先拆 timing/repeat 位再拆使能位）。

### ⑤ 其他硬件使用指纹
WAITCNT=`0x4494`（探针首帧实测）· DISPSTAT 池引用 28 处 · KEYINPUT 直接轮询 · TM0/TM3 池引用显著 · SIO/RCNT 存在（联机用，单卡不激活）· SOUNDCNT/PSG 簇在 0x602xx–0x60Bxx（M4A/Sappy 驱动）· DMA1 武装 `control=0xB600`（timing3 + FIFO_A + 32 位 + repeat）。
**`0x04000308` = 图形数据假阳性**（x1 @0x10ECBC，两种反汇编模式均垃圾解码）——**别追**。

### ⑥ 权威信源要点
- **mGBA `src/gba/overrides.c`（master）无 AMKE 条目；VBA-M vba-over.ini 无条目；mGBA CHANGES 无 MKSC 专属修复** ⇒ 通用机制即可完美跑（NGEmu "works perfect"）。
- GBADotnet #78（HDMA+halt）/ GBADotnet #64（OAM 仿射-非仿射优先级）/ notorious_beeg #45（**帧内 DISPCNT/BG 模式切换是正确行为**，非缺陷）。
- 音频 = 改造版 M4A/Sappy（hcs64 论坛），标准 FIFO+Timer Direct Sound 路径（loveemu MP2K 文档）。
- 美版反编译 **jellees/mksc** 存在（可选参照，未拉取）；mstan/MarioKartSuperCircuitRecomp 以其作符号来源。
- 完整 URL 清单见记忆 `mksc-compat-research` 与 2026-11-21 会话记录。

### ⑦ 本仓实现对照（全部 file:line 实读）

| 需求 | 坐标 | 判定 |
|---|---|---|
| Halt 时 DMA/时钟继续 + 唤醒 | `gba-core/src/cpu/arm7tdmi.rs:741-757`（r56，注释点名 MKSC） | ✅ |
| HBlank/VBlank DMA 触发 | `gba-core/src/bus.rs:1329-1341` | ✅ |
| 逐像素 PPU + 帧内 DISPCNT/模式切换 | `gba-core/src/cpu/hardware/lcd.rs:299-358` | ✅（粒度细于硬件） |
| affine 内部参考点（帧首锁存 + dmx/dmy 推进） | `gba-core/src/cpu/hardware/lcd/registers.rs:142-173` | ✅ |
| Flash64 建模 + 字符串检测 + 假 ID | `gba-core/src/cpu/hardware/internal_memory.rs:78-85, 841-940, flash_id()` | ✅ |
| FIFO DMA（timing3 / 定时器选择 / 复位位 / CPU 直写） | `bus.rs:715-741, 825-842` + `cpu/hardware/sound.rs:226-237` | ✅ |
| BIOS SWI 面（Thumb 低 8 位 + ARM bits16-23） | `src/rust/src/gba/swi/mod.rs` | ✅（240 帧未实现记录器零命中） |
| DISPSTAT 轮询（不依赖 IRQ 使能） | `bus.rs:1334-1339` | ✅ |
| DMA 优先级**交错** | `bus.rs:612-619`（按 0→3 但块原子执行） | ⚠️ → §六 |
| 定时器 start_delay=2 | LT-16 待裁决 | ⚠️ → §六 |
| savestate 逐帧等价 | v2.0.5 r14 已登记（引擎级） | ⚠️ → §六 引用 |
| OAM 仿射/非仿射优先级特例 | 排序存在（`lcd.rs`），#64 场景未实测 | ⚠️ → W4 |

### ⑧ 动态探针读数（2026-11-21，release，`run_probe.bat`）
- **probe_a**（30M 步）：卡带 ROM 86.6% / IWRAM 13.4% / stub BIOS 9 样本；**61% 停在 0x0806138C（VBlankIntrWait 返回点）= halt-wake 全程工作**；画面色集变化 15 次；无 panic / 无 STATE。
- **cartprobe**（60 预热 + 180 测量帧）：0/180 预算超限；**128 个不同帧哈希**（真跑非冻帧）；DMA1 armed timing3 `0xB600`（STILL ENABLED = 正常常驻）；**DMA0/2/3 未武装**（boot 窗口——HDMA 只在比赛阶段）；SWI 未实现记录器**零命中**；最热 PC 0x0806138C 连续 176 帧；verdict = running / picture changing。
- **dump_frames**：落盘语义与 doc 不符 → W1 步骤 2 澄清（探针行为不改）。

## §五 全局纪律（执行期适用）

1. 每批次开工前重读 §二 非目标与裁决一。
2. 探针只走 `run_probe.bat`（MSVC env）；环境变量从 bash 侧传入；路径全 ASCII（中文路径载入即静默终止，CHANGELOG 7b3ccef 教训）。
3. 任何 `src/rust/**` 变更 → `ctest -LE perf` 49 项全绿后才许收口（不变式 1）；跑 ctest 不与构建并行。
4. 新增锁测试当场确认计数（纪律 10：「测试写了」≠「测试会跑」）。
5. 变异验证三铁律：**先提交再变异**（`git checkout` 会丢未提交改动，2026-11-20 二次踩雷）；还原腿必须刷 mtime（`Copy-Item` 保留时间戳 ⇒ 假红）；共享 vendor 树的变异串行做。
6. 锁测试期望值从权威来源独立推导，不调用被测实现重算。
7. push / tag / 建分支一律等业主明确指示。

## §六 观察项（登记不修）

| 项 | 现状 | 触发条件 |
|---|---|---|
| DMA 优先级交错（块原子执行） | AGS DMA PRIORITY 红；v2.0.1 定性需独立 RE | 比赛窗口实测出 DMA 竞争伪影 |
| 定时器 start_delay=2 | LT-16 待裁决（GBATEK 零覆盖 + 四套参照均无延迟） | 业主裁决或音频时序实证 |
| savestate 逐帧等价 | v2.0.5 r14 已登记（引擎级） | TAS / 对拍需求出现 |
| SIO / 联机 | 不在范围 | 联机需求出现 |
| 音频听感 | M4A 结构通路已通，未人工验收 | 业主人工验收 |
| dump_frames 落盘语义 | 与 doc 不符（2026-11-21 实测） | W1 澄清后若判缺陷 → 另立小项 |
| **SELECT 设置屏入口条件（游戏侧）** | 「PRESS SELECT」提示下 Select 在 6 种状态/时长被无视；游戏活表（IWRAM `0x03004F30`，IWRAM 差分定位）完整看到 `bit2=0x04` ⇒ **非本仓缺陷**，是游戏 UI 条件未命中 | 业主 GUI 人工一次即可分辨（设置屏能否打开）；本仓不动 |
| `0x4000308` BIOSPROT | **已结案**：图形数据假阳性 | 无 |

## §七 出口判据

1. W1：三份基线文件 + README 可复跑；
2. W2：比赛窗口 DMA0 armed timing=2 证据（或降级路径与记录）；
3. W3：`.srm` 端到端闭环记录（或受阻分析记录）；
4. W4：OAM 优先级夹具绿 + 变异腿红（或确证已正确只留锁）；
5. §六 表定稿；本文归档 `docs/history/plans/`；AGENTS.md 长尾节指针就位；
6. 全程零 MKSC 专属分叉；若动过 `src/rust/**`，ctest 49 项全绿记录在案。

## §八 变更记录

- **r1（2026-11-21）**：立项。业主两项裁决入档：① 兼容性方法论裁定（通用机制主线 / Tier B 硬件变体 / Tier C 四门槛禁止清单）；② MKSC 取证批次 W1–W5 立项。§四 事实底座冻结（四路取证：ROM 反汇编 scan1–3.py / 权威信源 / 本仓实现实读 / 探针实测）。版本归属未定——命名用日期，不占用 v2.0.6。
- **r2（2026-11-21，全批执行收口）**：五批次全部执行完毕，逐项结果：
  - **W1 ✅**：三份基线落盘 `Research_only/mksc_study/baseline/`（boot / race / probe_a）+ 研究区
    README（含复跑命令与 harness 编译配方）。`dump_frames` 语义澄清：默认模式把
    `GBA_PROBE_FRAMES` 当**步数**循环（600 帧请求 ≈ 2.8 帧游戏时间、只落 1 个采样行），
    `TRACE=1` 是按周期的逐步 trace——判为探针缺陷候选登记 §六，未改行为。
  - **W2 ✅**：注入导航无头进比赛成功（排程与研究区 README 固化）。**`DMA0: armed with
    timing 2 (HBlank)` 于测量帧 1541 实测，control=`0xA660` 与 ROM 装载函数逐字一致**；
    cartprobe 比赛基线 2400 帧 = 2081 个不同帧哈希、0 预算超限、verdict running。
    GBA 核心跑真机级比赛玩法（透视 HDMA）实证。
  - **W3 ◐ 止损降级（计划 §三 预设路径）**：观测工具全部就绪并验证（电池哈希监视 +
    `.srm` 快照 + DMA/按键监视）；flash 全阶段零写入被证实（boot/菜单/比赛 12000+ 帧
    Flash64 65536 字节不变 = 只在真正存档动作时写）。**触发侧两条路径到线**：
    ① 设置屏 Select 在 6 状态被无视——**已排除本仓缺陷**（游戏活表 `0x03004F30`
    完整看到 bit2=0x04，IWRAM 差分定位），判定游戏 UI 条件未命中 → §六 新观察项；
    ② 盲驾 12000 帧 REVERSE+草地未完赛。降级：触发交业主 GUI 人工，观测侧就绪。
  - **W4 ✅**：`fixtures/oam_priority.rs` 两条镜像腿**从出生即绿**（本仓对 GBADotnet #64
    的裁决本来正确）；变异验证闭合（并列裁决 `>`→`>=` 翻转 ⇒ 双腿仅在重叠断言红、
    独占列全绿 = 外科级定位；`git checkout` 还原 ⇒ 绿）；G2 集合同提交收扩（14→16 条，
    守卫扫描模块 5→6）。实施坑：mode 3-5 下 OBJ 字符区对应 2D 编址 **tile 512+**，
    低号砖被整层跳过（GBATEK `:3156-3157`）。
  - **W5 ✅**：Rust 门禁全绿（root 301 passed/20 ignored = 321 总含 2 条新锁、gba-core
    249、debug 94、media 52、formats 158、utils 51、lua 54、双态 check EXIT=0）；
    ctest `-LE perf` 门禁随重链跑毕（读数见提交信息）。本文按 §七 归档
    `docs/history/plans/`；AGENTS.md 长尾节指针就位。
  - **实施新事实（非缺陷）**：root crate 是 `crate-type=["staticlib"]`，不能作 Rust
    依赖——研究区 harness 的编译配方（隔离 target + LTO off + `--crate-type rlib` +
    `ffi-stubs` feature + 双 `-L` + crt-static/panic=abort）固化在研究区 README。
    探针观测坑新增一条：`cartprobe` 的 keymirror 类观测必须先做 **IWRAM 差分定位**
    真表，pool 词猜地址会命中无关数据。
