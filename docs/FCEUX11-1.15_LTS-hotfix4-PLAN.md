# FCEUX11 v1.15 LTS hotfix4 PLAN
## 主界面菜单已确认缺陷修复

### 1. 已确认缺陷

扫描发现以下两处源码包含字面量 `` `n ``：

- `src/drivers/Qt/ConsoleTranslation.cpp`
- `src/drivers/Qt/ConsoleActions.cpp`

问题形态类似：

```cpp
#include "Qt/ConsoleTranslation.h" `n #include "Qt/ConsoleWindow.h"
```

`` `n `` 应为真实换行，但被脚本编辑残留在源码中。其影响包括：

- 后续 `#include` 可能不再被预处理器识别为独立指令；
- 产生编译警告；
- 在启用 `/WX` 时可能导致构建失败；
- 当前可能因 `ConsoleWindow.h` 在其他位置已提前包含而暂未暴露为明显运行时故障。

这属于实际源码缺陷，必须修复，不能作为“潜在风险”保留。

### 2. 修复要求

1. 将两处字面量 `` `n `` 替换为正常换行；
2. 保持 include 顺序和功能不变，不进行菜单重构；
3. 检查 Qt 源码中是否还有同类字面量残留；
4. 执行 Release/Debug 编译，确保无新增警告；
5. 执行现有 CTest，至少包括：

```powershell
cmake --build build --config Release
ctest --test-dir build --build-config Release --output-on-failure
```

### 3. 验收标准

- 两个文件中的 include 均为独立、合法的预处理指令；
- `src` 下无同类 `` `n `` 残留；
- Release 和 Debug 构建通过；
- 现有 CTest 全部通过；
- 本次 diff 不改变任何菜单路径、快捷键、槽函数或运行时行为。

### 4. 当前结论

本轮扫描目前只确认了上述源码编辑残留这一项实际缺陷，尚未确认某个具体菜单功能已经失效。菜单行为测试不足属于测试覆盖缺口，不应在本 PLAN 中当作已发生的功能缺陷。
