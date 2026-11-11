# 配置文件参考

`lsm` 只有一份全局配置 `config.toml`，保存输出语言与 skill 分组。配置与控制文件分离：`LSM_CONFIG_DIR` 只影响这份文件，`LSM_HOME` 只影响 skills 目录。

## 路径

| 平台 | 默认路径 |
|---|---|
| Windows | `%APPDATA%\lsm\config.toml` |
| Linux | `~/.config/lsm/config.toml` |
| macOS | `~/Library/Application Support/lsm/config.toml` |

- `LSM_CONFIG_DIR` 覆盖配置目录；空字符串按「未设置」处理。
- 文件名固定为 `config.toml`。
- 配置目录无法确定时，命令退出 1，hint 指向 `LSM_CONFIG_DIR`。
- 配置文件缺失不是错误：`lang` 使用默认值 `en`，分组视为空。

## 结构

```toml
# Output language: en | zh
lang = "en"

[groups]
frontend = ["tdd", "grill", "intent-lock"]
docs = ["docs-writer"]
```

### `lang`

| 项 | 值 |
|---|---|
| 类型 | string |
| 必填 | 否 |
| 默认 | `en` |
| 合法值 | `en`、`zh`；写入时接受并归一化 `en-US`、`zh-CN` |
| 非法值 | 运行时错误，退出 1 |

读取优先级：`--lang` > `LSM_LANG` > 配置文件 `lang` > `en`。`--lang` 或合法的 `LSM_LANG` 生效时，语言解析不会读取配置文件；但 `group` / `pull --group` 仍会为了分组读取它。

### `[groups]`

| 项 | 规则 |
|---|---|
| 类型 | TOML 表 |
| 键 | 分组名，必须符合 skill 名规则 |
| 值 | 字符串数组，每个元素是 skill 名或 `@组名` 引用 |
| 成员顺序 | 保留定义顺序 |
| 重复成员 | 读取与写入时按首次出现去重 |
| 空数组 | 允许存在；`pull --group` 会把它记为失败并提示组为空 |
| 缺失 | 等价于没有分组 |
| 成员是否必须已安装 | `group set/add` 写入时 skill 必须在 global 中已安装；手改配置可写入缺失成员，由 `pull --group` 逐条解析 |
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
[groups]
common   = ["tdd", "grill"]
frontend = ["@common", "docs-writer"]
```

`frontend` 展开后是 `tdd, grill, docs-writer`。

`[groups]` 是**惰性读取**的：只有 `group list/set/add/remove/delete` 与 `pull --group` 会读取。`list`、`search`、`add`、`remove`、`push`、`config`、`upgrade` 不读它，因此该表写坏不会影响这些命令。

## 写入

- `config set lang`、`group set`、`group add`、`group remove`、`group delete` 都会读改写同一份文件；
- 注释与未知键原样保留；
- 写入使用同目录临时文件加 rename；失败时清理临时文件，旧文件保持原样；
- 配置文件语法非法时不覆盖，必须先修复或删除；
- `[groups]` 删到空表时，整张表会被移除。

## 错误

| 错误 | 触发条件 | 退出码 |
|---|---|---|
| `cannot parse config file` | TOML 语法非法 | 1 |
| `invalid language` | `lang` 值不在合法集合内 | 1 |
| `invalid [groups] ...` | `groups` 不是表、组值不是数组、成员不是字符串、组名/成员名非法 | 1 |
| `cannot determine the config directory` | 无 `LSM_CONFIG_DIR` 且平台配置目录不可用 | 1 |

前三类错误的 hint 都是 `fix or delete <path>`。修复步骤见[配置或分组损坏时怎么办](../how-to/fix-config-problems.md)。

## 示例

最小配置：

```toml
lang = "zh"

[groups]
common = ["conventional-commit"]
```

保存后可以用裸值方式查语言：

```bash
lsm config get lang
```

完整实现规则（加载时机、原子写入、消息键）见 [`../impl/config.md`](../impl/config.md)。
