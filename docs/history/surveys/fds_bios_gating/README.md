# FDS BIOS 门控 —— kgmqa-081 重分类取证页（D7-A）

> **STATUS: CLOSED（known-limit）**（重分类裁决 2026-11-07，**baseline 不翻**）
> 本页是 kgmqa-081 的唯一 RE 入口页：记录「为什么 081 在 headless 下永远是 0xFE」
> 的完整事实链，以及把它从「FDS IRQ 精度债」重分类为「BIOS 门控 known-limit」的裁决。
> 证据强度标注：`[实测]` = 有实测读数支撑；`[读码]` = 读源码推断，未复核运行。

---

## 1. 结论（一段话）

kgmqa-081（Sour FdsIrqTests v7 patched）在 headless F11QA 下**从未跑起来过**：
headless 缺受版权的 FDS BIOS（disksys.rom）→ `FDSLoad` 失败 → runner 输出硬编码
的 0xFE（ROM 加载失败标记）。**这不是 FDS IRQ 精度缺陷**，ROM 文件形态本身无缺陷
（教科书 kind=1）。baseline 的 0xFE 计数维持原样，不翻动。

## 2. 事实链（D7-A 取证，2026-11-07）

1. **ROM 文件形态无缺陷** `[实测]`：`tests/fixtures/fdsirqtestsV7_patched.fds`
   （65516 字节）是教科书 kind=1 —— `FDS` 魔数在偏移 0、header[4]=1（单面）、
   65516 = 16 + 65500 自洽。镜像源 `tests/fixtures/_mirror_snapshot/sour/fdsirqtests/`。
2. **0xFE 是硬编码的加载失败标记** `[读码]`：
   `src/rust/crates/f11qa/src/runner/blargg.rs:476`（单跑 Err 分支）与 `:531`
   （批量路径）在 ROM 加载失败时固定回填 `value: 0xFE`；有单测
   `single_run_emits_0xFE_on_load_failure` 盖章。
3. **BIOS 路径拼装落在盘根** `[读码]`：headless 从不调 `fceu11::SetBaseDirectory`
   （全仓唯一调用点 `src/drivers/Qt/config.cpp:509`，属 Qt 启动路径）→
   `BaseDirectory` 为空串 → `FCEU_MakeFName(FCEUMKF_FDSROM)`
   （`src/file.cpp:764`）拼出 `\disksys.rom` 落到盘根 → 文件不存在 →
   `FDSLoad` 返回 LOADER_HANDLED_ERROR → `LoadGame` 返回 NULL → runner 报 0xFE。
4. **旁证证伪（082/083/084 不构成「BIOS 可达」证据）** `[实测]`：takuikaninja
   目录不在镜像里（上游 4 repo 均无 LICENSE，`vendor_state: advisory`），
   082/083/084 的 PASS 是 advisory skip 路径的产物，与 BIOS 是否可达无关。
5. **$6000 假绿陷阱** `[实测]`：假 BIOS 下跑 600 帧后 `$6000 == 0x00` 是
   **从未跑起来的干净 RAM 值** —— `--frames 1` 同样得 0x00。即使将来解决了
   BIOS，现有 `$6000` 判据也**无法区分「真跑 PASS」与「未启动」**。

## 3. 决策记录（用户裁决 2026-11-07）

- kgmqa-081 记为 **BIOS 门控 known-limit**，非精度缺陷；**baseline 不翻**
  （0xFE 维持，不改 frames、不改 expected）。`tests.json` 仅改该条目的
  `known_limit` / `provenance` 两个字段。
- **未来真跑 FDS 套件的两个前置**（缺一不立项）：
  1. **合法 BIOS 获取**：disksys.rom 受版权保护，不进仓库、不进 CI 镜像；
     需用户自备，且 headless 侧要先能让 `BaseDirectory` / `odirs[FCEUIOD_FDSROM]`
     被设置（当前唯一调用点在 Qt 侧）。
  2. **$6000 未启动判据区分**：需要先验「核心已启动」（如 BIOS banner /
     RAM 被写过）再读 `$6000`，否则 PASS 不可信。
- 先例一致性：`tests/f11qa/core/fds_load_test.cpp:191-193` 在缺 BIOS 时
  TSKIP（"FDS BIOS missing"）—— 本仓库对「缺 BIOS」的既有口径就是跳过，
  不是 FAIL，与本裁决同向。

## 4. 复现

```powershell
# ① 当前状态（0xFE = 加载失败；工作目录 = 仓库根，先加载 MSVC 环境）
cargo run -p f11qa --bin f11qa-rom-runner -- `
  --kgmqa-id kgmqa-081-fds-irq-tests-sour --tests-json tests/tests.json --frames 600
# ROM 由 dispatcher 从 mirror_path 叶名派生到 tests/fixtures/（rom_runner.rs:388）。

# ② 假 BIOS 对照实验（证明 0xFE 是 BIOS 缺失而非 ROM 缺陷）：
#    在 BaseDirectory 解析位置（headless 下为空串 ⇒ 盘根）放一个任意内容的
#    假 disksys.rom，重跑 ① 同一命令：
#      - 0xFE 消失 ⇒ 证明加载路径确实在找这个文件；
#      - 但 $6000 仍为 0x00（假 BIOS 跑不起来），且 --frames 1 同样 0x00
#        ⇒ 600 帧后的 0x00 是「未启动的干净 RAM」，不是 PASS。
#    实验后删除假文件。此实验即事实链第 5 条的出处。

# ③ 先例对照（缺 BIOS 走 TSKIP 而非 FAIL）
ctest -R f11qa_core --output-on-failure   # fds_load_test: "FDS BIOS missing" TSKIP
```

> 本页不附带修复立项：两个前置的任何一项都是业主决定。
