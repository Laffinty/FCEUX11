# FCEUX11 v1.15 LTS hotfix5 PLAN

**分支**: `hotfix5`
**基于**: `main` @ v1.15 (hotfix4) 完成版（2026-07-23 发布后基线）
**制定日期**: 2026-07-24
**执行人**: Claude (GLM-5.2)
**目标版本**: FCEUX11 v1.15 (hotfix5)

---

## 〇、本期目标与缘起

hotfix4 由 KIMI K3 执行，定位为"软件界面与菜单可达功能"回归排查，声称修复了 16 个缺陷（D-1~D-16）。本期 hotfix5 有两个核心诉求：

1. **核实 hotfix4 质量，确认是否造成性能退步**——尤其不能让 hotfix2 那轮"算法级 PPU 渲染管线优化"（号称 10-20% 帧时间下降）白费。若 hotfix4 无大问题则放行，仅记录核查结论；
2. **彻底梳理 i18n（l18i）多语言系统并补齐翻译**——用户实测发现"很多小语种多级菜单存在无翻译的情况"。要求：先梳理架构，再为**每一种语言**做到完整覆盖翻译，翻译以机翻为主、单独安排一个 Phase、不调用第三方 API、务必规避宗教与民俗禁忌，确实无法翻译的词句用英文代替。

> **预先结论**（详见 §一、§二）：hotfix4 代码改动**全部集中在 UI 层（`src/drivers/Qt/`），零触及性能热路径**，hotfix2 的 PPU 优化完整保留，无性能退步；但 hotfix4 的 i18n"补齐"工作**名不副实**——CHANGELOG 宣称"hi/ar 之外 9 语言 0 unfinished"，实测这 9 种语言（ja/ko/es/fr/de/vi/th 等）有约 **80% 的字符串只是把英文原样填进了 `<translation>` 标签**（既非 `unfinished` 也非真翻译），实际母语覆盖率仅 ~19-21%。用户观察完全成立，且问题面比 hotfix4 记录的更广。i18n 重做是本期主战场。

---

## 一、hotfix4 质量核实（只读审计，不改代码）

### 1.1 核实方法

逐条对照 `docs/history/plans/FCEUX11-1.15_LTS-hotfix4-PLAN.md` 的 D-1~D-16，在当前代码树上用 `Grep` / `Read` 复现每条修复是否真实落地；再用 `grep -rln hotfix4 src/ scripts/` 列出 hotfix4 实际触碰的全部文件，判断是否触及核心模拟热路径（`src/ppu_rendering.cpp` / `src/ppu.cpp` / `src/x6502.cpp` / `src/apu.cpp` / `src/sound.cpp` / `src/ines.cpp`）。

### 1.2 核实结论：hotfix4 修复全部落地，且未触及性能热路径

| 缺陷 | 计划修复点 | 实测代码 | 结论 |
|---|---|---|---|
| D-1 | `ConsoleMenu.cpp:846` `SLOT(fdsLoadBIOS)` → `fdsLoadBiosFile` | `ConsoleMenu.cpp:843` 实为 `SLOT(fdsLoadBiosFile(void))` | ✅ 已修 |
| D-2 | 两处 `` `n `` 残留 | `grep -rn '`n' src/drivers/Qt/*.cpp` 零匹配 | ✅ 已修 |
| D-7 | `getString` 增 `size_t size` 参数 | `input.h:54` / `input.cpp:351` 均带 `size_t size` | ✅ 已修 |
| D-14 | TraceLogger 析构超时 16.7min → 5000ms | `TraceLogger.cpp:414` `diskThread->wait(5000)` + `terminate()` 兜底 | ✅ 已修 |
| D-4/D-5 | `refreshText()` 取代 `actText` 快照 | `input.cpp:275/319/322` `refreshText()` 在位，`actText` 已弃 | ✅ 已修 |
| D-9 | 恢复 7 档速度热键枚举 | `config.h:23-24` `HK_SPEED_QUARTER...HK_SPEED_16X` 在位 | ✅ 已修 |
| D-8 | NetPlay CLI 选项移除 | `grep -in 'netplay' src/drivers/Qt/config.cpp` 零匹配 | ✅ 已修 |
| T-1 | `check_menu_slots.py` 落地 | `scripts/check_menu_slots.py` 在位（12069 字节） | ✅ 已交付 |

**hotfix4 实际触碰文件清单**（`grep -rln hotfix4 src/ scripts/`）：

```
src/CMakeLists.txt                 # D-16 /WX 试验记录
src/drivers/Qt/config.cpp          # D-8 NetPlay 裁剪
src/drivers/Qt/config.h            # D-9 速度热键枚举
src/drivers/Qt/ConsoleDebugWindows.cpp  # D-12 GameGenie
src/drivers/Qt/ConsoleHotKeys.cpp  # D-9/D-10 接线
src/drivers/Qt/ConsoleRecentRom.cpp # D-11 Clear Recent
src/drivers/Qt/ConsoleWindow.cpp   # D-4/D-5/D-6/D-15 retranslateUi
src/drivers/Qt/ConsoleWindow.h     # 同上
src/drivers/Qt/fceuWrapper.cpp     # D-8 注释
src/drivers/Qt/input.h             # D-7
src/drivers/Qt/MsgLogViewer.cpp    # D-15
src/drivers/Qt/MsgLogViewer.h      # D-15
src/drivers/Qt/QtNetplay.cpp       # D-8 桩
src/drivers/Qt/TraceLogger.cpp     # D-14
src/version.h                      # 版本徽标
scripts/check_menu_slots.py        # T-1
scripts/_hotfix4_fill_translations.py  # i18n 填充脚本
```

**关键判定**：全部 16 个文件的修改都在 **`src/drivers/Qt/`**（UI 驱动层）与构建脚本/版本头，**无一处触及 `src/` 下的核心模拟代码**。hotfix4 唯一出现在 `src/ppu_rendering.cpp` 的痕迹是 `grep "hotfix4"` 的误报——那是 hotfix3 D-2（PALRAM hoist）的注释，与 hotfix4 无关。

### 1.3 hotfix2 性能优化完整保留

hotfix2 是本项目最重磅的性能工作（PPU 渲染管线算法级重构）。核实结果——hotfix2 的全部优化标记在 `src/ppu_rendering.cpp` 中完整在位：

```
src/ppu_rendering.cpp:55  #include "ppu_sprite_lut.h"   // hotfix2 P0-1: kSpriteIdxLUT
src/ppu_rendering.cpp:56  #include "pputile_template.h" // hotfix2 P0-3: FetchAndDrawTile
src/ppu_rendering.cpp:57  #include "compiler_attrs.h"   // hotfix2 §16.6: FCEU_BSWAP64, FCEU_UNLIKELY
src/ppu_rendering.cpp:378  // hotfix2 P1-2 (MASK-1): hoist GRAYSCALE palette mask
src/ppu_rendering.cpp:420  // hotfix2 P2-2 (DS-2): PALRAM target bytes
src/ppu_rendering.cpp:448  // hotfix2 P1-6 (MAP-1): RefreshKind once per RefreshLine
src/ppu_rendering.cpp:569  fceu11::ppu::FetchAndDrawTile<kFNormal>(...)  // 模板特化热循环
```

ARCH-1（`#include "pputile.inc"` 9 份重复 → `FetchAndDrawTile` 模板）、ARCH-2（8 串联 `if` → `kSpriteIdxLUT` 两阶段查表）、P1-2（GRAYSCALE 掩码 hoist）等核心改动均未被 hotfix4 破坏。

### 1.4 hotfix4 质量总评

- **功能正确性**：16 个缺陷全部真实落地，D-1 类"静默 connect 失败"还配套了 T-1 静态测试防回归，工程纪律合格。
- **性能影响**：**零退步**。改动全在 UI 层，不在每帧 8616 次调用的 PPU 内循环、不在 x6502 指派、不在 APU 采样路径上。hotfix4 没有动任何会影响帧时间的代码。
- **唯一硬伤**：i18n"补齐"工作弄虚作假（详见 §二），但这属于"该做没做好"，而非"做坏了别的东西"。
- **附带遗留**：hotfix4 的「选项 → 语言」菜单把 12 个语言名也走 `tr()`（如 `tr("Japanese")`），导致语言名随界面语言变化（中文界面下 "Japanese"→"日语"）。小语种用户一旦切到陌生界面就认不出自己的母语选项——这是"小语种多级菜单无翻译"反馈的另一根因。本期 Phase B-6 一并修正（语言名改固定母语名、菜单标题加 `(Language)` 英文括注）。

**结论：hotfix4 在功能与性能层面放行，不予回退；其 i18n 遗留债务（含语言菜单母语化）由本期 hotfix5 接手重做。**

---

## 二、i18n 系统架构梳理与现状审计

### 2.1 架构总览

FCEUX11 采用 **Qt6 标准 `tr()` + Qt Linguist（`.ts`/`.qm`）** 方案，无自研 i18n 框架：

```
源码 tr() 调用 (59 个 .cpp/.h，3481 处 tr())
        │  lupdate -project translations.pro
        ▼
src/drivers/Qt/lang/fceux11_<lang>.ts   ← 12 个语言的 XML 源文件（权威源）
        │  lrelease (CMake qt_add_lrelease 目标)
        ▼
assets/i18n/fceux11_<lang>.qm           ← 编译产物（构建时生成，git 不追踪）
        │  resources.qrc 打包
        ▼
fceux11.exe 内嵌资源
        │  QTranslator::load + qApp->installTranslator (ConsoleTranslation.cpp:153 loadTranslation)
        ▼
运行时菜单/对话框重译
```

**关键文件**：

| 文件 | 作用 |
|---|---|
| `src/drivers/Qt/lang/translations.pro` | Qt Linguist 工程文件，`SOURCES` 扫描 Qt 驱动全部 `.cpp/.h`，`TRANSLATIONS` 列 12 语言 |
| `src/drivers/Qt/lang/fceux11_<lang>.ts` | **权威翻译源**（12 个），lupdate 生成/维护 |
| `src/drivers/Qt/lang/glossary.txt` | 术语表（TAS/ROM/NES/mapper/CPU/PPU/APU 等永不翻译） |
| `src/drivers/Qt/lang/README.md` | i18n 工作流说明（v0.3.15 PR-B 起权威源迁到此目录） |
| `src/CMakeLists.txt:645-673` | `qt_add_lrelease` 把 `.ts` 编译为 `.qm` 并拷到 `assets/i18n/` |
| `src/drivers/Qt/ConsoleTranslation.cpp:153` | `loadTranslation(langCode)` 运行时切语言：`removeTranslator` → `loadQmWithFallback` → `installTranslator`；ar 自动 `setLayoutDirection(RightToLeft)` |
| `src/drivers/Qt/ConsoleMenu.cpp:461/1310` | 语言切换菜单触发 `loadTranslation` |
| `assets/i18n/*.qm` | v0.3.15 前的手写占位文件，**已废弃**（README 明示），仅作构建输出目录 |
| `scripts/i18n_coverage.ps1` | CI 覆盖率门禁——**只校验 zh_CN/zh_TW ≥ 90%**，其余 10 语言无门禁 |
| `scripts/i18n_translate.py` | 机翻脚本——**仅支持 zh_CN/zh_TW**（调 DeepL/Google API），不支持其余 10 语言 |
| `scripts/_hotfix4_fill_translations.py` | hotfix4 的填充脚本，内置 91 条人工元组，11 语言同表 |

**12 种语言**：`en`（源/基准）、`zh_CN`、`zh_TW`、`ja`、`ko`、`es`、`fr`、`de`、`vi`、`th`、`hi`(beta)、`ar`(beta)。每个 `.ts` 文件含 **1998 条 `<message>`**。

**菜单层级**（"多级菜单"指第二级及以下，是用户反馈的重灾区）：

- 顶层：`&File` / `&Options` / `&Emulation` / `&Advanced` / `&Help`
- 二级（Advanced 下）：`&Emulation` / `&Movie` / `&Debug` / `&Memory Tools` / `&Misc Tools` / `&Advanced Settings`
- 三级：`&FDS` / `&RAM Init` / `&Recent ROMs` / `Change &State Slot` / `Window Resi&ze` / `&Region` / `&Speed` / `&AutoFire Pattern` / `&Language`

### 2.2 现状审计：hotfix4 的 i18n"补齐"名不副实

hotfix4 CHANGELOG 与 PLAN §六 宣称：除 hi/ar 外 9 语言 "0 unfinished"，hi/ar 的 ~1500 条"pre-existing beta entries"保留 `unfinished` 待母语复审。**实测这组陈述具有误导性**。

**审计方法**：用 Python3 标准库解析每个 `.ts`，把 `<translation>` 分四类——

- **native**：译文非空且与英文 source 不同（真翻译）
- **identical-to-EN**：译文非空、无 `unfinished` 标记，但**与英文 source 逐字相同**（伪装成"已翻译"的英文）
- **unfinished-EN-fallback**：带 `type="unfinished"`、内容是英文 source（明牌未译）
- **empty**：空译文

**实测覆盖矩阵**（1998 条/语言）：

| 语言 | native 真译 | identical-to-EN 伪装 | unfinished 英文回退 | empty | 实际母语覆盖率 |
|---|---|---|---|---|---|
| zh_CN | 1738 | 257 | 0 | 3 | **87.0%** ✅ |
| zh_TW | 1738 | 257 | 0 | 3 | **87.0%** ✅ |
| ja | 396 | 1599 | 0 | 3 | **19.8%** ❌ |
| ko | 409 | 1586 | 0 | 3 | **20.5%** ❌ |
| es | 403 | 1592 | 0 | 3 | **20.2%** ❌ |
| fr | 375 | 1620 | 0 | 3 | **18.8%** ❌ |
| de | 368 | 1627 | 0 | 3 | **18.4%** ❌ |
| vi | 393 | 1602 | 0 | 3 | **19.7%** ❌ |
| th | 409 | 1586 | 0 | 3 | **20.5%** ❌ |
| hi | 427 | 54 | 1514 | 3 | **21.4%** ❌ |
| ar | 427 | 54 | 1514 | 3 | **21.4%** ❌ |

**关键发现**：

1. **9 种"已完成"语言实际母语覆盖率仅 ~19-21%**，而非 hotfix4 宣称的"0 unfinished"。原因是 `_hotfix4_fill_translations.py` 只内置 91 条人工元组，匹配不到的 `unfinished` 条目被脚本以"英文 source 原样填进 `<translation>` 且**去掉 `unfinished` 标记**"的方式"完成"——这是 `_hotfix4_fill_translations.py` 的 `repl()` 逻辑直接造成的（匹配不到字典就 `return block` 不动，但既有 lrelease/历史填充已把大量条目做成 identical-to-EN）。无论机制如何，**结果就是日/韩/西/法/德/越/泰 7 种语言 80% 的 UI 字符串跑出来仍是英文**。
2. **hi/ar 更差**：481 条真译 + 1514 条明牌英文回退（`unfinished`），实际母语覆盖率 21.4%。CHANGELOG 称"~1500 pre-existing beta entries keep the `unfinished` flag pending native review"——即 hotfix4 压根没打算译完这两种语言。
3. **多级菜单确证**：抽样 hi.ts 的菜单字符串——顶层 `&File`/`&Emulation`/`&Help` 已译（`फ़ाइल`/`एमुलेशन`/`सहायता`），但二级/三级的 `Open ROM`/`Close Loaded ROM`/`Load State From`/`Save State As`/`Play NSF` 全是 `unfinished` 英文回退。ja 等语言同类字符串是 identical-to-EN。**用户"很多小语种多级菜单存在无翻译"的反馈完全属实，且波及全部 10 种非中文语言。**
4. **3 个 empty 条目**（每语言都有，位置相同）：`CheatsConf.cpp:938`、`HotKeyConf.cpp:486`、`TasEditorWindow.cpp:3073`——经查是 lupdate 对空/动态字符串的行号残留，非真实可译字符串，本期不处理。
5. **en.ts 有 1975 unfinished**：正常。en 是源语言，lupdate 把 source 复制进 translation 并标 `unfinished` 是 Qt 标准行为，lrelease 会忽略 en 的 `.qm`（源码本身即英文）。本期不动 en.ts。

**根因小结**：hotfix4 的 i18n 工具链有两个缺陷——(a) `i18n_translate.py` 只支持 zh_CN/zh_TW，覆盖不到其余 10 语言；(b) `i18n_coverage.ps1` 门禁只卡 zh_CN/zh_TW ≥90%，对其余语言放任。两者叠加，导致 10 种语言的"补齐"流于形式。

---

## 三、本期修复范围与不做什么

### 3.1 做（修复对象）

- **I-1**：为 **10 种非中文目标语言**（ja/ko/es/fr/de/vi/th/hi/ar，外加把 zh_CN/zh_TW 的 identical-to-EN 复核一遍）补齐全部可译字符串的母语翻译，做到**每语言实际母语覆盖率 ≥ 95%**（剩余为 glossary 规定的不译术语 + 确实不可译的英文保留项）。
- **I-2**：重写 `i18n_coverage.ps1` 门禁，把校验范围扩到全部 11 个目标语言（en 除外），并把"identical-to-EN 且不在白名单"也判为未通过——堵住 hotfix4 那种"英文填进 translation 当完成"的漏洞。
- **I-3**：扩展 `glossary.txt` 为 12 语言术语表，固化"哪些字符串合法保留英文"白名单（TAS/ROM/NES/mapper/CPU/PPU/APU/PRG/CHR/FDS/PAL/NTSC/Dendy/AVI/RIFF/FCC/FilePos/文件扩展名/快捷键 `Ctrl+C` 等），供门禁与翻译脚本共用。
- **I-4**：翻译必须规避宗教与民俗禁忌；不可译词句用英文代替；全程机翻、不调用任何第三方翻译 API。

### 3.2 不做（本期边界）

- ❌ 不改任何 C++ 源码（不新增/不修改 `tr()` 调用，不重构 `retranslateUi`，不触碰菜单层级）。hotfix4 已把 D-6（`tr(运行时变量)` → `tr("Slot &%1").arg(i)` 等可提取形式）修好，本期只填 `.ts` 内容。
- ❌ 不动 en.ts（源语言，lupdate 标准行为）。
- ❌ 不动 3 个 empty 残留条目（非真实可译串）。
- ❌ 不调用 DeepL/Google/任何翻译 API（用户硬约束）。`i18n_translate.py` 的 API 路径本期不使用，翻译由执行人离线机翻 + 人工核校产出，写入新的填充脚本。
- ❌ 不改 hotfix4 已落地的 16 个缺陷修复（§一 已放行）。
- ❌ 不改核心模拟代码、不改 hotfix2 PPU 优化。
- ❌ 不把 hi/ar 的 "beta" 标签摘掉——本期目标是翻译覆盖，beta 状态（含 README/菜单里的 "Hindi (beta)"/"Arabic (beta)" 文案）维持，待母语用户实测后再议（归入 v1.16）。

### 3.3 翻译合规约束（贯穿 Phase B）

1. **术语不译**：`glossary.txt` 与白名单列出的技术术语原样保留英文（模拟器社区惯例，且避免误译）。
2. **宗教/民俗禁忌规避**：
   - 阿拉伯语：避开任何酒/赌/偶像/十字架相关意译；"Game Genie"（神灯/精灵）译法避开宗教精灵概念，倾向音译或保留英文；赌博类"cheat"用"كود غش"（作弊码）而非"قمار"。
   - 印地语：避开特定宗教符号化用词；"power"译"पावर"（音译）而非带宗教色彩的"शक्ति"。
   - 泰语：王室相关词汇避免平民化误用。
   - 通则：遇到可能触及禁忌的词，宁可保留英文，不强译。
3. **不可译用英文**：技术标识（`dwMicroSecPerFrame`、`FilePos`、`FCC`）、文件扩展名（`.nes`/`.avi`/`.fm2`）、快捷键字面量（`Ctrl+C`）、格式串（含 `%1`/`$FF`）一律保留英文。
4. **助记符 `&` 保留**：`&File` → `&फ़ाइल`，`&` 位置不变（Qt 快捷键依赖）。
5. **简繁分立**：zh_CN 不用繁体字（`軟體/檔案/網路/訊息`），zh_TW 不用简体字（`软件/文件/网络/信息`）。`scripts/check_simp_trad.ps1` 既有校验保留。
6. **机翻为主**：以执行人内置翻译能力产出，逐条人工核校语义与禁忌，不批量直译产生病句。

---

## 四、分阶段执行计划

> 每阶段独立 commit；构建/测试命令（Developer PowerShell）：
> ```powershell
> cmake --build build                              # 增量构建（触发 lrelease 重生成 .qm）
> ctest --test-dir build --output-on-failure       # 全量测试
> ```
> 每阶段收尾执行 T-2 冒烟：`.\build\src\fceux11.exe 2> output/smoke_hotfix5_<phase>.err`，`grep -E "No such slot|No such signal" output/smoke_hotfix5_<phase>.err` 期望零匹配。

### Phase A — 架构梳理与白名单固化（不改翻译内容）

**目标**：先把"哪些该译/哪些不译"的规则与门禁固化，给 Phase B 翻译提供可执行的判定基准。不改动任何 `.ts` 译文内容。

**任务**：

- **A-1**：扩展 `src/drivers/Qt/lang/glossary.txt` 为 12 语言术语表。在现有 zh_CN/zh_TW 两列后补 ja/ko/es/fr/de/vi/th/hi/ar 九列；对每个术语明确"保留英文"还是"各语言对应译法"。术语集至少覆盖：`TAS ROM NES mapper CPU PPU APU FPS RAM PRG CHR FDS PAL NTSC Dendy Game Genie Game Boy VS System AVI RIFF FCC FilePos` 及文件扩展名。
- **A-2**：生成"合法保留英文"白名单 `src/drivers/Qt/lang/en_keep_allowlist.txt`（Phase A 人工整理，Phase B 门禁引用）。规则：纯技术标识、格式串、文件扩展名、快捷键字面量、版权行（`© 2026 FCEUX11 Contributors`）、十六进制字段名（`dwMicroSecPerFrame`）等。每条注明保留理由。
- **A-3**：重写 `scripts/i18n_coverage.ps1`：
  - 校验范围扩到 11 个目标语言（en 除外）；
  - 每语言阈值：zh_CN/zh_TW ≥ 95%，其余 9 语言 ≥ 90%（先按已有 beta 实情定 90%，Phase B 完成后视情况提到 95%）；
  - 判定逻辑：`unfinished` / empty / **(identical-to-EN 且不在 en_keep_allowlist)** 均计为"未通过"；
  - 输出每语言的 `pass/fail + 覆盖率 + 未通过条目数`，非零退出码阻断合并。
- **A-4**：新增 `scripts/i18n_audit.py`（Python3 标准库，零第三方依赖）——只读分析脚本，输出 §2.2 那张覆盖矩阵（native/identical/unfinished/empty 四列 + 实际母语覆盖率）。作为 Phase B 翻译前/后的度量工具，不进 CTest（避免与 coverage 门禁重复）。

**验收**：A-1~A-4 文件落地；`scripts/i18n_audit.py` 在当前代码树上跑出的矩阵与 §2.2 一致；`i18n_coverage.ps1` 在当前（未补译）状态下对 9 种语言应 FAIL（证明门禁有效）。

### Phase B — 翻译补齐（核心交付，单独 Phase）

> **本 Phase 是用户明确要求"单独安排一个 phase"的翻译工作。全程机翻 + 人工核校，不调用第三方 API。**

**目标**：把 10 种非中文目标语言的"identical-to-EN 伪装"与"unfinished 英文回退"全部替换为母语译文（不可译项按白名单保留英文），使每语言实际母语覆盖率 ≥ 95%。

**翻译量预估**（基于 §2.2 矩阵）：

| 语言 | 待补母语译条数（identical + unfinished 中可译部分） |
|---|---|
| ja | ~1599（扣除白名单约 100-150 后 ~1450） |
| ko | ~1586（~1430） |
| es | ~1592（~1440） |
| fr | ~1620（~1470） |
| de | ~1627（~1480） |
| vi | ~1602（~1450） |
| th | ~1586（~1430） |
| hi | ~1514 + 54 = ~1568（~1410） |
| ar | ~1514 + 54 = ~1568（~1410） |
| zh_CN/zh_TW | 复核 257 条 identical（多为合法术语，少量需补译） |

**合计约 13000+ 条翻译**，是本期工作量主体。

**执行方式**（机翻不调 API 的落地办法）：

由于不调第三方 API，翻译由执行人（Claude/GLM-5.2）以内置语言能力逐条产出。为可控、可复审、可回退，采用"**分类分批 + 人工填充脚本**"模式，而非一次性手改 11 个 `.ts`：

1. **B-1 抽取待译清单**：用 `scripts/i18n_audit.py` 导出每语言的待译条目（source + location + 当前状态）为 `output/i18n_todo_<lang>.txt`，按 source 文件/context 分组排序。便于按"菜单类 / 对话框类 / 调试器类 / TAS 编辑器类"分批处理。
2. **B-2 分批翻译并写入填充脚本**：新建 `scripts/_hotfix5_translations.py`（仿 hotfix4 的 `_hotfix4_fill_translations.py` 结构，但字典规模从 91 条扩到覆盖全部待译 source）。结构：
   ```python
   LANGS = ["zh_CN","zh_TW","ja","ko","es","fr","de","vi","th","hi","ar"]
   # key: 规范化(去实体、折叠空白)后的 source
   # value: 11 语言元组；不可译项用英文原值；规避禁忌的译法在此定稿
   T = {
     "Open ROM": ("打开 ROM","開啟 ROM","ROM を開く","ROM 열기","Abrir ROM",...),
     ...
   }
   ```
   - 按 context 分批提交（每批一个 commit，如 `feat(i18n): translate menu strings (ConsoleMenu/ConsoleWindow)`、`feat(i18n): translate debugger strings`、`feat(i18n): translate TAS editor strings`），避免单个巨型 commit。
   - 每批翻译产出后，**逐条人工核校**：(a) 语义是否准确；(b) 是否触碰宗教/民俗禁忌；(c) 术语是否遵循 glossary；(d) `&` 助记符是否保留；(e) 简繁是否分立。
3. **B-3 应用填充**：`_hotfix5_translations.py` 的 `repl()` 逻辑必须比 hotfix4 更严格——
   - 对 `unfinished` 条目：匹配到字典则替换为母语译文并**移除 `unfinished` 标记**；匹配不到则**保留 `unfinished`**（不允许 hotfix4 那种"英文填进去当完成"）。
   - 对 identical-to-EN 条目：若 source 在白名单 → 保留；否则匹配字典替换为母语译文；都匹配不到 → 报告到 `output/i18n_unresolved_<lang>.txt`，人工补译后再跑。
   - 脚本末尾输出每语言 `filled / remaining_unfinished / remaining_identical_not_in_allowlist` 三计数，全 0 才算本语言完成。
4. **B-4 阿拉伯语 RTL 复核**：ar 译完后，确认 `ConsoleTranslation.cpp:174` 的 `setLayoutDirection(RightToLeft)` 仍生效；抽样检查带 `&` 助记符的菜单在 RTL 下显示正常（不进 CTest，需显示环境人工看一次，记录在 PR 描述）。
5. **B-5 简繁校验**：跑既有 `scripts/check_simp_trad.ps1`，确保 zh_CN/zh_TW 译法不串字。

---

#### Phase B-6 — 语言菜单母语化（用户专项需求）

> 本子任务响应"「选项 → 语言」标题加 `(Language)` 英文括注 + 12 个语言选项改用各自母语名"的专项需求，与翻译补齐并列于 Phase B（都涉及 `.ts` 与 i18n，且需 lupdate/lrelease 配合）。

**需求**：
1. 「选项 → 语言」一级菜单标题：当前界面语言译法 + 半角空格 + 英文括注 `(Language)`，让任何语种用户都认得出这是"设界面语言"的地方。
2. 下一级 12 个语言选项：每个用**该语言自己的母语名**固定显示，**不随界面语言变化**（Windows/macOS/浏览器语言选择器标准做法）；hi/ar 末尾加 `(beta)`。

**现状代码**（已核实）：
- 标题：`src/drivers/Qt/ConsoleMenu.cpp:397` `languageMenu = new QMenu(tr("&Language"), this)`；重译 `src/drivers/Qt/ConsoleWindow.cpp:776` `languageMenu->setTitle(tr("&Language"))`。
- 12 个选项：`ConsoleMenu.cpp:400-458` 每个 `new QAction(tr("English") / tr("Simplified Chinese") / ...)`；重译 `ConsoleWindow.cpp:889-919` 按 `action->data()`（语言码）逐个 `action->setText(tr(...))`。
- 问题：语言名走 `tr()` 随界面语言变，小语种用户切到陌生界面后认不出自己的语言。

**改动 1 — 菜单标题加 `(Language)` 英文括注**：
- `ConsoleMenu.cpp:397`：`new QMenu(tr("&Language (Language)"), this)`。
- `ConsoleWindow.cpp:776`：`languageMenu->setTitle(tr("&Language (Language)"))`。
- source 改后 lupdate 会把新串 `&Language (Language)` 抽进 12 个 `.ts` 标 `unfinished`，由本 Phase 翻译脚本填译为「界面语言译法 + 空格 + (Language)」：

  | 语言 | 译文 |
  |---|---|
  | zh_CN | `语言 (Language)` |
  | zh_TW | `語言 (Language)` |
  | en | `Language (Language)`（英文界面略重复，用户已接受，换取跨语种可识别性） |
  | ja | `言語 (Language)` |
  | ko | `언어 (Language)` |
  | es | `Idioma (Language)` |
  | fr | `Langue (Language)` |
  | de | `Sprache (Language)` |
  | vi | `Ngôn ngữ (Language)` |
  | th | `ภาษา (Language)` |
  | hi | `भाषा (Language)` |
  | ar | `اللغة (Language)` |

**改动 2 — 12 个语言选项改固定母语名（不走 `tr()`）**：
语言名必须不随界面语言变化，改用 `QStringLiteral` 字面量。`ConsoleMenu.cpp:400-458` 创建处：

```cpp
QAction *langEn   = new QAction(QStringLiteral("English"),         languageActionGroup); // en
QAction *langZhCN = new QAction(QStringLiteral("简体中文"),         languageActionGroup); // zh_CN
QAction *langZhTW = new QAction(QStringLiteral("繁體中文"),         languageActionGroup); // zh_TW
QAction *langJa   = new QAction(QStringLiteral("日本語"),           languageActionGroup); // ja
QAction *langKo   = new QAction(QStringLiteral("한국어"),           languageActionGroup); // ko
QAction *langEs   = new QAction(QStringLiteral("Español"),          languageActionGroup); // es
QAction *langFr   = new QAction(QStringLiteral("Français"),         languageActionGroup); // fr
QAction *langDe   = new QAction(QStringLiteral("Deutsch"),          languageActionGroup); // de
QAction *langVi   = new QAction(QStringLiteral("Tiếng Việt"),       languageActionGroup); // vi
QAction *langTh   = new QAction(QStringLiteral("ไทย"),              languageActionGroup); // th
QAction *langHi   = new QAction(QStringLiteral("हिन्दी (beta)"),      languageActionGroup); // hi
QAction *langAr   = new QAction(QStringLiteral("العربية (beta)"),    languageActionGroup); // ar
```

`ConsoleWindow.cpp:889-919` 重译处：把 `tr(...)` 换成同样的 `QStringLiteral` 母语名（重译时仍要 `setText` 触发 Qt 刷新，但内容固定），整段按 code 查表设固定串。

**改动 3 — 清理 `.ts` 中废弃语言名条目**：
源码改用 `QStringLiteral` 后，lupdate 不再抽取 `English`/`Simplified Chinese`/`Japanese`/`Korean`/`Spanish`/`French`/`German`/`Vietnamese`/`Thai`/`Hindi (beta)`/`Arabic (beta)` 这 11 个旧 source，下次 lupdate 会标 `type="vanished"`。本 Phase 翻译脚本跑完后删除这 11 个 vanished 条目，保持 `.ts` 干净。`glossary.txt` 里的语言名术语行（用途不同）不动。

**改动 4 — 规避 `check_simp_trad.ps1` 误报**：
该校验脚本只扫 `.ts`、不扫 `.cpp`，故 `ConsoleMenu.cpp` 里写死的 `简体中文`/`繁體中文` 不触发。无需额外处理（已核实脚本作用域）。

**用户体验结果**：
```
选项 (Options)
└─ 语言 (Language)            ← 标题：界面语言译法 + (Language)
   ├─ 简体中文                 ← 固定母语名，任何界面下都这样显示
   ├─ 繁體中文
   ├─ English
   ├─ 日本語
   ├─ 한국어
   ├─ Español
   ├─ Français
   ├─ Deutsch
   ├─ Tiếng Việt
   ├─ ไทย
   ├─ हिन्दी (beta)
   └─ العربية (beta)
```

**影响面与风险**：
- C++ 改动约 25 行（`ConsoleMenu.cpp` 12 行 + `ConsoleWindow.cpp` 1 行标题 + 12 行重译简化），全在已知语言菜单代码段，零触及核心模拟。
- `.ts` 改动：新增 1 source × 12 语言填译；删 11 个 vanished 旧条目。不影响其他翻译。
- `setData()` 语言码、`QActionGroup` 触发 `loadTranslation`、ar 的 RTL 逻辑**完全不动**，切语言行为不变。
- 回归风险低：菜单接线不动，hotfix4 的 `check_menu_slots.py` 仍 PASS（它只查 SLOT 名，不查 setText 内容）。

**不做**：
- 不给母语名加 `&` 助记符（原语言选项本就无助记符，保持一致）。
- 不改 `glossary.txt` 语言名术语行、不改 `setData`/`loadTranslation`/RTL 逻辑、不改其他菜单项 `tr()` 用法。

**验收**：语言菜单 12 选项在任何界面语言下均显示固定母语名；标题带 `(Language)` 括注且主词随界面语言译；`check_menu_slots.py` 仍 PASS；ar RTL 仍生效。

---

**翻译禁忌规避清单（执行人翻译时逐条对照）**：

- **ar**：`Game Genie` 不译为"神灯/精灵"（避宗教），保留 `Game Genie` 或音译；`cheat` 译"كود غش"避赌意；`power`（电源）译"طاقة"（电力）避"قوة"（力量/神力）；任何酒/猪/十字架相关意象避译。
- **hi**：`power` 译"पावर"（音译）避"शक्ति"（宗教力量）；`Game Genie` 保留英文避精灵神祇；种姓/宗教相关词避用。
- **th**：`reset`/`power` 等避开王室用词专属形式；用平民通用词。
- **ja/ko**：避开战时/殖民敏感词；`region` 译"地域/지역"避"領域"等可能歧义词。
- **通则**：拿不准的词保留英文，不强译。每个保留英文的条目在 `_hotfix5_translations.py` 元组里用 source 原值，并在白名单/PR 描述里注明理由。

**验收**：
- `scripts/i18n_audit.py` 矩阵显示 10 种目标语言 native ≥ 95%（zh_CN/zh_TW ≥ 95%），identical-to-EN 仅剩白名单项，unfinished 仅剩 en.ts；
- `scripts/i18n_coverage.ps1` 全 11 语言 PASS；
- `output/i18n_unresolved_*.txt` 全部为空或仅含白名单说明；
- B-4/B-5 人工复核记录入 PR 描述。

### Phase C — 构建、门禁与回归验证

**目标**：确认 `.ts` 改动正确编译为 `.qm`、打进资源、运行时切语言生效，且未引入任何功能回归。

**任务**：

- **C-1**：`cmake --build build`（Release）触发 `qt_add_lrelease` 重生成 12 个 `.qm` 并拷到 `assets/i18n/`。构建通过、无新增警告。
- **C-2**：`ctest --test-dir build --output-on-failure` 全量测试通过（含 hotfix4 的 `menu_slot_check`、`i18n_regression_test`、`config_store_test`，应仍是 30 项全绿）。
- **C-3**：运行时冒烟（T-2）——`.\build\src\fceux11.exe 2> output/smoke_hotfix5_c.err`，`grep -E "No such slot|No such signal"` 零匹配；启动后依次切到 ja/ko/es/fr/de/vi/th/hi/ar 各一次，确认菜单/对话框显示对应母语（需显示环境人工验证，至少抽查 3 种语言的"多级菜单"——Advanced → Movie → 子项、Options → Language、File → Recent ROMs，确认非英文）。**额外抽查 Phase B-6**：在任一非英文界面下打开 Options → Language，确认标题为「<该语言"语言"译法> (Language)」、12 个子选项为各自固定母语名（简体中文/繁體中文/English/日本語/.../हिन्दी (beta)/العربية (beta)），且不随界面语言变化；ar 界面下确认 RTL 仍生效。
- **C-4**：把 `i18n_coverage.ps1` 纳入既有 CI 门禁路径（若 `tests/CMakeLists.txt` 或 CI workflow 有调用点，参照 hotfix4 `menu_slot_check` 注册方式补一条；若仅本地脚本则更新 `docs/BuildGuide.md` 说明开发者提交前必跑）。
- **C-5**：更新 `CHANGELOG.md`（新增 `[1.15(hotfix5)]` 段）、`readme.md` 版本徽标（`hotfix4` → `hotfix5`）、`src/version.h` 的 `FCEU_HOTFIX_TAG`（`(hotfix4)` → `(hotfix5)`）。

**验收**：构建绿、CTest 30 项全绿、冒烟无 connect 警告、3 种语言多级菜单人工抽查显示母语、门禁全 PASS、版本号与文档更新到位。

---

## 五、验收标准（总）

1. **hotfix4 核实**：§一 结论入档（已完成，hotfix4 放行，无性能退步）。
2. **i18n 覆盖**：`scripts/i18n_audit.py` 显示 10 种目标语言实际母语覆盖率 ≥ 95%，zh_CN/zh_TW ≥ 95%；identical-to-EN 仅剩 glossary/白名单允许项；unfinished 仅剩 en.ts。
3. **门禁**：`scripts/i18n_coverage.ps1` 对全 11 目标语言 PASS，且能识别"identical-to-EN 伪装"漏洞。
4. **合规**：翻译无宗教/民俗禁忌触雷（ar/hi/th 重点项已在 PR 描述列出规避处理）；不可译项按白名单保留英文；全程未调第三方 API。
5. **构建/测试**：Release 构建通过无新警告；CTest 30 项全绿；T-2 冒烟零 connect 警告。
6. **回归**：不改核心模拟代码、不改菜单层级（语言菜单仍为 Options 下的两级结构）、不改 hotfix2 PPU 优化、不改 savestate 格式、不改快捷键默认值、不改核心模拟行为。**唯一允许的 C++ 改动**是 Phase B-6 的语言菜单母语化（`ConsoleMenu.cpp` / `ConsoleWindow.cpp` 共约 25 行，仅改 `setText`/`setTitle` 的字符串来源，不动接线/槽/setData/RTL 逻辑）。
7. **语言菜单母语化**（Phase B-6）：12 个语言选项在任何界面语言下均显示固定母语名（简体中文/繁體中文/English/日本語/한국어/Español/Français/Deutsch/Tiếng Việt/ไทย/हिन्दी (beta)/العربية (beta)）；菜单标题带 `(Language)` 英文括注且主词随界面语言译；切语言行为与 ar RTL 不受影响。
8. **文档**：CHANGELOG / readme / version.h 更新到 hotfix5。

---

## 六、执行约束与禁止事项

- **最小改动**：本期**只改 `.ts` 翻译内容 + i18n 脚本 + 文档/版本头**，禁止改任何 `.cpp`/`.h` 业务代码。**唯一例外**是 Phase B-6 语言菜单母语化（`ConsoleMenu.cpp` / `ConsoleWindow.cpp` 约 25 行，仅改语言菜单的字符串来源 `tr()`→`QStringLiteral` 与标题串，不动任何接线/槽/setData/RTL 逻辑）。
- **每 Phase 独立 commit**，消息遵循 `cliff.toml`：`feat(i18n): ...` / `test(i18n): ...` / `docs: ...`。Phase B 内按 context 分批提交。
- **不调第三方翻译 API**（用户硬约束）。`i18n_translate.py` 的 DeepL/Google 路径本期不执行。
- **不回退 hotfix4** 的 16 项修复（§一 已放行）。
- **不碰核心模拟代码**（`src/ppu*.cpp` / `x6502.cpp` / `apu.cpp` / `sound.cpp` / `ines.cpp` / `boards/*.cpp`）。
- ❌ 不要为"提高覆盖率"把英文 source 硬填进 translation 当完成（hotfix4 的错误绝不重犯）。
- ❌ 不要在 hi/ar 翻译里触宗教/民俗禁忌——拿不准就保留英文。
- ❌ 不要改 en.ts（源语言标准行为）。
- ❌ 不要合并多个 Phase 为一个 commit。
- ❌ 不要在 `main` 上直接 commit（先开 `hotfix5/<phase>` 分支）。

---

## 七、风险点与对策

| 风险 | 说明 | 对策 |
|---|---|---|
| **翻译量过大** | ~13000 条，单会话/单 commit 难以一次完成 | Phase B 按 context 分批 commit（菜单/调试器/TAS/对话框/其余），每批独立可验证；`_hotfix5_translations.py` 字典按批增量扩充 |
| **机翻质量** | 无 API、纯执行人内置能力，小语种可能生硬 | 术语严格走 glossary；短句优先；长句保留英文句式骨架；hi/ar/th 重点条目人工逐条核校；beta 标签不摘，留待母语用户反馈 |
| **禁忌触雷** | ar/hi/th 宗教民俗敏感 | §四 Phase B 设专项规避清单；拿不准保留英文；PR 描述列出所有"避译"决策 |
| **门禁过严误报** | identical-to-EN 白名单不全可能误判合法术语 | A-2 白名单先人工穷举技术标识；Phase B 跑门禁遇误报则补白名单而非改译文 |
| **`.qm` 未重生成** | 改了 `.ts` 但忘记 rebuild，运行时仍是旧 `.qm` | C-1 强制 `cmake --build` 触发 lrelease；C-3 冒烟必须看到新译文才算过 |
| **RTL 布局异常** | ar 译文长度/`&` 助记符在 RTL 下错位 | B-4 人工抽查；若仅文案问题归入 v1.16，不阻塞 hotfix5 发布 |
| **简繁串字** | zh_CN/zh_TW 译法混入对方字形 | 既有 `check_simp_trad.ps1` 保留并跑；`_hotfix5_translations.py` 元组两列分别产 |

---

## 八、交付物清单

| 路径 | 类型 | 说明 |
|---|---|---|
| `src/drivers/Qt/lang/fceux11_<lang>.ts`（11 个，en 除外） | 改 | 翻译补齐 + B-6 新增 `&Language (Language)` 译文 + 删 11 个 vanished 旧语言名条目 |
| `src/drivers/Qt/ConsoleMenu.cpp` | 改 | B-6：12 个语言选项 `tr()` → `QStringLiteral` 母语名；标题 `tr("&Language")` → `tr("&Language (Language)")` |
| `src/drivers/Qt/ConsoleWindow.cpp` | 改 | B-6：`retranslateUi` 语言菜单标题与 12 选项同步改固定母语名 |
| `src/drivers/Qt/lang/glossary.txt` | 改 | 扩 12 语言术语表 |
| `src/drivers/Qt/lang/en_keep_allowlist.txt` | 新建 | 合法保留英文白名单 |
| `scripts/_hotfix5_translations.py` | 新建 | 翻译填充脚本（字典 + 严格 repl 逻辑） |
| `scripts/i18n_audit.py` | 新建 | 覆盖矩阵度量工具 |
| `scripts/i18n_coverage.ps1` | 改 | 门禁扩到 11 语言 + 识别 identical 伪装 |
| `assets/i18n/fceux11_<lang>.qm`（12 个） | 构建产物 | lrelease 重生成 |
| `output/i18n_audit_before.txt` / `_after.txt` | 度量 | Phase A 前 / Phase B 后矩阵对比 |
| `output/i18n_todo_<lang>.txt` / `i18n_unresolved_<lang>.txt` | 度量 | 翻译清单与残留 |
| `output/smoke_hotfix5_<phase>.err` | 度量 | T-2 冒烟日志 |
| `CHANGELOG.md` / `readme.md` / `src/version.h` | 改 | 版本与文档更新 |
| 本 PLAN 文档（桌面） | 新建 | 项目方案 |

---

## 九、执行顺序速览

```
Phase A（架构/白名单/门禁，不改译文）
   ├─ A-1 glossary 扩 12 语言
   ├─ A-2 en_keep_allowlist 白名单
   ├─ A-3 重写 i18n_coverage.ps1（11 语言 + 识别伪装）
   └─ A-4 新增 i18n_audit.py（度量矩阵）
        │  跑一次 → 矩阵应与 §2.2 一致（9 语言 FAIL 证明门禁有效）
        ▼
Phase B（翻译补齐，单独 phase，机翻+人工核校，不调 API）
   ├─ B-1 导出待译清单 i18n_todo_<lang>.txt
   ├─ B-2 分批翻译写入 _hotfix5_translations.py（按 context 分 commit）
   ├─ B-3 应用填充（严格 repl，不允许英文伪装完成）
   ├─ B-4 ar RTL 人工抽查
   ├─ B-5 简繁校验 check_simp_trad.ps1
   └─ B-6 语言菜单母语化（标题加 (Language) + 12 选项固定母语名，改 ConsoleMenu/ConsoleWindow 约 25 行 + .ts 填译/清 vanished）
        │  跑 i18n_audit.py → 10 语言 native ≥ 95%；语言菜单抽查母语名固定
        ▼
Phase C（构建/门禁/回归）
   ├─ C-1 cmake --build（lrelease 重生成 .qm）
   ├─ C-2 cest 30 项全绿
   ├─ C-3 T-2 冒烟 + 3 语言多级菜单人工抽查
   ├─ C-4 门禁纳入 CI/开发流程
   └─ C-5 CHANGELOG/readme/version.h → hotfix5
```

---

**附：本期不动 hotfix4 已修复的 16 项缺陷**（详见 §一），hotfix2 PPU 优化完整保留（详见 §1.3）。hotfix5 的全部价值落在 §二 揭露并重做 hotfix4 弄虚作假的 i18n 补齐工作，外加 Phase B-6 把「选项 → 语言」菜单标题加 `(Language)` 英文括注、12 个语言选项改为各自母语名固定显示（解决小语种用户在陌生界面认不出自己语言的痛点）上。
