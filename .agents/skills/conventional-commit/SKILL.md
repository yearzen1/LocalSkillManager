---
name: conventional-commit
description: 写提交信息、拆分原子提交、或 review 提交历史时套用 Conventional Commits。首行中英双语、禁止写 body，含 type 白名单、100 字符与提交前自查清单。Use when writing or reviewing git commit messages.
---

# Conventional Commits

## 1. 格式

```text
<type>(<scope>): <English subject>; <type>(<scope>): <中文主题>
<空行>
Refs: #123                 # 可选，唯一允许的 footer
```

首行示例：

```text
docs: correct Android SDK setup instructions; docs: 修正仓库不自带 Android SDK 的构建说明
```

- 首行必须**双语**：英文在前、中文在后，用 `; ` 分隔。
- 两段**各自重复** `type(scope)`，中文段不能省略。
- **禁止 body**：首行之后只允许空行和 `Refs: #123`，不写任何解释性段落。「为什么」写进 PR/MR 描述，不写进 commit message。
- `scope` 可省略：`docs: ...; docs: ...` 合法。
- 破坏性变更只在 type 后加 `!`：`feat(cli)!: drop the upgrade flag; feat(cli)!: 移除升级开关`，**不写** `BREAKING CHANGE` footer，说明放 PR/MR 描述。

## 2. type 白名单

只允许这些，自造 type（`feature`、`update`、`improvement`）一律不认：

| type | 用途 |
|---|---|
| `feat` | 用户可见的新能力 |
| `fix` | 修正错误行为 |
| `docs` | 只改文档、注释、README |
| `test` | 只加/改测试 |
| `refactor` | 不改行为的结构调整 |
| `perf` | 性能 |
| `style` | 格式、空白，不改语义 |
| `build` | 依赖、构建脚本、版本号 |
| `ci` | 流水线配置 |
| `chore` | 杂务（gitignore、脚手架） |
| `revert` | 撤销既有提交 |

**判不准时按"对使用者的影响"选**：影响行为就是 `feat`/`fix`，只影响开发者就是 `chore`/`build`/`test`/`docs`。

## 3. scope 怎么写

写实际受影响的模块名，小写，用 `-` 连接，没有固定清单；拿不准就不写，省略 `scope` 永远合法。

不要造 `misc`、`core`、`*` 这类含糊 scope。

## 4. 六条硬规则

1. **双语必填**：英文段 + 中文段，缺任一段都不合格。
2. **两段都写 `type(scope)`**：`docs: english; 中文` 这种省略不认。
3. **英文段用英文祈使句、小写开头**：`add` / `drop` / `wrap` / `stop`，不写 `added` / `Adds`。
4. **中文段不加句号**；中文与拉丁字母、数字之间留半角空格。
5. **整行 ≤ 100 字符**（中文按 1 字符计）。超了说明 subject 信息量不够，改写而不是缩写。
6. **不写 body**，首行之后只允许可选的 `Refs:` footer。

```text
feat(list): wrap descriptions with hanging indent; feat(list): 描述过长时按列宽缩进换行
```

## 5. 正反对照

| 写法 | 判定 | 原因 |
|---|---|---|
| `feature(list): wrap x; feature(list): 换行` | ✗ | 未知 type |
| `feat(list): wrap x` | ✗ | 缺中文段 |
| `feat(list): 缩进换行` | ✗ | 缺英文段 |
| `feat(list): Wrap x; feat(list): 换行` | ✗ | 英文大写开头 |
| `feat(list): wrap x; 换行` | ✗ | 中文段缺 type |
| `feat(list): wrap x.; feat(list): 换行。` | ✗ | 句号结尾 |
| 双语首行 + 正文段落 | ✗ | 禁止 body |
| 双语首行 + `BREAKING CHANGE:` footer | ✗ | footer 只允许 `Refs:` |
| `feat: update code; feat: 更新代码` | ✗ | subject 没有信息量 |
| `feat(list): wrap descriptions with hanging indent; feat(list): 描述过长时按列宽缩进换行` | ✓ | |
| `feat(cli)!: drop the upgrade flag; feat(cli)!: 移除升级开关` | ✓ | 破坏性变更只用 `!` 标记 |

## 6. 拆分原则

一个提交只表达一个意图，能单独 revert。

- TDD 循环拆成 `test:`（红）→ `feat:`（绿）→ `refactor:`，本仓库历史即如此。
- 文档与实现分开提交，避免 revert 实现时连带删掉文档。
- 重构与改行为**不要**混在一个提交里。
- 一次提交里出现两个不相关的目录，通常说明该拆。
- 说不清「为什么」时，要么拆提交，要么把意图写进双语首行——**不要**用 body 补救。

## 7. 提交前自查

1. 首行能不看 diff 就说出改了什么吗？
2. 英文段和中文段都在，且**两段都带 `type(scope)`** 吗？
3. type 是按"对使用者的影响"选的，还是按"改了哪些文件"选的？
4. scope 是实际受影响的模块名吗？含糊就删掉。
5. 英文段祈使句、小写开头？中文段没句号、中拉之间有空隙？
6. 整行 ≤ 100 字符吗？
7. **有 body 吗？有就删掉**；除 `Refs:` 外，首行之后不该有任何行。

## 8. 附录：需要硬约束时（可选）

本仓库默认**不装**钩子，靠这份约定加 review。若某个阶段需要强制，可自行加 `commit-msg` 钩子：

```sh
# .githooks/commit-msg，行尾必须是 LF，环境需 UTF-8 locale
#!/bin/sh
TYPE='(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)'
subject=$(sed -n '1p' "$1" | tr -d '\r')

printf '%s\n' "$subject" \
  | grep -Eq "^$TYPE(\([a-z0-9._-]+\))?!?: [^;]+; $TYPE(\([a-z0-9._-]+\))?!?: .*[一-龥]" \
  || { printf '首行不是双语 Conventional Commits：%s\n' "$subject" >&2; exit 1; }

[ "${#subject}" -le 100 ] \
  || { printf '首行超过 100 字符（%s）：%s\n' "${#subject}" "$subject" >&2; exit 1; }

# 禁止 body：非空行只允许 Refs footer
sed -n '2,$p' "$1" | tr -d '\r' \
  | grep -Ev '^(|Refs: .*)$' \
  && { printf '首行之后只允许 Refs: footer\n' >&2; exit 1; }
```

```bash
git config core.hooksPath .githooks   # 每个克隆执行一次
```

注意四点：钩子文件必须是 LF 换行、文件名不带扩展名、`${#subject}` 在部分 shell 按字节计数（要按字符算就换 `awk`）、`--no-verify` 能绕过，所以它只是"提醒"，真正的约束要靠 review。
