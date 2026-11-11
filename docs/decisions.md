# 决策记录

> 上级：[实现方案索引](impl.md) ｜ 需求：[Requirements.md](Requirements.md)
>
> 本文是设计决策的唯一事实来源。新增决策时按顺序编号（Q33、Q34…）追加，并同步更新对应主题文档；主题文档只保留规则本身，不复制本表。

| # | 决策 | 结论 |
|---|---|---|
| Q1 | project 作用域 | 固定 cwd，不向上查找 |
| Q2 | README 语言 | 英文主文件 + 中文独立文件 |
| Q3/Q11 | `list --json` 空目录 | 输出 `[]` |
| Q4 | 目录复制排除规则 | 不做任何排除，`.git` 也复制 |
| Q5/Q12 | `--force` 覆盖 | 备份 → 安装 → 删备份，失败回滚 |
| Q6/Q13 | v1 交付范围 | 代码 + 测试 + README + 本地 release，CI 后置 |
| Q7/Q14 | 短选项 | 仅分离写法，非法组合退出码 2 |
| Q8/Q15 | TDD 落地 | `git init`，每切片原子提交 |
| Q9 | frontmatter 长度 | name ≤ 64、description ≤ 1024，超限报错 |
| Q10 | 双语实现 | 运行时消息表 + `--lang` / `LSM_LANG` |
| Q16 | 源包含目标 | 硬报错 |
| Q17 | 源含 `.git` | 照常复制 + stderr 警告 |
| Q18 | 中文 help | help 全量本地化，覆盖 Q10 的"help 保持英文" |
| Q19 | skill 身份 | 目录名为准；名字不一致标 `[invalid]`；`copy` 先校验入参 |
| Q20 | 无子命令 | help 到 stderr + 退出码 2；显式 `--help` 才 stdout + 0 |
| Q21 | 符号链接 | 统一拒绝并报错，不跳过、不跟随 |
| Q22 | Windows 保留名 | name 校验阶段提前拒绝 |
| Q23 | 许可证 | MIT |
| Q24 | 项目结构 | 单 crate + `lib.rs` + 分层模块，不拆 workspace |
| Q25 | 模块拆分 | 采用 `domain` / `infra` / `cli` / `output` / `commands` 结构 |
| Q26 | 错误模型 | 结构化 `AppError`，`anyhow` 仅作 source |
| Q27 | 语言持久化 | 新增 `config.toml`（`dirs::config_dir()/lsm`，`LSM_CONFIG_DIR` 可覆盖）+ `lsm config get/set lang`；TOML 用 `toml_edit` 保留注释与未知键 |
| Q28 | 配置加载与失效 | 优先级 `--lang` > `LSM_LANG` > 文件 > `en`；惰性读取（前两档命中不读文件）；坏配置退 1 且不静默忽略；不做项目级配置 |
| Q29 | 分层收敛与错误映射 | `domain`/`infra` 返回自身错误枚举，`AppError` 只在边界构造；`i18n` 纯化（不读环境/文件，`Lang` 合并进 `domain::lang`）；`infra::env` 是唯一环境读取点；`paths` 收 `&Env` 的纯函数；排版统一到 `output`；`commands` 收 `Ctx` |
| Q30 | 自更新范围 | 新增 `lsm upgrade [--check] [--force]`，只更新 lsm 自身二进制，不触碰 `.agents/skills`；不加短选项；`--check` 只查不装，发现新版本也退 0（查询失败仍退 1）；`--force` 允许同版本重装与降级回滚；不做自动/后台检查更新 |
| Q31 | 升级的校验与替换 | HTTPS 下载后以 `<新文件> --version` 退出 0 且输出含 release tag 版本号才算校验通过；**原地替换：exe 路径与文件名保持不变**，禁止版本化文件名；备份只用 `<exe>.old-<pid>` 临时名，成功即删，删不掉留 warning 并在下次升级清理；任一步失败回滚且原文件可执行 |
| Q32 | 升级的网络与资产契约 | 查询 `https://api.github.com/repos/yearzen1/LocalSkillManager/releases/latest`；`LSM_UPGRADE_API` 覆盖端点（镜像与测试）；资产名 `lsm-<os>-<arch>[.exe]` 是唯一查找键；tag 必须是 `vMAJOR.MINOR.PATCH`；不读 `GITHUB_TOKEN`，限流给可操作提示 |
| Q33 | CLI 真源与文档同步 | 命令与参数的唯一真源是 `impl/cli.md` §2.1 的状态表（`implemented` / `planned`）；改动顺序固定为「先真源状态列 → 再 `cli/spec.rs`/`domain/action.rs` → 最后两份 README」；两份 README 只描述 `implemented` 命令；`tests/cli.rs::cli_reference_table_matches_binary` 强制状态列与二进制一致（`planned` 必须仍是未知子命令、退出 2） |

| Q34 | list 文本列宽 | 描述列固定 60 **显示列**（宽字符占 2 列），不探测终端宽度；超宽换行并悬挂缩进对齐描述列；最多 3 行，第 3 行末尾补 `...`；宽字符不跨行拆开；`--json` 原样输出，不换行、不截断 |

| Q35 | 显示宽度来源 | 列宽改用 `unicode-width`（`default-features = false`，只用 `width()` 不用 `width_cjk()`）：Ambiguous 字符按 1 列，与 Windows Terminal / VS Code 默认渲染一致；测量以整串为单位，ZWJ emoji 序列与组合标记按序列总宽计算；换行与截断的断点仍在 `char` 边界，不引 `unicode-segmentation`，极端序列可能被拆成两段（各按 2 列计，不超 60）；非空白的控制字符在排版前丢弃 |
| Q36 | upgrade 的 HTTP 客户端 | 用 `ureq` 3（`default-features = false, features = ["rustls"]`）：rustls + ring + 内置 webpki 根证书，无 gzip、无 async 运行时；`max_redirects` 用库默认的 10；元数据与资产各配一个 `timeout_global`（30 秒 / 5 分钟）；403 走 `http_status_as_error` 映射为限流提示；根证书不随系统更新，靠升级 ureq 版本刷新，内网 MITM 场景用 `LSM_UPGRADE_API` 镜像兜底；依赖在 `upgrade` 切片实现时再加 |

| Q37 | 选项表的真源地位 | `impl/cli.md` §2.1 除命令表外必须维护选项表（命令 / 选项 / 取值 / 默认 / 说明），与命令表同属唯一真源；选项表不单列状态，存在性跟随命令表；改动顺序先选项表、再 `cli/spec.rs`、最后 README；`tests/cli.rs::cli_option_table_matches_help` 双向强制（文档选项必须出现在 `--help`，`--help` 选项必须写进表）；`-a/-l/-c/-g/-p` 是归一化别名，不进选项表 |

| Q38 | v0.2.0 范围 | 只做 `copy --all` 与 `remove`；`symlink`、project→global 反向复制、`update` 检查仍为非目标；两条命令都先在 `impl/cli.md` §2.1 写 `planned`、实现后转 `implemented`；已实现命令上的新选项（如 `copy --all`）不预写 `planned`，与 `cli/spec.rs` 同一提交落地 |
| Q39 | 跨平台验证与 CI | v0.2.0 不做 GitHub Actions 矩阵、不做预编译产物、不做 Linux/macOS 实测，以 Windows 实测为唯一交付依据；触发条件=出现平台相关 issue，届时单独立计划，不并入 13 天节奏 |
| Q40 | `copy --all` 批量语义 | `--all` 与 `<name>` 互斥（同时给出退 2）；逐条复用 `copy <name>` 的校验与原子安装，单条失败不中断：目标已存在且无 `--force` 算**跳过**（不算失败），源非法或 I/O 失败算**失败**并逐条报错；结束打印成功/跳过/失败计数，任一失败退 1，只有跳过或全部成功退 0，全局为空退 0 并提示无可复制内容 |
| Q41 | `remove` 卸载语义 | 签名 `remove <name> <scope> [--force]`，scope 必填（与 `add`/`list` 一致），短选项 `-r <name> -g/-p`；无 `--force` 只删「目录名等于 name、含合法 `SKILL.md`、且 frontmatter name 与目录名一致」的目录，其余必须显式 `--force`；删除方式为「rename 到同级 `.tmp-remove-*` → 递归删临时目录」，rename 失败原目录不动；目标不存在退 1；不做批量删除 |
| Q42 | upgrade 的测试方式 | 测试与桩只走回环：`LSM_UPGRADE_API` 可指向任意 `http://127.0.0.1:<port>/...` 桩（沿用 Q32 的覆盖入口，不为测试另开开关或 `#[cfg(test)]` 分支）；桩用标准库 `std::net::TcpListener` 手写 HTTP 响应，不引 `httpmock`/`wiremock` 等测试依赖；集成测试一律先把已构建的 `lsm` 复制到临时目录再运行副本，`current_exe()` 指向副本，绝不替换 `target/` 下正在使用的二进制；真实 GitHub 请求只在发布前手工验收跑一次（`upgrade --check`），不进 `cargo test`、不进 `scripts/acceptance.sh`；不推翻 Q39（仍不做 CI 矩阵与 Linux/macOS 实测） |
| Q43 | v0.4.0 范围 | 只做四件事：多 name 的 `copy`/`remove`、`search`、`push`（project→global）、not-found hint 指向 `search`；不做日志功能、不在 not-found 时自动执行 search、不推翻 Q39（仍不做 CI 矩阵与 Linux/macOS 实测）；四条命令面先在本表之后改 `impl/cli.md` §2.1 为 `planned`，实现后同提交转 `implemented` |
| Q44 | 多 name 批量语义 | `copy <name>...` 与 `remove <name>... <scope>`；`--all` 与 name 互斥（同现退 2）；所有 name 在 `cli::parse` 全量预校验，任一非法即退 2 且一条都不执行；逐条复用单条流程，单条失败不中断：目标已存在且无 `--force` 算**跳过**，源非法或 I/O 失败算**失败**；结束打印成功/跳过/失败计数，任一失败退 1，只有跳过或全部成功退 0；重复 name 去重（保留首次出现）；单 name 输出保持现状，多 name 才打印汇总 |
| Q45 | `search` 语义 | `search <query> <scope> [--json]`，scope 必填且只搜一个作用域；匹配 name 与 description 的大小写不敏感子串，query 去首尾空白后不得为空（空串退 2）；文本输出复用 list 表格（非法项照常标 `[invalid]`），JSON 复用 list 的字段；结果按 name 排序；无匹配退 0，文本模式打印 `No skills matched ...`，JSON 模式输出 `[]` |
| Q46 | `push` 语义 | 新增 `push <name>... [--all] [--force]`：project（cwd）→ global；复用 copy 的单条校验、同路径/自包含检查、原子安装与 `.git` 警告；`--all` 扫描项目作用域并按 Q40 的成功/跳过/失败语义汇总；输出 `pushed <name> -> <global path>`；不加短选项、不参与归一化 |
| Q47 | not-found 提示 | `copy`/`remove`/`push` 找不到 skill 时只改 hint 指向 `lsm search`（copy/remove 用对应 scope，push 用 `lsm list project`），**不自动执行 search**；输出仍是错误体、退出码仍为 1 |
| Q48 | `copy` 改名为 `pull` | v0.5.0 起 `lsm pull <name>...` / `pull --all` 取代 `copy`；**不保留 `copy` 别名**，**移除 `-c/--copy` 短选项**（`-p` 仍表示 project，`pull` 不加短选项）；不做 “已改名” 特殊拦截，`copy` / `-c` 按未知子命令 / 未知参数退 2；breaking change 按 0.5.0 发布，迁移说明置顶 release notes |
| Q49 | `list` 条目之间空行 | 文本表格在**相邻条目之间插入一个空行**，折叠换行后的 description 不会和下一个条目的 name 粘在一起；表头与第一条之间、最后一条之后都不加空行，stdout 仍以单个换行收尾；空列表与单条输出完全不变；`search` 复用同一渲染器，因此同步生效；JSON 输出不受影响 |
| Q50 | name 前缀解析 | `pull`/`push`/`remove` 的 `<name>` 按「精确优先 → 唯一前缀」解析：先找同名目录，未命中再扫对应作用域目录名取前缀命中；恰好 1 个按全名执行，≥2 个为运行时错误（退 1）并列出候选，0 个沿用 not-found（退 1）；只做前缀、不做子串，不自动执行 `search`（不推翻 Q47）；格式校验仍在 `cli::parse`（退 2），解析在命令层；多 name 时单条歧义/缺失按失败记账并继续（沿用 Q44）；`--all` 不受影响 |
| Q51 | 作用域默认 `project` | `add`/`list`/`search`/`remove` 的 `<SCOPE>` 可省略，省略时为 `project`；`remove` 的最后一个参数恰好是 `global`/`project` 时按作用域处理，否则全部按 name（默认 `project`），技能名本身是 scope 词时必须显式写作用域；缩减版 `-a`/`-l`/`-r` 无 `-g`/`-p` 时同样补 `project`，`MissingScope` 用法错误与 `UsageMissingScope` 文案移除；`pull`/`push` 方向固定，不受影响 |
| Q52 | `search` 字段过滤 | 新增 `--name` / `--description` 两个布尔开关，无短选项；默认与两个 flag 同给都按 name ∪ description 匹配，`--name` 只看名称、`--description` 只看描述；匹配规则、排序、空 query 退 2、JSON 输出均不变；非法项的 description 视为空串，`--description` 不命中它们 |
| Q53 | 分组存放与成员规则 | 分组定义存放在全局 `config.toml` 的 `[groups]` 表（组名 → skill 名数组），不新增项目级配置；组名与成员名共用 skill 名校验，成员**只校验格式、不要求已安装**；成员重复按首次出现去重；`[groups]` 惰性加载，只有 `pull --group` 与 `group *` 读取，写坏不影响其他命令 |
| Q54 | 分组的读取与排序 | `group list` 按组名字典序输出，成员保留定义顺序；空组显示 `(empty)`；无组输出 `No groups found.`；`--json` 输出 `[{"name":...,"skills":[...]}]`；`[groups]` 结构非法退 1，hint 指向修复或删除配置文件 |
| Q55 | 分组命令面与 `pull --group` | 新增 `group list` / `group set` / `group add` / `group remove`；`pull --group <GROUP>...` 可重复，与 `<NAME>...`、`--all` 三者互斥（同给任两项退 2）；展开后复用 Q44 的批量语义，组不存在/组为空/成员缺失都按该条失败继续，即使只展开出 1 个也打印成功/跳过/失败计数，任一失败退 1 |
| Q56 | 分组的破坏性规则 | `group set` 创建不存在的组不需要 `--force`，覆盖已存在的组必须 `--force`，否则退 1；`group add` 组不存在时自动创建、已有成员静默去重；`group remove <GROUP>...` 批量删除且不需要 `--force`，不存在的组逐条报错、其余继续，任一失败退 1；`set` 不给成员退 2 |
| Q57 | v0.7.0 范围 | 只做分组：`[groups]` 持久化、`pull --group`、4 个 `group` 子命令与 `group list --json`；不把分组扩展到 `push`/`remove`/`list`/`search`，不做分组嵌套与通配；版本按 minor 升到 0.7.0 |
| Q58 | 分组成员校验与模糊匹配 | 组名规则不变，仍拒绝大写；大写输入在格式错误 hint 里给出全小写建议（如 `Predevelop` → `predevelop`）。`group set` / `group add` 写入前要求每个成员存在于 **global**：先按「精确 → 唯一前缀」解析并写入全名，缺失或多候选则整组不写盘，逐条打印错误后汇总退 1；组名本身仍精确匹配，不做前缀（避免 `--force` 误覆盖）。`group remove` 与 `pull --group` 的组名支持唯一前缀，多候选/缺失按该条失败并列出候选。候选提示用子串匹配加手写编辑距离，最多 3 条，不引入新依赖。直接手改 `config.toml` 写入不存在的成员仍允许，由 `pull --group` 在拉取时逐条失败。版本按 patch 升到 0.7.1 |

| Q59 | upgrade 过程反馈 | 新增 `upgrade --verbose` 与 `upgrade --progress` 两个布尔开关（无短选项，默认关闭，不加参数时行为与文案完全不变）。`--verbose` 把 `checking for the latest release...` / `downloading <资产名> (<大小>)...` / `verifying downloaded binary...` / `replacing <exe 路径>...` 逐行写到 **stderr**，无前缀；`--progress` 隐含 `--verbose`，并在下载阶段用单行 `\r` 重绘 `downloading <资产名> [##########----------]  47%  5.7/12.3 MB`。进度条固定 20 列纯 ASCII，不探测终端宽度、不探测是否 TTY、不做颜色；响应没有 `Content-Length` 时只显示已下载字节，不画条、不报百分比；整数百分比变化才重绘，未知总长每 256 KB 重绘一次，结束补换行；失败时先结束进度行再打错误体。过程输出只进 stderr，stdout 仍只有一行结果；不推翻 Q30–Q36，仍不做 `--json`、不做自动/后台检查更新 |

## 维护规则

1. 新决策追加到表尾，编号连续，不复用旧编号。
2. 先改本表，再改主题文档中的规则；两边必须同一提交。
3. 主题文档引用决策编号，不复制决策背景。
4. 决策被推翻时保留原行并追加新行说明替代关系，不直接删除历史。
