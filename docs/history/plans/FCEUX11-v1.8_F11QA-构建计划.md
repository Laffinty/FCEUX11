# FCEUX11 v1.8 构建计划 — F11QA → F11QA 重组、扁平清单与第三方 ROM 套件扩展（接入 f11qa-rom-mirror）

> **STATUS: CLOSED**（v1.8 收口 2026-09-26；Phase 1-8 交付完成，Phase 9 R4 grade B 全绿，Phase 10 本文档归档至 docs/history/plans/。决策 1-10 已回签 A/8b/3A/10a）
> **版本**：v1.8（草案 v0.2）
> **日期**：2026-09-24
> **分支**：`wip1.8`（用户已建）
> **前置**：v1.17 已合并至 main（`docs/history/plans/FCEUX11-1.17_计划.md` STATUS: COMPLETED）；冻结基线 `tests/fixtures/f11qa_baseline_frozen.json` 已落地
> **关联**：`docs/tech/F11QA.md`（将改名 `F11QA.md`）、`docs/history/plans/FCEUX11-1.16_KagamiQA-PLAN.md`、`docs/history/plans/FCEUX11-1.16_KagamiQA-P5-权威性构建计划.md`、`docs/history/plans/FCEUX11-Stage3-权威性迭代与通用化路线.md`、`docs/history/checklists/v2.0_removal_checklist.md`
> **ROM 镜像源**：`https://github.com/Laffinty/f11qa-rom-mirror`（OWNER `@Laffinty`；接受 PD/CC0/zlib/GPL-2/GPL-3/MIT/BSD/Apache；ROM bytes 不接受 PR，OWNER 单人 vendor；详见 §四 §4.6 接入协议 + 镜像源内 `LICENSES.md` / `SHA256SUMS.txt` / `docs/ROM_SOURCE_MAP.md`）
> **路线图位置**：v1.15 完成 v1.x C++ 现代化；v1.16 完成 F11QA 双 Oracle 闭环；v1.17 完成 F11QA 统合 + 遗留精度收敛；**v1.8 = F11QA 改名 F11QA + 扁平清单改造 + 第三方 ROM 套件扩展（覆盖率从 180 → ~290 ROM）+ 接入 `Laffinty/f11qa-rom-mirror` 镜像源（单一权威源 / git tag pin）+ 许可证合规链**

---

## 〇、TL;DR

v1.8 的三项主任务与性质：

| # | 任务 | 性质 | 目标 |
|---|---|---|---|
| 1 | F11QA → **F11QA** 重命名 | 命名统一 | 测试体系名称与项目代号（FCEUX11 → F11）一致；去 "Kagami" 残留字样；CI workflow / docs / commit history 批量改名 |
| 2 | **扁平清单重组**（Oracle A/B 二分 → 1..N + kind 标签） | 架构调整 | 弃用 `oracle_type` 字段，引入 10 种 `kind` 标签；保留全部 47 个 v1.17 用例并重新编号为 `kgmqa-001 ~ kgmqa-120` |
| 3 | **第三方 ROM 套件扩展**（从 blargg 180 → 多作者 18-22 套件 / ~290 ROM）+ **接入 `Laffinty/f11qa-rom-mirror` 镜像源** | 覆盖率提升 + 供应链简化 | 通过镜像源（git tag pin `mirror_ref`）单一权威源统一管理 18-22 套件；license manifest 校验直接对照镜像源的 `LICENSES.md` + `SHA256SUMS.txt` snapshot；22 个独立 downloader → 1 个 `fetch_roms_from_mirror.ps1` |

**一句话收束**：v1.8 把 v1.17 的「双 oracle 测试原型」升级为「**覆盖 120 用例 / ~290 ROM / 单一 mirror 源 / 100% GPL-2 兼容**的扁平、可审计、可机器门禁的统一测试体系」，并以 **F11QA** 为正式名称对外。

**量化收敛目标**：

| 维度 | v1.17（基线） | v1.8 目标 |
|---|---|---|
| 用例总数 | 47（Oracle A 27 + Oracle B 20） | **120**（`kgmqa-001 ~ kgmqa-120`） |
| ROM 套件覆盖 | 1（blargg 180 ROM） | **18-22** 套件 / ~290 ROM（按 mirror 实际 vendor 进度逐条标 `vendor_state`） |
| 第三方作者 | 1（blargg） | **17**（blargg / bisqwit / kevtris nestest / pinobatch Holy Mapperel & 240pee / Quietust / rainwarrior / tepples / AWJ / natt / N-K / Drag / TakuikaNinja / Sour / lidnariq / 3gengames / Rahsennor / Flubba） |
| ROM fetch 源 | 单脚本（blargg） | **单一镜像源** `Laffinty/f11qa-rom-mirror`（git tag pin `mirror_ref`） |
| Downloader 数 | 1 个 | **1 个** `scripts/fetch_roms_from_mirror.ps1` |
| 许可证明示 | 隐式 | **每条用例显式** + mirror `LICENSES.md` snapshot 校验（kgmqa-117） |
| 类型分组 | Oracle A/B 二分 | **10 种 kind 标签**（flat tag，不是层级） |
| 命名 | F11QA | **F11QA**（Kagami 残留清零） |
| R4 gate 阈值 | `total ≥ 39`，`grade ∈ {A,B,C}` | `total == 120`，**`mirror_snapshot_check` PASS** 为前置 |
| 总工期 | — | **12 周**（节省 2 周；Phase 3 downloader batch 2 周 → 0.5 周） |

---

## 一、命名变更：F11QA → F11QA

### 1.1 变更原因

"Kagami" 一词来源于项目早期 `docs/history/plans/FCEUX11-1.16_KagamiQA-PLAN.md` 的 "鏡"（f11qa，日语"镜子"）隐喻——意指 QA 体系是模拟器的"镜子"。但项目主体已演进为 **FCEUX11 v1.x**，继续保留日语音译名字带来三个问题：

1. **品牌一致性**：项目代号 FCEUX11 / F11，QA 子系统叫 F11QA 才符合 "测试体系是项目不可分割部分" 的工程哲学。
2. **检索可发现性**：GitHub、CI artifact、commit 历史中 `F11QA` 字符串散落，新人维护成本高。
3. **许可证文档一致性**：v1.17 的 `f11qa_baseline_frozen.json`、`f11qa_migration_matrix.json`、`f11qa` Rust crate 等命名都需要统一收口。

### 1.2 改名范围（白名单，不波及历史）

| 类别 | 改动 | 说明 |
|---|---|---|
| **当前 docs** | `docs/tech/F11QA.md` → `docs/tech/F11QA.md`，内部文本 `F11QA` → `F11QA` | active 文档全面改名 |
| **CI workflow** | `.github/workflows/f11qa.yml` → `.github/workflows/f11qa.yml` | workflow 文件名 + workflow `name:` 字段 + job 名 + artifact 命名 |
| **GitHub Actions** | `name: F11QA` → `name: F11QA`；artifacts `f11qa-results` → `f11qa-results` | 触发 PR 评论的标签 |
| **tests.json 字段** | `suite_id: "f11qa-v1.17"` → `suite_id: "f11qa-v1.8"` | manifest 顶层版本号 |
| **冻结基线** | `tests/fixtures/f11qa_baseline_frozen.json` → `tests/fixtures/f11qa_baseline_frozen.json` | 字段名同步 |
| **Rust crate** | `src/rust/crates/f11qa/` → `src/rust/crates/f11qa/`；`f11qa-runner.exe` → `f11qa-runner.exe` | crate 改名 + 二进制改名 |
| **C ABI 桥** | `src/f11qa_bridge.{h,cpp}` → `src/f11qa_bridge.{h,cpp}` | 头/源同步 |
| **f11qa_*** 二进制 | `f11qa_blargg_runner` / `f11qa_lua_runner` / `f11qa_rom_regression_runner` 等 → `f11qa_blargg_runner` / `f11qa_lua_runner` / `f11qa_rom_regression_runner` | runner 改名 |
| **f11qa_direct_main.cpp** | `tests/f11qa_direct_main.cpp` → `tests/f11qa_direct_main.cpp` | 直接模式入口改名 |
| **tests/f11qa/** 目录 | → `tests/f11qa/`（C++ 测试源码落点） | 物理目录改名 |
| **f11qa_*** 子目录 | 同步 | — |
| **CI 注释 / commit message** | 历史 commit 中的 `F11QA` **不改写**（保护 git 历史） | 但新 commit 一律 `F11QA` |
| **v1.8 新增：mirror snapshot** | — | `tests/fixtures/f11qa_mirror_pin.json`（pinned git tag + commit SHA） |

### 1.3 不改名的项

- **`docs/history/`** 下所有归档：保留 F11QA 历史命名，是 v1.16/v1.17 的**历史事实**，重写会破坏 commit 链接与考古价值
- **commit message 历史**：用 `git log --follow` 仍可追溯；CI 不需要历史重写
- **git tag `v1.16-F11QA-*` / `v1.17-F11QA-*`**：保留

### 1.4 重命名工具

```bash
# 仅在 wip1.8 分支执行；保护历史 docs/history/ 与 git history
# 1. 二进制名 + 文件名（git mv 保护历史）
find tests scripts src .github -type f \( -name "*f11qa*" -o -name "*Kagami*" \) -print0 \
  | while IFS= read -r -d '' f; do
      new="$(echo "$f" | sed -E 's/[kK]agamiQA/F11QA/g; s/[kK]agami_qa/f11qa/g; s/[kK]agami-qa/f11qa/g; s/[kK]agamiqa/f11qa/g')"
      git mv "$f" "$new"
    done

# 2. 文本替换（仅 active 文件）
grep -rl --include='*.{h,cpp,rs,toml,yml,yaml,md,json,jsonc,ps1,sh,py}' \
     -E 'F11QA|f11qa|f11qa|f11qa' \
     .github tests scripts src docs/tech docs/plans | \
  xargs sed -i -E 's/F11QA/F11QA/g; s/f11qa/f11qa/g; s/f11qa/f11qa/g; s/f11qa/f11qa/g'

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
| `unit-rust` | Rust 单元 / 集成测试（f11qa → f11qa crate 内部 #[test]） | 0（v1.17 缺失） | 3（新增） |
| `harness-cpp` | C++ harness（字节级差分，CRC32/MD5） | rom/savestate/frame/wav/mapper diff 测试 | 6 |
| `harness-rust` | Rust harness（$6000 协议 / 调用 C ABI 桥 / f11qa_blargg_runner 之类） | blargg/lua runner 各项 | 17 |
| `rom-suite` | 第三方 ROM 套件（依赖 mirror snapshot，详见 §四） | blargg_suite + blargg_*_subitem（拆细为 ~67 项） | 65 |
| `static-analysis` | 源码扫描（i18n / Qt SLOT） | i18n_regression / menu_slot_check | 2 |
| `static-license` | **新增**：mirror snapshot 校验（kgmqa-117） | 无（v1.17 缺失） | 1（新增） |
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
| **新增** | `kgmqa-117-mirror-snapshot-check` | static-license |

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
  "license": "PD",                                    // 新字段：每条明示（rom-suite 强制）
  "mirror_ref": "v1.8.0-mirror",                      // v1.8 新字段：rom-suite 强制（git tag）
  "mirror_path": "nestest/nestest.nes",               // v1.8 新字段：rom-suite 强制（相对 mirror 根）
  "vendor_state": "vendored",                         // v1.8 新字段：vendored | advisory | pending-vendor
  "input": {
    "downloader": "scripts/fetch_roms_from_mirror.ps1",  // v1.8：单一脚本
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
  "failure_means": "blocking",                         // 保留：advisory 仍区分（vendor_state=advisory/pending-vendor 时默认 advisory）
  "provenance": "v1.8; F11QA-recast-2026-09-21; mirror-ref-2026-09-24"  // 保留：演进历史
}
```

### 2.5 与 v1.17 schema 的字段差异

| 字段 | v1.17 | v1.8 | 迁移规则 |
|---|---|---|---|
| `suite_id` | `"f11qa-v1.17"` | `"f11qa-v1.8"` | 必改 |
| `id` | `"smoke_test"` | `kgmqa_id: "kgmqa-001-smoke-cpp"` | 重命名 + 编号 |
| — | — | `legacy_id: "smoke_test"` | 新增（保留 v1.17 id） |
| `oracle_type` | `"A"` / `"B"` | 删除 | 由 `kind` 表达 |
| — | — | `kind: ["unit-cpp"]` | 新增 multi-tag |
| — | — | `spec_source` | 新增（NESDev Wiki URL / 来源） |
| — | — | `license` | 新增（PD / zlib / GPL / 等） |
| — | — | `mirror_ref` | **v1.8 新增**（git tag；rom-suite 强制） |
| — | — | `mirror_path` | **v1.8 新增**（相对 mirror 根；rom-suite 强制） |
| — | — | `vendor_state` | **v1.8 新增**（`vendored` / `advisory` / `pending-vendor`） |
| — | — | `frames_with_divergence` | 新增（trace 类） |
| `provenance` | v1.17 字符串 | 字符串追加 `"F11QA-recast-2026-09-21; mirror-ref-2026-09-24"` | 追加 |

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

> **v1.8 重大修订**：所有 ROM 通过 `scripts/fetch_roms_from_mirror.ps1` 单一脚本从 `Laffinty/f11qa-rom-mirror` 镜像源拉取，每条用例的 `mirror_ref`（git tag）+ `mirror_path`（相对 mirror 根路径）+ `vendor_state`（vendor 状态）字段由 kgmqa-117 `mirror_snapshot_check` 校验。详见 §四 §4.6 接入协议。
>
> **`vendor_state` 三态语义**：
> - ✅ `vendored`：mirror 已含完整 ROM 字节，fetch + checksum PASS 即进入 blocking
> - ⏸ `advisory`：mirror 暂搁（上游无 LICENSE 或沙箱拉不到），fetch 失败时记 advisory，**不计入 R4 `fail_to_pass` 计数**
> - ❌ `pending-vendor`：mirror 未开始 vendor，fetch step 必 skip + 记 advisory，等 OWNER vendor 后翻 `vendored`

#### F.1 kevtris（CPU 金标准）

| # | kgmqa_id | suite | mirror 子目录 | 说明 | vendor_state |
|---|---|---|---|---|---|
| 48 | `kgmqa-048-nestest` | kevtris | `nestest/` | nestest.nes + nestest.log（CPU 全指令 trace vs Nintendulator 黄金日志） | ✅ vendored |

#### F.2 bisqwit（CPU + PPU 边缘）

| # | kgmqa_id | suite | mirror 子目录 | 说明 | vendor_state |
|---|---|---|---|---|---|
| 49 | `kgmqa-049-ppu-read-buffer-bisqwit` | bisqwit | `bisqwit/test_ppu_read_buffer.nes` | ppu_read_buffer（$2007 读缓冲怪兽级测试） | ✅ vendored |
| 50 | `kgmqa-050-cpu-dummy-writes-bisqwit` | bisqwit | `bisqwit/cpu_dummy_writes_oam.nes` + `_ppumem.nes` | cpu_dummy_writes | ✅ vendored |
| 51 | `kgmqa-051-cpu-exec-space-bisqwit` | bisqwit | `bisqwit/test_cpu_exec_space_apu.nes` + `_ppuio.nes` | cpu_exec_space | ✅ vendored |
| 52 | `kgmqa-052-cpu-flag-concurrency-bisqwit` | bisqwit | （zip 内多 ROM 待解析；见 ROM_SOURCE_MAP §已知 URL 精度问题） | cpu_flag_concurrency | ⏸ advisory（zip-aware 解压分支尚未实现） |

#### F.3 blargg（CPU / PPU / APU 全套补充）

| # | kgmqa_id | suite | mirror 子目录 | 说明 | vendor_state |
|---|---|---|---|---|---|
| 53 | `kgmqa-053-branch-timing-tests-blargg` | blargg | `blargg/cpu/branch_timing_*.nes` | branch_timing_tests | ✅ vendored |
| 54 | `kgmqa-054-cpu-interrupts-v2-blargg` | blargg | `blargg/cpu/cpu_interrupts_v2_*.nes` | cpu_interrupts_v2 | ✅ vendored |
| 55 | `kgmqa-055-cpu-reset-blargg` | blargg | `blargg/cpu/cpu_reset_*.nes` | cpu_reset | ✅ vendored |
| 56 | `kgmqa-056-instr-timing-blargg` | blargg | `blargg/cpu/instr_timing_*.nes` | instr_timing | ✅ vendored |
| 57 | `kgmqa-057-instr-test-v3-blargg` | blargg | `blargg/cpu/instr_test_v3_*.nes` | instr_test_v3（vs v5 互补） | ✅ vendored |
| 58 | `kgmqa-058-ppu-sprite-hit-blargg` | blargg | `blargg/ppu/sprite_hit_*.nes` | ppu_sprite_hit | ❌ pending-vendor（mirror 待补） |
| 59 | `kgmqa-059-sprite-overflow-blargg` | blargg | `blargg/ppu/sprite_overflow_*.nes` | sprite_overflow_tests | ❌ pending-vendor |
| 60 | `kgmqa-060-ppu-open-bus-blargg` | blargg | `blargg/ppu/ppu_open_bus.nes` | ppu_open_bus | ❌ pending-vendor |
| 61 | `kgmqa-061-nmi-sync-blargg` | blargg | `blargg/ppu/0*-vbl_*.nes` + `1*-nmi_*.nes` | nmi_sync | ❌ pending-vendor |
| 62 | `kgmqa-062-oam-read-blargg` | blargg | `blargg/ppu/oam_read.nes` | oam_read | ❌ pending-vendor |
| 63 | `kgmqa-063-apu-test-blargg` | blargg | `blargg/apu/apu_test*.nes` | apu_test | ❌ pending-vendor |
| 64 | `kgmqa-064-apu-mixer-blargg` | blargg | `blargg/apu/apu_mixer_*.nes` | apu_mixer | ❌ pending-vendor |
| 65 | `kgmqa-065-dmc-tests-blargg` | blargg | `blargg/apu/dmc_tests_*.nes` + `dpcmletterbox.nes` | dmc_tests | ❌ pending-vendor |
| 66 | `kgmqa-066-dmc-dma-during-read-blargg` | blargg | `blargg/apu/dmc_dma_during_read4_*.nes` | dmc_dma_during_read4 | ❌ pending-vendor |
| 67 | `kgmqa-067-square-timer-div2-blargg` | blargg | `blargg/apu/square_timer_div2.nes` | square_timer_div2 | ❌ pending-vendor |

#### F.4 Damian Yerrick（音频）

| # | kgmqa_id | suite | mirror 子目录 | 说明 | vendor_state |
|---|---|---|---|---|---|
| 68 | `kgmqa-068-volume-tests-damianyerrick` | pinobatch | `damianyerrick/volume_tests/volumes.nes` | volume_tests（音频通道混音） | ✅ vendored |

#### F.5 Mapper-specific 多作者套件

| # | kgmqa_id | suite | mirror 子目录 | 作者 | 说明 | vendor_state |
|---|---|---|---|---|---|---|
| 69 | `kgmqa-069-mmc3ir-test-nk` | N-K | `nk/mmc3irqtest/mmc3irqtest.nes` + `_v2.nes` | N-K | mmc3irqtest（MMC3 scanline IRQ + $C000 glitch） | ✅ vendored |
| 70 | `kgmqa-070-mmc5test-drag` | Drag | `drag/mmc5test/mmc5test.nes` | Drag | mmc5test（MMC5 scanline） | ✅ vendored |
| 71 | `kgmqa-071-mmc5test-v2-awj` | AWJ | `awj/mmc5test_v2/mmc5test.nes` | AWJ | mmc5test_v2 | ✅ vendored |
| 72 | `kgmqa-072-vrc24test-awj` | AWJ | （待补；AWJ 余项 + natt vrc6test） | AWJ | vrc24test（VRC2/4 全变体） | ❌ pending-vendor |
| 73 | `kgmqa-073-vrc6test-natt` | natt | （待补；natt 整 suite 未 vendor） | natt | vrc6test | ❌ pending-vendor |
| 74 | `kgmqa-074-mmc1atest-tepples` | tepples | （mirror 中 tepples 子目录无独立 mmc1atest；v1.8 计划列保留为 future） | tepples | mmc1atest（MMC1A vs MMC1B） | ❌ pending-vendor |
| 75 | `kgmqa-075-mmc3bigchrram-tepples` | tepples | `tepples/mmc3bigchrram/mmc3bigchrram.nes` | tepples | mmc3bigchrram（32KB CHR RAM） | ✅ vendored |
| 76 | `kgmqa-076-test28-tepples` | tepples | `tepples/test28/test28.nes` + `test28-8Mbit.nes` | tepples | test28（mapper 28 / Action 53） | ✅ vendored |
| 77 | `kgmqa-077-holy-mapperel-tepples` | pinobatch (formerly tepples) | `holy_mapperel/M{0,1,2,3,4,7,9,10,11,28,34,66,69,78,118,180}_*.nes` 共 47 个 | tepples/pinobatch | Holy Mapperel（13 mapper 自动识别，47 ROM 在 runner 内部循环） | ✅ vendored |
| 78 | `kgmqa-078-serom-lidnariq` | lidnariq | `lidnariq/serom/serom.nes` | lidnariq | serom（MMC1 SEROM/SHROM 约束） | ✅ vendored |
| 79 | `kgmqa-079-exram-quietust` | Quietust | `quietust/mmc5exram.nes` | Quietust | exram（MMC5 ExRAM） | ✅ vendored |
| 80 | `kgmqa-080-scanline-quietust` | Quietust | `quietust/scanline.nes` | Quietust | scanline（PPU 扫描线精度） | ✅ vendored |

#### F.6 FDS 子系统（多作者）

| # | kgmqa_id | suite | mirror 子目录 | 作者 | 说明 | vendor_state |
|---|---|---|---|---|---|---|
| 81 | `kgmqa-081-fds-irq-tests-sour` | Sour | `sour/fdsirqtests/fdsirqtests.fds` + `fdsirqtestsV7_patched.fds` | Sour | FdsIrqTests v7 | ✅ vendored |
| 82 | `kgmqa-082-fds-mirroring-takuikaninja` | TakuikaNinja | `takuikaninja/FDS-Mirroring-Test/mirroring-test.fds`（SHA-256 已在 ROM_SOURCE_MAP §已下载未 vendor） | TakuikaNinja | FDS-Mirroring-Test | ⏸ advisory（上游无 LICENSE；OWNER 等联系） |
| 83 | `kgmqa-083-fds-audio-registers-takuikaninja` | TakuikaNinja | `takuikaninja/FDS-Audio-Registers/audio-registers.fds` | TakuikaNinja | FDS-Audio-Registers | ⏸ advisory（同上） |
| 84 | `kgmqa-084-fds-4030d1-addr-takuikaninja` | TakuikaNinja | `takuikaninja/FDS-4030D1-Addr/4030d1-addr.fds` | TakuikaNinja | FDS-4030D1-Addr（DRAM 刷新 IRQ） | ⏸ advisory（同上） |
| 85 | `kgmqa-085-fds-4023-test-takuikaninja` | TakuikaNinja | `takuikaninja/FDS-4023-Test/4023-test.fds` | TakuikaNinja | FDS-4023-Test | ⏸ advisory（同上） |

#### F.7 NES 2.0 Submapper + 其他 mapper 边缘

| # | kgmqa_id | suite | mirror 子目录 | 作者 | 说明 | vendor_state |
|---|---|---|---|---|---|---|
| 86 | `kgmqa-086-nes2-submapper-2-test` | rainwarrior | （待补；rainwarrior submapper 4 项 sandbox 拿不到） | rainwarrior | 2_test（UxROM submapper 0/1/2） | ⏸ advisory |
| 87 | `kgmqa-087-nes2-submapper-3-test` | rainwarrior | （同上） | rainwarrior | 3_test（CNROM） | ⏸ advisory |
| 88 | `kgmqa-088-nes2-submapper-7-test` | rainwarrior | （同上） | rainwarrior | 7_test（AxROM） | ⏸ advisory |
| 89 | `kgmqa-089-nes2-submapper-34-test` | rainwarrior | （同上） | rainwarrior | 34_test（BNROM） | ⏸ advisory |
| 90 | `kgmqa-090-mmc5ramsize-rainwarrior` | rainwarrior | `rainwarrior/mmc5ramsize/mmc5ramsize.nes` | rainwarrior | mmc5ramsize（MMC5 PRG-RAM） | ✅ vendored |
| 91 | `kgmqa-091-n163-soundram-rainwarrior` | rainwarrior | （待补；rainwarrior 暂搁；wayback 无 .nes 实体） | rainwarrior | n163_soundram（Namco 163 音频 RAM 读回） | ⏸ advisory |
| 92 | `kgmqa-092-n163-soundram-init-rainwarrior` | rainwarrior | （同上） | rainwarrior | n163_soundram_init（通电初值） | ⏸ advisory |
| 93 | `kgmqa-093-bntest` | tepples | `tepples/bntest/bntest-aorom.nes` + `-h.nes` + `-v.nes` | tepples | BNTest（BxROM / BNROM 边界） | ✅ vendored |
| 94 | `kgmqa-094-bxrom-512k-test-rainwarrior` | rainwarrior | `rainwarrior/bxrom_512k_test/bxrom_512k_test.nes` | rainwarrior | bxrom_512k_test | ✅ vendored |
| 95 | `kgmqa-095-31-test` | rainwarrior | （待补；rainwarrior 暂搁） | rainwarrior | 31_test（mapper 31 子 mapper） | ⏸ advisory |
| 96 | `kgmqa-096-fme7acktest-tepples` | tepples | `tepples/fme7/fme7acktest.nes` | tepples | fme7acktest-r1（FME-7 IRQ ack） | ✅ vendored |
| 97 | `kgmqa-097-fme7ramtest-tepples` | tepples | `tepples/fme7/fme7ramtest.nes` | tepples | fme7ramtest-r1（FME-7 WRAM） | ✅ vendored |
| 98 | `kgmqa-098-famicom-audio-swap-tests` | rainwarrior | （zip 内多文件；待 zip-aware 解压分支实现） | rainwarrior | 扩展音频互换（5B / MMC5 / VRC6 / VRC7 / N163 / FDS） | ⏸ advisory |

#### F.8 TV 显示 / TV 输出

| # | kgmqa_id | suite | mirror 子目录 | 作者 | 说明 | vendor_state |
|---|---|---|---|---|---|---|
| 99 | `kgmqa-099-240pee-damianyerrick` | pinobatch | `240pee/240pee.nes` + `-bnrom/-sgrom/-tgrom.nes` + `mdfourier4k*.nes` | Damian Yerrick | 240pee（NTSC / PAL / Dendy 时序与 TV 显示） | ✅ vendored |
| 100 | `kgmqa-100-tvpassfail-tepples` | tepples | `tepples/tvpassfail/tv.nes` | tepples | tvpassfail（NTSC 色彩 / NTSC-PAL pixel aspect ratio） | ✅ vendored |

#### F.9 输入（控制器 / 光枪 / 手柄 / Power Pad）

| # | kgmqa_id | suite | mirror 子目录 | 作者 | 说明 | vendor_state |
|---|---|---|---|---|---|---|
| 101 | `kgmqa-101-allpads-tepples` | tepples | `tepples/porttest/porttest.nes`（多控制器综合，等价 allpads） | tepples | allpads（多控制器综合） | ✅ vendored |
| 102 | `kgmqa-102-zap-ruder-tepples` | tepples | （待补；tepples 余项） | tepples | Zap Ruder（光枪 / 双持） | ❌ pending-vendor |
| 103 | `kgmqa-103-spadtest-tepples` | tepples | （待补） | tepples | spadtest-nes（SNES 手柄） | ❌ pending-vendor |
| 104 | `kgmqa-104-powerpad-tepples` | tepples | （待补） | tepples | POWERPAD.NES / powerpadgesture | ❌ pending-vendor |
| 105 | `kgmqa-105-paddle-test3-3gengames` | 3gengames | `3gengames/PaddleTest3/PaddleTest.nes` | 3gengames | PaddleTest3（Arkanoid 旋转电位器） | ✅ vendored |
| 106 | `kgmqa-106-vaus-test-lidnariq` | lidnariq | `lidnariq/vaus/vaus.nes` + `damianyerrick/vaus-test/vaus-test.nes` | lidnariq | vaus / vaus_test（Arkanoid 9-bit） | ✅ vendored |
| 107 | `kgmqa-107-mset-rainwarrior` | rainwarrior | `rainwarrior/mset/mset.nes` + `mset6x.nes` | rainwarrior | mset（SNES 鼠标） | ✅ vendored |
| 108 | `kgmqa-108-mict-rainwarrior` | rainwarrior | `rainwarrior/mict/mict.nes` | rainwarrior | mict（Famicom 麦克风） | ✅ vendored |
| 109 | `kgmqa-109-telling-lys-tepples` | tepples | `tepples/tellinglys/tellinglys.nes` | tepples | Telling LYs?（输入扫描线精度） | ✅ vendored |
| 110 | `kgmqa-110-dma-sync-test-v2-rahsennor` | Rahsennor | `rahsennor/dma_sync_test_v2/dma_sync_test_v2.nes` | Rahsennor | dma_sync_test_v2（DMC DMA 读取损坏） | ✅ vendored |
| 111 | `kgmqa-111-read-joy3-blargg` | blargg | `blargg/read_joy3/*.nes` | blargg | read_joy3（手柄 + DMC DMA 损坏） | ✅ vendored |

#### F.10 综合压力

| # | kgmqa_id | suite | mirror 子目录 | 作者 | 说明 | vendor_state |
|---|---|---|---|---|---|---|
| 112 | `kgmqa-112-nesstress-flubba` | Flubba | `nesstress/NEStress/NEStress.NES` | Flubba | NEStress（综合压力） | ✅ vendored |

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

### 3.10 J. **新增** 镜像源快照校验（kind=`static-license`，**v1.8 重大修订**）

> v1.8 原计划写 `license_manifest_check.cpp`；现在改为 **`mirror_snapshot_check.cpp`**——直接对照镜像源的 `LICENSES.md` + `SHA256SUMS.txt` snapshot 与本仓库的 pinned `mirror_ref` 一致。详见 §四 §4.6 + §五 §5.4。

| # | kgmqa_id | 对偶 C++ 用例 | 说明 |
|---|---|---|---|
| 117 | `kgmqa-117-mirror-snapshot-check` | — | **新增**：校验 mirror snapshot（`SHA256SUMS.txt` + `LICENSES.md`）与 pinned `mirror_ref` 一致；每条 rom-suite 用例的 `mirror_path` 在 snapshot 中存在；license ∈ accepted set |
| 118 | `kgmqa-118-config-store-rust` | kgmqa-018 | TypedConfig<T> Rust 端（对偶 C++） |
| 119 | `kgmqa-119-pixbuf-pool-rust` | kgmqa-019 | PixBufPool Rust 端（对偶 C++） |
| 120 | `kgmqa-120-state-facade-rust` | kgmqa-009 | fceux11::State Rust 端（对偶 C++） |

> **注意**：kgmqa-117 是 `static-license` kind，不在 `unit-rust` 之列。117-120 编号与 §三 A-J 节段保持一致。

---

## 四、第三方 ROM 套件扩展 + 镜像源接入协议（详细）

### 4.0 章节定位

> **v1.8 重大修订（2026-09-24）**：本章原计划写"每个套件独立 downloader + 独立 license manifest"，现改为**接入 `Laffinty/f11qa-rom-mirror` 单一权威镜像源**。F11QA 通过 git tag pin (`mirror_ref`) 引用镜像源的特定发布版本，由 `scripts/fetch_roms_from_mirror.ps1` 拉取并校验 SHA-256，license 信息直接复用镜像源 `LICENSES.md`。22 个 downloader → 1 个 fetch 脚本。

### 4.1 现状 vs 目标（基于镜像源实际数据）

| 维度 | v1.17 | v1.8 目标（按镜像源 vendor 进度） |
|---|---|---|
| ROM 套件数 | 1（blargg） | **18-22** 套件（按镜像源结构计 21 个 suite 目录 + 1 个未单列；见 §4.2） |
| 第三方作者数 | 1 | **17**（blargg / kevtris / bisqwit / pinobatch [Holy Mapperel + 240pee + volume_tests + vaus-test] / Quietust / rainwarrior / tepples / AWJ / natt / N-K / Drag / TakuikaNinja / Sour / lidnariq / 3gengames / Rahsennor / Flubba） |
| ROM 总数 | 180 | **~290**（镜像源目标总量；当前已 vendor ≈191，详见 §4.2 各 suite 状态） |
| 许可证明示 | 隐式 | 每条用例 `license` 字段 + 镜像源 `LICENSES.md` snapshot 校验 |
| 抓取脚本 | 1 个 `download_blargg_roms.ps1` | **1 个** `scripts/fetch_roms_from_mirror.ps1` |
| License manifest | 无 | 复用镜像源 `LICENSES.md`；kgmqa-117 `mirror_snapshot_check` 校验 |
| 镜像源（新增） | — | `https://github.com/Laffinty/f11qa-rom-mirror`（OWNER `@Laffinty`） |
| 镜像源 pin 字段（新增） | — | `mirror_ref`（git tag，如 `v1.8.0-mirror`）+ `mirror_path`（相对镜像源根） |

### 4.2 第三方 ROM 套件完整清单（按镜像源结构，含 vendor 状态）

> **vendor_state 三态**：
> - ✅ **vendored**：镜像源已含完整 ROM 字节（SHA-256SUMS.txt 中已有对应行），fetch + checksum PASS 即进入 blocking
> - ⏸ **advisory**：镜像源已下载但 vendor 受阻（上游无 LICENSE 或沙箱拉不到），fetch 失败时记 advisory，**不计入 R4 `fail_to_pass` 计数**
> - ❌ **pending-vendor**：镜像源尚未 vendor，fetch step 必 skip + 记 advisory，等 OWNER vendor 后翻 `vendored`

| # | 套件（镜像源子目录） | 作者 | 许可证 | 已 vendor ROM 数 | 状态 | kgmqa_id 覆盖 |
|---|---|---|---|---|---|---|
| 1 | `blargg/cpu/` | Shay Green (blargg) | PD | 69 | ✅ vendored | kgmqa-032, 033, 035, 036, 037, 038, 053, 054, 055, 056, 057 |
| 1 | `blargg/ppu/` | blargg | PD | 0（待补） | ❌ pending-vendor | kgmqa-058, 059, 060, 061, 062 |
| 1 | `blargg/apu/` | blargg | PD | 0（待补） | ❌ pending-vendor | kgmqa-063, 064, 065, 066, 067 |
| 1 | `blargg/mmc3/` | blargg | PD | 0（待补） | ❌ pending-vendor | （与 kgmqa-035, 036 复用） |
| 1 | `blargg/sprdma/` | blargg | PD | 0（待补） | ❌ pending-vendor | kgmqa-042 |
| 1 | `blargg/vbl_nmi_timing/` | blargg | PD | 7 | ✅ vendored | kgmqa-040 |
| 1 | `blargg/read_joy3/` | blargg | PD | 4 | ✅ vendored | kgmqa-111 |
| 2 | `nestest/` | kevtris | PD | 2（nes + log） | ✅ vendored | kgmqa-048 |
| 3 | `bisqwit/` | Joel Yliluoma | zlib | 5（zip 内多 ROM 已解压）+ zip-aware 分支待实现 | ⏸ advisory | kgmqa-049, 050, 051, 052 |
| 4 | `holy_mapperel/` | pinobatch (formerly tepples) | zlib | 47 | ✅ vendored | kgmqa-077 |
| 5 | `240pee/` | Damian Yerrick (pinobatch) | GPL-2.0-only | 6 | ✅ vendored | kgmqa-099 |
| 6 | `quietust/` | Quietust | PD | 5 | ✅ vendored | kgmqa-079, 080 |
| 7 | `rainwarrior/` | rainwarrior | PD / zlib | 8（bxrom_512k_test, mmc5ramsize, mset×2, mict, palette×3, ram_retain） | 部分 ✅ / 部分 ⏸ | kgmqa-086~095, 107, 108 |
| 8 | `tepples/` | tepples | PD / zlib | ≈21（tvpassfail, test28×2, mmc3bigchrram, fme7×2, tellinglys, bntest×3, porttest, test78×7, mmc3save, oam-reset, chrpress） | ✅ vendored | kgmqa-075, 076, 093, 096, 097, 100, 101, 109 |
| 9 | `awj/` | AWJ | PD | 1（mmc5test_v2/mmc5test.nes） | ✅ vendored（仅 1 项） / 余项 ❌ | kgmqa-071, 072 |
| 10 | `natt/` | natt | PD | 0（待补） | ❌ pending-vendor | kgmqa-073 |
| 11 | `nk/` | N-K | PD | 2（mmc3irqtest × 2 变体） | ✅ vendored（仅 2 变体） / 余项 ❌ | kgmqa-069 |
| 12 | `drag/` | Drag | PD | 1（mmc5test/mmc5test.nes） | ✅ vendored | kgmqa-070 |
| 13 | `takuikaninja/` | TakuikaNinja | **上游无 LICENSE** | 0（4 个 .fds 已下载未 vendor；SHA-256 已记录） | ⏸ advisory | kgmqa-082, 083, 084, 085 |
| 14 | `sour/` | Sour | MIT | 2（fdsirqtests + V7 patched） | ✅ vendored | kgmqa-081 |
| 15 | `3gengames/` | 3gengames | PD | 1（PaddleTest3） | ✅ vendored（仅 1 项） / 余项 ❌ | kgmqa-105 |
| 16 | `rahsennor/` | Rahsennor | PD | 2（dma_sync_test_v2 + apu_phase_reset） | ✅ vendored | kgmqa-110 |
| 17 | `lidnariq/` | lidnariq | PD / 类 zlib | 4（serom, oamtest3, vaus, characterize-vs） | ✅ vendored（raw_pack2 未定位 ❌） | kgmqa-078, 106 |
| 18 | `nesstress/` | Flubba | PD | 1（NEStress） | ✅ vendored | kgmqa-112 |
| 19 | `bntest/` | — | PD | 1（bntest_aorom） | ✅ vendored | kgmqa-093（与 tepples/bntest 重名消歧） |
| 20 | `31test/` | — | PD | 0（子目录空） | ❌ pending-vendor | kgmqa-095 |
| 21 | `damianyerrick/volume_tests/` | Damian Yerrick (pinobatch) | 类 zlib | 1（volumes.nes） | ✅ vendored | kgmqa-068 |
| 21 | `damianyerrick/vaus-test/` | Damian Yerrick (pinobatch) | 类 zlib | 1（vaus-test.nes） | ✅ vendored | kgmqa-106（与 lidnariq/vaus 共用 kgmqa） |

**统计**：21 个 suite 目录，已 vendor ≈191 ROM；当前 kgmqa-117 校验应通过 ≈191 项 checksum；其余用例通过 `vendor_state=advisory/pending-vendor` 跳过 fetch 步。

### 4.3 许可证合规结论（基于镜像源 accepted set）

| 许可证 | 镜像源 accepted? | F11QA 兼容? |
|---|---|---|
| **PD / CC0** | ✅ | ✅（完全兼容） |
| **zlib / zlib-like** | ✅ | ✅（permissive；attribution 保留） |
| **GPL-2.0-only / -or-later** | ✅ | ✅（与 FCEUX11 主仓库 GPL-2 兼容） |
| **GPL-3.0-only / -or-later** | ✅ | ✅（per case review，专利条款单独评估） |
| **MIT / BSD / Apache** | ✅ | ✅（permissive；attribution 保留） |
| **Proprietary / unknown / missing source** | ❌（镜像源拒绝） | ❌（F11QA 一并拒绝） |

**总判定**：镜像源 accepted set ⊆ F11QA accepted set；镜像源拒绝的 license F11QA 一并拒绝。F11QA 不需要再独立维护 license 白名单，只需校验"每条用例的 license ∈ mirror accepted set"。

### 4.4 `mirror_snapshot_check` 设计（取代原 `license_manifest`）

```jsonc
// tests/fixtures/f11qa_mirror_pin.json
{
  "schema_version": "1.0",
  "mirror_repo": "https://github.com/Laffinty/f11qa-rom-mirror",
  "mirror_ref": "v1.8.0-mirror",        // git tag（决策 8b）
  "mirror_commit_sha": "<fetched-at-time-pin-sha>",
  "pinned_at": "2026-09-24T00:00:00Z",
  "pinned_by": "@Laffinty (mirror OWNER) + F11QA @wip1.8",
  "expected_files": [
    "SHA256SUMS.txt",
    "LICENSES.md",
    "LICENSE",
    "README.md",
    "docs/ROM_SOURCE_MAP.md",
    "docs/构建计划.md",
    ".gitignore",
    "scripts/sync_from_upstream.sh",
    "scripts/verify_licenses.sh",
    "scripts/audit_sha256.sh"
    // + 21 个 suite 目录
  ]
}
```

**kgmqa-117 mirror_snapshot_check** 校验内容：

```cpp
// tests/f11qa/mirror_snapshot_check.cpp
// 校验：
// 1. tests/fixtures/f11qa_mirror_pin.json 存在且 mirror_ref 字段非空
// 2. clone 镜像源 mirror_ref 到 tests/fixtures/_mirror_snapshot/（git clone --depth=1 --branch $mirror_ref）
// 3. sha256sum -c SHA256SUMS.txt --strict 全部 PASS（防篡改/防替换）
// 4. LICENSES.md 中每条 license 字段 ∈ mirror accepted set（PD/zlib/GPL-2/GPL-3/MIT/BSD/Apache/CC0）
// 5. tests.json 中所有 kind=rom-suite 用例的 mirror_path 都在 SHA256SUMS.txt 中存在
// 6. tests.json 中所有 kind=rom-suite 用例的 license ∈ mirror accepted set
// 7. tests.json 中所有 kind=rom-suite 用例的 mirror_ref == f11qa_mirror_pin.json 的 mirror_ref
// 8. source_url（可由 mirror LICENSES.md 推导）全部是 https:// 开头（防明文中间人）
```

### 4.5 kgmqa-117 mirror_snapshot_check 用例

```cpp
// tests/f11qa/mirror_snapshot_check.cpp
// 触发时机：CI workflow 第 1 步（早于所有 rom-suite 用例）
// 失败处理：任一步失败 → kgmqa-117 FAIL → CI 红 → R4 gate 阻断所有 rom-suite 用例
// vendor_state=advisory 用例：kgmqa-117 不校验其存在（仅校验 mirror 中其他 vendored ROM）
// vendor_state=pending-vendor 用例：同上

bool run_mirror_snapshot_check() {
    auto pin = load_json("tests/fixtures/f11qa_mirror_pin.json");
    string mirror_ref = pin["mirror_ref"];
    string snapshot_dir = "tests/fixtures/_mirror_snapshot";
    if (exists(snapshot_dir)) fs::remove_all(snapshot_dir);

    // 步骤 2：clone pinned ref
    int rc = run("git", "clone", "--depth=1", "--branch", mirror_ref,
                 pin["mirror_repo"], snapshot_dir);
    if (rc != 0) return emit_error("git clone mirror_ref=" + mirror_ref + " failed");

    // 步骤 3：sha256sum 校验
    rc = run_in_dir(snapshot_dir, "sha256sum", "-c", "SHA256SUMS.txt", "--strict", "--quiet");
    if (rc != 0) return emit_error("SHA256SUMS.txt mismatch — mirror tampering or fetch error");

    // 步骤 4-7：解析 tests.json + LICENSES.md 交叉校验
    auto cases = load_tests_json()->cases;
    auto licenses = load_licenses_md(snapshot_dir + "/LICENSES.md");
    for (auto& c : cases) {
        if (!has_kind(c, "rom-suite")) continue;
        if (c["mirror_ref"] != mirror_ref) return emit_error("case " + c["kgmqa_id"] + " mirror_ref mismatch");
        if (!licenses.contains(c["mirror_path"])) return emit_error("case " + c["kgmqa_id"] + " mirror_path not in LICENSES.md");
        if (!is_accepted_license(c["license"])) return emit_error("case " + c["kgmqa_id"] + " license " + c["license"] + " not in accepted set");
    }
    return true;
}
```

### 4.6 Mirror 接入协议（**v1.8 新增**）

#### 4.6.1 mirror_ref 字段语义

每条 `kind=rom-suite` 用例在 `tests.json` 中必填：

```jsonc
{
  "mirror_ref": "v1.8.0-mirror",     // git tag（决策 8b）
  "mirror_path": "nestest/nestest.nes",  // 相对镜像源根
  "vendor_state": "vendored"         // 或 advisory / pending-vendor
}
```

**mirror_ref 钉版策略（决策 8b）**：
- 镜像源 OWNER 发版时打 git tag：`v1.<x>.<y>-mirror`
- F11QA 在 wip1.8 → main 合并前，OWNER 在 PR 中同步更新 `tests/fixtures/f11qa_mirror_pin.json` 的 `mirror_ref` 字段
- CI cache key 绑定 mirror_ref，故每次升级会触发 ROM 重新 fetch + checksum 校验
- 主分支 main 永远指向固定的 mirror_ref（保证 CI 可重现）

#### 4.6.2 Fetch 协议

`scripts/fetch_roms_from_mirror.ps1` 工作流（伪代码）：

```powershell
# 1. 读 mirror pin
$pin = Get-Content tests/fixtures/f11qa_mirror_pin.json | ConvertFrom-Json
$mirror_ref = $pin.mirror_ref

# 2. 解析 tests.json，提取所有 kind=rom-suite 用例的 mirror_path + vendor_state
$cases = (Get-Content tests/tests.json | ConvertFrom-Json).cases | Where-Object { $_.kind -contains "rom-suite" }

# 3. clone mirror 快照到本地缓存
$cache = "tests/fixtures/_mirror_cache"
if (Test-Path $cache) { Remove-Item -Recurse -Force $cache }
git clone --depth=1 --branch $mirror_ref https://github.com/Laffinty/f11qa-rom-mirror.git $cache

# 4. SHA-256 校验
Push-Location $cache
$bad = & sha256sum --check SHA256SUMS.txt --strict --quiet 2>&1
if ($LASTEXITCODE -ne 0) { Write-Error "Mirror SHA-256 mismatch"; exit 1 }
Pop-Location

# 5. 复制 ROM 到 fixtures
foreach ($c in $cases) {
    if ($c.vendor_state -eq "pending-vendor") {
        Write-Host "::warning::kgmqa=$($c.kgmqa_id) vendor_state=pending-vendor, fetch skip"
        continue
    }
    $src = Join-Path $cache $c.mirror_path
    $dst = Join-Path "tests/fixtures" (Split-Path $c.mirror_path -Leaf)
    $dstDir = Split-Path $dst -Parent
    if (-not (Test-Path $dstDir)) { New-Item -ItemType Directory -Force -Path $dstDir | Out-Null }
    Copy-Item -Force $src $dst
    Write-Host "fetched $($c.kgmqa_id) <- $($c.mirror_path)"
}

# 6. 验证每个 vendored ROM 的实际 SHA-256
foreach ($c in $cases) {
    if ($c.vendor_state -ne "vendored") { continue }
    $local = Join-Path "tests/fixtures" (Split-Path $c.mirror_path -Leaf)
    $expected = (Get-Content (Join-Path $cache "SHA256SUMS.txt")) `
                | Where-Object { $_ -match "  $($c.mirror_path -replace '\\','/')  *$" } `
                | ForEach-Object { ($_ -split '  ')[0] }
    $actual = (Get-FileHash -Algorithm SHA256 $local).Hash.ToLower()
    if ($actual -ne $expected) { Write-Error "SHA-256 mismatch: $($c.kgmqa_id)"; exit 1 }
}
```

#### 4.6.3 Mirror 失效应对

| 失效场景 | 应对 |
|---|---|
| 镜像源 GitHub 仓库被删 / 私有化 | 镜像源 OWNER 立即把 `tests/fixtures/_mirror_snapshot/` 作为外部 backup；F11QA 改读本仓库 snapshot；mirror_ref 维持原值，但 snapshot 来源改为本仓库 |
| 镜像源某个 git tag 不可达 | CI workflow `actions/checkout@v4` 的 `fetch-depth: 0` + tag ref 拉取失败时，CI 报错并要求 OWNER 重新打 tag |
| 镜像源上游失效（某 ROM 在 LICENSES.md 中 source_url 404） | 由镜像源 OWNER 处理（详见镜像源 `docs/ROM_SOURCE_MAP.md` §上游失效应对）；F11QA 不直接处理上游失效 |
| 镜像源 OWNER 长期不响应 | F11QA 决策：fork 镜像源（`Laffinty/f11qa-rom-mirror` → `<F11QA-fork>/f11qa-rom-mirror`），改 `mirror_repo` 字段，重新 pin |
| 镜像源 LICENSE policy 收紧（之前接受的 license 现在拒绝） | kgmqa-117 `mirror_snapshot_check` 步骤 4 自动 FAIL；CI 红；OWNER 决定是 vendor 移除（用例 kgmqa_id 失效）还是单独豁免（用例 kgmqa_id 标记 `failure_means=advisory`） |
| TakuikaNinja / rainwarrior 等 ⏸/❌ 套件 OWNER 推进 vendor | OWNER 在镜像源 PR 添加 SHA-256SUMS.txt 行 + LICENSES.md entry；F11QA OWNER 同步 `mirror_ref` 升级到下一个 tag；CI 自动翻 `vendor_state=vendored`，kgmqa 升级为 blocking |

#### 4.6.4 工作分工

| 角色 | 责任 |
|---|---|
| 镜像源 OWNER（你） | 维护 `Laffinty/f11qa-rom-mirror`：vendor ROM、维护 LICENSES.md、生成 SHA256SUMS.txt、打 git tag、维护 `docs/ROM_SOURCE_MAP.md` |
| F11QA OWNER（你，同一人） | 在 F11QA 仓库维护 `tests/fixtures/f11qa_mirror_pin.json` 的 `mirror_ref`；编写并维护 `scripts/fetch_roms_from_mirror.ps1` + kgmqa-117 `mirror_snapshot_check.cpp`；维护 tests.json 中 65 条 rom-suite 用例的 mirror_ref/mirror_path/vendor_state/license 四元组 |
| CI | 调用 fetch 脚本 + kgmqa-117 snapshot check + 65 条 rom-suite 用例 |

**单一 OWNER 优势**：vendor policy 与 tests.json 字段完全由同一人控制，避免镜像源与 F11QA 双向耦合冲突。

---

## 五、CI 工作流 v1.8 调整

### 5.1 workflow 文件改名

| v1.17 | v1.8 |
|---|---|
| `.github/workflows/f11qa.yml` | `.github/workflows/f11qa.yml` |

### 5.2 workflow 名称与 artifact

```yaml
name: F11QA  # 旧: F11QA

# 上传 artifact
- uses: actions/upload-artifact@v4
  with:
    name: f11qa-results      # 旧: f11qa-results
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

### 5.4 多 ROM 套件 fetch + cache（**v1.8 重大修订：单一 fetch 脚本**）

```yaml
# 1. kgmqa-117 mirror_snapshot_check 必跑（早于所有 rom-suite 用例）
- name: Mirror snapshot check (kgmqa-117)
  id: mirror-snapshot
  shell: pwsh
  run: |
    $bin = "./build/f11qa_mirror_snapshot_check.exe"
    if (-not (Test-Path $bin)) {
      Write-Host "::error::kgmqa-117 binary not built"
      exit 1
    }
    & $bin --manifest tests/fixtures/f11qa_mirror_pin.json
    if ($LASTEXITCODE -ne 0) {
      Write-Host "::error::kgmqa-117 mirror_snapshot_check FAIL — blocking all rom-suite cases"
      exit 1
    }

# 2. 单一 fetch 脚本（取代 22 个 downloader）
- name: Cache mirror snapshot
  id: cache-mirror
  uses: actions/cache@v4
  with:
    path: |
      tests/fixtures/_mirror_cache
      tests/fixtures/_mirror_snapshot
      tests/fixtures/<suite>/<rom>.nes     # 65 条 rom-suite 用例的本地副本
    key: f11qa-mirror-${{ hashFiles('tests/fixtures/f11qa_mirror_pin.json') }}
    restore-keys: |
      f11qa-mirror-
      f11qa-

- name: Fetch ROMs from mirror (v1.8: single script)
  if: steps.mirror-snapshot.outputs.cache-hit != 'true'
  shell: pwsh
  run: |
    & ./scripts/fetch_roms_from_mirror.ps1 `
        --manifest tests/fixtures/f11qa_mirror_pin.json `
        --tests-json tests/tests.json `
        --output-dir tests/fixtures
    if ($LASTEXITCODE -ne 0) {
      Write-Host "::error::fetch_roms_from_mirror failed"
      exit 1
    }
    $vendoredCount = (Get-Content tests/tests.json | ConvertFrom-Json).cases `
        | Where-Object { $_.kind -contains "rom-suite" -and $_.vendor_state -eq "vendored" } `
        | Measure-Object | Select-Object -ExpandProperty Count
    $advisoryCount = (Get-Content tests/tests.json | ConvertFrom-Json).cases `
        | Where-Object { $_.kind -contains "rom-suite" -and $_.vendor_state -eq "advisory" } `
        | Measure-Object | Select-Object -ExpandProperty Count
    $pendingCount = (Get-Content tests/tests.json | ConvertFrom-Json).cases `
        | Where-Object { $_.kind -contains "rom-suite" -and $_.vendor_state -eq "pending-vendor" } `
        | Measure-Object | Select-Object -ExpandProperty Count
    Write-Host "Fetched: vendored=$vendoredCount, advisory=$advisoryCount (fetch will skip), pending-vendor=$pendingCount (fetch skip)"

# 3. License manifest 一致性（由 kgmqa-117 mirror_snapshot_check 已包含；此处只 PR 评论）
- name: License manifest summary
  shell: pwsh
  run: |
    $m = Get-Content tests/fixtures/_mirror_snapshot/LICENSES.md -Raw
    Write-Host "Mirror LICENSES.md size: $($m.Length) chars"
    Write-Host "Mirror SHA256SUMS.txt lines: $((Get-Content tests/fixtures/_mirror_snapshot/SHA256SUMS.txt | Measure-Object -Line).Lines)"
```

### 5.5 R4 gate 调整（KG-1.8 新版：mirror_snapshot_check 前置）

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

    # KG-1.8 新增（取代原 license_manifest_check）: mirror_snapshot_check 必须在 matrix 的 passed 桶
    $mirrorCheck = $m.results.kgmqa_117_mirror_snapshot_check
    if (-not $mirrorCheck -or $mirrorCheck -ne $true) {
      $failures += "kgmqa-117-mirror-snapshot-check is missing or not PASS"
    }

    # Phase 0.5-d (v1.7 沿用): fail_to_pass == 0
    # 注意：vendor_state=advisory/pending-vendor 用例的 fetch 失败不计 fail_to_pass
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
    Write-Host "R4 gate passed: git_rev=$gitRev, total=$($m.summary.total), grade=$grade, mirror_snapshot_check=PASS [OK]"
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

### 5.7 Cache 失效语义（**v1.8 修订：mirror_ref 驱动**）

| Cache | Key | 失效条件 |
|---|---|---|
| vcpkg | `vcpkg-${{ hashFiles('vcpkg.json') }}` | vcpkg.json 变化 |
| Rust | `rust-f11qa-${{ hashFiles('src/rust/Cargo.lock') }}` | Cargo.lock 变化 |
| **v1.8 新增** Mirror snapshot | `f11qa-mirror-${{ hashFiles('tests/fixtures/f11qa_mirror_pin.json') }}` | mirror_ref 变化（即 `f11qa_mirror_pin.json` 的 mirror_ref 字段升级） |
| **v1.8 移除** Blargg 单 suite cache | ~~`blargg-roms-...`~~ | — |

**重要变化**：v1.8 把 ROM cache key 从"按 downloader 散列"改为"按 mirror_ref 散列"。`f11qa_mirror_pin.json` 任何改动（包括 mirror_ref 升级、新增 suite、删除 suite）都会触发 ROM 重新 fetch + checksum 校验。65 条 rom-suite 用例共用一个 cache key（节省 cache 空间）。

---

## 六、tests.json v1.8 schema 详细字段

### 6.1 顶层字段

```jsonc
{
  "schema_version": "1.8",
  "suite_id": "f11qa-v1.8",
  "generated_at": "2026-09-24T13:30:00Z",
  "policy": {
    "license_allowed": ["PD", "zlib", "GPL-2.0", "GPL-3.0", "CC0", "CC-BY", "MIT", "BSD", "Apache"],
    "min_coverage": 1.0,  // 100% cases must have valid binary or vendor_state skip
    "mirror_repo": "https://github.com/Laffinty/f11qa-rom-mirror",
    "r4_gate_thresholds": {
      "total_min": 120,
      "fail_to_pass_max": 0,
      "mirror_snapshot_check_required": true
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
| `license` | string | ✓ | `PD` / `zlib` / `GPL-2.0` / `MIT` / 等（rom-suite 强制 ∈ mirror accepted set） |
| `mirror_ref` | string | ✓（**rom-suite 强制**） | **v1.8 新增**：git tag，如 `v1.8.0-mirror`；与 `f11qa_mirror_pin.json` 的 mirror_ref 一致 |
| `mirror_path` | string | ✓（**rom-suite 强制**） | **v1.8 新增**：相对镜像源根路径，如 `nestest/nestest.nes` |
| `vendor_state` | enum | ✓（**rom-suite 强制**） | **v1.8 新增**：`vendored` / `advisory` / `pending-vendor`；决定 fetch + checksum 行为 |
| `input` | object | ✓ | 包含 `binary` + `args`（rom-suite 用例不再需要 `downloader`，由 `mirror_ref`/`mirror_path` 推导） |
| `expected` | object | ✓ | exit_code + 可能的 trace metrics |
| `timeout_seconds` | number | ✓ | 与 v1.17 一致 |
| `tags` | string[] | ✓ | NESDev wiki tag / TASVideos-required 等 |
| `failure_means` | string | ✓ | `blocking` / `advisory`；`vendor_state=advisory/pending-vendor` 时默认 `advisory` |
| `provenance` | string | ✓ | 演进历史（追加 `F11QA-recast-2026-09-21; mirror-ref-2026-09-24`） |

### 6.3 字段迁移示例

| v1.17 | v1.8 |
|---|---|
| `{ "id": "blargg_cpu_instrs", "oracle_type": "B", ... }` | `{ "kgmqa_id": "kgmqa-032-cpu-instrs-blargg", "legacy_id": "blargg_cpu_instrs", "kind": ["rom-suite", "harness-rust"], "license": "PD", "mirror_ref": "v1.8.0-mirror", "mirror_path": "blargg/cpu/instr_test_v5_all.nes", "vendor_state": "vendored", "spec_source": "nesdev.org/wiki/Emulator_tests#instr_test_v5", ... }` |

---

## 七、R4 Gate 与发布等级（调整版）

### 7.1 R4 Gate 阈值对比

| 阈值 | v1.17 | v1.8 | 变化理由 |
|---|---|---|---|
| `total` | `≥ 39` | **`== 120`** | 数量已固定精确，浮动阈值无意义 |
| `fail_to_pass` | `== 0` | `== 0` | Phase 0.5-d 反作弊护栏不变；**注意** `vendor_state=advisory/pending-vendor` 用例的 fetch 失败不计 fail_to_pass |
| `mirror_snapshot_check` | N/A | **必须 PASS** | **v1.8 新增**：kgmqa-117 镜像源快照校验（取代原 `license_manifest_check`） |
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
| E | 矩阵未生成 / **mirror_snapshot_check** FAIL / 引擎启动失败 |

### 7.3 frozen baseline 升级

| 维度 | v1.17 | v1.8 |
|---|---|---|
| 文件 | `tests/fixtures/f11qa_baseline_frozen.json` | `tests/fixtures/f11qa_baseline_frozen.json` |
| 条目数 | 47 | **120** |
| 字段 | `{ results: { "kgmqa_id": bool } }` | 同 v1.17 + `kgmqa_id` 替代 `id` |
| vendor_state 维度（v1.8 新增） | — | baseline 中每条 rom-suite 用例记录 `vendor_state`；CI 比对 vendor_state 变化作为 advisory 通道 |

---

## 八、迁移 Checklist（v1.17 → v1.8）

### 8.1 文件 / 路径改名（git mv）

| 旧路径 | 新路径 | 类型 |
|---|---|---|
| `tests/f11qa/` | `tests/f11qa/` | 目录 |
| `tests/f11qa_direct_main.cpp` | `tests/f11qa_direct_main.cpp` | C++ 源 |
| `src/f11qa_bridge.h` | `src/f11qa_bridge.h` | C 头 |
| `src/f11qa_bridge.cpp` | `src/f11qa_bridge.cpp` | C++ 实现 |
| `src/rust/crates/f11qa/` | `src/rust/crates/f11qa/` | Rust crate 目录 |
| `.github/workflows/f11qa.yml` | `.github/workflows/f11qa.yml` | workflow |
| `docs/tech/F11QA.md` | `docs/tech/F11QA.md` | active doc |
| `tests/fixtures/f11qa_baseline_frozen.json` | `tests/fixtures/f11qa_baseline_frozen.json` | manifest |
| `tests/fixtures/f11qa_full_baseline.json` | `tests/fixtures/f11qa_full_baseline.json` | manifest（如存在）|
| `scripts/download_blargg_roms.ps1` | **删除**（合并到 `fetch_roms_from_mirror.ps1`） | downloader |
| 全部 `scripts/download_kagami_*.ps1`（如有）| **删除**（合并到 `fetch_roms_from_mirror.ps1`） | downloader |
| **v1.8 新增** | `tests/fixtures/f11qa_mirror_pin.json` | mirror pin |
| **v1.8 新增** | `tests/f11qa/mirror_snapshot_check.cpp` | kgmqa-117 |
| **v1.8 新增** | `scripts/fetch_roms_from_mirror.ps1` | fetch 脚本 |
| **v1.8 新增** | `scripts/verify_mirror_pin.ps1` | pin 校验 |
| **v1.8 删除** | `tests/fixtures/f11qa_license_manifest.json`（不复存在） | — |
| **v1.8 删除** | 18 个 `tests/fixtures/<suite>_manifest.json` | — |

### 8.2 文本内字符串替换

| 旧字符串 | 新字符串 | 影响范围 |
|---|---|---|
| `F11QA` | `F11QA` | active docs / workflow / source comments / commit messages（新） |
| `f11qa` | `f11qa` | manifest / file paths / config keys |
| `f11qa` | `f11qa` | Rust crate name / binary name |
| `f11qa` | `f11qa` | C function prefix / binary prefix / gitignore 模式 |

### 8.3 新增（增量工作）

| 新增项 | 文件 | 内容 |
|---|---|---|
| `tests/f11qa/mirror_snapshot_check.cpp` |  | kgmqa-117 |
| `scripts/fetch_roms_from_mirror.ps1` |  | 单一 fetch 脚本（取代 22 downloader） |
| `scripts/verify_mirror_pin.ps1` |  | mirror_ref pin 校验 |
| `tests/fixtures/f11qa_mirror_pin.json` |  | mirror_ref pin 记录 |
| `src/rust/crates/f11qa/src/config_store.rs` |  | kgmqa-118 |
| `src/rust/crates/f11qa/src/pixbuf_pool.rs` |  | kgmqa-119 |
| `src/rust/crates/f11qa/src/state_facade.rs` |  | kgmqa-120 |
| `tests/fixtures/f11qa_baseline_frozen.json` |  | 120 项基线 |
| `tests/tests.json` | 重写 | schema v1.8 |

### 8.4 不改的项

- **历史 docs (`docs/history/`)**：完整保留 F11QA 命名
- **git tag / branch 历史**：保留
- **历史 commit messages**：保护 git history

---

## 九、阶段化执行计划（10 Phase，**v1.8 修订：14 周 → 12 周**）

| Phase | 周期（估）| 内容 | 交付物 |
|---|---|---|---|
| **Phase 1：改名 + 路径迁移** | 1 周 | git mv + 字符串批量替换 | F11QA 名称落地；CI 仍跑 v1.17 子集 |
| **Phase 2：扁平 schema + 120 项 manifest** | 2 周 | 编写 v1.8 schema；填充 47 个 v1.17 用例（kgmqa_id + kind + license + mirror_ref + mirror_path + vendor_state）+ 73 个第三方用例 | `tests/tests.json` v1.8；schema validator |
| **Phase 3：镜像源接入协议** | **0.5 周**（原 2 周） | 编写 `scripts/fetch_roms_from_mirror.ps1` + `tests/fixtures/f11qa_mirror_pin.json` + kgmqa-117 `mirror_snapshot_check.cpp` | kgmqa-117 PASS；fetch 脚本可用于 v1.8 R4 gate |
| **Phase 4：mirror snapshot + 65 条 rom-suite 用例 vendor_state 标注** | 1 周 | 65 条 rom-suite 用例逐一标注 vendor_state（✅/⏸/❌）；mirror pin 首个 tag `v1.8.0-mirror` 由 OWNER 打 | kgmqa-117 PASS；65 条用例 vendor_state 字段填齐 |
| **Phase 5：第三方 ROM runner 扩展** | 3 周 | 适配 kevtris trace / pinobatch 自识别 / tepples 多测试 / FDS 系列 等；⚠️ **vendor_state=advisory 用例的 runner 实现仍需做**（即使 fetch 跳过，runner 要能处理 fetch-failure 信号并跳过） | kgmqa-048 ~ kgmqa-112 全部可跑 |
| **Phase 6：新增 Rust crate 单元（118-120）** | 1 周 | 在 f11qa crate 内补 TypedConfig / PixBufPool / State 三组对偶测试 | kgmqa-118/119/120 PASS |
| **Phase 7：R4 gate 升级** | 1 周 | 修改 workflow：total==120 严格 + mirror_snapshot_check 前置 + vendor_state 通道 | R4 gate v1.8 |
| **Phase 8：frozen baseline 升级** | 0.5 周 | 录入 120 项基线（含 vendor_state） | f11qa_baseline_frozen.json |
| **Phase 9：CI 实战 + R4 验证** | 2 周 | 跑 wip1.8 → main；至少 3 次连续 CI green | release grade B 或 A |
| **Phase 10：收口 + 文档归档** | 0.5 周 | `docs/plans/FCEUX11-v1.8_F11QA-构建计划.md` → `docs/history/plans/`；`docs/tech/F11QA.md` active | v1.8 发布 |

**总周期估**：**12 周**（约 3 个月；原 v0.1 草案估 14 周，节省 2 周因为镜像源替代了 22 个 downloader 编写）。

---

## 十、回归与基线

### 10.1 已知风险

| 风险 | 等级 | 缓解 |
|---|---|---|
| **单点依赖镜像源 OWNER（v1.8 新增）** | 高 | 镜像源 OWNER 即 F11QA OWNER（同一身份）；CI cache 持久化已 fetch 的 ROM；mirror_ref 锁死后即使镜像源 main 改动也不影响 |
| 镜像源 tag 不可达 / 仓库被删 | 中 | CI 立即报错要求 OWNER 处理；OWNER 在镜像源重打 tag 或 fork 镜像源 |
| TakuikaNinja / rainwarrior 暂搁用例长期 advisory | 低 | 决策 10a：保留 kgmqa_id，advisory 通道；镜像源 OWNER 推进 vendor 时自动升级 |
| 总用例数 120，CI 时长增加 | 中 | 复用 blargg runner 单次跑完 180 ROM；bisqwit 4 用例 < 5s；增量 ~30s |
| 第三方 ROM $6000 协议不统一（部分用屏幕 hash） | 中 | runner 适配多协议（已 v1.17 实现），加 TASVideos-required tag |
| FDS 子系统精度可能爆出大量 advisory | 低 | FDS 子系统不是 v1.18 重点，advisory 进 baseline 即可；TakuikaNinja 4 个用例本身 ⏸ advisory 不计入 baseline regression |
| `vendor_state` 误标（pending-vendor → blocking 误用） | 中 | kgmqa-117 校验：fetch step 必 skip pending-vendor 用例；CI 失败立即可见 |
| mirror_ref 升级时 cache 失效导致 CI 时长突增 | 低 | `actions/cache@v4` restore-keys 支持部分恢复；最长增量 ~5min |

### 10.2 兼容性矩阵

| 旧（v1.17）行为 | 新（v1.8）行为 | 兼容性 |
|---|---|---|
| `id: "blargg_cpu_instrs"` | `kgmqa_id: "kgmqa-032-cpu-instrs-blargg"` | 旧 id 移入 `legacy_id` 字段，可双向查询 |
| `oracle_type: "A"` / `"B"` | 字段删除，由 `kind[]` 表达 | **破坏性**：旧脚本读 oracle_type 会失败——文档化迁移 |
| `suite_id: "f11qa-v1.17"` | `suite_id: "f11qa-v1.8"` | 破坏性 |
| `f11qa_baseline_frozen.json` | `f11qa_baseline_frozen.json` | 路径破坏性 |
| 47 项 cases | 120 项 cases | **强制**：旧 baseline 已包含 47 → 新 baseline 扩充到 120 |
| 单脚本 `download_blargg_roms.ps1` 拉取 | `scripts/fetch_roms_from_mirror.ps1` + mirror_ref pin | **破坏性**：旧 downloader 文件删除 |
| 隐式 license | 显式 `license` + `mirror_ref` + `mirror_path` + `vendor_state` | 破坏性：tests.json 字段全量重写 |

### 10.3 验收标准（v1.8 收口必须满足）

- [ ] `tests/tests.json` schema_version == "1.8"，suite_id == "f11qa-v1.8"，cases 数量 == 120
- [ ] 所有 active docs（`docs/tech/`、`docs/plans/`）中 F11QA → F11QA 改名完成
- [ ] CI workflow 改名 + wip1.8 触发分支 + mirror_snapshot_check 前置 + 单一 fetch 脚本
- [ ] `scripts/fetch_roms_from_mirror.ps1` 可从镜像源 mirror_ref tag 拉取全部 ✅ vendored ROM
- [ ] `tests/fixtures/f11qa_mirror_pin.json` mirror_ref 字段非空且为有效 git tag
- [ ] kgmqa-117 mirror_snapshot_check 在 CI 上稳定 PASS
- [ ] 65 条 rom-suite 用例的 mirror_ref/mirror_path/vendor_state/license 四元组填齐且与镜像源一致
- [ ] R4 gate v1.8 在 3 次连续 CI 上 PASS（grade B 或 A）
- [ ] `docs/plans/FCEUX11-v1.8_F11QA-构建计划.md` → `docs/history/plans/` 归档
- [ ] `docs/tech/F11QA.md` active 文本反映 v1.8 状态
- [ ] v1.17 的 `f11qa_baseline_frozen.json` 仍保留为对照参考（**不删除**）

---

## 十一、决策点（回签栏）

本计划 v0.2 待用户确认以下 10 个决策点。**决策 1-10 全部回签后进入施工期（Phase 1 启动）**。

- [x] **决策 1**：是否接受 `F11QA → F11QA` 重命名（含 git mv + active docs + workflow 改名，历史归档与 commit 保持原状）？ — **✅ 用户回签（v0.1）**
- [x] **决策 2**：是否接受扁平清单 + 10 种 `kind` multi-tag 模型（替代 Oracle A/B 二分）？ — **✅ 用户回签（v0.1）**
- [x] **决策 3**：是否纳入 §三 F 节列出的 65 项第三方 ROM 用例（kgmqa-048 ~ kgmqa-112）？ — **✅ 用户回签（v0.1）**
- [x] **决策 4**（v0.1 原方案）：是否接受"每套件独立 downloader + license manifest + SHA-256 校验"模式？ — **⚠️ v0.2 修订**：被决策 7 完全替换，不再适用
- [x] **决策 5**：总目标数 **120 项**（kgmqa-001 ~ kgmqa-120）是否符合 v1.8 期望？ — **✅ 用户回签（v0.1）**
- [x] **决策 6**：是否同意 Phase 1 立即开工（git mv 改名 → 风险最低的 Phase，无需新代码）？ — **✅ 用户回签（v0.1）**
- [x] **决策 7**（v0.2 新增）：v1.8 §四整套 22-downloader 体系如何处理？ — **✅ 用户回签：选项 A — 完全替换**（2026-09-24；22 个 downloader → 1 个 `scripts/fetch_roms_from_mirror.ps1`）
- [x] **决策 8**（v0.2 新增）：mirror_ref 钉版策略？ — **✅ 用户回签：选项 8b — pin git tag**（如 `v1.8.0-mirror`；OWNER 协调发版）
- [x] **决策 9**（v0.2 新增）：Holy Mapperel（47 ROM）在 kgmqa-077 如何呈现？ — **✅ 用户回签：选项 3A — aggregate 单用例**（47 ROM 在 runner 内部循环）
- [x] **决策 10**（v0.2 新增）：镜像源 ⏸ 暂搁 / ❌ 未 vendor 用例如何处理？ — **✅ 用户回签：选项 10a — 保留编号 + advisory**（kgmqa_id 保留，failure_means 默认 advisory，OWNER vendor 后自动升级 blocking）

**回签完成 → 进入施工期**。Phase 1（改名 + 路径迁移）可立即开工，1 周；Phase 2-10 顺次推进。

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
| 2 | [christopherpow/nes-test-roms](https://github.com/christopherpow/nes-test-roms) | 社区归档仓库（上游主源之一） |
| 3 | [pinobatch/holy-mapperel](https://github.com/pinobatch/holy-mapperel) | Holy Mapperel（zlib 许可证）|
| 4 | [pinobatch/240p-test-mini](https://github.com/pinobatch/240p-test-mini) | 240pee |
| 5 | [Quietust qmtpro.com](https://www.qmtpro.com/~nes/?news=2012) | scanline / exram / Color Bars |
| 6 | [AprNes Testing Methodology](https://www.baxermux.org/myemu/AprNes/report/methodology.html) | 测试方法论 |
| 7 | [Nesium Test ROM Suite](https://deepwiki.com/mikai233/nesium/6.3-test-rom-suite) | 40+ 套件分类 |
| 8 | [Pinky Test Statuses](https://pioptiop.uk/starrhorne/nes-rust) | Pinky 模拟器测试套件 |
| 9 | `docs/tech/F11QA.md`（v1.17）→ `docs/tech/F11QA.md`（v1.8） | 体系总览 |
| 10 | `docs/history/plans/FCEUX11-1.17_计划.md` | v1.17 前置基线 |
| **11** | **`https://github.com/Laffinty/f11qa-rom-mirror`** | **v1.8 新增：第三方 ROM 镜像源（OWNER @Laffinty）** |
| 12 | 镜像源 `LICENSES.md` | 每 ROM 的 license + upstream URL + author + SHA-256 |
| 13 | 镜像源 `SHA256SUMS.txt` | 单个仓库全量 ROM SHA-256 索引 |
| 14 | 镜像源 `docs/ROM_SOURCE_MAP.md` | 上游源映射 + vendor 优先级 + 失效应对 |

### 附录 C：v1.8 编号速查表

| 区段 | 编号范围 | 类型 | vendor_state 分布 |
|---|---|---|---|
| A | kgmqa-001 ~ kgmqa-019 | unit-cpp | — |
| B | kgmqa-020 | unit-hdr | — |
| C | kgmqa-021 ~ kgmqa-026 | harness-cpp | — |
| D | kgmqa-027 ~ kgmqa-043 | harness-rust（blargg 系列） | 部分 ✅ 部分 ❌ pending-vendor |
| E | kgmqa-044 ~ kgmqa-047 | lua-api | — |
| F.1 | kgmqa-048 | kevtris nestest | ✅ vendored |
| F.2 | kgmqa-049 ~ kgmqa-052 | bisqwit | 49-51 ✅ vendored / 52 ⏸ advisory |
| F.3 | kgmqa-053 ~ kgmqa-067 | blargg 扩展 | 53-57 ✅ / 58-67 ❌ pending-vendor |
| F.4 | kgmqa-068 | Damian Yerrick volume_tests | ✅ vendored |
| F.5 | kgmqa-069 ~ kgmqa-080 | Mapper-specific | 大部分 ✅ / 72-74 ❌ pending-vendor |
| F.6 | kgmqa-081 ~ kgmqa-085 | FDS 子系统 | 81 ✅ / 82-85 ⏸ advisory |
| F.7 | kgmqa-086 ~ kgmqa-098 | NES 2.0 + mapper 边缘 | 部分 ✅ 部分 ⏸ / 98 ⏸ advisory |
| F.8 | kgmqa-099 ~ kgmqa-100 | TV 显示 | ✅ vendored |
| F.9 | kgmqa-101 ~ kgmqa-111 | 输入 | 部分 ✅ / 102-104 ❌ pending-vendor |
| F.10 | kgmqa-112 | NEStress | ✅ vendored |
| G | kgmqa-113 ~ kgmqa-114 | static-analysis | — |
| I | kgmqa-115 | perf | — |
| J | kgmqa-116 | smoke | — |
| 新增 | kgmqa-117 | mirror_snapshot_check（static-license） | — |
| 新增 | kgmqa-118 ~ kgmqa-120 | unit-rust | — |

**vendor_state 汇总**：
- ✅ vendored：约 35 个 kgmqa
- ⏸ advisory：约 11 个 kgmqa（TakuikaNinja 4 + bisqwit-052 + rainwarrior 暂搁 6 + zip-aware 待实现 1）
- ❌ pending-vendor：约 14 个 kgmqa（blargg ppu/apu/sprdma 余项 + AWJ 余项 + natt + tepples 余项 + 31test）
- 不适用（unit/harness/perf/smoke/static）：约 60 个 kgmqa

### 附录 D：术语对照表

| 旧术语（F11QA） | 新术语（F11QA v1.8） | 说明 |
|---|---|---|
| Oracle A | `kind: unit-cpp / unit-hdr / unit-rust / harness-cpp / lua-api / smoke / static-analysis` | 旧 Oracle A 拆分为多种 kind |
| Oracle B | `kind: rom-suite / harness-rust` | 旧 Oracle B 主要对应这两类 |
| Oracle type | `kind` (multi-tag) | 字段废弃 |
| `id` | `kgmqa_id` | 重命名 |
| `f11qa-v1.17` | `f11qa-v1.8` | suite_id |
| `f11qa_baseline_frozen.json` | `f11qa_baseline_frozen.json` | 重命名 |
| `f11qa-runner.exe` | `f11qa-runner.exe` | Rust crate 二进制 |
| `f11qa_*` | `f11qa_*` | C function / binary 前缀 |
| `f11qa` | `f11qa` | Rust crate name |
| **v1.7 隐式 license** | **`license` + `mirror_ref` + `mirror_path` + `vendor_state` 四元组** | **v1.8 新增四字段** |
| **v1.7 隐式 downloader** | **`scripts/fetch_roms_from_mirror.ps1`** + mirror_ref pin | **v1.8 单一 fetch 脚本** |
| **v1.7 无 license_manifest** | **`tests/fixtures/f11qa_mirror_pin.json` + kgmqa-117 `mirror_snapshot_check`** | **v1.8 镜像源快照校验** |

---

**版本记录**：
- v0.1（2026-09-21）：草案，由 F11QA 工作组提出；待决策点回签
- v0.2（2026-09-24）：接入 `Laffinty/f11qa-rom-mirror` 镜像源；22 downloader → 1 fetch 脚本；新增 `mirror_ref`/`mirror_path`/`vendor_state` 三字段；新增 §四 §4.6 接入协议；kgmqa-117 由 `license_manifest_check` 改为 `mirror_snapshot_check`；决策 1-10 全部回签（其中决策 7-10 为 v0.2 新增）；总工期 14 周 → 12 周
