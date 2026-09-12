# Rust 与 C++ 共享状态的所有权规则

> 来源：FCEUX11 v2.1.2（2026-09-12）IRQ 风暴排障的沉淀。工程经验，不是硬件事实。

## 1. 事实

- Rust 6502 与 C++ 侧共享同一个 64 字节 X6502Layout（C++ 的 X6502 struct），由 src/cpu.cpp 的 static_assert 与 Rust 的 offset_of 双重固定。
- 每次 fceux11_cpu_run 类调用是：把 blob 拷进 Rust 的 FFI_CPU_STATE、跑指令、结束时整块写回。
- 但有些字段是 host（C++）拥有的，会在一次调用中途被 mapper/APU 直接改写：最典型的是 irq_low（X6502_IRQBegin 与 X6502_IRQEnd 直接写这块 blob）。


## 2. 陷阱与修复

整块写回会把调用开始时的快照盖回 host，抹掉调用中途 host 的修改。KIRA.nes 的案例：MMC3 中断应答写 E000 触发 X6502_IRQEnd 清位，随后这次 run 的整块写回又把 EXTERNAL 位恢复，CPU 无限重入 IRQ 处理程序，画面冻结。

当前实现（v2.1.2 起）：

- merge_post_run_irq_low(host_now, rust_now)：host 拥有的线位（EXTERNAL 与 EXTERNAL2 与 DPCM 与 FRAME）取 host 当前值，只有调度器自己管理的位（RESET 与 NMI2 与 NMI 与 TEMP）取 Rust 值。
- 三个 run 入口统一走 store_post_run_regs()。
- 调度边界仍有 sync_irq_from_host（读 host）与 sync_irq_to_host（只清本次消费的位），nmi_fresh 单独桥接。
- 新增跨边界共享字段时，必须明确所有权、加回写保护、并写反向验证的单测（把修复注释掉时测试必须失败）。


## 3. 影响面提醒

- 共享 blob 的字段变化会进入 savestate，从而影响 golden savestate 哈希；修复后需按仓库流程判定是否重生成基准，并说明差异根因。
- 像素与帧哈希门禁只覆盖渲染输出，不覆盖 CPU/PPU 内部状态；状态类 bug 往往要靠状态轨迹或 savestate 基准才能暴露。
- 相关代码：src/cpu.cpp（X6502_IRQBegin 与 X6502_IRQEnd）、src/rust/crates/fceux11-core/src/cpu/bus.rs（merge_post_run_irq_low 与 sync_irq_from_host/to_host）、src/rust/crates/fceux11-core/src/cpu/ffi.rs（store_post_run_regs）。

