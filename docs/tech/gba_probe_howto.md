# GBA 探针与测量工具 HOWTO（C3，v2.0.2）

> 本页是探针族的**触发方式**索引。每个探针的语义与输出判读以其源码 doc comment
> 为准——那才是事实源，本页只回答「怎么跑起来」。
> 计数基线：**21 条 `#[ignore]`**（root crate 19 + vendor gba-core 2），与
> AGENTS.md 当前值一致；本页落地于 v2.0.2 P0/P1 之后。

## 运行方式（两类）

```powershell
$vc = "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat"

# 诊断探针（debug 构建即可；只看行为不看时序时用这档）
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-rust --no-default-features --features gba --lib <探针名> -- --ignored --nocapture"

# 计时探针（--release 强制；debug 数字无意义）
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-rust --release --no-default-features --features gba --lib <探针名> -- --ignored --nocapture"
```

要点：

1. **`#[ignore]` 不进门禁**——「全绿」不等于跑过；跑探针必须显式 `--ignored`，
   且当场看到测试计数与输出（纪律 10）。
2. **ROM 发现**（frame.rs 的 probe 族共用 `probe_rom_path()`）：`GBA_PROBE_ROM`
   环境变量优先（须为已存在文件）；否则扫 `%USERPROFILE%\Desktop` 下第一个
   `.gba`。找不到时打印提示并**优雅返回**（不是失败）。
3. **路径须 ASCII**（中文路径载入即静默终止——见 CHANGELOG 7b3ccef 的教训；
   已修但探针路径仍建议 ASCII）。
4. **ROM 须 ≥512 字节**（`MIN_CARTRIDGE`，加载器前置检查）。
5. 日常冒烟用 MKSC（桌面测试区副本）或任何真卡带；无 ROM 时多数探针仍可跑
   合成卡带（`tests` 内的 `cartridge()` 形状：0x200 字节、分支@0、SWI 0@0xC0）。

## 探针清单

### frame.rs —— S4 诊断族（5 条，均需真 ROM）

| 探针 | 用途 | 环境变量 |
|---|---|---|
| `probe_a_real_cartridge` | 跑 N 步报告 PC 区域分布/点亮率/热点地址；CPU 没到卡带 = stub BIOS 断 | `GBA_PROBE_STEPS`（默认 30M） |
| `probe_why_a_game_stalls` | 卡死游戏的寄存器/中断现场 | 同上 |
| `probe_trace_a_stuck_game` | 卡死轨迹 | `GBA_PROBE_WARMUP`（默认 5M）+ `GBA_PROBE_TRACESTEPS`（默认 600K）。⚠️ **不读 `GBA_PROBE_TRACE`** —— 该变量只被 `dump_frames_like_the_reference` 读（2026-11-06 核实更正） |
| `probe_what_one_frame_changes` | 一次帧边界改了哪些内存半字 | 不用环境变量（步数是硬编码常量，`:2931`） |
| `dump_frames_like_the_reference` | 逐帧差分转储（对拍 mGBA 等参照） | `GBA_PROBE_FRAMES`（默认 1800）、`GBA_PROBE_OUT`（默认 ours.txt）、`GBA_PROBE_PERIOD`（默认 280896）、`GBA_PROBE_TRACE` |

### frame.rs —— 计时探针（1 条）

| 探针 | 用途 | 环境变量 |
|---|---|---|
| `measure_frame_export` | 帧导出 FFI 全链 vs 转换的分解读数（v2.0.2 C1 的前后对照基线） | `GBA_PROBE_FRAMES`（默认 300，另 60 帧预热）；**--release 必须** |

### save.rs（2 条，无需 ROM）

| 探针 | 用途 | 环境变量 |
|---|---|---|
| `the_raw_decoder_needs_a_stack_of_its_own` | 小栈下主动把进程带崩的栈探针 | `GBA_PROBE_DIR`、`GBA_PROBE_STACK_BYTES` |
| `measure_a_state_on_disk` | savestate 编解码耗时/体积（L13、C2 的对照基线；REPEATS=20） | 无 |

### cartprobe.rs（1 条）

| 探针 | 用途 | 环境变量 |
|---|---|---|
| `probe_cartridges_for_wedges` | 批量卡带老化报告（每卡打印 steps/frame、DMA 计数、逐帧哈希） | `FCEUX11_GBA_PROBE_DIR`（**必填**，未设即 panic 并给指引；目录下逐个 `.gba` 跑）。**AGS 卡须强制 SRAM 存档类型**（见其 doc comment） |

### waitprobe.rs（10 条，无需 ROM）

WAIT 状态时序测量族——模块 doc comment 是唯一完整清单；编号与用途随 v2.0.1 P2
先行工作项固化的期望值测试（`gba-core` 侧另有一条 WAIT 期望值锁测试）配套。

### vendor gba-core（2 条）

`src/rust/crates/gba-core/src/cpu/hardware/timers.rs` 内 2 条计时测量；
跑法：`cargo test -p gba-core --lib <名> -- --ignored --nocapture`。

### 全局

| 变量 | 作用 |
|---|---|
| `FCEUX11_DIAG_LOG=<path>` | Rust panic 落盘（Qt `main.cpp` 与 `gba/mod.rs` 双入口）——**任何探针首次运行都建议设**，崩了有现场 |

## 方法论（r53/r56 铁律）

新的兼容性报告按 r53/r56 的**探针 + 差分**方法处理：先用探针取「核心真的做了
什么」的现场，再与参照实现逐帧差分。**锁测试全绿不排除系统性缺陷**——三次先例
（r33、r56、v2.0.1 P0）都是「测试与实现同错，闭环自洽」，靠外部参照才暴露。
新增门禁时先问：**这条断言的通过原因，与它要验的那条路径是不是同一条。**
