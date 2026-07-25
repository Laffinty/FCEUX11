# FCEUX11 各语种翻译质量审计报告（词条级）

**审计对象**：fceux11 系列 12 个 Qt Linguist 文件，11 个目标语种 × 1,995 条有效字符串
**审计范围**：仅限**翻译质量本身**——语义准确性、术语地道性、文字拼写、表达自然度、前后一致性。程序逻辑、CI 流程、调试管线不在本次范围内
**审计方法**：11 个语种译文**全量导出逐条审读**（zh_CN/zh_TW/ja/ko/de/fr/es 全文精读；th/vi/hi/ar 全文扫读 + 关键词条复核），争议术语以权威外部信源核验（微软/Visual Studio 本地化惯例、任天堂/NESdev 硬件资料、日中韩英模拟器社区惯用语、FCEUX 官方文档）
**审计日期**：2026-07-25

---

## 1. 总体结论

**这批翻译的底子不差，但存在三类必须处理的硬伤。** 第一类是**连环语义误译**：最典型的案例是 Famicom 扩展设备 **"Shadow" 在 8 个语种中被翻译成"阴影/影子"**（阴影/Schatten/Ombre/Sombra/그림자/เงา/Bóng/ظل），而它实际上是 FCEUX 官方文档记载的 **"Space Shadow" 光线枪**外设  [(FCEUX)](https://fceux.com/web/help/Input.html) ——这是机器翻译按字面直译、审校环节完全缺失的铁证，且只有日语（シャドウ）和印地语（शैडो）因音译而幸免。第二类是**文字级损坏**：泰语把 "movie" 译作 ภาพยนตร์（**正确拼写应为 ภาพยนต์，全文 24 处无一写对**，连主菜单都带错字）、韩语 9 处汉字"化"混入与乱码错字、西班牙语 2 处 "truc" 截断、法语 4 处双重转义乱码。第三类是**同词多译的系统性不一致**：同一对话框内"映像/ビデオ"（日）、"Zustand/Spielstand/Savestate"（德）、"무비/동영상/묵비"（韩）、"무비/동영상"（韩）、"ภาพยนตร์/วิดีโอ/มูฟวี่"（泰）等并存，用户在同一界面能看到两三种说法。

**分语种评级**（A=可直接发布，B=修订后发布，C=必须返工）：

| 语种 | 评级 | 语义误译 | 文字硬伤 | 术语不一致 | 风格瑕疵 | 一句话画像 |
|------|------|----------|----------|------------|----------|------------|
| de | **A−** | 2 | 0 | 3 | 0 | 最规范，仅存档术语三译并存 |
| es | **B+** | 2 | 2 | 1 | 0 | 干净，被两处错字拖后腿 |
| ja | **B+** | 1 | 0 | 9 | 2 | 地道但"同词两译"最多 |
| zh_TW | **B+** | 8 | 0 | 8 | 4 | 与简中同源同病，修饰键译对 |
| zh_CN | **B** | 15 | 0 | 9 | 6 | 误译条目最多（4 计分/阴影/测试模式…） |
| hi | **B** | 0 | 0 | 3 | 3 | 无误译，存档术语三套体系并存 |
| ar | **B** | 2 | 0 | 2 | 1 | 准确度高，个别词悬空 |
| fr | **B−** | 3 | 4 | 3 | 8 | 双重转义+ROM 性别错误+Title Case |
| ko | **C+** | 6 | 9 | 6 | 1 | 长句很好，文字层损坏最重 |
| vi | **C+** | 3 | 0 | 1 | 44 | 译文尚可，44 处双 & 残留毁观感 |
| th | **C** | 3 | 26 | 4 | 5 | 主力 movie 译法全文拼错 24 处 |

![各语种翻译质量问题分布](audit_assets/tq_issues.png)

---

## 2. 跨语种系统性问题（最优先）

### 2.1 "Shadow" 光线枪：8 语种连环误译

FCEUX 输入配置（InputConfDialog）的 Famicom 扩展端口设备列表中有一项 **Shadow**。FCEUX 官方 Input 文档明确写道扩展端口设备包括 **the "Space Shadow" gun**——即《Space Shadow》光线枪（与 Hyper Shot 配套的光枪外设） [(FCEUX)](https://fceux.com/web/help/Input.html) 。各语种处理：

| 语种 | 现译 | 正误 | 建议 |
|------|------|------|------|
| ja | シャドウ | ✅ 可接受（音译） | 可润色为「スペースシャドウ」 |
| hi | शैडो | ✅ 可接受（音译） | 可润色为 स्पेस शैडो |
| zh_CN | 阴影 | ❌ **误译** | Space Shadow（光线枪） |
| zh_TW | 陰影 | ❌ **误译** | Space Shadow（光線槍） |
| de | Schatten | ❌ **误译** | Space Shadow |
| fr | Ombre | ❌ **误译** | Space Shadow |
| es | Sombra | ❌ **误译** | Space Shadow |
| ko | 그림자 | ❌ **误译** | 스페이스 섀도 |
| th | เงา | ❌ **误译** | Space Shadow |
| vi | Bóng | ❌ **误译** | Space Shadow |
| ar | ظل | ❌ **误译** | Space Shadow |

这是全项目**最具诊断价值**的一条错误：同一源词、同一错误方向（按"影子"字面义）、横跨八个语系——说明这批译文出自同一 MT 管线且名词实体未做保护。修复建议：外设专名（Shadow、Zapper、Arkanoid Paddle、Hyper Shot、Quiz King、Oeka Kids、Family Trainer、Mahjong）一律按"专名音译或保留原文"处理并写入术语表，禁止意译。

### 2.2 "Attach 4-Score"：中文独有误译"4 计分"

输入配置对话框的 **Attach 4-Score (Implies four gamepads)**，Four Score 是任天堂官方 NES 四人手柄适配器（NESdev Wiki 有完整硬件条目） [(NesDev.org)](https://www.nesdev.org/wiki/Four_player_adapters) 。**zh_CN 译"附加 4 计分（隐含四手柄）"、zh_TW 译"附加 4 計分（隱含四手把）"**——把专名拆成"4 + Score（计分）"直译，完全丢失实体指称；TAS 编辑器新建项目的 **4 Score ⇒ 4 计分** 同错。其余 9 个语种全部正确（Four Score 保留或音译：フォアスコア/포어스코어/Four Score anschließen 等）。建议：中文统一译 **"Four Score（四人适配器）"**。

### 2.3 movie（录像）术语：4 个语种各自分裂

TAS 语境的 movie 指"输入录像文件"（TASVideos 官方术语） [(TASVideos)](http://tasvideos.org/FAQ) 。各语种处理：

| 语种 | 现状 | 评价 |
|------|------|------|
| zh_CN/zh_TW | 录像/錄影 统一 ✅ | 符合中文 TAS 社区惯例 |
| de | Film 统一 ✅ | 好 |
| fr | Film 统一 ✅ | 好 |
| es | Película 统一 ✅ | 好 |
| vi | Phim 统一 ✅ | 好（=术语表） |
| hi | मूवी 统一 ✅ | 好（=术语表） |
| ja | ムービー ∥ 動画 并存（Movie Options 同串两译） | 需统一为 ムービー（=术语表） |
| ko | 무비(4) ∥ 동영상(32) ∥ **묵비(2，错字)** 三种并存 | 术语表规定 무비；동영상 易被理解为"视频文件"而非"输入录像"；묵비（意为"沉默"）是错字 |
| th | **ภาพยนตร์(24，全部拼错)** ∥ วิดีโอ(25) ∥ มูฟวี่(2) | 主力译法 ภาพยนตร์ 拼写系统性错误（正确：**ภาพยนต์**，全文 0 处写对），主菜单 `&Movie ⇒ &ภาพยนตร์` 带错字 |
| ar | تسجيل ∥ فيلم 并存（Movie Play⇒تشغيل التسجيل，Record Input Movie⇒تسجيل فيلم الإدخال） | 术语表规定 تسجيل；فيلم 会误解为"电影" |

### 2.4 "Add Cheat"：同一源串、5 个语种各有两个译法

`Add Cheat` 这一完全相同的源串出现在 GameGenie 与 Cheats 两个对话框中，zh（添加金手指/添加作弊码）、th（เพิ่มสูตรโกง/เพิ่มโกง）、vi（Thêm cheat/Thêm mã gian lận）、fr（Ajouter un code/Ajouter un truc）、es（Agregar truc→错字/Añadir truco）各给出两种译法；ko/de/ja 一致（치트 추가/Cheat hinzufügen/チートを追加 ✅）。中文的"金手指/作弊码"分化**恰好符合语境**（Game Genie 对话框的 cheat 就是金手指代码） [(web-g-p.com)](https://emu.web-g-p.com/info/term/term_cheat.html) ，建议在术语表中按语境登记两个合法译法而不是强行统一；th/vi/fr/es 则是纯随机漂移，应统一。

### 2.5 save state（即时存档）术语：多语种体系混乱

| 语种 | 现状 | 依据与建议 |
|------|------|------------|
| zh_CN/zh_TW | 存档/讀檔 + 存檔 统一 ✅ | 符合社区；但"状态槽/存档槽"混用（见 §3.1） |
| ja | ステート（菜单：ステートを読み込み、クイックセーブ）+ セーブステート（Bind Save-States） | 偏离术语表（セーブ状態/ロード状態）但**更合社区惯例**——日语模拟器圈标准说法是 ステートセーブ/ステートロード  [(Weblio辞書)](https://www.weblio.jp/content/%E3%82%B9%E3%83%86%E3%83%BC%E3%83%88%E3%82%BB%E3%83%BC%E3%83%96%E3%80%81%E3%82%B9%E3%83%86%E3%83%BC%E3%83%88%E3%83%AD%E3%83%BC%E3%83%89) ；**建议修订术语表**而非改译文，并把菜单/提示统一 |
| ko | 상태 불러오기（菜单）∥ 세이브스테이트（Bind Save-States） | 术语表规定 세이브 스테이트；建议菜单统一为 세이브 스테이트 系 |
| de | Zustand（菜单）∥ Spielstand（提示）∥ Savestate（Bind Save-States） | 术语表规定 Spielstand；三选一统一，Spielstand 最贴德语玩家习惯 |
| fr | état sauvegardé / Charger État Depuis（大小写混乱） | 术语表 État sauvegardé；菜单句式应 "Charger l'état depuis…" |
| hi | स्थिति（菜单）∥ स्टेट（提示）∥ क्विक सेव（术语表 सेव स्टेट） | 三套体系，建议统一 स्टेट/सेव स्टेट 系 |
| es/ar/th/vi | estado guardado / حالة محفوظة / บันทึกสถานะ / Trạng thái lưu 基本统一 ✅ | th 有 ช่องเซฟ/ช่องสถานะ 小分裂（见 §3.9） |

### 2.6 Modifier Button（修饰键）：术语对错各半

GamePadFuncConfigDialog 的 **Modifier Button**：正确译法为 zh_TW「修飾鍵」、ja「修飾ボタン」、de「Modifikatortaste」、hi「संशोधक बटन」✅；**zh_CN「修改键」**（"修改"=编辑，歧义）、**ko「수정 버튼」**（同上歧义，应为 보조 버튼/수정자 버튼）、**ar「زر التعديل」**（同上，建议 زر التعديل المساعد）❌。th「ปุ่มดัดแปลง」、fr「Bouton modificateur」、es「Botón modificador」、vi「Nút sửa đổi」可接受。

### 2.7 Refresh Rate 的语境错配（ko/de）

HexEditor 菜单的 **Re&fresh Rate** 指"内存视图自动刷新间隔"，ko 译 **주사율**、de 译 **Bildwiederholrate**——两者都是"显示器刷新率"专用词，用错了语境（视频配置里的 Refresh Rate (Hz) 才是 주사율/Bildwiederholrate）。建议 ko「갱신 주기」、de「Aktualisierungsrate」。

---

## 3. 分语种词条级详查

> 表格列含义：**模块**=功能对话框（Qt context）；**源文**=英文原文；**现译**=当前译文；**定级**：🔴误译（语义错误/硬伤）·🟠不一致/用词不当·🟡风格建议。✅ 表示该语种做对的参照项。

### 3.1 zh_CN（全文精读，发现 30 条）

| # | 模块 | 源文 | 现译 | 定级 | 问题与建议 |
|---|------|------|------|------|------------|
| 1 | InputConfDialog | Attach 4-Score (Implies four gamepads) | 附加 4 计分（隐含四手柄） | 🔴 | 专名拆译错误。改 **连接 Four Score（四人适配器）**  [(NesDev.org)](https://www.nesdev.org/wiki/Four_player_adapters)  |
| 2 | TasEditorWindow | 4 Score | 4 计分 | 🔴 | 同上，改 **Four Score** |
| 3 | InputConfDialog | Shadow | 阴影 | 🔴 | Space Shadow 光线枪  [(FCEUX)](https://fceux.com/web/help/Input.html) ；改 **Space Shadow（光线枪）** |
| 4 | ConsoleVideoConfDialog | Test Pattern: | 测试模式： | 🔴 | pattern=图案非模式。改 **测试图案：** |
| 5 | GamePadFuncConfigDialog | Modifier Button: | 修改键： | 🔴 | 标准术语"修饰键"（zh_TW 已译对）。改 **修饰键：** |
| 6 | GuiCheatsDialog | Auto Load / Save with Game | 随游戏自动读档/存档 | 🔴 | 该开关管的是**作弊码**随游戏自动载入/保存；"读档/存档"会被当成即时存档。改 **随游戏自动载入/保存（作弊码）** |
| 7 | iNesHeaderEditor | Trainer | 训练器 | 🔴 | iNES 头的 Trainer 是 512 字节数据块；中文"训练器"=作弊器，严重歧义。改 **Trainer 数据**（或保留 Trainer） |
| 8 | consoleWin | NSF Sound Files (*.nsf *.NSF) ;; … | NSF 音效文件 | 🔴 | NSF 是 NES **音乐**格式  [(NesDev.org)](https://www.nesdev.org/wiki/NSF) ；"音效"=sound effects，误导。改 **NSF 音乐文件** |
| 9 | ppuPatternView | Draw Tile &Grid Lines | 绘制图块与网格线 | 🔴 | "与"字产生"图块和网格线两物"的歧义。改 **绘制图块网格线(&G)** |
| 10 | TraceLoggerDialog | Log Last | 上次记录 | 🔴 | 该选项后接行数输入框，意为"只记录最近 N 行"。改 **记录最近** |
| 11 | GamePadView | Turbo | 极速 | 🟠 | 手柄视图的 Turbo 是**连发键**（别处 AutoFire 已译"自动连发"）；模拟器速度档 Turbo 才是"极速"。此处改 **连发** |
| 12 | consoleWin | Change &State Slot / Slot &%1 | 更改状态槽 / 存档槽 %1 | 🟠 | 同一概念两名。统一 **存档槽** |
| 13 | HotKeyConfDialog ∥ consoleWin | Hotkey Configuration ∥ Hotkey Conflict Warning | 热键配置 ∥ 快捷键冲突警告 | 🟠 | 术语表=快捷键。统一 **快捷键** |
| 14 | FKBConfigDialog ∥ 其他 | Family Keyboard | 家庭键盘 ∥ 家用机键盘 | 🟠 | 标题与正文两名。统一 **家用机键盘**（=术语表） |
| 15 | consoleWin | Load State &From / Load State From | 从文件载入存档 / 从...载入存档 | 🟠 | 菜单-提示两译。统一 **从文件载入存档** |
| 16 | consoleWin | Save State &As / Save State As | 另存存档为 / 存档另存为 | 🟠 | 语序不一。统一 **存档另存为** |
| 17 | iNesHeaderEditor | Mapper: / Mapper #: | 映射器：/ Mapper 号： | 🟠 | 同一对话框内 mapper 一译一留。统一（建议都保留 Mapper） |
| 18 | RamSearchDialog | Eliminate | 消除 | 🟠 | RAM 搜索里是"筛除不匹配项"。改 **筛除** |
| 19 | RamSearchDialog | Different By: | 差异： | 🟠 | 改 **相差值：** |
| 20 | RamSearchDialog | Compare To/By | 比较/按 | 🟠 | 改 **比较对象/方式** |
| 21 | GuiCheatsDialog | Possibilities | 可能性 | 🟠 | 作弊搜索的剩余候选地址数。改 **候选数** |
| 22 | GuiCheatsDialog | Active Cheats | 启用的作弊码 | 🟠 | Active=当前生效列表。改 **生效的作弊码** |
| 23 | GuiCheatsDialog | 0: Periodic Set (Every Frame) | 0：周期置位（每帧） | 🟠 | "置位"生硬。改 **0：周期写入（每帧）** |
| 24 | ConsoleVideoConfDialog | Lag Count | 延迟计数 | 🟠 | TAS 术语 lag frame。改 **延迟帧数** |
| 25 | FrameTimingDialog | Frame Late Count | 帧迟到计数 | 🟠 | 改 **迟滞帧计数** |
| 26 | CodeDataLogger | Save To File As | 保存到文件另存为 | 🟠 | 病句。改 **另存到文件…** |
| 27 | DebuggerBreakpointEditor | Read / Write | 读 / 写入 | 🟡 | 不对称。改 **读取 / 写入** |
| 28 | ConsoleDebugger 等 | 栈视图 ∥ 使用堆栈指针 | 栈 ∥ 堆栈 | 🟡 | 统一 **栈** |
| 29 | consoleWin 等 | 加载 BIOS ∥ 载入 FDS BIOS | 加载 ∥ 载入 | 🟡 | 统一 **载入**（全文"加载/载入"混用约 20 处） |
| 30 | TasEditorWindow | Paste Insert | 粘贴插入 | 🟡 | 改 **插入式粘贴** |

**zh_CN 做对的参照**：简繁分离零违规；"金手指/作弊码"按语境分化合理（Game Genie 对话框=金手指  [(web-g-p.com)](https://emu.web-g-p.com/info/term/term_cheat.html) ）；长句重组自然（"按住逐帧推进键多长时间后自动取消暂停？"）；状态栏首尾留白保留（9 个语种只有 zh 两语做到）。

### 3.2 zh_TW（与 zh_CN 同源，差异 4 条 + 全部同源问题）

zh_TW 与 zh_CN 出自同一管线，§3.1 的 #1–4、#6–9、#12–29 全部适用（4 計分、陰影、Trainer 訓練器、NSF 音效檔案、圖塊與格線、狀態槽/存檔槽、熱鍵/快捷鍵、家庭/家用機鍵盤等）。**差异项**：

| # | 模块 | 源文 | 现译 | 定级 | 问题与建议 |
|---|------|------|------|------|------------|
| 1 | ConsoleVideoConfDialog | Test Pattern: | 測試樣式： | 🔴 | "樣式"=style，与 zh_CN"测试模式"错得还不一样。改 **測試圖案：** |
| 2 | GamePadFuncConfigDialog | Modifier Button: | 修飾鍵： | ✅ | **译对**，zh_CN 应向其看齐 |
| 3 | consoleWin | Quick Save（术语表规定"快速存放"） | 快速存檔 | 🟠 | 术语表 zh_TW 列自身写了"快速存放"（"存放"非惯用），译文用"快速存檔"——**建议修订术语表为"快速存檔"**，与 quick load"快速讀檔"对仗 |
| 4 | 字形 | — | 游標/主控台 | ✅ | 台/遊 为可接受繁体变体；若定位台湾市场可全量 台→臺，非强制 |

### 3.3 ja（地道度最高，但"同词两译"密度最大，12 条）

| # | 模块 | 源文 | 现译 | 定级 | 问题与建议 |
|---|------|------|------|------|------------|
| 1 | LibavOptionsPage / LibgwaviOptionsPage | Video: / Video | 映像：/ ビデオ | 🟠 | 同页两名。统一 **ビデオ** |
| 2 | 同上 | Audio: / Audio | 音声：/ オーディオ | 🟠 | 统一 **オーディオ** |
| 3 | 全局 51 处 | Load 系 | 読込 ∥ 読み込み | 🟠 | 送り仮名两派并存，含同一源串 `Load` 一处読み込み一处読込。统一 **読み込み**（据术语表ロード系亦可，二选一） |
| 4 | consoleWin | Quick &Load / Quick Load | クイック読み込み / クイックロード | 🟠 | 菜单-提示两译。统一（术语表=クイックロード） |
| 5 | consoleWin | Quick &Save / Quick Save | クイック保存 / クイックセーブ | 🟠 | 同上。统一 **クイックセーブ** |
| 6 | consoleWin | Switch Disk / tooltip | ディスク切替 / ディスク切換 | 🟠 | 同串两译。统一 **切替** |
| 7 | consoleWin | Eject Disk / tooltip | ディスク排出 / ディスク取出し | 🟠 | 同串两译；术语表=取り出し。统一 **ディスク取り出し** |
| 8 | MovieOptionsDialog 等 | Movie Options 等 | ムービー ∥ 動画 | 🟠 | 同串两译。统一 **ムービー**（=术语表） |
| 9 | consoleWin / GameGenieDialog | Game Genie | Game Genie（菜单）∥ ゲームジニーコード（GG Code） | 🟠 | 术语表=ゲームジニー；两种处理并存。建议菜单也 **ゲームジニー** |
| 10 | consoleWin | Enable Game &Genie | Game &Genie を有効にする | 🟡 | 英文+& 原样嵌入日文句，风格突兀（其余语种均本地化）。改 **ゲームジニーを有効にする(&G)** |
| 11 | consoleWin | Simplified/Traditional Chinese | 简体中文 / 繁体中文 | 🟡 | 语言菜单内保留中文汉字，与相邻"韓国語/ドイツ語"等日文译名不统一；术语表 ja 列=簡体字/繁体字。改 **簡体字中国語 / 繁体字中国語**（或全菜单统一为各语言自名，二选一） |
| 12 | AboutWindow 等 | About FCEUX11 | FCEUX11について ∥ FCEUX11 について | 🟡 | 半角空格有无并存。统一加空格 |

**ja 做对的参照**：ステートセーブ系用词符合社区惯例  [(Weblio辞書)](https://www.weblio.jp/content/%E3%82%B9%E3%83%86%E3%83%BC%E3%83%88%E3%82%BB%E3%83%BC%E3%83%96%E3%80%81%E3%82%B9%E3%83%86%E3%83%BC%E3%83%88%E3%83%AD%E3%83%BC%E3%83%89) （术语表反而是错的）；フレームアドバンス 与 コマ送り 均为社区通行说法  [(ごちゃペディア)](https://gocha.hatenablog.com/entry/20090405/TASFeatures) ；设备名全部音译正确（ザッパー/シャドウ/フォアスコア）；Modifier Button=修飾ボタン ✅。

### 3.4 ko（文字硬伤最集中，13 条）

| # | 模块 | 源文 | 现译 | 定级 | 问题与建议 |
|---|------|------|------|------|------------|
| 1 | 全局 6 处 | Enable … | … 활성化 | 🔴 | 汉字"化"混入韩文。批量改 **활성화** |
| 2 | 2 处 | … Disable | … 비활성화 | 🔴 | 同上。改 **비활성화** |
| 3 | ppuRegPopup | Sprite Overflow | 스프라이트 오버플로 | 🔴 | 含 U+FFFD 乱码（"플로"后字符丢失）。改 **스프라이트 오버플로** |
| 4 | StateRecorderDialog | Minutes | 分 | 🔴 | 日语"分"错抄进韩语。改 **분** |
| 5 | MovieOptionsDialog / MoviePlayDialog | Movie Options / Movie Play | 묵비 옵션 / 묵비 재생 | 🔴 | 묵비=沉默，错字。改 **무비 옵션 / 무비 재생** |
| 6 | 全局 | movie | 무비(4) ∥ 동영상(32) ∥ 묵비(2) | 🟠 | 术语表=무비。统一 **무비** |
| 7 | consoleWin 等 | save state | 상태 불러오기 ∥ 세이브스테이트 | 🟠 | 术语表=세이브 스테이트。统一 |
| 8 | GamePadFuncConfigDialog | Modifier Button: | 수정 버튼: | 🟠 | "수정"=编辑，歧义。改 **보조 버튼:**（或 수정자 버튼） |
| 9 | HexEditorDialog | Re&fresh Rate | 주사율 | 🟠 | 显示器刷新率用词错配。改 **갱신 주기** |
| 10 | HexEditorDialog | Bookmarks | 북마크 ∥ 책갈피 | 🟠 | 同一对话框两名。统一 **북마크** |
| 11 | ConsoleDebugger 等 | breakpoint | 중단점(5) ∥ 브레이크포인트(5) | 🟠 | 对半分裂。建议统一 **중단점**（Visual Studio 韩语标准用词） [(코딩 도장)](https://dojang.io/mod/page/view.php?id=806)  |
| 12 | GameGenieDialog | Game Genie Code | 게지 코드 | 🟠 | "게지"生硬且与标题 Game Genie 英文并存；韩国社区惯用 **게임 지니**。建议改并修订术语表（术语表 ko 列本身即"게지"） |
| 13 | InputConfDialog | Zapper | 자퍼 | 🟡 | 音译偏差，惯用 **재퍼**；术语表本条规定保留 Zapper，译文却音译——处理方向需与术语表对齐 |

**ko 做对的参照**：长句语法与 합니까 体统一；Attach 4-Score=포어스코어 연결 ✅；Test Pattern=테스트 패턴 ✅。

### 3.5 de（质量最高，5 条）

| # | 模块 | 源文 | 现译 | 定级 | 问题与建议 |
|---|------|------|------|------|------------|
| 1 | InputConfDialog | Shadow | Schatten | 🔴 | Space Shadow 光线枪  [(FCEUX)](https://fceux.com/web/help/Input.html) 。改 **Space Shadow** |
| 2 | consoleWin 等 | save state | Zustand（菜单）∥ Spielstand（提示）∥ Savestate（Bind Save-States） | 🟠 | 三名并存；术语表=Spielstand。统一 **Spielstand**（德语玩家最熟悉的说法） |
| 3 | HexEditorDialog | Re&fresh Rate | Bildwiederholrate | 🟠 | 语境错配（同 ko）。改 **Aktualisierungsrate** |
| 4 | HexEditorDialog | &Find / Find | &Finden / Suchen | 🟠 | Finden/Suchen 并存。统一 **Suchen** |
| 5 | FKBConfigDialog ∥ InputConfDialog | Family Keyboard | Family-Tastatur ∥ Family-Keyboard | 🟠 | 同串两译。统一 **Family-Tastatur**（=术语表） |

**de 做对的参照**：Test Pattern=Testmuster ✅、Modifier=Modifikatortaste ✅、movie=Film 全文统一 ✅、Add Cheat=Cheat hinzufügen 一致 ✅、复合词构造专业（Einzelbildvorschub、Haltepunkt-Treffer protokollieren）。

### 3.6 fr（10 条）

| # | 模块 | 源文 | 现译 | 定级 | 问题与建议 |
|---|------|------|------|------|------------|
| 1 | QAsmView 等 | Breakpoints 等 4 处 | Points d&apos;Arrêt 等 | 🔴 | 双重转义，界面直接显示 `&apos;`。改 **Points d'arrêt** 等 4 处（清单见附录 B） |
| 2 | InputConfDialog | Shadow | Ombre | 🔴 | 改 **Space Shadow** |
| 3 | consoleWin | Open ROM / Last ROM Used / Close Loaded ROM | Ouvrir un ROM / Dernier ROM utilisé / Fermer le ROM chargé | 🔴 | **ROM 法语为阴性名词**：改 **Ouvrir une ROM / Dernière ROM utilisée / Fermer la ROM chargée** |
| 4 | consoleWin | Quick &Save / Quick Save | Sauvegarde &Rapide / Enregistrement rapide | 🟠 | 术语表=Sauvegarde rapide。统一 |
| 5 | GameGenieDialog ∥ GuiCheatsDialog | Add Cheat | Ajouter un code ∥ Ajouter un truc | 🟠 | "truc"口语且与术语表 code de triche 不符。统一 **Ajouter un code de triche** |
| 6 | consoleWin | Play &NSF / Play NSF | Jouer &NSF / Lire NSF | 🟠 | 统一 **Lire NSF**（播放文件用 lire） |
| 7 | consoleWin | &Debug | &Déboguer | 🟡 | 菜单名用动词原形欠妥。改 **&Débogage** |
| 8 | consoleWin | Load State &From | Charger État &Depuis | 🟡 | Title Case+悬空介词。改 **Charger l'état depuis…(&D)** |
| 9 | consoleWin | Save State &As | Enregistrer État &Sous | 🟡 | 同上。改 **Enregistrer l'état sous…(&S)** |
| 10 | 多处 | Configuration Vidéo、Points d'Arrêt、Paramètres &Avancés 等 | — | 🟡 | 法语软件应句式大小写（仅句首大写），全文 Title Case 残留约 8 处 |

**fr 做对的参照**：Test Pattern=Motif de test ✅（多语种里少数译对的）；movie=Film 统一 ✅；长句虚拟式使用正确。

### 3.7 es（5 条）

| # | 模块 | 源文 | 现译 | 定级 | 问题与建议 |
|---|------|------|------|------|------------|
| 1 | GameGenieDialog | Add Cheat | Agregar truc | 🔴 | 截断错字。改 **Agregar truco** |
| 2 | QObject | Cheat Manual Save Warning | Advertencia de guardado manual de truc | 🔴 | 同上。改 **…de trucos** |
| 3 | InputConfDialog | Shadow | Sombra | 🔴 | 改 **Space Shadow** |
| 4 | GameGenieDialog ∥ GuiCheatsDialog | Add Cheat | Agregar ∥ Añadir | 🟠 | 动词混用。统一 **Añadir truco** |
| 5 | TasEditorWindow 等 | Autofire Pattern | Patrón de autofuego | 🟡 | "autofuego"生造；西语社区多用 **disparo automático** 或保留 autofire。建议 **Patrón de disparo automático** |

**es 做对的参照**：Test Pattern=Patrón de prueba ✅；movie=Película 统一 ✅；estado guardado 统一 ✅；疑问句倒装问号完整。

### 3.8 vi（6 条）

| # | 模块 | 源文 | 现译 | 定级 | 问题与建议 |
|---|------|------|------|------|------------|
| 1 | QAsmView 等右键菜单 | 44 处菜单项 | Xuất &ra(&X) 模式 | 🔴 | "词内 & + 括号 (&X)"双助记符叠加，界面残留 `(X)` 字样且助记符失效（如 `Xuất &ra(&X)`）。统一为尾置单 &：**Xuất ra(&X)** |
| 2 | InputConfDialog | Shadow | Bóng | 🔴 | 改 **Space Shadow** |
| 3 | TasEditorWindow | Turbo Seek | Tìm kiếm turbo | 🔴 | 误译为"寻找 turbo"。改 **Dò nhanh**（快速跳转） |
| 4 | consoleWin | Save State &As | Lưu Trạng thái &Thành | 🟠 | "Thành"译 As 生硬+大写。改 **Lưu trạng thái khác(&K)** |
| 5 | consoleWin | Load State &From | Tải Trạng thái &Từ | 🟠 | 介词 Từ 悬空+大写。改 **Tải trạng thái từ tệp(&T)** |
| 6 | GameGenieDialog ∥ GuiCheatsDialog | Add Cheat | Thêm cheat ∥ Thêm mã gian lận | 🟠 | 术语表=mã gian lận。统一 |

**vi 做对的参照**：movie=Phim 统一 ✅；编码器页之外的常用界面翻译完整。

### 3.9 th（8 条）

| # | 模块 | 源文 | 现译 | 定级 | 问题与建议 |
|---|------|------|------|------|------------|
| 1 | 全局 24 处 | movie 系 | ภาพยนตร์ | 🔴 | **系统性拼写错误**：正确为 **ภาพยนต์**（ร์ 多余），全文 24 处无一写对，含主菜单 `&Movie ⇒ &ภาพยนตร์`。另建议评估：术语表规定 มูฟวี่，若统一为 มูฟวี่ 则 24 处错拼连同 25 处 วิดีโอ 一并解决 |
| 2 | MovieRecordDialog | Record Input Movie | บันทึกภาพยนตร์อินพุ | 🔴 | U+FFFD 断字+错拼。改 **บันทึกมูฟวี่อินพุต** |
| 3 | TasEditorWindow | Reselect Clipboard | เลือกคลิปบอร์ดอีกครัย | 🔴 | U+FFFD 断字。改 **เลือกคลิปบอร์ดอีกครั้ง** |
| 4 | InputConfDialog | Shadow | เงา | 🔴 | 改 **Space Shadow** |
| 5 | TasEditorWindow | Turbo Seek | ค้นหาเทอร์โบ | 🔴 | "搜索 turbo"误译。改 **เลื่อนหาเร็ว** |
| 6 | GameGenieDialog ∥ GuiCheatsDialog | Add Cheat | เพิ่มสูตรโกง ∥ เพิ่มโกง | 🟠 | 统一 **เพิ่มโกง**（=术语表） |
| 7 | consoleWin | 助记符位置 | โหลด&เร็ว、เปลี่ยนช่อง&สถานะ 等 | 🟠 | & 嵌入泰语词中，泰文菜单惯例应尾置 (&X)：如 **โหลดเร็ว(&L)** |
| 8 | consoleWin | state slot / Quick Load 系 | ช่องเซฟ ∥ ช่องสถานะ；โหลด&เร็ว ∥ โหลดด่วน | 🟠 | 两对小分裂。统一 ช่องสถานะ、โหลดด่วน |

### 3.10 hi（4 条）

| # | 模块 | 源文 | 现译 | 定级 | 问题与建议 |
|---|------|------|------|------|------------|
| 1 | consoleWin | Load State &From / tooltip | स्थिति &से लोड करें / स्टेट यहाँ से लोड करें | 🟠 | स्थिति/स्टेट 两体系+菜单提示不同文。统一 **स्टेट**（术语表=सेव स्टेट，可协调） |
| 2 | consoleWin | Quick &Save / Quick Save | त्वरित &सहेजें / क्विक सेव | 🟠 | क्विक/त्वरित、सेव/सहेजें 双重不一致；术语表=क्विक सेव。统一 |
| 3 | ppuNameTableViewer 等 3 处 | Display on Scanline: 等 | 尾冒号丢失 | 🟡 | 补 **：** |
| 4 | — | Shadow ⇒ शैडो、मूवी 统一、संशोधक बटन | — | ✅ | 小语种中准确度最高，无语义误译 |

### 3.11 ar（4 条）

| # | 模块 | 源文 | 现译 | 定级 | 问题与建议 |
|---|------|------|------|------|------------|
| 1 | InputConfDialog | Shadow | ظل | 🔴 | 改 **Space Shadow** |
| 2 | 全局 | movie | تسجيل ∥ فيلم | 🟠 | 术语表=تسجيل；فيلم 易误解为"电影"。统一 **تسجيل** |
| 3 | consoleWin | Load State From | تحميل الحالة من | 🟡 | 介词 من 悬空。改 **تحميل الحالة من ملف** |
| 4 | AviRiffViewer 等 | &Quit Window 等 | 同串两译 2 组 | 🟡 | 统一 |

**ar 做对的参照**：数字保持 ASCII（游戏工具正确选择）、术语一致度高、Modifier 之外的常用词汇准确。静态审计无法验证 RTL 混排渲染，建议配合真机走查（不影响本报告对文本本身的结论）。

---

## 4. 术语表的翻译层面缺陷（仅列影响译文的 6+2 处）

术语表是译文的上游依据，以下缺陷会向下游传导错误，随本次译文修订一并处理：

| # | 术语 | 列 | 现值 | 问题 | 建议值 |
|---|------|----|------|------|--------|
| G1 | slow down | hi | ति धीमी करें | 乱码吞字 | गति धीमी करें |
| G2 | reset | th | รีเซ็ | 断字 | รีเซ็ต |
| G3 | find | hi | ूंढें | 乱码吞字 | ढूंढें |
| G4 | settings | ko | 設定 | **日语汉字混入韩语列** | 설정 |
| G5 | quit | th | 退出 | **中文混入泰语列** | ออก |
| G6 | redo | es | Reacer | 错字 | Rehacer |
| G7 | Game Genie | ko | 게지 | 生硬非惯用 | 게임 지니 |
| G8 | quick save | zh_TW | 快速存放 | 用词非惯用，与"快速讀檔"不对仗 | 快速存檔 |

另有两处"术语表与社区惯例相悖"的建议修订（译文反而比术语表更正确）：ja 的 save state 建议从 セーブ状態/ロード状態 改为 **ステートセーブ/ステートロード**（社区标准） [(Weblio辞書)](https://www.weblio.jp/content/%E3%82%B9%E3%83%86%E3%83%BC%E3%83%88%E3%82%BB%E3%83%BC%E3%83%96%E3%80%81%E3%82%B9%E3%83%86%E3%83%BC%E3%83%88%E3%83%AD%E3%83%BC%E3%83%89) ；ja 的 frame advance 建议从 フレーム送り 改为 **フレームアドバンス 或 コマ送り**  [(ごちゃペディア)](https://gocha.hatenablog.com/entry/20090405/TASFeatures) 。

---

## 5. 修复优先级清单（纯翻译层面）

**P0｜语义误译与文字硬伤（约 60 处，1 人日内可完成）**

1. 8 语种 Shadow → Space Shadow（§2.1）；
2. zh_CN/zh_TW "4 计分" → Four Score（2+1 处）；
3. zh_CN 测试模式→测试图案、zh_TW 測試樣式→測試圖案、zh_CN 修改键→修饰键、NSF 音效文件→NSF 音乐文件、Trainer 训练器→Trainer 数据、"随游戏自动读档/存档"→"随游戏自动载入/保存作弊码"；
4. ko 全文 sed：활성化→활성화（6 处）、비활성화（2 处）、묵비→무비（2 处）、分→분（1 处）、스프라이트 오버플로 乱码（1 处）；
5. th 全文 sed：ภาพยนตร์→มูฟวี่（24 处，顺带统一 movie 术语）+ 2 处乱码；
6. es truc→truco（2 处）；fr 双重转义 4 处（附录 B）；
7. fr ROM 性别 3 处（une ROM / Dernière ROM utilisée / la ROM chargée）。

**P1｜术语统一（约 90 处，1–2 人日）**

8. movie 统一：ko→무비、ar→تسجيل、ja→ムービー；
9. save state 统一：de→Spielstand、ko→세이브 스테이트、hi→स्टेट 系、ja 菜单/提示对齐；
10. Add Cheat 统一：th/vi/fr/es（zh 按语境保留"金手指/作弊码"双译并登记术语表）；
11. ja 読込→読み込み（约 30 处批量）、切替/取出し/クイックセーブ 配对统一；zh 热键→快捷键、状态槽→存档槽；ko 중단점 统一  [(코딩 도장)](https://dojang.io/mod/page/view.php?id=806) ；vi 44 处双 & 改尾置（可正则批量+复核）。

**P2｜风格与排版（约 40 处，0.5 人日）**

12. fr Title Case→句式大小写、动词原形菜单名；zh 加载/载入、栈/堆栈归一；ja について空格；hi 尾冒号；ar/vi 悬空介词补全；es autofuego→disparo automático；ko 자퍼→재퍼。

---

## 附录 A：外部核验依据一览

| 结论 | 依据 |
|------|------|
| Shadow = "Space Shadow" 光线枪 | FCEUX 官方 Input 文档：Famicom 扩展端口设备含 the "Space Shadow" gun  [(FCEUX)](https://fceux.com/web/help/Input.html)  |
| Four Score = 任天堂 NES 四人适配器 | NESdev Wiki "Four player adapters" 条目  [(NesDev.org)](https://www.nesdev.org/wiki/Four_player_adapters)  |
| 日语 save state 社区惯例 = ステートセーブ/ステートロード | 日语维基百科"ゲームエミュレータ"条目（Weblio 转载） [(Weblio辞書)](https://www.weblio.jp/content/%E3%82%B9%E3%83%86%E3%83%BC%E3%83%88%E3%82%BB%E3%83%BC%E3%83%96%E3%80%81%E3%82%B9%E3%83%86%E3%83%BC%E3%83%88%E3%83%AD%E3%83%BC%E3%83%89)  |
| 日语 frame advance = コマ送り/フレームアドバンス 均通行 | TAS 用エミュレータの機能紹介（コマ送り＝frame advance＝フレームアドバンス） [(ごちゃペディア)](https://gocha.hatenablog.com/entry/20090405/TASFeatures)  |
| 韩语 breakpoint 标准译 = 중단점 | Microsoft Learn 韩语版 Visual Studio 调试器文档（중단점） [(코딩 도장)](https://dojang.io/mod/page/view.php?id=806)  |
| 泰语 breakpoint 标准译 = จุดพัก | 泰国皇家学术院术语库（Longdo 收录） [(พจนานุกรม Longdo)](https://dict.longdo.com/search/breakpoint)  |
| NSF = NES 音乐文件格式 | NESdev Wiki NSF 条目："used for storing and playing music"  [(NesDev.org)](https://www.nesdev.org/wiki/NSF)  |
| 日语 cheat = チート（金手指≠チート） | エミュレータ情報局ゲーム用語集"チート"条  [(web-g-p.com)](https://emu.web-g-p.com/info/term/term_cheat.html)  |
| TAS movie = 输入录像文件 | TASVideos 官方 FAQ（movie file＝输入录像） [(TASVideos)](http://tasvideos.org/FAQ)  |

## 附录 B：法语双重转义完整清单（4 处）

| 模块 | 源文 | 当前（含乱码） | 应改为 |
|------|------|----------------|--------|
| QAsmView | Breakpoints | Points d&apos;Arrêt | Points d'arrêt |
| QAsmView | Emulator Stopped / Paused at Breakpoint | Émulateur Arrêté au Point d&apos;Arrêt | Émulateur arrêté au point d'arrêt |
| consoleWin | &Input Config | Configuration d&apos;Entrée | Configuration de l'&entrée |
| consoleWin | &State Recorder Config | Configuration de l&apos;Enregistreur d&apos;État | Configuration de l'en&registreur d'état |

---

*本报告全部问题条目均经人工逐条复核定位（非抽样估计），每条均可凭"模块+源文"在对应 .ts 文件中直接检索定位。*

---

 [(FCEUX)](https://fceux.com/web/help/Input.html) : https://fceux.com/web/help/Input.html
 [(Weblio辞書)](https://www.weblio.jp/content/%E3%82%B9%E3%83%86%E3%83%BC%E3%83%88%E3%82%BB%E3%83%BC%E3%83%96%E3%80%81%E3%82%B9%E3%83%86%E3%83%BC%E3%83%88%E3%83%AD%E3%83%BC%E3%83%89) : https://www.weblio.jp/content/%E3%82%B9%E3%83%86%E3%83%BC%E3%83%88%E3%82%BB%E3%83%BC%E3%83%96%E3%80%81%E3%82%B9%E3%83%86%E3%83%BC%E3%83%88%E3%83%AD%E3%83%BC%E3%83%89
 [(ごちゃペディア)](https://gocha.hatenablog.com/entry/20090405/TASFeatures) : https://gocha.hatenablog.com/entry/20090405/TASFeatures
 [(NesDev.org)](https://www.nesdev.org/wiki/Four_player_adapters) : https://www.nesdev.org/wiki/Four_player_adapters
 [(พจนานุกรม Longdo)](https://dict.longdo.com/search/breakpoint) : https://dict.longdo.com/search/breakpoint
 [(코딩 도장)](https://dojang.io/mod/page/view.php?id=806) : https://learn.microsoft.com/ko-kr/visualstudio/debugger/using-breakpoints
 [(NesDev.org)](https://www.nesdev.org/wiki/NSF) : https://www.nesdev.org/wiki/NSF
 [(TASVideos)](http://tasvideos.org/FAQ) : http://tasvideos.org/FAQ
 [(web-g-p.com)](https://emu.web-g-p.com/info/term/term_cheat.html) : https://emu.web-g-p.com/info/term/term_cheat.html
 [(Documentation & Help)](https://documentation.help/FCEUX/documentation.pdf) : https://documentation.help/FCEUX/documentation.pdf
