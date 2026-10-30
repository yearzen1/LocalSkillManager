# v0.2.0 进度表（40 分钟/天）

> 上级：[实现方案索引](impl.md) ｜ 决策：[decisions.md](decisions.md) ｜ 需求：[Requirements.md](Requirements.md)
>
> 版本目标：v0.2.0（`copy --all` + `remove`）
> 节奏：13 个工作日，每天 40 分钟，一个可验收切片
> 起始日：2026-11-02（周一）｜ 收尾日：2026-11-18（周三）

## 1. 怎么用

- 每天 40 分钟拆成：**5 分钟回顾**昨天结论 → **30 分钟执行**当天切片 → **5 分钟记录**到第 4 节。
- 完成标准只认**可执行命令**或可观察结果，不认"感觉做完了"。
- 卡住超过 10 分钟就停：把卡点写进当天「卡住时」栏，第二天第一件事处理，不要在同一处硬耗。
- 涉及代码的切片按 [tdd](../AGENTS.md) 走"红 → 绿 → 重构"，先写失败用例再改实现。
- 当天没做完不跳天：把剩余步骤顺延到次日，把次日任务里可延后的部分往后推，保持"每天一个切片"。
- 每个切片完成即提交，提交信息已写在日卡里，不要攒到最后一起提交。

## 2. 总览

| 周 | 天 | 主题 | 周交付物 |
|---|---|---|---|
| 1 | D1–D5 | 收尾 + 起头 | 工作区干净、v0.2.0 决策、验收脚本、`copy --all` 红用例与 CLI |
| 2 | D6–D10 | 两个切片的主体 | `copy --all` 完成并入文档、`remove` 决策与 CLI |
| 3 | D11–D13 | 收尾发布 | `remove` 落地、全量回归、v0.2.0 tag |

## 3. 每日任务

### 第 1 周

#### Day 1 · 2026-11-02（周一）· 提交现有改动

- **目标**：把工作区里已完成的 YAML hint 与测试隔离分别提交，让历史保持单一目的。
- **步骤**
  1. `git status --short` 确认改动只有 `src/error/mapping.rs`、`src/i18n/*`、`tests/cli.rs`
  2. 跑 `cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test`
  3. `git add src/` 后提交 hint；再 `git add tests/cli.rs` 后提交测试隔离
- **完成标准**：三条命令退出码均为 0；`git status --short` 输出为空；`git log --oneline -2` 就是下面两条提交
- **提交信息**：`fix(i18n): hint how to fix invalid yaml`、`test: isolate config dir in cli tests`
- **卡住时**：测试红就先只跑失败项（`cargo test <用例名> -- --nocapture`），把原因记到第 4 节；两类改动绝不合并成一个提交
- [x] 完成（偏差见第 4 节 D1 备注）

#### Day 1.5 · 2026-11-02（周一）· `list` 描述列改按列换行

- **目标**：`lsm list` 的中文 description 折行时不再顶到第 0 列，也不再按"字符数"假装 60 列。
- **步骤**
  1. `docs/decisions.md` 追加 Q34：描述列固定 60 **显示列**、换行后悬挂缩进、最多 3 行；后续决策顺延为 Q35+
  2. `docs/impl/storage.md` §2.4 与 §3.2 示例改成换行规则
  3. 先写红用例（中文换行、续行对齐描述列、宽字符不跨行、超 3 行补 `...`），再改 `output/list.rs`
- **完成标准**：`cargo test` 全绿；`lsm list project` 里 `intent-lock` 的续行从描述列开始，每行 ≤ 60 列
- **提交信息**：`docs: decide list text wrapping`、`test(list): pin list wrapping behavior`、`feat(list): wrap descriptions with hanging indent`、`docs: document list wrapping`
- **卡住时**：中英混排仍对不齐就以 `display_width` 为准修换行算法，不改列宽常量
- [x] 完成

#### Day 2 · 2026-11-03（周二）· 决策先行，钉住 v0.2.0 范围

- **目标**：按仓库"决策先行"规则，先把范围写进 `decisions.md`，后面不再为"要不要做"返工。
- **步骤**
  1. `docs/decisions.md` 表尾追加 Q35：v0.2.0 只含 `copy --all` 与 `remove`；`symlink`、反向 copy、`update` 检查仍为非目标
  2. 追加 Q36：CI 矩阵、预编译产物、Linux/macOS 实测**延后，等真实 issue 触发再立计划**（当前交付以 Windows 实测为准）
  3. 同步 `docs/impl.md`「非目标」段与第 5 节约束表，只放摘要 + 链接
- **完成标准**：`Select-String -Path docs/decisions.md -Pattern '\| Q3[56] \|'` 命中 2 行；`docs/impl.md` 新增行能点回 `decisions.md`
- **提交信息**：`docs: record v0.2.0 scope decisions`
- **卡住时**：Q31 的措辞拿不准就写"暂不做，触发条件=有用户 issue 提出平台相关问题"，不写时间点
- [ ] 完成

#### Day 3 · 2026-11-04（周三）· 手工验收写成脚本

- **目标**：把 `docs/impl/quality.md`「手工验收」那串命令变成一条可重复执行的脚本。
- **步骤**
  1. 新建 `scripts/acceptance.sh`，内容对应 `quality.md` 的 7 条手工步骤，每步失败即 `exit 1`
  2. 用临时 `LSM_HOME` 与 demo 项目目录，结束时清理，不写死本机绝对路径
  3. 在 Git Bash 里执行，逐条修到全绿
- **完成标准**：`bash scripts/acceptance.sh` 退出码 0，末尾打印 `PASS`
- **提交信息**：`test: script manual acceptance`
- **卡住时**：Windows 上 `chmod +x` 无效不影响使用，脚本内注释标明用 `bash scripts/acceptance.sh` 调用
- [ ] 完成

#### Day 4 · 2026-11-05（周四）· `copy --all` 红用例

- **目标**：先写三条会失败的用例，把行为钉死。
- **步骤**
  1. `tests/cli.rs` 加用例：全量复制两个 skill、全局为空时退出码 0 且提示无内容、目标已存在时汇总冲突而不中途中断
  2. 只加测试不改实现，`cargo test` 确认三条红
  3. 用 `-- --nocapture` 记录实际输出，作为实现依据
- **完成标准**：`cargo test` 中三条新用例失败，失败原因是"未实现/行为不符"而非用例本身写错
- **提交信息**：`test(copy): add copy-all cases`
- **卡住时**：空全局的期望输出拿不准就先按"退出码 0 + 一行 info 提示"写，文案留到 D7 定稿
- [ ] 完成

#### Day 5 · 2026-11-06（周五）· `copy --all` CLI 接线

- **目标**：`-c --all` 能解析成 `Action::Copy { all: true }`。
- **步骤**
  1. `domain/action.rs` 扩展 `Copy` 变体；`cli/spec.rs` 加 `--all`
  2. `cli/normalize.rs` 里 `-c` 吞参数的行为要兼容 `-c --all`
  3. 补参数用例：`-c --all`、`copy --all`、`copy <name>`，三者互斥
- **完成标准**：三条参数用例绿；`lsm copy --all` 返回"未实现"而非解析错误
- **提交信息**：`feat(cli): add copy --all flag`
- **卡住时**：与 `copy <name>` 的互斥规则有歧义时，取"同时给出则退出码 2 并提示"，与现有短选项非法组合策略一致
- [ ] 完成

### 第 2 周

#### Day 6 · 2026-11-09（周一）· `copy --all` 全量复制实现

- **目标**：D4 的三条用例全绿。
- **步骤**
  1. `commands/copy.rs` 增加全量分支：扫描全局目录 → 逐个 `install_skill_dir`
  2. 冲突走既有 `--force` 语义，未加 `--force` 时累计失败项而不是第一个就退出
  3. 结束打印"成功 N 个 / 跳过 M 个"，全部失败时退出码 1
- **完成标准**：`cargo test` 全绿；临时目录放两个全局 skill 后 `lsm copy --all` 复制成功
- **提交信息**：`feat(copy): copy all global skills`
- **卡住时**：`commands` 不允许直接调 `std::fs`，扫描必须复用 `infra::scan`；需要新接口就先加在 `infra` 再接线
- [ ] 完成

#### Day 7 · 2026-11-10（周二）· `copy --all` i18n 与输出

- **目标**：新提示全部走消息表，en/zh 同步。
- **步骤**
  1. `i18n/mod.rs` 加 key，`en.rs` / `zh.rs` 各补文案（成功汇总、空全局、部分失败）
  2. `output` 侧确认汇总输出用 `tr_args` 而不是字符串拼接
  3. 分别用 `--lang en` / `--lang zh` 各跑一次
- **完成标准**：`en.rs` 与 `zh.rs` 的 key 集合一致；代码里无硬编码用户可见文案
- **提交信息**：`feat(i18n): add copy-all messages`
- **卡住时**：中英文案长度差异导致列宽不齐，就改用逐行输出，别为对齐写死宽度
- [ ] 完成

#### Day 8 · 2026-11-11（周三）· `copy --all` 文档同步

- **目标**：`copy --all` 在文档里与代码一致。
- **步骤**
  1. `docs/impl/storage.md`「各命令实现」补全量分支规则
  2. `docs/impl/cli.md` 命令表加 `-c --all`
  3. 两份 README 的命令参考各加一行示例
- **完成标准**：`docs/impl.md` 第 5 节指向的规则里能找到 `--all`；两份 README 命令表行数一致
- **提交信息**：`docs: document copy --all`
- **卡住时**：文档与实现不一致时以代码为准改文档，不改代码迁就旧文档；差异写进第 5 节风险表
- [ ] 完成

#### Day 9 · 2026-11-12（周四）· `remove` 决策 + 红用例

- **目标**：把"卸载"从非目标移出，并先用例钉住破坏性行为。
- **步骤**
  1. `docs/decisions.md` 追加 Q36：`remove <name> [global|project]` + `--force` 语义、原子删除方式
  2. 改写 `docs/impl.md`「非目标」段，删掉"不做删除、卸载"
  3. `tests/cli.rs` 加三条用例：正常删除、不存在时报错退 1、`--force` 才删非空
- **完成标准**：`Select-String -Path docs/decisions.md -Pattern '\| Q36 \|'` 命中 1 行；三条新用例红
- **提交信息**：`docs: decide skill removal scope`
- **卡住时**：`--force` 语义拿不准就取"无 `--force` 只删由 `add` 创建的合法 skill 目录（含 SKILL.md 且目录名与 name 一致）"，其余必须显式 `--force`
- [ ] 完成

#### Day 10 · 2026-11-13（周五）· `remove` CLI 与校验

- **目标**：`-r/--remove` 能解析并做前置校验。
- **步骤**
  1. `domain/action.rs` 加 `Remove { scope, name, force }`；`cli/spec.rs` 加子命令与短选项
  2. 复用 `SkillName::parse` 防路径穿越；存在性检查放在命令层
  3. 参数用例：无 name、非法 name、scope 缺省
- **完成标准**：参数用例绿；`lsm remove ..` 被拦截且退出码 2
- **提交信息**：`feat(cli): add remove command`
- **卡住时**：短选项冲突时优先换长选项并在 `docs/impl/cli.md` 注明，不为凑短选项改既有行为
- [ ] 完成

### 第 3 周

#### Day 11 · 2026-11-16（周一）· `remove` 原子删除实现

- **目标**：删除失败可回滚，不留半个 skill。
- **步骤**
  1. `infra/fsops.rs` 加 `remove_skill_dir`：先 `rename` 到同级临时目录，成功后再递归删临时目录
  2. `commands/remove.rs` 编排：解析路径 → 校验 → 删除 → 输出
  3. 失败路径用例：目标不可删时不破坏原目录
- **完成标准**：`cargo test` 全绿；D9 三条用例全绿；异常路径下原目录仍完整
- **提交信息**：`feat(remove): uninstall skill atomically`
- **卡住时**：Windows 上 `rename` 仍可能因占用失败，就把错误如实抛出并提示"是否被编辑器占用"，不做静默重试
- [ ] 完成

#### Day 12 · 2026-11-17（周二）· `remove` i18n + 全量回归

- **目标**：命令可用、文案齐、旧功能没被带坏。
- **步骤**
  1. 补 `remove` 的 en/zh 文案（成功、不存在、拒绝删除的提示）
  2. 更新 `storage.md` / `cli.md` / 双 README
  3. 跑四道门禁：`cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test`、`bash scripts/acceptance.sh`
- **完成标准**：四道门禁全绿；`git status --short` 只剩预期文件
- **提交信息**：`feat(remove): add messages and docs`
- **卡住时**：验收脚本里旧的 `add/copy` 步骤失败，说明回归问题优先于文案，先修回归再补文案
- [ ] 完成

#### Day 13 · 2026-11-18（周三）· 版本 0.2.0 与 tag

- **目标**：v0.2.0 收口，以 tag 为唯一发布标记。
- **步骤**
  1. `Cargo.toml` 版本改 `0.2.0`，`cargo build --release` 让 `Cargo.lock` 同步
  2. 跑一次 `./target/release/lsm --version` 与 `lsm list global`，确认本地可用
  3. `git tag v0.2.0` 并 `git push origin v0.2.0`
- **完成标准**：`lsm --version` 输出 `0.2.0`；`git tag --points-at HEAD` 含 `v0.2.0`；远端能看到该 tag
- **提交信息**：`chore: release v0.2.0`
- **卡住时**：tag 打错就删远端 tag 重打（`git push --delete origin v0.2.0`），不靠加新 tag 掩盖
- [ ] 完成

## 4. 打卡表

| 天 | 日期 | 完成 | 实际用时 | 备注（卡点/顺延原因） |
|---|---|---|---|---|
| D1 | 2026-11-02 | ☑ | | 偏差：hint + 测试隔离已合成 `dd005ea` 并推送，按"不改写已发布历史"处理；另以 `5b1563c` 跟踪 `.agents/`、忽略 `TODO.md` |
| D1.5 | 2026-10-30 | ☑ | | 提前完成：列表描述列改按显示列换行（Q34），占掉原 Q34，D2 顺延为 Q35/Q36 |
| D2 | 2026-11-03 | ☐ | | |
| D3 | 2026-11-04 | ☐ | | |
| D4 | 2026-11-05 | ☐ | | |
| D5 | 2026-11-06 | ☐ | | |
| D6 | 2026-11-09 | ☐ | | |
| D7 | 2026-11-10 | ☐ | | |
| D8 | 2026-11-11 | ☐ | | |
| D9 | 2026-11-12 | ☐ | | |
| D10 | 2026-11-13 | ☐ | | |
| D11 | 2026-11-16 | ☐ | | |
| D12 | 2026-11-17 | ☐ | | |
| D13 | 2026-11-18 | ☐ | | |

## 5. 风险与应对

| 风险 | 触发信号 | 应对 |
|---|---|---|
| `remove` 破坏性 | 用户删错目录 | 无 `--force` 只删合法 skill 目录；先 rename 到临时目录再删，失败可回滚 |
| 40 分钟不够 | 连续两天顺延 | 把当天任务砍成"只写用例"或"只改文档"两个半天切片，不合并任务 |
| 文档与实现漂移 | D8/D12 发现文档与代码不同 | 以代码为准改文档，同一提交内完成；差异记入本表 |
| 跨平台问题冒出来 | 出现 Linux/macOS 相关 issue | 单独立计划补 CI 与实测，不在本表临时插入，避免打乱 13 天节奏 |

## 6. 明确不做

- 不做 CI 矩阵、不做预编译产物、不做 Linux/macOS 实测：等真实 issue 触发再立计划（见 Q31）。
- 不排缓冲日、不排机动日：13 天排满，掉队按"顺延不跳天"处理。
- 不建 `CHANGELOG.md`，版本说明以 tag 为唯一事实来源。
- 不做甘特图、不写精确到分钟的时间分配、不做完成率打分。
- 不做 `update` 检查、不做符号链接、不做从 project 反向复制到 global。
