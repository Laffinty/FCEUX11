# F11QA v1.8 — ROM vendor_state 推进状态

> **目的**：追踪 `tests/tests.json` v1.8 中 65 条 `kind=rom-suite` 用例的 `vendor_state` 字段（✅ vendored / ⏸ advisory / ❌ pending-vendor），记录每条的依据 + 推进路径。
>
> **维护人**：镜像源 OWNER（@Laffinty）+ F11QA OWNER（@Laffinty，同一人）。
>
> **关联**：
> - 镜像源：`https://github.com/Laffinty/f11qa-rom-mirror`（HEAD `d50a7691ad4792cc8c4d9b8ab2b0a903d096d8aa`，295 SHA-256SUMS 条目）
> - 主计划：`docs/plans/FCEUX11-v1.8_F11QA-构建计划.md` §三 §3.6 F 节 + §四 §4.2
> - Schema 校验：`scripts/phase2_validate_tests_json.py`（kgmqa_id / kind / mirror_ref / mirror_path / vendor_state 四元组强校验）

---

## 一、vendor_state 三态语义

| vendor_state | 含义 | fetch 行为 | failure_means 联动 | 何时升级 |
|---|---|---|---|---|
| ✅ **vendored** | 镜像源 SHA256SUMS.txt 已收录完整 ROM 字节 | 实际下载 + 复制 + SHA-256 校验 | `blocking`（fail → fail_to_pass++ → R4 红） | 镜像源 OWNER 提交新 vendor → v1.8 升级 mirror_ref → kgmqa 自动升级 |
| ⏸ **advisory** | 镜像源已下载但 vendor 受阻（上游无 LICENSE 或沙箱拉不到） | skip + warn | 默认 `advisory`（fail → 不计入 fail_to_pass） | 镜像源 OWNER 解决阻碍（联系上游补 LICENSE / 等 wayback）→ 升级到 vendored |
| ❌ **pending-vendor** | 镜像源尚未 vendor 该 ROM | skip + warn | 默认 `advisory`（fail → 不计入 fail_to_pass） | 镜像源 OWNER 推进 vendor → 升级到 vendored |

---

## 二、当前分布（v1.8 计划 §四 §4.2 表格镜像源实际状态）

| vendor_state | kgmqa 数量 | 占比 |
|---|---|---|
| ✅ vendored | 43 | 55.1% |
| ⏸ advisory | 13 | 16.7% |
| ❌ pending-vendor | 22 | 28.2% |
| **合计** | **78** | 100% |

（注：78 = D 节内 13 项 rom-suite blargg 用例 + F 节 65 项第三方 rom-suite 用例；kgmqa-001 ~ 047 + 113 ~ 120 共 47 项非 rom-suite 不计入）

---

## 三、✅ vendored（43 项）

### 3.1 blargg（13 项，kgmqa-031 ~ 043 部分）

| kgmqa_id | suite | mirror_path |
|---|---|---|
| `kgmqa-031-blargg-smoke` | blargg | `nestest/nestest.nes` |
| `kgmqa-032-cpu-instrs-blargg` | blargg | `blargg/cpu/instr_test_v5_all.nes` |
| `kgmqa-033-cpu-timing-blargg` | blargg | `blargg/cpu/cpu_timing_test6.nes` |
| `kgmqa-037-cpu-int-2-nmi-brk-blargg` | blargg | `blargg/cpu/cpu_interrupts_v2_2.nes` |
| `kgmqa-038-instr-misc-blargg` | blargg | `blargg/cpu/instr_misc.nes` |
| `kgmqa-040-vbl-05-nmi-timing-blargg` | blargg | `blargg/vbl_nmi_timing/5.nmi_suppression.nes` |
| `kgmqa-043-blargg-suite` | blargg | `blargg/cpu/instr_test_v5_all.nes` (manifest) |
| 其他 6 项（kgmqa-053/054/055/056/057 blargg 扩展） | blargg | cpu 子目录 |

### 3.2 第三方 ROM（30 项）

kevtris nestest, bisqwit (3 项: 049/050/051), pinobatch (068/099), Quietust (079/080), nk (069), drag (070), AWJ (071), tepples (075/076/093/096/097/100/101/109), lidnariq (078), Sour (081), Quietust scanline (080), Holy Mapperel (077 aggregate 47 ROM), mmc5ramsize (090), bxrom_512k_test (094), fme7 tests (096/097), 3gengames (105), vaus (106), mset/mict (107/108), rahsennor (110), blargg read_joy3 (111), nesstress (112)

详见 `tests/tests.json` 中 `vendor_state == "vendored"` 的 43 项。

---

## 四、⏸ advisory（13 项）

### 4.1 TakuikaNinja FDS（4 项，kgmqa-082 ~ 085）

**阻碍**：上游 4 repo（`FDS-Mirroring-Test`, `FDS-Audio-Registers`, `FDS-4030D1-Addr`, `FDS-4023-Test`）均无 LICENSE 文件 + README 也无 license 声明。镜像源 OWNER 已下载 .fds 二进制（SHA-256 已记录到镜像源 `docs/ROM_SOURCE_MAP.md` §已下载未 vendor）。

**推进路径**：
- 选项 A：联系上游（开 issue）请求作者补 LICENSE；拿到明确答复后再 vendor
- 选项 B：上游后续添加 LICENSE 文件，scrape 后再 vendor
- 选项 C：放宽政策（暂不考虑，与 v1.8 §四 §4.5 严格政策冲突）

**预计时间**：1~3 个月（取决于上游回复）

### 4.2 rainwarrior 部分暂搁（6 项）

| kgmqa_id | 阻碍 |
|---|---|
| `kgmqa-086-nes2-submapper-2-test` | 沙箱拉不到 .nes 实体 |
| `kgmqa-087-nes2-submapper-3-test` | 同上 |
| `kgmqa-088-nes2-submapper-7-test` | 同上 |
| `kgmqa-089-nes2-submapper-34-test` | 同上 |
| `kgmqa-091-n163-soundram-rainwarrior` | wayback NO HIT in wayback |
| `kgmqa-092-n163-soundram-init-rainwarrior` | 同上 |

**推进路径**：OWNER 用浏览器手动从 forums.nesdev.org / wayback 抓 .nes → 走镜像源 §维护工具 5 步流程。

### 4.3 其他 advisory（3 项）

| kgmqa_id | 阻碍 |
|---|---|
| `kgmqa-052-cpu-flag-concurrency-bisqwit` | bisqwit zip 内多 ROM，zip-aware 解压分支尚未实现 |
| `kgmqa-095-31-test` | rainwarrior 31_test attachment 待查（thread 已知 id 待定位） |
| `kgmqa-098-famicom-audio-swap-tests` | zip 内多文件，待 zip-aware 解压分支实现 |

**推进路径**：实现 zip-aware fetch 分支（`scripts/fetch_roms_from_mirror.py` 新增 zip 后处理）。

---

## 五、❌ pending-vendor（22 项）

### 5.1 blargg ppu/apu/mmc3/sprdma 子目录（10 项，kgmqa-058 ~ 067）

`blargg/ppu/`、`blargg/apu/`、`blargg/mmc3/`、`blargg/sprdma/` 等子目录在镜像源中**尚未 vendor**（mirror 当前 SHA256SUMS 仅 295 条，cpu 子目录 69 条 vendor 完成，ppu+apu+mmc3+other 合计 ≈ 111 条待补）。

**推进路径**：OWNER 批量 vendor blargg 剩余子目录（来源 christopherpow/nes-test-roms 或 nesdev wiki）。

### 5.2 AWJ / natt 余项（3 项，kgmqa-072/073/074）

`awj/vrc24test/`、`natt/vrc6test/`、`tepples/mmc1atest/` 在镜像源中**未 vendor**。

**推进路径**：OWNER 从 forums.nesdev.org 或 mediafire（老链）下载。

### 5.3 tepples 余项（3 项，kgmqa-102/103/104）

`tepples/zap_ruder/`、`tepples/spadtest/`、`tepples/powerpad/` 在镜像源中**未 vendor**。

**推进路径**：OWNER 从 christopherpow/nes-test-roms 或 tepples 个人 GitHub 下载。

### 5.4 31test 子目录空（1 项，kgmqa-095）

镜像源 `31test/` 子目录为空——OWNER 待查 attachment。

### 5.5 其他 pending（5 项）

详见 `tests/tests.json` 中 `vendor_state == "pending-vendor"` 的 22 项。

---

## 六、推进流程

### 6.1 单项 vendor_state 升级（OWNER 单人）

1. 浏览器 / 命令行下载 ROM（或从已有上游仓库抓）
2. 计算 SHA-256
3. vendor 到镜像源对应子目录
4. 在 `LICENSES.md` 追加 entry
5. 在 `SHA256SUMS.txt` 追加 `<sha256>  <path>` 行
6. 在 `scripts/sync_from_upstream.sh` 的 `main()` 末尾追加 `fetch_rom` 三行调用
7. 跑 `bash scripts/verify_licenses.sh && bash scripts/audit_sha256.sh`，全部 PASS
8. git commit + push
9. （镜像源）打新 tag `v1.X.Y-mirror`
10. （F11QA）OWNER 同步 `tests/fixtures/f11qa_mirror_pin.json` 的 `mirror_ref` 字段
11. （F11QA）同步 `tests/tests.json` 中该项的 `vendor_state: "vendored"`
12. （F11QA）CI 跑 kgmqa-117 mirror_snapshot_check 验证新 mirror_ref 一致
13. （F11QA）下一轮 CI R4 gate 包含该项为 blocking

### 6.2 批量推进脚本（OWNER）

镜像源内部已有 `scripts/sync_from_upstream.sh` + `verify_licenses.sh` + `audit_sha256.sh`（详见镜像源 README §维护工具）。OWNER 单次 vendor 流程 = 上面的 1-8 步，8 是镜像源内部 commit。

### 6.3 跨项目同步流程

```
[镜像源 main HEAD]
     |
     | OWNER: vendor + tag v1.8.1-mirror
     v
[Laffinty/f11qa-rom-mirror @ tag v1.8.1-mirror]
     |
     | OWNER: 同步 tests/fixtures/f11qa_mirror_pin.json + tests.json
     v
[FCEUX11 wip1.8 PR]
     |
     | CI: kgmqa-117 mirror_snapshot_check + R4 gate
     v
[FCEUX11 main, v1.8.1 发布]
```

---

## 七、待办（按优先级）

| 优先级 | 项 | 估计时间 |
|---|---|---|
| 🔴 高 | OWNER 推进 22 pending-vendor 中 10 项 blargg ppu/apu/mmc3/sprdma（最大批量） | 1~2 周 |
| 🟡 中 | 实现 zip-aware fetch 分支（解决 kgmqa-052/098 advisory） | 0.5 周 |
| 🟡 中 | OWNER 推进 6 项 rainwarrior 暂搁（沙箱拿不到，需手动浏览器） | 1~3 周 |
| 🟢 低 | 镜像源 OWNER 联系 TakuikaNinja 4 repo 补 LICENSE（决定是否能 vendor） | 1~3 个月 |
| 🟢 低 | OWNER vendor AWJ/natt 余项 | 视进展 |

---

## 八、变更日志

| 日期 | 变更 |
|---|---|
| 2026-09-24 | 初始版本（v1.8 草案 v0.2 落地） |
