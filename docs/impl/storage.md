# 存储与命令实现

> 上级：[实现方案索引](../impl.md) ｜ 相关：[CLI 与输出](cli.md) ｜ [配置与持久化](config.md) ｜ [测试与交付](quality.md) ｜ [决策记录](../decisions.md)
>
> 本文拥有以下规则的完整定义：作用域与路径、SKILL.md 规范、add/list/pull、文件系统操作。其他文档只做摘要和链接。

## 1. 作用域与路径

| 作用域 | 解析规则 | 最终路径 |
|---|---|---|
| global | `LSM_HOME` 存在则用之，否则 `~/.agents` | `<base>/skills/<name>/` |
| project | **固定为进程当前工作目录（cwd）** | `<cwd>/.agents/skills/<name>/` |

已锁定：

- project **不做**向上查找 `.git` / `.agents`。在子目录执行就会装到子目录，这是明确接受的行为；README 需提示"请在项目根目录执行"。
- `LSM_HOME` 指向 `.agents` 这一层（便于测试整体重定向），`skills` 由程序拼接。
- **配置文件不在 skills 目录下**：`config.toml` 位于 `dirs::config_dir()/lsm`（可用 `LSM_CONFIG_DIR` 覆盖），`LSM_HOME` 只影响 skills 根目录，两者互不影响；详见 [config.md](config.md)。
- Windows 家目录由 `dirs::home_dir()` 得到，例如 `C:\Users\jiahui\.agents\skills`。
- 所有路径使用 `PathBuf`，禁止硬编码 `/` 或 `\`。
- `Env::read()` 一次读全环境，返回快照；空串按未设置处理，`LSM_LANG` 非 UTF-8 同样按未设置。除 `LSM_HOME` / `LSM_CONFIG_DIR` / `LSM_LANG` 外还快照 `LSM_UPGRADE_API` 与 `current_exe()`（供 `upgrade` 使用，规则见 [upgrade.md](upgrade.md)）。
- `home` / `config_dir` 无法确定时为 `None`，由 `paths` 返回 `HomeUnavailable` / `ConfigDirUnavailable`，不做猜测性回退。
- 安装前按需 `create_dir_all`，不要求用户手工建目录。
- cwd 由 `lib::run` 读取一次并显式传给 `infra::paths`，避免测试修改进程级 cwd。

`domain::scope` 与 `infra::paths` 接口：

```rust
// domain/scope.rs
pub enum Scope { Global, Project }

impl Scope {
    pub fn label(self) -> &'static str;
}

// infra/env.rs —— 唯一读取进程环境的位置，返回一次性快照
pub struct Env {
    pub home: Option<PathBuf>,         // LSM_HOME，否则 ~/.agents
    pub config_dir: Option<PathBuf>,   // LSM_CONFIG_DIR，否则 dirs::config_dir()/lsm
    pub lang: Option<String>,          // LSM_LANG 原始值，空串视为未设置
    pub upgrade_api: Option<String>,   // LSM_UPGRADE_API，空串视为未设置
    pub exe: Option<PathBuf>,          // current_exe()，供 upgrade 原地替换
}

impl Env {
    pub fn read() -> Self;
}

// infra/paths.rs —— 纯函数，环境由 Env 传入
pub fn skills_dir(env: &Env, scope: Scope, cwd: &Path) -> Result<PathBuf, PathError>;
pub fn skill_dir(env: &Env, scope: Scope, cwd: &Path, name: &SkillName) -> Result<PathBuf, PathError>;
pub fn config_file(env: &Env) -> Result<PathBuf, PathError>;

pub enum PathError {
    HomeUnavailable,      // 无 LSM_HOME 且无家目录
    ConfigDirUnavailable, // 无 LSM_CONFIG_DIR 且无配置目录
}
```

## 2. SKILL.md 规范

### 2.1 文件格式

```markdown
---
name: my-skill
description: 一句话说明这个 skill 做什么、什么时候用。
---

# My Skill

（正文）
```

- 只解析文件开头被 `---` 包裹的 YAML frontmatter。
- 只读取 `name`、`description`，其余字段原样保留、不做改写。
- 文件内容在安装时保持字节级不变（不重新序列化 YAML）。

### 2.2 校验规则

| 规则 | 失败行为 |
|---|---|
| 必须存在 frontmatter 块 | 报错，给出模板提示 |
| 必须含 `name` 且 trim 后非空 | 报错 |
| 必须含 `description` 且 trim 后非空 | 报错 |
| `name` 匹配 `^[a-z0-9]+(-[a-z0-9]+)*$` | 报错，提示允许字符 |
| `name` 长度 ≤ 64 字符 | 报错 |
| `description` 长度 ≤ 1024 字符 | 报错 |
| `name` 是 Windows 保留设备名 | 报错，提示 `name is reserved on Windows` |
| 目录安装时，目录名与 `name` 不一致 | 报错，要求先把目录名改成与 `name` 一致 |

- 长度按 Unicode 字符计数（`chars().count()`），不按字节。
- 上限以 `domain::name::MAX_NAME_LEN`（64）与 `domain::meta::MAX_DESCRIPTION_LEN`（1024）常量表达，代码不写字面量。
- `SkillName::parse` 的校验顺序固定为：长度 → 格式 → Windows 保留名，只返回第一个命中的错误。
- 超限一律报错，**不截断、不自动修复**。
- YAML 语法非法时报错并提示修法：plain scalar 不能包含 `: `（冒号+空格），值里有就加引号或改用 `>-` 折叠块。
- 不做正则依赖：`name` 校验用字符遍历（首尾必须是字母/数字，中间允许单个连字符）。
- Windows 保留设备名（大小写不敏感）：`con`、`prn`、`aux`、`nul`、`com1`~`com9`、`lpt1`~`lpt9`。提前拒绝，保证三平台行为一致。
- **目录名是 skill 的唯一身份**：`SKILL.md` 的 `name` 必须与所在目录名一致；不一致时 `list` 标记 `[invalid]`，`pull` 视为源非法。
- `pull <name>` 的入参必须先通过同一套 name 校验（含长度、保留名），并额外拒绝 `/`、`\`、`.`、`..`，校验失败按用法错误退出 2，绝不先拼路径再校验。

### 2.3 数据结构

```rust
// domain/name.rs
pub struct SkillName(String);

impl SkillName {
    pub fn parse(input: &str) -> Result<Self, NameError>;
    pub fn as_str(&self) -> &str;
}

// domain/meta.rs
pub struct SkillMeta {
    pub name: SkillName,
    pub description: String,
}

impl SkillMeta {
    pub fn validate(&self) -> Result<(), MetaError>;
}

// infra/frontmatter.rs
pub fn parse_skill_md(path: &Path) -> Result<SkillMeta, FrontmatterError>;
```

- `SkillName` 在构造时完成全部校验（格式、64 字符、Windows 保留名），后续代码拿到的都是已合法的值。
- 目录名一致性由 `infra::scan` / `commands` 比对，不放进 `SkillMeta::validate`。

解析步骤：

1. 读取为 UTF-8 字符串（非法编码报错）。
2. 去掉可选 BOM。
3. 兼容 CRLF：按行切分后判断 `---`。
4. YAML 反序列化为 `serde_yaml_ng::Value`，读取 `name` / `description`。
5. 校验格式与长度。

`FrontmatterError` 变体（每个都带 `path`，便于上层提示定位）：

```rust
pub enum FrontmatterError {
    Io { path: PathBuf, source: anyhow::Error },
    InvalidUtf8 { path: PathBuf },
    MissingFrontmatter { path: PathBuf },
    MissingField { path: PathBuf, field: &'static str },
    InvalidYaml { path: PathBuf, source: anyhow::Error },
    InvalidName { path: PathBuf, source: NameError },
    InvalidMeta { path: PathBuf, source: MetaError },
}
```

### 2.4 list 展示规则

- 文本输出前把 description 的 `\r`、`\n`、连续空白压成单个空格；非空白的控制字符直接丢弃，不进入排版（Q35）。
- 按**显示列**排版：描述列宽 60 列，宽度由 `unicode-width` 按 UAX #11 计算（CJK 等宽字符占 2 列），超出换行（Q34 / Q35）。
- 换行后悬挂缩进，续行起点与首行描述列对齐；最多 3 行，超出部分在第 3 行末尾以 `...` 截断。
- 断行规则：空格后的词放得下一整行就在空格处断，避免劈开英文单词；放不下（如整段无空格的中文）就按列宽硬断，把当前行填满。
- 不探测终端宽度，固定 60 列，保证脚本输出稳定。
- JSON 输出返回原始 description（由 JSON 自行转义），不换行、不截断。

## 3. 各命令实现

### 3.1 add

```
lsm add <source> <scope> [--force]
```

流程：

1. 解析 `scope`，得到 `skills_dir`。
2. 判断 `source`：
   - **文件**：扩展名必须为 `.md`（大小写不敏感）；解析 frontmatter 得到 `name`。
   - **目录**：根目录必须存在 `SKILL.md`；解析得到 `name`；目录名与 `name` 不一致时报错。
   - **符号链接**：直接报错，不支持。
   - 其他：报错。
3. 计算目标：`skills_dir/<name>`。
4. **自包含检查**：目标路径位于源目录内部时直接报错：
   ```
   error: destination is inside source directory
   hint: choose a source directory outside <cwd>/.agents
   ```
   典型触发：`lsm add . project`。
5. 目标已存在且未传 `--force` → 报错，退出码 1：
   ```
   error: skill 'foo' already exists at <path>
   hint: use --force to overwrite
   ```
6. **`.git` 警告**：目录源包含 `.git` 时，照常完整复制，但向 stderr 打一行：
   ```
   warning: source contains .git directory, it will be copied
   ```
7. 安装（原子）：
   1. `create_dir_all(skills_dir)`；
   2. 同级创建临时目录 `.tmp-<name>-<pid>`（已存在先删）；
   3. 复制内容：
      - 文件源 → 复制为 `SKILL.md`；
      - 目录源 → **不做任何排除**，递归复制全部条目（含隐藏文件、`.git`、`scripts/`、`assets/`）。
   4. `--force` 且旧目录存在：旧目录改名为 `.bak-<name>-<pid>`，`rename` 新目录成功后再删除备份；失败则回滚。
   5. 失败路径：清理临时目录与备份，返回错误。
8. 输出：
   ```
   installed my-skill -> C:\Users\jiahui\.agents\skills\my-skill
   ```

覆盖细节：Windows 上 `rename` 到已存在目录会失败，因此必须先移走旧目录，不能直接覆盖。

### 3.2 list

```
lsm list <scope> [--json]
```

流程：

1. 解析 `skills_dir`；不存在或为空 → 文本模式输出 `No skills found in <path>`，`--json` 模式输出 `[]`；退出码均为 0。
2. 遍历直接子目录，跳过 `.` 开头的条目（临时目录、备份）。
3. 每个子目录尝试解析 `SKILL.md`：
   - 解析成功且 frontmatter `name` 与目录名一致 → `valid = true`；
   - 缺失、非法或 `name` 与目录名不一致 → `valid = false`，`name` 用目录名，`description` 为空，并记录 `error`（不一致时 `error = "name mismatch: <dir> vs <name>"`）。
4. 按 `name` 字典序排序。

文本输出（描述列 60 列宽，超宽换行并悬挂缩进，最多 3 行；相邻条目之间空一行）：

```
NAME         DESCRIPTION
-----------  ------------------------------------------
broken       [invalid] missing SKILL.md

code-review  Review a diff for correctness and style, flagging risky
             changes before they reach main.

my-skill     One-line description.
```

- 固定 60 **显示列**上限，不做终端宽度探测，保证脚本输出稳定（Q34）。
- 宽度按 `unicode-width` 的 `str::width()` 计算，ZWJ emoji 序列、组合标记按整串计宽（Q35）。
- 续行用空格补齐到描述列起点，不顶到第 0 列。
- 空格后的词放得下一整行时在空格处断行；放不下（如整段无空格的中文）则按列宽硬断填满当前行。
- name 列宽取 `max(10, 最长名称字符数)`，分隔线随列宽生成。
- 相邻条目之间插入一个空行（Q49）；表头后与末尾都不加，输出仍以单个换行收尾。
- 非法项标记 `[invalid]`，不中断整体输出。

JSON 输出：

```json
[
  {
    "name": "my-skill",
    "description": "One-line description.",
    "path": "C:\\Users\\jiahui\\.agents\\skills\\my-skill",
    "valid": true
  }
]
```

- 非法项追加 `"error": "..."` 字段。
- `error` 固定英文、不参与 i18n，取值仅有：`missing SKILL.md`、`unreadable SKILL.md`、`SKILL.md is not valid UTF-8`、`missing frontmatter`、`missing <field>`、`invalid YAML frontmatter`、`invalid name`、`invalid description`、`invalid directory name`、`name mismatch: <dir> vs <name>`。
- JSON 为 2 空格缩进的 pretty 输出，末尾带一个换行；空列表输出 `[]\n`。
- JSON 直接 `println!` 到 stdout，不带任何日志或提示语。

### 3.3 pull

```
lsm pull <name>... [--force]
lsm pull --all [--force]
```

`<name>` 按 [§3.7](#37-name-前缀解析) 解析：精确优先，未命中再按唯一前缀。

流程（单条）：

1. **先校验 `name`**（不拼路径）：同一套 name 规则 + 拒绝 `/`、`\`、`.`、`..`。校验在 `cli::parse` 构造 `Action` 时执行，保证在命令逻辑之前失败，退出码 2：
   ```
   error: invalid skill name '..'
   hint: skill names use lowercase letters, digits and single hyphens
   ```
2. 源目录：`infra::paths::skills_dir(&ctx.env, Scope::Global, ctx.cwd)/<name>`，不存在则报错：
   ```
   error: skill 'foo' not found in global
   hint: run `lsm search foo global` to see similar skills
   ```
3. 源目录必须有合法 `SKILL.md`，且 frontmatter `name` 与目录名一致（防止复制半成品或错位 skill）。
4. 目标：`<cwd>/.agents/skills/<name>`。
5. 源与目标规范化后相同 → 报错，禁止自我复制；目标位于源内部 → 报错。
6. 复用 add 的「递归复制 + 原子安装 + 覆盖」逻辑（`infra::fsops::install_skill_dir`）。
7. 目标已存在且无 `--force` → 报错并提示。
8. 输出：
   ```
   copied my-skill -> <cwd>/.agents/skills/my-skill
   ```

pull 同样执行 `.git` 警告与自包含检查（目标在源内时拒绝）。

多 name（Q44）：

1. 所有 name 在 `cli::parse` 全量预校验；任一非法退 2，一条都不执行。
2. 重复 name 去重，保留首次出现的顺序。
3. 逐条复用上面的单条流程，单条失败不中断：
   - **成功**：安装完成。
   - **跳过**：目标已存在且无 `--force`，打 warning，不算失败。
   - **失败**：源缺失/非法或 I/O 错误，逐条把错误体打到 stderr。
4. 单 name 保持现有输出；多 name 每条形如 `copied ...`，结束再打一行成功/跳过/失败计数。
5. 退出码：任一失败退 1；只有跳过或全部成功退 0。

`--all` 与 `<name>...` 互斥（同时给出退出码 2）。全量分支：

1. 扫描全局 skills 目录（`infra::scan::scan_skills`，跳过隐藏项与非目录），按名字排序逐条处理。
2. 每条复用上面的单条流程（校验源 → 算目标 → 原子安装）。
3. 结果分三类累计，单条失败不中断：
   - **成功**：安装完成；`.git` 警告照常打到 stderr。
   - **跳过**：目标已存在且无 `--force`，不算失败。
   - **失败**：源非法（缺 `SKILL.md`、name 不一致）或 I/O 错误；逐条把错误体打到 stderr，其余条目继续。
4. 汇总一行到 stdout：成功 / 跳过 / 失败计数。
5. 退出码：任一失败退 1；只有跳过或全部成功退 0；全局为空退 0 并提示没有可复制的 skill，且不创建项目 skills 目录。

### 3.4 remove

```
lsm remove <name>... <global|project> [--force]
lsm -r <name>... -g|-p [--force]
```

`<name>` 按 [§3.7](#37-name-前缀解析) 解析：精确优先，未命中再按唯一前缀。

流程（单条）：

1. **先校验 `name`**：与 `pull` 同一套规则（含长度、保留名），不拼路径；失败退 2。
2. 目标：对应作用域 skills 目录下的 `<name>`；不存在 → 报错退 1，hint 指向 `lsm search <name> <scope>`。
3. 无 `--force` 时只删**合法 skill 目录**：目录名等于 `<name>`、`SKILL.md` 可解析、frontmatter `name` 与目录名一致。缺 `SKILL.md`、name 不一致或根本不是目录 → 拒绝，提示加 `--force`。
4. 删除方式：先把目标 `rename` 到同级 `.tmp-remove-<name>-<pid>`，成功后再递归删除临时目录。`rename` 失败时原目录保持完整；临时目录删不掉只留 warning（`.tmp-*` 已被 `scan` 忽略，不会污染 `list`）。
5. 输出：
   ```
   removed my-skill -> <cwd>/.agents/skills/my-skill
   ```
6. `--force` 只放开第 3 步的合法性检查，不改变 name 校验。

多 name（Q44）：

1. scope 永远取最后一个位置参数，前面的都是 name；至少一个 name，否则由 clap 报用法错误退 2。
2. 所有 name 全量预校验，任一非法退 2，一条都不删。
3. 重复 name 去重；逐条复用单条流程，单条失败不中断（不存在、非法目录、I/O 失败都算失败）。
4. 单 name 保持现有输出；多 name 每条形如 `removed ...`，结束打印成功/失败计数。
5. 退出码：任一失败退 1；全部成功退 0。

### 3.5 search

```
lsm search <query> <scope> [--json]
```

1. scope 必填，只搜一个作用域（Q45）；`scopes` 仍由 `value_parser` 校验。
2. query 去掉首尾空白后不得为空，空串退 2。
3. 扫描对应 skills 目录（`infra::scan::scan_skills`），保留合法与非法项。
4. 匹配规则：query 与 `name`、`description` 做大小写不敏感的子串匹配；非法项的 description 视为空串，只按目录名匹配。
5. 结果按 `name` 字典序排序；无匹配时文本模式输出 `No skills matched '<query>' in <scope>`，JSON 模式输出 `[]`，退出码均为 0。
6. 文本与 JSON 排版完全复用 `list`（`output::list::render_text` / `render_json`），非法项照常标 `[invalid]`；`--json` 的 stdout 只有 JSON。
7. 只读，不创建目录、不写盘。

### 3.6 push

```
lsm push <name>... [--force]
lsm push --all [--force]
```

`<name>` 按 [§3.7](#37-name-前缀解析) 解析：精确优先，未命中再按唯一前缀。

`push` 是 `pull` 的反向操作（Q46）：project（cwd）→ global。

单条流程与 `pull` 对称：

1. name 在 `cli::parse` 校验，非法退 2。
2. 源目录：`<cwd>/.agents/skills/<name>`，不存在则报错退 1：
   ```
   error: skill 'foo' not found in project
   hint: run `lsm list project` to see available skills
   ```
3. 源必须有合法 `SKILL.md`，且 frontmatter `name` 与目录名一致。
4. 目标：`<global skills dir>/<name>`。
5. 源与目标相同或目标在源内 → 报错；目标已存在且无 `--force` → 报错。
6. 复用 `install_skill_dir` 原子安装，`.git` 目录照常复制并告警。
7. 输出：
   ```
   pushed my-skill -> <global skills dir>/my-skill
   ```

多 name 与 `--all` 的语义完全复用 Q40 / Q44：全量预校验、去重、单条失败不中断、目标已存在无 `--force` 算跳过、结束打印成功/跳过/失败计数、任一失败退 1；`--all` 与 name 互斥；项目目录为空退 0 并提示没有可推送的 skill。

`push` 不加短选项，也不参与归一化。

### 3.7 name 前缀解析

`pull` / `push` / `remove` 的 `<name>` 先按目录名精确匹配，未命中时再按前缀解析（Q50）。解析只发生在命令层：`cli::parse` 仍只做格式校验，非法格式（大写、`..`、`/`）依旧退 2。

1. 目录名精确等于输入 → 精确命中，行为与 §3.3 / §3.4 / §3.6 完全一致。
2. 否则扫描对应作用域的 skills 目录（`infra::scan::dir_names`，跳过隐藏项、非目录与非法目录名），按字典序取以输入为前缀的名字：
   - 恰好 1 个 → 视为该全名继续执行，输出里显示全名（`pulled code-review -> ...`）。
   - 多个 → 运行时错误，退 1：
     ```
     error: 'code' matches more than one skill in global: code-lint, code-review
     hint: use a longer prefix, or run `lsm list global`
     ```
     单 name 时直接失败；多 name 时该条记为失败，其余照常执行。
   - 0 个 → 沿用各命令的 not-found 错误（退 1），hint 不变。
3. 解析结果在批量里重新去重：`pull code code-review` 只执行一次。
4. 多 name 时收尾仍打印成功/跳过/失败计数并退 1（与 Q44 一致）。
5. `--all` 分支不受影响。
6. 只做前缀，不做子串；name 规则本就只允许小写，命中必然大小写一致。

## 4. 文件系统细节

`infra::fsops` 与 `infra::scan` 接口：

```rust
// src/infra/fsops.rs
pub fn copy_recursive(src: &Path, dst: &Path) -> Result<(), FsError>;
pub fn contains_path(src: &Path, dst: &Path) -> Result<bool, FsError>;
pub fn contains_git_dir(src: &Path) -> bool;
pub fn is_present(path: &Path) -> bool;
pub fn is_same_path(left: &Path, right: &Path) -> Result<bool, FsError>;
pub fn inspect_source(src: &Path) -> Result<SourceKind, FsError>;
pub fn install_skill_dir(src: &Path, dst: &Path, force: bool) -> Result<InstallReport, FsError>;
pub fn install_skill_file(src: &Path, dst: &Path, force: bool) -> Result<InstallReport, FsError>;
/// 原地替换当前可执行文件；调用方先完成下载与冒烟校验（规则见 upgrade.md）。
pub fn replace_self(new: &Path, exe: &Path) -> Result<ReplaceReport, FsError>;

pub enum SourceKind {
    Directory,
    File,
}

/// 备份删除失败时把路径交回命令层，由命令层打 warning 并仍以 0 退出。
pub struct InstallReport {
    pub leftover_backup: Option<PathBuf>,
}

/// 旧 exe 删除失败时同样交回命令层，下次升级再清理。
pub struct ReplaceReport {
    pub leftover_backup: Option<PathBuf>,
}

pub enum FsError {
    Io { path: PathBuf, source: anyhow::Error },
    SymlinkUnsupported { path: PathBuf },
    AlreadyExists { path: PathBuf },
    SourceMissing { path: PathBuf },
    SourceNotExpectedType { path: PathBuf },
}

// src/domain/entry.rs
pub struct SkillEntry {
    pub name: String,
    pub description: String,
    pub path: PathBuf,
    pub valid: bool,
    pub error: Option<String>,
}

// src/infra/scan.rs
pub fn scan_skills(dir: &Path) -> Result<Vec<SkillEntry>, FsError>;
```

规则：

- 递归复制保留相对目录结构；先建目录，再复制文件。
- **不做任何排除**：隐藏文件、`.git`、`target`、`node_modules` 全部复制。
- **符号链接**：不跳过、不跟随，遇到即报错 `error: symbolic links are not supported: <path>`。
- **权限**：Unix 上沿用 `fs::copy` 保留的源文件模式；Windows 不处理 ACL。
- **临时目录**：`.tmp-<name>-<pid>`，与目标同卷，保证 `rename` 原子性。
- **备份目录**：`.bak-<name>-<pid>`，成功后递归删除；删除失败时把路径放进 `InstallReport::leftover_backup`，由命令层打印 warning，退出码仍为 0。
- **崩溃残留**：临时/备份目录以 `.` 开头，`lsm list` 自动跳过；不自动清理其他进程的残留。
- **自更新替换**：`replace_self` 只负责同卷 `rename` 与失败回滚，不下载、不校验、不判版本；`exe` 的路径与文件名必须保持不变，临时名只用 `.lsm-upgrade-<pid>` 与 `<exe>.old-<pid>`，完整规则见 [upgrade.md](upgrade.md)「原地替换与回滚」。
- **自包含检测**：源路径 `canonicalize`，目标取「cwd 规范化 + 相对后缀」，用 `starts_with` 判断；目标尚不存在也能判断。
- 所有错误都带路径上下文：底层 I/O 错误用 `anyhow::Context` 包装后挂在 `FsError` 变体里，再由 `error::mapping` 转成 `AppError.source`；`infra` 内不构造 `AppError`。

## 5. 相关决策

- Q1：project 固定为 cwd。
- Q4：目录复制不做任何排除。
- Q9：name ≤ 64、description ≤ 1024。
- Q16：目标位于源内时硬报错。
- Q17：源含 `.git` 时照常复制 + 警告。
- Q19：目录名是 skill 唯一身份。
- Q21：符号链接统一拒绝。
- Q22：Windows 保留名提前拒绝。
- Q27：配置文件独立于 `LSM_HOME`，不放进 `.agents` 目录。
- Q29：`paths` 收 `&Env` 纯函数；`fsops`/`scan` 返回 `FsError`，不构造 `AppError`。
- Q31：自更新用原地替换，`exe` 路径与文件名不变（`fsops::replace_self` 只实现替换与回滚）。
- Q40：`pull --all` 逐条复制、冲突跳过、失败继续，任一失败退 1（拉动方向由 Q48 从 `copy` 改名）。
- Q41：`remove` 只删合法 skill 目录，`--force` 才放开；rename 到临时目录再递归删。

完整决策见 [decisions.md](../decisions.md)。
