# Local Skill Manager（`lsm`）

把 agent skill 安装到 `~/.agents/skills` 或 `<项目>/.agents/skills`，把全局 skill 复制到当前项目，也能更新 `lsm` 自身二进制。

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

# 3. 复制到当前项目
lsm copy my-skill
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
lsm copy <name> [--force]
lsm upgrade [--check] [--force]
lsm config get lang
lsm config set lang <en|zh>
lsm --lang <en|zh> <command> ...
```

缩减版只支持分离写法（不支持 `-lg` 这种合并写法）：

| 缩减版 | 等价于 |
|---|---|
| `lsm -a foo.md -g` | `lsm add foo.md global` |
| `lsm -a foo.md -p` | `lsm add foo.md project` |
| `lsm -l -g` | `lsm list global` |
| `lsm -l -p --json` | `lsm list project --json` |
| `lsm -c foo` | `lsm copy foo` |

`-g` 与 `-p` 不能同时使用、不能与 `copy` 一起用，也不能与完整子命令写法混用。`upgrade` 没有短选项。

### add

- 接受 `.md` 文件或 skill 目录。目录源**不做任何排除**：隐藏文件、`scripts/`、`assets/`、`.git` 全部复制。
- 目录源包含 `.git` 时照常完整复制，并向 stderr 打印 `警告: 源目录包含 .git 目录，将被完整复制`。
- 安装是原子的：内容先构建到 `.tmp-<name>-<pid>`，带 `--force` 时旧目录移到 `.bak-<name>-<pid>`，再整体换位；任一步失败都会回滚且不残留。
- 目标位于源目录内部（例如 `lsm add . project`）时直接报错，不会递归复制。

### list

```
名称        描述
----------  ------------------------------------------
code-review 审查 diff 的正确性与风格...
my-skill    一句话说明。
broken      [invalid] missing SKILL.md
```

- description 会压成单行并截断到 60 个字符，超出补 `...`；
- 非法 skill 目录标记 `[invalid]`，不会中断整体输出；
- `--json` 输出单个 JSON 数组（2 空格缩进，空目录为 `[]`），含 `name`、`description`、`path`、`valid`，非法项额外带 `error`；warning 永远不会写进 stdout。

### copy

- 源必须是合法的全局 skill：存在 `SKILL.md` 且其 `name` 与目录名一致。
- 源与目标相同、或目标位于源内部时拒绝；目标已存在需要 `--force`。
- `name` 在拼任何路径之前完成校验，`..`、`/`、`\` 与保留设备名都以退出码 2 失败。

### upgrade

- 从 GitHub Releases 取最新版，**原地替换**正在运行的二进制。升级后 exe 的路径与文件名保持不变——不会出现 `lsm-v0.2.0.exe`，也不会长期留一个改名后的旧文件。
- `--check` 只报告不安装：不下载、不写盘；发现新版本退出 0，查询失败退出 1。
- `--force` 重装当前版本，或安装更旧的 release（回滚）。
- 替换前会跑一次 `--version` 冒烟校验；校验失败时原二进制不动、仍可执行。
- 二进制位于 `~/.cargo/bin` 时会给一条 `cargo install --force local-skill-manager` 的 warning，但升级照常进行。
- 需要能访问 `api.github.com`；可用 `LSM_UPGRADE_API` 指向返回同样 release JSON 的镜像。不读 `GITHUB_TOKEN`。
- 升级只动可执行文件，不触碰 `.agents/skills`。

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
| 0 | 成功 | 空目录执行 `list`、`upgrade --check`、显式 `--help` |
| 1 | 运行时错误 | 源不存在、frontmatter 非法、目标已存在、配置损坏、`upgrade` 失败 |
| 2 | 用法错误 | 缺参数、`-g` 与 `-p` 同用、非法 `--lang`、非法 skill 名称、未知配置键、未知 `upgrade` 参数、无子命令 |

`--help` / `--version` 输出到 stdout 并退出 0；无子命令时 help 输出到 **stderr** 并退出 2。

## 开发

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## 许可证

[MIT](LICENSE)
