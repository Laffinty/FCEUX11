# GBA 外部测试报告（jsmolka 套件 + AGS 老化卡现状）— 2026-10

> **性质**：一次独立的外部测试。全部测试产物位于 `Research_only/`（gitignored），
> 仓库零改动（报告提交前 `git status` 确认 working tree clean）。本报告**不代表**
> 将任何测试套件引入 GBA 模块或门禁——按既定裁决（计划 §十 不变式 9），GBA 的
> 出口判据仍然只有 `ctest` 不回归与它自己的锁测试。

## 一、摘要

用无头 harness 驱动 FCEUX11 的 GBA 核心（与 App 完全相同的 C ABI + stub BIOS +
SWI hook），运行 jsmolka/gba-tests 全部 13 个测试 ROM。结果：

- **10 个 ROM 直接全过**（memory、thumb、nes、unsafe、save×4、ppu×3 视觉正确）；
- **2 个 ROM（arm、bios）白屏挂死**——定位出一个**真实核心缺陷**：
  ARM 模式 SWI 编号取错字节（`src/rust/crates/gba-core/src/cpu/arm7tdmi.rs:686-696`），
  已用 ROM 变异验证锁死因果；
- arm 套件隔离该缺陷后唯一实质失败是 t234（ARM7TDMI 的 MSR 别名怪癖，极冷门）；
- bios 套件 4 项全是 BIOS open-bus 行为检查，属 stub BIOS 设计差异，非核心精度缺陷；
- **AGS 老化卡 v7.0 已跑出成绩单**（需用 `gba_set_save_type` 强制 SRAM——该 ROM
  带 `EEPROM_V` 签名但程序按 SRAM 总线访问设置区，原样运行会 CHECK SUM ERROR
  死循环，mGBA 同样如此）：KEY INPUT/INTERRUPT/DMA 大部/LCD 大部通过，
  **失败集中在时序敏感类**（存储器等待周期与预取缓冲、定时器预分频/级联、
  HBlank 状态窗口）——AGS 级严格度的真实精度差距信号。

## 二、测试方法与保真度

### Harness

`Research_only/gbatech/gbaharness.cpp` → `gbaharness.exe`，C++ 编写，链接
`cargo build -p fceux11-rust --no-default-features --features gba --release`
产出的 `fceux11_rust.lib`（与锁测试同 feature 配置，无 `direct-adapter`），调用
与 App 完全一致的导出面：`gba_load_rom` → `gba_step_frame`（逐帧 wedge 上限
语义同 `frame.rs::gba_step_frame`）→ `gba_frame_buffer`（BGR555→RGBA 转换与
App 相同）。因此测的就是**发布栈**：stub BIOS、`install_swi_hook` 的 SWI
dispatch、vendored clementine 核心、颜色转换链，全部在环。

链接说明：staticlib 会强制包含全部对象，其中 `fceux11_lua_*`（78 个符号）是
App C++ 侧提供的 Lua 回调，独立链接必然 LNK2019。`Research_only/gbatech/lua_stubs.cpp`
生成了 abort() 桩——GBA 无头路径永不调用 Lua 引擎，桩安全且触发即响。
（注：曾尝试以 rlib 方式让 Rust 直接依赖根 crate，被 fat-LTO 位码消费问题挡住
——这正是计划 R14/r8 已记录的工具链限制，遂改走 C++/staticlib 路线。）

### 证据与判卷

每个 ROM 在第 1/30/60/120/300/600/1200 帧导出 BMP（`out_js/*.bmp/png`）+
帧哈希（静态屏可见）+ 末帧 savestate JSON（完整机器状态，用于取证）。
jsmolka 的判卷模型：全部通过显示 "All tests passed"，否则显示首个失败测试号。

## 三、AGS 老化卡：已运行出成绩单（需强制 SRAM）

用户提供的 ROM：`AGS Aging Cartridge (World) (Rev 1) (v7.0) (Test Program).gba`
（4 MB，md5 `6087dc2b8532e1cba787e1f53ab5a051`）。注意这不是 DenSinH 补丁所针对
的版本（`9f74b2ad...`），两个补丁脚本的偏移对它无效，全部结果按 ROM 原生屏幕判卷。

### 3.1 直接运行：CHECK SUM ERROR 死循环（与 mGBA 行为一致）

原样运行：正常引导（Nintendo logo + "AGBIOS TEST PROGRAM Version 7.0"）→
**"CHECK SUM ERROR! DEFAULT IS SAVED."** → 自复位 → 无限循环，进不了测试序列。
逐帧 savestate 证明 EEPROM 内容写一次后逐字节不变（默认值持久化成功），但每次
引导读回校验都失败。**mGBA 0.10.5 同样如此**：参照 trace 的 PC 轨迹与本核心逐值
相同（双方都稳定在 `0x0800D660` 区循环）。mGBA 日志显示程序除 EEPROM 串行协议
（DMA3 9 位命令/68 位响应）外，还在顺序直读 **0x0E001004+ 的 SRAM 总线区**。

### 3.2 根因：EEPROM_V 误检；强制 SRAM 即解锁

ROM 内含 `EEPROM_V` 签名（0x141DFC，无 SRAM_V），签名扫描把存档介质判为 EEPROM
（本核心与 mGBA 同判）。但 v7.0 程序实际以 **SRAM 总线直读**方式访问设置区
（0x0E001000+）；EEPROM 激活时串行协议的读回数据参与校验导致永远失败。用 S3-1
的手动覆盖 `gba_set_save_type(1 /* SRAM */)` 强制后，擦除态（全 0xFF）被校验接受，
程序直接进入测试序列。这同时是 S3-1「存档类型手动覆盖」设计价值的实证。

### 3.3 成绩单（frame 600 起稳定，`out_agssram/`）

符号含义（像素级核对）：**`0` = 通过、`X` = 失败**、`-` = 未执行（COM 需链路
硬件，程序自动跳过）、行尾 `✱FAIL`/`✱PASS` 为整类判定。测试名与顺序按
Normmatt/ags_aging 的测试表映射：

| 类别 | 结果 | 明细 |
|---|---|---|
| MEMORY（9 项） | ✱FAIL | EWRAM/IWRAM/PALETTE/VRAM/OAM/CARTRIDGE TYPE FLAG/PREFETCH/WAITSTATE/RAM-WAIT = `XXXXX 0 XXX`——仅 **CARTRIDGE TYPE FLAG** 过 |
| LCD（7 项） | ✱FAIL | `00000X0`——仅 **H BLANK STATUS** 失败，V COUNTER/三组 INTR FLAG/V COUNT STATUS/V BLANK STATUS 全过 |
| TIMER（3 项） | ✱FAIL | `XX0`——**PRESCALER、CONNECT** 失败，INTR FLAG 过 |
| DMA（显示 8 列/表 9 项） | ✱FAIL | `000000X0`——地址控制 4 通道与 V BLANK START 等大多通过，1–2 项失败（列与 9 项表的对应缺 1 列，精确到项待查） |
| COM | `-` | 未执行（无链路硬件，符合预期） |
| KEY INPUT | ✱PASS | 通过 |
| INTERRUPT（4 列） | 全 `0` | 通过 |

**解读**：失败集中在**时序敏感**类——存储器等待周期/预取缓冲时序、定时器预分频
与级联时序、HBlank 状态窗口、个别 DMA 时序项。这正是 AGS 老化卡著名的高严格度
所在（jsmolka 套件不覆盖的精度层），是 vendored 核心**真实的精度差距信号**；
逐测试的根因定位属后续工作（按长尾 3 的探针+差分方法论立项）。

### 3.4 局限

- mGBA 屏幕级对照未做（本地 mGBA 为 lib-only 构建，无前端）；社区公开结果中
  mGBA 接近全过，故上述失败应视为本核心的真实差距而非 ROM/环境问题。
- 本 ROM 不是 DenSinH 补丁的目标版本，无法用 output patch 拿到逐测试的
  flags 位级数据；符号判卷的粒度以屏幕为准。
- DMA 行 8 列对 9 项表的对应关系存在 1 列缺口，需显示代码或 mGBA 对照确认。

## 四、jsmolka 套件结果总表

| ROM | 覆盖面 | 结果 |
|---|---|---|
| memory | 内存访问/镜像/video_strb | **All tests passed** |
| thumb | THUMB 指令面 | **All tests passed** |
| nes | nes | **All tests passed** |
| unsafe | 未定义/边角行为 | **All tests passed** |
| save/none, save/sram, save/flash64, save/flash128 | 卡带存档硬件（SRAM、Flash 64K/128K、无芯片） | **All tests passed**（S3-1 存档通路的强佐证） |
| ppu/hello, ppu/shades, ppu/stripes | mode 4 位图/渐变/条纹 | 渲染正确 |
| arm | ARM 指令面 513 项（conditions/branches/flags/shifts/data_processing/psr/multiply/transfer×2/swap/block） | 白屏挂死 → 隔离后 **Failed test 234**；跳过 t234 后全过 |
| bios | BIOS open-bus 行为 4 项 | 白屏挂死 → 隔离后 Failed test 002 → 逐项确认全为 open-bus 检查 |

## 五、发现 1（实质缺陷）：ARM 模式 SWI 编号取错字节

**位置**：`src/rust/crates/gba-core/src/cpu/arm7tdmi.rs:686-696`

```rust
// ARM SWI: read the word at PC-8 and extract bits 0-23
let swi_pc = (next_ins as u32).wrapping_sub(4);
let swi_instr = self.bus.read_word(swi_pc as usize);
swi_instr & 0xFF // GBA BIOS only uses lower 8 bits
```

**问题**：ARM7TDMI 的约定（GBATEK，且与真机/BIOS/mGBA 一致）是 **bits 16-23**
承载 SWI 编号，低 8 位是注释字段。规范写法 `swi 0x060000`（Div）在本核心被读成
`0x00` = **SoftReset**：每次 ARM 模式 BIOS 调用都触发一次软复位。jsmolka 套件在
eval 的失败打印路径调用 Div，于是「测试失败 → 打印 → 软复位 → 重跑 → 再失败」
无限循环，屏幕停留在 init 阶段（白屏）。THUMB 模式 SWI 为 imm8 编码，不受影响
——这正是 thumb 套件全过、arm/bios 挂死的差异根源。

**证据链**：
1. 卡死态 savestate：机器每帧推进恰好 280,896 cycles、PC 恒在 eval 的 vsync
   宏内振荡、VRAM 两页全空（打印从未发生）、IWRAM 中 Div 结果全零；
2. mGBA 0.10.5 参照（`Research_only/refdump.exe`）对同一条 `0xEF060000` 解码为
   SWI 06 并正常执行；
3. **变异验证**：把 ROM 内 `EF060000` 补丁为 `EF000006`（迁就本核心的取字节位）
   后，机器立即完成判卷——bios 显示 "Failed test 001"、arm 显示
   "Failed test 234"（`roms_jsmolka/bios_mut.gba`、`arm_mut.gba`，截图在
   `gbatech/out_js/`）。因果锁死。

**影响面**：所有以规范编码使用 ARM 模式 BIOS 调用的代码——jsmolka/AGS 等测试
ROM、libgba 风格的 homebrew、以及任何在 ARM 段调用 SWI 的商业代码。vendored
核心的本地补丁集不涉及此处（改动属上游代码），修复时应按 ATTRIBUTION §3 登记。

**建议修复**（未实施，等裁决）：ARM 分支改为 `(swi_instr >> 16) & 0xFF`。
同时建议按纪律 3 补一条变异可红的锁测试（构造 ARM 模式 `swi 0x060000` 断言
Div 语义生效且 CPSR/寄存器不被 SoftReset 干扰）。

## 六、发现 2（边角）：ARM7TDMI 的 MSR 别名怪癖未实现

arm 套件 t234「Bad CMP / CMN / TST / TEQ change the mode」：真机 ARM7TDMI 上
`0xE15FF000` 这类畸形编码会作为 MSR 执行并改变 CPU 模式；本核心按普通 CMP
处理。该怪癖只影响刻意利用编码别名的代码，商业软件几乎不会触碰。测试 234
之后的其余 279 项（multiply/single/halfword/swap/block transfer）在跳过该测试
后**全部通过**。

## 七、发现 3（设计差异）：BIOS open-bus 未建模

bios 套件全部 4 项测的是「SWI/IRQ 执行期间读 BIOS 总线返回真 BIOS 对应偏移的
指令字」（0xE129F000 / 0xE3A02004 / 0xE25EF004 / 0xE55EC002）。本项目的 stub
BIOS 是自写的向量+引导代码，读到的自然是 stub 自身——这 4 项失败是 stub 设计
的直接推论，不是核心精度缺陷。mGBA 之所以能过，是其 HLE BIOS 刻意模仿真 BIOS
字节。若未来要过这一族，需要建模「BIOS 窗口 open-bus 读」，涉及与纪律 4
（不凭真 BIOS 字节/位级格式）的边界，属于设计取舍而非修复。

## 八、发现 4（加载器行为）：MIN_CARTRIDGE 拒载微型 ROM

`frame.rs` 的 `MIN_CARTRIDGE = 0x200` 使 <512 字节的 ROM 在装载时报
`GBA_ERR_BAD_ROM`。jsmolka 的 shades.gba（352 B）/stripes.gba（324 B）被拒，
补齐零到 512 字节后正常运行。真实卡带均 ≥512 B，影响面仅限微型 homebrew 与
测试 ROM；记录在案即可，不建议改动（该检查保护的是核心头解析的防短切片
panic 语义）。

## 九、与既有工作的一致性

- SMA4 经 harness 启动到语言选择画面，与 r51 手测一致；GB PLAYER logo 正确，
  说明 boot 链（stub BIOS → 头部校验 → 卡带入口）与颜色通道顺序正确；
- 存档硬件四项全过，与 S3-1 的 `.srm` 读写通路互为佐证（注意：本套件验证的是
  flash/SRAM 芯片行为，**不含** EEPROM，也**不验证**跨模拟器 `.srm` 字节互通）；
- 本次外部测试全部为 Research_only 工具，未触碰 244 项锁测试与任何门禁口径。

## 十、附录：复现

```
# 组装测试 ROM（FASMARM v1.44，Research_only/fasmarm/）
Research_only/fasmarm/fasmarm.exe <dir>/<name>.asm Research_only/roms_jsmolka/<name>.gba

# 构建 staticlib（Research_only/gbatech/build_staticlib.bat，cargo 无 direct-adapter 配置）
# 编译 harness（build_harness.bat；lua_stubs.cpp 为自动生成的链接桩）
# 运行单个 ROM
Research_only/gbatech/gbaharness.exe <rom.gba> <帧数> <输出目录> <标签>
# 逐帧 savestate 轨迹（取证用）
set GBA_SS_EVERY_FRAME=<目录> 后同上
# 强制存档介质（AGS v7.0 需要 SRAM=1；S3-1 手动覆盖的 C ABI）
set GBA_FORCE_SAVE_TYPE=1 后同上
```

证据目录：`Research_only/gbatech/out_js/`（jsmolka 截图/savestate）、
`Research_only/gbatech/out_ags/`（AGS 原样运行 = CHECK SUM ERROR 死循环）与
`out_agssram/`（强制 SRAM = 成绩单及逐行放大图）、`Research_only/roms_jsmolka/`
（原版与变异 ROM）、`Research_only/agsuite_mod/`（源码级适配副本，仅用于隔离
诊断）。adapted 副本的改动仅有三类，均已注释 `RESEARCH DIAG`：SWI 编码适配、
t234/t001-t004 跳过、m_vsync 未动。AGS ROM 本体在 `Research_only/` 根目录，
未做任何修改。
