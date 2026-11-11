# 测试、README 与发布

> 上级：[实现方案索引](../impl.md) ｜ 相关：[存储与命令](storage.md) ｜ [CLI 与输出](cli.md) ｜ [配置与持久化](config.md) ｜ [决策记录](../decisions.md)
>
> 本文拥有以下规则的完整定义：测试计划、README 结构、许可证与发布流程。

## 1. 测试计划

### 1.1 单元测试

单测与模块同文件（`#[cfg(test)]`）；涉及磁盘的 `infra` 用例使用 `tempfile`。

- `domain::lang`：`parse` 接受 `en`/`zh`/`en-US`/`zh-CN`，拒绝 `fr`、空串、大小写变体；拒绝时返回 `LangError`。
- `domain::name`：合法（`a`、`a1`、`my-skill`）、非法（`My-Skill`、`-a`、`a--b`、`a_`、空串）、64/65 字符边界、Windows 保留名（`con`、`nul`、`com1`、`lpt9`）。
- `domain::resolve`：精确优先于前缀（`a` 与 `ab` 并存时 `a` 仍精确命中）、唯一前缀命中、多命中按字典序返回候选、零命中。
- `domain::meta`：description 空值、1024/1025 字符边界。
- `domain::version`：接受 `v0.1.1`/`0.1.1`，拒绝 `v0.2`、`v0.2.0-rc.1`、`latest`、空串；三段数值比较；前导零按数值处理。
- `domain::release`：资产选择命中/缺失/重复；压缩包不匹配；平台映射覆盖表内组合，未知 OS/arch 报错。
- `infra::frontmatter`：正常、缺 `---`、缺 `name`、缺 `description`、非法 YAML、BOM 开头、CRLF 行尾、额外字段保留。
- `infra::env`：`Env::read` 正确拾取 `LSM_HOME`/`LSM_CONFIG_DIR`/`LSM_LANG`/`LSM_UPGRADE_API`，空串按未设置；读取是快照，后续设置环境变量不影响已读值；`exe` 来自 `current_exe()`。
- `infra::paths`：纯函数测试，构造 `Env` 字面量即可，**不修改进程环境**；`LSM_HOME` 覆盖生效、未设置时落到家目录、project 始终为 cwd、`LSM_CONFIG_DIR` 覆盖配置目录、缺省落到 `dirs::config_dir()/lsm`。
- `infra::net`：本地桩覆盖 200、302、404、403、非 JSON、超时；请求头含 `User-Agent: lsm/<version>`；`LSM_UPGRADE_API` 空串回落到默认端点。
- `cli::lang`：`prescan` 支持 `--lang zh`/`--lang=zh`/多处取最后一个；`decide` 三态决策表（`Cli` 优先 → `Env` 次之 → `NeedFile`），非法值返回 `LangError`，前两档命中时永不返回 `NeedFile`。
- `infra::config`：读写往返；注释与未知键保留；缺文件 → `None`；坏 TOML → `Parse`；`lang = "fr"` → `InvalidValue`；同值写入仍成功；写入失败不破坏旧文件。
- `cli::normalize`：短选项映射、省略作用域时补 `project`、互斥、混用形式；`config` 子命令不被归一化改写。
- `error::mapping`：每个窄错误枚举都能转成 `AppError`，`kind` 正确且带预期 `MsgKey`/hint。
- `output::error`：`error:`/`hint:` 与 `错误:`/`提示:` 两套前缀、hint 省略时不输出空行、`--json` 场景由调用方控制 stdout。
- `infra::fsops`：`contains_path` 对 `..`、符号链接、目标不存在等场景的判断；`replace_self` 原地替换成功、权限位保留、备份清理、回滚时旧文件仍可执行。

### 1.2 集成测试（`tests/cli.rs`、`tests/config.rs`、`tests/upgrade.rs`）

每个用例使用独立 `tempfile::TempDir`，设置 `LSM_HOME`、`LSM_CONFIG_DIR`、`current_dir`，互不干扰。命令功能用 `tests/cli.rs`，配置持久化用 `tests/config.rs`，升级冒烟校验用 `tests/upgrade.rs`。

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
13. `pull foo`：global → project 成功。
14. `pull foo` 目标已存在 → 失败；`--force` → 成功。
15. `pull missing` → 失败，提示 `lsm search missing global`。
16. 短选项：`-a ... -g`、`-a ... -p`、`-l -g`、`-l -p`、`-r foo -p` 全部通过；`-c foo` 退 2（已移除）。
17. 非法组合：`-g -p`、`lsm add foo.md -g` → 退出码 2。
18. 默认 help 为英文（含 `Usage:`），`--lang zh --help` 为中文（含 `用法:`）。
19. `LSM_LANG=zh` 时错误消息为中文；命令行 `--lang en` 覆盖环境变量。
20. `--json` 输出中不混入 warning / 提示文本。
21. 安装失败后目标目录不存在残留（构造只读/冲突场景）。
22. `lsm` 无参数 → 本地化 help 到 stderr、退出码 2；`lsm --help` → stdout、退出码 0。
23. `pull ../evil`、`pull .`、`pull a/b` → 退出码 2，且磁盘上不产生任何新目录。
24. 目录名与 frontmatter `name` 不一致 → `list` 标 `[invalid]`，`pull` 该目录报错。
25. 保留设备名：`add`（frontmatter `name: con`）→ 退出码 1；`pull con` → 退出码 2。
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
37. `upgrade --check` 已最新 → 退出 0，stdout 含 `up to date`，磁盘无任何新文件。
38. 桩返回更高 tag → `upgrade --check` 打印 `new version available`，退出 0，不下载、不写盘。
39. `upgrade --force`（桩 tag = 当前版本）→ 二进制被替换，**路径与文件名不变**，内容等于下载载荷，`.lsm-upgrade-*` 已清理；Windows 上运行中的旧映像删不掉，允许留下 `<exe>.old-*` 由下次升级清理（[upgrade.md](upgrade.md) §5）。
40. 当前平台资产缺失 → 退出 1，stderr 含期望资产名与 tag。
41. 桩返回 404 / 非 JSON / 403 → 退出 1，hint 指向镜像或手动下载。
42. exe 所在目录不可写 → 退出 1，原文件字节不变（仅 Unix 可稳定复现，Windows 上不造 ACL）。
43. `upgrade --help` 两种语言均输出到 stdout 且退出 0；`lsm -u`、`upgrade --bogus` → 退出 2。
44. 升级成功后再次执行 `upgrade --check`（桩 tag 不变）→ 退出 0 且报告已最新。
45. 真源一致性：`docs/impl/cli.md` §2.1 表的 `implemented` 行必须 `lsm <command> --help` 退出 0，`planned` 行必须被视为未知子命令（退出 2）——由 `tests/cli.rs::cli_reference_table_matches_binary` 强制。
46. 不加过程 flag 时 `upgrade --force` 成功路径 **stderr 为空**，stdout 仍只有结果那一行（默认静默回归守护）。
47. `upgrade --force --verbose` → stderr 含 `checking for the latest release`、`downloading`、`verifying`、`replacing`，stdout 不变；`upgrade --check --verbose` → stderr 只有 checking 行。
48. `upgrade --force --progress` → stderr 含进度行（`downloading`、`%`、`\r` 重绘），最后一行以换行收尾；下载中途失败时错误块另起一行。

v0.4.0 新增（Q43–Q47）：

46. `pull a b` 两个都成功 → 退出 0，两个目录都在，stdout 含成功计数。
47. `pull a missing` → 退出 1；`a` 已复制，`missing` 的错误体在 stderr。
48. `pull a b` 其中 `b` 目标已存在且无 `--force` → 跳过并 warning，退出 0。
49. `pull a ../evil` → 退出 2，且 `a` 也没有被复制（全量预校验）。
50. `remove a b project` → 两个都删，退出 0。
51. `remove a missing project` → `a` 被删，`missing` 报错，退出 1。
52. `remove a ../evil project` → 退出 2，`a` 仍在。
53. `-r a b -p` → 归一化为 `remove a b project`，两个都删。
54. `search review global` 命中 name 与 description，大小写不敏感；文本表格与 `list` 同格式。
55. `search review global --json` → 合法 JSON 数组，字段与 `list` 一致。
56. `search zzz global` → 退出 0，文本含 `No skills matched`；`--json` 输出 `[]`。
57. `search "" global` / `search x`（缺 scope）→ 退出 2。
58. `push a` → 项目 skill 进入 global；目标已存在无 `--force` → 失败；`--force` 覆盖。
59. `push --all` 汇总成功/跳过/失败；项目为空 → 退出 0 并提示。
60. `push a ../evil` → 退出 2，`a` 未推送。
61. not-found hint：`pull`/`remove` 指向 `lsm search <name> <scope>`，`push` 指向 `lsm list project`；不自动执行 search。
62. `search --help` / `push --help` 双语输出到 stdout 且退出 0；未知参数退 2。
63. `copy foo`、`copy --all` → 退出 2（未知子命令），不保留别名。
64. `-c foo`、`--copy foo` → 退出 2（未知参数），`-c` 已随 `copy` 移除。

name 前缀解析（Q50）：

65. `pull code`（global 只有 `code-review`）→ 退出 0，项目内出现 `code-review`。
66. `pull code`（global 有 `code-lint`、`code-review`）→ 退出 1，stderr 列出两个候选，项目目录无新增。
67. `remove code project`（只有 `code-review`）→ 成功删除 `code-review`。
68. `remove code project`（有 `code-lint`、`code-review`）→ 退出 1，两个目录都还在。
69. `push code`（project 只有 `code-review`）→ 进入 global。
70. `pull code missing` → 退出 1，`code-review` 已复制。
71. `pull Code` / `pull ../evil` → 仍退 2，不产生任何目录。

批量输出排版：

72. `pull missing-a missing-b` → 退出 1，stderr 两个错误块之间、汇总行与批次总错误之间各有一个空行；单 name `pull nope` 的 stderr 不以 `\n\n` 结尾。
73. `pull --all` 遇两个非法 skill → 错误块之间与批次总错误前同样有空行。
74. `remove missing-a missing-b project` → 两个错误块之间与批次总错误前同样有空行。

作用域默认 `project`（Q51）：

75. `add foo.md`（无 scope）→ 安装到 `<cwd>/.agents/skills/foo`。
76. `list`（无 scope）→ 列出 project；`search foo`（无 scope）→ 只搜 project。
77. `remove foo`（无 scope）→ 删除 project 里的 `foo`；`remove foo global` → 只删 global。
78. `remove global`（无 name）→ 退出 2，hint 提示写 `lsm remove <name> <scope>`；`remove global project` 可删除名为 `global` 的 project skill。
79. `-a foo.md`、`-l`、`-r foo` → 均落到 project；`-g`/`-p` 仍覆盖默认。
80. `add`/`list`/`search` 的 `--help` 显示 `[SCOPE]`；`remove` 的 `--help` 只显示 `<NAME>...`，scope 规则写在其帮助文案里；取值仍大小写敏感。

`search` 字段过滤（Q52）：

81. `search review --name` 命中名称含 `review` 的 skill，不命中仅描述含 `review` 的 skill。
82. `search review --description` 反向命中；非法项（description 为空）不命中。
83. `search review --name --description` 与不写 flag 的结果一致。
84. `search review --name --json` 的过滤结果与文本模式一致。

v0.6.1 help 汉化：

85. `--lang zh` 下 `add` / `list` / `search` 的 `<SCOPE>` 帮助显示 `[可选值: global, project]`，`config get/set` 的 `<KEY>` 同样；英文仍为 `[possible values: ...]`。
86. `config get/set` 的 `<KEY>` 帮助文案为「配置键：lang」/「Configuration key: lang」。

批量输出与本地化回归：

87. `pull --all` 遇到两条安装阶段失败（而非 invalid 条目）时，错误块之间与批次总错误之前各有一个空行。
88. `--lang zh` 下 `pull` / `remove` 遇到目录名与 frontmatter `name` 不一致时，错误详情为中文（`与 frontmatter 中的 name ... 不一致`），英文仍为 `does not match frontmatter name`。

skill 分组（Q53–Q57，v0.7.0）：

89. `group set frontend tdd grill` → 退出 0，`groups.toml` 出现 `frontend` 与两个成员；`group list` 输出 `frontend: tdd, grill`；`config.toml` 不被创建或修改。
90. 再次 `group set frontend ...` 无 `--force` → 退出 1 且旧成员不变；加 `--force` → 覆盖成功。
91. `group set Frontend tdd` / `group set frontend Bad-Name` → 退出 2，配置文件不产生任何新组。
92. `group add frontend grill tdd`：组不存在时自动创建；已有成员静默去重，成员顺序保持首次出现。
93. `group list` 无组 → 退出 0 输出 `No groups found.`；`--json` 输出 `[]`；有组时 JSON 字段为 `name` / `skills`，组按字典序、成员保留定义顺序。
94. `group delete a missing` → 退出 1，`a` 被删除、`missing` 报错，结束打印成功/失败计数；删除未被引用的组不需要 `--force`。
95. `groups.toml` 结构非法（`frontend = "bad"`、成员是数字）→ `group list` / `pull --group` 退 1，提示修复或删除 `groups.toml`；`list` / `search` / 其他命令仍正常。
96. `pull --group common`（成员 `alpha`、`beta`）→ 两个 skill 都进入项目，结束打印 `pulled 2, skipped 0, failed 0`；只有 1 个成员时也打印计数。
97. `pull --group a --group b` 展开后重复 skill 只拉取一次；组不存在、组为空、成员缺失都逐条报错并继续，任一失败退 1。
98. `pull alpha --group common`、`pull --group common --all` → 退出 2，不复制任何 skill。
99. `pull --group` 的成员仍支持唯一前缀（成员 `code` → `code-review`）；目标已存在且无 `--force` 算跳过，退 0。
100. `group --help` / `group list --help` / `group set --help` 两种语言均正常；`group set --help` 列出 `--force`，`group list --help` 列出 `--json`。

分组成员校验与模糊匹配（Q58，v0.7.1）：

101. `group set Frontend tdd` → 退出 2，hint 提示 `did you mean 'frontend'?`；组名规则未放宽。
102. `group set frontend missing-skill`（global 只有 `grill`）→ 退出 1，hint 含 `grill`，配置文件不产生或修改 `frontend`。
103. `group add frontend code`（global 只有 `code-review`）→ 退出 0，配置里存储 `code-review` 而不是 `code`。
104. global 同时有 `code-lint`、`code-review` 时 `group add frontend code` → 退出 1 并列出两个候选，整组不写盘。
105. 一次给多个坏成员 → 每个坏成员各打印一条错误，最后汇总一行并退 1；合法成员也不写入。
106. `group delete predev`（已存在 `predevelop`）→ 删除 `predevelop`；存在多个同前缀组时退 1 并列出候选。
107. `pull --group predev`（已存在 `predevelop`）→ 展开 `predevelop`，输出显示全名。
108. 已有 `predevelop` 时 `group add predev x` → 创建新组 `predev`，不做前缀解析。
109. 手改 `groups.toml` 写入 global 不存在的成员后执行 `pull --group` → 该成员按条失败，其余继续（原 97 的兜底不变）。

组引用与成员删除（Q60，v0.8.0）：

110. `group add frontend @common` → 退出 0，`group list` 显示 `frontend: @common, ...`。
111. `group list --resolved` 把 `@common` 展开成其成员，嵌套两层也正确；`--json` 同样生效。
112. `pull --group frontend` 拉取引用组里的全部 skill，重复 skill 只拉一次。
113. 引用唯一前缀：`@comm` → `@common`；组名不存在时退 1 并列出候选。
114. `group add a @a` → 退出 1，提示不能引用自身，不写盘。
115. 已有 `b = ["@a"]` 时 `group add a @b` → 退出 1，报 `a -> b -> a`，不写盘。
116. 手改配置制造环后 `pull --group a` → 该组失败并打印环路径，其他组继续。
117. 写入不存在的 `@ref` 退 1；手改的悬空引用在 `pull --group` 时按该组失败。
118. `group remove frontend @common` 只移除引用，`common` 分组仍然存在。
119. `group remove frontend tdd` 移除成员；成员不在组里时退 1、整组不修改并给候选。
120. `group remove frontend`（不给成员）→ 退出 2，hint 指向 `group delete frontend`。
121. `group delete common` 被 `frontend` 引用时退 1 并列依赖者；加 `--force` 后删除 `common` 并移除 `frontend` 里的 `@common`。
122. `group delete a missing` 批量删除：`a` 被删、`missing` 报错，结束打印计数并退 1。
123. `group list --json` 输出 `name` / `skills` / `includes`，`skills` 不含 `@ref`。
124. `group set frontend tdd` 后 `groups.toml` 出现顶层 `frontend = ["tdd"]` 与文件头 `# Skill groups`；`config.toml` 不被创建或修改。
125. 旧版 `config.toml` 含 `[groups]` 且没有 `groups.toml`：`group list` 读到旧分组且不写盘；随后 `group add` 写出 `groups.toml` 并删除旧 `[groups]`，`lang` 与注释保留。
126. `groups.toml` 与旧 `[groups]` 同时存在时以 `groups.toml` 为准，旧表被忽略。
127. `config.toml` 语法坏、`groups.toml` 正常：`--lang en group list` 成功；不带 `--lang` 仍因语言解析退 1。
128. `groups.toml` 语法坏时不回退旧表：报 `cannot parse groups file` 并指向 `groups.toml`。
129. 旧 `[groups]` 删除失败（如 `config.toml` 只读）→ 分组已写入 `groups.toml`，stderr 打 warning，退出码仍为 0。

`[planned]` 条目不计入已实现用例数；实现后删掉标记，并同步 `docs/impl/cli.md` §2.1 的状态列（当前无 `[planned]` 用例）。

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
cd <demo-project> && lsm pull sample && lsm list project
lsm search sample project                       # 按 name / description 搜索
lsm pull sample                                # 多 name 也可以：lsm pull a b（a 已存在则跳过）
lsm pull --all                                 # 目标已存在时跳过，退出码 0
lsm add ../gamma.md project && lsm push gamma  # project → global
lsm push --all                                 # 全局已有则跳过
lsm remove sample gamma project                # 多 name：lsm remove a b project

# v0.6.x：默认作用域、唯一前缀、字段过滤与 help 汉化
lsm list                                       # 默认 project
lsm pull samp                                  # 唯一前缀 → sample
lsm remove samp project                        # 唯一前缀 → sample
lsm search second --description global         # 只在描述里命中
lsm search second --name global                # 无匹配，退出 0
lsm --lang zh add --help | grep '可选值'        # [可选值: global, project]

# v0.7.x：分组
lsm group set frontend sample                  # 创建分组
lsm group list                                 # frontend: sample
lsm pull --group frontend                      # 拉取整组
lsm group add frontend sample                  # 追加成员
lsm group delete frontend                      # 删除分组

# v0.7.1：分组成员校验与模糊匹配
lsm group set Backend sample                   # 退出 2，hint 提示 'backend'
lsm group set backend samp                     # 唯一前缀 → sample，写入全名
lsm group set backend nope                     # 退出 1，不写盘
lsm group delete back                          # 唯一前缀 → backend

# v0.7.2：upgrade 过程反馈
lsm upgrade --check --verbose                   # stderr 打印 checking 行，stdout 仍是结果行
lsm upgrade --progress                          # 下载阶段 stderr 单行重绘进度条

# v0.8.0：组引用与成员删除
lsm group set base sample
lsm group add frontend @base                   # 引用无需安装
lsm group list --resolved                      # 展开 @base
lsm group remove frontend @base                # 只移除引用
lsm group set frontend sample
lsm group remove frontend sample               # 只移除成员
lsm group delete base                          # 无引用时直接删除

# v0.9.0：分组独立存储
lsm group set frontend sample
lsm group list                                 # frontend: sample
cat "$LSM_CONFIG_DIR/groups.toml"              # 顶层 frontend = ["sample"]；未设变量时用平台默认路径
lsm config set lang zh                         # 只改 config.toml，不动 groups.toml

# 离线：只验子命令存在与非法参数，不联网
lsm upgrade --help                              # 列出 --check / --force / --verbose / --progress，退出 0
lsm upgrade --bogus                             # 退出 2
```

外部可达时还有一次**手工联网**验收（不进脚本、不进 `cargo test`）：

```bash
cp ./target/release/lsm /tmp/lsm-check
/tmp/lsm-check upgrade --check    # 只查不装：打印已最新，或新版本提示（含当前版本）
rm -f /tmp/lsm-check
```

- 这一段是唯一的联网步骤，且只对**副本**执行：`upgrade` 会替换 `current_exe()`，不要对日常使用的 `lsm` 直接跑。
- Windows 上用 `Copy-Item` 复制到临时目录后执行同样的 `--check`。
- 发布前的 `upgrade --force` 手工验收见 [release.md](../release.md)「升级契约」：用上一版二进制执行一次，确认替换后 `--version` 正常且没有 `.old-*` 残留。
- v0.6.x 的默认作用域、唯一前缀、`search` 字段过滤与 help 汉化由 `scripts/acceptance.sh` 第 13–16 步覆盖；v0.7.x / v0.8.0 的分组由第 19–21 步覆盖。脚本与本节的文字不一致时以脚本为准。

上面这串步骤的执行入口是 `scripts/acceptance.sh`：Windows 用 Git Bash 调用（`"D:/Git/bin/bash.exe" scripts/acceptance.sh`），**不要用 WSL 的 `bash`**——它会把 Linux 路径原样传给 Windows 二进制，测试会在错误目录里跑出假绿。Linux/macOS 直接 `bash scripts/acceptance.sh`。脚本与本节的步骤不一致时以脚本为准，并在同一提交内改本节。

## 2. README 与许可证

### 2.1 双文件约定

- `README.md`：英文，主文件，顶部放指向 `README.zh-CN.md` 的中文切换链接。
- `README.zh-CN.md`：中文，顶部放指向 `README.md` 的英文切换链接。
- 两份内容结构一致，命令示例、错误示例都覆盖两种语言输出。
- 两份 README 只描述 `implemented` 命令；`planned` 命令只出现在 [`cli.md`](cli.md) §2.1 的状态列里。

### 2.2 内容大纲

`README.md`（英文，主文件）与 `README.zh-CN.md`（中文）结构一致，只做落地页，不重复用户文档正文：

1. 一句话介绍 + 语言切换链接。
2. 安装：`cargo install --path .`、本地 release 构建。
3. 快速开始：写一个 `my-skill.md`，`lsm add my-skill.md global`，`lsm pull my-skill`。
4. 文档入口：按 Diátaxis 四问链接到 [`../README.md`](../README.md) 下的教程 / 指南 / 参考 / 解释。
5. 目录约定：`~/.agents/skills`、`<project>/.agents/skills`；提示"在项目根目录执行"。
6. 退出码摘要，完整表格链接到 [`../reference/cli.md`](../reference/cli.md)。
7. 开发：`cargo test`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`。
8. License：MIT，链接 `LICENSE`。

用户文档目录（教程 / 指南 / 参考 / 解释）的职责与入口由 [`../README.md`](../README.md) 定义；README 只链接，不复制命令参数、配置字段或 SKILL.md 校验表。

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
