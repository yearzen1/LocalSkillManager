# 配置与语言持久化

> 上级：[实现方案索引](../impl.md) ｜ 相关：[CLI 与输出](cli.md) ｜ [存储与命令](storage.md) ｜ [测试与交付](quality.md) ｜ [决策记录](../decisions.md)
>
> 本文拥有以下规则的完整定义：配置文件路径、文件格式、加载时机与优先级、失效策略、`config` 命令。`cli.md` 只保留语言选择的摘要并链接到本文。

## 1. 配置文件

### 1.1 路径

| 平台 | 默认路径 |
|---|---|
| Windows | `%APPDATA%\lsm\config.toml` |
| Linux | `~/.config/lsm/config.toml` |
| macOS | `~/Library/Application Support/lsm/config.toml` |

- 基础目录优先取环境变量 `LSM_CONFIG_DIR`，否则 `dirs::config_dir()/lsm`；文件名固定 `config.toml`。
- `LSM_CONFIG_DIR` 与 `LSM_HOME` 是两件独立的事：前者只放 `lsm` 自己的配置，后者只决定 skills 根目录。
- 配置文件**不放在** `.agents` 下：那是与其他 agent 共享的 skill 目录，不属于本工具的私有状态。
- 路径解析统一在 `infra::paths`：

  ```rust
  // src/infra/paths.rs —— 纯函数，环境由 Env 传入
  pub fn config_file(env: &Env) -> Result<PathBuf, PathError>;
  ```

- `LSM_CONFIG_DIR` 为空字符串按"未设置"处理（与 `LSM_HOME` 一致）。
- `dirs::config_dir()` 返回 `None` 且未设置 `LSM_CONFIG_DIR` → 运行时错误（退出码 1），提示设置 `LSM_CONFIG_DIR`。

### 1.2 文件格式

TOML，v1 只有一个键：

```toml
# Output language: en | zh
lang = "zh"
```

- UTF-8 编码；含 BOM 时先去 BOM 再解析。
- 值只接受 `en`/`zh` 及其别名（`en-US`/`zh-CN`）；**落盘永远写规范形式** `en`/`zh`。
- 文件可手改：注释与未知键在 `config set` 时原样保留（用 `toml_edit` 做读改写）。
- 未知键不报错；v1 不读取它们，也不删除它们。
- 缺文件、缺 `lang` 键都表示"未设置"，不是错误。

### 1.3 数据结构

```rust
// src/domain/lang.rs —— 语言身份与归一化的唯一来源
pub enum Lang { En, Zh }
pub fn parse(raw: &str) -> Result<Lang, LangError>;

// src/domain/config.rs
pub enum ConfigKey { Lang }          // 将来加 default_scope 时追加变体

// src/infra/config.rs
pub enum ConfigError {
    Path(PathError),                 // 配置目录无法确定
    Io { path, source },             // 读取失败
    Write { path, source },          // 建目录 / 临时文件 / rename 失败
    Parse { path, source },          // TOML 语法非法
    InvalidValue { path, raw },      // lang 值不受支持
}

pub fn load(env: &Env) -> Result<Option<Lang>, ConfigError>;
pub fn set_lang(env: &Env, value: Lang) -> Result<PathBuf, ConfigError>; // 返回写入路径
```

- `ConfigError` 定义在 `infra::config`；`error::mapping` 把它转成 `AppError { kind: Runtime }`，不在 infra 内构造 `AppError`。
- 只有 `domain::lang::Lang` 一个语言类型：`--lang`、`LSM_LANG`、配置文件值都走同一个 `Lang::parse`，不存在两套归一化。

## 2. 加载与优先级

### 2.1 优先级

`--lang` > 环境变量 `LSM_LANG` > 配置文件 `lang` > 默认 `en`。配置文件只是"用户偏好"这一层，等同于 git 的 `config` 在 `--flag` 和 `GIT_*` 环境变量之下。

### 2.2 惰性读取

只有前两档都没有给出值时，才读配置文件：

| 命令行 | 环境变量 | 是否读文件 | 结果 |
|---|---|---|---|
| `--lang en` | 任意 | 否 | `en` |
| 无 `--lang` | `LSM_LANG=zh` | 否 | `zh` |
| 无 `--lang` | 未设置 | 是 | 文件值；文件或键缺失则 `en` |

- 由此可推出：`--lang` 或 `LSM_LANG` 已经生效时，坏掉的配置文件**不会**被读到，也不会报错。这是有意的"逃生通道"，不是漏检，文档与 README 要写明。
- 例外：`config get` / `config set` 直接面向配置本身，**始终**读写文件，与惰性读取无关。

### 2.3 解析流程

语言解析路径上只有组合根 `lib::run` 读配置与进程环境；`commands::config` 因命令本身就要读写配置，直接调用 `infra::config`（见 §4）。`cli` 只看 `argv`，`i18n` 保持纯函数。

```rust
// src/infra/env.rs
pub struct Env {
    pub home: Option<PathBuf>,        // LSM_HOME
    pub config_dir: Option<PathBuf>,  // LSM_CONFIG_DIR
    pub lang: Option<String>,         // LSM_LANG（原始字符串）
}
pub fn read() -> Env;

// src/cli/lang.rs —— 纯决策表
pub fn prescan(argv: &[OsString]) -> Option<String>;      // 取最后一个 --lang 的原始值
pub enum Decision { Cli(Lang), Env(Lang), NeedFile }
pub fn decide(cli: Option<&str>, env: Option<&str>) -> Result<Decision, LangError>;

// src/lib.rs
pub fn run(args: impl Iterator<Item = OsString>) -> ExitCode;
```

1. `cli::lang::prescan` 命中 `--lang`：`Lang::parse` 失败 → 用法错误退出 2；成功即返回 `Decision::Cli`，**不读环境与文件**。
2. 否则 `Env::read()` 取 `LSM_LANG`：存在则 `Lang::parse`，失败 → 用法错误 2；成功返回 `Decision::Env`。
3. 否则 `decide` 返回 `Decision::NeedFile`，由 `lib::run` 调 `infra::config::load()`：`Ok(Some(lang))` → 采用；`Ok(None)` → `en`；`Err(e)` → 运行时错误 1。
4. 语言确定后再 `cli::spec::build(lang)`，保证 help 已本地化。

- 多个 `--lang` 同时出现时取最后一个（与 clap 默认行为一致）。
- `LSM_LANG` 为空字符串按"未设置"处理。
- `decide` 只做三态判定、不碰 I/O，单测可直接断言"前两档命中时永不返回 `NeedFile`"。

### 2.4 渲染语言

错误与 help 的渲染语言按最可靠的一档选取：

1. 显式且合法的 `--lang`；
2. 合法的 `LSM_LANG`；
3. 已成功解析出的文件值；
4. 默认 `en`。

- 配置文件解析失败时第 3 档不存在，因此落到第 2 档（若有）或 `en`。
- `LSM_LANG=zh` + 坏文件 + `config set` → 报错用**中文**渲染：语言已确定，配置失败与它无关。
- 显式 `--lang` 非法时（例如 `--lang fr`）用 `en` 渲染错误文案，避免"用没通过校验的值决定语言"。

### 2.5 不做项目级配置

- 只有一份全局配置；不支持 `<cwd>/.agents/config.toml`、`.lsm.toml` 之类的项目级覆盖。
- 理由：语言是用户偏好而非项目属性；project 作用域已固定 cwd、不向上查找，再叠加项目级配置会让"在子目录执行换了语言"成为新坑。需要时按新决策追加。

## 3. 失效策略

| 情况 | 行为 | 退出码 |
|---|---|---|
| 文件不存在 | 用默认 `en`，不警告 | 0 |
| 缺 `lang` 键 | 用默认 `en`，不警告 | 0 |
| 含未知键 | 忽略并保留，不警告 | 0 |
| TOML 解析失败 | 运行时错误，提示 `fix or delete <path>` | 1 |
| `lang = "fr"` 等非法值 | 运行时错误，提示合法值与 `fix or delete <path>` | 1 |
| `config set` 遇到损坏文件 | 不覆盖，报运行时错误并提示修复或删除 | 1 |

- 不采用"忽略坏配置 + stderr warning 继续跑"：那样"为什么语言不对"会变成迷宫，而 `--lang` 已经提供了安全的逃生通道。
- 坏配置的报错走 stderr；`--json` 模式下 stdout **一个字节都不输出**，符合 `cli.md`「`--json` 下 stdout 只允许 JSON」。
- 报错文案按 §2.4 选语言；`fix or delete <path>` 必须带真实路径，方便直接复制执行。

## 4. `config` 命令

### 4.1 语法

```
lsm config get lang
lsm config set lang <en|zh>
```

- 顶层第 4 个子命令；`--lang` 仍是根命令全局参数，可出现在任意位置。
- 不提供 `config list`、`config unset`、`--json`。

### 4.2 get

- 输出**裸值**：`en` 或 `zh`，不含键名、路径、颜色或本地化说明，方便 `$(lsm config get lang)`。
- 读的是**配置文件里的值**，不是当前生效语言；文件或键缺失时输出默认 `en`。
- 输出到 stdout，退出码 0。
- 文件损坏或值非法 → 运行时错误 1（渲染语言按 §2.4），stdout 无输出。

### 4.3 set

```
$ lsm config set lang zh
lang = zh (saved to C:\Users\jiahui\AppData\Roaming\lsm\config.toml)
```

- 值接受 `en`/`zh`/`en-US`/`zh-CN`，落盘统一为 `en`/`zh`；非法值 → 用法错误 2，提示合法值。
- 写入步骤：
  1. `create_dir_all` 配置目录；
  2. 读现有文件（存在时）并用 `toml_edit` 修改 `lang`，保留注释与未知键；
  3. 同目录创建临时文件 `.config.toml.tmp-<pid>`；
  4. `rename` 覆盖目标（Rust 的 `std::fs::rename` 在 Windows 上对文件即替换语义，无需先移走）；
  5. 失败时删除临时文件，旧文件保持原样，返回运行时错误 1 并带路径。
- 不调用 `fsync`：配置只存语言，掉电丢失可接受，避免无谓复杂度。
- 文件权限沿用平台默认（Unix `0644`），内容非敏感，不设 `0600`。
- 新值与旧值相同时**仍然写一次**：保证幂等，并让权限/磁盘问题立刻暴露。
- 成功提示的语言 = 显式 `--lang`；没有时用**刚写入的值**（设成中文就用中文确认）。
- 原文件解析失败时**不覆盖**，报运行时错误 1 并提示修复或删除，避免冲掉用户手写内容。

### 4.4 help 与用法错误

- `lsm config --help` / `-h`：本地化 help 到 stdout，退出码 0。
- `lsm config`（无子命令）：本地化 help 到 **stderr**，退出码 2（沿用 Q20）。
- `lsm config get`（缺键）、`lsm config get scope`（v1 无此键）、`lsm config set lang fr`：用法错误 2，提示合法键只有 `lang`、合法值为 `en`/`zh`。
- `config` 子命令不参与短选项归一化：`-a/-l/-c/-g/-p` 只映射 add/list/copy，其余原样交给 clap。

## 5. 消息键

键是 `MsgKey` 的枚举变体（定义在 `i18n/mod.rs`），文案在 `i18n/en.rs` / `i18n/zh.rs`，两边必须各一份：

| Key | en | zh |
|---|---|---|
| `ConfigAbout` | Manage persistent settings | 查看或修改持久化配置 |
| `ConfigGetAbout` | Print a saved setting | 打印已保存的配置值 |
| `ConfigSetAbout` | Save a setting | 保存配置值 |
| `ConfigKeyLangHelp` | Output language | 输出语言 |
| `ConfigValueArg` | Language value: en or zh | 语言值：en 或 zh |
| `ConfigSaved` | lang = {0} (saved to {1}) | lang = {0}（已保存到 {1}） |
| `ConfigInvalidValue` | invalid language '{0}' | 非法语言 '{0}' |
| `ConfigParseFailed` | cannot parse config file: {0} | 无法解析配置文件：{0} |
| `ConfigWriteFailed` | failed to write config file: {0} | 写入配置文件失败：{0} |
| `ConfigUnknownKey` | unknown config key '{0}' | 未知配置键 '{0}' |
| `ConfigFixHint` | fix or delete {0} | 修复或删除 {0} |

- `config get` 的输出是裸值，不经过消息表。
- 键名用 `Config` 前缀分组，将来加键时直接追加变体；`i18n::tests` 会遍历全部键，检查两种语言都有非空文案、占位符集合一致。

## 6. 相关决策

- Q27：语言持久化（配置文件 + `config get/set`，TOML + `toml_edit`）。
- Q28：优先级与失效策略（惰性读取、坏配置退 1、不做项目级配置）。
- Q29：分层收敛（`Lang` 唯一、`Env` 快照、`decide` 纯决策表、错误映射与渲染在边界）。

完整决策见 [decisions.md](../decisions.md)。
