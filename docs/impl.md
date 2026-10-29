# LocalSkillManager 实现方案（索引）

> 对应需求：`docs/Requirements.md`
> 状态：v0.1.1 已实现（Windows 实测；Linux/macOS 未实测）；`upgrade` 规则已定、代码未实现
> 最后更新：2026-10-29
>
> 本文只保留架构、跨文档约定与交付流程。具体规则按主题拆分，避免单文件过长与规则重复。命令与参数面的唯一真源是 [cli.md](impl/cli.md) §2.1 的状态表。

## 1. 文档地图

| 文档 | 内容 | 何时读 |
|---|---|---|
| [impl/storage.md](impl/storage.md) | 作用域与路径、SKILL.md、add/list/copy、文件系统 | 实现核心功能 |
| [impl/cli.md](impl/cli.md) | i18n、CLI、help、输出与错误 | 实现命令行与输出 |
| [impl/config.md](impl/config.md) | 配置文件路径、格式、语言持久化、`config` 命令 | 实现持久化语言 |
| [impl/quality.md](impl/quality.md) | 测试计划、README、许可证、发布 | 测试与交付 |
| [impl/upgrade.md](impl/upgrade.md) | 自更新：命令、版本比较、下载校验、原地替换 | 实现或排查 upgrade |
| [release.md](release.md) | Release 资产命名、tag 约定、发布与改名流程 | 发布与改资产名 |
| [decisions.md](decisions.md) | Q1–Q32 决策记录 | 查决策依据 |
| [Requirements.md](Requirements.md) | 原始需求 | 对齐范围 |

推荐阅读顺序：本文 → `storage.md` → `cli.md` → `config.md` → `quality.md` → `upgrade.md`。决策有疑问时查 `decisions.md`。

## 2. 目标与范围

`lsm` 是一个跨平台 CLI，用于把 Agent Skill 安装到全局目录或当前项目目录，并能把全局 skill 复制到项目。

核心能力：

1. `add`：把单个 `.md` 文件或完整 skill 目录安装到 `global` / `project`。
2. `list`：列出指定作用域下已安装的 skill。
3. `copy`：把全局 skill 复制到当前项目的 `.agents/skills/`。
4. `upgrade`：从 GitHub Releases 取最新版本并原地替换 lsm 自身二进制，**不改变 exe 的路径与文件名**。

非目标（本期不做）：

- 不做 skill 的删除、卸载；不做自动/后台检查更新（`upgrade` 只在显式调用时联网）。
- 不做符号链接（Windows 上不可靠），一律物理复制。
- 不做从 project 反向复制到 global。
- 不代理任何 agent 私有目录（如 `.claude/skills`），只维护 `.agents/skills` 这一份事实来源。
- 不做 GitHub Actions 发布矩阵（第二阶段）。

## 3. 技术选型

| 项 | 选择 | 理由 |
|---|---|---|
| 语言 | Rust（edition 2021，当前 stable） | 单二进制、无运行时、三平台一致 |
| 项目结构 | 单 crate + `lib.rs` + 分层模块 | 规模不需要 workspace，核心可复用、易测试 |
| CLI 解析 | `clap` 4（builder API） | 需要按语言动态生成 help，derive 无法满足 |
| 错误处理 | `anyhow` | 上下文链式错误，统一出口 |
| 序列化 | `serde` + `serde_yaml_ng` | 解析 SKILL.md frontmatter |
| JSON 输出 | `serde_json` | `list --json` |
| HTTP 客户端 | `ureq`（rustls） | `upgrade` 下载 release；同步、纯 Rust，避免 reqwest 量级的依赖树 |
| 主目录 | `dirs` 7 | Windows/Linux/macOS 家目录 |
| 递归复制 | 自研（`std::fs`） | 约 40 行，避免额外依赖 |
| 国际化 | 自研消息表 | 仅 en/zh，不引入 fluent 等重型方案 |
| 配置持久化 | `toml_edit` | 手改 `config.toml` 时保留注释与未知键 |
| 许可证 | MIT | 简洁、宽松，适合小工具 |
| 测试 | `assert_cmd`、`predicates`、`tempfile` | 端到端验证真实二进制 |

> MSRV 不在 `Cargo.toml` 中写死；以当前 stable 工具链构建（开发环境为 Rust 1.97.1），避免声明与实际依赖要求不一致。

`Cargo.toml`：

```toml
[package]
name = "local-skill-manager"
version = "0.1.1"
edition = "2021"
description = "Install and copy agent skills under .agents/skills"
license = "MIT"

[lib]
name = "lsm"
path = "src/lib.rs"

[[bin]]
name = "lsm"
path = "src/main.rs"

[dependencies]
anyhow = "1"
clap = "4"
dirs = "7"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml_ng = "0.10"
toml_edit = "0.22"
ureq = "3"              # upgrade：下载 release（默认 rustls 后端）

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
tempfile = "3"
```

## 4. 模块划分

单 crate，不拆 workspace。`lib.rs` 暴露核心，`main.rs` 只做接线；未来出现第二个前端时再把 `domain` + `infra` 抽成 `lsm-core`。

```
LocalSkillManager/
├── Cargo.toml
├── LICENSE                # MIT
├── .gitignore             # /target
├── README.md              # 英文，主文件
├── README.zh-CN.md        # 中文
├── docs/
│   ├── Requirements.md
│   ├── impl.md            # 本文件
│   ├── decisions.md
│   └── impl/
│       ├── cli.md
│       ├── storage.md
│       ├── config.md
│       └── quality.md
├── src/
│   ├── main.rs            # 仅接线：argv → lib::run → ExitCode
│   ├── lib.rs             # 导出模块 + run() + Ctx 构造
│   ├── error/
│   │   ├── mod.rs         # AppError / ErrorKind / exit_code（无文案、无渲染）
│   │   └── mapping.rs     # domain/infra 错误 → AppError（含 hint）
│   ├── i18n/
│   │   ├── mod.rs         # MsgKey、tr/tr_args（Lang 来自 domain）
│   │   ├── en.rs          # 英文消息
│   │   └── zh.rs          # 中文消息
│   ├── domain/            # 纯数据与规则，零 I/O、零文案
│   │   ├── mod.rs
│   │   ├── lang.rs        # Lang 枚举 + parse（唯一归一处）
│   │   ├── name.rs        # SkillName 校验（格式/长度/保留名）
│   │   ├── meta.rs        # SkillMeta 校验
│   │   ├── scope.rs       # Scope 枚举
│   │   ├── config.rs      # ConfigKey 枚举
│   │   ├── version.rs     # Version：升级用的版本解析与比较
│   │   ├── release.rs     # Release / Asset / 平台资产名选择
│   │   ├── action.rs      # Action：Add / List / Copy / Upgrade / Config
│   │   └── entry.rs       # SkillEntry：list 展示数据
│   ├── infra/             # 所有文件系统与环境访问
│   │   ├── mod.rs
│   │   ├── env.rs         # 环境快照：LSM_HOME / LSM_CONFIG_DIR / LSM_LANG / LSM_UPGRADE_API / current_exe
│   │   ├── paths.rs       # 纯函数：skills_dir(&Env, ...)、config_file(&Env)
│   │   ├── config.rs      # config.toml 读写与校验
│   │   ├── frontmatter.rs # 解析 SKILL.md
│   │   ├── fsops.rs       # 递归复制、原子安装、覆盖、自包含检测、原地替换自身
│   │   ├── net.rs         # 取最新 release、下载资产（唯一网络访问点）
│   │   └── scan.rs        # 扫描 skills 目录，产出 SkillEntry
│   ├── cli/
│   │   ├── mod.rs         # parse(ctx, argv) -> Action
│   │   ├── lang.rs        # --lang 预扫描 + 语言决策表
│   │   ├── normalize.rs   # 短选项归一化（纯函数）
│   │   ├── spec.rs        # CLI 元数据 + clap builder
│   │   └── help.rs        # 本地化 help 渲染
│   ├── output/            # 唯一排版出口
│   │   ├── mod.rs         # 成功/警告/信息前缀与流向
│   │   ├── error.rs       # render(&AppError, Lang)
│   │   └── list.rs        # 文本表格 + JSON
│   └── commands/
│       ├── mod.rs         # dispatch(&Ctx, Action)
│       ├── add.rs
│       ├── list.rs
│       ├── copy.rs
│       ├── upgrade.rs     # 自更新编排：查询 → 校验 → 替换 → 输出
│       └── config.rs
└── tests/
    ├── cli.rs
    └── config.rs
```

### 4.1 模块职责

| 模块 | 职责 | 关键接口 |
|---|---|---|
| `domain::lang` | 语言枚举与归一化（唯一归一处） | `Lang::{En, Zh}`、`Lang::parse(&str) -> Result<Lang, LangError>` |
| `domain::name` | name 格式、长度、Windows 保留名 | `SkillName::parse(&str) -> Result<Self, NameError>` |
| `domain::meta` | `name` / `description` 字段校验 | `SkillMeta::validate() -> Result<(), MetaError>` |
| `domain::scope` | 作用域枚举 | `Scope::{Global, Project}` |
| `domain::config` | 配置键枚举 | `ConfigKey::Lang` |
| `domain::version` | 升级版本解析与比较 | `Version::parse(&str)`、`Ord` |
| `domain::release` | release 元数据与平台资产名 | `Release::select_asset(Platform)`、`Platform::current()` |
| `domain::action` | 解析后的用户意图 | `Action::{Add, List, Copy, Upgrade, Config}` |
| `domain::entry` | list 的展示数据 | `SkillEntry`（name/description/path/valid/error） |
| `infra::env` | 进程环境快照（唯一读环境点） | `Env::read() -> Env`、`Env::{home, config_dir, lang, upgrade_api, exe}` |
| `infra::paths` | 纯函数路径解析 | `skills_dir(&Env, Scope, &Path) -> Result<PathBuf, PathError>`、`config_file(&Env) -> Result<PathBuf, PathError>` |
| `infra::config` | 读写并校验 `config.toml` | `load() -> Result<Option<Lang>, ConfigError>`、`set_lang(Lang)` |
| `infra::frontmatter` | 读取并解析 SKILL.md | `parse_skill_md(&Path) -> Result<SkillMeta, FrontmatterError>` |
| `infra::fsops` | 复制、原子安装、覆盖、自包含、原地替换自身 | `install_skill_dir/file`、`contains_path`、`replace_self`（错误为 `FsError`） |
| `infra::net` | 取 release 元数据、下载资产（唯一网络点） | `fetch_latest_release(&str) -> Result<Release, NetError>`、`download(&str, &Path)` |
| `infra::scan` | 扫描目录、标记非法项 | `scan_skills(&Path) -> Result<Vec<SkillEntry>, FsError>` |
| `cli::lang` | `--lang` 预扫描 + 语言决策表 | `prescan(&[OsString]) -> Option<String>`、`decide(cli, env) -> Decision` |
| `cli::normalize` | `-a/-l/-c/-g/-p` 归一化 | `normalize(Vec<OsString>) -> Result<Vec<OsString>, NormalizeError>` |
| `cli::spec` / `help` | 双语 CLI 与 help | `build(Lang) -> Command`、`localize(&str, Lang) -> String` |
| `error::mapping` | 各层错误 → `AppError` | `impl From<NameError/MetaError/...> for AppError` |
| `output::error` | 错误体渲染 | `render(&AppError, Lang) -> String` |
| `output::list` | 文本表格 / JSON | `render_text`、`render_json` |
| `commands::*` | 业务流程编排 | `dispatch(&Ctx, Action) -> Result<(), AppError>` |

### 4.2 依赖方向

```
lib::run（main 只调用它；唯一组装 Ctx 的地方）
 ├─→ cli ─────────→ domain, i18n, error
 ├─→ infra ───────→ domain
 ├─→ commands ────→ domain, infra, output, error
 └─→ output ──────→ domain, i18n, error

error::mapping ──→ domain, infra, error, i18n   # 唯一的"各层错误 → AppError"适配点
i18n ──────────→ domain（只取 `Lang`）
domain ────────→ 无内部依赖（零 I/O、零文案）
infra ─────────→ domain（`net` 另外依赖 ureq，`env` 另外依赖 dirs）
```

硬规则：

- `domain` 零 I/O、零 clap、零打印、零消息表：错误用自身枚举（`NameError` 等）表达，不引用 `MsgKey`。
- `infra` 不构造 `AppError`、不写文案：错误用自身枚举（`FsError`、`FrontmatterError`、`ConfigError`、`NetError`），路径等上下文放在变体里。
- `infra::env` 是**唯一**读取进程环境的位置；`paths` 收 `&Env` 后是纯函数，单测不需要改进程环境。每次 `Env::read` 额外快照 `LSM_UPGRADE_API` 与 `current_exe()`。
- `infra::net` 是**唯一**访问网络的位置；`cli`、`domain`、`output` 不出现 HTTP，`commands` 只调用 `infra::net`。
- `i18n` 只依赖 `domain`，不读环境、不读文件、不依赖 `error`；`LangError` 由 `domain::lang` 定义。
- `AppError` 只在边界（`cli`、`commands`、`lib::run`）构造，`From` 映射集中在 `error::mapping`，hint 在那里配。
- `output` 是唯一排版出口（成功/警告/信息/错误/列表），`error::mod` 只存数据与 `exit_code()`。
- `commands` 不直接调用 `std::fs`，只编排 `infra` 与 `output`；入口签名统一为 `dispatch(&Ctx, Action)`。
- `cli` 不访问文件系统与环境：`--lang` 预扫描只看 `argv`，环境值由 `lib::run` 从 `infra::env` 取好后传入。
- 所有用户可见文案来自 `i18n`，业务代码禁止硬编码。
- 模块对外接口用 `pub(crate)`，`lib.rs` 只导出未来确实需要复用的类型。

`Ctx` 定义在 `lib.rs`（组合根），字段固定为四项：

```rust
pub struct Ctx {
    pub env: infra::env::Env,   // 进程环境快照
    pub lang: domain::lang::Lang,
    pub cwd: PathBuf,           // 本次调用的工作目录，由 lib::run 读一次
    pub lang_explicit: bool,    // 语言是否来自 --lang / LSM_LANG
}
```

`lang_explicit` 只服务于 `config set`：显式指定语言时用该语言确认，否则用刚写入的值确认（见 [config.md](impl/config.md)「`config` 命令」）。

### 4.3 数据流

- **add**：`cli` → `Action::Add` → `paths::skills_dir(&ctx.env, ...)` → `frontmatter::parse_skill_md` → `meta::validate` → `fsops::install_skill_*` → `output`。
- **list**：`cli` → `Action::List` → `paths::skills_dir` → `scan::scan_skills` → `output::list`（text/json）。
- **copy**：`cli`（name 已由 value_parser 校验）→ `paths` → 校验源目录 → `fsops::install_skill_dir` → `output`。
- **upgrade**：`cli` → `Action::Upgrade` → `env.exe` + 编译期版本 → `net::fetch_latest_release` → `release::select_asset` →（`--check` 到此结束）→ `net::download` → 冒烟校验 → `fsops::replace_self` → `output`。
- **语言解析**：`lib::run` → `infra::env::read()` → `cli::lang::decide(--lang, env.lang)` →（`Decision::NeedFile` 时）`infra::config::load()` → `Lang` → `cli::spec::build(lang)`。
- **config get**：`cli` → `Action::Config(Get(Lang))` → `infra::config::load` → 裸值到 stdout（缺省 `en`）。
- **config set**：`cli` → `Action::Config(Set(Lang, Lang))` → `infra::config::set_lang` → 本地化成功提示。
- **错误出口**：任一层返回自身错误 → `error::mapping` 转 `AppError` → `output::error::render(&err, ctx.lang)` → stderr。

## 5. 跨文档关键约束

这里只放一句话摘要和链接；完整规则在对应主题文档中定义，禁止在此复制细节。

| 约束 | 摘要 | 详情 |
|---|---|---|
| project 作用域 | 固定 cwd，子目录执行会装到子目录 | [storage.md](impl/storage.md)「作用域与路径」 |
| skill 身份 | 目录名是唯一身份，必须与 frontmatter `name` 一致 | [storage.md](impl/storage.md)「SKILL.md 规范」 |
| 复制策略 | 不做排除；`.git` 复制并告警；符号链接报错 | [storage.md](impl/storage.md)「各命令实现」「文件系统细节」 |
| 原子覆盖 | 备份 → 安装 → 删备份，失败回滚 | [storage.md](impl/storage.md)「文件系统细节」 |
| 语言选择 | `--lang` > `LSM_LANG` > `config.toml` > `en`，惰性读取 | [config.md](impl/config.md)「加载与优先级」 |
| 语言持久化 | `config.toml` + `config get/set lang`，坏配置退 1 | [config.md](impl/config.md)「配置文件」「`config` 命令」 |
| 参数与 help | 仅分离短选项；无子命令 help 到 stderr + 2；命令面真源见 cli.md §2.1 | [cli.md](impl/cli.md)「CLI 设计」 |
| 错误模型 | 结构化 `AppError`，`anyhow` 仅作 source | [cli.md](impl/cli.md)「输出、错误与退出码」 |
| 分层边界 | `domain`/`infra` 自身错误枚举，映射与渲染在边界；`Ctx` 承载 env/lang/cwd | [impl.md](impl.md)「模块职责」「依赖方向」 |
| 自更新 | 原地替换，exe 路径与文件名不变；备份只用临时名 | [upgrade.md](impl/upgrade.md)「原地替换与回滚」 |
| 测试与交付 | 用例数不写死，以 `cargo test` 输出为准（`upgrade` 的 8 条在 quality.md 标记 `[planned]`）；v1 不配 CI | [quality.md](impl/quality.md)「测试计划」「发布」 |

## 6. 文档维护规则

1. **单一事实来源**：每条规则只在主题文档完整定义一次；索引只放摘要和链接。
2. **决策先行**：新增或推翻决策时，先改 [decisions.md](decisions.md) 并分配连续编号，再改主题文档，同一提交完成。
3. **禁用章节号引用**：跨文件引用使用「文件名 + 章节名」，避免拆分/重排后链接失效。
4. **链接检查**：提交前检查所有相对路径存在，且没有指向已删除章节的描述。
5. **README 与文档分离**：面向用户的 README 不复制实现细节，只链接命令与行为。

## 7. 实现顺序（TDD）

文档拆分已在本次完成，不计入代码提交。先 `git init`，每个切片严格"红 → 绿 → 重构"，测试先行，完成后原子提交：

| # | 切片 | 提交信息 |
|---|---|---|
| 1 | 脚手架（Cargo、`lib.rs`、`main.rs`、测试依赖） | `chore: scaffold cargo project` |
| 2 | `domain`（lang / name / meta / scope / config / action） | `feat(domain): add skill name and metadata rules` |
| 3 | `infra::env` + `infra::paths` 环境快照与路径解析 | `feat(paths): resolve global and project skill dirs` |
| 4 | `infra::frontmatter` | `feat(frontmatter): parse SKILL.md` |
| 5 | `infra::fsops` + `infra::scan` | `feat(fs): add atomic install and skill scanning` |
| 6 | `i18n` 消息表（error / help / warning 键定稿） | `feat(i18n): add en/zh message catalog` |
| 7 | `cli` + `error::mapping` + `output::error`（normalize / lang / spec / help） | `feat(cli): add localized parser and short flags` |
| 8 | `infra::config` + `commands::config` + 配置消息键 | `feat(config): persist language preference` |
| 9 | `commands::add` | `feat(add): install markdown or directory skills` |
| 10 | `commands::list` + `output::list` | `feat(list): add text and json listing` |
| 11 | `commands::copy` | `feat(copy): copy global skill to project` |
| 12 | 双语 README | `docs: add bilingual readme` |
| 13 | 决策 Q30–Q32 + `impl/upgrade.md` + 索引同步（已完成） | `docs: decide self-upgrade scope` |
| 14 | `domain::version` + `domain::release`（版本比较、资产选择） | `feat(domain): add version comparison and asset selection` |
| 15 | `infra::net`（元数据 + 下载，桩服务器用例） | `feat(net): fetch latest release over https` |
| 16 | `infra::fsops::replace_self`（原地替换、回滚、备份清理） | `feat(fs): replace running binary atomically` |
| 17 | `cli`（`Action::Upgrade`、`--check` / `--force`、i18n key） | `feat(cli): add upgrade command` |
| 18 | `commands::upgrade` + 集成用例 + 文档收尾 | `feat(upgrade): self-update from github releases` |

> i18n 排在 CLI 之前而非 `domain` 之后：消息 key 由 help / 错误 / 警告文案决定，等 `MetaError`、`FrontmatterError`、`FsError`、`ConfigError` 都定型后再定稿，可避免消息表返工；各命令的成功/警告提示在对应命令切片内增量追加 key。
>
> 第 13 片是文档先行：规则、文案与验收标准先落地，第 14–18 片按红 → 绿实现，每片一个原子提交。

收尾检查：`cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test` 全绿。

## 8. 验收标准

验收范围：第 1 条的三平台只在 Windows 实测，其余条目均在本机通过。

- [x] `lsm --version`、`lsm --help` 正常（Windows 实测；Linux/macOS 未实测）。
- [x] 需求中的前 5 条命令、2 条 `config` 命令与 5 个短选项全部可用（第 6 条 `upgrade` 见 8.1）。
- [x] `.md` 与目录两种输入都能安装；目录不做任何排除，`.git` 完整复制并告警。
- [x] 目标位于源目录内时硬报错，不发生递归复制。
- [x] 非法 SKILL.md 被拦截：格式、64 / 1024 长度限制全部生效。
- [x] 重复安装默认失败，`--force` 原子覆盖且不残留临时目录。
- [x] `copy` 能把全局 skill 复制到当前项目，冲突处理与 add 一致。
- [x] `list` 文本与 JSON 输出稳定；空 JSON 为 `[]`；非法项不导致崩溃。
- [x] help、错误、成功提示支持 en/zh，`--lang` / `LSM_LANG` / 配置文件 / 默认值的四级优先级正确。
- [x] `lsm config set lang zh` 后新进程默认输出中文；`get` 输出裸值且缺省为 `en`。
- [x] 配置文件损坏时 `--lang` / `LSM_LANG` 仍可用，裸调用退 1 并给出路径；配置目录可用 `LSM_CONFIG_DIR` 重定向。
- [x] `README.md`（英文）与 `README.zh-CN.md`（中文）互相链接。
- [x] 目录名与 frontmatter `name` 不一致时，`list` 标 `[invalid]`，`copy` 拒绝。
- [x] `copy` 的 name 校验能阻止路径穿越（`..`、`/`、`\`）与 Windows 保留设备名。
- [x] 无子命令 → help 到 stderr + 退出码 2；显式 `--help` → stdout + 退出码 0。
- [x] `main.rs` 仅做接线；`domain` 零 I/O、零文案；`domain`/`infra` 不构造 `AppError`；`i18n` 不读环境与文件；`paths` 不读进程环境；`commands` 不直接调用 `std::fs`（代码审查）。
- [x] 模块树与本文"模块划分"一致，`lib.rs` 暴露核心逻辑。
- [x] `LICENSE` 为 MIT，`Cargo.toml` 与两份 README 均标注。
- [x] `docs/impl.md`、`docs/impl/*.md`、`docs/decisions.md` 的相对链接全部有效，规则无重复。
- [x] `cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test` 全绿。

### 8.1 upgrade（规则已定，代码未实现）

- [ ] `lsm upgrade --check` 在「已最新」与「有新版本」两种情况下都退出 0，且不写盘。
- [ ] `lsm upgrade` 成功后 exe 的路径与文件名不变，`.old-*` 不残留；失败路径上原 exe 仍可执行。
- [ ] 当前平台无资产、网络失败、限流（403/429）、目录不可写均退出 1 并带 hint。
- [ ] `upgrade` 的 en/zh 文案齐全，`upgrade --help` 两种语言正常，未知参数退出 2。

