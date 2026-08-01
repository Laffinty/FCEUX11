# FCEUX11 翻译质量复测报告（优化后验证）

> [!WARNING]
> **STATUS: OBSOLETE（已过时）** — 本文档为 2026-07-25 的「优化后复测」（v2 轮，结论"有条件通过"），
> 已被 `docs/history/reports/FCEUX11-1.15_翻译质量最终复测报告.md`（v3 发布评审，2026-07-26 ✅ 通过）取代。
> 仅作翻译审计链的历史保留，请勿作为当前结论引用。

**复测对象**：优化后的 12 个 `.ts` 文件 + `glossary(1).txt`（对照组：2026-07-25 首审的原始文件）
**复测方法**：新旧文件逐条 diff（精确到 message 级）+ 首审报告 P0/P1/P2 清单逐项核销 + 全量回归扫描（乱码、占位符、助记符、拼写、语法）
**复测日期**：2026-07-25

---

## 1. 复测结论

**总体判定：有条件通过。** 本轮优化共修改 **198 处译文 + 术语表 14 行**，首审报告中的核心语义误译（Shadow 八语种连环误译、"4 计分"、测试模式、NSF 音效、Trainer 训练器）和文字硬伤（韩语汉字混入/乱码、泰语 24 处错拼、西语 truc）**已全部修复且修复质量高**——Shadow 各语种均按建议译为 Space Shadow 并同步补进了术语表，泰语 movie 全量统一到 มูฟวี่ 后连错拼问题也一并消除，日语 51 处"読込/読み込み"分裂和 27 处"動画/ムービー"分裂一次清零。**但有三类问题阻碍无条件放行**：① 法语的 2 项 P0（4 处双重转义乱码、3 处 ROM 阴阳性错误）**原样未动**；② 批量替换在韩语引入 **2 处助词语法回退**（무비을→应为 무비를、중단점로→应为 중단점으로）；③ 越/泰/中（繁）各有清单内项目未修（详见 §4）。修复这 9 处（fr 7 + ko 2）即可达到发布标准。

**关键数字**：

| 指标 | 首审 | 复测 | 变化 |
|------|------|------|------|
| 译文改动总数 | — | **198 处** | ja 57 / ko 49 / th 28 / vi 26 / zh_CN 12 / de 8 / zh_TW 5 / hi 5 / es 4 / fr 3 / ar 1 |
| 翻译质量问题总数（11 语种） | 209 | **122** | **−87（−42%）** |
| P0 级（语义误译+文字硬伤） | 60 | **9**（全部在 fr/ko） | −51 |
| U+FFFD 乱码 | 3 | **0** | 清零 ✅ |
| 双重转义 `&apos;` | 4 | 4（fr 未修） | 不变 ❌ |
| 拼写错误（ภาพยนตร์/truc/묵비/활성化） | 38 | **0** | 清零 ✅ |
| 术语表缺陷（G1–G8） | 8 | 1（G4 未修） | 修 7 ✅ |
| 批量替换引入的回退 | — | **2**（ko 助词） | 新增 ⚠️ |

![优化前后对比](audit_assets/retest_before_after.png)

---

## 2. 首审清单逐项核销

### 2.1 P0（语义误译与文字硬伤）：应修 8 组，实修 6 组，2 组未动

| # | 首审条目 | 状态 | 复测证据 |
|---|----------|------|----------|
| 1 | Shadow → Space Shadow（8 语种） | ✅ **全修** | zh_CN`Space Shadow（光线枪）`、zh_TW`Space Shadow（光線槍）`、ko`스페이스 섀도우`、de/fr/es/th/vi/ar`Space Shadow`；并新增术语表词条 |
| 2 | zh "4 计分" → Four Score（3 处） | ✅ **全修** | `连接 Four Score（四人适配器）`（简/繁）+ TAS 编辑器 `Four Score`；并新增术语表词条 |
| 3 | zh_CN 五项：测试模式→测试图案 / 修改键→修饰键 / NSF 音效→NSF 音乐 / Trainer 训练器→Trainer 数据 / "随游戏自动读档存档"→"…（作弊码）" | ✅ **全修** | 五处均按建议值精确落地；zh_TW 同步修了测试图案与 NSF（但 Trainer/Auto Load 未同步，见 §4.2） |
| 4 | ko 文字硬伤 12 处（활성化×6、비활성화×2、스프라이트乱码、分、묵비×2） | ✅ **全修** | 全文扫描：활성化=0、묵비=0、FFFD=0、`Minutes⇒분` ✅ |
| 5 | th ภาพยนตร์×24 + 乱码 2 处 | ✅ **全修** | ภาพยนตร์ 残留 0，movie 统一为 มูฟวี่（=术语表）；`เลือกคลิปบอร์ดอีกครั้ง`、`บันทึกมูฟวี่อินพุต` 均正确 |
| 6 | es `truc`→`truco`（2 处） | ✅ **全修** | 且顺手统一为 `Añadir truco`，动词混用问题一并解决 |
| 7 | **fr 双重转义 4 处** | ❌ **未修** | `Points d&apos;Arrêt`、`Configuration d&apos;Entrée` 等 4 处原样保留，UI 仍会显示乱码 |
| 8 | **fr ROM 阴阳性 3 处** | ❌ **未修** | `Ouvrir un ROM`、`Dernier ROM utilisé`、`Fermer le ROM chargé` 原样保留（应为 une ROM / Dernière…utilisée / la ROM…） |

### 2.2 P1（术语统一）：应修 6 组，实修 5 组，1 组部分完成

| # | 首审条目 | 状态 | 复测证据 |
|---|----------|------|----------|
| 1 | movie 统一（ko→무비、ja→ムービー、th、ar） | ✅ 3/4 | ko/ja/th 全量统一；**残留동영상×3、動画×3 均为 AVI 视频文件语境，属正确区分**，不算遗漏。**ar 未统一**：Movie Options×2、Movie Play 仍用 تسجيل（术语表已改 فيلم，译文未跟进） |
| 2 | save state 统一（de、ko、hi、ja） | ✅ 基本修 | de `Zustand`残留 0，全量 `Spielstand`；hi 存档语境全改 `स्टेट`（残留 स्थिति 均为"status"含义，正确）；ja 术语表改 ステートセーブ/ロード；**ko 只改了 Load State From（세이브 스테이트 불러오기），Save State As 仍是"상태 다른 이름으로 저장"，菜单内不对称** |
| 3 | Add Cheat 统一（th/vi/fr/es） | ✅ **全修** | th`เพิ่มโกง`、vi`Thêm mã gian lận`、fr`Ajouter un code de triche`、es`Añadir truco`，四语种两语境全部一致 |
| 4 | ja 読込→読み込み（约 30 处批量） | ✅ **全修** | 読込 残留 0，51 处统一为 読み込み |
| 5 | zh 热键→快捷键（zh_CN 3 处） | ⚠️ **半修** | zh_CN 已改 3 处；**zh_TW 相同 3 处（熱鍵設定/設定熱鍵/熱鍵選擇）未改**，两文件处理不对称 |
| 6 | vi 双 & 44 处改尾置 | ⚠️ **修 23/44（52%）** | 剩余 21 处集中在 QAsmView/QHexEdit/ppuNameTableViewer 右键菜单（`Sửa &dấu gỡ lỗi ký hiệu(&S)` 等） |

### 2.3 P2（风格排版）：基本未动（符合预期，但需记账）

首审 P2 项大部分保持原状：fr Title Case（`Charger État &Depuis`/`Enregistrer État &Sous`）、`Jouer &NSF`、ja `FCEUX11について`（无空格×2）、hi 尾冒号×3、es `autofuego`×2、ko `수정 버튼`/`주사율`（HexEditor 语境错配）/`게지`/`자퍼`、de HexEditor `Bild&wiederholrate`、vi/th `Turbo Seek`（`Tìm kiếm turbo`/`ค้นหาเทอร์โบ`）、th 词中 &×23、zh_CN 尾批风格项（延迟计数/帧迟到计数/上次记录/图块与网格线/加载∥载入混用/Ctrl+左×4 等 17 处）。这些不阻断发布，但应进入下一轮清单（§5 已整理为可直接执行的表格）。

### 2.4 术语表修订核销（G1–G8 + 2 项新增）

| # | 条目 | 状态 | 说明 |
|---|------|------|------|
| G1 slow down·hi | ✅ | गति धीमी करें（乱码修复） |
| G2 reset·th | ✅ | รีเซ็ต（断字修复） |
| G3 find·hi | ✅ | ढूंढें（乱码修复） |
| G4 settings·ko | ❌ **未修** | 仍为日语汉字 **設定**（应为 설정）——术语表唯一残留缺陷 |
| G5 quit·th | ✅ | ออก（中文"退出"已移除） |
| G6 redo·es | ✅ | Rehacer（错字修复） |
| G7 Game Genie·ko | ✅ | 게임 지니（但 ko 译文 `게지 코드` 未跟进，见 §4.4） |
| G8 quick save·zh_TW | ✅ | 快速存檔（与"快速讀檔"对仗） |
| 新增 Shadow / Four Score | ✅ | 两词条 12 列译法完整入库 |
| ja save state / load state / frame advance | ✅ | 按首审建议改为 ステートセーブ/ステートロード/フレームアドバンス（向社区惯例对齐） |

README 与 en_keep_allowlist 本轮**未改动**（md5 与首审一致）。

---

## 3. 回归扫描：批量替换引入的 2 处新错误

对全部 198 处改动逐一做了语法与格式复查，发现韩语批量替换（동영상→무비、브레이크포인트→중단점）时**未处理助词搭配**，产生 2 处新语法错误：

| # | 模块 | 源文 | 当前译文 | 问题 | 应改为 |
|---|------|------|----------|------|--------|
| R1 | MovieOptionsDialog | Loading states in record mode will not… | 녹화 모드에서 상태 로드가 **무비을** 즉시 자르지 않으며… | 무비 以元音结尾，宾格助词应用 를 | …상태 로드가 **무비를** 즉시 자르지 않으며… |
| R2 | ConsoleDebugger | Emulator Paused on Lua Breakpoint | Lua **중단점로** 에뮬레이터 일시 중지 | 중단점 以辅音(ㅁ)结尾，方向助词应用 으로 | Lua **중단점으로** 에뮬레이터 일시 중지 |

两处均属"替换词与原词收尾音不同（동영상**을**→무비**를**、브레이크포인트**로**→중단점**으로**）但助词照搬"的典型批量替换事故。修复成本：2 分钟。**建议将此模式加入韩语审校清单**：任何术语批量替换后，全文扫描`무비을|무비은|중단점로|중단점를`等误配组合。

其余 196 处改动经占位符多重集合、`&` 助记符、十六进制、换行四项机器校验及人工抽读，**未发现其他回退**；ja 的"読み込みしても"（旧文"読込しても"遗留的口语化表达，规范为"読み込んでも"）为旧有问题被替换放大，仅作记录不计回退。

---

## 4. 分语种复测结论

### 4.1 zh_CN（30 → 17，评级 B → **A−**）

P0 七项全修（测试图案/修饰键/作弊码语境说明/Four Score×2/Shadow/NSF/Trainer），热键 3 处、状态槽 1 处术语统一完成。残留 17 处均为首审 🟠/🟡 级：**语义类**——`Lag Count⇒延迟计数`（宜"延迟帧数"）、`Frame Late Count⇒帧迟到计数`（宜"迟滞帧计数"）、`Log Last⇒上次记录`（宜"记录最近"）、`Draw Tile &Grid Lines⇒绘制图块与网格线`（"与"字歧义）；**术语类**——`家庭键盘∥家用机键盘`并存、`Mapper 号：∥映射器：`同框不一致、`加载∥载入`全文混用（11∶44）、`栈∥堆栈`混用；**风格类**——`消除/差异：/可能性/启用的作弊码/周期置位/粘贴插入/保存到文件另存为/读∥写入不对称`，以及 `Ctrl+左/Ctrl+右/Alt+左/Alt+右` 4 处按键字面量仍被翻译（功能风险项，建议优先于其他风格项处理）。

### 4.2 zh_TW（20 → 10，评级 B+ → **A−**，但与 zh_CN 修复不同步）

Shadow/Four Score/測試圖案/NSF 音樂檔案已修。**与 zh_CN 不对称的 4 处**：`熱鍵設定/設定熱鍵/熱鍵選擇`（zh_CN 已改快捷鍵，繁体未改）、`變更狀態槽`（zh_CN 已改存檔槽，繁体未改）。**P0 级残留 2 处**：`Trainer⇒訓練器`（zh_CN 已改"Trainer 数据"，繁体未跟进）、`Auto Load / Save with Game⇒隨遊戲自動讀檔/存檔`（zh_CN 已加"（作弊码）"语境，繁体未跟进）——这 2 处首审同样标 🔴，建议按 zh_CN 的改法补齐。另有 延遲計數/上次記錄 等小项与简中相同。

### 4.3 ja（12 → 10，评级 B+ → **A−**）

本轮改动量最大（57 处）：読込/読み込み 51 处统一、動画→ムービー 全量统一（残留 3 处動画均为 AVI 视频语境，正确）。**值得肯定**：Movie Options 同串两译已统一为 ムービーオプション。残留：`映像：×2 ∥ ビデオ`（Libav/Libgwavi 编码器页）、`音声：×2 ∥ オーディオ`（同页两名）、`ディスク切換∥切替`（2 处）、`ディスク取出し∥排出`（1 处）、菜单-提示不配对的 `クイック保存∥クイックセーブ`、`クイック読み込み∥クイックロード`、`FCEUX11について` 无空格×2、`Enable Game &Genie` 英文原样嵌入日文句。全部为一致性/风格级，无语义错误。

### 4.4 ko（22 → 8，评级 C+ → **B+**，进步最大但有 2 处回退）

首审 12 处文字硬伤**全部清零**（활성화/비활성화/스프라이트乱码/분/묵비），movie 全量 무비 化（残留동영상×3 为 AVI 语境，正确），breakpoint 全量 중단점 化（=Visual Studio 韩语标准），Load State From 统一为 세이브 스테이트 불러오기。**需立即修复的 2 处回退**：`무비을→무비를`、`중단점로→중단점으로`（§3）。**清单内未修 5 处**：`Modifier Button:⇒수정 버튼:`（歧义，宜 보조 버튼）、HexEditor `Re&fresh Rate⇒주사율(&F)`（显示器刷新率用词错配，宜 갱신 주기）、`Game Genie Code⇒게지 코드`（术语表已改 게임 지니，译文未跟进）、`Zapper⇒자퍼`（宜 재퍼 或保留 Zapper）、`Save State As⇒상태 다른 이름으로 저장`（与已统一的 Load State From 不对称，宜 세이브 스테이트 다른 이름으로 저장）。

### 4.5 de（5 → 1，评级 A− → **A**，全场最干净）

save state 三名并存（Zustand/Spielstand/Savestate）**全量统一为 Spielstand**（残留 Zustand=0），Shadow 已修。唯一残留：HexEditor `Re&fresh Rate⇒Bild&wiederholrate` 语境错配（宜 Aktualisierungsrate）。另：首审提及的 100 条调试器/配置页漏翻（译文=英文）本轮未处理——该项属完整性而非翻译错误，单独立项跟踪。

### 4.6 fr（18 → 17，评级 B− → **C+**，本轮最大落后者）

仅修 3 处（Shadow、Add Cheat×2）。**首审 7 处 P0 全部原样保留**：4 处双重转义（`Points d&apos;Arrêt` 等，UI 直接显示 `&apos;`）、3 处 ROM 阴阳性（`Ouvrir un ROM`/`Dernier ROM utilisé`/`Fermer le ROM chargé`）。另有 `Jouer &NSF∥Lire NSF`、Quick Save 两译、Title Case 菜单（`Charger État &Depuis`/`Enregistrer État &Sous`）未动。fr 是唯一"P0 零修复"的语种，建议下轮最高优先级。

### 4.7 es（5 → 2，评级 B+ → **A−**）

truc→truco 两处全修且统一为 `Añadir truco`（动词混用一并解决），Shadow 已修。残留仅 `autofuego`×2（首审 P2 建议项，宜"disparo automático"）。

### 4.8 vi（48 → 24，评级 C+ → **B**）

双 & 修复 23/44（52%），Add Cheat 统一、Shadow 已修。**残留 21 处双 &** 集中在调试器与查看器右键菜单（QAsmView×5、ppuNameTableViewer×5、QHexEdit×4、ppuPatternView×3、ppuViewer×2、spriteViewer×2），可用与上轮相同的正则法批量处理；另有 `Turbo Seek⇒Tìm kiếm turbo`（误译"寻找 turbo"，宜 Dò nhanh）未修。

### 4.9 th（38 → 26，评级 C → **B+**）

ภาพยนตร์ 24 处错拼**全量清零**（统一为术语表规定的 มูฟวี่），2 处乱码修复，Add Cheat 统一，Shadow 已修——首审全部 🔴 项已清。残留为风格级：`Turbo Seek⇒ค้นหาเทอร์โบ`（误译，宜 เลื่อนหาเร็ว）、& 嵌入泰语词中 23 处（如 `ตัวเลือก&มูฟวี่`，惯例应尾置 (&X)）、`โหลด&เร็ว∥โหลดด่วน` 小分裂。

### 4.10 hi（6 → 3，评级 B → **B+**）

存档语境 स्थिति→स्टेट 统一完成（残留 6 处 स्थिति 均为"status"语义，属正确保留）。残留：`Display on Scanline:` 尾冒号丢失×3。

### 4.11 ar（5 → 4，评级 B → **B**）

Shadow 已修。**movie 术语未统一**：术语表已改为 فيلم，但 `Movie Options⇒خيارات التسجيل`（×2）、`Movie Play⇒تشغيل التسجيل` 仍用 تسجيل，与 18 处 فيلم 并存；`Load State From⇒تحميل الحالة من` 介词悬空未补。RTL 真机走查建议维持。

---

## 5. 下一轮行动清单（按优先级）

**R0｜阻断项（9 处，0.5 小时内可完成）**

| # | 语种 | 位置 | 现在 → 改为 |
|---|------|------|-------------|
| R0-1 | ko | MovieOptionsDialog | `무비을` → `무비를` |
| R0-2 | ko | ConsoleDebugger | `Lua 중단점로` → `Lua 중단점으로` |
| R0-3 | fr | ConsoleDebugger | `Points d&apos;Arrêt` → `Points d'arrêt` |
| R0-4 | fr | ConsoleDebugger | `Émulateur Arrêté au Point d&apos;Arrêt` → `Émulateur arrêté au point d'arrêt` |
| R0-5 | fr | consoleWin | `Configuration d&apos;Entrée` → `Configuration de l'&entrée` |
| R0-6 | fr | consoleWin | `Configuration de l&apos;Enregistreur d&apos;État` → `Configuration de l'en&registreur d'état` |
| R0-7 | fr | consoleWin | `Ouvrir un ROM` → `Ouvrir une ROM` |
| R0-8 | fr | consoleWin | `Dernier ROM utilisé` → `Dernière ROM utilisée` |
| R0-9 | fr | consoleWin | `Fermer le ROM chargé` → `Fermer la ROM chargée` |

**R1｜清单内未修项（14 处，1 人时内）**

zh_TW：`Trainer⇒訓練器`→`Trainer 資料`、`隨遊戲自動讀檔/存檔`→`隨遊戲自動載入/儲存（作弊碼）`、`熱鍵×3`→`快捷鍵`、`變更狀態槽`→`變更存檔槽`；ko：`수정 버튼`→`보조 버튼`、HexEditor`주사율`→`갱신 주기`、`게지 코드`→`게임 지니 코드`、`Save State As`→`세이브 스테이트 다른 이름으로 저장`；de：HexEditor`Bild&wiederholrate`→`Aktualisierungsrate`；vi/th：`Turbo Seek`→`Dò nhanh`/`เลื่อนหาเร็ว`；ar：Movie 语境 `تسجيل×3`→`فيلم`；术语表 G4：ko 列 `設定`→`설정`。

**R2｜批量收尾（约 60 处，可脚本化+复核）**

vi 剩余 21 处双 & 尾置化；th 23 处词中 & 尾置化；zh_CN `Ctrl+左`等 4 处按键字面量回退英文；ja `切換→切替`、`取出し→排出`、编码器页 `映像→ビデオ`/`音声→オーディオ`、Quick 菜单-提示配对、`について` 前空格；hi 尾冒号×3；es `autofuego→disparo automático`×2；zh 两语 `加载∥载入`、`栈∥堆栈` 归一及 §4.1 风格项。

**R3｜流程建议（防止下一轮回退）**

韩语/日语等**有黏着助词或送り仮名的语种，术语批量替换后必须做"替换词+助词搭配"全文扫描**（本次 ko 的 2 处回退即此原因）；建议把 `무비을|중단점로` 类误配模式与 `&apos;`、U+FFFD 一并加入提交前检查。

---

## 附录：复测方法说明

1. **diff 基线**：新文件与首审原文件按 (context, source) 全序列对齐（英文源串零变化），共检出 198 处译文差异，全部逐一人工判读。
2. **核销依据**：首审报告《FCEUX11_各语种翻译质量审计报告.md》§3–§5 的每一条 🔴/🟠/🟡 条目均在新文件中回查定位，状态分为 ✅已修 / ⚠️部分修 / ❌未修。
3. **回归检查**：全量扫描 U+FFFD、`&apos;`/`&amp;`、占位符多重集合、十六进制保留、`&` 助记符丢失/重复、拼写模式（ภาพยนตร์/truc/묵비/활성化/게지）、韩语助词误配、日语送り仮名分裂。
4. **计数口径**：问题总数按"语义误译/文字硬伤/术语不一致/风格瑕疵"四类合并计数，与首审口径一致；de/fr/es 的漏翻条目（译文=英文）属完整性问题，不计入本表，单独跟踪。

*复测全部结论可凭"模块+源文"在对应 .ts 文件中直接检索验证。*
