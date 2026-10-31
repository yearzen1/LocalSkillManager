# 决策记录

> 上级：[实现方案索引](impl.md) ｜ 需求：[Requirements.md](Requirements.md)
>
> 本文是 32 项设计决策的唯一事实来源。新增决策时按顺序编号（Q33、Q34…）追加，并同步更新对应主题文档；主题文档只保留规则本身，不复制本表。

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

## 维护规则

1. 新决策追加到表尾，编号连续，不复用旧编号。
2. 先改本表，再改主题文档中的规则；两边必须同一提交。
3. 主题文档引用决策编号，不复制决策背景。
4. 决策被推翻时保留原行并追加新行说明替代关系，不直接删除历史。
