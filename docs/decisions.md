# 决策记录

> 上级：[实现方案索引](impl.md) ｜ 需求：[Requirements.md](Requirements.md)
>
> 本文是 29 项设计决策的唯一事实来源。新增决策时按顺序编号（Q30、Q31…）追加，并同步更新对应主题文档；主题文档只保留规则本身，不复制本表。

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

## 维护规则

1. 新决策追加到表尾，编号连续，不复用旧编号。
2. 先改本表，再改主题文档中的规则；两边必须同一提交。
3. 主题文档引用决策编号，不复制决策背景。
4. 决策被推翻时保留原行并追加新行说明替代关系，不直接删除历史。
