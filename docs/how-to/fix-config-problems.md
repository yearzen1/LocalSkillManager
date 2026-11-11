# 配置或分组损坏时怎么办

`lsm` 的持久化状态分成两份全局文件：`config.toml` 保存输出语言 `lang`，`groups.toml` 保存 skill 分组。这篇指南处理文件读不出来、值非法、分组文件写坏、配置目录无法确定这几类问题。

## 适用场景

- 命令报 `cannot parse config file`、`invalid language`、`cannot parse groups file` 或 `invalid groups file`；
- 手改 `groups.toml` 后 `lsm group list` 失败，但其他命令似乎正常；
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

| 文件 | Windows | Linux | macOS |
|---|---|---|---|
| `config.toml` | `%APPDATA%\lsm\config.toml` | `~/.config/lsm/config.toml` | `~/Library/Application Support/lsm/config.toml` |
| `groups.toml` | `%APPDATA%\lsm\groups.toml` | `~/.config/lsm/groups.toml` | `~/Library/Application Support/lsm/groups.toml` |

如果设置了 `LSM_CONFIG_DIR`，实际路径是该目录下的这两个文件。报错信息里的 `<path>` 永远是最可靠的来源。

### 2. TOML 语法坏了

症状：命令退出 1，提示 `cannot parse config file: <path>`，hint 为 `fix or delete <path>`。

处理：

1. 打开该文件，按 TOML 语法修复（常见问题：引号没闭合、表头写错、数组少逗号）；
2. 不想保留时直接删除，下一次写配置会重新创建；
3. 修好后重新执行原命令。

注意：`--lang` 或 `LSM_LANG` 只影响语言解析。`groups.toml` 已经存在时，`group` / `pull --group` 不再读 `config.toml`，配置语法坏也能用 `--lang en` 继续；`groups.toml` 还没生成（首次迁移前）时分组命令仍会回退读旧表，此时 `config.toml` 语法错会让它们失败。

### 3. `lang` 值非法

症状：退出 1，提示 `invalid language 'xx' (allowed: en, zh)`。

处理：

```bash
lsm config set lang en
```

或把文件里的值改成 `en` / `zh` 后重试。配置里的 `lang` 只接受 `en`、`zh` 及 `en-US` / `zh-CN` 别名；写盘时统一成 `en` / `zh`。

### 4. 分组文件写坏

症状：`lsm group list` 或 `lsm pull --group` 退出 1，提示 `cannot parse groups file: <path>` 或 `invalid groups file <path>: <detail>`。

`groups.toml` 是**顶层表**，不要写 `[groups]` 表头：

```toml
frontend = ["tdd", "grill"]
common = ["@frontend"]
```

常见 `<detail>` 与修法：

| detail 示例 | 含义 | 修法 |
|---|---|---|
| `group 'x' must be an array of skill names` | 组的值不是数组 | 写成 `x = ["tdd", "grill"]` |
| `group 'x' members must be strings` | 数组里有数字/布尔 | 成员全部改成带引号的字符串 |
| `invalid group name 'X'` | 组名不符合 skill 名规则 | 改成小写字母、数字和单个连字符 |
| `group 'x' has invalid member 'Y'` | 成员名非法 | 改成合法 skill 名 |

如果报错路径是 `config.toml` 里的 `[groups]`，说明还在迁移窗口（`groups.toml` 尚未生成）：修好旧表后执行一次 `group add` 就会迁移；也可以把旧表内容手抄进 `groups.toml` 并去掉 `[groups]` 表头。

`groups.toml` 是**惰性读取**的：只有 `group ...` 和 `pull --group` 会读它。`list`、`search`、`add`、`remove`、`push`、`config`、`upgrade` 不受影响。

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

该目录只需要可写；`lsm` 会自动创建 `config.toml` 与 `groups.toml`。

### 6. 恢复到最小可用配置

如果不想逐项排查，可以把 `config.toml` 替换成下面这份最小配置：

```toml
# Output language: en | zh
lang = "en"
```

分组文件可以直接删除，或者留一个空文件：

```toml
# Skill groups: name = [skill, "@group"]
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

- **修好 `groups.toml` 后 `pull --group` 仍失败**：文件本身没问题时，检查组名是否存在、组是否为空、成员是否已安装在 global。
- **`config set` 报错且文件没变**：`lsm` 不会覆盖解析失败的文件，必须先修复或删除；这是为了避免冲掉你手写的注释和未知键。
- **`--lang en` 能跑 `list` 但不能跑 `group list`**：只有 `groups.toml` 还没生成时才会这样——迁移窗口内分组命令仍要回退读 `config.toml`；随便执行一次 `group add` 完成迁移后就恢复了。
- **分组还在 `config.toml` 的 `[groups]` 里**：0.9.0 起 `groups.toml` 才是唯一写入位置。读取仍兼容旧表，第一次 `group set/add/remove/delete` 会自动迁移并清掉旧表；想手动迁移就把内容抄进 `groups.toml` 并去掉 `[groups]` 表头。
- **未知键会不会报错**：不会。`lsm` 保留并忽略未知键，`config set` / `group *` 写盘时会连同注释一起保留。
