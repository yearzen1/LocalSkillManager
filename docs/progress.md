# v0.3.0 进度表（40 分钟/天）

> 上级：[实现方案索引](impl.md) ｜ 决策：[decisions.md](decisions.md) ｜ 需求：[Requirements.md](Requirements.md)
>
> 上一版：v0.2.0 已发布（tag `v0.2.0`，`copy --all` + `remove`）
>
> 版本目标：v0.3.0（`upgrade` 自更新，`Requirements.md` 第 3 条）
> 节奏：10 个工作日，每天 40 分钟，一个可验收切片
> 起始日：2026-11-02（周一）｜ 收尾日：2026-11-13（周五）

## 1. 怎么用

- 每天 40 分钟拆成：**5 分钟回顾**昨天结论 → **30 分钟执行**当天切片 → **5 分钟记录**到第 4 节。
- 完成标准只认**可执行命令**或可观察结果，不认"感觉做完了"。
- 卡住超过 10 分钟就停：把卡点写进当天「卡住时」栏，第二天第一件事处理，不要在同一处硬耗。
- 涉及代码的切片按 [tdd](../AGENTS.md) 走"红 → 绿 → 重构"，先写失败用例再改实现。
- 当天没做完不跳天：把剩余步骤顺延到次日，把次日任务里可延后的部分往后推。
- 每个切片完成即提交，提交信息已写在日卡里，不要攒到最后一起提交。
- **联网切片只打本地桩**：`LSM_UPGRADE_API` 指向回环地址；真实 GitHub 请求只在 D10 手工验收时跑一次。
- **自替换只碰临时副本**：测试与验收都先 `Copy-Item` 一份 exe，不对正在使用的二进制做替换。

## 2. 总览

| 周 | 天 | 主题 | 周交付物 |
|---|---|---|---|
| 1 | D1–D5 | 解析与网络 | 决策补口、版本解析、资产映射、HTTP 客户端、`--check` 命令层 |
| 2 | D6–D10 | 下载、替换与发布 | 下载校验、原地替换、CLI 接线、文档、v0.3.0 tag |

## 3. 每日任务

### 第 1 周

#### Day 1 · 2026-11-02（周一）· 决策补口：升级怎么测

- **目标**：把"本地桩 + 临时副本"写进决策，后面不再为"能不能连真网"返工。
- **步骤**
  1. `docs/decisions.md` 表尾追加 Q42：`LSM_UPGRADE_API` 可指向任意回环 HTTP 桩；测试只操作临时 exe 副本；真实联网验收只保留一次手动步骤；不推翻 Q39（CI 仍非目标）
  2. `docs/impl/upgrade.md` §8 测试策略补一句：桩服务用标准库 `TcpListener` 起，不引额外测试依赖
  3. 确认 `docs/impl/quality.md` 的 8 条 `[planned]` 用例编号 37–44 不变
- **完成标准**：`Select-String -Path docs/decisions.md -Pattern '\| Q42 \|'` 命中 1 行；`cargo test` 全绿
- **提交信息**：`docs: decide upgrade test harness`
- **卡住时**：Q42 的措辞拿不准就只写两条硬约束——"测试不联网"、"测试不碰正在使用的 exe"

#### Day 2 · 2026-11-03（周二）· 版本解析

- **目标**：`v0.3.0` / `0.3.0` 能被解析并比较，为 `--check` 提供判据。
- **步骤**
  1. 红用例：接受 `v0.3.0`、`0.3.0`；拒绝 `v0.3`、`v0.3.0-rc.1`、`latest`、空串；三段数值比较；前导零按数值处理
  2. 新建 `domain/version.rs`，错误用独立枚举，不构造 `AppError`
  3. 在 `error/mapping.rs` 补 `AppError` 转换
- **完成标准**：`cargo test version` 全绿；`domain::version` 不依赖 `std::fs` 与网络
- **提交信息**：`test(domain): pin version parsing`、`feat(domain): parse release versions`
- **卡住时**：前导零与 `v` 前缀的边界拿不准就以 `upgrade.md` §2.1 为准，不改规则迁就实现

#### Day 3 · 2026-11-04（周三）· 资产映射与元数据解析

- **目标**：把当前平台映射到唯一资产名，并能读出 release 元数据里的 tag 与资产列表。
- **步骤**
  1. 红用例：`upgrade.md` §2.2 的 6 种 os/arch 组合各命中一个资产名；未知组合报错；元数据缺字段、资产重复、非 JSON 各有独立错误
  2. 新建 `domain/release.rs`：平台 → `lsm-<os>-<arch>[.exe]`，元数据结构只保留 `tag_name` 与资产 `name`/下载地址
  3. 解析用 `serde_json`，不把原始 JSON 泄漏到命令层
- **完成标准**：`cargo test release` 全绿；资产名与 `docs/release.md` §1 的命名表逐条一致
- **提交信息**：`test(domain): pin asset selection`、`feat(domain): map platforms to release assets`
- **卡住时**：Windows ARM64 等暂不发布的组合照表实现，不因"当前只有 x64"删映射

#### Day 4 · 2026-11-05（周四）· HTTP 客户端与错误映射

- **目标**：能取到 release 元数据，并把网络、状态码、解析失败分门别类地报出来。
- **步骤**
  1. `Cargo.toml` 加 `ureq` 3（`default-features = false, features = ["rustls"]`），按 Q36 配两个超时
  2. 新建 `infra/http.rs`：GET 元数据端点，`LSM_UPGRADE_API` 覆盖
  3. 红用例用标准库 `TcpListener` 起桩：200 正常、403/429 限流、404、非 JSON 各一条
- **完成标准**：`cargo test http` 全绿；测试全程不访问外网
- **提交信息**：`build: add ureq for upgrade`、`test(http): pin upgrade error mapping`、`feat(http): fetch release metadata`
- **卡住时**：`ureq` 版本 API 不熟就先只跑通 200 一条，再补错误分支，不要一次写四类错误

#### Day 5 · 2026-11-06（周五）· `upgrade --check` 命令层

- **目标**：已最新与有新版本两种结论都能算出来，且不下载、不写盘。
- **步骤**
  1. 新建 `commands/upgrade.rs`：取元数据 → 解析 tag → 与 `CARGO_PKG_VERSION` 比较 → 返回结论
  2. `--check` 分支只输出结论；`--force` 对 `--check` 无意义，忽略
  3. 单测覆盖：已最新、有新版本、当前版本更新且未加 `--force`
- **完成标准**：`cargo test upgrade` 全绿；测试后临时目录无新增文件
- **提交信息**：`test(upgrade): pin check behavior`、`feat(upgrade): report latest release`
- **卡住时**：这一层先不接 CLI——`cli.md` 的 `planned` 行要求命令仍是未知子命令，接子命令留到 D8 一次做完

### 第 2 周

#### Day 6 · 2026-11-09（周一）· 下载与冒烟校验

- **目标**：下载到的文件只有通过 `<新文件> --version` 才算可用。
- **步骤**
  1. `infra/http.rs` 加下载到临时文件的能力，超时 5 分钟（Q36）
  2. 冒烟校验：以子进程执行 `<新文件> --version`，退出 0 且输出含目标 tag 的版本号
  3. 红用例：下载失败、版本不匹配、子进程非 0，三种都不得进入替换流程
- **完成标准**：`cargo test download` 全绿；校验失败时临时文件被清理
- **提交信息**：`feat(upgrade): download and verify release assets`
- **卡住时**：子进程在 Windows 上拿不到输出就先写文件再读，不要为此换校验方式

#### Day 7 · 2026-11-10（周二）· 原地替换与回滚

- **目标**：替换后 exe 路径与文件名不变，失败时原文件仍可执行。
- **步骤**
  1. `infra/fsops.rs` 加 `replace_self`：备份为 `<exe>.old-<pid>` → 换位 → 删备份，失败回滚（Q31）
  2. 单测在临时目录用副本跑：成功、换位失败、备份删不掉留 warning
  3. 确认不产生版本化文件名
- **完成标准**：`cargo test replace_self` 全绿；测试后目录里没有 `.old-*` 残留
- **提交信息**：`feat(fs): replace the running binary in place`
- **卡住时**：Windows 上正在运行的 exe 不能直接覆盖，必须走 rename；失败就如实报错，不静默重试

#### Day 8 · 2026-11-11（周三）· CLI 接线与状态列

- **目标**：`lsm upgrade [--check] [--force]` 可用，`cli.md` 同步转 `implemented`。
- **步骤**
  1. `domain/action.rs` 加 `Upgrade { check, force }`；`cli/spec.rs` 加子命令（无短选项）
  2. `docs/impl/cli.md` §2.1 把 `upgrade` 行改成 `implemented`，同一提交完成
  3. `tests/cli.rs` 落地 `quality.md` 37–44 八条，并删掉 `[planned]` 标记
- **完成标准**：`cargo test` 全绿；`lsm upgrade --help` 退出 0；`lsm -u` 退 2
- **提交信息**：`feat(cli): add upgrade command`
- **卡住时**：`cli_reference_table_matches_binary` 红了先看状态列，不要为了过测试把 `planned` 留着

#### Day 9 · 2026-11-12（周四）· 文档与验收脚本

- **目标**：用户面文档只描述已实现行为，脚本覆盖不联网的部分。
- **步骤**
  1. 两份 README 命令参考各加 `lsm upgrade [--check] [--force]` 与行为说明
  2. `scripts/acceptance.sh` 加两步：`upgrade --help` 退 0、`upgrade --bogus` 退 2（不联网）
  3. `docs/impl/quality.md` §1.3 去掉 `upgrade` 冒烟的 `[planned]` 注释，写清真实联网只手动跑一次
- **完成标准**：`"D:/Git/bin/bash.exe" scripts/acceptance.sh` 退出 0；两份 README 命令表行数一致
- **提交信息**：`docs: document upgrade`、`test: cover upgrade in acceptance script`
- **卡住时**：脚本里起本地桩太复杂就先只做 `--help` 与非法参数两步，真实请求留给手动验收

#### Day 10 · 2026-11-13（周五）· 全量回归与发布

- **目标**：v0.3.0 收口，tag 与 release 是唯一发布标记。
- **步骤**
  1. 四道门禁：`cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test`、`"D:/Git/bin/bash.exe" scripts/acceptance.sh`
  2. `Cargo.toml` 改 `0.3.0`，`cargo build --release`，产物改名为 `lsm-windows-x64.exe`
  3. 手工联网验收：复制一份 exe，`upgrade --check` 确认能查到 `v0.2.0`；再跑 `upgrade --force` 确认替换后 `--version` 正常
  4. `git tag -a v0.3.0`，推送 `main` 与 tag，按 [release-notes.md](release-notes.md) 模板发 release
- **完成标准**：四道门禁全绿；`lsm --version` 输出 `0.3.0`；远端 release 资产可下载且字节数一致
- **提交信息**：`chore: release v0.3.0`
- **卡住时**：联网验收失败先判断是 GitHub 限流还是资产缺失，限流就等 10 分钟重试，不改实现绕过

## 4. 打卡表

| 天 | 日期 | 完成 | 实际用时 | 备注（卡点/顺延原因） |
|---|---|---|---|---|
| D1 | 2026-11-02 | ☑ | 15 min | Q42 入库；`upgrade.md` §8 补「桩不引测试依赖」；`quality.md` 37–44 编号未变 |
| D2 | 2026-11-03 | ☑ | 30 min | `domain::version` 纯函数；新增 `UpgradeInvalidVersion(+Hint)` 两条文案，`upgrade.md` §7 补 tag 非法一行；顺手修掉 Q42 之前留下的「32 项」计数 |
| D3 | 2026-11-04 | ☑ | 30 min | `domain::release`：六组合映射 + 元数据解析；重复资产按文档取首个并记录（计划写的是「独立错误」，以 §2.3 为准）；未接 `AppError`，留到 D5 |
| D4 | 2026-11-05 | ☑ | 40 min | 模块名用 `infra::net`（计划写 `infra/http.rs`，以 §3/§8 与 `quality.md` 为准）；`ureq` 3 已落；超时用例未写——固定 30 秒超时无法在单测里等，留手工人为验证；`NetError → AppError` 映射留到 D5 |
| D5 | 2026-11-06 | ☑ | 35 min | `check` 返回 `Check`，打印留给 D8；`--force` 尚未进签名，等于天然忽略；新增 7 条 upgrade 文案与 `NetError`/`ReleaseError` 映射；`UnsupportedPlatform` 暂时并入取元数据失败，D7 再细分 |
| D6 | 2026-11-09 | ☐ | | |
| D7 | 2026-11-10 | ☐ | | |
| D8 | 2026-11-11 | ☐ | | |
| D9 | 2026-11-12 | ☐ | | |
| D10 | 2026-11-13 | ☐ | | |

## 5. 风险与应对

| 风险 | 触发信号 | 应对 |
|---|---|---|
| 自替换弄坏正在运行的 exe | 替换后 `--version` 失败 | 只在临时副本上跑测试；Windows 用 rename 换位，失败回滚并保留 `.old-*` |
| 网络测试不稳定 | 测试偶发超时 | 只用回环桩，不联外网；真实请求只留一次手动验收 |
| 下载内容不可信 | 冒烟校验失败 | 以 `<新文件> --version` 退出 0 且版本匹配为唯一通过条件（Q31） |
| 文档与实现漂移 | D8/D9 发现 `cli.md` 与 `--help` 不一致 | 以代码为准改文档，同一提交内完成；状态列与子命令必须同一次提交 |

## 6. 明确不做

- 不做 CI 矩阵、预编译产物、Linux/macOS 实测：仍按 Q39，等真实 issue 触发再立计划。
- 不做自动/后台检查更新：只在用户显式执行 `upgrade` 时联网（Q30）。
- 不发布 `.sha256`：v1 用「下载后执行 `--version`」兜底（Q31）。
- 不建 `CHANGELOG.md`：版本说明按 [release-notes.md](release-notes.md) 模板写进 release。

## 7. 候选池（来自 `TODO.md`，待决策后再排期）

| 候选 | 状态 | 前置 |
|---|---|---|
| `copy a b c` / `remove a b c` 多参数 | 待决策 | 需扩 `--all` 与多 name 的互斥规则 |
| `search` 命令 | 待决策 | 需定匹配字段（name / description）与输出格式 |
| copy/remove 找不到时自动 search | 待决策 | 依赖 `search`，且会改现有 `not found` 输出 |
| project → global 反向复制 | 待决策 | 与 Q38 冲突，需先定命令形态 |
| 日志功能 | 待决策 | 范围未定（文件日志 / 调试开关 / 审计） |
