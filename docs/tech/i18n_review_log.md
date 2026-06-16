# i18n Native Speaker Review Log (v0.3.15 PR-C)

> **状态**: ⏳ **PENDING** — 待母语审校者签字
> **目标覆盖率**: zh_CN ≥ 90% (3,133/3,481 条), zh_TW ≥ 90% (3,133/3,481 条)
> **本 PR 交付**: 20 条主菜单种子翻译 + 翻译工作流工具链
> **待办**: 全量机翻 + 母语审校

## 母语审校者（待招募）

| 语言 | 审校者 ID | GitHub | 签字日期 | 范围 |
|------|----------|--------|---------|------|
| zh_CN | _TBD_ | _TBD_ | _TBD_ | 待 3,481 条全量翻译后签字 |
| zh_TW | _TBD_ | _TBD_ | _TBD_ | 待 3,481 条全量翻译后签字 |

## PR-C 已完成

- [x] 20 条主菜单种子翻译（en / zh_CN / zh_TW 一一对应）写入 `src/drivers/Qt/lang/fceux11_*.ts`
- [x] `glossary.txt` 含 80 项核心术语
- [x] `scripts/i18n_update.ps1` — 调 lupdate 同步
- [x] `scripts/i18n_release.ps1` — 调 lrelease 编译
- [x] `scripts/i18n_coverage.ps1` — 覆盖率闸门
- [x] `scripts/check_simp_trad.ps1` — 繁简字串隔离校验
- [x] `translations.pro` — Qt Linguist 工程
- [x] `lang/README.md` — 翻译工作流

## 待办（v0.3.15.x hotfix 路径）

- [ ] 运行 `lupdate -project translations.pro` 扫描全部 3,481 条 `tr()` 源串
- [ ] 用 DeepL API（或 Google Translate）+ `glossary.txt` 全量机翻 zh_CN → `lang/auto/zh_CN.ts`
- [ ] 母语审校者（zh_CN）逐条校对 `lang/auto/zh_CN.ts` → `lang/fceux11_zh_CN.ts`
- [ ] 用 DeepL API 全量机翻 zh_TW → `lang/auto/zh_TW.ts`
- [ ] 母语审校者（zh_TW）逐条校对 → `lang/fceux11_zh_TW.ts`
- [ ] CI 闸门：覆盖率 ≥ 90% + 繁简隔离通过
- [ ] 母语审校者在本文件签字
- [ ] 4 种 IME（微软 / 搜狗 / QQ / 谷歌）手动测试合成态

## 翻译工序（v0.3.15.x 详细步骤）

```bash
# 1. 拉取最新 lupdate 输出（在 PR-B CMake 集成后会自动跑）
powershell scripts/i18n_update.ps1

# 2. 自动化机翻
#    - DeepL API key 通过环境变量 DEEPL_API_KEY 传入
#    - glossary.txt 上传到 DeepL 后台作为术语表
#    - 输出 lang/auto/zh_CN.ts 与 lang/auto/zh_TW.ts（unfinished type）

# 3. 母语审校
#    linguist lang/auto/zh_CN.ts    # 母语审校者
#    linguist lang/auto/zh_TW.ts    # 母语审校者

# 4. 复制审校结果到正式 .ts
copy lang/auto/zh_CN.ts lang/fceux11_zh_CN.ts
copy lang/auto/zh_TW.ts lang/fceux11_zh_TW.ts

# 5. CI 闸门
powershell scripts/i18n_coverage.ps1
powershell scripts/check_simp_trad.ps1

# 6. 编译 .qm
powershell scripts/i18n_release.ps1

# 7. 母语审校者在本文件签字
```

## 闸门与已知问题

**当前已知会失败的闸**（必须 v0.3.15.x 修复）：
- `i18n_coverage.ps1` → zh_CN/zh_TW 覆盖率 < 90%（当前仅 20 条已翻译）
- `check_simp_trad.ps1` → 应通过（20 条已严格按繁简对照表翻译）

**已知翻译质量风险**：
- 调试器 `ConsoleDebugger.cpp` 的 507 条 tr()（★ 优先级）含大量技术术语，需资深 NES 模拟器用户审校
- TAS Editor 的 521 条 tr()（★ 优先级）含 TAS 社区俚语
- 错误信息 / 异常路径的低频串可接受英文回退

## 见 PR-D 闸 4

v0.3.15 PR-D 验收要求「启动 → 切到 zh_CN → 5+1 菜单结构 + 已翻译 UI 切中 + 4 IME 正常」。**该闸门在 v0.3.15.x 完成全量翻译后才能真正过**；v0.3.15 主版本发布时本文件应留空（即"待办"路径），CHANGELOG 标注 `[i18n] zh_CN/zh_TW 90%+ machine-translation coverage pending native review`。
