# E-1 调查记录：PPU VBL 周期与 blargg `ppu_vbl_nmi`

状态：**未完成（调查阶段）**。本文件记录本次会话的实测数据与结论，供后续接手。
代码未做任何行为改动 —— `src/ppu_rendering.cpp` 的 VBL 段仍是 `9998b2b` 的配置。

## 0. 最重要的一条：之前的结论建立在过期二进制上

上一会话记录的「`01-vbl_basics` 因 #8 回归」是**错误结论**。

`build-c1/` 是增量构建树，其中的 `fceux11_blargg_runner.exe` 是用 P4-1 配置
（`runppu(1)` 前置 + `delay=19`）编译的，而工作树里的源码早已被 `9998b2b`
改回 `delay=20` / VBL 在 cycle 0。两者不一致，导致读数失真。

本次**重新完整构建 `build-c1` 后实测**（Release，CTest 34/34 全通过）：

| ROM | frames=300..900 | $6000 |
|---|---|---|
| `vbl_01_basics.nes` | 全部 **PASS** | `0x00` |

即：当前提交状态下 `01-vbl_basics` 是通过的，不存在「修 02 会打坏 01」的
既有回归。之前 `ppu_rendering.cpp` 里那段「fundamental tension」注释是基于
错误数据写的，不要再当作约束。

> 教训：动 PPU 时序前，先 `do_build.ps1` 全量重建再取基线。增量树里的
> `.exe` 可能比源码旧好几个 commit。

## 1. 当前实测基线（需以全新构建复测确认）

用 `fceux11_blargg_runner.exe --rom <单项 ROM> --frames N` 逐项跑
`tests/fixtures/blargg/ppu/vbl_0*.nes`。**注意**：下表 02~10 的读数取自
**过期二进制**，`01` 已用新构建复核；02~10 尚未在新构建上复测，接手第一步
应当补测。

| ROM | $6000 | 现象 |
|---|---|---|
| 01-vbl_basics | `0x00` | PASS（新构建已确认） |
| 02-vbl_set_time | `0x80` | 停在 `T+ 1 2 / 00 `，测试不推进 |
| 03-vbl_clear_time | `0x80` | 停在 `00 ` |
| 04-nmi_control | `0x00` | PASS |
| 05-nmi_timing | `0x80` | 停在 `00 ` |
| 06-suppression | `0x80` | 停在 `00 ` |
| 07-nmi_on_timing | `0x80` | 停在 `00 ` |
| 08-nmi_off_timing | `0x80` | 停在 `03 ` |
| 09/10 even_odd | `0x80` | 无输出 |

`$6000 = 0x80` 在 blargg 协议里是 **“测试仍在运行”**，不是错误码。
把它当成「VBL period too long」是误读 —— 真正的失败码是 `0x00-0x7F`。
所以 02/03/05/06/07/08/09/10 的症状是**挂死**，不是判定失败。
（`0x81` 才是「需要按 reset」。）

## 2. 插桩实测数据

临时在 `runppu()/runppu1_inline()` 加全局 dot 计数、在 VBL set/clear 点和
`A2002` 读点打点（env `FCEUX11_E1_TRACE`，**已回滚，未提交**），跑
`vbl_01_basics` 400 帧：

- VBL 周期：`period=6820`，**398/398 帧全部一致** → 周期本身是对的。
- 帧长：`89342`（渲染关）/ `89341`（渲染开的奇数帧跳点）交替 → 符合硬件。

结论：**「VBL 周期算错」这个原假设不成立**。E-1 在计划里的描述
（`ppu_rendering.cpp:1547-1582` 周期过冲）是错的。

## 3. 真正的障碍：`sync_vbl` 需要 1 PPU dot 的读取分辨率

02/03/05/06/07/08/10 全部 `.include "sync_vbl.s"`，其核心是：

```asm
:       delay 27 - 11
        bit $2002
        bit $2002
        bpl :-
```

循环 27 CPU cycle，帧长 29780.67 CPU cycle，`27 × 1103 = 29781`。
即每 1103 次迭代，读点相对 VBL **前移正好 1 PPU dot**，靠这个把 CPU
精确对齐到 VBL 边沿（注释原文：“to accuracy of 1/3 CPU clock”）。

我们的 CPU 是**指令原子执行**的：`X6502_RunDebug` 里
`ADDCYC(CycTable[b1])` 在 dispatch **之前**就把整条指令的周期记完，
然后才做内存访问。于是：

- `bit $2002` 在硬件上第 4 个 cycle 才读 PPU；
- 在我们这里，读到的是**指令起始 dot 时刻**的 `PPU_status`。

也就是 CPU 观察到的 VBL 相位固定偏早约 3 CPU cycle = 9 PPU dot，且
**指令内部没有 dot 级分辨率**。`sync_vbl` 的收敛循环因此可能永远拿不到
它要的那个边沿状态 → 挂死在第一行输出。

这解释了为什么这些测试是「卡住」而不是「打印出错误的表」。

## 4. 下一步建议（按性价比排序）

1. **先补测**：新构建下重跑 02~10，确认第 1 节表格。
2. **验证第 3 节假设**：在 `A2002` 里为 newppu 路径加「读发生在指令第 N
   个 cycle」的相位补偿（把 `PPU_status` 的观察时刻后移到指令末尾），
   看 `sync_vbl` 是否收敛。这是最小改动的验证手段。
   - 风险：会改变所有 `$2002` 读的时序，必须回归 `01`/`04` 与
     Oracle A（`rom_regression_test`、`golden_savestate`）。
3. 若 2 有效，再谈 02 的 9 行表格对齐（VBL set 落在 sl 241 dot 0 还是
   dot 1）。**在 sync 收敛之前调 dot 0/1 是无意义的** —— 之前 P4-1 就是
   在这一步反复横跳。
4. `08-nmi_off_timing` 停在 `03` 而不是 `00`，说明它前几行是走通的，
   可作为最先攻克的目标。

## 5. 判定口径

`tests/tests.json` 里 `blargg_ppu_vbl_nmi` 目前是 `advisory`。
在 E-1 真正修好前**不要**改回 blocking，也不要为了让它变绿去动
`failure_means`。
