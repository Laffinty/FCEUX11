# FCEUX11 — GBA OBJ 瓦片越界 panic 修复计划

> **状态：已实施并收口（2026-11-21，业主拍板放行当日）**——见 §六 r2；主计划 §十 r5
> 立项依据：批次 1 个案 B（0004+0015）取证，主计划 `docs/plans/FCEUX11-GBA兼容性批次1-测试分析与优化计划-2026-11-21.md` §5.3 / §十 r4
> 方法论：`docs/tech/gba-compat-methodology.md`；修复纪律 = 主计划 §六（通用机制修复、锁测试双断言、变异双向可红、全门禁）
> ⚠️ 本修属 vendor 改动：`src/rust/crates/gba-core/` 下，`ATTRIBUTION.md` §3.2 须新增条目

---

## §〇 范围

| 项 | 内容 |
|---|---|
| 修 | `layer_obj.rs` 4 处 OBJ 瓦片读取的越界 panic——按硬件忠实语义化 |
| 不修 | 0004/0015 游戏侧任何逻辑；0015「声音异响」伴生症状（画面根因修复后业主复测再定） |
| 波及面 | 任何 `tile_number + 精灵跨度` 越过 OBJ 窗口（tile 1023 界）的精灵——批次内已实证 2 张，潜在影响面大 |
| 验证卡 | 0004（1D tile=1004）+ 0015（2D tile=992），FFI 2500 帧须越过 panic 点 |

## §一 事实底座（取证冻结，坐标经主线程抽查确认）

1. **缺陷坐标**：`src/rust/crates/gba-core/src/cpu/hardware/lcd/layers/layer_obj.rs`
   **:421（8bpp）/ :445（4bpp）/ :654（affine 8bpp）/ :669（affine 4bpp）**——
   `memory.video_ram[0x10000 + tile_data_offset as usize]` 直索引无防护；
   VRAM = 96KB 平铺数组（`lcd/memory.rs:115`，len = 98304）。
2. **触发形态**（两卡三方一致：FFI / cartprobe / 帧内细采样）：
   - 0004：1D 映射 64×32 精灵 tile=1004 → 跨度 1004..1035 越过 1023 → index 98304 越界 panic，**帧 77**（显示点亮首帧）进程 abort；
   - 0015：2D 映射 64×32 tile=992 → 跨 charblock 行同界 → **帧 204** panic（白屏 = 游戏自写 DISPCNT=0x0040 层全关的 boot 常态——非 Forced Blank、palette 非空，主循环活着，flash boot 初始化在跑）。
3. **硬件语义**（GBATEK 快照）：attr2 tile 域 0-1023、OBJ 窗口 32KB
   （0x06010000-0x06017FFF）、**0x06018000+ "Not used"（快照 :439）** ⇒ 真机越窗读 = 垃圾值不崩溃。
4. **家族史**：bit15 帧缓冲 panic（59f95c3 修）→ v2.0.3 S2-a OAM attr panic（映射 `ObjMode::Disabled`）→ **本例第三例**——共同形态：硬件合法值 → vendor panic → release `panic="abort"` 进程死亡（用户所见「卡死/白屏」）。
5. 排除项：两卡 SWI 未实现记录器零命中；0004 静态 Halt/Stop 各 1、0015 Stop×8（0x06×3 = Div 非停机，快照 :88120/:88495/:88550 核对）——均与症状无关。

## §二 修复设计

1. **语义钉定先于写码（纪律 4）**：越窗读的真机值 = ？——三候选（open bus / 0x00 / 镜像），以 GBATEK 快照 VRAM 节 + Gekkio/tonc 交叉钉定；**取不到权威出处即停工记录**，不得凭记忆写。若钉定为「值无关紧要」（后续绘制覆盖）也须有出处。
2. **改动面**：4 处读取统一收敛到**一个带边界语义的助手函数**（一处实现、四处复用，防复发）；越界分支按 §二.1 钉定语义返回；不动任何非 OBJ 路径。
3. 变异守卫：把 panic 改回去必须红。
4. `ATTRIBUTION.md` §3.2 新增条目，**标注「上游 panic 依旧存在，re-vendor 会静默带回」**（与 §3.2.12 同级警示）。

## §三 锁测试与变异

1. 最小夹具（注册进 G2 守卫集合）：attr2 tile=1004（1D）与 tile=992（2D）的 64×32 精灵各一；断言渲染不 panic + 越窗像素语义（按 §二.1 结果）；期望值按字段拼装，不手写字面量（纪律 5）。
2. 变异双向：还原 panic ⇒ 红；**只修 :445 一处 ⇒ 其余三处各自的夹具腿红**（防「修一处漏三处」）。
3. 假阳性防御：断言的通过原因与被测路径同源（纪律 3/12；0008 路③ 两次自我否证先例）。

## §四 验收门禁

1. 两卡 FFI 2500 帧**越过 panic 点**（0004 帧 77 / 0015 帧 204 后继续运行，画面推进读数）。
2. Rust 全套：root gba feature / gba-core / debug / media / formats / utils / lua + 双态 `cargo check`。
3. LTCG 重链（`-j 1`、勿中途取消）后 `ctest -LE perf` 全绿（计数以 `ctest -N -LE perf` 为准，现值 49）。
4. 路② 补课：mGBA / VBA-M 对 AKRJ（0004）/ AFPJ（0015）的 override 与已知状态检索（引用到行）。
5. 业主 GUI 复测两卡 + `.srm` 读写；CHANGELOG `[Unreleased]` 登记（已知限制收口）。

## §五 止损

- 语义钉定失败（无权威出处）⇒ 停工记录转业主裁决（登记已知限制 vs 显式标注的保守语义）。
- 工作量预估 1-2 会话（修复 + 锁 + 门禁）；两轮会话未收口 ⇒ 降级登记，不硬拖。

## §六 变更记录

- **r1（2026-11-21）**：立项草案。依据 = 批次 1 个案 B 取证（坐标/触发形态/硬件语义引注见 §一）；**待业主拍板后动代码**；拍板即按 §二.1 → §三 → §四 顺序执行。
- **r2（2026-11-21，同日实施收口）**：业主拍板放行后按 §二→§三→§四 全链落地。① **语义钉定**：gbadoc VRAM 节一手原文「Bytes 0x06010000 - 0x06017FFF is mirrored from 0x06018000 - 0x0601FFFF」（GBATEK 快照 :439 只标 Not used 无读语义）⇒ 镜像回绕 `& 0x7FFF`；Gekkio 无 GBA 内存映射公开文档（其文档以 DMG 为主），gbadoc 为 gbadev 社区维护的现代权威替代。② **修复**：`layer_obj.rs` 新增 `obj_tile_data` 助手（模块常量 `OBJ_TILE_DATA_BASE/WINDOW` + 引注），四处调用点收敛；提交 7d9c59d。③ **锁**：四腿夹具注册进 G2（`obj_tile_span_1d_{4,8}bpp_reads_the_window_alias` / `obj_window_span_{2d_4,1d_8}bpp_masks_through_the_alias`），期望值全手工推导；实施期夹具侧两坑（与核心无关，如实登记）：WINOUT 写入地址曾误植 `0x0400_04A`（=0x4004A，少一位）落进写匹配黑洞静默消失、8bpp 瓦片字节即调色板指数（铺 0x11 = 指数 17 黑屏）。④ **变异三腿全闭**：还原 panic 4/4 红（panic 落在变异行）；透明语义（越窗读零）4/4 红走像素断言（区分镜像与透明两类假实现）；只留 4bpp 渲染点用助手 1 绿 3 红在各自被退回的读取点（:448 等）。⑤ **门禁**：Rust 全套绿（root 306+20 / gba-core 249 / debug 94 / formats 158 / utils 51 / lua 54 / media 52 + 双态 check）；ctest -LE perf 全绿（读数见主计划 §十 r5）。⑥ **FFI 验证**：两卡 2500 帧全过——0004 首变帧 76（=修前死亡点）1773 个变化帧、帧 100 实拍 = 开场小鸟场景完整渲染；0015 越过修前死亡帧 204，帧 350 实拍 = VAILL logo（白屏按真机序列过渡）。⑦ **路② 补课**：两卡在 mGBA 社区兼容清单均 Works、无专门 issue（MGBAWii WikiTemp / RetroArch mGBA 清单）；mGBA 博客「Holy Grail bug」史提及 Kururin 族早年启动时序 bug——与本次 OBJ panic 不同族，如实区分。⑧ ATTRIBUTION §3.2.18 + 计数刷新（34 处/8 文件）；CHANGELOG [Unreleased] 新增 Fixed 段。
