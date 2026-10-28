# 存储与命令实现

> 上级：[实现方案索引](../impl.md) ｜ 相关：[CLI 与输出](cli.md) ｜ [配置与持久化](config.md) ｜ [测试与交付](quality.md) ｜ [决策记录](../decisions.md)
>
> 本文拥有以下规则的完整定义：作用域与路径、SKILL.md 规范、add/list/copy、文件系统操作。其他文档只做摘要和链接。

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
- `Env::read()` 一次读全三个变量，返回快照；空串按未设置处理，`LSM_LANG` 非 UTF-8 同样按未设置。
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
    pub home: Option<PathBuf>,        // LSM_HOME，否则 ~/.agents
    pub config_dir: Option<PathBuf>,  // LSM_CONFIG_DIR，否则 dirs::config_dir()/lsm
    pub lang: Option<String>,         // LSM_LANG 原始值，空串视为未设置
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
- 不做正则依赖：`name` 校验用字符遍历（首尾必须是字母/数字，中间允许单个连字符）。
- Windows 保留设备名（大小写不敏感）：`con`、`prn`、`aux`、`nul`、`com1`~`com9`、`lpt1`~`lpt9`。提前拒绝，保证三平台行为一致。
- **目录名是 skill 的唯一身份**：`SKILL.md` 的 `name` 必须与所在目录名一致；不一致时 `list` 标记 `[invalid]`，`copy` 视为源非法。
- `copy <name>` 的入参必须先通过同一套 name 校验（含长度、保留名），并额外拒绝 `/`、`\`、`.`、`..`，校验失败按用法错误退出 2，绝不先拼路径再校验。

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

- 文本输出前把 description 的 `\r`、`\n`、连续空白压成单个空格。
- 按 Unicode 字符截断到 60，超出加 `...`。
- JSON 输出返回原始 description（由 JSON 自行转义），不做截断。

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

文本输出（description 超过 60 字符截断为 `...`）：

```
NAME        DESCRIPTION
----------  ------------------------------------------
code-review Review a diff for correctness and style...
my-skill    One-line description.
broken      [invalid] missing SKILL.md
```

- 固定 60 字符上限，不做终端宽度探测，保证脚本输出稳定。
- name 列宽取 `max(10, 最长名称字符数)`，分隔线随列宽生成。
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

### 3.3 copy

```
lsm copy <name> [--force]
```

流程：

1. **先校验 `name`**（不拼路径）：同一套 name 规则 + 拒绝 `/`、`\`、`.`、`..`。校验在 `cli::parse` 构造 `Action` 时执行，保证在命令逻辑之前失败，退出码 2：
   ```
   error: invalid skill name '..'
   hint: skill names use lowercase letters, digits and single hyphens
   ```
2. 源目录：`infra::paths::skills_dir(&ctx.env, Scope::Global, ctx.cwd)/<name>`，不存在则报错：
   ```
   error: skill 'foo' not found in global
   hint: run `lsm list global` to see available skills
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

copy 同样执行 `.git` 警告与自包含检查（目标在源内时拒绝）。

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

pub enum SourceKind {
    Directory,
    File,
}

/// 备份删除失败时把路径交回命令层，由命令层打 warning 并仍以 0 退出。
pub struct InstallReport {
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

完整决策见 [decisions.md](../decisions.md)。
