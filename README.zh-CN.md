# Local Skill Manager（`lsm`）

把 agent skill 安装到 `~/.agents/skills` 或 `<项目>/.agents/skills`，把全局 skill 拉取到当前项目、把项目 skill 推送到全局，把常用 skill 放进分组，也能更新 `lsm` 自身二进制。

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

## 文档

用户文档按 Diátaxis 四象限组织，按你此刻的需求选入口：

| 我想…… | 读这篇 |
|---|---|
| 从零学会 | [入门教程](docs/tutorials/getting-started.md) |
| 完成一个具体任务 | [管理 skill 分组](docs/how-to/manage-skill-groups.md) · [修复配置问题](docs/how-to/fix-config-problems.md) |
| 查命令、配置或文件格式 | [命令索引](docs/reference/cli.md) · [配置文件](docs/reference/config.md) · [SKILL.md](docs/reference/skill-md.md) |
| 理解为什么这样设计 | [核心概念](docs/explanation/concepts.md) |
| 参与开发 | [实现方案索引](docs/impl.md) · [决策记录](docs/decisions.md) |

完整入口：[`docs/README.md`](docs/README.md)。

命令与参数的唯一定义仍在 [`docs/impl/cli.md`](docs/impl/cli.md) §2.1；参考页只做索引与链接，不复制第二份参数表。

## 目录约定

| 作用域 | 位置 | 覆盖方式 |
|---|---|---|
| global | `~/.agents/skills/<name>/` | `LSM_HOME` 指向 `.agents` 这一层 |
| project | `<cwd>/.agents/skills/<name>/` | 无 |

`project` 固定为**当前工作目录**，不会向上查找项目根目录。请在项目根目录执行；在子目录执行就会装到子目录里。本工具只维护 `.agents/skills` 这一份目录，不代理其他 agent 的私有目录（如 `.claude/skills`）。

## 退出码

成功 `0`，运行时失败 `1`，用法错误 `2`。完整表格见 [`docs/reference/cli.md`](docs/reference/cli.md)。

## 开发

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## 许可证

[MIT](LICENSE)
