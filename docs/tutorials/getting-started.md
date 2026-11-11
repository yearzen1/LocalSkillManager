# 入门教程：安装 lsm 并拉起第一个 skill 组

读完本教程，你会把两个 skill 安装到全局，定义一个分组，并在一个新项目里用一条命令把整组 skill 拉下来。所有操作都在临时目录里完成，不会碰你现有的 `~/.agents`。

## 你将完成

- 安装 `lsm` 并确认版本；
- 创建两个 `SKILL.md`，安装到隔离的全局目录；
- 定义一个名为 `review` 的分组；
- 在新项目里执行 `lsm pull --group review`，得到两个项目 skill。

预计耗时：10 分钟。

## 前置条件

- 已安装 Rust stable，`cargo --version` 有输出；
- 终端使用 bash（Windows 可用 Git Bash）；
- 当前目录是 lsm 仓库根目录。

先检查工具链：

```bash
cargo --version
```

预期输出形如 `cargo 1.9x.x (...)`。

## 步骤

### 1. 安装 lsm

```bash
cargo install --path .
```

预期：命令成功结束，没有编译错误。

确认二进制可用：

```bash
lsm --version
```

预期输出以 `lsm ` 开头，例如 `lsm 0.7.0`。

### 2. 准备隔离的练习目录

```bash
mkdir -p ~/lsm-tutorial/demo
export LSM_HOME="$HOME/lsm-tutorial/agents"
export LSM_CONFIG_DIR="$HOME/lsm-tutorial/config"
```

`LSM_HOME` 决定全局 skills 根目录，`LSM_CONFIG_DIR` 决定配置文件位置。把它们指向临时目录，后面的 `add`、`pull`、`group` 都不会污染日常环境。

创建两个 skill 文件：

```bash
cat > ~/lsm-tutorial/code-review.md <<'EOF'
---
name: code-review
description: 审查 diff 的正确性与风格。
---

# Code Review

(给 agent 的审查指令)
EOF

cat > ~/lsm-tutorial/docs-writer.md <<'EOF'
---
name: docs-writer
description: 按现有风格补写项目文档。
---

# Docs Writer

(给 agent 的写作指令)
EOF
```

### 3. 安装到全局

```bash
lsm add ~/lsm-tutorial/code-review.md global
lsm add ~/lsm-tutorial/docs-writer.md global
```

预期：两行 `installed ...`。

查看结果：

```bash
lsm list global
```

预期表格里有 `code-review` 与 `docs-writer`，描述就是上面写的两句话。

### 4. 定义分组

```bash
lsm group set review code-review docs-writer
```

预期：`group 'review' saved (2 skills) -> ...`。

查看分组：

```bash
lsm group list
```

预期：

```text
review: code-review, docs-writer
```

### 5. 在新项目里整组拉取

```bash
cd ~/lsm-tutorial/demo
lsm pull --group review
```

预期：

```text
pulled code-review -> ...
pulled docs-writer -> ...
pulled 2, skipped 0, failed 0
```

`--group` 会展开分组里的全部成员，再按普通 `pull` 的批量规则逐条复制。

### 6. 验证项目里的结果

```bash
lsm list project
```

预期表格里出现 `code-review` 与 `docs-writer`。目录结构应类似：

```text
~/lsm-tutorial/demo/.agents/skills/code-review/SKILL.md
~/lsm-tutorial/demo/.agents/skills/docs-writer/SKILL.md
```

## 你完成了什么

- 安装了 `lsm`；
- 用隔离的 `LSM_HOME` / `LSM_CONFIG_DIR` 建立了可丢弃的练习环境；
- 把两个 skill 安装到全局；
- 用 `group set` 保存了一个常用组合；
- 用 `pull --group` 一次性把整组拉进新项目。

练习结束后可以删除整个目录：

```bash
rm -rf ~/lsm-tutorial
```

## 下一步

- 想创建、追加、覆盖或删除分组：[如何管理 skill 分组](../how-to/manage-skill-groups.md)
- 想理解作用域、目录名身份与分组为什么放在全局配置：[核心概念](../explanation/concepts.md)
- 想查命令与参数：[命令索引](../reference/cli.md)
