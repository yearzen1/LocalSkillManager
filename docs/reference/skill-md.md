# SKILL.md 参考

每个 skill 目录的核心文件是 `SKILL.md`。`lsm` 只读取开头 `---` 包裹的 YAML frontmatter 中的 `name` 与 `description`，其余内容原样保留。

## 文件结构

```markdown
---
name: code-review
description: 审查 diff 的正确性与风格。
---

# Code Review

（给 agent 的指令正文）
```

目录结构：

```text
<skills dir>/
└── code-review/
    ├── SKILL.md        # 必须是这个文件名
    ├── scripts/        # 可选，安装时完整复制
    └── assets/         # 可选
```

## 字段

| 字段 | 类型 | 必填 | 约束 |
|---|---|---|---|
| `name` | string | 是 | 小写字母、数字与单个连字符；首尾必须是字母或数字；≤64 字符；非 Windows 保留名 |
| `description` | string | 是 | trim 后非空；≤1024 字符（按 Unicode 字符计，不按字节） |
| 其他字段 | 任意 | 否 | 原样保留，不参与校验、不改写 |

### `name`

- 合法示例：`a`、`my-skill`、`code-review-2`；
- 非法示例：`My-Skill`（大写）、`a--b`（连续连字符）、`-a`（首字符连字符）、`a_`（下划线）、`中文`；
- 长度按 `chars().count()` 计算，超过 64 报错，不截断；
- Windows 保留设备名（大小写不敏感）：`con`、`prn`、`aux`、`nul`、`com1`–`com9`、`lpt1`–`lpt9`。

### `description`

- 必须非空；
- 超过 1024 字符报错，不截断；
- `list` 文本输出会把连续空白压成单个空格，再按 60 显示列换行；JSON 保留原始值。

### 目录名

**目录名是 skill 的唯一身份**：安装目录时，目录名必须等于 frontmatter 的 `name`。

- 不一致时，`list` 把该目录标记为 `[invalid]`；
- `pull` / `push` 会把不一致的源视为非法；
- 需要改名时，先把目录名改成与 `name` 一致，再重新安装。

## 解析规则

1. 文件必须是 UTF-8；含 BOM 时先去掉 BOM。
2. 只解析文件开头被 `---` 包裹的 frontmatter 块；CRLF 行尾兼容。
3. 必须含 `name` 与 `description`，且 trim 后非空。
4. YAML 语法非法时报错，不会尝试修复。
5. frontmatter 块之外的正文与额外字段不做改写。

YAML 值里包含 `: `（冒号加空格）时会解析失败。两种修法：

```yaml
# 加引号
description: "Fix: the failing test"

# 或使用折叠块
description: >-
  Fix the failing test
  without changing behavior.
```

## 安装行为

- 文件源（`lsm add foo.md`）：文件被复制为 `<name>/SKILL.md`；
- 目录源（`lsm add dir`）：目录内容递归完整复制，**不做任何排除**，隐藏文件、`scripts/`、`assets/`、`.git` 都会复制；
- 目录源含 `.git` 时向 stderr 打印一行 warning；
- 符号链接不支持，遇到即报错；
- 文件内容在安装时保持字节级不变，不重新序列化 YAML。

## 错误

| 症状 | 含义 |
|---|---|
| `missing frontmatter block` | 文件开头没有 `---` 块 |
| `missing required field 'name'` / `'description'` | 字段缺失或 trim 后为空 |
| `invalid YAML frontmatter` | YAML 语法非法 |
| `invalid skill name` | `name` 违反格式、长度或保留名规则 |
| `skill description is too long` | description 超过 1024 字符 |
| `directory name ... does not match frontmatter name ...` | 目录名与 `name` 不一致 |
| `symbolic links are not supported` | 源目录含符号链接 |

完整校验流程见 [`../impl/storage.md`](../impl/storage.md) §2。

## 示例

最小合法文件：

```markdown
---
name: my-skill
description: One-line description.
---
```

带正文与额外字段：

```markdown
---
name: my-skill
description: One-line description.
version: 1
author: me
---

# My Skill

Follow these instructions.
```

`version` 与 `author` 不参与校验，安装时原样保留。
