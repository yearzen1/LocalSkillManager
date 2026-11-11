# 配置文件参考

`lsm` 的持久化状态分成两份全局文件：`config.toml` 只保存输出语言 `lang`，`groups.toml` 保存 skill 分组。配置与技能目录分离：`LSM_CONFIG_DIR` 只影响这两份文件，`LSM_HOME` 只影响 skills 目录。

## 路径

| 文件 | Windows | Linux | macOS |
|---|---|---|---|
| `config.toml` | `%APPDATA%\lsm\config.toml` | `~/.config/lsm/config.toml` | `~/Library/Application Support/lsm/config.toml` |
| `groups.toml` | `%APPDATA%\lsm\groups.toml` | `~/.config/lsm/groups.toml` | `~/Library/Application Support/lsm/groups.toml` |

- `LSM_CONFIG_DIR` 覆盖配置目录；空字符串按「未设置」处理。
- 两个文件名固定，始终同目录。
- 配置目录无法确定时，命令退出 1，hint 指向 `LSM_CONFIG_DIR`。
- 文件缺失不是错误：`lang` 使用默认值 `en`，分组视为空。
- `config set` 只写 `config.toml`，`group *` 只写 `groups.toml`，两者不会互相覆盖。

## 结构

`config.toml`：

```toml
# Output language: en | zh
lang = "en"
```

`groups.toml`（顶层表，没有 `[groups]` 包裹）：

```toml
# Skill groups: name = [skill, "@group"]
frontend = ["tdd", "grill", "intent-lock"]
docs = ["docs-writer"]
common = ["@frontend", "conventional-commit"]
```

### `lang`

| 项 | 值 |
|---|---|
| 类型 | string |
| 必填 | 否 |
| 默认 | `en` |
| 合法值 | `en`、`zh`；写入时接受并归一化 `en-US`、`zh-CN` |
| 非法值 | 运行时错误，退出 1 |

读取优先级：`--lang` > `LSM_LANG` > 配置文件 `lang` > `en`。`--lang` 或合法的 `LSM_LANG` 生效时，语言解析不会读取配置文件。

### 分组（`groups.toml`）

| 项 | 规则 |
|---|---|
| 类型 | TOML 顶层表 |
| 键 | 分组名，必须符合 skill 名规则 |
| 值 | 字符串数组，每个元素是 skill 名或 `@组名` 引用 |
| 成员顺序 | 保留定义顺序 |
| 重复成员 | 读取与写入时按首次出现去重 |
| 空数组 | 允许存在；`pull --group` 会把它记为失败并提示组为空 |
| 缺失 | 等价于没有分组 |
| 成员是否必须已安装 | `group set/add` 写入时 skill 必须在 global 中已安装；手改文件可写入缺失成员，由 `pull --group` 逐条解析 |
| 成员前缀 | `group set/add` 写入前按「精确 → 唯一前缀」解析并存储全名（`code` → `code-review`） |
| 组引用 | `@common` 引用另一个分组；写入时要求已定义，读取时递归展开并检测环 |

组名与成员名的规则：

- 小写字母、数字与单个连字符；
- 首尾必须是字母或数字；
- 最多 64 个字符；
- 不能是 Windows 保留设备名（`con`、`prn`、`aux`、`nul`、`com1`–`com9`、`lpt1`–`lpt9`）。

组名输入大写时会提示对应的全小写写法（如 `Predevelop` → `predevelop`），不会自动改写。`group remove`、`group delete` 与 `pull --group` 的组名支持唯一前缀（`predev` → `predevelop`）；`group set/add` 的目标组名必须精确匹配。

`@ref` 的展开是深度优先、按首次出现去重；`group list --resolved` 和 `pull --group` 会做缺失引用与环检测。示例：

```toml
common   = ["tdd", "grill"]
frontend = ["@common", "docs-writer"]
```

`frontend` 展开后是 `tdd, grill, docs-writer`。

`groups.toml` 是**惰性读取**的：只有 `group list/set/add/remove/delete` 与 `pull --group` 会读取。`list`、`search`、`add`、`remove`、`push`、`config`、`upgrade` 不读它，因此该文件写坏不会影响这些命令。

## 从旧版迁移

0.8.0 及更早的版本把分组存在 `config.toml` 的 `[groups]` 表里。0.9.0 起：

- `groups.toml` 不存在时，读取会回退到 `config.toml` 的 `[groups]`，所以升级后 `group list` 与 `pull --group` 仍能读到旧分组，不会自动写盘；
- 第一次执行 `group set` / `group add` / `group remove` / `group delete` 时，会把当前生效的分组写进 `groups.toml`，并尝试删除 `config.toml` 里的 `[groups]`；
- 旧表删除失败只打一条 warning（退出码仍为 0），之后该表被忽略；
- 迁移只搬成员值，`[groups]` 里手写的注释会丢失一次；
- 迁移完成后再手改 `config.toml` 的 `[groups]` 不会生效，请改 `groups.toml`。

## 写入

- `config set lang` 只读改写 `config.toml`；
- `group set`、`group add`、`group remove`、`group delete` 只读改写 `groups.toml`；
- 注释与未知键原样保留；
- 写入使用同目录临时文件加 rename；失败时清理临时文件，旧文件保持原样；
- 文件语法非法时不覆盖，必须先修复或删除；
- `groups.toml` 删到没有分组时保留一个带文件头的空文件。

## 错误

| 错误 | 触发条件 | 退出码 |
|---|---|---|
| `cannot parse config file` | `config.toml` TOML 语法非法 | 1 |
| `invalid language` | `lang` 值不在合法集合内 | 1 |
| `cannot parse groups file` | `groups.toml` TOML 语法非法 | 1 |
| `invalid groups file ...` | 组的值不是数组、成员不是字符串、组名/成员名非法 | 1 |
| `cannot determine the config directory` | 无 `LSM_CONFIG_DIR` 且平台配置目录不可用 | 1 |

这些错误的 hint 都是 `fix or delete <path>`。修复步骤见[配置或分组损坏时怎么办](../how-to/fix-config-problems.md)。

## 示例

最小配置：

```toml
# config.toml
lang = "zh"
```

```toml
# groups.toml
common = ["conventional-commit"]
```

保存后可以用裸值方式查语言：

```bash
lsm config get lang
```

完整实现规则（加载时机、迁移、原子写入、消息键）见 [`../impl/config.md`](../impl/config.md)。
