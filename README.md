# Local Skill Manager (`lsm`)

Install agent skills into `~/.agents/skills` or `<project>/.agents/skills`, pull a global skill into the current project, push a project skill back to the global directory, keep frequently used skills in groups, and update the `lsm` binary itself.

English | [中文](README.zh-CN.md)

## Installation

```bash
cargo install --path .
# or build a release binary
cargo build --release   # target/release/lsm
```

Supported platforms: Windows, Linux, macOS.

## Quick start

```bash
# 1. write a skill file
cat > my-skill.md <<'EOF'
---
name: my-skill
description: Reviews a diff for correctness and style.
---

# My Skill

(instructions for the agent)
EOF

# 2. install it globally
lsm add my-skill.md global

# 3. pull it into the current project
lsm pull my-skill
```

## Documentation

User documentation is written in Chinese and organised by Diátaxis. Pick the entry that matches what you need:

| I want to... | Read |
|---|---|
| Learn from scratch | [Getting started](docs/tutorials/getting-started.md) |
| Finish a specific task | [Manage skill groups](docs/how-to/manage-skill-groups.md) · [Fix config problems](docs/how-to/fix-config-problems.md) |
| Look up a command, setting or file format | [CLI index](docs/reference/cli.md) · [Config file](docs/reference/config.md) · [SKILL.md](docs/reference/skill-md.md) |
| Understand why it works this way | [Concepts](docs/explanation/concepts.md) |
| Contribute to lsm | [Implementation index](docs/impl.md) · [Decision records](docs/decisions.md) |

Full documentation index: [`docs/README.md`](docs/README.md).

Command and argument definitions live only in [`docs/impl/cli.md`](docs/impl/cli.md) §2.1; the reference pages link to it instead of defining a second copy.

## Directory conventions

| Scope | Location | Override |
|---|---|---|
| global | `~/.agents/skills/<name>/` | `LSM_HOME` points at the `.agents` level |
| project | `<cwd>/.agents/skills/<name>/` | none |

`project` is fixed to the **current working directory** and never walks up to find a project root. Run commands at your project root; running them in a subdirectory installs into that subdirectory. Only `.agents/skills` is maintained — other agents' private directories (e.g. `.claude/skills`) are not touched.

## Exit codes

`0` on success, `1` on runtime failures, `2` on usage errors. The full table is in [`docs/reference/cli.md`](docs/reference/cli.md).

## Development

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## License

[MIT](LICENSE)
