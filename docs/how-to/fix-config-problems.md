# 配置或分组损坏时怎么办

`lsm` 的配置只有一份全局 `config.toml`，里面保存输出语言 `lang` 和 skill 分组 `[groups]`。这篇指南处理配置读不出来、值非法、分组表写坏、配置目录无法确定这几类问题。

## 适用场景

- 命令报 `cannot parse config file`、`invalid language` 或 `invalid [groups]`；
- 手改 `config.toml` 后 `lsm group list` 失败，但其他命令似乎正常；
- 换机器或重装后提示无法确定配置目录。

不适用：

- 想了解配置字段的完整定义：看[配置文件参考](../reference/config.md)；
- 想学习创建与使用分组：看[如何管理 skill 分组](manage-skill-groups.md)。

## 前置条件

- 能打开终端并执行 `lsm`；
- 知道自己的配置目录，或能读到报错里的真实路径。

## 步骤

### 1. 找到配置文件

按平台确认默认路径：

| 平台 | 默认路径 |
|---|---|
| Windows | `%APPDATA%\lsm\config.toml` |
| Linux | `~/.config/lsm/config.toml` |
| macOS | `~/Library/Application Support/lsm/config.toml` |

如果设置了 `LSM_CONFIG_DIR`，实际路径是该目录下的 `config.toml`。报错信息里的 `<path>` 永远是最可靠的来源。

### 2. TOML 语法坏了

症状：命令退出 1，提示 `cannot parse config file: <path>`，hint 为 `fix or delete <path>`。

处理：

1. 打开该文件，按 TOML 语法修复（常见问题：引号没闭合、表头写错、数组少逗号）；
2. 不想保留时直接删除，下一次写配置会重新创建；
3. 修好后重新执行原命令。

注意：`--lang` 或 `LSM_LANG` 只能让**语言解析**跳过配置文件，不能绕过 `group` / `pull --group` 对文件的读取。TOML 语法错误时，这些命令仍会失败。

### 3. `lang` 值非法

症状：退出 1，提示 `invalid language 'xx' (allowed: en, zh)`。

处理：

```bash
lsm config set lang en
```

或把文件里的值改成 `en` / `zh` 后重试。配置里的 `lang` 只接受 `en`、`zh` 及 `en-US` / `zh-CN` 别名；写盘时统一成 `en` / `zh`。

### 4. `[groups]` 写坏

症状：`lsm group list` 或 `lsm pull --group` 退出 1，提示 `invalid [groups] in config file <path>: <detail>`。

常见 `<detail>` 与修法：

| detail 示例 | 含义 | 修法 |
|---|---|---|
| `groups must be a table of skill-name arrays` | `groups` 不是表 | 改成 `[groups]` 表头，或删除该键 |
| `group 'x' must be an array of skill names` | 组的值不是数组 | 写成 `x = ["tdd", "grill"]` |
| `group 'x' members must be strings` | 数组里有数字/布尔 | 成员全部改成带引号的字符串 |
| `invalid group name 'X'` | 组名不符合 skill 名规则 | 改成小写字母、数字和单个连字符 |
| `group 'x' has invalid skill name 'Y'` | 成员名非法 | 改成合法 skill 名 |

`[groups]` 是**惰性读取**的：只有 `group ...` 和 `pull --group` 会读它。`list`、`search`、`add`、`remove`、`push`、`config`、`upgrade` 不受影响。

### 5. 配置目录无法确定

症状：退出 1，提示 `cannot determine the config directory`，hint 为 `set LSM_CONFIG_DIR`。

处理：

```bash
export LSM_CONFIG_DIR=/path/to/writable/dir
```

Windows PowerShell：

```powershell
$env:LSM_CONFIG_DIR = "C:\path\to\writable\dir"
```

该目录只需要可写；`lsm` 会自动创建 `config.toml`。

### 6. 恢复到最小可用配置

如果不想逐项排查，可以把文件替换成下面这份最小配置，再按需补分组：

```toml
# Output language: en | zh
lang = "en"

[groups]
```

然后重新创建需要保留的分组：

```bash
lsm group set frontend tdd grill
```

## 验证

```bash
lsm config get lang
lsm group list
lsm list project
```

预期：三条命令都退出 0；`config get lang` 输出裸值 `en` 或 `zh`；`group list` 输出分组或 `No groups found.`。

## 常见问题

- **修好 `[groups]` 后 `pull --group` 仍失败**：配置文件本身没问题时，检查组名是否存在、组是否为空、成员是否已安装在 global。
- **`config set` 报错且文件没变**：`lsm` 不会覆盖解析失败的文件，必须先修复或删除；这是为了避免冲掉你手写的注释和未知键。
- **`--lang en` 能跑 `list` 但不能跑 `group list`**：语言逃生通道只跳过语言读取，分组命令仍要读同一份文件。
- **未知键会不会报错**：不会。`lsm` 保留并忽略未知键，`config set` / `group *` 写盘时会连同注释一起保留。
