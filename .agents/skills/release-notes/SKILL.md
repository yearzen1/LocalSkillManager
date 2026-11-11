---
name: release-notes
description: 写发布说明 / Release Note 时套用固定双语模板，输出 `### 中文` 与 `### English` 两段，每段一行一条、行内不用分号，中英条数一一对应，只写用户可感知的变化。Use when writing release notes, changelog entries for a release, or a GitHub Release body.
---

# Release Notes

## 1. 输出契约

产物就是完整的 release body，两个标题由本 skill 写：

```markdown
### 中文
- 待办输入草稿跨页面保留
- 今日页显示截止日期与倒计时，挂件不再显示截止信息
- 胶囊右键支持展开与隐藏
- 统一 Chip 与菜单样式，设置页支持开发构建预览更新分区，移除侧栏底部版本行

### English
- Keep todo quick-add drafts across views
- Show deadline dates on Today while hiding them in the widget
- Add a capsule context menu with expand and hide
- Unify chip and menu styling, preview the Updates section in dev builds, remove the sidebar version footer
```

| 规则 | 要求 |
|---|---|
| 结构 | `### 中文` 在前、`### English` 在后，各一段，段间空一行 |
| 条数 | 1–5 条；中文条数 = 英文条数，逐条同序对应 |
| 行 | 一行一条，`- ` 开头，**行内不出现分号** |
| 中文 | 不加句号；中文与拉丁字母、数字之间留半角空格 |
| 英文 | 首字母大写、祈使句（`Add` / `Show` / `Keep` / `Remove` / `Fix`）、不加句号 |
| 范围 | 只写用户可感知的变化 |
| 禁止 | 版本号、日期、署名、「本次更新」/「What's new」、emoji、链接、文件路径、函数名、组件名、type 前缀 |

**唯一例外**：项目发版脚本自己拼这两个标题时（脚本输出形如 `### 中文\n…\n\n### English\n…`），只提供两段 bullet 正文，不要再写标题，否则重复。

## 2. 取材

按顺序回退，够用就停：

1. 提交历史：`git log --oneline "$(git describe --tags --abbrev=0)..HEAD"`；仓库还没有 tag 时用 `git log --oneline -30`。
2. 提交信息不规范时：`gh pr list --state merged --limit 30` 的标题，配 `git diff --stat <上一版tag>..HEAD`。
3. 上一版说明的调性：`gh release view <上一版tag>`；仓库有 `CHANGELOG.md` 就对照最新一段。
4. 只有判断不了影响面时才读具体 diff，不要逐行通读。

每条都要能指回一个 commit 或 PR，不凭印象补。

## 3. 写作流程

1. 列出本次全部改动。
2. 合并在同一处 UI / 同一交互上的改动，一条只讲一件事。
3. 排序：新增能力 → 体验修正 → 移除 / 调整。
4. 写中文，控制在 1–5 条。
5. 逐条对译英文，保持条数与顺序不变。
6. 过第 6 节自查清单。

单条改动的小补丁就写 1 条，不要凑数；超过 5 条说明该合并同类项。

## 4. 什么算「用户可感知」

- **写**：新按钮、新交互、可见行为变化、操作路径变短、性能体感、破坏性变更。
- **不写**：重构、内部 API 改名、依赖升级（除非修掉用户可见问题）、测试与 CI、代码风格、纯文档。

`refactor` / `chore` / `test` / `build` / `docs` 类提交默认不入条目；只有它们改变了用户看到的结果，才折算成用户视角的一句话。

## 5. 正反例对照

| 写法 | 判定 | 原因 |
|---|---|---|
| 中文段整段用「；」串成一句话 | ✗ | 必须一行一条 |
| 中文 4 条、英文 3 条 | ✗ | 条数与顺序必须对应 |
| 自己再加 `## 更新说明` 或 `**更新内容**` | ✗ | 只允许两个 `###` 标题 |
| `- 优化 store 结构，抽公共函数` | ✗ | 用户不可感知 |
| `- 升级 Vite 到 7` | ✗ | 依赖升级，除非修掉用户可见问题 |
| `- 新增 X。` | ✗ | 中文不加句号 |
| `- 🎉 新增 X` | ✗ | 不用 emoji |
| `- add X` | ✗ | 英文首字母没大写 |
| `- Add X.` | ✗ | 英文不加句号 |
| `- 新增 X; Add X` | ✗ | 中英分两段，不挤在同一行 |
| `- 新增 X` / `- Add X` | ✓ | |

## 6. 自查清单

1. 两个标题都在、顺序正确、各一段吗？
2. 中文 1–5 条，英文条数相同、逐条对应吗？
3. 每行都是 `- ` 开头、一条一行、行内没有分号吗？
4. 中文没句号、中拉之间有空隙吗？英文首字母大写、祈使句、无句号吗？
5. 有没有版本号、日期、「本次更新」、emoji、链接、内部实现词？
6. 每条都能指回一个 commit 或 PR 吗？
7. 项目脚本已自带标题时，是不是只提交了两段正文？

## 7. 交付

默认只写文案，不构建、不打 tag、不上传资产；用户明确要求发版时再走项目自己的流程。

主路径（`--notes-file` 免去多行引号问题）：

```bash
# 先确认上次 tag：git describe --tags --abbrev=0
# 写好 release-notes.md 后
gh release create v1.2.3 --notes-file release-notes.md

# 已存在的 release 补充说明
gh release edit v1.2.3 --notes-file release-notes.md
```

只能用内联参数时，Windows 用 here-string，避免换行被吞：

```powershell
$notes = @'
### 中文
- 条目一
- 条目二

### English
- First item
- Second item
'@
gh release create v1.2.3 --notes $notes
```

项目已有发版脚本时以脚本参数为准，例如 `node scripts/release.mjs <ver> --notes "<中文>" --notes-en "<英文>"`，多行内容同样用 here-string 或临时文件传入。

## 8. 与其他 skill 的分工

- `conventional-commit`：管单条提交信息（一行、双语、禁止 body）。
- 本 skill：管整版发布说明（两段、逐条一行）。
- 同一份改动两边各写各的，措辞不必一致；提交信息里的 `refactor` 等内部改动不必出现在发布说明里。
