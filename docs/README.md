# lsm 文档入口

按 Diátaxis 四象限组织：先问自己此刻要做什么，再选一篇。每篇只服务一种需求，操作步骤与背景解释不混装。

## 我想学：跟着做一遍

- [入门教程：安装 lsm 并拉起第一个 skill 组](tutorials/getting-started.md) —— 从安装、创建 skill、安装到全局，到定义分组并整组拉取。

## 我想做：完成一个具体任务

- [如何管理 skill 分组](how-to/manage-skill-groups.md) —— 创建、追加、查看、用组拉取、删除分组。
- [配置或分组损坏时怎么办](how-to/fix-config-problems.md) —— 坏 TOML、非法 `lang`、非法 `[groups]`、配置目录不可用。

## 我想查：找准确的事实

- [命令索引](reference/cli.md) —— 命令列表、`--help` 入口、全局选项与退出码。
- [配置文件参考](reference/config.md) —— `config.toml` 的路径、`lang` 与 `[groups]` 结构。
- [SKILL.md 参考](reference/skill-md.md) —— frontmatter 字段、校验规则与目录名约束。

## 我想懂：理解设计与取舍

- [核心概念](explanation/concepts.md) —— 作用域、目录名身份、pull/push 方向，以及分组为什么放在全局配置。

## 贡献者与发布

这些文档面向实现与发布，不是上手入口：

- [实现方案索引](impl.md) —— 架构、模块划分、数据流。
- [CLI 与输出](impl/cli.md) —— 命令与参数的唯一真源、i18n、错误与退出码。
- [存储与命令](impl/storage.md) —— 路径、SKILL.md、add/list/pull/remove 的实现规则。
- [配置与持久化](impl/config.md) —— 配置文件加载与 `config` / `group` 命令规则。
- [测试与交付](impl/quality.md) —— 测试计划、README 约定、发布。
- [自更新](impl/upgrade.md) —— `upgrade` 的下载、校验与原地替换。
- [决策记录](decisions.md) —— Q1 起的全部设计决策。
- [发布规则](release.md) 与 [Release notes 模板](release-notes.md)。

## 文档约定

| 类型 | 回答的问题 | 不写什么 |
|---|---|---|
| 教程 | 我该怎么开始？ | 完整参数表、原理长文、多个分支 |
| 操作指南 | 我该怎么完成 X？ | 从零教学、长篇背景 |
| 参考 | X 是什么？有哪些参数？ | 主观建议、教程式叙述 |
| 解释 | 为什么会这样？ | 操作步骤、参数表 |

命令与参数的唯一定义在 [`impl/cli.md`](impl/cli.md) §2.1；用户参考页只做索引与链接，不维护第二份参数表。
