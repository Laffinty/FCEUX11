# FCEUX11 v2.0.5 — A1 音频探测线程门控修复（构建计划）

> **STATUS: 阶段 1 执行中**
>
> **上游依据**：`docs/plans/FCEUX11-v2.0.5_音视频异常缓慢-稳定性审计报告-2026-11-19.md`
> （下称「审计报告」）。本计划只做审计报告 §9.2 建议的那一项（A1），外加**同文件同根因的
> A2**，不碰 `WriteSound`（留阶段 2）。
> **开工指令**：业主 2026-11-19「请开工第一阶段」。

---

## 一、范围

### 1.1 纳入

| 编号 | 内容 | 归属 |
|---|---|---|
| **A1** | `KillSound()` 无条件 `join` 后台探测线程，且落在 GUI 线程 + `FCEU_WRAPPER_LOCK` 内 ⇒ 探测未结束时把剩余探测时长整段搬回 GUI 线程 | `36af048`（2026-11-16）引入 |
| **A2** | 该探测线程是文件级 `static std::thread`；存在一条「线程被创建但永不 join」的可达路径 ⇒ 进程退出时 `std::terminate` | `36af048` 引入 |

### 1.2 不纳入（另立阶段）

- `WriteSound` 三处「设备已开但停排空」自旋（审计报告 F1 的残留面）⇒ 阶段 2。
- A3 `SDL_setenv` 副作用 / A5 无设备时合成为空 / GBA 节流残留 / §5.4 锁外双检查 ⇒ 阶段 3。
- 审计报告里已实测否证的面（`draining` 零出现）不重测。

### 1.3 硬约束

1. **NES 侧零行为变更**：`WriteSound` / 帧循环 / 模拟线程一行不动；只动音频设备**开/关的时序**。
2. **不做新的阻塞**：任何 GUI 线程调用路径的新增阻塞上限必须为 0（除进程退出路径）。
3. **探测时长不变**：`SDL_OpenAudio()` 仍然在后台线程跑（PERF-01 的收益不能丢）。
4. **AV 归属**：不动 `src/rust/`，不新增 GBA 引用（`decoupling.rs` 的三条守卫不受影响）。

---

## 二、现状坐标（实施前核对，均以 `main` @ `9e0b0f6` 为准）

```
src/drivers/Qt/sdl-sound.cpp
  :95-100     s_audioDeviceOpen / s_audioOpenInFlight / s_audioOpenRate /
              s_audioFailAtMs / s_audioOpenMutex / s_audioOpenThread
  :253-324    AudioOpenDeviceWorker（探测真正发生的地方）
  :335-371    AudioEnsureDeviceOpen（复用判定 / 在飞判定 / 退避 / join + 起线程）
  :470-491    InitSound 的 deviceReady 两分支（!deviceReady ⇒ 重新分配 s_Buffer）
  :757-778    KillSound（无条件 join ⇒ SDL_CloseAudio + QuitSubSystem + s_Buffer.reset）
src/drivers/Qt/fceuWrapper.cpp
  :156-159    DriverInitialize: if (InitSound()) inited |= 1
  :190-194    DriverKill: if (inited&1) KillSound()
  :1139-1147  fceuWrapperClose: CloseGame(); fceu11::Kill(); SDL_Quit();
src/drivers/Qt/ConsoleSoundConf.cpp
  :466-467 / :665-666 / :682-683   KillSound(); InitSound();（成对）
  :586                              InitSound()（**不更新 inited** ⇒ A2 的入口）
```

审计报告 §1.3 的量化门槛（探测 > 约 455 ms 才可观测）成立；本计划不改该结论。

---

## 三、设计

### 3.1 为什么不是「`KillSound` 里判断在飞就跳过 join」这一行式修法

跳过 join 会引入两个新缺陷，必须一起解决，否则是换一种坏法：

1. **`SDL_CloseAudio()` 与 `SDL_OpenAudio()` 并发**（原 `join` 正是为此存在）；
2. **`KillSound(); InitSound();`**（三个设置对话框的标准姿势）在探测在飞时会被跳过 ⇒
   设备要么永不打开（用户开一次设置就永久无声），要么再排一次满时长探测。

### 3.2 采用：可注入的探测门控状态机

新文件 `src/drivers/Qt/audio_probe_gate.{h,cpp}`，**纯 std、不依赖 Qt/SDL/config**
（唯一目的：让它能配一条几乎零成本的独立锁测试）。

状态：`m_openRate`（0 = 关闭）、`m_inFlight`、`m_inFlightRate`、`m_teardown`、
`m_pendingOpen`（深度 1，后到覆盖先到）、`m_thread`、`m_mutex`（只保护状态迁移，
**绝不跨 SDL 调用持有**）。

| 入口 | 语义 |
|---|---|
| `requestOpen(rate, openFn)` | 设备已在 `rate` 上 ⇒ `Opened`；在飞且同速率 ⇒ **清 teardown 并复用**（这是 `KillSound(); InitSound();` 不二次探测的关键）；在飞且异速率 ⇒ 挂 pending；空闲 ⇒ 起线程 |
| `requestClose()` | 在飞 ⇒ **只置 teardown 并立即返回 false（完全委托，不碰 SDL、不碰 `s_Buffer`）**；空闲 ⇒ 内联执行 `closeFn` 并返回 true |
| 线程体 | `openFn` → 取 teardown：置位则关设备不发布；否则发布 `m_openRate`。随后若有 pending ⇒ 续跑一轮，否则置 `m_inFlight=false` 退出 |
| `waitIdle()` | **仅退出路径**：置 teardown 后 join。阻塞可接受 |
| `~AudioProbeGate` | 兜底：joinable ⇒ 置 teardown + join（析构内不调用 SDL） |

`join` 的唯一出现处是 `waitIdle()`/析构（退出路径），以及 `requestOpen` 里对**已结束**
线程的回收（此时线程体已返回，耗时为微秒级，非「剩余探测时长」）。

### 3.3 sdl-sound.cpp 的接线

- `AudioOpenDeviceWorker` 的 SDL 工作整体搬进注入的 `openFn`；成功分支仍按原顺序
  `SDL_PauseAudio(0)` → 归零三个游标 → 发布 `s_audioDeviceOpen/s_audioOpenRate`（顺序不动，
  见 `:296-301` 的既有理由）。`closeFn` = `SDL_CloseAudio() + SDL_QuitSubSystem()` 并清两个原子。
- `AudioEnsureDeviceOpen` 保留**退避窗口**判定（`s_audioFailAtMs`），在飞判定交给门控。
- `KillSound`：`fceu11::Sound(0)` → `gate.requestClose()`；**仅当返回 true（内联关闭）**
  才 `s_Buffer.reset()` 与归零游标。委托路径不动 `s_Buffer`（否则会与紧随其后的
  `InitSound` 抢，见 `:479` 的重新分配）。
- 新增导出 `WaitSoundProbeIdle()`（`dface.h`），在 `fceuWrapper.cpp` 的四个 `SDL_Quit()`
  之前调用 ⇒ 同时修掉「SDL_Quit 与 SDL_OpenAudio 并发」与 A2 的 `terminate`。

### 3.4 取证开关

`FCEUX11_AUDIO_TRACE=1` 时向 stderr 打印：探测开始/结束（含耗时）、`requestClose` 的
「内联关 / 委托」、teardown 实际执行点、pending 续跑。默认不读环境、零成本（首次使用时
缓存一次 getenv 结果）。

---

## 四、步骤

1. 本计划落档 + 审计报告 r2 更正（A6 REFUTED / §5.4 REFUTED / F1 否证面收窄）。
2. 新增 `audio_probe_gate.{h,cpp}`。
3. `sdl-sound.cpp` 接线；`dface.h` 加 `WaitSoundProbeIdle()`；`fceuWrapper.cpp` 四处 `SDL_Quit()` 前调用。
4. 新增 `tests/f11qa/audio/audio_probe_gate_test.cpp` + `tests/CMakeLists.txt` 目标
   （**直连两个 TU、不链 core** ⇒ 构建成本秒级，不占 LTCG 的 5–25 min）。
5. 构建 + 跑锁 + `ctest -LE perf`（45 → 46）。
6. 变异验证。
7. CHANGELOG `[Unreleased]`。

---

## 五、验收判据

### 5.1 锁测试（四腿，注入「慢 open = sleep 2000 ms」）

| 腿 | 断言 | 变异后的预期 |
|---|---|---|
| T1 | 在飞时 `requestClose()` 返回耗时 **< 100 ms** 且返回 false | 还原无条件 join ⇒ **红**（实测 ≈2000 ms） |
| T2 | 委托 teardown 完成后 `isOpen()==false`，且 `closeFn` 恰好被调用一次 | 去掉 teardown 判定 ⇒ 红（设备泄漏为打开态） |
| T3 | `requestClose(); requestOpen(同速率);` ⇒ **只探测一次**（`openFn` 调用计数 == 1），最终打开 | 去掉「同速率复用/清 teardown」⇒ 红（计数 2 或最终未打开） |
| T4 | 异速率请求：`requestOpen(48000)` 在 44100 在飞时 ⇒ 第一轮 openFn=44100（被 teardown 丢弃）、第二轮 openFn=48000 且最终 `openRate()==48000` | 去掉 pending 续跑 ⇒ 红 |

### 5.2 门禁

- `ctest -LE perf` **45 → 46**，全绿。
- `cargo` 五条 + vendor 不受影响（本阶段零 Rust 改动）—— 仍按 AGENTS.md 门禁节跑一次确认。
- 本机实跑：加载 ROM 帧率仍 58–60 fps；`Loading SDL sound with ... driver` 仍出现一次；无 `draining` 打印。

### 5.3 反向守卫

- `grep -n "join()" src/drivers/Qt/sdl-sound.cpp` **只应剩 0 处**（join 全部收敛到门控）。
- `git diff` 中 `WriteSound` / 帧循环零改动。

---

## 六、风险

| 风险 | 说明 | 处置 |
|---|---|---|
| 委托 teardown 后无人关闭设备 | 线程体必须无条件执行 teardown 分支 | T2 锁死 |
| `s_Buffer` 与委托关闭抢内存 | 只在**内联**关闭路径 `reset()` | 见 §3.3 |
| 退出期 `SDL_Quit` 与探测并发 | `WaitSoundProbeIdle()` 在四处 `SDL_Quit()` 前 | 见 §3.3 |
| 探测慢的机器上退出要等探测 | 业主机器可能 +8 s 退出 | 可接受（进程已在关闭）；已在 trace 里可见 |
| LTCG 重链可能 5–25 min/个 exe | 本机既有事实（坑 13） | `-j 1` 串行，不中途取消 |

---

## 七、变更记录

| r | 日期 | 内容 |
|---|---|---|| r1 | 2026-11-19 | 立项：范围 A1+A2、设计（可注入门控状态机）、四腿锁 + 变异、四处 `SDL_Quit` 前等待；把 F1 残留/A3/A5/GBA 残留/§5.4 划到阶段 2–3 |
| r2 | 2026-11-19 | **实施记录**。① 锁腿由 4 条增至 **5 条**：T5 改为**源码守卫**（`sdl-sound.cpp` 全文无 `.join()`，且 `KillSound` 函数体内用 `requestClose()` 且**不得**调 `WaitSoundProbeIdle`）——只测门控测不出「调用方又变回去」。② 实施期发现并修正设计缺口：`requestOpen(异速率)` **不得**取消待办关闭（SDL 不能在设备还开着时按新速率重开），原设计里「在飞即清 teardown」会让 T4 的关闭丢失；现为「同速率才清」。③ 变异 **M1–M5 逐腿实测转红**（M1 关闭改回 join / M2 忘记关设备 / M3 同速率不清 teardown / M4 丢弃异速率排队 / M5 `KillSound` 改回等待），基线腿绿，源文件还原**逐字节一致**（sha256 核对）。④ 新增防御性守卫：`InitSound` 的重新分配只在**没有设备打开**时执行（防「委托关闭 + 紧接着的 InitSound」把回环缓冲从回调线程脚下拿走）；`s_BufferSize` 在「设备仍打开」时被刷新这一既有面**未动**（复用分支同样存在，属阶段 3 登记的观察项）。⑤ 门禁：`ctest -LE perf` **45 → 46**，新锁 `audio_probe_gate_test` 绿。 |
| r3 | 2026-11-19 | **本机端到端 A/B（决定性取证）**。本机探测只需约 76 ms（`[audio-gate] probe started` → `probe thread idle` = 76 ms），结构上复现不出 A1，故新增一个**纯诊断**开关：`FCEUX11_TEST_AUDIO_OPEN_DELAY_MS=<ms>` 让每次设备探测至少耗时该值（`sdl-sound.cpp` 的 `AudioProbeTestDelayMs()`，只读一次环境变量，未设时零成本）。配合 `FCEUX11_AUDIO_TRACE=1`（门控事件 + 单调时钟毫秒戳）。脚本 `.zcode/soundprobe/a1check.ps1`（gitignored）：加载 ROM → **在探测仍在飞时**触发 Close ROM（Ctrl+C，先 `AppActivate` + 校验前台窗口，失败则 PostMessage 直投 WM_KEYDOWN）→ 以 `SendMessageTimeout(WM_NULL)` 度量**窗口连续不可响应的时长**。读数：<br>• **修复前**（把 `KillSound` 改回「`requestClose()` 后再 `waitIdle()`」的 M5 变异体，探测 15 s，close 落在 +1115 ms）⇒ `worst_window_blocked_ms = 13,981`；<br>• **修复后**（同一脚本、同一 ROM、同一机器，close 落在 +1085 ms）⇒ `worst_window_blocked_ms = 0`；<br>• **常规路径**（无延迟，close 落在 +1154 ms、探测已在 +76 ms 结束）⇒ `close performed inline`、阻塞 0 ms，与审计报告 §四 的基线一致（不回归）。<br>⇒ 症状（窗口冻结 = 音视频一起停）**在本机被完整复现并消除**，不再依赖「业主机器探测慢」这一外部条件。⚠️ 该开关只影响探测时长，不改变任何音频行为；变异体已删除，源码以 sha256 核对还原。 |
| r4 | 2026-11-19 | **阶段 2 落地（F1 残留面：设备已开但停排空）**。① 新增 `src/drivers/Qt/audio_sink_wait.h`（纯 std）：`SinkWaitGuard` 把「环满怎么办」从三处复制的 `while (full) { SDL_Delay(1); if (++waitCount > 1000) return; }` 换成**有界等待 + 停滞闩锁 + 丢弃**——一个预算（默认 16 ms ≈ 一帧）内等到空位就照常写，超预算则**闩锁**并让后续调用**零等待**丢弃，环重新排空时自动解锁；新设备打开时复位。② 三处写路径（underflow / 1:1 / overflow）改为调用 `AudioWaitForRingRoom()`，**语义其余不变**（仍是解锁后等待、仍以 `return` 丢弃本帧剩余样本），停滞日志由「每帧一条」变为「每个 episode 一条」，文案保留 `Sound sink is not draining` 以便审计报告的判据继续成立。③ `GetWriteSound()` 改走 `RingFree()` 夹逼（旧式 `size - in` 在 `s_BufferIn > s_BufferSize` 时下溢到 ~4 GiB，GBA 路径直接喂给 `resize()` ⇒ OOM abort）；同时把 underflow 模式的重复写入循环收尾处 `s_BufferIn` 夹到 `s_BufferSize`，从结构上消除「计数越过环尾」。④ 新锁 `audio_sink_wait_test` 5 腿（永不排空 ⇒ 一次有界等待后零等待 / 排空后解锁且 episode 计数 / 健康路径零 sleep / 抖动静默解决 / `RingFree` 六组含 `0xFFFFFFFF`），**ctest 46 → 47**；变异 **M1–M4 逐腿转红**（删闩锁 / 去掉预算 / `RingFree` 回退 / 不清理闩锁），基线绿、头文件还原逐字节一致。⚠️ **未覆盖**：端到端「设备打开后真的失去端点」的模拟（需要能拔掉的端点或 SDL 设备移除注入），该场景仍只有代码级论据 + 本锁的机制保证。 |
| r5 | 2026-11-19 | **阶段 2 端到端 A/B（闭合 r4 的覆盖缺口）**。新增诊断开关 `FCEUX11_TEST_AUDIO_STALL=1`（`sdl-sound.cpp` 的 `AudioProbeTestStall()`）：设备打开后立刻 `SDL_PauseAudio(1)`，**设备仍算打开、回调不再消费** —— 正是「端点消失」后的状态。另加：`FCEUX11_AUDIO_TRACE` 置位时**每丢一帧打印一行**（`[audio-sink] dropped one frame ...`），该行数即**帧计数**；未置位时每个 episode 只打一条 `Sound sink is not draining` 警告。脚本 `.zcode/soundprobe/a2stall.ps1`（gitignored）在 14 s 窗口内统计行数。读数：<br>• **修复前**（`kDefaultBudgetMs = 1000` 且去掉闩锁的变异体）⇒ `dropped_frames = 7` / 14 s ≈ **0.5 fps**；<br>• **修复后** ⇒ `dropped_frames = 840` / 14 s ≈ **60 fps**，窗口阻塞 0 ms。<br>⇒ 「音视频异常缓慢」的**持续变慢**形状在本机被复现并消除（0.5 fps → 60 fps，同一脚本/同一 ROM）。⚠️ **度量口径**：`WriteSound` 阻塞的是**模拟线程**，不是 GUI 线程 ⇒ 窗口响应性测不出该缺陷，必须用帧计数（本开关提供的丢帧行数）。**常规播放不回归**：15 s 持续播放、`stall_messages = 0`、窗口延迟 4 ms、探测 78 ms 完成。 |
