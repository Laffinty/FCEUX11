# D4 探针取证产物（2026-11-06 复跑）

> **来源**：FCEUX11 v2.0.2 计划 P3 批次 D4，探针提交 `adc45ae`
> **复跑日期**：2026-11-06
> **目的**：把 r6/r7 变更记录里的 D4 数字从「二手表格单元格」变成**可离线复核的产物**
> （此前仓库内无任何产物，r6 数字只有 commit message 与计划表格两处二手记录）。

## 复现命令

```powershell
# 前置：构建 f11qa-rom-runner（Cargo [[bin]]，非 CMake 目标）
cd src/rust
cargo build -p f11qa --bin f11qa-rom-runner
# 产物：src\rust\target\x86_64-pc-windows-msvc\debug\f11qa-rom-runner.exe

# 050（阴性）：探针 env 必须传给子进程 f11qa_blargg_runner.exe ⇒ PATH 要含 build\tests
$env:FCEUX11_OAMBUS_PROBE = "<产物目录>"
$env:PATH = "D:\Project\FCEUX11\build\tests;D:\Project\FCEUX11\vcpkg_installed\x64-windows\bin;$env:PATH"
cd D:\Project\FCEUX11
& src\rust\target\x86_64-pc-windows-msvc\debug\f11qa-rom-runner.exe `
    --kgmqa-id kgmqa-050-cpu-dummy-writes-bisqwit --tests-json tests/tests.json --frames 2000

# 110（阳性对照）：kgmqa_id 里的真实名字是 kgmqa-110-dma-sync-test-v2-rahsennor
& src\rust\target\x86_64-pc-windows-msvc\debug\f11qa-rom-runner.exe `
    --kgmqa-id kgmqa-110-dma-sync-test-v2-rahsennor --tests-json tests/tests.json --frames 2000
```

**踩过的坑**：
- runner 的 `--tests-json` 是**相对仓库根**的路径（`tests/tests.json`）；在 `tests/` 下跑会报「找不到路径」。
- 子进程 `f11qa_blargg_runner.exe` 靠 **PATH** 找（不在仓库根），缺了会 exit 3「program not found」。
- 110 的用例 id **不是** `kgmqa-110-dma-sync-bisqwit`（那不在 tests.json 里），真实是
  `kgmqa-110-dma-sync-test-v2-rahsennor`。

## 050 探针输出（阴性）

```
OAMBUS PROBE begin dir=... trace_cap=268435456 bytes
OAMBUS POWER RESET cpu_ts=0 — armed=0, shadow resynced
OAMBUS ARMED cpu_ts=90192 dot=152 sl=248 oamaddr=0x36 PPUON=0 — shadow synced, monitoring SPRAM rewrites
OAMBUS PROBE end dma_bursts=0 dma_bytes=0 diff_events=0 armed=1 trace_bytes=8
```

被测结果：`BLARGG_RESULT: rom=cpu_dummy_writes_oam.nes addr=0x6000 value=0x06 diag=[0xDE,0xB0,0x61] status=FAIL duration_ms=3109`

**读法**：
- `value=0x06` ⇒ **复现了历史报告的 0x06 失败**（缺陷确实在，不是环境问题）。
- `armed=1` ⇒ **探针确实 armed 过**（`PPU[3]==0x36 && PPUON==0` 命中）⇒
  「`diff_events=0`」**不是「从未 armed」的平凡真值**，是有效的阴性证据。
  ⚠️ r6 未记录 armed 状态；本次复跑补上了这一条（调查页 §6.3 曾把它列为「完全未知」）。
- `dma_bursts=0` ⇒ 该 ROM 的失败窗口内**没有 DMA burst** ⇒ 与 r6 一致。
- `diff_events=0` ⇒ 失败窗口内**没有 SPRAM 改写** ⇒ 与 r6 一致。

## 110 探针输出（阳性对照）

```
OAMBUS PROBE begin dir=... trace_cap=268435456 bytes
OAMBUS POWER RESET cpu_ts=0 — armed=0, shadow resynced
OAMBUS ARMED cpu_ts=59744 dot=213 sl=241 oamaddr=0x36 PPUON=0 — shadow synced, monitoring SPRAM rewrites
OAMBUS PROBE end dma_bursts=75164 dma_bytes=19241984 diff_events=0 armed=1 trace_bytes=268435472
OAMBUS PROBE: trace.bin hit 268435456-byte cap — further records dropped (spram_diff.log continues)
```

被测结果：`BLARGG_RESULT: rom=dma_sync_test_v2.nes addr=0x6000 value=0x00 diag=[0x00,0x00,0x00] status=PASS duration_ms=4415`

**读法**（这是让 050 的阴性结论可信的关键对照）：
- `dma_bursts=75164` / `dma_bytes=19241984` ⇒ **探针在真实 DMA 场景下确实在记录**
  （trace 写到 256 MB 上限、`trace.bin` 撞 cap）⇒ 050 的 `dma_bursts=0` 不是「探针坏了」。
- `armed=1` 同样命中 ⇒ 两侧 armed 条件一致。
- `diff_events=0` ⇒ 该 ROM 下 SPRAM 也无改写（110 是 PASS，两侧一致）。

## 为什么没有 `trace.bin`

`trace.bin` 是二进制事件流，110 对照在 2000 帧内写满 **256 MB 上限**
（`trace_cap=268435456`）——**不适合入库**（git 仓库放不下这么大的二进制）。
因此本目录**只保留 `spram_diff.log`（探针的文本汇总，含全部关键计数）**，
`trace.bin` 需要时按上面的命令复跑即可重现。

**推论（未验证）**：trace 容量按 24 字节/记录算，256 MB ≈ 1100 万条事件；
说明 110 的 DMA 事件密度极高，若要长期保留 trace 产物，需先加采样/过滤策略。
