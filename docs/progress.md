# v0.5.0 进度表（40 分钟/天）

> 上级：[实现方案索引](impl.md) ｜ 决策：[decisions.md](decisions.md) ｜ 需求：[Requirements.md](Requirements.md)
>
> 上一版：v0.4.0 已发布（tag `v0.4.0`，多 name `copy`/`remove`、`search`、`push`）。v0.4.0 的每日记录见 git 历史（`e4d8cad` 及之前）。
>
> 版本目标：v0.5.0 —— `copy` 改名为 `pull`，移除 `copy` 与 `-c`（Q48，breaking change）
> 节奏：5 个切片，每个切片保持可编译、测试绿
> 起始日：2026-11-30（周一）｜ 收尾日：2026-12-04（周五）

## 1. 范围与硬约束

- `pull` 成为唯一命令：`lsm pull <name>... [--force]`、`lsm pull --all [--force]`。
- **不保留 `copy` 别名**；`copy` / `--copy` 按未知子命令 / 未知参数退 2。
- **移除 `-c`**；`-p` 始终表示 `project`，`pull` 没有短选项。
- 不做 “`copy` 已改名” 的特殊拦截，错误就是普通的未知子命令。
- 复制逻辑、路径校验、批量语义不变，只改命令面与文案。
- 不推翻 Q39（仍不做 CI 矩阵与 Linux/macOS 实测）。

## 2. 切片

### S1 · 决策与真源

- **目标**：把 breaking change 写进决策和命令真源。
- **步骤**
  1. `docs/decisions.md` 追加 Q48
  2. `docs/Requirements.md` 命令表 `copy` → `pull`，缩减版删除 `-c`
  3. `docs/impl/cli.md` 命令表/选项表/短选项规则 `copy` → `pull`，删除 `-c`
  4. `docs/impl/storage.md` 章节与正文、`impl.md` 模块树/数据流/范围表同步
  5. `docs/impl/quality.md` 用例改名，补 63–64（`copy` / `-c` 退 2）
- **完成标准**：`cargo test` 全绿
- **提交信息**：`docs: decide copy to pull rename`

### S2 · 领域与 CLI 改名

- **目标**：代码里不再有 `Action::Copy`、`CopyTarget`、`commands::copy`。
- **步骤**
  1. `domain::action`：`Copy` → `Pull`，`CopyTarget` → `TransferTarget`
  2. `cli::spec`：`copy_command` → `pull_command`，子命令名 `pull`
  3. `cli::mod`：match 分支与 `transfer_target`，usage key 改名
  4. `cli::normalize`：删除 `-c/--copy` 分支与 `CopyWithScope` 特判
  5. i18n：`Copy*` → `Pull*`，文案 `copied` → `pulled`、`复制` → `拉取`
  6. `git mv commands/copy.rs commands/pull.rs`，函数与 dispatch 改名
  7. `push` 改用 `TransferTarget`
- **完成标准**：`cargo build` 通过；`lsm pull --help` 退 0；`lsm copy --help`、`lsm -c foo` 退 2
- **提交信息**：`refactor(cli): rename copy command to pull`

### S3 · 测试回归

- **目标**：旧 `copy` 用例全部覆盖新命令，并锁死移除行为。
- **步骤**
  1. 24 个 `copy_*` 测试改名 `pull_*`，参数同步
  2. `-c` 两条成功用例改为 `copy` / `-c` 退 2 用例
  3. 新增 `copy_command_is_removed`、`pull_rejects_scope_flag`
  4. 多 name、`--all`、not-found、i18n 断言同步为 `pulled` / `拉取`
- **完成标准**：`cargo test` 全绿
- **提交信息**：`test(cli): pin pull rename and copy removal`

### S4 · README 与验收脚本

- **目标**：用户面只描述 `pull`，并为旧用户提供迁移说明。
- **步骤**
  1. 两份 README 命令参考、短选项表、`pull` 小节、迁移提示
  2. `scripts/acceptance.sh` 全部改为 `pull`，新增第 14 步验证 `copy` / `-c` 退 2
  3. `docs/impl/quality.md` 手工验收同步
- **完成标准**：`"D:/Git/bin/bash.exe" scripts/acceptance.sh` 退出 0
- **提交信息**：`docs: document pull`、`test: cover pull in acceptance`

### S5 · 门禁与发布

- **目标**：v0.5.0 收口，release notes 置顶迁移说明。
- **步骤**
  1. `Cargo.toml` 改 `0.5.0`，文档版本字样同步
  2. 四道门禁：`cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test`、验收脚本
  3. `cargo build --release`，确认 `lsm --version` 输出 `0.5.0`
  4. release notes 顶部写：`copy` 已改名 `pull`，`copy` / `-c` 不再可用
  5. `git tag -a v0.5.0`，推送 `main` 与 tag，上传 `lsm-windows-x64.exe`
- **完成标准**：四道门禁全绿；release 资产字节数一致；副本 `upgrade --check` 报告已最新
- **提交信息**：`chore: bump version to 0.5.0`、`chore: release v0.5.0`

## 3. 打卡表

| 切片 | 日期 | 完成 | 备注 |
|---|---|---|---|
| S1 决策与真源 | 2026-11-30 | ☑ | Q48；Requirements/cli/storage/impl/quality 同步 |
| S2 代码改名 | 2026-12-01 | ☑ | `pull.rs`、`TransferTarget`；`-c` 分支删除 |
| S3 测试回归 | 2026-12-02 | ☑ | 179 单测 + 91 CLI + 11 config + 5 upgrade 全绿 |
| S4 README/验收 | 2026-12-03 | ☑ | 两份 README 同步；验收 14 步 PASS |
| S5 门禁与发布 | 2026-12-04 | ☑ | 四道门禁全绿；tag `v0.5.0` 已推；资产 3,724,288 字节一致；副本 `upgrade --check` 报「已是最新版本」 |

## 4. 风险与应对

| 风险 | 触发信号 | 应对 |
|---|---|---|
| 全局替换误伤 `#[derive(Copy)]` / `fs::copy` | 编译错误或复制行为异常 | 只改命令级标识符；保留 `copy_recursive` 与 std API |
| `normalize` 漏改作用域特判 | `lsm -g pull` 报错不清晰 | 所有短选项动作都要求 scope，`pull` 不参与归一化 |
| `copy` 移除后没有测试兜底 | 别名被误加回 | `copy_command_is_removed` + 验收脚本第 14 步 |
| README 残留 `lsm copy` | 用户照旧输入 | 迁移提示 + grep 审计 |
| i18n 只改 en | zh 文案过期 | 消息表覆盖测试会红 |

## 5. 明确不做

- 不保留 `copy` 别名、不保留 `-c`。
- 不做 “copy 已改名” 的特殊错误提示。
- 不改底层复制逻辑与批量语义。
- 不做 CI 矩阵、预编译产物、Linux/macOS 实测（Q39）。
