# B · 风险登记册与缓解策略

> 本文档跟踪 vNESU11 项目所有已知风险，每条标明：**严重度 / 概率 / 触发条件 / 缓解 / 责任人 / 状态**。

---

## 1. 风险评分标准

| 严重度 | 定义 |
|--------|------|
| 🔴 高 | 阻塞 v2.0 发布；必须修复或绕开 |
| 🟠 中 | 影响质量但不阻塞；可由缓解措施控制 |
| 🟡 低 | 不影响主要功能；记录但不阻塞 |

| 概率 | 定义 |
|------|------|
| 高 | > 50% 概率触发 |
| 中 | 20-50% |
| 低 | < 20% |

---

## 2. 风险清单

### R-001 · Savestate 字节布局漂移

| 项 | 值 |
|----|----|
| **严重度** | 🔴 高 |
| **概率** | 中 |
| **阶段** | Phase 0, Phase 7 |
| **触发条件** | Rust 端 `CpuRegsLayout` 字段顺序、对齐、填充与 C++ `x6502struct.h` 不一致 |
| **影响** | 所有现有玩家存档、TAS movie 失效 |
| **缓解** | 1. `static_assert` 双侧（Rust + C++）<br>2. CI 字节对比测试<br>3. Golden savestate round-trip 强制验证 |
| **检测** | `cargo test -p vnesu11 layout_check` + C++ `gtest` |
| **责任人** | Phase 0 owner |
| **状态** | 设计完成（Phase 0 实施中） |

### R-002 · PPU 渲染性能回归

| 项 | 值 |
|----|----|
| **严重度** | � 高 |
| **概率** | 中 |
| **阶段** | Phase 3, Phase 7 |
| **触发条件** | Rust 端 PPU 渲染 hot path 比 C++ 慢 ≥ 10% |
| **影响** | 帧率下降（60 FPS → 45 FPS），用户感知卡顿 |
| **缓解** | 1. Phase 3 末必须 ≤ baseline（不达标则 profile + 优化）<br>2. 备选 SIMD tile fetch<br>3. 备选 dynarec（最后手段） |
| **检测** | `cargo bench -p vnesu11 ppu_dot` + frame-level benchmark |
| **责任人** | Phase 3 owner |
| **状态** | 监控中 |

### R-003 · Mapper thunk 性能失控

| 项 | 值 |
|----|----|
| **严重度** | � 中 |
| **概率** | 低 |
| **阶段** | Phase 5 |
| **触发条件** | mapper FFI thunk 调用开销 > 5% CPU cycles |
| **影响** | 帧时间增加 |
| **缓解** | 1. Benchmark 早期发现<br>2. 备选 `&mut dyn`（更轻量）<br>3. mapper batch read（多字节合并 thunk） |
| **检测** | callgrind + per-mapper profiling |
| **责任人** | Phase 5 owner |
| **状态** | 待监控 |

### R-004 · Lua 内存钩子 hot-path 退化

| 项 | 值 |
|----|----|
| **严重度** | 🟡 低 |
| **概率** | 低 |
| **阶段** | Phase 1, Phase 5 |
| **触发条件** | Lua 已注册时，每次 RdMem/WrMem 多一次 FFI 调用 |
| **影响** | 帧时间 +1-3% |
| **缓解** | `AtomicBool` flag 守卫；Lua 未注册场景零开销 |
| **检测** | Lua 启用/禁用场景对比 benchmark |
| **责任人** | Phase 5 owner |
| **状态** | 设计完成 |

### R-005 · MSVC LTCG c2.dll 物化崩溃复发

| 项 | 值 |
|----|----|
| **严重度** | 🟠 中 |
| **概率** | 中 |
| **阶段** | 全程 |
| **触发条件** | Rust LTO 编译大常量表（如 sprite LUT）时崩溃 |
| **影响** | Release 构建失败 |
| **缓解** | 1. sprite LUT 用 `LazyLock`（不静态分配）<br>2. 大表拆子 crate<br>3. 备选关闭 Rust LTO |
| **检测** | `cargo build --release` + cmake Release |
| **责任人** | Phase 0 / Phase 3 owner |
| **状态** | 设计缓解（LazyLock） |

### R-006 · blargg 测试出现新 FAIL

| 项 | 值 |
|----|----|
| **严重度** | 🔴 高 |
| **概率** | 中 |
| **阶段** | Phase 1, 3, 4, 7 |
| **触发条件** | Rust 实现某个边界条件（如 IRQ 时序、sprite 0 hit dot、palette 镜像） |
| **影响** | v2.0 不达标，阻塞发布 |
| **缓解** | 1. Shadow run 早期捕获<br>2. 单测覆盖（每个边界 case）<br>3. Fallback `VNESU11_CORE=OFF`（v2.1 前保留） |
| **检测** | kagami-qa-runner Oracle B |
| **责任人** | 全体 phase owner |
| **状态** | 监控中 |

### R-007 · TAS movie round-trip 不一致

| 项 | 值 |
|----|----|
| **严重度** | 🔴 高 |
| **概率** | 中 |
| **阶段** | Phase 6, Phase 7 |
| **触发条件** | joypad 输入时序差 1 周期、IRQ 采样偏差 |
| **影响** | TAS 工具链失效 |
| **缓解** | 1. Phase 6 末强制 TAS movie 录放测试<br>2. shadow run diff 包含 joypad state |
| **检测** | TAS movie round-trip test |
| **责任人** | Phase 6 owner |
| **状态** | 待监控 |

### R-008 · 工期低估

| 项 | 值 |
|----|----|
| **严重度** | 🟠 中 |
| **概率** | 高 |
| **阶段** | 全程 |
| **触发条件** | Phase 3（PPU）实际工期 9 周 vs 预估 6 周 |
| **影响** | v2.0 延期 |
| **缓解** | 1. Phase 3 已预留 1.5x buffer（6 周）<br>2. Phase 1 与 Phase 3 可并行（不同工程师）<br>3. Phase 8 cleanup 可压缩 |
| **检测** | 每个 phase 末回顾 |
| **责任人** | PM |
| **状态** | 已知风险（已缓冲） |

### R-009 · 调试器符号 API 破坏

| 项 | 值 |
|----|----|
| **严重度** | 🟠 中 |
| **概率** | 低 |
| **阶段** | Phase 1, Phase 6 |
| **触发条件** | `FCEUI_DebugGetByte` 等函数未正确转 vNESU11 调用 |
| **影响** | 断点、内存查看失效 |
| **缓解** | 1. 兼容垫片（Phase 6）保证零改动<br>2. Phase 8 直接迁移 |
| **检测** | 调试器手动测试 + 自动化断点测试 |
| **责任人** | Phase 6 owner |
| **状态** | 设计完成 |

### R-010 · Rust ↔ C++ ABI 不匹配

| 项 | 值 |
|----|----|
| **严重度** | 🟠 中 |
| **概率** | 低 |
| **阶段** | Phase 0 |
| **触发条件** | Windows x64 ABI 误用（如 struct 返回值、HFA 类型） |
| **影响** | 链接错 / 运行时崩溃 |
| **缓解** | 1. 所有 FFI 用 `#[repr(C)]`<br>2. thunk 用 `extern "C"`<br>3. 早期集成测试 |
| **检测** | cmake Release build + 运行 SMB1 |
| **责任人** | Phase 0 owner |
| **状态** | 设计完成 |

### R-011 · cbindgen 输出与 C++ 风格冲突

| 项 | 值 |
|----|----|
| **严重度** | 🟡 低 |
| **概率** | 中 |
| **阶段** | Phase 0 |
| **触发条件** | cbindgen 默认输出与项目 include 风格不一致 |
| **影响** | C++ 编译失败或风格不统一 |
| **缓解** | 1. cbindgen.toml 显式配置<br>2. 早期 review vnesu11_ffi.h 样本 |
| **检测** | cmake 构建 |
| **责任人** | Phase 0 owner |
| **状态** | 待验证 |

### R-012 · 工具链 / 依赖更新

| 项 | 值 |
|----|----|
| **严重度** | 🟡 低 |
| **概率** | 低 |
| **阶段** | 全程 |
| **触发条件** | Rust toolchain / Cargo dependency 版本漂移 |
| **影响** | 构建不可重现 |
| **缓解** | 1. `Cargo.lock` 提交<br>2. MSVC 锁版本（与现有策略一致）<br>3. CI matrix 锁定 |
| **检测** | CI |
| **责任人** | 全程 |
| **状态** | 已缓解 |

### R-013 · 文档遗漏

| 项 | 值 |
|----|----|
| **严重度** | 🟡 低 |
| **概率** | 中 |
| **阶段** | Phase 7, 8 |
| **触发条件** | BuildGuide / ChangeLog 未及时更新 |
| **影响** | 用户构建失败 |
| **缓解** | Phase 7 末强制 review |
| **检测** | docs review checklist |
| **责任人** | PM |
| **状态** | 待执行 |

---

## 2b. 审计新增风险（2026-08-10，见 AUDIT_20260810.md）

### R-014 · Savestate SFORMAT tag 契约漏项（S2）

| 项 | 值 |
|----|----|
| **严重度** | 🔴 高 |
| **概率** | 中 |
| **阶段** | Phase 0 |
| **触发条件** | `savestate_tags.md` 未穷举 `SFCPU`/`SFCPUC`/`FCEU_NEWPPU_STATEINFO` 等全部 tag |
| **影响** | savestate 加载/保存字节漂移 |
| **缓解** | 逐行对照 `state.cpp`/`core_state.cpp`；golden round-trip 兜底 |
| **检测** | `crates/vnesu11/tests/savestate_roundtrip.rs` 逐 tag 对比 |
| **责任人** | Phase 0 owner |
| **状态** | 修订后新增 |

### R-015 · 时序模型复刻偏差（S3）

| 项 | 值 |
|----|----|
| **严重度** | 🔴 高 |
| **概率** | 中 |
| **阶段** | Phase 1, 3 |
| **触发条件** | `X6502_Run` 魔法常数（256/85/69/16）抄录错误或段结构理解偏差 |
| **影响** | 逐帧不等价，shadow run 全红 |
| **缓解** | 逐行对照 `ppu_rendering.cpp`；段边界 shadow diff |
| **检测** | shadow run 帧级三级对比（S10） |
| **责任人** | Phase 1/3 owner |
| **状态** | 修订后新增（决策 A，ADR-008） |

### R-016 · newppu=0 movie 兼容回退失效（S5）

| 项 | 值 |
|----|----|
| **严重度** | 🟠 中 |
| **概率** | 低 |
| **阶段** | Phase 6, 7 |
| **触发条件** | 误删 C++ 旧 PPU 路径，或 vNESU11 错误接管 newppu=0 |
| **影响** | PPUflag=0 的 TAS movie 无法播放 |
| **缓解** | 旧 PPU 保留（ADR-009）；Phase 6 录 old-PPU movie round-trip |
| **检测** | old-PPU movie 录放测试 |
| **责任人** | Phase 6 owner |
| **状态** | 修订后新增 |

### R-017 · NSF/FDS/VS 覆盖缺失（S6）

| 项 | 值 |
|----|----|
| **严重度** | 🟠 中 |
| **概率** | 中 |
| **阶段** | Phase 3, 4, 5 |
| **触发条件** | 只实现 iNES 路径，FDS/NSF/VS 功能失效 |
| **影响** | FCEUX11 三类系统功能全废 |
| **缓解** | 系统类型矩阵（00_overview §2.4）；NSF 空转 PPU（Phase 3）；FDS 外部 IRQ（Phase 4） |
| **检测** | 每类系统测试 ROM/文件 |
| **责任人** | 对应 phase owner |
| **状态** | 修订后新增 |

### R-018 · RAM 随机源逐位不等价（S7）

| 项 | 值 |
|----|----|
| **严重度** | 🔴 高 |
| **概率** | 低 |
| **阶段** | Phase 2 |
| **触发条件** | `splitmix64`/`xoroshiro128plus` 复刻偏差，或 `RAMInitOption` 语义错 |
| **影响** | shadow run 第一帧即 diff |
| **缓解** | 固定 seed golden 输出测试锁定；禁用"看起来一样"的实现 |
| **检测** | `crates/vnesu11/tests/ram_rng_tests.rs` |
| **责任人** | Phase 2 owner |
| **状态** | 修订后新增 |

---

## 3. 风险汇总

| ID | 严重度 | 概率 | 阶段 | 状态 |
|----|--------|------|------|------|
| R-001 | 🔴 | 中 | 0, 7 | 设计完成 |
| R-002 | 🔴 | 中 | 3, 7 | 监控中 |
| R-003 | 🟠 | 低 | 5 | 待监控 |
| R-004 | 🟡 | 低 | 1, 5 | 设计完成 |
| R-005 | 🟠 | 中 | 全 | 设计缓解 |
| R-006 | 🔴 | 中 | 1-7 | 监控中 |
| R-007 | 🔴 | 中 | 6, 7 | 待监控 |
| R-008 | 🟠 | 高 | 全 | 已缓冲 |
| R-009 | 🟠 | 低 | 1, 6 | 设计完成 |
| R-010 | 🟠 | 低 | 0 | 设计完成 |
| R-011 | 🟡 | 中 | 0 | 待验证 |
| R-012 | 🟡 | 低 | 全 | 已缓解 |
| R-013 | 🟡 | 中 | 7, 8 | 待执行 |
| **R-014** | 🔴 | 中 | 0 | 修订新增（S2） |
| **R-015** | 🔴 | 中 | 1, 3 | **已定：决策 A（2026-08-10，见 DECISIONS_S3_S5_analysis.md）** |
| **R-016** | 🟠 | 低 | 6, 7 | 修订新增（S5，选项③ 已定） |
| **R-017** | 🟠 | 中 | 3-5 | 修订新增（S6） |
| **R-018** | 🔴 | 低 | 2 | 修订新增（S7） |

**🔴 高风险数**：6（R-001, R-002, R-006, R-007, R-014, R-015, R-018 实际 7 个）

---

## 4. 风险应对策略

### 4.1 阻塞级风险（🔴）的"必须"条件

- **R-001 / R-006 / R-007 / R-014 / R-015 / R-018**：每个 phase DoD 必须包含
  shadow run / golden round-trip 测试
- **R-002**：Phase 3 末 benchmark 不达标**不进入 Phase 4**，必须 profile + 优化
- **R-015**（时序模型）：决策 A（budget 复刻）必须逐行对照 `ppu_rendering.cpp`，
  禁止重推导魔法常数

### 4.2 兜底（fallback）

任何 🔴 风险触发后的兜底：

1. **立即修复**：48 小时内 hotfix
2. **临时回退**：发布 v1.18 hotfix，`VNESU11_CORE=OFF` 默认
3. **延期**：保留 wip_v2.0 分支继续修，下次小版本再切

### 4.3 风险沟通

- 每周 phase 进度更新包含风险状态
- 任何 🔴 风险升级 = 立即通知 PM + 架构组
- 月度 review：所有风险重新评估

---

## 5. 风险登记流程

新增风险：

1. 在本文档添加条目（含完整字段）
2. 评估严重度 / 概率
3. 设计缓解措施
4. 添加检测方法
5. 更新风险汇总表
6. 通知相关 phase owner

关闭风险：

1. 标注"已关闭"+ 关闭日期
2. 移到"已关闭风险"附录
3. 保留作为历史参考
