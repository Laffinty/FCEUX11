# FCEUX11 i18n 架构（v0.3.15 PR-B）

> **目标读者**：v0.3.15.x 后续维护者 / 翻译贡献者
> **决策时间**：2026-06-16
> **对应 plan**：`docs/v0.3.x_Construction_Plan_v3.md` §5 v0.3.15「简体中文汉化完善化任务」

## 一、为什么 v0.3.15 必须做 i18n

**实测现状（v0.3.14 收尾时）**：
- `src/drivers/Qt/` 全文 `tr()` 调用 **3,481 处**，跨 **59 个文件**
- `src/drivers/Qt/lang/` 目录**不存在**
- `assets/i18n/*.ts` 共 3 份（en / zh_CN / zh_TW），每份仅 **101 个 `<source>` 条目**，**全部手写**（不是 lupdate 产物）
- `assets/i18n/*.qm` 每个 20-23 字节，**只含 lrelease 头 + 0 条目**
- `loadTranslation()` 已实现（`ConsoleWindow.cpp:4625-4661`），从 `:/i18n/fceux11_<lang>.qm` 加载 QTranslator
- 三语单选菜单（English / Simplified Chinese / Traditional Chinese）已挂载（`ConsoleWindow.cpp:1192-1214`）
- **`changeEvent(QEvent::LanguageChange)` override 仅 2 个**（consoleWin_t + AboutWindow），**30+ 子对话框运行时切语言不响应**
- `assets/i18n/fceux11_zh_CN_test.qm` 存在但 `resources.qrc` 未注册，孤立文件

**用户痛点**：Options → Language → 简体中文 → UI 仍是英文。

## 二、v0.3.15 PR-B 决策链路

### D1. 旧 `assets/i18n/*.ts` 怎么办？

**决策**：**全部废弃**，迁到 `src/drivers/Qt/lang/`。

**理由**：
- 旧 101 条是手写菜单占位，与源码 3,481 处 `tr()` 完全脱节
- 一旦跑 `lupdate`，旧 101 条手写翻译**会被保留**（lupdate 按 source string 合并），但 .ts 文件结构会从 101 条暴涨到 3,481 条
- 「从原位不动」会让 qt_add_lupdate 现代 API 与手写 qrc 冲突

### D2. 旧手写翻译怎么办？

**决策**：**保留 101 条真实翻译**（在 PR-B 创建的 `lang/fceux11_zh_CN.ts` 与 `lang/fceux11_zh_TW.ts` 中作为 20 条核心菜单串的翻译），**剩余 3,360 条走 PR-C 机翻**。

**理由**：
- 旧翻译如「文件(&F) / 影片(&M) / 选项(&O)」是手写但准确的
- 机翻对这 20 条核心串的成本接近 0，但能立刻修复"切到中文，菜单切中"的最显眼用户感知
- 大量低频 `tr()`（错误信息、调试器细节）走机翻可接受

### D3. CMake 现代 API vs 手写 custom_command

**决策**：用 `qt_add_lupdate + qt_add_lrelease + qt_add_resources`（见 `src/CMakeLists.txt:580-606`）。

**理由**：
- `qt_add_lupdate` 自动把 .ts 的依赖 .cpp / .h 写入 CMake dependency graph，**改任何 .cpp 触发 lupdate 重跑**
- `qt_add_lrelease` 直接产出 QM_FILES_OUTPUT_VARIABLE，免去手写 `foreach(ts_file ${TS_FILES})`
- `qt_add_resources` 把 .qm 自动嵌入 `:/i18n/` 资源
- **副作用**：必须删 `resources.qrc` 的 `/i18n` prefix（与 qt_add_resources alias 冲突）

### D4. tr() 字串集合冻结时机

**决策**：**PR-B merge 后冻结**。任何新增 `tr()` 必须走 v0.3.15.x hotfix + 重跑 lupdate。

**理由**：
- `tr()` 是无类型签名的字符串，编译期无法检测遗漏
- 若不冻结，每个 PR 都会引入新 unfinished 条目 → CI 闸门（90% 覆盖率）会反复挂掉
- 冻结机制让 PR-C 集中精力翻译这 3,481 条，后续 PR 不再增条

### D5. 30+ 子 widget retranslateUi 怎么办？

**决策**：PR-B 阶段**只交付架构骨架**——给出 `consoleWin_t` / `AboutWindow` 的完整模式，剩余 widget 走 v0.3.15.x 分批 PR。

**理由**：
- 30+ widget 全部加 `changeEvent + retranslateUi` 需约 1-1.5 人天集中工作量
- 每个 widget 需逐个审查构造期 `new QPushButton(tr(...), this)` 改为成员指针，工作量大但机械
- 30+ widget 不影响**主菜单**翻译（PR-B 闸门），**仅影响运行时切语言后子窗口的刷新**
- 推迟到 v0.3.15.x 不阻塞 PR-B 闸门

### D6. 字体 fallback

**决策**：在 `QFont::setFamilies()` 加 `Microsoft YaHei UI / Microsoft YaHei / Noto Sans CJK SC` 三级 fallback。**不引入 `QFontDatabase::addApplicationFont`**（避免字体文件分发负担）。

**理由**：
- Win11 默认带 Microsoft YaHei UI，覆盖 99% CJK 字符
- `Segoe UI Variable` 基本 CJK 集（~7000 简体字）已能渲染，生僻字 fallback 到 YaHei
- Noto Sans CJK SC 是 Linux/Mac 兼容方案（多 OS 时）

### D7. 4 种 IME 兼容性

**决策**：在 6 处 `keyPress override` 头部加 `BaseClass::keyPressEvent(event)` 转发（仅 ConsoleWindow / ConsoleViewerGL / GamePadConf 3 处，**HexEditor 因自定义游标逻辑风险推迟到 v0.3.15.x**）。

**理由**：
- v0.3.14 BUG B 修复路径（`SDL_PumpEvents` 0ms timer + `pushKeyEvent` + `event->accept()`）保留不动
- 关键是先 `BaseClass::keyPressEvent(event)` 让 `QInputMethodEvent` 路由到焦点 QLineEdit，再 `event->accept()`
- HexEditor 自定义游标逻辑风险高（一旦转发可能破坏现有 hex 编辑器行为），推迟

### D8. 覆盖率闸门

**决策**：PR-C 闸门由原 plan 的 **≥ 95% 调整为 ≥ 90%**（用户确认）。

**理由**：
- 90% 闸门对机翻+人工复审已足够（剩余 10% 留给低频错误信息等可接受英文回退的串）
- 母语审校走 v0.3.15.x hotfix，不阻塞 v0.3.15 主版本发布

## 三、PR-B 落地产物清单

### 新增文件
- `src/drivers/Qt/lang/translations.pro` — Qt Linguist 工程
- `src/drivers/Qt/lang/glossary.txt` — 80 项术语对照
- `src/drivers/Qt/lang/README.md` — 翻译工作流说明
- `src/drivers/Qt/lang/fceux11_en.ts` — English 基线（20 条已翻译）
- `src/drivers/Qt/lang/fceux11_zh_CN.ts` — 简中（20 条已翻译 + PR-C 目标 ≥ 90%）
- `src/drivers/Qt/lang/fceux11_zh_TW.ts` — 繁中（20 条已翻译 + PR-C 目标 ≥ 90%）
- `scripts/i18n_update.ps1` — lupdate 同步脚本
- `scripts/i18n_release.ps1` — lrelease 编译脚本
- `scripts/i18n_coverage.ps1` — 覆盖率闸门（≥ 90%）
- `scripts/check_simp_trad.ps1` — 繁简字串隔离校验

### 修改文件
- `src/CMakeLists.txt:580-617` — 改用 `qt_add_lupdate/lrelease/resources` 现代 API
- `resources.qrc` — 删除 `/i18n` prefix 段（与 qt_add_resources alias 冲突）
- `src/drivers/Qt/main.cpp:136-138` — 字体 fallback 链
- `src/drivers/Qt/ConsoleWindow.cpp:684-690, 692-698` — keyPress/keyRelease 先调父类
- `src/drivers/Qt/ConsoleViewerGL.cpp:514-524` — keyPress/keyRelease 先调父类
- `src/drivers/Qt/GamePadConf.cpp:607` — keyPress 先调父类

## 四、PR-B 闸门（必须通过）

1. **闸 1 — 编译**：`cmake --build` 0 错误
2. **闸 2 — 切换语言**：启动 → Options → Language → 简体中文 → **主菜单切中**（其他 widget 仍是英文，符合预期）
3. **闸 3 — 字串冻结**：`tr()` 集合在 PR-B merge 后冻结；新增必须 hotfix
4. **闸 4 — 资源**：`resources.qrc` 不再含 `/i18n` 段，qm 通过 qt_add_resources 嵌入
5. **闸 5 — 字体**：启动 → 含繁体字（如「邊 / 備 / 綠」）的窗口不出现「豆腐块」(□)

## 五、PR-C 闸门（待 PR-B merge 后启动）

1. **闸 1 — 覆盖率**：`lang/fceux11_zh_CN.ts` 已翻译 ≥ 3,133 条（90% × 3,481）；`zh_TW.ts` 同上
2. **闸 2 — 完整性**：所有 `<message>` 均有 `<translation>`（无遗漏占位）
3. **闸 3 — 繁简隔离**：`zh_CN.ts` 不含繁体字 / `zh_TW.ts` 不含简体字（CI 校验）
4. **闸 4 — 运行时**：启动 → zh_CN → 主菜单 + 已翻译子对话框切中
5. **闸 5 — IME**：4 种主流 IME 输入合成态正常（手动测试）

## 六、已知的 PR-B 局限（不影响 v0.3.15 主版本）

| 局限 | 影响 | 解决方案 |
|------|------|---------|
| 30+ widget 不响应 LanguageChange | 切语言后子窗口不刷新 | v0.3.15.x 分批 PR |
| HexEditor keyPress 未修复 | IME 在 hex 搜索框失效 | v0.3.15.x（自定义游标逻辑需谨慎） |
| `fceuWrapper.cpp:36` 跨 Qt/C 边界 tr() | 潜在 libfceux 静态库污染 | 待审计，v0.3.15.x |
| TypedConfig<T> 包装类未落地 | 74 处 QSettings 散落 | v0.4.x 议程 |
| `--no-console` 未实现 | 见 PR-D | PR-D 落 |

## 七、参考链接

- [Qt Linguist Manual](https://doc.qt.io/qt-6/linguist-programmers.html)
- [Qt cmake qt_add_lupdate](https://doc.qt.io/qt-6/qtlinguist-cmake-qt-add-lupdate.html)
- [Qt cmake qt_add_lrelease](https://doc.qt.io/qt-6/qtlinguist-cmake-qt-add-lrelease.html)
- [Qt cmake qt_add_resources](https://doc.qt.io/qt-6/qtcore-cmake-qt-add-resources.html)
- [Microsoft YaHei UI 字体说明](https://learn.microsoft.com/en-us/windows/apps/design/downloads/#fonts)
