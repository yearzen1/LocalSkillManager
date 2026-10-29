# 测试、README 与发布

> 上级：[实现方案索引](../impl.md) ｜ 相关：[存储与命令](storage.md) ｜ [CLI 与输出](cli.md) ｜ [配置与持久化](config.md) ｜ [决策记录](../decisions.md)
>
> 本文拥有以下规则的完整定义：测试计划、README 结构、许可证与发布流程。

## 1. 测试计划

### 1.1 单元测试

单测与模块同文件（`#[cfg(test)]`）；涉及磁盘的 `infra` 用例使用 `tempfile`。

- `domain::lang`：`parse` 接受 `en`/`zh`/`en-US`/`zh-CN`，拒绝 `fr`、空串、大小写变体；拒绝时返回 `LangError`。
- `domain::name`：合法（`a`、`a1`、`my-skill`）、非法（`My-Skill`、`-a`、`a--b`、`a_`、空串）、64/65 字符边界、Windows 保留名（`con`、`nul`、`com1`、`lpt9`）。
- `domain::meta`：description 空值、1024/1025 字符边界。
- `domain::version`：接受 `v0.1.1`/`0.1.1`，拒绝 `v0.2`、`v0.2.0-rc.1`、`latest`、空串；三段数值比较；前导零按数值处理。
- `domain::release`：资产选择命中/缺失/重复；压缩包不匹配；平台映射覆盖表内组合，未知 OS/arch 报错。
- `infra::frontmatter`：正常、缺 `---`、缺 `name`、缺 `description`、非法 YAML、BOM 开头、CRLF 行尾、额外字段保留。
- `infra::env`：`Env::read` 正确拾取 `LSM_HOME`/`LSM_CONFIG_DIR`/`LSM_LANG`/`LSM_UPGRADE_API`，空串按未设置；读取是快照，后续设置环境变量不影响已读值；`exe` 来自 `current_exe()`。
- `infra::paths`：纯函数测试，构造 `Env` 字面量即可，**不修改进程环境**；`LSM_HOME` 覆盖生效、未设置时落到家目录、project 始终为 cwd、`LSM_CONFIG_DIR` 覆盖配置目录、缺省落到 `dirs::config_dir()/lsm`。
- `infra::net`：本地桩覆盖 200、302、404、403、非 JSON、超时；请求头含 `User-Agent: lsm/<version>`；`LSM_UPGRADE_API` 空串回落到默认端点。
- `cli::lang`：`prescan` 支持 `--lang zh`/`--lang=zh`/多处取最后一个；`decide` 三态决策表（`Cli` 优先 → `Env` 次之 → `NeedFile`），非法值返回 `LangError`，前两档命中时永不返回 `NeedFile`。
- `infra::config`：读写往返；注释与未知键保留；缺文件 → `None`；坏 TOML → `Parse`；`lang = "fr"` → `InvalidValue`；同值写入仍成功；写入失败不破坏旧文件。
- `cli::normalize`：短选项映射、缺作用域、互斥、混用形式；`config` 子命令不被归一化改写。
- `error::mapping`：每个窄错误枚举都能转成 `AppError`，`kind` 正确且带预期 `MsgKey`/hint。
- `output::error`：`error:`/`hint:` 与 `错误:`/`提示:` 两套前缀、hint 省略时不输出空行、`--json` 场景由调用方控制 stdout。
- `infra::fsops`：`contains_path` 对 `..`、符号链接、目标不存在等场景的判断；`replace_self` 原地替换成功、权限位保留、备份清理、回滚时旧文件仍可执行。

### 1.2 集成测试（`tests/cli.rs`、`tests/config.rs`）

每个用例使用独立 `tempfile::TempDir`，设置 `LSM_HOME`、`LSM_CONFIG_DIR`、`current_dir`，互不干扰。命令功能用 `tests/cli.rs`，配置持久化用 `tests/config.rs`。

1. `add foo.md global`：生成 `LSM_HOME/skills/foo/SKILL.md`，内容字节一致。
2. `add foo.md project`：生成 `<tmp>/.agents/skills/foo/SKILL.md`。
3. `add dir global`：`scripts/`、`assets/`、隐藏文件全部完整复制。
4. 目录源含 `.git`：仍完整复制，stderr 出现 warning。
5. 无 frontmatter → 退出码 1，stderr 含 `frontmatter`。
6. `name` 非法 / 超 64 字符 / description 超 1024 字符 → 退出码 1。
7. 重复 add → 退出码 1；`--force` → 成功且旧内容被替换。
8. `lsm add . project` → 退出码 1，提示目标在源内。
9. `list global` 文本输出包含已装 skill；空目录提示 `No skills found`。
10. `list global --json` 空目录输出合法 `[]`。
11. `list global --json` 可被解析，字段齐全；非法项含 `error`。
12. 非法 skill 目录在文本输出标记 `[invalid]`。
13. `copy foo`：global → project 成功。
14. `copy foo` 目标已存在 → 失败；`--force` → 成功。
15. `copy missing` → 失败，提示 `lsm list global`。
16. 短选项：`-a ... -g`、`-a ... -p`、`-l -g`、`-l -p`、`-c foo` 全部通过。
17. 非法组合：`-g -p`、`-c foo -g`、`lsm add foo.md -g` → 退出码 2。
18. 默认 help 为英文（含 `Usage:`），`--lang zh --help` 为中文（含 `用法:`）。
19. `LSM_LANG=zh` 时错误消息为中文；命令行 `--lang en` 覆盖环境变量。
20. `--json` 输出中不混入 warning / 提示文本。
21. 安装失败后目标目录不存在残留（构造只读/冲突场景）。
22. `lsm` 无参数 → 本地化 help 到 stderr、退出码 2；`lsm --help` → stdout、退出码 0。
23. `copy ../evil`、`copy .`、`copy a/b` → 退出码 2，且磁盘上不产生任何新目录。
24. 目录名与 frontmatter `name` 不一致 → `list` 标 `[invalid]`，`copy` 该目录报错。
25. 保留设备名：`add`（frontmatter `name: con`）→ 退出码 1；`copy con` → 退出码 2。
26. 源目录含符号链接 → 退出码 1，错误信息含 `symbolic links` 与具体路径。
27. `--lang zh` 下 warning 与错误前缀分别为 `警告:` / `错误:`。
28. `config set lang zh` 后另起进程执行 `list global` → 默认中文输出（无需 `--lang`）。
29. `config set lang zh` 后 `lsm --lang en list global` → 英文，文件值未变。
30. `config set lang zh` + `LSM_LANG=en` → 英文；同时把 `config.toml` 写成坏 TOML，仍退出 0（惰性读取不报错）。
31. 裸调用（无 `--lang`、无 `LSM_LANG`）+ 坏 TOML → 退出码 1，stderr 含配置文件路径与 `fix or delete`；`--json` 时 stdout 为空。
32. 配置文件不存在 → 正常执行，语言为 `en`，退出码 0。
33. `config get lang` 无配置 → 输出 `en`；`set` 后 → 输出 `zh`；非法/损坏文件 → 退出码 1，stdout 为空。
34. 手写注释与未知键（如 `theme = "dark"`）+ `config set lang zh` → 注释与未知键仍在，`lang` 被更新。
35. `config set lang zh-CN` → 文件内落盘为 `zh`；`config set lang fr` → 退出码 2。
36. `lsm config`（无子命令）→ help 到 stderr、退出码 2；`lsm config --help` → stdout、退出码 0。
37. [planned] `upgrade --check` 已最新 → 退出 0，stdout 含 `up to date`，磁盘无任何新文件。
38. [planned] 桩返回更高 tag → `upgrade --check` 打印 `new version available`，退出 0，不下载、不写盘。
39. [planned] `upgrade --force`（桩 tag = 当前版本）→ 二进制被替换，**路径与文件名不变**，内容等于下载载荷，`.old-*` 已清理。
40. [planned] 当前平台资产缺失 → 退出 1，stderr 含期望资产名与 tag。
41. [planned] 桩返回 404 / 非 JSON / 403 → 退出 1，hint 指向镜像或手动下载。
42. [planned] exe 所在目录不可写 → 退出 1，原文件字节不变。
43. [planned] `upgrade --help` 两种语言均输出到 stdout 且退出 0；`lsm -u`、`upgrade --bogus` → 退出 2。
44. [planned] 升级成功后再次执行 `upgrade --check`（桩 tag 不变）→ 退出 0 且报告已最新。
45. 真源一致性：`docs/impl/cli.md` §2.1 表的 `implemented` 行必须 `lsm <command> --help` 退出 0，`planned` 行必须被视为未知子命令（退出 2）——由 `tests/cli.rs::cli_reference_table_matches_binary` 强制。

`[planned]` 条目不计入已实现用例数；实现后删掉标记，并同步 `docs/impl/cli.md` §2.1 的状态列。

### 1.3 手工验收（三平台）

```bash
cargo test
cargo build --release
./target/release/lsm add docs/sample.md global
./target/release/lsm list global
./target/release/lsm --lang zh --help
lsm config set lang zh
lsm config get lang
lsm list global          # 应为中文
cd <demo-project> && lsm copy sample && lsm list project

# [planned] upgrade 冒烟（`upgrade` 未实现前跳过）：复制一份再执行，确认替换的不是正在使用的二进制
# cp ./target/release/lsm /tmp/lsm-check
# /tmp/lsm-check upgrade --check          # 只查不装：打印已最新，或新版本提示（含当前版本）
# rm -f /tmp/lsm-check
```

`[planned]`：`upgrade` 实现前跳过这一段；实现后它是这里唯一的联网步骤，不要对日常使用的 `lsm` 直接跑 `upgrade` 做验收。Windows 上用 `Copy-Item` 复制到临时目录后执行同样的 `--check`。

## 2. README 与许可证

### 2.1 双文件约定

- `README.md`：英文，主文件，顶部放指向 `README.zh-CN.md` 的中文切换链接。
- `README.zh-CN.md`：中文，顶部放指向 `README.md` 的英文切换链接。
- 两份内容结构一致，命令示例、错误示例都覆盖两种语言输出。
- 两份 README 只描述 `implemented` 命令；`planned` 命令只出现在 [`cli.md`](cli.md) §2.1 的状态列里。

### 2.2 内容大纲

`README.md`（英文，主文件）与 `README.zh-CN.md`（中文）结构一致：

1. 一句话介绍 + 语言切换链接。
2. 安装：`cargo install --path .`、本地 release 构建。
3. 快速开始：写一个 `my-skill.md`，`lsm add my-skill.md global`，`lsm copy my-skill`。
4. 目录约定：`~/.agents/skills`、`<project>/.agents/skills`；提示"在项目根目录执行"。
5. SKILL.md 模板与字段校验规则（64 / 1024 上限）。
6. 命令参考：完整版 + 缩减版对照表 + `--lang` + `config get/set`，并注明命令与参数的唯一真源是 [`cli.md`](cli.md) §2.1。
7. 语言持久化：`config set lang zh`、`config get lang`、配置文件路径（三平台）、四级优先级、`LSM_CONFIG_DIR`、坏配置会报错。
8. 双语输出示例。
9. 退出码与错误处理。
10. 开发：`cargo test`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`。
11. License：MIT，链接 `LICENSE`。

### 2.3 许可证

`LICENSE`（MIT）与 `Cargo.toml` 的 `license = "MIT"` 随仓库提交；README 两种语言都标注 MIT。

## 3. 发布

- v1：本地 `cargo build --release` + `cargo install --path .`，不配置 CI。
- Release 资产命名、tag 约定、上传与改名流程、CI 矩阵：见 [发布规则](../release.md)。

## 4. 相关决策

- Q6 / Q13：v1 只做本地构建，CI 后置。
- Q8 / Q15：TDD 与原子提交。
- Q23：MIT 许可证。
- Q27 / Q28：语言持久化与配置失效策略（配置用例集中在 `tests/config.rs`）。
- Q29：分层收敛与错误映射（`domain`/`infra` 自身错误枚举，`output` 独占排版，`Ctx` 传参）。
- Q30 / Q31 / Q32：`upgrade` 的范围、校验与替换、网络与资产契约（测试用本地桩，不连真实 GitHub）。

完整决策见 [decisions.md](../decisions.md)。
