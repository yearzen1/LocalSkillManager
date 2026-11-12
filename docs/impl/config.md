# 配置与语言持久化

> 上级：[实现方案索引](../impl.md) ｜ 相关：[CLI 与输出](cli.md) ｜ [存储与命令](storage.md) ｜ [测试与交付](quality.md) ｜ [决策记录](../decisions.md)
>
> 本文拥有以下规则的完整定义：配置文件路径、文件格式、加载时机与优先级、失效策略、`config` 命令。`cli.md` 只保留语言选择的摘要并链接到本文。

## 1. 配置文件

### 1.1 路径

| 文件 | 内容 | Windows 默认路径 |
|---|---|---|
| `config.toml` | 输出语言 `lang` | `%APPDATA%\lsm\config.toml` |
| `groups.toml` | skill 分组 | `%APPDATA%\lsm\groups.toml` |

- Linux 为 `~/.config/lsm/<文件名>`，macOS 为 `~/Library/Application Support/lsm/<文件名>`。
- 基础目录优先取环境变量 `LSM_CONFIG_DIR`，否则 `dirs::config_dir()/lsm`；两个文件名固定。
- 两个文件同目录但互不读写：`config set` 不碰 `groups.toml`，`group *` 不碰 `config.toml`。这是 Q61 的拆分理由——语言是机器偏好，分组是可以单独同步/分享的数据。
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

两份都是 TOML。`config.toml` 只存语言：

```toml
# Output language: en | zh
lang = "zh"
```

`groups.toml` 是**顶层表**「组名 → 成员数组」，不再有 `[groups]` 包裹：

```toml
# Skill groups: name = [skill, "@group"]
frontend = ["tdd", "grill", "intent-lock"]
common = ["@frontend", "conventional-commit"]
```

- UTF-8 编码；含 BOM 时先去 BOM 再解析。
- `lang` 值只接受 `en`/`zh` 及其别名（`en-US`/`zh-CN`）；**落盘永远写规范形式** `en`/`zh`。
- 文件可手改：注释与未知键在 `config set` / `group *` 写盘时原样保留（用 `toml_edit` 做读改写）。
- `config.toml` 里的未知键不报错；当前版本不读取它们，也不删除它们。
- 缺文件、缺 `lang` 键、缺 `groups.toml` 都表示"未设置"，不是错误。

### 1.4 分组（`groups.toml`）

- 顶层表，键是分组名，值是成员数组；数组顺序即成员定义顺序，重复成员按首次出现去重。
- 成员有两种写法：`tdd` 是 skill 名；`@common` 是组引用（引用另一个分组）。因为 skill 名不允许 `@`，TOML 层面不需要转义；但 Windows PowerShell 会把 token 开头的 `@组名` 当成 splatting 展开，命令行里必须写成 `'@组名'`（或用 `--%` 停止解析），否则该参数不会传进 lsm。
- 分组名与 skill 名共用同一套校验（小写字母/数字加单个连字符、≤64、拒绝 Windows 保留名）；大写输入在格式错误 hint 里提示对应的全小写写法（如 `Predevelop` → `predevelop`），但不会自动改写。
- skill 成员在 `group set` / `group add` 写入前要求已安装在 **global**，并按「精确 → 唯一前缀」解析后写入全名（`code` → `code-review`）。
- `@ref` 在写入前要求目标分组已定义，并按「精确 → 唯一前缀」解析；拒绝自引用与成环（`a -> b -> a`）。
- 任一成员缺失、多候选或引用成环时整组不写盘：逐条打印错误，最后汇总退 1。缺失时若存在相近名字（子串或编辑距离，最多 3 条），hint 列出候选。
- 直接手改 `groups.toml` 仍可写入未安装的 skill 或悬空引用；展开时（`group list --resolved`、`pull --group`）检测缺失引用与环，并逐条失败。
- 展开规则：深度优先，`@ref` 在原位置插入被引用组的成员，skill 按首次出现去重；顶层组展开为空时 `pull --group` 记为失败。
- `group remove`、`group delete` 与 `pull --group` 的组名按「精确 → 唯一前缀」解析：恰好一个组匹配时用全名；多候选或零命中按该条失败，零命中时 hint 列出相近组名；`group set` / `group add` 的目标组名保持精确。
- `group remove` 只删除成员/引用；`group delete` 删除整组，被其他组引用时默认拒绝，`--force` 才删除并同时移除这些引用。
- `group list --json` 输出 `[{"name":...,"skills":[...],"includes":[...]}]`；`skills` 是直接 skill 成员，`includes` 是 `@ref`；`--resolved` 时 `skills` 换成展开后的列表。
- 结构非法（值不是数组、成员不是字符串、名字不合法）→ 运行时错误退 1，提示修复或删除 `groups.toml`。
- **惰性读取**：只有 `pull --group` 与 `group list/set/add/remove/delete` 读取 `groups.toml`；`list`、`search`、`add`、`remove`、`push`、`config`、`upgrade` 与语言解析完全不碰它，`groups.toml` 写坏不影响这些命令。
- **迁移（Q61）**：`groups.toml` 不存在时读回退到 `config.toml` 的 `[groups]`（0.8.0 及更早的存放位置）；第一次写盘把当前生效的分组写进 `groups.toml`，并尽力删除 `config.toml` 里的旧表。删除失败只打一条 warning（`GroupLegacyLeftover`）、退出码仍为 0，旧表从此被忽略。`groups.toml` 存在时旧表完全不读，也不在写盘时合并。
- 迁移是**一次性**的：`[groups]` 里手写的注释不会搬过去（值会完整保留），文档在 `how-to/fix-config-problems.md` 里说明。
- 迁移窗口内（`groups.toml` 尚未生成）读回退仍需解析 `config.toml`，因此坏掉的 `config.toml` 仍会让 `group *` 失败；一旦写过分组就完全隔离。

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

// src/infra/groups.rs
pub enum GroupError {
    Path(PathError),                 // 配置目录无法确定
    Io { path, source },
    Write { path, source },
    Parse { path, source },
    Invalid { path, detail },        // 结构或成员名非法
    GroupExists { name },            // group set 覆盖已存在的组且未加 --force
    GroupNotFound { name },          // 要 add/remove/delete 的组不存在
}

pub struct GroupWriteReport {
    pub path: PathBuf,                   // 写盘后的 groups.toml
    pub members: usize,                  // 成员总数；delete 固定 0
    pub legacy_leftover: Option<PathBuf>, // 旧 [groups] 没删掉时带回路径
}

pub struct StripReport {
    pub stripped: Vec<(GroupName, GroupName)>,
    pub legacy_leftover: Option<PathBuf>,
}

pub fn load_groups(env: &Env) -> Result<Vec<Group>, GroupError>;        // 按组名字典序
pub fn set_group(env: &Env, name: &GroupName, members: &[Member], force: bool)
    -> Result<GroupWriteReport, GroupError>;
pub fn add_group(env: &Env, name: &GroupName, members: &[Member])
    -> Result<GroupWriteReport, GroupError>;                            // 不存在则创建，已存在则追加去重
pub fn replace_group_members(env: &Env, name: &GroupName, members: &[Member])
    -> Result<GroupWriteReport, GroupError>;                            // group remove 用
pub fn strip_group_references(env: &Env, targets: &[GroupName])
    -> Result<StripReport, GroupError>;                                 // group delete --force 用
pub fn remove_group(env: &Env, name: &GroupName) -> Result<GroupWriteReport, GroupError>;

// src/infra/tomlfile.rs —— 两份文件共用的读改写
pub fn read(path: &Path) -> Result<(DocumentMut, bool), TomlFileError>;  // bool = 文件不存在
pub fn write(path: &Path, doc: &DocumentMut) -> Result<(), TomlFileError>;
pub fn write_rendered(path: &Path, rendered: &str) -> Result<(), TomlFileError>;
```

- `GroupWriteReport.members` 是写盘后的成员总数，供成功提示显示；`legacy_leftover` 由命令层转成 warning。
- 所有写操作复用 `infra::tomlfile` 的「读 → `toml_edit` 改 → 临时文件 → rename」流程，保留注释与未知键。临时文件名按目标文件生成：`.config.toml.tmp-<pid>` / `.groups.toml.tmp-<pid>`。
- `group delete` 删到空表时保留空的 `groups.toml`（带文件头），不做文件删除。
- 替换成员数组时**原地改 `Item`**，不重建键：键上挂的注释不会因为 `group set/add` 丢失。
- `ConfigError` / `GroupError` 定义在 `infra`；`error::mapping` 把它们转成 `AppError { kind: Runtime }`，不在 infra 内构造 `AppError`。
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
| `groups.toml` 语法或结构非法 | 运行时错误，只影响 `group *` 与 `pull --group` | 1 |
| `groups.toml` 缺失且 `config.toml` 的 `[groups]` 非法 | 运行时错误，提示修复或删除 `config.toml`（迁移窗口内） | 1 |
| 旧 `[groups]` 删除失败 | 分组已写入 `groups.toml`，打 warning，旧表被忽略 | 0 |

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
- `config` 子命令不参与短选项归一化：`-a/-l/-r/-g/-p` 只映射 add/list/remove，其余原样交给 clap。

### 4.5 `group` 命令

```
lsm group list [--json] [--resolved]
lsm group set <GROUP> <ITEM>... [--force]
lsm group add <GROUP> <ITEM>...
lsm group remove <GROUP> <ITEM>...
lsm group delete <GROUP>... [--force]
```

- `group` 是顶层子命令；`list`/`set`/`add`/`remove`/`delete` 是它的嵌套子命令，`group` 无子命令时 help 到 stderr、退 2。
- 组名与成员在 `cli::parse` 全量预校验，任一非法退 2、一条都不写。
- `<ITEM>` 是 skill 名或 `@组名` 引用；格式非法（含 `@` 后为空）退 2。
- `group list`：按组名字典序输出，一行一组 `frontend: @common, tdd`；空组显示 `(empty)`；没有任何组时输出 `No groups found.`；`--json` 输出 `[{"name":"frontend","skills":["tdd"],"includes":["common"]}]`（2 空格缩进，空结果为 `[]`）；`--resolved` 把 `@ref` 展开进 `skills`。
- `group set`：组不存在则创建；已存在时必须 `--force`，否则退 1 并提示先 `group list`；成员为空由 clap 报缺参数退 2。重复成员去重。
- `group add`：组不存在则创建；已存在则把新成员追加到末尾，重复成员静默去重；成功退 0。
- `group remove <GROUP> <ITEM>...`：只从该组移除成员或引用，组本身保留；成员按组内已有成员前缀解析，找不到时整组不修改并退 1；只给组名退 2，hint 指向 `group delete`。
- `group delete <GROUP>...`：删除整组；被其他组引用时默认拒绝并列出依赖者，`--force` 时同时移除这些引用；不存在的组逐条报错、其余继续，结束打印成功/失败计数，任一失败退 1。
- 写入失败、`groups.toml` 坏 TOML 或结构非法都退 1，hint 指向修复或删除 `groups.toml`；旧 `config.toml` 的 `[groups]` 删除失败只打 warning、退 0。
- 所有写操作都先落 `groups.toml`，再尝试清理旧表；两个文件的临时名分别是 `.config.toml.tmp-<pid>` 与 `.groups.toml.tmp-<pid>`。
- 成功提示：`group 'frontend' saved (3 skills) -> <配置路径>`、`removed 'tdd' from group 'frontend'` 与 `removed group 'frontend'`，按当前语言渲染。

## 5. 消息键

键是 `MsgKey` 的枚举变体（定义在 `i18n/mod.rs`），文案在 `i18n/en.rs` / `i18n/zh.rs`，两边必须各一份：

| Key | en | zh |
|---|---|---|
| `ConfigAbout` | Manage persistent settings | 查看或修改持久化配置 |
| `ConfigGetAbout` | Print a saved setting | 打印已保存的配置值 |
| `ConfigSetAbout` | Save a setting | 保存配置值 |
| `ConfigKeyLangHelp` | Configuration key: lang | 配置键：lang |
| `ConfigValueArg` | Language value: en or zh | 语言值：en 或 zh |
| `ConfigSaved` | lang = {0} (saved to {1}) | lang = {0}（已保存到 {1}） |
| `ConfigInvalidValue` | invalid language '{0}' | 非法语言 '{0}' |
| `ConfigParseFailed` | cannot parse config file: {0} | 无法解析配置文件：{0} |
| `ConfigWriteFailed` | failed to write config file: {0} | 写入配置文件失败：{0} |
| `ConfigUnknownKey` | unknown config key '{0}' | 未知配置键 '{0}' |
| `ConfigFixHint` | fix or delete {0} | 修复或删除 {0} |
| `GroupAbout` | Manage skill groups | 管理 skill 分组 |
| `GroupListAbout` | List skill groups | 列出 skill 分组 |
| `GroupSetAbout` | Create or replace a skill group | 创建或替换 skill 分组 |
| `GroupAddAbout` | Append skills to a group | 向分组追加 skill |
| `GroupRemoveAbout` | Remove skills from a group | 从分组移除 skill |
| `GroupDeleteAbout` | Delete skill groups | 删除 skill 分组 |
| `GroupNameArg` | Group name | 分组名 |
| `GroupSetForceFlag` | Overwrite an existing group | 覆盖已存在的分组 |
| `GroupDeleteForceFlag` | Delete even if other groups reference it | 即使被其他组引用也删除 |
| `GroupSkillArg` | Skills or @group references | skill 名称或 @group 引用 |
| `GroupJsonFlag` | Print machine-readable JSON | 输出机器可读的 JSON |
| `GroupResolvedFlag` | Expand @group references | 展开 @group 引用 |
| `GroupInvalid` | invalid group name '{0}' | 非法的分组名称 '{0}' |
| `GroupInvalidHint` | group names use lowercase letters, digits and single hyphens | 分组名称只能使用小写字母、数字和单个连字符 |
| `GroupTooLong` | group name is too long: {0} > 64 characters | 分组名称过长：{0} > 64 个字符 |
| `GroupReserved` | group name '{0}' is reserved on Windows | 分组名称 '{0}' 是 Windows 保留设备名 |
| `GroupAlreadyExists` | group '{0}' already exists | 分组 '{0}' 已存在 |
| `GroupNotFound` | group '{0}' not found | 找不到分组 '{0}' |
| `GroupNotFoundHint` | run `lsm group list` to see available groups | 运行 `lsm group list` 查看可用分组 |
| `NameSuggestion` | did you mean: {0}? | 你是不是想找：{0}？ |
| `GroupLowercaseHint` | did you mean '{0}'? group names must be lowercase | 你是不是想用 '{0}'？分组名只能小写 |
| `GroupMemberMissing` | skill '{0}' is not installed in global | skill '{0}' 尚未安装到 global |
| `GroupMemberMissingHint` | run `lsm list global` to see installed skills | 运行 `lsm list global` 查看已安装的 skill |
| `GroupMembersFailed` | {0} skill(s) are unavailable in global; the group was not written | {0} 个 skill 在 global 中不可用，未写入任何分组 |
| `GroupMembersFailedHint` | fix the skills above, then run the command again | 修正上面的 skill 名称后重新运行 |
| `GroupSetFailed` | {0} item(s) could not be resolved; the group was not changed | {0} 个成员无法解析，未修改分组 |
| `GroupSetFailedHint` | fix the names above, then run the command again | 修正上面的名称后重新运行 |
| `GroupListFailed` | {0} group(s) could not be expanded | {0} 个分组无法展开 |
| `GroupListFailedHint` | fix the @group references above, then run the command again | 修正上面的 @group 引用后重新运行 |
| `GroupAmbiguous` | group '{0}' matches more than one group: {1} | 分组 '{0}' 匹配到多个组：{1} |
| `GroupAmbiguousHint` | use a longer prefix, or run `lsm group list` | 请使用更长的前缀，或运行 `lsm group list` |
| `GroupItemNotInGroup` | '{0}' is not in group '{1}' | 分组 '{1}' 中没有 '{0}' |
| `GroupRemoveFailed` | {0} item(s) are not in group '{1}'; the group was not changed | {0} 个成员不在分组 '{1}' 中，未修改分组 |
| `GroupRemoveFailedHint` | fix the names above, then run the command again | 修正上面的名称后重新运行 |
| `GroupMemberRemoved` | removed '{0}' from group '{1}' | 已从分组 '{1}' 移除 '{0}' |
| `GroupRefMissing` | group '{0}' references undefined group '{1}' | 分组 '{0}' 引用了不存在的分组 '{1}' |
| `GroupRefMissingHint` | run `lsm group list` to see defined groups | 运行 `lsm group list` 查看已定义的分组 |
| `GroupCycle` | group reference cycle: {0} | 分组引用成环：{0} |
| `GroupCycleHint` | remove one of the references in groups.toml | 移除 groups.toml 中的一条引用 |
| `GroupSelfReference` | group '{0}' cannot include itself | 分组 '{0}' 不能引用自身 |
| `GroupDeleteReferenced` | group '{0}' is referenced by: {1} | 分组 '{0}' 被以下分组引用：{1} |
| `GroupDeleteReferencedHint` | use --force to delete it and remove the references | 使用 --force 删除并移除这些引用 |
| `GroupRefStripped` | removed reference '@{0}' from group '{1}' | 已从分组 '{1}' 移除引用 '@{0}' |
| `GroupEmpty` | group '{0}' is empty | 分组 '{0}' 为空 |
| `GroupEmptyHint` | add members with `lsm group add {0} <skill>...` | 运行 `lsm group add {0} <skill>...` 添加成员 |
| `GroupNoGroups` | No groups found. | 未找到任何分组。 |
| `GroupLine` | {0}: {1} | {0}: {1} |
| `GroupEmptyMembers` | (empty) | （空） |
| `GroupSaved` | group '{0}' saved ({1} skills) -> {2} | 分组 '{0}' 已保存（{1} 个成员）-> {2} |
| `GroupRemoved` | removed group '{0}' | 已删除分组 '{0}' |
| `GroupBatchSummary` | removed {0}, failed {1} | 成功 {0} 个，失败 {1} 个 |
| `GroupBatchFailed` | {0} group(s) failed to remove | {0} 个分组删除失败 |
| `GroupBatchFailedHint` | fix the groups above, then run `lsm group delete` again | 修复上面的分组后重新运行 `lsm group delete` |
| `GroupParseFailed` | cannot parse groups file: {0} | 无法解析分组文件：{0} |
| `GroupWriteFailed` | failed to write groups file: {0} | 写入分组文件失败：{0} |
| `GroupFileInvalid` | invalid groups file {0}: {1} | 分组文件 {0} 非法：{1} |
| `GroupLegacyLeftover` | could not remove [groups] from {0}; it is now ignored | 无法从 {0} 删除旧 [groups]；该表已被忽略 |
| `PullGroupFlag` | Pull every skill in a group (repeatable) | 拉取分组内的全部 skill（可重复） |

- `config get` 的输出是裸值，不经过消息表。
- 键名用 `Config` 前缀分组，将来加键时直接追加变体；`i18n::tests` 会遍历全部键，检查两种语言都有非空文案、占位符集合一致。

## 6. 相关决策

- Q27：语言持久化（配置文件 + `config get/set`，TOML + `toml_edit`）。
- Q28：优先级与失效策略（惰性读取、坏配置退 1、不做项目级配置）。
- Q29：分层收敛（`Lang` 唯一、`Env` 快照、`decide` 纯决策表、错误映射与渲染在边界）。
- Q53 / Q54：分组定义、读取与排序规则；Q61 起存放位置改为 `groups.toml`。
- Q55：`group list/set/add/remove/delete` 的命令形态与批量语义。
- Q56：`group set` 覆盖已存在的组必须 `--force`；`group delete` 删除被引用的组需要 `--force`。
- Q57：分组属于 v0.7.0。
- Q58：`group set/add` 的成员写入前必须存在于 global 并解析前缀；组名读取支持唯一前缀；大写组名只给提示，不自动改写。
- Q60：成员支持 `@组名` 引用；展开去重并检测环；`group remove` 删成员、`group delete` 删组；`--resolved` 与 JSON `includes`。
- Q61：分组从 `config.toml` 的 `[groups]` 拆到同目录的 `groups.toml`（顶层表）；读取时回退旧表，首次写盘迁移并清理；版本升到 0.9.0。

完整决策见 [decisions.md](../decisions.md)。
