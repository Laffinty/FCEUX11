# FCEUX11 迁移至微软系 UI 框架可行性分析报告

> **报告性质**：v2.x 战略预研（必须在 `docs/v1.x_Modernization_Roadmap.md` 全部完成后方可启动）  
> **分析对象**：FCEUX11 当前 Qt 6 驱动层 → 微软全系 UI 框架（WinUI 3 → Win32）  
> **编制日期**：2026-06-21  
> **版本**：v1.0

---

## 1. 摘要

FCEUX11 目前以 **Qt 6.8 LTS** 作为唯一现役 GUI 驱动层（约 10.4 万行、159 个文件，全部手写代码构建 UI）。Qt 深度融入了模拟器核心生命周期的每一个角落：主窗口、调试器、TAS 编辑器、PPU/名称表查看器、Hex 编辑器、输入配置、AVI 录制、OpenGL 渲染视图、信号/槽事件网等。

本报告对从最新到最旧的微软系 UI 框架进行逐一可行性评估：

| 框架 | 结论一句话 |
|------|-----------|
| **WinUI 3** | 战略方向最贴合，但 C++/WinRT 生态与工具链成熟度仍存风险；适合作为 v2.x 首选目标，但不应在 v1.x 期间切换。 |
| **WinUI 2 / UWP** | 已进入只修 bug 不增功能的维护期，且 UWP 应用模型与模拟器需求冲突，**不建议**。 |
| **WPF** | 生态成熟、工具链稳定，但基于 .NET/C#，与现有 C++ 核心 + Rust 的代码库语言异构；可作为次选或混合方案。 |
| **WinForms** | 开发快、兼容广，但 GDI+ 架构老旧、高 DPI 与现代视觉表现差；仅适合维护模式，不适合旗舰模拟器。 |
| **Win32 原生** | 兼容性最好、性能最高、体积最小，但开发效率最低；可作为完全放弃 Qt 后的“保底”或“内核 + 轻量 UI”方案。 |

**核心建议**：在 v1.14 收官、v2.0 启动之前，将 Qt 驱动层彻底解耦（v1.11 Bridge 已完成设计），形成稳定的 **Core-Driver 接口边界**。v2.x 的首要 UI 迁移目标可定为 **WinUI 3（C++/WinRT）**，同时保留 **Win32 原生** 作为降级/备选路径；WPF 可作为需要快速验证 UI 时的过渡方案。

---

## 2. 前置约束：v1.x 现代化必须先完成

根据 `docs/v1.x_Modernization_Roadmap.md`，v1.x 的核心目标是：

1. **核心引擎去全局化**：`fceu11::Cpu` / `Ppu` / `Apu` / `Bus` / `Cart` 对象化（v1.3~v1.8）。
2. **Rust 迁移闭环**：ROM 解析、Savestate、工具、Lua 等迁移到 Rust（v1.9~v1.10）。
3. **Qt 驱动层解耦**：v1.11 Bridge 明确提出用 `fceu11::DriverCallbacks` 替代核心中的 `#ifdef __QT_DRIVER__`，使核心不再 include 任何驱动头文件。
4. **巨型文件拆分与清理**：v1.12~v1.13 将 `TasEditorWindow.cpp`（6747行）、`ConsoleWindow.cpp`（3999行）等拆分，便于 UI 框架迁移时按子模块重写。

### 2.1 为什么必须等 v1.x 完成后才迁移 UI 框架

- **双向依赖尚未解除**：核心代码中仍存在 `#ifdef __QT_DRIVER__`，`fceuWrapper.cpp` 同时持有核心全局变量定义与 Qt 事件循环。若此时切换 UI 框架，会同时触发“核心重构 + UI 重写”，风险不可控。
- **Savestate 二进制格式锁定**：v1.x 承诺 savestate 在 v1.x 全系列保持互操作。UI 框架切换不应与 savestate 格式变更重叠。
- **性能基线需要稳定**：v1.14 Anvil 要求所有性能基准与 v1.0 偏差 ≤ 2%。UI 框架迁移会引入渲染管线变化，必须在已有硬化基线上评估。
- **团队认知成本**：v1.11~v1.12 拆分后的子模块（如 `TasEditorTimeline`、`ConsoleMenu`）是 UI 迁移的最小单元；提前迁移会导致拆分校准困难。

> **硬性原则**：本报告所讨论的迁移行动，仅应在 `v1.14` 全部验收标准通过、且 `v2.0` 里程碑正式启动后进入工程实施阶段。

---

## 3. 当前 Qt 驱动层现状量化

### 3.1 规模

| 指标 | 数值 |
|------|------|
| `.cpp` 文件数 | 77 |
| `.h` 文件数 | 82 |
| `.ui` 文件数 | 0（全部手写代码构建 UI） |
| **合计文件数** | **159** |
| **代码总行数** | **约 104,169 行** |

### 3.2 主要窗口/模块行数

| 模块 | 文件 | 约行数（cpp+h） |
|------|------|----------------|
| `ConsoleDebugger` | `ConsoleDebugger.cpp/h` | ~9,057 |
| `TasEditorWindow` + 子模块 | `TasEditor/*` | ~20,971 |
| `consoleWin_t`（主窗口） | `ConsoleWindow.cpp/h` | ~5,432 |
| `AviRecord` | `AviRecord.cpp/h` | ~4,805 |
| `HexEditor` | `HexEditor.cpp/h` | ~4,697 |
| `ppuViewer` | `ppuViewer.cpp/h` | ~4,489 |
| `TraceLogger` | `TraceLogger.cpp/h` | ~3,008 |
| `GamePadConf` | `GamePadConf.cpp/h` | ~2,926 |
| `NameTableViewer` | `NameTableViewer.cpp/h` | ~2,334 |
| `RamSearch` | `RamSearch.cpp/h` | ~2,294 |
| 其他小型对话框 | — | 各 300~700 行 |

### 3.3 Qt 特性使用强度

| Qt 特性 | 强度 | 对迁移的影响 |
|---------|------|-------------|
| **信号/槽（Q_OBJECT / connect）** | 很重（109 个 Q_OBJECT，1049 处 connect） | 必须整体替换为各框架的事件/命令/绑定机制。 |
| **QPainter 自定义绘制** | 中等偏重（66 处，24 个文件） | 调试器反汇编视图、PPU 查看器、Hex 编辑器、TAS PianoRoll 等需重写。 |
| **QOpenGL** | 集中（`ConsoleViewerGL_t`） | 可保留现有 OpenGL shader 逻辑，只需替换窗口上下文创建。 |
| **QThread** | 中等（3 个派生工作线程） | 可迁移为 `std::thread` + 框架 Dispatcher。 |
| **QMenu/QMenuBar/QToolBar** | 重（161 处菜单） | 各框架均有对应，但行为细节（上下文菜单、快捷键）需重新适配。 |
| **QSplitter/QTabWidget** | 少量 | 各框架均有对应。 |
| **Qt Network / Qt Multimedia** | 无 | 网络使用原生 socket，降低迁移复杂度。 |
| **QML** | 无 | 无需处理 QML 运行时。 |
| **Qt Linguist i18n** | 有（3 个 `.ts` 文件） | 需迁移到新的本地化方案。 |

### 3.4 CMake 配置

- `src/CMakeLists.txt` 已明确 **Qt6 only**（Qt5 支持已在 Phase 2 移除）。
- 链接目标：`Qt6::Widgets`、`Qt6::OpenGL`、`Qt6::OpenGLWidgets`，可选 `Qt6::Help`。
- 预编译头包含 `<QtWidgets>`、`<QtOpenGL>`。
- 宏定义：`__QT_DRIVER__` 在核心与驱动之间仍有条件编译残留。

---

## 4. 微软 UI 框架逐一分析

### 4.1 WinUI 3（Windows App SDK）—— 最新、官方主推

#### 4.1.1 现状（截至 2026-06）

- **版本**：Windows App SDK 稳定版 **1.8**（2025-09 发布），**2.0 Preview** 已面向 .NET 10。
- **定位**：微软官方当前推荐的“最新原生 UI 框架”，将 UWP XAML 视觉层从 UWP 应用模型中解耦，运行在 Win32 桌面进程上。
- **C++ 支持**：通过 **C++/WinRT** 编写 WinUI 3 应用；XAML 编译为 C++ 代码（`XamlCompiler`）。
- **Windows 版本要求**：Windows 10 1809（Build 17763）及以上；Windows 11 体验最佳。

#### 4.1.2 可行性

| 维度 | 评估 |
|------|------|
| **可行性** | 高，但非零摩擦。WinUI 3 是微软当前对桌面应用的主推方向，适合全新 Windows 原生 UI。 |
| **性能** | 合成器（Compositor）渲染，比 WPF 更流畅；启动快于 WPF；C++/WinRT 原生路径可避免托管 GC 开销。 |
| **兼容性** | 仅限 Windows 10 1809+；不再支持 Windows 7/8。对 FCEUX11 这类偏技术用户的模拟器，需评估用户群旧系统比例。 |
| **美观程度** | 最好。原生 Fluent Design、Mica/Acrylic、圆角、Snap Layouts、深色模式等一等公民支持。 |
| **改造难度** | 极高。约 10 万行 Qt 代码需重写为 XAML + C++/WinRT；信号槽→`{x:Bind}`/`ICommand`/`winrt::event`；QPainter→自定义控件/Win2D/SwapChainPanel。 |

#### 4.1.3 具体优势

1. **原生 Windows 11 体验**：Mica/Acrylic 背景、圆角窗口、自定义标题栏、Snap Layout 菜单，无需像 Qt 那样自己实现 DWM 桥接。
2. **C++ 原生路径**：与 FCEUX11 的 C++20 + Rust 核心同构，避免引入 .NET 运行时。
3. **Win32 API 互操作**：WinUI 3 应用本质是 Win32 进程，可直接调用现有 Win32 代码、SDL、DirectStorage 探测等。
4. **Native AOT 潜力**：Windows App SDK 2.0 在 AOT 上持续投入，未来可减小分发体积、提升启动速度。
5. **官方长期投入**：Build 2026 再次强调 WinUI 3 / Windows App SDK 是原生 Windows 应用的核心方向。

#### 4.1.4 具体风险与限制

1. **C++/WinRT 工具链成熟度不足**：
   - 没有可视化 XAML Designer（截至 WASDK 2.0，Visual Studio 的 XAML Designer 仍不支持 WinUI 3 C++ 项目）。
   - C++/WinRT 被部分社区声音认为处于“维护模式”，新功能有限，文档分散在 UWP 旧文档中。
   - 需要手写 IDL/MIDL，开发体验远逊于 C# 或 Qt Designer。

2. **控件生态缺口**：
   - 没有官方 `DataGrid`（社区有 `WinUI.TableView` 等第三方实现）。
   - `InkCanvas` 仅为实验性；TAS 编辑器的标注/绘图功能需额外方案。
   - 第三方控件库（Telerik、DevExpress）正在跟进，但覆盖度不及 WPF。

3. **打包与分发**：
   - MSIX 是首选分发方式；虽然支持 unpackaged，但运行时依赖 `Microsoft.WindowsAppRuntime`。
   - 有开发者报告 Store 下载版本因打包工具链兼容性而崩溃的案例。

4. **性能反直觉**：
   - 微软官方文档承认：Windows App SDK 2.0 的启动速度、内存占用、安装体积仍高于 UWP（虽然优于 Electron）。
   - 对模拟器这种“窗口多、绘制重、延迟敏感”的应用，需实测验证。

5. ** HWND / 窗口模型限制**：
   - 多窗口支持、自定义窗口边框、HWND interop 仍存在较多 GitHub 上的未解决 issue。
   - 例如 `Window.Close()` 行为与标题栏关闭按钮不一致，需要手动发送 `WM_CLOSE`。

#### 4.1.5 对 FCEUX11 的适配评估

- **ConsoleViewerGL（OpenGL）**：可通过 `SwapChainPanel` 或 `Microsoft.UI.Composition` 的 `SpriteVisual` + `CompositionDrawingSurface` 承载现有 OpenGL/DirectX 渲染。
- **调试器/TAS 编辑器**：大量 `QPainter` 自定义绘制需要重写为 `Microsoft.Graphics.Canvas`（Win2D）或 `Composition` 层。
- **信号槽**：替换为 WinRT 事件、`ICommand`、`x:Bind` 到 ViewModel。
- **线程模型**：模拟器线程可保持 `std::thread`，通过 `DispatcherQueue` 将帧完成事件投递到 UI 线程。

#### 4.1.6 结论

**推荐作为 v2.x 首选目标**，但前提是：
- v1.11 已彻底解耦 Qt；
- 团队愿意接受 C++/WinRT 的学习曲线；
- 计划在 Windows 11 上提供最佳体验，并接受 Windows 10 1809+ 的兼容性下限；
- 准备投入大量时间重写自定义控件和调试/TAS 编辑器。

---

### 4.2 WinUI 2 / UWP —— 已被官方淡化

#### 4.2.1 现状

- **WinUI 2.x**：最后一个稳定版 2.8.7，仅面向 UWP；不再新增功能，仅修 bug/安全。
- **UWP**：微软已公开确认 UWP 进入“维护期”，不再积极开发；新功能集中在 Windows App SDK / WinUI 3。
- **应用模型**：UWP 是沙盒化应用模型，对需要访问大量 Win32 API、文件系统、外部模拟器插件的模拟器极不友好。

#### 4.2.2 可行性

| 维度 | 评估 |
|------|------|
| **可行性** | 低。UWP 应用模型与 FCEUX11 的 Win32 核心、Lua 脚本、外部 ROM/插件、低延迟输入需求存在根本冲突。 |
| **性能** | 理论上优于 WinUI 3（微软官方承认），但 UWP 的沙盒限制会抵消这部分优势。 |
| **兼容性** | Windows 10+，但应用商店分发和权限模型是巨大障碍。 |
| **美观程度** | 与 WinUI 3 相近（Fluent Design 同源）。 |
| **改造难度** | 极高，且需要重写为沙盒应用模型；同时是“死胡同”，迁移价值低。 |

#### 4.2.3 结论

**不建议**。WinUI 2 / UWP 无论从战略方向还是应用模型都不适合 FCEUX11。即使要追求 Fluent 视觉，也应直接选择 WinUI 3。

---

### 4.3 WPF（Windows Presentation Foundation）—— 成熟次选

#### 4.3.1 现状

- WPF 作为 .NET 的一部分，随 .NET 9 / .NET 10 持续更新；微软明确 WPF **不废弃**，处于“维护 + 定向增强”模式。
- .NET 9 引入官方 **Fluent 主题**，支持系统深色模式与强调色，可在视觉上接近 WinUI 3。
- 生态极其成熟：Telerik、DevExpress、Actipro、CommunityToolkit 等第三方控件丰富。
- Visual Studio 设计器、XAML Hot Reload、Blend 等工具链非常稳定。

#### 4.3.2 可行性

| 维度 | 评估 |
|------|------|
| **可行性** | 中高。如果允许引入 .NET 运行时，WPF 是最稳妥的大型桌面应用框架之一。 |
| **性能** | 渲染基于 DirectX，但布局在 UI 线程；复杂界面（如 TAS 编辑器大量自定义绘制）可能掉帧；内存占用高于 WinUI 3。 |
| **兼容性** | 支持 Windows 7+（.NET 9 的 WPF 仍可在 Windows 7 上运行，但官方支持周期已结束）。 |
| **美观程度** | 良好（.NET 9 Fluent 主题），但 Windows 11 原生集成（Mica、Snap Layout）不如 WinUI 3 深。 |
| **改造难度** | 高。需将 C++ Qt 代码重写为 C# XAML；核心与 UI 之间需要 C++/CLI 或 P/Invoke 桥接。 |

#### 4.3.3 优势

1. **工具链成熟**：Visual Studio XAML Designer、Hot Reload、丰富的调试经验。
2. **生态最丰富**：DataGrid、图表、PropertyGrid、Docking 等控件唾手可得；对调试器、TAS 编辑器这种复杂 UI 非常友好。
3. **团队学习曲线平缓**：如果团队有 C# / XAML 背景，上手快于 C++/WinRT。
4. **跨 .NET 生态**：可方便地集成 ML.NET、System.Text.Json、NuGet 生态。

#### 4.3.4 劣势

1. **语言异构**：FCEUX11 核心是 C++20 + Rust。WPF 需要 C# / VB / F# 写 UI，核心与 UI 之间需要 FFI/C++/CLI 桥接，增加维护复杂度。
2. **托管运行时开销**：.NET GC 对延迟敏感型模拟器是潜在风险；需小心管理大数组、帧缓冲、音频缓冲的内存 pinned。
3. **启动与体积**：WPF + .NET 运行时打包后体积较大；Native AOT 对 WPF 支持有限（截至 .NET 10 预览仍在改进）。
4. **不是微软未来主投方向**：虽然不会废弃，但新功能以 Fluent 主题、高 DPI 为主，不会有 WinUI 3 级别的原生集成。

#### 4.3.5 对 FCEUX11 的适配评估

- **混合架构**：可将 WPF 作为 UI 壳，核心仍保留为 C++/Rust DLL；通过 C++/CLI 或 `DllImport`/`LibraryImport` 调用。
- **OpenGL 渲染**：可用 `WindowsFormsHost` 承载 OpenGL 控件，或改用 `SkiaSharp`、`Silk.NET`、`OpenTK`。
- **自定义绘制**：`DrawingVisual`、`WriteableBitmap`、`SkiaSharp`、`SharpDX` 均可替代 QPainter。
- **调试器/TAS 编辑器**：WPF 的 DataGrid、TreeView、Docking 控件生态能显著降低重写成本。

#### 4.3.6 结论

**可作为次选或混合方案**。如果 v2.x 希望快速获得现代化 UI 且团队熟悉 C#/XAML，WPF 是风险较低的选择；但长期看会背负 .NET 运行时和语言桥接的额外负担。不建议作为“彻底放弃 Qt”后的唯一目标，除非同时接受核心 UI 分层重构。

---

### 4.4 WinForms —— 维护模式，不适合旗舰产品

#### 4.4.1 现状

- WinForms 基于 GDI+，是 .NET 时代最早的 Windows UI 框架。
- 微软明确 WinForms **不废弃**，持续接收高 DPI、ARM64、设计器改进等定向更新。
- 但 WinForms 的底层是 Win32 控件 + GDI+，现代视觉表现力有限。

#### 4.4.2 可行性

| 维度 | 评估 |
|------|------|
| **可行性** | 中。技术上可行，但视觉与架构不适合高端模拟器。 |
| **性能** | GDI+ 软渲染，高 DPI 和动画场景性能差；不适合需要 60fps 自定义绘制的模拟器主视图。 |
| **兼容性** | 最好。支持 Windows 7+，甚至可通过 Mono 部分跨平台。 |
| **美观程度** | 差。默认控件为经典 Windows 风格；虽然可通过第三方皮肤库（如 DevExpress、Telerik）美化，但永远不如 WinUI 3 原生。 |
| **改造难度** | 中高。事件模型简单，但大量 Qt 自定义控件需要从头实现；高 DPI 适配繁琐。 |

#### 4.4.3 优势

1. **开发快、学习成本低**：事件驱动模型直观，适合快速原型。
2. **兼容性好**：对旧 Windows 版本支持最友好。
3. **与 Win32 互操作无缝**：可直接调用任何 Win32 API。

#### 4.4.4 劣势

1. **视觉落后**：默认控件无法提供现代 Fluent 体验，需要大量自定义绘制。
2. **高 DPI 支持差**：虽然近年有改进，但与 WPF/WinUI 3 差距明显。
3. **自定义绘制能力弱**：`OnPaint` + GDI+ 难以替代 QPainter 的高性能场景；PPU 查看器、Hex 编辑器、TAS 编辑器会很吃力。
4. **生态停滞**：第三方控件以维护为主，缺乏面向 Windows 11 的创新。

#### 4.4.5 结论

**不建议作为 v2.x 主目标**。WinForms 适合内部工具或维护老旧系统，不适合 FCEUX11 这种需要现代外观和高性能自定义绘制的旗舰模拟器。仅在“必须支持 Windows 7 且资源极度受限”时作为保底方案考虑。

---

### 4.5 Win32 原生（C/C++）—— 兼容性王者，开发效率最低

#### 4.5.1 现状

- Win32 API 是 Windows 桌面开发的底层基础，所有其他框架最终都构建在 Win32 之上。
- “现代 Win32”可通过 DWM API（`DwmSetWindowAttribute`）、DirectComposition、`TaskDialog`、Common Controls 6.0 manifest 获得部分现代外观（深色模式、圆角、Mica/Acrylic 需借助 Windows App SDK 的 Composition 层）。
- 可使用 WTL、MFC、或直接 Win32 API 编写。

#### 4.5.2 可行性

| 维度 | 评估 |
|------|------|
| **可行性** | 高。任何 Windows 应用最终都能回到 Win32；与现有 C++ 核心完全同构。 |
| **性能** | 最高。无额外框架开销，渲染路径完全可控。 |
| **兼容性** | 最好。支持 Windows 7/8/10/11，甚至可通过 Wine 在 Linux/macOS 运行。 |
| **美观程度** | 取决于投入。基础 Win32 控件外观古老；但通过自绘、DWM、DirectComposition 可实现接近 WinUI 的效果，工程量大。 |
| **改造难度** | 最高。约 10 万行 Qt 代码需替换为 Win32 消息循环、自绘控件、布局管理；没有数据绑定、没有布局系统，一切从零开始。 |

#### 4.5.3 优势

1. **完全可控**：窗口消息、渲染、输入、线程模型完全由自己掌握，最适合延迟敏感的模拟器。
2. **最小依赖**：不依赖 .NET 运行时、Windows App SDK、Qt 库，分发体积最小。
3. **与核心代码零摩擦**：C++ 核心可以直接调用 UI 代码，无需 FFI。
4. **长期稳定**：Win32 API 是 Windows 兼容性的基石，未来 20 年不会消失。

#### 4.5.4 劣势

1. **开发效率极低**：没有布局系统、没有数据绑定、没有现代控件；每个对话框都需要手写资源或代码。
2. **自定义控件成本高**：调试器、TAS 编辑器、Hex 编辑器这类复杂 UI 用 Win32 实现是“噩梦级”工程。
3. **现代外观需大量自绘**：Mica/Acrylic、圆角、Fluent 动画需要调用 DWM/Composition/Direct2D，复杂度接近自己实现半个 UI 框架。
4. **高 DPI 适配繁琐**：需要手动处理 `WM_DPICHANGED`、缩放图标和字体。

#### 4.5.5 对 FCEUX11 的适配评估

- **主窗口 + 菜单**：可用标准 Win32 + Common Controls 6.0 快速实现。
- **渲染视图**：直接用 OpenGL/DirectX 在 `HWND` 上渲染，无需额外框架。
- **调试器/TAS 编辑器**：需要大量自绘或引入第三方网格/编辑器控件（如 Scintilla、ListView 自绘），成本极高。
- **布局**：可考虑引入轻量级布局库（如 `libui`、`DirectUI`、`Nuklear`），但这些库又带来新的依赖和限制。

#### 4.5.6 结论

**适合作为“保底路径”或“内核 + 极简 UI”方案**。如果 v2.x 的核心目标是“彻底去 Qt、最小依赖、Windows 全版本兼容”，Win32 原生是唯一彻底可控的终点；但需要接受巨大的开发投入。更现实的用法是：**核心模拟器以 Win32 为底层，复杂工具窗口继续用 WinUI 3 或 WPF 承载**。

---

## 5. 多维度对比矩阵

| 维度 | WinUI 3 | WinUI 2/UWP | WPF | WinForms | Win32 原生 |
|------|:-------:|:-----------:|:---:|:--------:|:----------:|
| **战略推荐度** | ⭐⭐⭐⭐ | ⭐ | ⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐ |
| **技术可行性** | 中高 | 低 | 高 | 中 | 高 |
| **性能潜力** | 高 | 中 | 中高 | 低 | 最高 |
| **Windows 11 原生美观度** | 最高 | 高 | 中高 | 低 | 中（需大量自绘） |
| **Windows 10 兼容性** | 1809+ | 10+ | 7+ | 7+ | 7+ |
| **Windows 7/8 兼容性** | ❌ | ❌ | ✅ | ✅ | ✅ |
| **与 C++20/Rust 核心同构性** | 高（C++/WinRT） | 低（C++/CX） | 低（C#） | 低（C#） | 最高（C++） |
| **现有 Qt 代码可复用度** | 低 | 低 | 低 | 低 | 最低 |
| **自定义控件/复杂 UI 友好度** | 中高 | 中 | 高 | 低 | 低 |
| **工具链成熟度** | 中（无 C++ XAML Designer） | 中 | 高 | 高 | 高 |
| **第三方控件生态** | 成长中 | 停滞 | 最丰富 | 维护中 | 稀少 |
| **分发体积/依赖复杂度** | 中（WASDK runtime） | 高（Store/沙盒） | 中高（.NET runtime） | 中（.NET runtime） | 最低 |
| **团队学习曲线** | 陡峭（C++/WinRT + WinRT） | 陡峭 | 中等 | 平缓 | 陡峭 |
| **改造工期预估** | 12~18 人月 | 不建议 | 10~15 人月 | 8~12 人月 | 18~30 人月 |

> **说明**：改造工期预估基于当前约 10.4 万行 Qt 驱动代码、30+ 个主要窗口/对话框、大量自定义绘制的规模，且假设核心已完成 v1.x 解耦。实际工期受团队对各框架熟悉度影响极大。

---

## 6. 关键场景在各框架下的实现难度

### 6.1 模拟器主窗口与菜单系统

| 框架 | 实现难度 | 说明 |
|------|---------|------|
| WinUI 3 | 中 | `NavigationView`/`MenuBar` 可替代 QMenuBar；自定义标题栏原生支持。 |
| WinUI 2/UWP | 高 | 沙盒限制下文件/ROM 打开困难。 |
| WPF | 低 | `Menu`、`ToolBar`、`StatusBar` 成熟；数据绑定简化状态同步。 |
| WinForms | 低 | `MenuStrip`/`ToolStrip` 快速实现，但外观老旧。 |
| Win32 原生 | 中 | `CreateMenu`/`TrackPopupMenu` 直接可用，但需手写布局。 |

### 6.2 游戏画面渲染（OpenGL/DirectX）

| 框架 | 实现难度 | 说明 |
|------|---------|------|
| WinUI 3 | 中 | `SwapChainPanel` / `Composition` 可承载；但透明与 Acrylic 采样有限制。 |
| WinUI 2/UWP | 高 | UWP 对 OpenGL 支持有限，通常需 ANGLE 或 DirectX。 |
| WPF | 中 | `WindowsFormsHost` + OpenGL 控件，或改用 `SkiaSharp`/`Silk.NET`。 |
| WinForms | 低 | `Control.Handle` 直接做 OpenGL 上下文，最简单。 |
| Win32 原生 | 低 | `HWND` 直接创建 OpenGL/DirectX 上下文，完全可控。 |

### 6.3 调试器（反汇编、断点、内存/寄存器视图）

| 框架 | 实现难度 | 说明 |
|------|---------|------|
| WinUI 3 | 高 | 无内置 DataGrid/Hex 控件，需大量自定义或第三方。 |
| WinUI 2/UWP | 高 | 同 WinUI 3，且沙盒限制。 |
| WPF | 中 | 第三方 DataGrid、AvalonEdit/ICSharpCode 等可复用。 |
| WinForms | 高 | DataGridView 功能有限，大量自绘。 |
| Win32 原生 | 极高 | 需自绘列表/树/编辑控件，或集成 Scintilla。 |

### 6.4 TAS 编辑器（时间线、钢琴卷、书签、分支）

| 框架 | 实现难度 | 说明 |
|------|---------|------|
| WinUI 3 | 高 | PianoRoll 自定义绘制可用 Win2D/Composition；但复杂交互需自研。 |
| WinUI 2/UWP | 高 | 同 WinUI 3。 |
| WPF | 中 | `Canvas`、`ItemsControl`、WriteableBitmap 适合时间线；第三方图表库可选。 |
| WinForms | 高 | GDI+ 绘制 PianoRoll 性能瓶颈明显。 |
| Win32 原生 | 极高 | 几乎需从零实现所有交互控件。 |

### 6.5 输入配置与热键

| 框架 | 实现难度 | 说明 |
|------|---------|------|
| WinUI 3 | 中 | 键盘/手柄输入通过 WinRT `Input` API，低延迟需额外处理。 |
| WinUI 2/UWP | 高 | UWP 输入模型与模拟器需求差异大。 |
| WPF | 低 | `KeyBinding`、`InputBindings` 成熟；XInput 需 P/Invoke。 |
| WinForms | 低 | 事件模型简单，XInput 直接 P/Invoke。 |
| Win32 原生 | 低 | `Raw Input`/`GetAsyncKeyState`/XInput 直接可用，延迟最低。 |

---

## 7. 推荐的 v2.x 迁移路径

### 7.1 总体策略：分阶段、可回退

FCEUX11 作为长期维护的模拟器项目，不应一次性完成“Qt → 某单一微软框架”的赌博式迁移。建议采用以下分层策略：

```
v1.14 收官
   │
   ▼
v2.0 启动：保留 Qt，先建立新的 Core-Driver 边界（已完成 v1.11 Bridge）
   │
   ├── 阶段 A（v2.0~v2.1）：新增 WinUI 3 驱动壳
   │      - 仅实现主窗口、菜单、渲染视图、基本配置对话框
   │      - 验证 C++/WinRT + OpenGL/DirectX + 模拟器线程模型
   │      - Qt 驱动继续并行维护
   │
   ├── 阶段 B（v2.2~v2.4）：窗口级逐步迁移
   │      - 按使用频率迁移：About → Movie → Cheats → Input → Video/Sound → HexEditor → NameTable/PPUViewer → TraceLogger → GamePadConf → Debugger → TAS Editor
   │      - 每个窗口迁移后回归测试
   │
   ├── 阶段 C（v2.5~v2.6）：TAS 编辑器与调试器大模块迁移
   │      - 这两个模块合计近 3 万行，需要专门版本
   │      - 考虑在 WinUI 3 下引入 Win2D / 第三方 DataGrid 降低自研成本
   │
   └── 阶段 D（v2.7+）：Qt 驱动退役
          - 当 WinUI 3 驱动功能覆盖度 ≥ 95% 且稳定 2 个版本后移除 Qt
          - 同步清理 CMake 中的 Qt 依赖、i18n、预编译头
```

### 7.2 如果 WinUI 3 在 v2.x 期间出现战略变故

微软 UI 框架历史反复证明需要“留后路”。建议 v2.x 同时维护两条轻量路径：

1. **主路径**：WinUI 3（C++/WinRT）—— 现代外观、原生体验。
2. **保底路径**：Win32 原生（轻量 UI 壳）—— 确保在以下场景仍可编译运行：
   - WinUI 3 出现不可接受的 bug 或战略调整；
   - 需要支持 Windows 7/8；
   - 需要最小分发体积。

> 保底路径不需要一开始就完整实现所有工具窗口，只需保证“能加载 ROM、能运行、能暂停/保存/加载”即可；复杂调试/TAS 功能可标记为“在旧版 Qt 或 WinUI 3 下可用”。

### 7.3 不建议的路线

- **不要直接“Qt → UWP/WinUI 2”**：应用模型不兼容，且 UWP 已边缘化。
- **不要直接“Qt → WinForms”**：无法满足模拟器对现代 UI 和自定义绘制的需求。
- **不要“Qt → WPF”作为唯一终点**：虽然可行，但会引入 .NET 运行时和 C#/C++ 桥接的长期负担；除非团队强烈偏好 C#，否则应优先 WinUI 3 或 Win32。

---

## 8. 风险矩阵与缓解策略

| 风险 | 影响 | 可能性 | 缓解策略 |
|------|------|--------|---------|
| **C++/WinRT 工具链不成熟**（无 Designer、文档分散、维护模式传言） | 高 | 中 | 先在 v2.0 做最小 PoC；保留 Win32 保底路径；必要时用 C# 写工具窗口并通过 COM 互操作。 |
| **WinUI 3 战略方向突变** | 高 | 中 | 坚持分阶段迁移，Qt 并行维护至 v2.7+；核心不依赖特定 UI 框架。 |
| **TAS 编辑器/调试器重写成本爆炸** | 高 | 高 | 拆分后按子模块迁移；引入 Win2D/第三方控件；保留旧 Qt 窗口作为 fallback。 |
| **性能退步（渲染延迟、输入延迟）** | 高 | 中 | 每个阶段跑 `bench_full_frame` + 输入延迟基准；OpenGL 渲染视图尽量接近底层。 |
| **Windows 7/8 用户流失** | 中 | 低~中 | 若用户调研显示旧系统占比高，同步维护 Win32 原生或保留 Qt 构建。 |
| **i18n 方案替换成本** | 中 | 中 | 提前在 v2.0 设计统一的资源字符串表，WinUI 3 用 `.resw`，WPF 用 `.resx`，Win32 用 `.rc`/`.po`。 |
| **构建系统复杂度上升** | 中 | 高 | CMake 增加 `FCEUX11_UI_BACKEND` 选项（Qt/WinUI3/Win32），CI 同时覆盖。 |
| **第三方插件/Lua 脚本与 UI 交互** | 中 | 中 | 通过 `DriverCallbacks` 暴露稳定 API；v2.x 保持接口兼容。 |

---

## 9. 结论与建议

### 9.1 最终结论

1. **v1.x 期间不启动 UI 框架迁移**。必须等到 `v1.14 Anvil` 全部验收标准通过，核心已完成对象化、Rust 迁移、Qt 解耦、巨型文件拆分。
2. **v2.x 首选目标：WinUI 3（C++/WinRT）**。它是微软当前官方主推、与 Windows 11 原生体验最契合、与 C++ 核心同构的框架；尽管 C++/WinRT 工具链仍有摩擦，但长期方向最明确。
3. **v2.x 保底路径：Win32 原生**。为兼容性、性能、可控性、战略风险提供退路。
4. **WPF 可作为过渡/混合方案**，但不建议作为彻底放弃 Qt 后的唯一终点。
5. **WinUI 2/UWP 与 WinForms 不建议**作为 FCEUX11 这种高性能、复杂桌面工具的迁移目标。

### 9.2 即刻可做的准备工作（不影响 v1.x 主线）

虽然迁移本身要推迟到 v2.x，但以下工作可在 v1.x 期间“顺手”完成，以降低未来成本：

- [ ] 在 v1.11 Bridge 中，将 `fceu11::DriverCallbacks` 接口设计得足够通用，不隐含 Qt 概念（如用 `std::function` 替代 Qt 信号槽类型）。
- [ ] 将 Qt 驱动中的“纯 UI 无关逻辑”（如最近 ROM 列表、配置序列化）下沉到 `drivers/common/` 或 Rust 层。
- [ ] 为 `ConsoleViewerGL_t` 的 OpenGL 渲染抽象出一个与 Qt 无关的接口，便于未来接入 WinUI 3 的 `SwapChainPanel` 或 Win32 的 `HWND`。
- [ ] 在 CI 中保留一个“无 UI 核心测试”构建配置，验证核心不依赖 Qt 也能编译链接。
- [ ] 建立 UI 无关的字符串资源表，为未来的 `.resw`/`.resx`/`.rc` 本地化做准备。

### 9.3 v2.x 启动时的第一动作

当 v2.0 里程碑启动时，建议按以下顺序行动：

1. 创建 `src/drivers/winui3/` 目录，实现最小可运行应用：
   - 创建主窗口；
   - 加载 ROM；
   - 运行模拟器线程；
   - 通过 `DriverCallbacks` 将帧缓冲提交到 `SwapChainPanel` 显示；
   - 实现“暂停/继续/重置”菜单。
2. 同时创建 `src/drivers/win32/` 目录，实现同样的最小功能集。
3. 让 CI 同时构建 `Qt`、`WinUI 3`、`Win32` 三个后端，确保核心接口不被破坏。
4. 只有当 WinUI 3 的最小集稳定运行 1~2 个小版本后，才进入窗口级逐步迁移。

---

## 10. 参考来源

- FCEUX11 项目内部文档：`docs/v1.x_Modernization_Roadmap.md`
- Microsoft Learn: [What's supported when migrating from UWP to WinUI 3](https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/migrate-to-windows-app-sdk/what-is-supported)
- Microsoft Learn: [Windows developer FAQ](https://learn.microsoft.com/en-us/windows/apps/get-started/windows-developer-faq)
- Microsoft Learn: [Apply Mica in Win32 desktop apps](https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/ui/apply-mica-win32)
- Microsoft Learn: [Apply rounded corners in desktop apps for Windows 11](https://github.com/MicrosoftDocs/windows-dev-docs/blob/docs/hub/apps/desktop/modernize/ui/apply-rounded-corners.md)
- CTCO Blog: [WinUI vs WPF in 2026](https://www.ctco.blog/posts/winui-vs-wpf-2026-practical-comparison/)
- GitHub Discussion: [Ohh...WinUI3 is really dead!](https://github.com/microsoft/microsoft-ui-xaml/discussions/9417)
- WindowsForum: [Build 2026: WinUI 3, Windows App SDK, and AI Agents](https://windowsforum.com/threads/build-2026-winui-3-windows-app-sdk-and-ai-agents-push-native-windows-apps.422225/)
- Avalonia UI Blog: [WinUI vs WPF vs UWP](https://avaloniaui.net/blog/winui-vs-wpf-vs-uwp)

---

*本报告为技术预研文档，不构成工程决策的最终指令。具体迁移时机、目标框架与路径应在 v1.x 收官后由维护团队根据当时各框架成熟度、团队技能储备和用户反馈重新评估。*
