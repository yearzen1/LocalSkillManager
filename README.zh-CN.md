# Local Skill Manager（`lsm`）

把 agent skill 安装到 `~/.agents/skills` 或 `<项目>/.agents/skills`，把全局 skill 拉取到当前项目、把项目 skill 推送到全局，也能更新 `lsm` 自身二进制。

[English](README.md) | 中文

## 安装

```bash
cargo install --path .
# 或者构建 release 二进制
cargo build --release   # target/release/lsm
```

支持平台：Windows、Linux、macOS。

## 快速开始

```bash
# 1. 写一个 skill 文件
cat > my-skill.md <<'EOF'
---
name: my-skill
description: 审查 diff 的正确性与风格。
---

# My Skill

（给 agent 的指令）
EOF

# 2. 安装到全局
lsm add my-skill.md global

# 3. 拉取到当前项目
lsm pull my-skill
```

## 目录约定

| 作用域 | 位置 | 覆盖方式 |
|---|---|---|
| global | `~/.agents/skills/<name>/` | `LSM_HOME` 指向 `.agents` 这一层 |
| project | `<cwd>/.agents/skills/<name>/` | 无 |

`project` 固定为**当前工作目录**，不会向上查找项目根目录。请在项目根目录执行；在子目录执行就会装到子目录里。

本工具只维护 `.agents/skills` 这一份目录，不代理其他 agent 的私有目录（如 `.claude/skills`）。

## SKILL.md 格式

```markdown
---
name: my-skill
description: 审查 diff 的正确性与风格。
---

# My Skill

（正文）
```

校验规则：

- 文件开头必须有 `---` 包裹的 frontmatter 块；
- `name` 与 `description` 必填且不能为空白；
- `name` 匹配 `^[a-z0-9]+(-[a-z0-9]+)*$`，最多 **64** 个字符；
- `description` 最多 **1024** 个字符（按 Unicode 字符计，不按字节）；
- `name` 不能是 Windows 保留设备名（`con`、`prn`、`aux`、`nul`、`com1`–`com9`、`lpt1`–`lpt9`）；
- 安装目录时，目录名必须与 frontmatter 的 `name` 一致。

超限一律报错，**不截断、不自动修复**。

## 命令参考

```
lsm add <source> <global|project> [--force]
lsm list <global|project> [--json]
lsm search <query> <global|project> [--json]
lsm pull <name>... [--force]
lsm pull --all [--force]
lsm push <name>... [--force]
lsm push --all [--force]
lsm remove <name>... <global|project> [--force]
lsm upgrade [--check] [--force]
lsm config get lang
lsm config set lang <en|zh>
lsm --lang <en|zh> <command> ...
```

命令、参数与退出码只在 [`docs/impl/cli.md`](docs/impl/cli.md) 定义一次；本 README 只记录当前二进制支持的能力。

缩减版只支持分离写法（不支持 `-lg` 这种合并写法）：

| 缩减版 | 等价于 |
|---|---|
| `lsm -a foo.md -g` | `lsm add foo.md global` |
| `lsm -a foo.md -p` | `lsm add foo.md project` |
| `lsm -l -g` | `lsm list global` |
| `lsm -l -p --json` | `lsm list project --json` |
| `lsm -r foo -p` | `lsm remove foo project` |
| `lsm -r a b -p` | `lsm remove a b project` |

`-g` 与 `-p` 不能同时使用、不能与 `pull` 一起用，也不能与完整子命令写法混用。`search` 与 `push` 没有短选项。`pull` 也没有短选项：`-p` 始终表示 `project`，`-c` 已在 0.5.0 移除。

> **从 0.4.x 迁移**：`lsm copy ...` 现在叫 `lsm pull ...`；`lsm -c ...` 已不可用。请使用 `lsm pull <name>...` 或 `lsm pull --all`。

### add

- 接受 `.md` 文件或 skill 目录。目录源**不做任何排除**：隐藏文件、`scripts/`、`assets/`、`.git` 全部复制。
- 目录源包含 `.git` 时照常完整复制，并向 stderr 打印 `警告: 源目录包含 .git 目录，将被完整复制`。
- 安装是原子的：内容先构建到 `.tmp-<name>-<pid>`，带 `--force` 时旧目录移到 `.bak-<name>-<pid>`，再整体换位；任一步失败都会回滚且不残留。
- 目标位于源目录内部（例如 `lsm add . project`）时直接报错，不会递归复制。

### list

```
名称         描述
-----------  ------------------------------------------
broken       [invalid] missing SKILL.md

code-review  审查 diff 的正确性与风格，标出合并前需要确认的风险点，并给出
             具体修改建议。

my-skill     一句话说明。
```

- 条目之间空一行，表头后与末尾不空，折行后的 description 不会和下一个条目粘在一起；
- description 会压成单行，按 60 个显示列换行（宽字符按 2 列计），续行悬挂缩进到描述列；最多 3 行，第 3 行末尾补 `...`；
- 非法 skill 目录标记 `[invalid]`，不会中断整体输出；
- `--json` 输出单个 JSON 数组（2 空格缩进，空目录为 `[]`），含 `name`、`description`、`path`、`valid`，非法项额外带 `error`；warning 永远不会写进 stdout。

### search

- `lsm search <query> <global|project>` 对 `name` 与 `description` 做大小写不敏感的子串匹配；scope 必填，一次只搜一个作用域。
- 输出复用 `list` 表格（非法项仍标 `[invalid]`）；`--json` 复用 `list` 的 JSON 数组字段。
- 无匹配不算失败：退出码 0，文本模式打印 `在 <scope> 中没有匹配 '<query>' 的 skill`（JSON 为 `[]`）。空查询退 2。

### pull

- 源必须是合法的全局 skill：存在 `SKILL.md` 且其 `name` 与目录名一致。
- 源与目标相同、或目标位于源内部时拒绝；目标已存在需要 `--force`。
- `name` 在拼任何路径之前完成校验，`..`、`/`、`\` 与保留设备名都以退出码 2 失败。多个 name 时先全部校验，任一个非法则一条都不拉取。
- 多个 name 时单条失败不中断：目标已存在且无 `--force` 算跳过，源缺失/非法算失败，结束打印 `成功 N 个，跳过 M 个，失败 K 个`。任一失败退 1，只有跳过退 0。
- `--all` 拉取全部合法全局 skill。目标已存在时默认跳过（`--force` 覆盖）；非法 skill 逐条报错但不中断。跳过不算失败：只有真的拉取失败才退出码 1。

### push

- `lsm push <name>...` 是 `pull` 的反向操作：project（当前目录）→ global。源必须是合法项目 skill，目录名与 `SKILL.md` 的 `name` 一致。
- 目标已存在需要 `--force`；`--all` 推送全部合法项目 skill，成功/跳过/失败语义与 `pull --all` 相同。
- 多个 name、先校验后执行、汇总行的行为与 `pull` 完全一致。
- 源不存在时 hint 指向 `lsm list project`；没有短选项。

### remove

- `lsm remove <name>... <global|project>` 卸载一个或多个 skill。无 `--force` 时只删合法 skill 目录（存在 `SKILL.md` 且其 `name` 与目录名一致），其余必须显式加 `--force`。
- 删除是原子的：先把目标 rename 到同级 `.tmp-remove-*`，再递归删除；rename 失败时原目录保持完整。
- 多个 name 时先全部校验，再逐条删除；单条失败不中断，结束打印 `成功 N 个，失败 M 个`。任一失败退 1。
- 目标不存在退出码 1，hint 指向 `lsm search`；`name` 在拼任何路径之前完成校验，`..` 与保留设备名以退出码 2 失败。

### upgrade

- `lsm upgrade` 取最新 GitHub release，与当前版本比较后**原地替换**二进制：路径与文件名都不变（`lsm.exe`，或你改名后的名字）。
- `--check` 只报告——`已是最新版本` 或 `有新版本可用`——不下载、不写盘。发现新版本不算失败：退出码 0。
- `--force` 允许同版本重装，也允许 release 比当前版本旧时降级。
- 下载后先跑 `<新文件> --version`，输出必须包含 release tag 的版本号才算通过；校验失败会删掉临时文件，原可执行文件不动。
- 可执行文件所在目录必须可写；替换失败会回滚，旧二进制仍然可以运行。
- 只有显式执行 `upgrade` 才联网。HTTP 403/429 按限流提示，`LSM_UPGRADE_API` 可指向返回同样 JSON 的镜像。

## 语言

输出、help 与错误支持英文（`en`）与中文（`zh`）。

优先级：`--lang` > `LSM_LANG` > 配置文件 > `en`。

```bash
lsm --lang zh --help
LSM_LANG=zh lsm list global
lsm config set lang zh   # 持久化默认语言
lsm config get lang      # 输出裸值：en / zh
```

配置文件与 `LSM_HOME` 相互独立：

| 平台 | 默认路径 |
|---|---|
| Windows | `%APPDATA%\lsm\config.toml` |
| Linux | `~/.config/lsm/config.toml` |
| macOS | `~/Library/Application Support/lsm/config.toml` |

`LSM_CONFIG_DIR` 可覆盖配置目录（空串按未设置处理）。`config set` 会保留文件里的注释与未知键。

读取是惰性的：`--lang` 或 `LSM_LANG` 合法时根本不读配置文件，所以坏配置不会被触发——这是有意的逃生通道，不是漏检。两者都没有时，坏配置会让命令以退出码 1 失败，并提示 `修复或删除 <path>`。

## 输出示例

```
$ lsm --lang en add my-skill.md global
installed my-skill -> /home/me/.agents/skills/my-skill

$ lsm --lang zh add my-skill.md global
已安装 my-skill -> /home/me/.agents/skills/my-skill

$ lsm --lang zh list global
错误: 不支持符号链接：/home/me/.agents/skills/bad/link
```

## 退出码

| 退出码 | 含义 | 例子 |
|---|---|---|
| 0 | 成功 | 空目录执行 `list`、显式 `--help` |
| 1 | 运行时错误 | 源不存在、frontmatter 非法、目标已存在、配置损坏 |
| 2 | 用法错误 | 缺参数、`-g` 与 `-p` 同用、非法 `--lang`、非法 skill 名称、未知配置键、无子命令 |

`--help` / `--version` 输出到 stdout 并退出 0；无子命令时 help 输出到 **stderr** 并退出 2。

## 开发

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## 许可证

[MIT](LICENSE)
