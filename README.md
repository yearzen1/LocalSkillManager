# Local Skill Manager (`lsm`)

Install agent skills into `~/.agents/skills` or `<project>/.agents/skills`, copy a global skill into the current project, and update the `lsm` binary itself.

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

# 3. copy it into the current project
lsm copy my-skill
```

## Directory conventions

| Scope | Location | Override |
|---|---|---|
| global | `~/.agents/skills/<name>/` | `LSM_HOME` points at the `.agents` level |
| project | `<cwd>/.agents/skills/<name>/` | none |

`project` is fixed to the **current working directory** and never walks up to find a project root. Run the command at your project root; running it in a subdirectory installs into that subdirectory.

Only `.agents/skills` is maintained — other agents' private directories (e.g. `.claude/skills`) are not touched.

## SKILL.md format

```markdown
---
name: my-skill
description: Reviews a diff for correctness and style.
---

# My Skill

(body)
```

Validation rules:

- the file must start with a `---` frontmatter block;
- `name` and `description` are required and must not be blank;
- `name` matches `^[a-z0-9]+(-[a-z0-9]+)*$`, at most **64** characters;
- `description` is at most **1024** characters (counted in Unicode characters, not bytes);
- `name` must not be a Windows reserved device name (`con`, `prn`, `aux`, `nul`, `com1`–`com9`, `lpt1`–`lpt9`);
- when installing a directory, the directory name must equal the frontmatter `name`.

Limits are never truncated or auto-fixed: violating them is an error.

## Command reference

```
lsm add <source> <global|project> [--force]
lsm list <global|project> [--json]
lsm copy <name> [--force]
lsm config get lang
lsm config set lang <en|zh>
lsm --lang <en|zh> <command> ...
```

Commands, arguments and exit codes are defined once in [`docs/impl/cli.md`](docs/impl/cli.md) (in Chinese). This README only documents what the current binary supports.

Short forms (separate arguments only — `-lg` is not supported):

| Short | Expands to |
|---|---|
| `lsm -a foo.md -g` | `lsm add foo.md global` |
| `lsm -a foo.md -p` | `lsm add foo.md project` |
| `lsm -l -g` | `lsm list global` |
| `lsm -l -p --json` | `lsm list project --json` |
| `lsm -c foo` | `lsm copy foo` |

`-g`/`-p` may not be combined, may not be used with `copy`, and may not be mixed with the long subcommand form.

### add

- Accepts a `.md` file or a skill directory. Directory sources are copied **without any exclusions** — hidden files, `scripts/`, `assets/` and `.git` are all copied.
- A directory source containing `.git` is copied in full and prints `warning: source contains .git directory, it will be copied` on stderr.
- Installations are atomic: content is built in `.tmp-<name>-<pid>`, the previous directory (with `--force`) is moved to `.bak-<name>-<pid>`, then renamed into place. Failures roll back and leave nothing behind.
- If the destination is inside the source directory (e.g. `lsm add . project`), the command fails instead of copying recursively.

### list

```
NAME        DESCRIPTION
----------  ------------------------------------------
code-review Reviews a diff for correctness and style...
my-skill    One-line description.
broken      [invalid] missing SKILL.md
```

- descriptions are collapsed to one line and truncated to 60 characters with `...`;
- invalid skill directories are marked `[invalid]` and never abort the listing;
- `--json` prints a single JSON array (pretty-printed, `[]` when empty) with `name`, `description`, `path`, `valid`, and `error` for invalid entries. Warnings never go to stdout.

### copy

- The source must be a valid global skill: `SKILL.md` present and its `name` must match the directory.
- Copying onto the same path, or into the source itself, is refused; an existing target needs `--force`.
- `name` is validated before any path is built, so `..`, `/`, `\` and reserved device names exit with code 2.

## Language

Output, help and errors are available in English (`en`) and Chinese (`zh`).

Priority: `--lang` > `LSM_LANG` > config file > `en`.

```bash
lsm --lang zh --help
LSM_LANG=zh lsm list global
lsm config set lang zh   # persisted default
lsm config get lang      # prints a bare value: en / zh
```

The configuration file is independent of `LSM_HOME`:

| Platform | Default path |
|---|---|
| Windows | `%APPDATA%\lsm\config.toml` |
| Linux | `~/.config/lsm/config.toml` |
| macOS | `~/Library/Application Support/lsm/config.toml` |

`LSM_CONFIG_DIR` overrides the directory (empty string counts as unset). `config set` preserves comments and unknown keys in the file.

Reading is lazy: when `--lang` or `LSM_LANG` is valid, the file is not read at all, so a broken config cannot block you — that is the escape hatch, not a missing check. Without either, a broken config file exits 1 and tells you to `fix or delete <path>`.

## Output examples

```
$ lsm --lang en add my-skill.md global
installed my-skill -> /home/me/.agents/skills/my-skill

$ lsm --lang zh add my-skill.md global
已安装 my-skill -> /home/me/.agents/skills/my-skill

$ lsm --lang zh list global
错误: 不支持符号链接：/home/me/.agents/skills/bad/link
```

## Exit codes

| Code | Meaning | Examples |
|---|---|---|
| 0 | success | `list` on an empty directory, explicit `--help` |
| 1 | runtime error | missing source, invalid frontmatter, existing target, broken config |
| 2 | usage error | missing argument, `-g` with `-p`, invalid `--lang`, invalid skill name, unknown config key, no subcommand |

`--help` / `--version` print to stdout and exit 0. Running `lsm` with no subcommand prints help to **stderr** and exits 2.

## Development

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## License

[MIT](LICENSE)
