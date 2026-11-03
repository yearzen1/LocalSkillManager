# v0.4.0 进度表（40 分钟/天）

> 上级：[实现方案索引](impl.md) ｜ 决策：[decisions.md](decisions.md) ｜ 需求：[Requirements.md](Requirements.md)
>
> 上一版：v0.3.0 已发布（tag `v0.3.0`，`upgrade` 自更新）。v0.3.0 的每日记录见 git 历史（`db5e868` 及之前）。
>
> 版本目标：v0.4.0 —— 多 name 的 `copy`/`remove`、`search`、`push`（`TODO.md` 候选池，Q43–Q47）
> 节奏：10 个工作日，每天 40 分钟，一个可验收切片
> 起始日：2026-11-16（周一）｜ 收尾日：2026-11-27（周五）

## 1. 怎么用

- 每天 40 分钟拆成：**5 分钟回顾**昨天结论 → **30 分钟执行**当天切片 → **5 分钟记录**到第 4 节。
- 完成标准只认**可执行命令**或可观察结果，不认"感觉做完了"。
- 卡住超过 10 分钟就停：把卡点写进当天「卡住时」栏，第二天第一件事处理，不要在同一处硬耗。
- 涉及代码的切片按 [tdd](../AGENTS.md) 走"红 → 绿 → 重构"，先写失败用例再改实现。
- 当天没做完不跳天：把剩余步骤顺延到次日，把次日任务里可延后的部分往后推。
- 每个切片完成即提交，提交信息已写在日卡里，不要攒到最后一起提交。
- **命令面先改文档**：`docs/impl/cli.md` §2.1 是唯一真源，新增命令先写 `planned`，实现完成后同一提交转 `implemented`；已实现命令上的参数扩展不预写 `planned`，与代码同提交。
- **不做范围外**：本版不做日志、不在 not-found 时自动执行 search、不推翻 Q39（不做 CI 矩阵与 Linux/macOS 实测）。

## 2. 总览

| 周 | 天 | 主题 | 周交付物 |
|---|---|---|---|
| 1 | D1–D5 | 命令面与单条实现 | 决策/文档、`search`、`push`、多 name `copy`、多 name `remove` |
| 2 | D6–D10 | 批量收敛、文档与发布 | 汇总与 hint、状态列、README/验收、四道门禁、v0.4.0 tag |

## 3. 每日任务

### 第 1 周

#### Day 1 · 2026-11-16（周一）· 决策与命令面

- **目标**：把 v0.4.0 的边界、批量语义、`search`/`push` 规则一次写清，后面不返工。
- **步骤**
  1. `docs/decisions.md` 追加 Q43–Q47：范围、多 name 批量语义、`search`、`push`、not-found hint
  2. `docs/impl/cli.md` §2.1 新增 `search` / `push` 两行 `planned`，选项表补 `search --json`、`push --force/--all`
  3. `docs/impl/storage.md` 补多 name 规则与「search」「push」章节；`impl.md` 同步模块树/数据流/范围表
  4. `docs/impl/quality.md` 追加 46–62 用例，手工验收补 `search` / `push`
- **完成标准**：`cargo test` 全绿（`search` / `push` 仍是未知子命令，`planned` 行被测试接受）
- **提交信息**：`docs: decide v0.4.0 commands`
- **卡住时**：批量语义拿不准就只写 Q40 的三分类——成功 / 跳过 / 失败，其余细节从简

#### Day 2 · 2026-11-17（周二）· `search` 命令层

- **目标**：`lsm search <query> <scope> [--json]` 能按 name/description 搜出结果，且复用 `list` 排版。
- **步骤**
  1. 红用例：命中 name、命中 description、大小写不敏感、非法项只按目录名匹配、无匹配、空 query
  2. `domain/action.rs` 加 `Search { query, scope, json }`；`cli/spec.rs` 加子命令与 `--json`
  3. 新建 `commands/search.rs`：`scan_skills` → 过滤 → `output::list::{render_text, render_json}`
- **完成标准**：`cargo test search` 全绿；`--json` 的 stdout 只有 JSON
- **提交信息**：`test(search): pin query matching`、`feat(search): find skills by name or description`
- **卡住时**：匹配先只做 `to_lowercase().contains()`，不要引入模糊匹配库

#### Day 3 · 2026-11-18（周三）· `push` 单条

- **目标**：`lsm push <name>` 能把项目 skill 推到 global，校验与 `copy` 对称。
- **步骤**
  1. 红用例：成功、源不存在、源非法、目标已存在、`--force` 覆盖、自包含/同路径
  2. `Action::Push { target, force }` + `cli/spec.rs` 子命令
  3. `commands/push.rs` 复用 `copy` 的源校验与 `install_skill_dir`；新增 i18n 文案
- **完成标准**：`cargo test push` 全绿；目标路径在 `LSM_HOME/skills/<name>`
- **提交信息**：`test(push): pin reverse copy`、`feat(push): copy project skills to global`
- **卡住时**：先复制 `copy.rs` 再改方向，不要在 D3 就抽公共函数

#### Day 4 · 2026-11-19（周四）· 多 name `copy`

- **目标**：`lsm copy a b c` 逐条复制，单条失败不中断。
- **步骤**
  1. 红用例：全成功、部分缺失、部分跳过、非法 name 全量预校验、重复 name 去重
  2. `CopyTarget::One` → `Many(Vec<SkillName>)`；`cli/spec.rs` 的 name 改为 `num_args(1..)`
  3. `commands/copy.rs` 逐条调用单条流程，累计成功/跳过/失败；多 name 才打印汇总
- **完成标准**：`cargo test copy` 全绿；`lsm copy a b` 后两个目录都在
- **提交信息**：`test(copy): pin multi-name batching`、`feat(copy): accept multiple skill names`
- **卡住时**：clap 的变长位置参数不熟就先写 `num_args(1..)` 最小用例，再补汇总

#### Day 5 · 2026-11-20（周五）· 多 name `remove` 与短选项

- **目标**：`lsm remove a b project` 与 `lsm -r a b -p` 都删对。
- **步骤**
  1. 红用例：多 name 全成功、部分缺失、非法 name 全量预校验、只有 name 缺 scope
  2. `Action::Remove` 的 name 改 `Vec<SkillName>`；scope 取最后一个位置参数
  3. `cli/normalize.rs` 对 `remove` 把 scope 放到 name 之后（`-r a b -p` → `remove a b project`）
- **完成标准**：`cargo test remove` 全绿；`lsm -r a b -p` 删除两个
- **提交信息**：`test(remove): pin multi-name batching`、`feat(remove): accept multiple skill names`
- **卡住时**：clap 的「变长 + 末位必填」排不顺就先用单个 `ARGS` 收全部，再在 `cli::parse` 拆最后一个为 scope

### 第 2 周

#### Day 6 · 2026-11-23（周一）· 批量收敛与 not-found hint

- **目标**：三个批量命令的输出/退出码一致，找不到时 hint 指向 `search`。
- **步骤**
  1. 统一成功/跳过/失败计数文案与退出码，抽 `commands/batch.rs` 或小函数
  2. `copy`/`remove` 的 not-found hint 改 `lsm search <name> <scope>`；`push` 改 `lsm list project`
  3. 重复 name、空列表、全失败等边界用例补齐
- **完成标准**：`cargo test` 全绿；not-found 时 stderr 含 `lsm search`
- **提交信息**：`refactor(commands): share batch summaries`、`feat(cli): point not-found hints at search`
- **卡住时**：公共函数抽不顺就先复制三份，等测试全绿再重构（本版允许一次重构提交）

#### Day 7 · 2026-11-24（周二）· CLI 状态列与 help

- **目标**：`search`/`push` 转 `implemented`，`copy`/`remove` 参数列同步，help 样例更新。
- **步骤**
  1. `docs/impl/cli.md` §2.1：`search`/`push` → `implemented`，`copy` → `<NAME>...`，`remove` → `<NAME>...`
  2. §1.3 help 样例补 `search`/`push`，版本改 `0.4.0`
  3. `tests/cli.rs` 落地 `quality.md` 46–62，`cli_reference_table_matches_binary` 转绿
- **完成标准**：`cargo test` 全绿；`lsm search --help`、`lsm push --help` 退 0
- **提交信息**：`feat(cli): complete v0.4.0 command surface`
- **卡住时**：状态列红了先看 `planned` 是否还留着，不要为了过测试把状态写反

#### Day 8 · 2026-11-25（周三）· README 与验收脚本

- **目标**：用户面文档只描述已实现行为，脚本覆盖离线部分。
- **步骤**
  1. 两份 README 命令参考补 `search`、`push`、多 name `copy`/`remove`，行为说明对齐
  2. `scripts/acceptance.sh` 加 `search`/`push`/多 name 步骤与 `--help`、非法参数
  3. `docs/impl/quality.md` 去掉残留的 `[planned]` 说明，保持用例编号连续
- **完成标准**：`"D:/Git/bin/bash.exe" scripts/acceptance.sh` 退出 0；两份 README 行数一致
- **提交信息**：`docs: document search and push`、`test: cover v0.4.0 commands in acceptance`
- **卡住时**：脚本步骤和文档不一致时以脚本为准，同一提交改文档

#### Day 9 · 2026-11-26（周四）· 全量回归与版本号

- **目标**：四道门禁全绿，`Cargo.toml` 与文档版本一致。
- **步骤**
  1. `cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test`、验收脚本
  2. `Cargo.toml` 改 `0.4.0`，`cargo build --release`
  3. 检查 `docs/impl.md` 验收清单、`impl/cli.md` help 样例、两份 README 的版本字样
- **完成标准**：四道门禁全绿；`lsm --version` 输出 `0.4.0`
- **提交信息**：`chore: bump version to 0.4.0`
- **卡住时**：clippy 新告警先看是否批量重构引入，必要时回退到上一提交再拆小

#### Day 10 · 2026-11-27（周五）· 发布 v0.4.0

- **目标**：tag 与 release 是唯一发布标记。
- **步骤**
  1. 手工验收：`search` 命中、`push` 到临时 `LSM_HOME`、多 name `copy`/`remove`、`upgrade --check`
  2. 产物改名为 `lsm-windows-x64.exe`，记录字节数
  3. `git tag -a v0.4.0`，推送 `main` 与 tag，按 [release-notes.md](release-notes.md) 发 release
- **完成标准**：release 资产可下载且字节数一致；`upgrade --check` 能查到 `v0.4.0`
- **提交信息**：`chore: release v0.4.0`
- **卡住时**：联网验收失败先判断限流还是资产缺失，限流等 10 分钟重试，不改实现绕过

## 4. 打卡表

| 天 | 日期 | 完成 | 实际用时 | 备注（卡点/顺延原因） |
|---|---|---|---|---|
| D1 | 2026-11-16 | ☑ | 30 min | Q43–Q47 入库；`cli.md` 新增 `search`/`push`，`storage.md` 补多 name/search/push；`quality.md` 46–62 |
| D2 | 2026-11-17 | ☑ | 35 min | `search` 复用 `list` 的文本/JSON 排版；空 query 退 2；无匹配退 0；非法项只按目录名匹配 |
| D3 | 2026-11-18 | ☑ | 40 min | `push` 单条与 `copy` 对称；`--all` 同语义；新增 push 文案与 `.git` 警告 |
| D4 | 2026-11-19 | ☑ | 35 min | `copy` 改 `num_args(1..)`；`CopyTarget::Many`；全量预校验、去重；单 name 输出不变 |
| D5 | 2026-11-20 | ☑ | 40 min | `remove` 变长 name + 末位 scope；`normalize` 把 scope 放末尾；`-r a b -p` 用例通过 |
| D6 | 2026-11-23 | ☑ | 35 min | 三命令统一 Q40 成功/跳过/失败语义；not-found hint 指向 `lsm search` / `lsm list project` |
| D7 | 2026-11-24 | ☑ | 40 min | `search`/`push` 转 `implemented`；`copy`/`remove` 参数列同步；help 样例更新为 0.4.0 |
| D8 | 2026-11-25 | ☑ | 35 min | 两份 README 同步 search/push/多 name；验收脚本扩到 13 步，`PASS` |
| D9 | 2026-11-26 | ☑ | 50 min | 四道门禁全绿（181 单测 + 90 CLI + 11 config + 5 upgrade）；`Cargo.toml` 0.4.0 |
| D10 | 2026-11-27 | ☑ | 25 min | 四道门禁全绿；tag `v0.4.0` 已推；release 资产 `lsm-windows-x64.exe` 字节数一致（3,722,752）；发布后副本 `upgrade --check` 报「已是最新版本」 |

## 5. 风险与应对

| 风险 | 触发信号 | 应对 |
|---|---|---|
| 批量命令退出码不一致 | 同一场景 `copy` 退 0、`remove` 退 1 | 三命令共用 Q40 的三分类与汇总函数，用例逐条对齐 |
| 变长位置参数把 scope 吃进 name | `remove a project` 报 name 非法 | scope 永远取最后一个位置参数；`-r` 归一化把 scope 放到末尾 |
| `search` 匹配范围膨胀 | 用户期望模糊/正则/多字段 | 本版固定 name + description 子串，其余进候选池 |
| `push` 覆盖全局 skill | 误传 `--force` 覆盖同名全局 skill | 默认拒绝，`--force` 才覆盖；测试覆盖目标已存在分支 |
| 文档与实现漂移 | `cli_reference_table_matches_binary` 红 | 状态列与子命令同一提交改；README 只写 `implemented` |

## 6. 明确不做

- 不做日志功能（文件日志 / 调试开关 / 审计）。
- 不在 `copy`/`remove`/`push` 找不到时自动执行 `search`，只改 hint（Q47）。
- 不做正则、模糊匹配、跨作用域一次搜索（Q45）。
- 不做 CI 矩阵、预编译产物、Linux/macOS 实测（Q39）。
- 不发布 `.sha256`，不建 `CHANGELOG.md`（沿用 v0.3.0）。

## 7. 候选池（下一版再排期）

| 候选 | 状态 | 前置 |
|---|---|---|
| 日志功能 | 待决策 | 范围未定（文件日志 / 调试开关 / 审计） |
| `search` 支持正则 / 模糊 / 多作用域 | 待决策 | 先收集真实使用反馈 |
| `remove --all` | 待决策 | 需定「删除全部」的二次确认与 `--force` 关系 |
| project → global 的 `push --all` 冲突策略细化 | 待观察 | 看是否有覆盖全局 skill 的真实需求 |
