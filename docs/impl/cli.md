# CLI、国际化与输出

> 上级：[实现方案索引](../impl.md) ｜ 相关：[存储与命令](storage.md) ｜ [配置与持久化](config.md) ｜ [测试与交付](quality.md) ｜ [决策记录](../decisions.md)
>
> 本文拥有以下规则的完整定义：语言选择及其渲染语言、消息表、help、参数归一化、错误渲染与退出码。配置文件的路径、格式、加载时机与失效策略见 [config.md](config.md)。

## 1. 国际化（i18n）

### 1.1 语言选择

- 支持 `en`、`zh`；默认 `en`。`Lang` 定义在 `domain::lang`，`Lang::parse` 是唯一的归一化入口（`--lang`、`LSM_LANG`、配置文件共用）。
- 优先级：`--lang <en|zh>` > 环境变量 `LSM_LANG` > 配置文件 `lang` > 默认 `en`。
- 读取是**惰性**的：`cli::lang::decide` 先只吃命令行与环境两档，返回 `Decision::{Cli, Env, NeedFile}`；只有 `NeedFile` 时 `lib::run` 才调用 `infra::config::load()`。因此 `LSM_LANG`/`--lang` 生效时，坏掉的配置文件不会影响本次运行。
- 接受 `zh`、`zh-CN`、`en`、`en-US`，其余值报用法错误（退出码 2）；`config set` 落盘统一为 `zh`/`en`。
- `--lang` 可出现在任意位置，启动时先预扫描（支持 `--lang zh` 和 `--lang=zh`），再构建 clap Command，保证 `lsm --lang zh --help` 生效；多个 `--lang` 取最后一个。
- 错误/help 的渲染语言按 `--lang` > 合法 `LSM_LANG` > 已解析的配置值 > `en` 选取，详见 [config.md](config.md)「加载与优先级」。
- 配置文件路径、TOML 格式、损坏时的报错规则、`config get/set` 命令全部由 [config.md](config.md) 定义。

### 1.2 消息表

`src/i18n/mod.rs` 提供：

```rust
// src/domain/lang.rs —— 语言身份与归一化的唯一来源
pub enum Lang { En, Zh }
pub fn parse(raw: &str) -> Result<Lang, LangError>;

// src/i18n/mod.rs —— 只有消息表
pub enum MsgKey { /* ... */ }
pub fn tr(lang: Lang, key: MsgKey) -> &'static str;
pub fn tr_args(lang: Lang, key: MsgKey, args: &[&str]) -> String;
```

- `i18n` 只依赖 `domain`（取 `Lang`），不读环境、不读文件、不依赖 `error`；`LangError` 随 `Lang` 一起定义在 `domain::lang`。
- 环境读取由 `infra::env` 完成，`--lang` 预扫描与决策表由 `cli::lang` 完成，I/O 与命令衔接由 `lib::run` 完成；`i18n` 不再参与检测。
- `LangError` 由 `cli` 转成用法错误 `AppError`（退出码 2）。
- 文案按语言拆到 `i18n/en.rs`、`i18n/zh.rs`，`mod.rs` 只做分发。
- 所有 `error:` / `hint:` / 成功提示 / help 正文都从消息表取，禁止在业务代码里写死文案。
- 中文前缀用 `错误:` / `提示:`，英文用 `error:` / `hint:`。
- 版本号 `lsm 0.1.1`、JSON 字段名、JSON 内的 `error` 文案保持英文，不参与翻译。

### 1.3 help 与参数错误

- clap 用 builder API 构建，`about`、子命令 `about`、参数 `help`、`--lang` 说明全部按当前语言注入。
- clap 自带的结构性标题（`Usage:`、`Commands:`、`Options:`、`Arguments:`）与结尾提示，在渲染后按语言替换为 `用法:`、`命令:`、`选项:`、`参数:` 等；自带的 `-h/--help` 与 `-V/--version` 说明同样是英文，因此改为自建参数注入本地化文案。
- 参数解析错误按 `clap::error::ErrorKind` 映射常用类型：缺参数、非法值、未知参数、多余参数、缺子命令、非法子命令，并带上 clap 记录的出错参数名；未覆盖的类型回退英文原文。
- `DisplayHelp` / `DisplayVersion` 直接输出并退出 0。
- 实测 help（v0.1.1，与 `cli/spec.rs` 一致）：

```
$ lsm --lang zh --help
Local Skill Manager

用法:
  lsm <COMMAND>

命令:
  add      安装 skill 到 global 或 project
  list     列出已安装的 skill
  copy     把全局 skill 复制到当前项目
  config   查看或修改持久化配置

选项:
  --lang <LANG>  输出语言 [en, zh]
  -h, --help     打印帮助
  -V, --version  打印版本
```

- `planned`：`upgrade` 落地后，`命令:` 段应在 `config` 之前插入一行 `upgrade  更新 lsm 自身（不改文件名）`，其余不变。未实现前禁止把这一行写进上面的实测样例。
- 英文模式输出完全对应的英文版本。
- `-h` 与 `--help` 走同一套渲染；`lsm add --lang zh --help` 同样生效。

## 2. CLI 设计

### 2.1 完整版（命令与参数的唯一真源）

本表是**命令与参数面的唯一真源**，其它文档（两份 README、`quality.md` 的测试计划）只能引用，不得另行定义。

- 新增或修改命令的顺序固定：**先改本表状态列 → 再改 `cli/spec.rs` / `domain/action.rs` → 再改两份 README**。顺序颠倒即视为文档漂移。
- `implemented` 表示当前二进制支持；`planned` 表示规则已定、代码未实现。两份 README **不得描述任何 `planned` 条目**。
- 本表与二进制的对应关系由 `tests/cli.rs::cli_reference_table_matches_binary` 强制：`implemented` 行必须 `lsm <command> --help` 退出 0，`planned` 行必须被视为未知子命令（退出 2）。实现完成却忘记改状态列，测试会红。

| 命令 | 参数 | 状态 |
|---|---|---|
| `add` | `<SOURCE> <SCOPE> [--force]` | implemented |
| `list` | `<SCOPE> [--json]` | implemented |
| `copy` | `<NAME> [--force]` | implemented |
| `config get` | `lang` | implemented |
| `config set` | `lang <LANG>` | implemented |
| `upgrade` | `[--check] [--force]` | planned |

- `<SCOPE>`：`global` 或 `project`，大小写敏感。
- `<LANG>`：`en` 或 `zh`；接受 `zh-CN` / `en-US`，落盘统一为 `zh` / `en`。
- 全局选项（任意位置可用）：`--lang <LANG>`、`-h`/`--help`、`-V`/`--version`。
- `upgrade` 的行为细节见 [upgrade.md](upgrade.md)，实现前不进入 README。

### 2.2 缩减版

只支持**分离写法**，不支持 `-lg` 这类合并写法。

| 输入 | 归一化结果 |
|---|---|
| `lsm -a foo.md -g` | `lsm add foo.md global` |
| `lsm -a foo.md -p` | `lsm add foo.md project` |
| `lsm -a dir -g --force` | `lsm add dir global --force` |
| `lsm -l -g` | `lsm list global` |
| `lsm -l -p` | `lsm list project` |
| `lsm -l -g --json` | `lsm list global --json` |
| `lsm -c foo` | `lsm copy foo` |
| `lsm --add foo.md --global` | `lsm add foo.md global` |

归一化规则：

1. 识别动作标志 `-a/--add`、`-l/--list`、`-c/--copy`（`-a`/`-c` 吞掉下一个参数）。
2. 识别作用域标志 `-g/--global`、`-p/--project`，转换为位置参数。
3. 未出现任何动作标志时原样交给 clap，走完整版语法。
4. 以下情况直接报用法错误（退出码 2），并打印两种正确写法：
   - `-g` 与 `-p` 同时出现；
   - 多个动作标志同时出现；
   - `-a` / `-c` 缺少参数；
   - `add` / `list` 缺少作用域；
   - `-c` 与 `-g` / `-p` 同现；
   - 混用形式（如 `lsm add foo.md -g`）。
5. `-h`、`-V` 不参与归一化，交给 clap。
6. `config` 子命令不参与短选项归一化，永远原样交给 clap。
7. `upgrade`（planned）没有短选项，也不参与归一化：`lsm -u` 按未知参数退出 2；`upgrade` 与 `-g` / `-p` 同现视为未知参数，不解释为升级某个作用域。

### 2.3 clap 结构

- 直接用 builder API 构建 `clap::Command`，所有 about / 参数说明 / `--lang` 说明都从消息表按语言注入（`cli/spec.rs::build(lang)`）。
- 作用域参数用 `value_parser(["global", "project"])` 限定合法值，大小写敏感，错误时由 clap 报用法错误。
- `config` 子命令的 `get`/`set` 是嵌套子命令，键用 `value_parser(["lang"])`（v1 只有 `lang`），未知键由 clap 报用法错误。
- `upgrade`（planned）只有两个布尔 flag：`--check`、`--force`，不带位置参数；行为与文案见 [upgrade.md](upgrade.md)。
- clap 自带的 `-h` / `-V` 说明是英文，因此根命令与各子命令都 `disable_help_flag(true)` 后自建 `-h/--help`，根命令另外自建 `-V/--version`，说明文案来自消息表。
- 参数校验错误统一由 `cli::parse` 转成 `AppError`，在 `lib::run` 边界渲染。
- 代码落点：`cli/spec.rs`（构建 Command）、`cli/help.rs`（结构标题本地化）、`cli/lang.rs`（预扫描 + 决策表）、`cli/mod.rs`（解析为 `Action`）；`Action` 定义在 `domain/action.rs`，配置键定义在 `domain/config.rs`。

### 2.4 无参数与帮助行为

- `lsm --help` / `-h` / `lsm <command> --help`：本地化 help 输出到 **stdout**，退出码 0。
- `lsm --version` / `-V`：输出 `lsm 0.1.1` 到 stdout，退出码 0。
- `lsm` 无子命令，或只有全局参数（如 `lsm --lang zh`）：本地化 help 输出到 **stderr**，退出码 2。
- 嵌套子命令沿用同一规则：`lsm config` 无子命令 → help 到 stderr、退出码 2；`lsm config --help` → stdout、退出码 0。
- `lsm upgrade --help` 与 `lsm upgrade --check --help` 同样走 stdout、退出码 0，且不联网、不写盘。
- 实现方式：clap 解析到 `ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand` 时，按上述规则选择流；不要依赖 clap 默认行为。

## 3. 输出、错误与退出码

| 退出码 | 含义 | 例子 |
|---|---|---|
| 0 | 成功 | `list` 为空、`upgrade --check` 发现新版本、`upgrade` 已是最新 |
| 1 | 运行时错误 | 文件不存在、frontmatter 非法、目标冲突、配置文件损坏、`upgrade` 网络/资产/校验/替换失败 |
| 2 | 用法错误 | 参数缺失、`-g` 与 `-p` 互斥、非法 `--lang`、非法 `copy` name、非法 `config` 键/值、`upgrade` 未知参数、无子命令 |

- 正常结果、显式 help、version 走 stdout；错误、warning、无子命令时的 help 走 stderr。
- `upgrade` 的成功/警告/错误同样全部来自消息表；具体文案、`--check` 不写盘的约束与替换规则见 [upgrade.md](upgrade.md)「输出与退出码」。
- 显式 `--help` / `-h` / `--version` / `-V` → 退出码 0；无子命令 → 退出码 2。
- 错误格式：

  ```
  error: <问题>
  hint: <下一步动作>        # 可选
  ```

  中文：

  ```
  错误: <问题>
  提示: <下一步动作>
  ```

### 3.1 结构化错误

`src/error/mod.rs`：

```rust
pub enum ErrorKind { Usage, Runtime }

pub struct Msg {
    pub key: MsgKey,
    pub args: Vec<String>,
}

pub struct AppError {
    pub kind: ErrorKind,
    pub msg: Msg,
    pub hint: Option<Msg>,
    pub source: Option<anyhow::Error>,
}

impl AppError {
    pub fn exit_code(&self) -> u8; // Usage = 2, Runtime = 1
}
```

- `error` 只存数据与退出码，**不含渲染**；渲染唯一入口是 `output::error::render(&AppError, Lang) -> String`。
- `domain` / `infra` 不返回 `AppError`，各自返回窄错误枚举；`error/mapping.rs` 集中提供 `impl From<...> for AppError`，hint 在该处配置。
- 只有边界（`cli`、`commands`、`lib::run`）构造 `AppError`；业务层不打印。
- `anyhow::Error` 只作为底层 `source` 保留，不直接进入用户输出。
- 消息 key 与参数交给 `i18n` 渲染，`error` 不写任何文案。
- `main.rs` 只做接线：

  ```rust
  fn main() -> ExitCode {
      lsm::run(std::env::args_os().skip(1))
  }
  ```

- `lsm::run(args: impl Iterator<Item = OsString>) -> ExitCode`：只接参数迭代器（`main` 已 `skip(1)` 去掉程序名），`run` 自己不再读 `std::env::args()`，便于单测注入参数。

- 用法错误由归一化阶段 / clap 错误映射统一 `exit(2)`；`copy` 的 name 在 `cli::parse` 构造 `Action` 时用 `SkillName::parse` 校验，同样返回 2，且绝不先拼路径再校验。
- `--json` 模式下 stdout 只允许 JSON；warning 仍然只走 stderr。

## 4. 相关决策

- Q2 / Q10 / Q18：双语 README、消息表、中文 help。
- Q7 / Q14：短选项仅分离写法。
- Q20：无子命令时 help 到 stderr + 退出码 2。
- Q26：结构化 `AppError`。
- Q27 / Q28：语言持久化、配置加载与失效策略（见 [config.md](config.md)）。
- Q29：分层收敛与错误映射（错误枚举在 `domain`/`infra`，`AppError` 只在边界构造，渲染归 `output`）。
- Q30 / Q31 / Q32：`upgrade` 的命令形态、原地替换（文件名不变）、网络与资产契约（见 [upgrade.md](upgrade.md)）。

完整决策见 [decisions.md](../decisions.md)。

