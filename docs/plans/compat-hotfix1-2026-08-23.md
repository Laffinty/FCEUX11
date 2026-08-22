# 兼容性优化计划 hotfix1（TESTNES 批量扫描）

- 日期：2026-08-23
- 状态：草案（扫描已完成，修复待实施）

## 1. 背景
在修复 9ad00a0151ee4e76b77cc7328a78640f.nes 灰屏问题（提交 7be9c8f，mapper 修正传播）后，对 C:/Users/ikrx2/Desktop/TESTNES 目录下 123 个 .nes 样本做批量兼容性扫描，排查同类问题并制定 hotfix1 优化计划。

## 2. 扫描方法
- 工具：kagami_qa_cycle_trace（临时启用 FCEUX11_FRAME_PPM 导出末帧）+ Python 驱动脚本（build/sweep_testnes.py，结果 build/sweep/results*.json）。
- 流程：
  1. 解析每个 ROM 的 iNES 头（PRG/CHR/mapper/NES2/镜像）。
  2. 运行 30 帧，记录 CPU PC 轨迹（CSV）与第 30 帧画面（PPM）。
  3. 30 帧画面纯色者复测 120 帧、300 帧。
- 判定标准：
  - 加载失败：LoadGame 返回失败（进程退出码不等于 0）。
  - PC 异常：轨迹中 PC 越出 0x8000-0xFFFF 或卡死在固定值。
  - 画面异常：末帧画面颜色数 == 1（纯色）。

## 3. 扫描结果（123 个样本）
- 正常：118 个（30 帧内出画面，或 120 帧内出画面且 PC 健康）。
- 确认问题 4 个 + 待确认 1 个：

| ROM | mapper | 现象 | 300 帧颜色数 | 状态 |
| --- | --- | --- | --- | --- |
| 10083_西天取经.nes | 241 | 首条指令 PC=0x3434，113990 条轨迹全部卡死 | 1 | 同型灰屏（与 9ad00a01 相同签名） |
| 10021_魂斗罗3代.nes | 4 | 加载失败（LoadGame failed） | - | 加载被拒 |
| 10302_吞食天地2.nes | 4 | 加载失败（LoadGame failed） | - | 加载被拒 |
| 10004_雪人兄弟.nes | 1 (MMC1) | CPU 正常（705 个 PC）但画面恒为底色 | 1 | MMC1 渲染异常 |
| 10036_三国志2.nes | 19 | 300 帧仅 2 色 | 2 | 待人工确认 |

- 观察项：10078_地底探险.nes（mapper 0，300 帧 4 色，疑似等待输入/极慢启动）；其余 33 个 30 帧纯色样本均在 120 帧内出画面（慢启动）。

## 4. 根因分析
1. 10083（mapper 241，datalatch 板）：与 9ad00a01 灰屏完全同型——复位向量 0xFFFC/0xFFFD 读到 0x3434（开放总线值）而非 ROM 中的有效向量 0xFFD0，即复位向量读取时 PRG 映射未生效。该 ROM 头部声明 241（BNROM 类），修正库无对应 CRC 改写条目。待调试 LatchPower/M241Sync 的 setprg32 映射与 CPU 复位 dispatch 的时序；若映射正确则需确认该卡实际板型并补修正条目。
2. 10021/10302（mapper 4，MMC3）：iNES 1.0 非 2 的幂尺寸被向上取整（10021 CHR 18 块取整为 32 块；10302 PRG 40 块取整为 64 块），Rust 加载器 fceux11_rust_ines_load 的硬性长度校验（src/rust/crates/fceux11-formats/src/ines.rs:963/972）因取整后尺寸超过文件长度而拒绝加载。上游 FCEUX 对短文件容忍（按实际长度读取）。
3. 10004（mapper 1，128K PRG + 128K CHR）：CPU 运行正常但画面恒为单一底色，疑似 MMC1 CHR bank/镜像配置或 PPU 渲染路径问题，需 PPU 级调试。

## 5. hotfix1 修复方案
A. iNES 加载尺寸容错（修复 10021/10302）
   - fceux11-formats/src/ines.rs：rounded 尺寸超过文件长度时回退 raw 尺寸或按实际长度截断，保证合法海盗 ROM 可加载；C++ 侧以实际尺寸分配/拷贝，取整余量零填充。
   - 注意 MMC3 40 块 PRG 超出标准 32 块寻址范围，需验证 10302 实际使用的 bank；必要时仅映射前 32 块。
   - 回归：13 ROM golden 回归 + ctest 34 项全量。
B. mapper 241 复位向量映射（修复 10083）
   - 调试 datalatch LatchPower 的 PRG 映射在 CPU 复位 dispatch 前是否生效；
   - 若该卡实际为其它板型，则补充 hinfo/CRC 修正条目；
   - 将 10083 加入回归样本。
C. MMC1 渲染问题（修复 10004）
   - 抓取 PPU 寄存器与 CHR 页表状态，定位镜像/CHR bank/渲染路径问题；
   - 修复后加入回归样本。
D. 扫描工具收编
   - 将 build/sweep_testnes.py 的扫描逻辑整理为 tests/ 下正式工具（可选样本集），纳入 CI 冒烟。

## 6. 验收标准
- 123 个样本：加载成功、PC 不越界、120 帧内画面非纯色（等待输入类除外）。
- ctest 34/34 通过；9ad00a01 ROM 回归通过。

## 7. 风险与注意事项
- 尺寸容错需保持 round 语义仅对 iNES 1.0 生效，避免影响 NES 2.0 与 golden 样本。
- MMC3 40 块 PRG 修复后 10302 可能可进标题但中后期异常，需人工抽测。
- mapper 19（10036）与观察项（10078）需人工在 GUI 中确认。
