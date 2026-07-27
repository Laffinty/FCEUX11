# KagamiQA P4-bridge: newppu=1 Headless 初始化修复

> **性质**：Bug fix — 新 PPU 在 headless 下的 ppudead 路径缺少 VBlank/NMI 触发。
> **范围**：`src/ppu_rendering.cpp`（3 行改动）+ `tests/blargg_runner.cpp`（2 行改动）
> **依赖**：无（独立于其他 P4 工作）

---

## 一、根因

### 1.1 症状

`blargg_runner` 设置 `newppu=1` + `normalscanlines=241` 后，`fceu11::Emulate()` 调用的 `FCEUX_PPU_Loop` 导致 runner 挂起（无 `BLARGG_RESULT` 输出，进程不返回）。

### 1.2 代码路径

```
blargg_runner::run_one_rom()
  ├─ core_init()        → fceu11::Initialize()
  ├─ load_rom()         → LoadGameVirtual() → PowerNES() → FCEUPPU_Power()
  │                        └─ FCEUPPU_Reset(): new_ppu_reset = true, ppudead = 2
  ├─ newppu = 1         ← 设置点
  └─ emulate_n(frames)  → fceu11::Emulate() × N
       └─ FCEUPPU_Loop  → FCEUX_PPU_Loop (newppu 分支)
```

### 1.3 新旧 PPU 的 ppudead 路径差异

**旧 PPU** (`FCEUPPU_Loop:1179-1184`)：
```cpp
if (ppudead) {
    X6502_Run(256 + 85);        // post-render scanline
    PPU_status |= 0x80;          // ← 设置 VBL flag
    // ...
    if (VBlankON) TriggerNMI();  // ← 触发 NMI
    // ...
    --ppudead;
    goto finish;
}
```

**新 PPU** (`FCEUX_PPU_Loop:1523-1537`)：
```cpp
if (ppudead) {
    ppur.status.sl = 241;
    runppu(20 * kLineTime);      // VBlank scanlines
    ppur.status.sl = 0;
    runppu(242 * kLineTime);     // 渲染 scanlines
    --ppudead;
    goto finish;
    // ❌ 没有 PPU_status |= 0x80
    // ❌ 没有 TriggerNMI()
}
```

**后果**：blargg ROM 在 `ppudead` 期间（前 2 帧）收不到 NMI。ROM 的启动代码通常依赖 NMI 来进入主测试循环。如果 ROM 在 NMI 到达前处于 `wait_for_nmi` 自旋，而新 PPU 的 ppudead 路径只推进 CPU/PPU 周期不生成 NMI，CPU 在 `X6502_Run(82522)` 中空转完 82522 周期后 ROM 仍停在自旋中——下一个 `Emulate()` 调用同理。**实际上不会死锁**（X6502_Run 正常返回），但前 2 帧 ROM 没有任何 NMI，如果 ROM 的逻辑要求 NMI 必须在第一帧到达，测试序列无法启动。

### 1.4 修复策略

在新 PPU 的 `ppudead` 路径末尾（`goto finish` 之前）补上 VBL flag 置位和 NMI 触发，与旧 PPU 保持行为一致：

```cpp
if (ppudead) {
    ppur.status.sl = 241;
    runppu(20 * kLineTime);
    ppur.status.sl = 0;
    runppu(242 * kLineTime);

    PPU_status |= 0x80;          // ← 新增
    ppuphase = PPUPHASE_VBL;     // ← 新增
    if (VBlankON) TriggerNMI();  // ← 新增

    --ppudead;
    goto finish;
}
```

**为什么这是正确修复**：
- 旧 PPU 的 ppudead 有 VBL+NMI，新 PPU 没有——这是行为不一致的 bug，不是设计意图
- ppudead 是"开机后 PPU 稳定期"的模拟，真机在这期间确实有 VBlank 和 NMI
- 三行代码不改变新 PPU 的核心渲染逻辑，只补齐被遗漏的框架信号

### 1.5 同时修改 blargg_runner.cpp

在 `LoadGame` 成功后、`emulate_n` 之前设置：
```cpp
newppu = 1;
normalscanlines = 241;
```

上一次尝试设置此二值后 runner 挂起——根因即 1.3 所述，修复后应正常。

---

## 二、改动文件

| 文件 | 改动 | 行数 |
|------|------|------|
| `src/ppu_rendering.cpp` | ppudead 路径补 VBL+NMI | +3 |
| `tests/blargg_runner.cpp` | LoadGame 后设 newppu=1 | +2 |

## 三、验证

1. `cmake --build build --target fceux11_blargg_runner`
2. `fceux11_blargg_runner.exe --rom fixtures/blargg/ppu/ppu_vbl_nmi.nes --frames 300`
   - 预期：`status=PASS`（P4-1 修复在此路径下生效）
3. `fceux11_blargg_runner.exe --manifest fixtures/blargg_manifest.json`
   - 预期：17 PASS 保持，ppu_vbl_nmi 从 FAIL→PASS
4. `ctest --test-dir build` 全绿（Oracle A 无回归）

## 四、回滚方案

若修复后 runner 仍然挂起：删除 `blargg_runner.cpp` 中的 `newppu=1` 两行，恢复到仅旧 PPU 可用状态。`ppu_rendering.cpp` 的 ppudead 修复保留（行为与旧 PPU 一致，无副作用）。

## 五、与 P4/P5 的关系

- **P4-1**：此 bridge 完成后，P4-1 的 VBL-cycle-1 修复立即可在 headless 下验证
- **P4-3**：此后所有 CPU/PPU/APU 精度修复均可使用 `--baseline` + `--save-baseline` 进行 transition_matrix 差分验证
- **P5**：此 bridge 消灭了 newppu=1 在 headless 下的最后一个障碍——P5 开工门禁"收益预期重估"可以用实际 Oracle B 数据支撑
