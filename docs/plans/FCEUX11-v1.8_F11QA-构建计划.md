# FCEUX11 v1.8 构建计划 — KagamiQA → F11QA 重组、扁平清单与第三方 ROM 套件扩展

> **STATUS: PROPOSED**（v1.8 设计阶段；草案由 F11QA 工作组 2026-09-21 提出，待 §十一 决策点回签后进入施工期）
> **版本**：v1.8（草案 v0.1）
> **日期**：2026-09-21
> **分支**：`wip1.8`（用户已建）
> **前置**：v1.17 已合并至 main（`docs/history/plans/FCEUX11-1.17_计划.md` STATUS: COMPLETED）；冻结基线 `tests/fixtures/kagamiqa_baseline_frozen.json` 已落地
> **关联**：`docs/tech/KagamiQA.md`（将改名 `F11QA.md`）、`docs/history/plans/FCEUX11-1.16_KagamiQA-PLAN.md`、`docs/history/plans/FCEUX11-1.16_KagamiQA-P5-权威性构建计划.md`、`docs/history/plans/FCEUX11-Stage3-权威性迭代与通用化路线.md`、`docs/history/checklists/v2.0_removal_checklist.md`
> **路线图位置**：v1.15 完成 v1.x C++ 现代化；v1.16 完成 KagamiQA 双 Oracle 闭环；v1.17 完成 KagamiQA 统合 + 遗留精度收敛；**v1.8 = KagamiQA 改名 F11QA + 扁平清单改造 + 第三方 ROM 套件扩展（覆盖率从 180 → ~290 ROM）+ 许可证合规链**

---

## 〇、TL;DR

v1.8 的三项主任务与性质：

| # | 任务 | 性质 | 目标 |
|---|---|---|---|
| 1 | KagamiQA → **F11QA** 重命名 | 命名统一 | 测试体系名称与项目代号（FCEUX11 → F11）一致；去 "Kagami" 残留字样；CI workflow / docs / commit history 批量改名 |
| 2 | **扁平清单重组**（Oracle A/B 二分 → 1..N + kind 标签） | 架构调整 | 弃用 `oracle_type` 字段，引入 10 种 `kind` 标签；保留全部 47 个 v1.17 用例并重新编号为 `kgmqa-001 ~ kgmqa-NNN` |
| 3 | **第三方 ROM 套件扩展**（从 blargg 180 → 多作者 67 项 + 加 ROM 套件合计 ~290 ROM） | 覆盖率提升 | 引入 bisqwit / kevtris nestest / pinobatch Holy Mapperel & 240pee / Quietust / rainwarrior / tepples / AWJ / natt / N-K / Drag / TakuikaNinja / Sour / lidnariq / 3gengames / Rahsennor / Flubba 等权威套件 |

**一句话收束**：v1.8 把 v1.17 的「双 oracle 测试原型」升级为「**覆盖 120 用例 / ~290 ROM / 18 第三方作者 / 100% GPL-2 兼容**的扁平、可审计、可机器门禁的统一测试体系」，并以 **F11QA** 为正式名称对外。

**量化收敛目标**：

| 维度 | v1.17（基线） | v1.8 目标 |
|---|---|---|
| 用例总数 | 47（Oracle A 27 + Oracle B 20） | **120**（`kgmqa-001 ~ kgmqa-120`） |
| ROM 套件覆盖 | 1（blargg 180 ROM） | **18** 套件 / ~290 ROM |
| 第三方作者 | 1（blargg） | **17**（blargg / bisqwit / kevtris / pinobatch / Quietust / rainwarrior / tepples / AWJ / natt / N-K / Drag / TakuikaNinja / Sour / lidnariq / 3gengames / Rahsennor / Flubba） |
| 许可证明示 | 隐式 | **每条用例显式**（`license` 字段 + `license_manifest_check` 用例） |
| 类型分组 | Oracle A/B 二分 | **10 种 kind 标签**（flat tag，不是层级） |
| 命名 | KagamiQA | **F11QA**（Kagami 残留清零） |
| R4 gate 阈值 | `total ≥ 39`，`grade ∈ {A,B,C}` | `total == 120`，**`license_manifest_check` PASS** 为前置 |

---

## 一、命名变更：KagamiQA → F11QA

### 1.1 变更原因

"Kagami" 一词来源于项目早期 `docs/history/plans/FCEUX11-1.16_KagamiQA-PLAN.md` 的 "鏡"（kagami，日语"镜子"）隐喻——意指 QA 体系是模拟器的"镜子"。但项目主体已演进为 **FCEUX11 v1.x**，继续保留日语音译名字带来三个问题：

1. **品牌一致性**：项目代号 FCEUX11 / F11，QA 子系统叫 F11QA 才符合 "测试体系是项目不可分割部分" 的工程哲学。
2. **检索可发现性**：GitHub、CI artifact、commit 历史中 `KagamiQA` 字符串散落，新人维护成本高。
3. **许可证文档一致性**：v1.17 的 `kagamiqa_baseline_frozen.json`、`kagamiqa_migration_matrix.json`、`kagami-qa` Rust crate 等命名都需要统一收口。

### 1.2 改名范围（白名单，不波及历史）

| 类别 | 改动 | 说明 |
|---|---|---|
| **当前 docs** | `docs/tech/KagamiQA.md` → `docs/tech/F11QA.md`，内部文本 `KagamiQA` → `F11QA` | active 文档全面改名 |
| **CI workflow** | `.github/workflows/kagami-qa.yml` → `.github/workflows/f11qa.yml` | workflow 文件名 + workflow `name:` 字段 + job 名 + artifact 命名 |
| **GitHub Actions** | `name: KagamiQA` → `name: F11QA`；artifacts `kagamiqa-results` → `f11qa-results` | 触发 PR 评论的标签 |
| **tests.json 字段** | `suite_id: "kagamiqa-v1.17"` → `suite_id: "f11qa-v1.8"` | manifest 顶层版本号 |
| **冻结基线** | `tests/fixtures/kagamiqa_baseline_frozen.json` → `tests/fixtures/f11qa_baseline_frozen.json` | 字段名同步 |
| **Rust crate** | `src/rust/crates/kagami-qa/` → `src/rust/crates/f11qa/`；`kagami-qa-runner.exe` → `f11qa-runner.exe` | crate 改名 + 二进制改名 |
| **C ABI 桥** | `src/kagami_bridge.{h,cpp}` → `src/f11qa_bridge.{h,cpp}` | 头/源同步 |
| **kagami_qa_*** 二进制 | `kagami_qa_blargg_runner` / `kagami_qa_lua_runner` / `kagami_qa_rom_regression_runner` 等 → `f11qa_blargg_runner` / `f11qa_lua_runner` / `f11qa_rom_regression_runner` | runner 改名 |
| **kagami_direct_main.cpp** | `tests/kagami_direct_main.cpp` → `tests/f11qa_direct_main.cpp` | 直接模式入口改名 |
| **tests/kagami/** 目录 | → `tests/f11qa/`（C++ 测试源码落点） | 物理目录改名 |
| **kagami_*** 子目录 | 同步 | — |
| **CI 注释 / commit message** | 历史 commit 中的 `KagamiQA` **不改写**（保护 git 历史） | 但新 commit 一律 `F11QA` |

### 1.3 不改名的项

- **`docs/history/`** 下所有归档：保留 KagamiQA 历史命名，是 v1.16/v1.17 的**历史事实**，重写会破坏 commit 链接与考古价值
- **commit message 历史**：用 `git log --follow` 仍可追溯；CI 不需要历史重写
- **git tag `v1.16-KagamiQA-*` / `v1.17-KagamiQA-*`**：保留

### 1.4 重命名工具

```bash
# 仅在 wip1.8 分支执行；保护历史 docs/history/ 与 git history
# 1. 二进制名 + 文件名（git mv 保护历史）
find tests scripts src .github -type f \( -name "*kagami*" -o -name "*Kagami*" \) -print0 \
  | while IFS= read -r -d '' f; do
      new="$(echo "$f" | sed -E 's/[kK]agamiQA/F11QA/g; s/[kK]agami_qa/f11qa/g; s/[kK]agami-qa/f11qa/g; s/[kK]agamiqa/f11qa/g')"
      git mv "$f" "$new"
    done

# 2. 文本替换（仅 active 文件）
grep -rl --include='*.{h,cpp,rs,toml,yml,yaml,md,json,jsonc,ps1,sh,py}' \
     -E 'KagamiQA|kagamiqa|kagami_qa|kagami-qa' \
     .github tests scripts src docs/tech docs/plans | \
  xargs sed -i -E 's/KagamiQA/F11QA/g; s/kagamiqa/f11qa/g; s/kagami_qa/f11qa/g; s/kagami-qa/f11qa/g'

# 3. 排除历史与 .git
echo "docs/history/" >> .sed_exclude
echo ".git/" >> .sed_exclude
```

---

## 二、扁平清单设计（核心改造）

### 2.1 弃用 `oracle_type`，引入 `kind` 标签

**问题**：v1.17 把用例分成 Oracle A（CTest 回归）/ Oracle B（硬件 ROM）。这层划分在 P5 阶段有意义（双 oracle 差分测试），但 v1.17 合并为统一 runner 后，"Oracle A vs B" 的边界已经模糊（Lua 测试、smoke 测试、benchmark 测试都属于"非硬件 ROM"，却被混在 Oracle A 内）。继续保留二分结构只会让 `kind` 表达力受限。

**方案**：**完全弃用 `oracle_type` 字段，引入 10 种 `kind` flat tag**。每条用例可同时有多个 `kind`（multi-tag），无层级关系。

### 2.2 `kind` 枚举（v1.8 schema）

| kind | 含义 | v1.17 映射 | 数量（v1.8） |
|---|---|---|---|
| `unit-cpp` | C++ 单元测试（链接引擎），CTEST 框架 | 全部 Oracle A "unit" tag | 19 |
| `unit-hdr` | Header-only C++（不链引擎） | enum_class_bitflags_test | 1 |
| `unit-rust` | Rust 单元 / 集成测试（kagami-qa → f11qa crate 内部 #[test]） | 0（v1.17 缺失） | 3（新增） |
| `harness-cpp` | C++ harness（字节级差分，CRC32/MD5） | rom/savestate/frame/wav/mapper diff 测试 | 6 |
| `harness-rust` | Rust harness（$6000 协议 / 调用 C ABI 桥 / kagami_qa_blargg_runner 之类） | blargg/lua runner 各项 | 17 |
| `rom-suite` | 第三方 ROM 套件（依赖 downloader + checksum + license manifest） | blargg_suite + blargg_*_subitem（拆细为 ~67 项） | 65 |
| `static-analysis` | 源码扫描（i18n / Qt SLOT） | i18n_regression / menu_slot_check | 2 |
| `static-license` | 许可证 manifest 校验（新增） | 无（v1.17 缺失） | 1（新增） |
| `perf` | 性能门禁（基准对基线） | bench_tolerance_test | 1 |
| `lua-api` | Lua 脚本绑定测试（harness-rust 内部子类） | 4 个 lua_* 测试 | 4 |
| `smoke` | 烟雾测试（链接/启动/符号） | smoke_test / headless_smoke_test | 2 |

**总计**：120 项（其中 73 项 `kind` 重叠，例如 `harness-rust` 与 `rom-suite` 在 blargg suite 中同时成立）。

### 2.3 编号约定：`kgmqa-NNN-kebab-id`

- `kgmqa` 前缀：保留旧 Kagami 词根做"测试体系标识符"，与新品牌 F11QA 并行（避免与 f11qa-runner / f11qa-blake 这类二进制前缀混淆）
- `NNN` 三位数：从 001 开始，扁平递增，无分组含义
- `kebab-id` 人类可读 id：来自 v1.17 `id` 字段（如 `cpu_instrs` → `cpu-instrs-blargg`）

**示例**：

| v1.17 id | v1.8 kgmqa_id | kind |
|---|---|---|
| `smoke_test` | `kgmqa-001-smoke-cpp` | unit-cpp |
| `cpu_test` | `kgmqa-002-cpu` | unit-cpp |
| `blargg_cpu_instrs` | `kgmqa-032-cpu-instrs-blargg` | rom-suite, harness-rust |
| `i18n_regression_test` | `kgmqa-113-i18n-regression` | static-analysis |
| **新增** | `kgmqa-048-nestest` | rom-suite, harness-rust |
| **新增** | `kgmqa-077-holy-mapperel-tepples` | rom-suite |
| **新增** | `kgmqa-099-240pee-damianyerrick` | rom-suite |
| **新增** | `kgmqa-117-license-manifest-check` | static-license |

### 2.4 `tests.json` v1.8 schema

```jsonc
{
  "schema_version": "1.8",
  "suite_id": "f11qa-v1.8",
  "kgmqa_id": "kgmqa-048-nestest",                    // 新字段：扁平编号
  "legacy_id": null,                                  // 新字段：v1.17 id（如有）
  "title": "kevtris nestest CPU trace vs Nintendulator reference log",
  "kind": ["rom-suite", "harness-rust"],             // 新字段：multi-tag
  "layer": "core",
  "spec_source": "nesdev.org/wiki/Emulator_tests#nestest",  // 新字段：权威源链接
  "license": "PD",                                    // 新字段：每条明示
  "input": {
    "downloader": "scripts/download_nestest.ps1",     // 新字段：rom-suite 强制
    "manifest_entry": "fixtures/nestest_manifest.json",
    "binary": "f11qa_nestest_runner",                 // 重命名后的二进制
    "args": ["--rom", "fixtures/nestest/nestest.nes",
             "--log",  "fixtures/nestest/nestest.log",
             "--frames", "255"]
  },
  "expected": {
    "exit_code": 0,
    "frames_with_divergence": 0                        // 新字段：trace 模式
  },
  "timeout_seconds": 60,
  "tags": ["kevtris", "cpu", "trace", "rom-suite", "TASVideos-required"],
  "failure_means": "blocking",                         // 保留：advisory 仍区分
  "provenance": "v1.8; F11QA-recast-2026-09-21"        // 保留：演进历史
}
```

### 2.5 与 v1.17 schema 的字段差异

| 字段 | v1.17 | v1.8 | 迁移规则 |
|---|---|---|---|
| `suite_id` | `"kagamiqa-v1.17"` | `"f11qa-v1.8"` | 必改 |
| `id` | `"smoke_test"` | `kgmqa_id: "kgmqa-001-smoke-cpp"` | 重命名 + 编号 |
| — | — | `legacy_id: "smoke_test"` | 新增（保留 v1.17 id） |
| `oracle_type` | `"A"` / `"B"` | 删除 | 由 `kind` 表达 |
| — | — | `kind: ["unit-cpp"]` | 新增 multi-tag |
| — | — | `spec_source` | 新增（NESDev Wiki URL / 来源） |
| — | — | `license` | 新增（PD / zlib / GPL / 等） |
| — | — | `downloader` | 新增（rom-suite 强制） |
| — | — | `frames_with_divergence` | 新增（trace 类） |
| `provenance` | v1.17 字符串 | 字符串追加 `"F11QA-recast-2026-09-21"` | 追加 |

---

## 三、扁平清单 1..120（kgmqa_id 总表）

### 3.1 A. C++ 单元（kind=`unit-cpp`，链接引擎）

| # | kgmqa_id | v1.17 legacy_id | 说明 |
|---|---|---|---|
| 1 | `kgmqa-001-smoke-cpp` | `smoke_test` | 核心符号 + fceu11::Initialize() 烟雾 |
| 2 | `kgmqa-002-cpu` | `cpu_test` | CPU 单元 ≥10 case |
| 3 | `kgmqa-003-ppu` | `ppu_test` | PPU 单元 |
| 4 | `kgmqa-004-apu` | `apu_test` | APU 单元 |
| 5 | `kgmqa-005-bus` | `bus_test` | Bus 单元 |
| 6 | `kgmqa-006-mapper-core` | `mapper_core_test` | Mapper core 单元 |
| 7 | `kgmqa-007-cart-class` | `cart_class_test` | Cart class 表面 |
| 8 | `kgmqa-008-driver-callbacks` | `driver_callbacks_test` | g_driver + register_driver POD 契约 |
| 9 | `kgmqa-009-core-driver-boundary` | `core_driver_boundary_test` | core/driver 边界 mutex pImpl 不变量 |
| 10 | `kgmqa-010-savestate-core` | `savestate_core_test` | FCEUSS_SaveMS / LoadMS 字段完整性 |
| 11 | `kgmqa-011-fds-load` | `fds_load_test` | FDS 加载 + 坏 ROM 检测 |
| 12 | `kgmqa-012-ppu-rendering-lut` | `ppu_rendering_lut_test` | LUT 两阶段 sprite 解码 vs 8x 移位链 |
| 13 | `kgmqa-013-ppu-phase-c` | `ppu_phase_c_test` | 原子字节填充 + SPRB + pshift |
| 14 | `kgmqa-014-ppu-phase-d` | `ppu_phase_d_test` | constexpr bitrev LUT + InputScanlineHook + vnapage |
| 15 | `kgmqa-015-expected-api` | `expected_api_test` | tl::expected<T,E> API 覆盖 |
| 16 | `kgmqa-016-mapper-load` | `mapper_load_test` | 遍历所有内置 mapper iNES 加载 |
| 17 | `kgmqa-017-mapper-reset` | `mapper_reset_test` | 加载+reset+power-cycle+reload |
| 18 | `kgmqa-018-config-store` | `config_store_test` | TypedConfig<T> |
| 19 | `kgmqa-019-pixbuf-pool` | `pixbuf_pool_test` | PixBufPool resize/clear/bytes |

### 3.2 B. Header-only / 静态断言（kind=`unit-hdr`）

| # | kgmqa_id | v1.17 legacy_id | 说明 |
|---|---|---|---|
| 20 | `kgmqa-020-enum-class-bitflags` | `enum_class_bitflags_test` | FCEU_ENUM_CLASS_BITFLAGS 宏 |

### 3.3 C. C++ Harness（kind=`harness-cpp`，字节级差分）

| # | kgmqa_id | v1.17 legacy_id | 说明 |
|---|---|---|---|
| 21 | `kgmqa-021-rom-regression-cpp` | `rom_regression_test` | 12 ROM × 60 帧 → CRC32 vs golden_hashes.json |
| 22 | `kgmqa-022-savestate-regression-cpp` | `savestate_regression_test` | 12 ROM × 60 帧 → MD5 vs golden_savestate_hashes.json |
| 23 | `kgmqa-023-ppu-frame-diff` | `ppu_frame_diff_test` | 256×240 XBuf 整帧字节 vs golden_frames/*.xbuf |
| 24 | `kgmqa-024-apu-wav-diff` | `apu_wav_diff_test` | 16-bit mono vs golden_wav/*.wav |
| 25 | `kgmqa-025-mapper-byte-diff-cpp` | `mapper_byte_diff_test` | mapper 状态字节 vs golden_mapper/*.bin（169 mapper） |
| 26 | `kgmqa-026-golden-savestate` | `golden_savestate_test` | 加载 .fc0 → 1 帧 → MD5 vs golden_index.json |

### 3.4 D. Rust 进程内 runner（kind=`harness-rust`）

| # | kgmqa_id | v1.17 legacy_id | 说明 |
|---|---|---|---|
| 27 | `kgmqa-027-rom-regression-rust` | （C-2 迁移自 cpp） | Rust harness：CRC32 视频帧 |
| 28 | `kgmqa-028-savestate-regression-rust` | （C-3 迁移自 cpp） | Rust harness：MD5 savestate |
| 29 | `kgmqa-029-mapper-byte-diff-rust` | （C-3 迁移自 cpp） | Rust harness：mapper 字节差 |
| 30 | `kgmqa-030-blargg-runner` | `blargg_runner` 内核 | $6000 协议 runner（180 ROM 全量入口） |
| 31 | `kgmqa-031-blargg-smoke` | `blargg_smoke` | nestest.nes 烟雾 |
| 32 | `kgmqa-032-cpu-instrs-blargg` | `blargg_cpu_instrs` | instr_test-v5 all_instrs |
| 33 | `kgmqa-033-cpu-timing-blargg` | `blargg_cpu_timing` | cpu_timing_test6 |
| 34 | `kgmqa-034-ppu-vbl-nmi-blargg` | `blargg_ppu_vbl_nmi` | ppu_vbl_nmi（advisory 已知限制） |
| 35 | `kgmqa-035-mmc3-4-scanline-blargg` | `blargg_mmc3_4_scanline_timing` | mmc3_test_4（bucket A） |
| 36 | `kgmqa-036-mmc3-v2-4-scanline-blargg` | `blargg_mmc3_v2_4_scanline_timing` | mmc3_test_v2_4 |
| 37 | `kgmqa-037-cpu-int-2-nmi-brk-blargg` | `blargg_cpu_int_2_nmi_brk` | cpu_int_2（bucket B） |
| 38 | `kgmqa-038-instr-misc-blargg` | `blargg_instr_misc` | instr_misc（bucket B） |
| 39 | `kgmqa-039-oam-stress-blargg` | `blargg_oam_stress` | oam_stress（bucket C） |
| 40 | `kgmqa-040-vbl-05-nmi-timing-blargg` | `blargg_vbl_05_nmi_timing` | vbl_05_nmi_timing（PASS monitor） |
| 41 | `kgmqa-041-ppu-read-buffer-blargg` | `blargg_ppu_read_buffer` | ppu_read_buffer（PASS monitor） |
| 42 | `kgmqa-042-sprdma-dmc-dma-blargg` | `blargg_sprdma_dmc_dma` | sprdma_dmc_dma（bucket D） |
| 43 | `kgmqa-043-blargg-suite` | `blargg_suite` | 全套 180 ROM batch |

### 3.5 E. Lua 绑定（kind=`lua-api` + `harness-rust`）

| # | kgmqa_id | v1.17 legacy_id | 说明 |
|---|---|---|---|
| 44 | `kgmqa-044-lua-bit` | `lua_bit_test` | bit 库 11 函数 |
| 45 | `kgmqa-045-lua-emu` | `lua_emu_test` | emu 库 |
| 46 | `kgmqa-046-lua-memory` | `lua_memory_test` | memory 库 9 函数 |
| 47 | `kgmqa-047-lua-joypad` | `lua_joypad_test` | joypad 库 get/set + mask1/mask2 |

### 3.6 F. 第三方 ROM 套件（kind=`rom-suite`，**v1.8 重点扩展**）

> 所有 ROM 通过独立 `scripts/download_<suite>_roms.ps1` 拉取，gitignore，license manifest 收录。

#### F.1 kevtris（CPU 金标准）

| # | kgmqa_id | suite | 说明 | spec_source |
|---|---|---|---|---|
| 48 | `kgmqa-048-nestest` | kevtris | nestest.nes + nestest.log（CPU 全指令 trace vs Nintendulator 黄金日志） | nesdev.org/wiki/Emulator_tests#nestest |

#### F.2 bisqwit（CPU + PPU 边缘）

| # | kgmqa_id | suite | 说明 | spec_source |
|---|---|---|---|---|
| 49 | `kgmqa-049-ppu-read-buffer-bisqwit` | bisqwit | ppu_read_buffer（$2007 读缓冲怪兽级测试） | bisqwit.iki.fi/src/nes_tests |
| 50 | `kgmqa-050-cpu-dummy-writes-bisqwit` | bisqwit | cpu_dummy_writes | 同上 |
| 51 | `kgmqa-051-cpu-exec-space-bisqwit` | bisqwit | cpu_exec_space | 同上 |
| 52 | `kgmqa-052-cpu-flag-concurrency-bisqwit` | bisqwit | cpu_flag_concurrency | 同上 |

#### F.3 blargg（CPU / PPU / APU 全套补充）

| # | kgmqa_id | suite | 说明 | spec_source |
|---|---|---|---|---|
| 53 | `kgmqa-053-branch-timing-tests-blargg` | blargg | branch_timing_tests | nesdev.org/wiki/Emulator_tests |
| 54 | `kgmqa-054-cpu-interrupts-v2-blargg` | blargg | cpu_interrupts_v2 | 同上 |
| 55 | `kgmqa-055-cpu-reset-blargg` | blargg | cpu_reset | 同上 |
| 56 | `kgmqa-056-instr-timing-blargg` | blargg | instr_timing | 同上 |
| 57 | `kgmqa-057-instr-test-v3-blargg` | blargg | instr_test_v3（vs v5 互补） | 同上 |
| 58 | `kgmqa-058-ppu-sprite-hit-blargg` | blargg | ppu_sprite_hit | 同上 |
| 59 | `kgmqa-059-sprite-overflow-blargg` | blargg | sprite_overflow_tests | 同上 |
| 60 | `kgmqa-060-ppu-open-bus-blargg` | blargg | ppu_open_bus | 同上 |
| 61 | `kgmqa-061-nmi-sync-blargg` | blargg | nmi_sync | 同上 |
| 62 | `kgmqa-062-oam-read-blargg` | blargg | oam_read | 同上 |
| 63 | `kgmqa-063-apu-test-blargg` | blargg | apu_test | 同上 |
| 64 | `kgmqa-064-apu-mixer-blargg` | blargg | apu_mixer | 同上 |
| 65 | `kgmqa-065-dmc-tests-blargg` | blargg | dmc_tests | 同上 |
| 66 | `kgmqa-066-dmc-dma-during-read-blargg` | blargg | dmc_dma_during_read4 | 同上 |
| 67 | `kgmqa-067-square-timer-div2-blargg` | blargg | square_timer_div2 | 同上 |

#### F.4 Damian Yerrick（音频）

| # | kgmqa_id | suite | 说明 | spec_source |
|---|---|---|---|---|
| 68 | `kgmqa-068-volume-tests-damianyerrick` | pinobatch | volume_tests（音频通道混音） | github.com/christopherpow/nes-test-roms/tree/master/volume_tests |

#### F.5 Mapper-specific 多作者套件

| # | kgmqa_id | suite | 作者 | 说明 |
|---|---|---|---|---|
| 69 | `kgmqa-069-mmc3ir-test-nk` | N-K | mmc3irqtest（MMC3 scanline IRQ + $C000 glitch） |
| 70 | `kgmqa-070-mmc5test-drag` | Drag | mmc5test（MMC5 scanline） |
| 71 | `kgmqa-071-mmc5test-v2-awj` | AWJ | mmc5test_v2 |
| 72 | `kgmqa-072-vrc24test-awj` | AWJ | vrc24test（VRC2/4 全变体） |
| 73 | `kgmqa-073-vrc6test-natt` | natt | vrc6test |
| 74 | `kgmqa-074-mmc1atest-tepples` | tepples | mmc1atest（MMC1A vs MMC1B） |
| 75 | `kgmqa-075-mmc3bigchrram-tepples` | tepples | mmc3bigchrram（32KB CHR RAM） |
| 76 | `kgmqa-076-test28-tepples` | tepples | test28（mapper 28 / Action 53） |
| 77 | `kgmqa-077-holy-mapperel-tepples` | pinobatch (formerly tepples) | Holy Mapperel（13 mapper 自动识别） |
| 78 | `kgmqa-078-serom-lidnariq` | lidnariq | serom（MMC1 SEROM/SHROM 约束） |
| 79 | `kgmqa-079-exram-quietust` | Quietust | exram（MMC5 ExRAM） |
| 80 | `kgmqa-080-scanline-quietust` | Quietust | scanline（PPU 扫描线精度） |

#### F.6 FDS 子系统（多作者）

| # | kgmqa_id | suite | 作者 | 说明 |
|---|---|---|---|---|
| 81 | `kgmqa-081-fds-irq-tests-sour` | Sour | FdsIrqTests v7 |
| 82 | `kgmqa-082-fds-mirroring-takuikaninja` | TakuikaNinja | FDS-Mirroring-Test |
| 83 | `kgmqa-083-fds-audio-registers-takuikaninja` | TakuikaNinja | FDS-Audio-Registers |
| 84 | `kgmqa-084-fds-4030d1-addr-takuikaninja` | TakuikaNinja | FDS-4030D1-Addr（DRAM 刷新 IRQ） |
| 85 | `kgmqa-085-fds-4023-test-takuikaninja` | TakuikaNinja | FDS-4023-Test |

#### F.7 NES 2.0 Submapper + 其他 mapper 边缘

| # | kgmqa_id | suite | 作者 | 说明 |
|---|---|---|---|---|
| 86 | `kgmqa-086-nes2-submapper-2-test` | rainwarrior | 2_test（UxROM submapper 0/1/2） |
| 87 | `kgmqa-087-nes2-submapper-3-test` | rainwarrior | 3_test（CNROM） |
| 88 | `kgmqa-088-nes2-submapper-7-test` | rainwarrior | 7_test（AxROM） |
| 89 | `kgmqa-089-nes2-submapper-34-test` | rainwarrior | 34_test（BNROM） |
| 90 | `kgmqa-090-mmc5ramsize-rainwarrior` | rainwarrior | mmc5ramsize（MMC5 PRG-RAM） |
| 91 | `kgmqa-091-n163-soundram-rainwarrior` | rainwarrior | n163_soundram（Namco 163 音频 RAM 读回） |
| 92 | `kgmqa-092-n163-soundram-init-rainwarrior` | rainwarrior | n163_soundram_init（通电初值） |
| 93 | `kgmqa-093-bntest` | — | BNTest（BxROM / BNROM 边界） |
| 94 | `kgmqa-094-bxrom-512k-test-rainwarrior` | rainwarrior | bxrom_512k_test |
| 95 | `kgmqa-095-31-test` | — | 31_test（mapper 31 子 mapper） |
| 96 | `kgmqa-096-fme7acktest-tepples` | tepples | fme7acktest-r1（FME-7 IRQ ack） |
| 97 | `kgmqa-097-fme7ramtest-tepples` | tepples | fme7ramtest-r1（FME-7 WRAM） |
| 98 | `kgmqa-098-famicom-audio-swap-tests` | rainwarrior | 扩展音频互换（5B / MMC5 / VRC6 / VRC7 / N163 / FDS） |

#### F.8 TV 显示 / TV 输出

| # | kgmqa_id | suite | 作者 | 说明 |
|---|---|---|---|---|
| 99 | `kgmqa-099-240pee-damianyerrick` | pinobatch | 240pee（NTSC / PAL / Dendy 时序与 TV 显示） |
| 100 | `kgmqa-100-tvpassfail-tepples` | tepples | tvpassfail（NTSC 色彩 / NTSC-PAL pixel aspect ratio） |

#### F.9 输入（控制器 / 光枪 / 手柄 / Power Pad）

| # | kgmqa_id | suite | 作者 | 说明 |
|---|---|---|---|---|
| 101 | `kgmqa-101-allpads-tepples` | tepples | allpads（多控制器综合） |
| 102 | `kgmqa-102-zap-ruder-tepples` | tepples | Zap Ruder（光枪 / 双持） |
| 103 | `kgmqa-103-spadtest-tepples` | tepples | spadtest-nes（SNES 手柄） |
| 104 | `kgmqa-104-powerpad-tepples` | tepples | POWERPAD.NES / powerpadgesture |
| 105 | `kgmqa-105-paddle-test3-3gengames` | 3gengames | PaddleTest3（Arkanoid 旋转电位器） |
| 106 | `kgmqa-106-vaus-test-lidnariq` | lidnariq | vaus / vaus_test（Arkanoid 9-bit） |
| 107 | `kgmqa-107-mset-rainwarrior` | rainwarrior | mset（SNES 鼠标） |
| 108 | `kgmqa-108-mict-rainwarrior` | rainwarrior | mict（Famicom 麦克风） |
| 109 | `kgmqa-109-telling-lys-tepples` | tepples | Telling LYs?（输入扫描线精度） |
| 110 | `kgmqa-110-dma-sync-test-v2-rahsennor` | Rahsennor | dma_sync_test_v2（DMC DMA 读取损坏） |
| 111 | `kgmqa-111-read-joy3-blargg` | blargg | read_joy3（手柄 + DMC DMA 损坏） |

#### F.10 综合压力

| # | kgmqa_id | suite | 作者 | 说明 |
|---|---|---|---|---|
| 112 | `kgmqa-112-nesstress-flubba` | Flubba | NEStress（综合压力） |

### 3.7 G. 静态分析（kind=`static-analysis`）

| # | kgmqa_id | v1.17 legacy_id | 说明 |
|---|---|---|---|
| 113 | `kgmqa-113-i18n-regression` | `i18n_regression_test` | i18n .ts 覆盖率 >90% + simp/trad 污染 + retranslateUi() 完备 |
| 114 | `kgmqa-114-menu-slot-check` | `menu_slot_check` | Qt SLOT 静态分析（Python scripts/check_menu_slots.py） |

### 3.8 H. 性能门禁（kind=`perf`）

| # | kgmqa_id | v1.17 legacy_id | 说明 |
|---|---|---|---|
| 115 | `kgmqa-115-bench-tolerance` | `bench_tolerance_test` | R4 协议（warmup 3 + timed 7，drop extremes，median vs baseline） |

### 3.9 I. 烟雾（kind=`smoke`）

| # | kgmqa_id | v1.17 legacy_id | 说明 |
|---|---|---|---|
| 116 | `kgmqa-116-headless-smoke` | `headless_smoke_test` | null driver 编译/链接/启动 |

### 3.10 J. **新增** Rust crate 内单元（kind=`unit-rust`）

> v1.17 缺失——Rust crate 仅做 harness，没有反向校验 C++ 契约的 Rust 测试。v1.8 补齐三组对偶。

| # | kgmqa_id | 对偶 C++ 用例 | 说明 |
|---|---|---|---|
| 117 | `kgmqa-117-license-manifest-check` | — | **新增**：每条 rom-suite 用例的 license manifest + SHA-256SUMS 一致性 |
| 118 | `kgmqa-118-config-store-rust` | kgmqa-018 | TypedConfig<T> Rust 端（对偶 C++） |
| 119 | `kgmqa-119-pixbuf-pool-rust` | kgmqa-019 | PixBufPool Rust 端（对偶 C++） |
| 120 | `kgmqa-120-state-facade-rust` | kgmqa-009 | fceux11::State Rust 端（对偶 C++） |

> **注意**：kgmqa-117 是 `static-license` kind，不在 `unit-rust` 之列。117-120 编号与 §三 A-J 节段保持一致，仅最后一组定位调整。

---

## 四、第三方 ROM 套件扩展（详细）

### 4.1 现状 vs 目标

| 维度 | v1.17 | v1.8 目标 |
|---|---|---|
| ROM 套件数 | 1（blargg） | 18（详见下表） |
| 第三方作者数 | 1 | 17 |
| ROM 总数 | 180 | ~290 |
| 许可证明示 | 隐式 | 每条用例 `license` 字段 |
| 抓取脚本 | 1 个 `download_blargg_roms.ps1` | 18 个独立 downloader + checksum 验证 |
| License manifest | 无 | **新增** `tests/fixtures/f11qa_license_manifest.json` |

### 4.2 第三方 ROM 套件完整清单（含 downloader 与 license）

| # | 套件 | 作者 | 许可证 | 下载源 | downloader |
|---|---|---|---|---|---|
| 1 | blargg 全套 | Shay Green (blargg) | **PD**（nesninja.com 标记） | christopherpow/nes-test-roms | `download_blargg_roms.ps1`（已存在） |
| 2 | nestest | kevtris | **PD** | qmtpro.com/~nes | `download_nestest.ps1` |
| 3 | ppu_read_buffer / cpu_dummy_writes / cpu_exec_space / cpu_flag_concurrency | bisqwit (Joel Yliluoma) | **类 zlib / PD**（bisqwit iki.fi 主页声明） | bisqwit.iki.fi/src/nes_tests | `download_bisqwit_roms.ps1` |
| 4 | volume_tests | Damian Yerrick (pinobatch) | **类 zlib**（README.txt 声明） | christopherpow/nes-test-roms/volume_tests | `download_damianyerrick_roms.ps1` |
| 5 | Holy Mapperel | tepples / pinobatch | **zlib** | github.com/pinobatch/holy-mapperel | `download_holy_mapperel.ps1` |
| 6 | 240pee | Damian Yerrick (pinobatch) | **GPL**（与 GPL-2 兼容） | github.com/pinobatch/240p-test-mini | `download_240pee.ps1` |
| 7 | scanline / exram / Color Bars | Quietust | **PD** | qmtpro.com/~nes | `download_quietust_roms.ps1` |
| 8 | NES 2.0 submapper tests / bxrom_512k / mmc5ramsize / n163_soundram / n163_soundram_init / mset / mict / ctrltest / zapper tests | rainwarrior | **PD**（nesdev wiki 公共领域惯例 + 个人 GitHub） | christopherpow/nes-test-roms | `download_rainwarrior_roms.ps1` |
| 9 | test28 / mmc3bigchrram / mmc1atest / fme7acktest-r1 / fme7ramtest-r1 / allpads / Zap Ruder / spadtest-nes / powerpadgesture / POWERPAD.NES / Telling LYs? / tvpassfail | tepples | **PD / 类 zlib**（个人 GitHub） | christopherpow/nes-test-roms + tepples GitHub | `download_tepples_roms.ps1` |
| 10 | vrc24test / mmc5test_v2 | AWJ | **PD**（nesdev 论坛惯例）| nesdev 论坛附件 | `download_awj_roms.ps1` |
| 11 | vrc6test | natt | **PD** | nesdev 论坛附件 | `download_natt_roms.ps1` |
| 12 | mmc3irqtest | N-K | **PD** | nesdev 论坛附件 | `download_nk_roms.ps1` |
| 13 | mmc5test | Drag | **PD** | nesdev 论坛附件 | `download_drag_roms.ps1` |
| 14 | FDS-Mirroring-Test / FDS-Audio-Registers / FDS-4030D1-Addr / FDS-4023-Test | TakuikaNinja | **PD**（TakuikaNinja GitHub）| github.com/TakuikaNinja | `download_takuikaninja_roms.ps1` |
| 15 | FdsIrqTests v7 | Sour | **PD** | Sour GitHub | `download_sour_roms.ps1` |
| 16 | PaddleTest3 | 3gengames | **PD** | nesdev 论坛附件 | `download_3gengames_roms.ps1` |
| 17 | dma_sync_test_v2 | Rahsennor | **PD** | nesdev 论坛附件 | `download_rahsennor_roms.ps1` |
| 18 | oamtest3 / vaus / serom / raw pack2 | lidnariq | **PD / 类 zlib** | nesdev 论坛附件 | `download_lidnariq_roms.ps1` |
| 19 | NEStress | Flubba | **PD**（nesdev wiki） | nesdev wiki | `download_nesstress.ps1` |
| 20 | BNTest | — | **PD** | nesdev 论坛附件 | `download_bntest.ps1` |
| 21 | 31_test | — | **PD** | nesdev 论坛附件 | `download_31test.ps1` |
| 22 | famicom_audio_swap_tests | rainwarrior | **PD** | christopherpow/nes-test-roms | `download_rainwarrior_roms.ps1`（共享） |

### 4.3 许可证合规结论

| 许可证 | 数量 | GPL-2 兼容？ |
|---|---|---|
| **PD**（公共领域） | 16 套件 | ✅ 完全兼容 |
| **zlib / 类 zlib** | 3 套件（Holy Mapperel / volume_tests / 部分 tepples） | ✅ 完全兼容（zlib 是 permissive license） |
| **GPL**（240pee） | 1 套件 | ✅ GPL 兼容 GPL-2（相同许可证可再分发） |
| **MIT / BSD / Apache** | 0 | — |
| **专有 / 不明** | 0 | — |

**总判定**：所有 22 套件 / ~290 ROM 均与 GPL-2 兼容，可安全纳入 F11QA v1.8。

### 4.4 License manifest 设计

```jsonc
// tests/fixtures/f11qa_license_manifest.json
{
  "manifest_version": "1.0",
  "generated_at": "2026-09-21",
  "policy": "All ROMs must be either PD / zlib / GPL-2-compatible. Each entry must have: suite, rom_path, sha256, license, source_url, downloader.",
  "entries": [
    {
      "suite": "blargg",
      "rom_path": "tests/fixtures/blargg/cpu/instr_v5_all.nes",
      "sha256": "<computed-at-fetch-time>",
      "license": "PD",
      "source_url": "https://github.com/christopherpow/nes-test-roms/raw/master/instr_test-v5/all_instrs.nes",
      "downloader": "scripts/download_blargg_roms.ps1"
    },
    {
      "suite": "pinobatch-holy-mapperel",
      "rom_path": "tests/fixtures/holy_mapperel/mapperel.nes",
      "sha256": "<computed-at-fetch-time>",
      "license": "zlib",
      "source_url": "https://github.com/pinobatch/holy-mapperel/releases/download/0.02/mapperel-master.nes",
      "downloader": "scripts/download_holy_mapperel.ps1"
    }
    // ... 共 ~290 项
  ]
}
```

### 4.5 `kgmqa-117-license-manifest-check` 用例

```cpp
// tests/f11qa/license_manifest_check.cpp
// 验证：
// 1. f11qa_license_manifest.json 中每个 rom_path 真实存在
// 2. SHA-256SUMS 与文件实际哈希一致（防篡改/防替换）
// 3. license 字段 ∈ {PD, zlib, GPL, GPL-2.0, GPL-3.0, CC0}
// 4. downloader 脚本路径存在且可执行
// 5. tests.json 中所有 kind=rom-suite 用例的 rom_path 都已登记
// 6. source_url 全部是 https:// 开头（防明文中间人）
```

---

## 五、CI 工作流 v1.8 调整

### 5.1 workflow 文件改名

| v1.17 | v1.8 |
|---|---|
| `.github/workflows/kagami-qa.yml` | `.github/workflows/f11qa.yml` |

### 5.2 workflow 名称与 artifact

```yaml
name: F11QA  # 旧: KagamiQA

# 上传 artifact
- uses: actions/upload-artifact@v4
  with:
    name: f11qa-results      # 旧: kagamiqa-results
    path: |
      build/f11qa_migration_matrix.json
      build/f11qa_accuracy_table.md
      build/f11qa_baseline_next.json
      build/f11qa_quality_report.pdf
```

### 5.3 触发分支

```yaml
on:
  push:
    branches: [main, wip_1.16, wip_v1.17, wip1.8]   # 新增 wip1.8
  pull_request:
    branches: [main, wip_1.16, wip_v1.17, wip1.8]
  workflow_dispatch:
```

### 5.4 多 ROM 套件 fetch + cache（核心新增）

```yaml
# 每个套件独立 cache，避免一个 suite 失败影响其他
- name: Cache ROM suites
  id: cache-roms
  uses: actions/cache@v4
  with:
    path: |
      tests/fixtures/blargg
      tests/fixtures/nestest
      tests/fixtures/bisqwit
      tests/fixtures/damianyerrick
      tests/fixtures/holy_mapperel
      tests/fixtures/240pee
      tests/fixtures/quietust
      tests/fixtures/rainwarrior
      tests/fixtures/tepples
      tests/fixtures/awj
      tests/fixtures/natt
      tests/fixtures/nk
      tests/fixtures/drag
      tests/fixtures/takuikaninja
      tests/fixtures/sour
      tests/fixtures/3gengames
      tests/fixtures/rahsennor
      tests/fixtures/lidnariq
      tests/fixtures/nesstress
      tests/fixtures/bntest
      tests/fixtures/31test
    key: f11qa-roms-${{ hashFiles('tests/fixtures/f11qa_license_manifest.json') }}
    restore-keys: f11qa-roms-

- name: Fetch all ROM suites
  shell: pwsh
  run: |
    $suites = @("blargg","nestest","bisqwit","damianyerrick","holy_mapperel",
                "240pee","quietust","rainwarrior","tepples","awj","natt",
                "nk","drag","takuikaninja","sour","3gengames","rahsennor",
                "lidnariq","nesstress","bntest","31test")
    foreach ($s in $suites) {
      $script = "./scripts/download_${s}_roms.ps1"
      if (Test-Path $script) {
        Write-Host "::group::Fetching $s"
        & $script
        Write-Host "::endgroup::"
      } else {
        Write-Host "::error::missing downloader: $script"
        exit 1
      }
    }

# 验证 manifest 一致性
- name: Verify license manifest against fetched ROMs
  shell: pwsh
  run: |
    $manifest = Get-Content tests/fixtures/f11qa_license_manifest.json -Raw | ConvertFrom-Json
    $bad = @()
    foreach ($entry in $manifest.entries) {
      if (-not (Test-Path $entry.rom_path)) {
        $bad += "$($entry.rom_path) missing"
        continue
      }
      $hash = (Get-FileHash $entry.rom_path -Algorithm SHA256).Hash.ToLower()
      if ($hash -ne $entry.sha256.ToLower()) {
        $bad += "$($entry.rom_path) hash mismatch (got $hash, expected $($entry.sha256))"
      }
    }
    if ($bad.Count -gt 0) {
      foreach ($b in $bad) { Write-Host "::error::$b" }
      exit 1
    }
    Write-Host "License manifest verified: $($manifest.entries.Count) entries"
```

### 5.5 R4 gate 调整（KG-1.8 新版）

```powershell
# .github/workflows/f11qa.yml 末尾
- name: R4 Gate — migration matrix must exist and be sane
  if: always()
  shell: pwsh
  run: |
    $matrixPath = "build/f11qa_migration_matrix.json"
    $failures = @()

    if (-not (Test-Path $matrixPath)) {
      Write-Host "::error::R4 gate: $matrixPath was not produced."
      exit 1
    }

    $m = Get-Content $matrixPath -Raw | ConvertFrom-Json

    # S-4 (v1.7 沿用): git_rev 必须是真实 commit hash
    $gitRev = $m.engine.git_rev
    if ([string]::IsNullOrWhiteSpace($gitRev) -or $gitRev -eq 'unknown') {
      $failures += "engine.git_rev is '$gitRev'"
    }

    # KG-1.8 新增: total 必须精确等于 120（不再 ≥39）
    if ($m.summary.total -ne 120) {
      $failures += "summary.total is $($m.summary.total), expected == 120"
    }

    # KG-1.8 新增: license_manifest_check 必须在 matrix 的 passed 桶
    $licenseCheck = $m.results.kgmqa_117_license_manifest_check
    if (-not $licenseCheck -or $licenseCheck -ne $true) {
      $failures += "kgmqa-117-license-manifest-check is missing or not PASS"
    }

    # Phase 0.5-d (v1.7 沿用): fail_to_pass == 0
    $f2pCount = if ($m.transition_matrix.fail_to_pass.Count) { $m.transition_matrix.fail_to_pass.Count } else { 0 }
    if ($f2pCount -ne 0) {
      $failures += "fail_to_pass is $f2pCount"
    }

    # Task 5 (v1.7 沿用): grade ∈ {A, B, C}
    $grade = $m.grade
    if ($null -eq $grade -or $grade -eq 'D' -or $grade -eq 'E') {
      $failures += "matrix grade is $grade"
    }

    if ($failures.Count -gt 0) {
      foreach ($f in $failures) { Write-Host "::error::R4 gate: $f" }
      exit 1
    }
    Write-Host "R4 gate passed: git_rev=$gitRev, total=$($m.summary.total), grade=$grade, license_check=PASS [OK]"
```

### 5.6 PR 评论（Baseline Drift）

```yaml
- name: Baseline Drift Detection
  if: github.event_name == 'pull_request'
  shell: pwsh
  run: |
    $matrixPath = "build/f11qa_migration_matrix.json"
    if (-not (Test-Path $matrixPath)) { exit 0 }
    $matrix = Get-Content $matrixPath | ConvertFrom-Json
    $passToFail = $matrix.transition_matrix.pass_to_fail
    if ($passToFail -and $passToFail.Count -gt 0) {
      $msg = "## ⚠️ F11QA Baseline Drift Alert`n`n**PASS→FAIL regressions detected:** $($passToFail.Count)`n`n"
      foreach ($entry in $passToFail) {
        $msg += "- ❌ **$($entry.kgmqa_id)** ($($entry.legacy_id))`n"
      }
      gh pr comment ${{ github.event.pull_request.number }} --body "$msg" 2>$null
    }
```

### 5.7 Cache 失效语义（与 v1.17 一致 + ROM 扩展）

| Cache | Key | 失效条件 |
|---|---|---|
| vcpkg | `vcpkg-${{ hashFiles('vcpkg.json') }}` | vcpkg.json 变化 |
| Rust | `rust-kagami-${{ hashFiles('src/rust/Cargo.lock') }}` | Cargo.lock 变化 |
| **v1.8 新增** ROM 全部 | `f11qa-roms-${{ hashFiles('tests/fixtures/f11qa_license_manifest.json') }}` | license manifest 变化（即新增 ROM 或版本变化） |
| Blargg（已存在） | `blargg-roms-${{ hashFiles('scripts/download_blargg_roms.ps1') }}` | downloader 变化 |

**重要变化**：v1.8 把 blargg ROM 的 cache key 从 `blargg-roms-...` 改为统一 `f11qa-roms-...`，与 license manifest 绑定。这样 license manifest 任何一行变化都会触发整个 ROM 重新 fetch + checksum 校验。

---

## 六、tests.json v1.8 schema 详细字段

### 6.1 顶层字段

```jsonc
{
  "schema_version": "1.8",
  "suite_id": "f11qa-v1.8",
  "generated_at": "2026-09-21T13:30:00Z",
  "policy": {
    "license_allowed": ["PD", "zlib", "GPL-2.0", "GPL-3.0", "CC0", "CC-BY"],
    "min_coverage": 1.0,  // 100% cases must have valid binary or downloader
    "r4_gate_thresholds": {
      "total_min": 120,
      "fail_to_pass_max": 0,
      "license_manifest_required": true
    }
  },
  "cases": [
    // ... 120 项 kgmqa-001 ~ kgmqa-120
  ]
}
```

### 6.2 单 case 字段（完整列表）

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `kgmqa_id` | string | ✓ | `kgmqa-NNN-kebab-id`（NNN 三位） |
| `legacy_id` | string \| null | ✓ | v1.17 id；新用例为 `null` |
| `title` | string | ✓ | 人类可读标题 |
| `kind` | string[] | ✓ | multi-tag（≥1，从 10 种 kind 中选） |
| `layer` | string | ✓ | `core` / `boards` / `driver` / `lua` / `benchmark` / `script` |
| `spec_source` | string | ✓ | NESDev Wiki URL / 作者主页 / commit hash 等 |
| `license` | string | ✓ | `PD` / `zlib` / `GPL-2.0` / 等 |
| `input` | object | ✓ | 包含 `downloader`（rom-suite 强制） |
| `expected` | object | ✓ | exit_code + 可能的 trace metrics |
| `timeout_seconds` | number | ✓ | 与 v1.17 一致 |
| `tags` | string[] | ✓ | NESDev wiki tag / TASVideos-required 等 |
| `failure_means` | string | ✓ | `blocking` / `advisory` |
| `provenance` | string | ✓ | 演进历史（追加 `F11QA-recast-2026-09-21`） |

### 6.3 字段迁移示例

| v1.17 | v1.8 |
|---|---|
| `{ "id": "blargg_cpu_instrs", "oracle_type": "B", ... }` | `{ "kgmqa_id": "kgmqa-032-cpu-instrs-blargg", "legacy_id": "blargg_cpu_instrs", "kind": ["rom-suite", "harness-rust"], "license": "PD", "spec_source": "nesdev.org/wiki/Emulator_tests#instr_test_v5", ... }` |

---

## 七、R4 Gate 与发布等级（调整版）

### 7.1 R4 Gate 阈值对比

| 阈值 | v1.17 | v1.8 | 变化理由 |
|---|---|---|---|
| `total` | `≥ 39` | **`== 120`** | 数量已固定精确，浮动阈值无意义 |
| `fail_to_pass` | `== 0` | `== 0` | Phase 0.5-d 反作弊护栏不变 |
| `license_manifest_check` | N/A | **必须 PASS** | 新增：vendor ROM 必须有合规 manifest |
| `grade` | `∈ {A, B, C}` | `∈ {A, B, C}` | 不变 |
| `engine.git_rev` | 非 `unknown` | 非 `unknown` | 不变 |
| `pass_to_fail_count` | 仅警告（PR 评论） | **仍仅警告** | 不变 |

### 7.2 grade 评级逻辑（不变）

| grade | 条件 |
|---|---|
| A | `fail_to_pass == 0` 且所有 advisory 项与 frozen baseline 完全一致 |
| B | `fail_to_pass == 0` 且新增 advisory 数 ≤ frozen baseline 容差 |
| C | `fail_to_pass == 0` 但新增 advisory 项 |
| D | 有真实 regression（PASS→FAIL） |
| E | 矩阵未生成 / license_manifest_check FAIL / 引擎启动失败 |

### 7.3 frozen baseline 升级

| 维度 | v1.17 | v1.8 |
|---|---|---|
| 文件 | `tests/fixtures/kagamiqa_baseline_frozen.json` | `tests/fixtures/f11qa_baseline_frozen.json` |
| 条目数 | 47 | **120** |
| 字段 | `{ results: { "kgmqa_id": bool } }` | 同 v1.17 + `kgmqa_id` 替代 `id` |

---

## 八、迁移 Checklist（v1.17 → v1.8）

### 8.1 文件 / 路径改名（git mv）

| 旧路径 | 新路径 | 类型 |
|---|---|---|
| `tests/kagami/` | `tests/f11qa/` | 目录 |
| `tests/kagami_direct_main.cpp` | `tests/f11qa_direct_main.cpp` | C++ 源 |
| `src/kagami_bridge.h` | `src/f11qa_bridge.h` | C 头 |
| `src/kagami_bridge.cpp` | `src/f11qa_bridge.cpp` | C++ 实现 |
| `src/rust/crates/kagami-qa/` | `src/rust/crates/f11qa/` | Rust crate 目录 |
| `.github/workflows/kagami-qa.yml` | `.github/workflows/f11qa.yml` | workflow |
| `docs/tech/KagamiQA.md` | `docs/tech/F11QA.md` | active doc |
| `tests/fixtures/kagamiqa_baseline_frozen.json` | `tests/fixtures/f11qa_baseline_frozen.json` | manifest |
| `tests/fixtures/kagamiqa_full_baseline.json` | `tests/fixtures/f11qa_full_baseline.json` | manifest（如存在）|
| `scripts/download_blargg_roms.ps1` | 不改（仍是 blargg） | downloader |
| 全部 `scripts/download_kagami_*.ps1`（如有）| `scripts/download_f11qa_*.ps1` | downloader |

### 8.2 文本内字符串替换

| 旧字符串 | 新字符串 | 影响范围 |
|---|---|---|
| `KagamiQA` | `F11QA` | active docs / workflow / source comments / commit messages（新） |
| `kagamiqa` | `f11qa` | manifest / file paths / config keys |
| `kagami-qa` | `f11qa` | Rust crate name / binary name |
| `kagami_qa` | `f11qa` | C function prefix / binary prefix / gitignore 模式 |

### 8.3 新增（增量工作）

| 新增项 | 文件 | 内容 |
|---|---|---|
| `tests/f11qa/license_manifest_check.cpp` |  | kgmqa-117 |
| `src/rust/crates/f11qa/src/config_store.rs` |  | kgmqa-118 |
| `src/rust/crates/f11qa/src/pixbuf_pool.rs` |  | kgmqa-119 |
| `src/rust/crates/f11qa/src/state_facade.rs` |  | kgmqa-120 |
| `scripts/download_<suite>_roms.ps1` | ×18 | 第三方 ROM fetch |
| `tests/fixtures/<suite>_manifest.json` | ×18 | ROM inventory |
| `tests/fixtures/f11qa_license_manifest.json` |  | license 集中登记 |
| `tests/fixtures/f11qa_baseline_frozen.json` |  | 120 项基线 |
| `tests/tests.json` | 重写 | schema v1.8 |

### 8.4 不改的项

- **历史 docs (`docs/history/`)**：完整保留 KagamiQA 命名
- **git tag / branch 历史**：保留
- **历史 commit messages**：保护 git history

---

## 九、阶段化执行计划（10 Phase）

| Phase | 周期（估）| 内容 | 交付物 |
|---|---|---|---|
| **Phase 1：改名 + 路径迁移** | 1 周 | git mv + 字符串批量替换 | F11QA 名称落地；CI 仍跑 v1.17 子集 |
| **Phase 2：扁平 schema + 120 项 manifest** | 2 周 | 编写 v1.8 schema；填充 47 个 v1.17 用例（kgmqa_id + kind + license）+ 73 个第三方用例 | `tests/tests.json` v1.8；schema validator |
| **Phase 3：下载脚本 batch（18 套件）** | 2 周 | 编写 `download_<suite>_roms.ps1` × 18 + checksum 验证 | tests/fixtures/ 各套件 ROM（CI cache） |
| **Phase 4：license manifest + kgmqa-117** | 1 周 | 编写 `f11qa_license_manifest.json` + license_manifest_check.cpp | kgmqa-117 PASS |
| **Phase 5：第三方 ROM runner 扩展** | 3 周 | 适配 kevtris trace / pinobatch 自识别 / tepples 多测试 / FDS 系列 等 | kgmqa-048 ~ kgmqa-112 全部可跑 |
| **Phase 6：新增 Rust crate 单元（118-120）** | 1 周 | 在 f11qa crate 内补 TypedConfig / PixBufPool / State 三组对偶测试 | kgmqa-118/119/120 PASS |
| **Phase 7：R4 gate 升级** | 1 周 | 修改 workflow：total==120 严格 + license_manifest_check 前置 | R4 gate v1.8 |
| **Phase 8：frozen baseline 升级** | 0.5 周 | 录入 120 项基线 | f11qa_baseline_frozen.json |
| **Phase 9：CI 实战 + R4 验证** | 2 周 | 跑 wip1.8 → main；至少 3 次连续 CI green | release grade B 或 A |
| **Phase 10：收口 + 文档归档** | 0.5 周 | `docs/plans/FCEUX11-v1.8_F11QA-构建计划.md` → `docs/history/plans/`；`docs/tech/F11QA.md` active | v1.8 发布 |

**总周期估**：**14 周**（约 3.5 个月）。

---

## 十、回归与基线

### 10.1 已知风险

| 风险 | 等级 | 缓解 |
|---|---|---|
| 第三方 ROM 拉取失败（上游 404 / 改名） | 高 | 每个 downloader 有 `tests/fixtures/<suite>_REVERSE_PROXY_FALLBACK.md` 列出 mirror |
| license 误标（PD vs GPL） | 中 | KG-1.8 manifest 由 kgmqa-117 校验；CI 失败立即可见 |
| 总用例数 120 偏高，CI 时长增加 | 中 | 复用 blargg runner 单次跑完 180 ROM；bisqwit 4 用例 < 5s；增量 ~30s |
| 第三方 ROM $6000 协议不统一（部分用屏幕 hash） | 中 | runner 适配多协议（已 v1.17 实现），加 TESVideos-required tag |
| FDS 子系统精度可能爆出大量 advisory | 低 | FDS 子系统不是 v1.18 重点，advisory 进 baseline 即可 |

### 10.2 兼容性矩阵

| 旧（v1.17）行为 | 新（v1.8）行为 | 兼容性 |
|---|---|---|
| `id: "blargg_cpu_instrs"` | `kgmqa_id: "kgmqa-032-cpu-instrs-blargg"` | 旧 id 移入 `legacy_id` 字段，可双向查询 |
| `oracle_type: "A"` / `"B"` | 字段删除，由 `kind[]` 表达 | **破坏性**：旧脚本读 oracle_type 会失败——文档化迁移 |
| `suite_id: "kagamiqa-v1.17"` | `suite_id: "f11qa-v1.8"` | 破坏性 |
| `kagamiqa_baseline_frozen.json` | `f11qa_baseline_frozen.json` | 路径破坏性 |
| 47 项 cases | 120 项 cases | **强制**：旧 baseline 已包含 47 → 新 baseline 扩充到 120 |

### 10.3 验收标准（v1.8 收口必须满足）

- [ ] `tests/tests.json` schema_version == "1.8"，suite_id == "f11qa-v1.8"，cases 数量 == 120
- [ ] 所有 active docs（`docs/tech/`、`docs/plans/`）中 KagamiQA → F11QA 改名完成
- [ ] CI workflow 改名 + wip1.8 触发分支 + 多套件 cache + license manifest gate
- [ ] 18 个 `scripts/download_<suite>_roms.ps1` 全部可独立 fetch + SHA-256 校验
- [ ] `tests/fixtures/f11qa_license_manifest.json` 收录全部 ~290 ROM
- [ ] kgmqa-117 license_manifest_check 在 CI 上稳定 PASS
- [ ] R4 gate v1.8 在 3 次连续 CI 上 PASS（grade B 或 A）
- [ ] `docs/plans/FCEUX11-v1.8_F11QA-构建计划.md` → `docs/history/plans/` 归档
- [ ] `docs/tech/F11QA.md` active 文本反映 v1.8 状态
- [ ] v1.17 的 `kagamiqa_baseline_frozen.json` 仍保留为对照参考（**不删除**）

---

## 十一、决策点（回签栏）

本计划待用户确认以下 6 个决策点。**回签后进入施工期（Phase 1 启动）**。

- [ ] **决策 1**：是否接受 `KagamiQA → F11QA` 重命名（含 git mv + active docs + workflow 改名，历史归档与 commit 保持原状）？
- [ ] **决策 2**：是否接受扁平清单 + 10 种 `kind` multi-tag 模型（替代 Oracle A/B 二分）？
- [ ] **决策 3**：是否纳入 §三 F 节列出的 65 项第三方 ROM 用例（kgmqa-048 ~ kgmqa-112）？
- [ ] **决策 4**：是否接受"每套件独立 downloader + license manifest + SHA-256 校验"模式（不直接 vendor ROM 到 git）？
- [ ] **决策 5**：总目标数 **120 项**（kgmqa-001 ~ kgmqa-120）是否符合 v1.8 期望？还是希望砍到 ~80 项（小步快跑）或扩到 150+？
- [ ] **决策 6**：是否同意 Phase 1 立即开工（git mv 改名 → 风险最低的 Phase，无需新代码）？

---

## 十二、附录

### 附录 A：NESDev Wiki 完整测试 ROM 索引（参考引用）

> 本计划纳入的所有第三方 ROM 套件，均可在 [NESDev Wiki: Emulator tests](https://www.nesdev.org/wiki/Emulator_tests) 找到权威分类。所有 ROM 的"为何存在"都可追溯到此页面 + 各自的 GitHub 仓库。

| Wiki 节 | 套件数 | F11QA 覆盖 |
|---|---|---|
| CPU Tests | 11 | 13 个 kgmqa（v1.7 + v1.8 增量） |
| PPU Tests | 13 | 8 个 |
| APU Tests | 13 | 6 个 |
| Mapper-specific Tests | 22 | 14 个 |
| Input Tests | 16 | 11 个 |
| Misc Tests | 4 | 3 个 |
| **总计** | **~79** | **~55**（F11QA 覆盖度 70%） |

### 附录 B：参考文献清单

| # | 来源 | 用途 |
|---|---|---|
| 1 | [NESDev Wiki: Emulator tests](https://www.nesdev.org/wiki/Emulator_tests) | ROM 套件权威索引 |
| 2 | [christopherpow/nes-test-roms](https://github.com/christopherpow/nes-test-roms) | 社区归档仓库 |
| 3 | [pinobatch/holy-mapperel](https://github.com/pinobatch/holy-mapperel) | Holy Mapperel（zlib 许可证）|
| 4 | [pinobatch/240p-test-mini](https://github.com/pinobatch/240p-test-mini) | 240pee |
| 5 | [Quietust qmtpro.com](https://www.qmtpro.com/~nes/?news=2012) | scanline / exram / Color Bars |
| 6 | [AprNes Testing Methodology](https://www.baxermux.org/myemu/AprNes/report/methodology.html) | 测试方法论 |
| 7 | [Nesium Test ROM Suite](https://deepwiki.com/mikai233/nesium/6.3-test-rom-suite) | 40+ 套件分类 |
| 8 | [Pinky Test Statuses](https://pioptiop.uk/starrhorne/nes-rust) | Pinky 模拟器测试套件 |
| 9 | `docs/tech/KagamiQA.md`（v1.17） | 体系总览 |
| 10 | `docs/history/plans/FCEUX11-1.17_计划.md` | v1.17 前置基线 |

### 附录 C：v1.8 编号速查表

| 区段 | 编号范围 | 类型 |
|---|---|---|
| A | kgmqa-001 ~ kgmqa-019 | unit-cpp |
| B | kgmqa-020 | unit-hdr |
| C | kgmqa-021 ~ kgmqa-026 | harness-cpp |
| D | kgmqa-027 ~ kgmqa-043 | harness-rust（blargg 系列） |
| E | kgmqa-044 ~ kgmqa-047 | lua-api |
| F.1 | kgmqa-048 | kevtris nestest |
| F.2 | kgmqa-049 ~ kgmqa-052 | bisqwit |
| F.3 | kgmqa-053 ~ kgmqa-067 | blargg 扩展 |
| F.4 | kgmqa-068 | Damian Yerrick volume_tests |
| F.5 | kgmqa-069 ~ kgmqa-080 | Mapper-specific |
| F.6 | kgmqa-081 ~ kgmqa-085 | FDS 子系统 |
| F.7 | kgmqa-086 ~ kgmqa-098 | NES 2.0 + mapper 边缘 |
| F.8 | kgmqa-099 ~ kgmqa-100 | TV 显示 |
| F.9 | kgmqa-101 ~ kgmqa-111 | 输入 |
| F.10 | kgmqa-112 | NEStress |
| G | kgmqa-113 ~ kgmqa-114 | static-analysis |
| I | kgmqa-115 | perf |
| J | kgmqa-116 | smoke |
| 新增 | kgmqa-117 | static-license |
| 新增 | kgmqa-118 ~ kgmqa-120 | unit-rust |

### 附录 D：术语对照表

| 旧术语（KagamiQA） | 新术语（F11QA v1.8） | 说明 |
|---|---|---|
| Oracle A | `kind: unit-cpp / unit-hdr / unit-rust / harness-cpp / lua-api / smoke / static-analysis` | 旧 Oracle A 拆分为多种 kind |
| Oracle B | `kind: rom-suite / harness-rust` | 旧 Oracle B 主要对应这两类 |
| Oracle type | `kind` (multi-tag) | 字段废弃 |
| `id` | `kgmqa_id` | 重命名 |
| `kagamiqa-v1.17` | `f11qa-v1.8` | suite_id |
| `kagamiqa_baseline_frozen.json` | `f11qa_baseline_frozen.json` | 重命名 |
| `kagami-qa-runner.exe` | `f11qa-runner.exe` | Rust crate 二进制 |
| `kagami_qa_*` | `f11qa_*` | C function / binary 前缀 |
| `kagami-qa` | `f11qa` | Rust crate name |

---

**版本记录**：
- v0.1（2026-09-21）：草案，由 F11QA 工作组提出；待决策点回签