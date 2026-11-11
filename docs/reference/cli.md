# 命令索引

`lsm` 的命令面速查入口。这里只做定位：**命令、参数、默认值、互斥关系与退出码的唯一定义在 [`../impl/cli.md`](../impl/cli.md) §2.1 与 §3**。两者不一致时，以 `impl/cli.md` 和二进制 `--help` 为准。

所有命令都支持 `--lang <en|zh>`；每个子命令都可以用 `--help` 查看当前语言下的完整用法。

## 命令

| 命令 | 用途 | 查看用法 |
|---|---|---|
| `add` | 把 `.md` 文件或 skill 目录安装到 global / project | `lsm add --help` |
| `list` | 列出某个作用域已安装的 skill | `lsm list --help` |
| `search` | 按名称或描述搜索 skill | `lsm search --help` |
| `pull` | 把全局 skill 拉取到当前项目；支持多个 name、分组与 `--all` | `lsm pull --help` |
| `push` | 把当前项目 skill 推送到全局 | `lsm push --help` |
| `remove` | 从 global / project 卸载 skill | `lsm remove --help` |
| `group list` | 列出分组 | `lsm group list --help` |
| `group set` | 创建或整体替换分组 | `lsm group set --help` |
| `group add` | 向分组追加成员 | `lsm group add --help` |
| `group remove` | 从分组移除 skill 或 `@group` 引用 | `lsm group remove --help` |
| `group delete` | 删除一个或多个分组 | `lsm group delete --help` |
| `config get` | 输出已保存的配置值 | `lsm config get --help` |
| `config set` | 保存配置值 | `lsm config set --help` |
| `upgrade` | 更新 `lsm` 自身二进制 | `lsm upgrade --help` |

## 全局选项

| 选项 | 作用 | 定义位置 |
|---|---|---|
| `--lang <en\|zh>` | 选择本次输出的语言；可出现在任意位置 | [`impl/cli.md`](../impl/cli.md) §1 |
| `-h`, `--help` | 输出本地化 help 到 stdout，退出 0 | [`impl/cli.md`](../impl/cli.md) §2.4 |
| `-V`, `--version` | 输出 `lsm <版本>`，只对根命令生效 | [`impl/cli.md`](../impl/cli.md) §2.4 |

## 常用选项定位

只列「哪个命令有它」，完整语义看定义位置或该命令的 `--help`。

| 选项 | 出现在 | 定义位置 |
|---|---|---|
| `--force` | `add` / `pull` / `push` / `remove` / `group set` / `group delete` / `upgrade` | [`impl/cli.md`](../impl/cli.md) §2.1 选项表 |
| `--all` | `pull` / `push` | 同上 |
| `--group <GROUP>` | `pull`，可重复 | 同上 |
| `--json` | `list` / `search` / `group list` | 同上 |
| `--resolved` | `group list` | 同上 |
| `--name` / `--description` | `search` | 同上 |
| `--check` | `upgrade` | 同上 |
| `--verbose` / `--progress` | `upgrade` | 同上 |

`-a` / `-l` / `-r` / `-g` / `-p` 是历史短选项别名，由参数归一化处理，不在选项表内；新命令不新增短选项。

## 退出码

| 退出码 | 含义 | 典型场景 |
|---|---|---|
| `0` | 成功 | 空目录 `list`、显式 `--help`、只有跳过没有失败的批量 `pull` |
| `1` | 运行时错误 | 源不存在、frontmatter 非法、目标已存在、分组不存在、配置文件损坏、`upgrade` 网络/校验失败 |
| `2` | 用法错误 | 缺参数、非法 `--lang`、非法 skill/组名、互斥选项同给、未知子命令或无子命令 |

完整定义与示例见 [`impl/cli.md`](../impl/cli.md) §3。

## `--json` 输出约定

- 开启后 stdout 只放 JSON，warning 与错误仍走 stderr；
- `list` / `search` 输出 skill 数组，字段为 `name`、`description`、`path`、`valid`，非法项额外带 `error`；
- `group list` 输出 `{"name", "skills"}` 数组；
- 空结果为 `[]`，末尾带一个换行。

## 相关文档

- 想直接完成任务：[如何管理 skill 分组](../how-to/manage-skill-groups.md)
- 查配置字段：[配置文件参考](config.md)
- 查 SKILL.md 字段：[SKILL.md 参考](skill-md.md)
- 理解作用域与方向：[核心概念](../explanation/concepts.md)
