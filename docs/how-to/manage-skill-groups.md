# 如何管理 skill 分组

分组是一组常用 skill 的名字列表。把分组保存到全局配置后，可以用一条 `lsm pull --group` 把整组 skill 拉进当前项目。

## 适用场景

- 你每次新建项目都要拉同一批 skill，不想反复输入多个名字；
- 你想把「后端常用」「文档常用」这类组合固定下来；
- 你需要查看、追加、覆盖或删除已有分组。

不适用：

- 想为每个项目保存不同的分组。分组目前存在**全局** `config.toml` 里，不是项目级配置；需要项目级差异时，请分别执行需要的 `lsm pull`。
- 想按标签或子串自动匹配 skill。分组是显式名字列表，不支持通配与嵌套。

## 前置条件

- `lsm` 已安装，`lsm --version` 有输出；
- 目标 skill 已经存在于全局目录，或你接受 `pull` 时逐条报缺失；
- 你知道自己的配置目录（默认见[配置文件参考](../reference/config.md)）。

## 步骤

### 1. 创建或整体替换分组

```bash
lsm group set frontend tdd grill
```

成功标志：输出 `group 'frontend' saved (2 skills) -> <配置路径>`。

`set` 是**整体替换**：组里最终只有这次列出的成员，顺序也按这次给出的顺序。

如果分组已存在，必须显式覆盖：

```bash
lsm group set frontend tdd grill intent-lock --force
```

不加 `--force` 会退出 1 并提示分组已存在，旧成员保持不变。

### 2. 向已有分组追加成员

```bash
lsm group add frontend conventional-commit
```

成功标志：输出中的成员数增加。组不存在时 `add` 会自动创建；已经存在的成员静默去重，不会重复写入。

### 3. 查看分组

```bash
lsm group list
```

输出一行一组：

```text
frontend: tdd, grill, conventional-commit
```

组按组名字典序排列，成员保持定义顺序；没有分组时输出 `No groups found.`。

需要机器可读输出时：

```bash
lsm group list --json
```

空结果为 `[]`，非空结果形如：

```json
[
  {
    "name": "frontend",
    "skills": [
      "tdd",
      "grill"
    ]
  }
]
```

### 4. 用分组拉取

```bash
lsm pull --group frontend
```

成功标志：逐条 `pulled ...`，最后一行形如 `pulled 2, skipped 0, failed 0`。

可以一次给出多个组：

```bash
lsm pull --group frontend --group docs
```

多个组按给出顺序展开，重复 skill 只拉一次。组内成员仍支持唯一前缀解析：组里写 `code`、全局只有 `code-review` 时，会拉取 `code-review`。`--group` 不能与 `<name>...` 或 `--all` 同时使用：

```bash
lsm pull tdd --group frontend   # 退出 2
lsm pull --group frontend --all # 退出 2
```

### 5. 删除分组

```bash
lsm group remove frontend
```

批量删除：

```bash
lsm group remove frontend docs
```

删除分组**不需要** `--force`。不存在的组逐条报错，其余照常删除；任一失败退出 1。

## 验证

```bash
lsm group list
lsm list project
```

预期：

- `group list` 里只剩你保留的分组；
- 在项目根目录执行过 `pull --group` 后，`list project` 能看到对应 skill。

## 常见问题

- **`group 'x' already exists`**：这是 `set` 的覆盖保护。确认要替换时加 `--force`；如果只是想加成员，改用 `group add`。
- **`invalid group name` / `invalid skill name`**：组名和成员名都使用 skill 名规则（小写字母、数字、单个连字符，≤64 字符，不能是 Windows 保留设备名）。命令在写盘前整体校验，失败时不会留下半个分组。
- **`group 'x' not found`**：`pull --group` 引用了不存在的组。先用 `lsm group list` 确认；该组记为一次失败，其他组继续拉取。
- **`group 'x' is empty`**：手改配置留下了空数组。用 `lsm group add x <skill>...` 补成员，或删除该组。
- **某个成员 `not found in global`**：成员名只校验格式，不要求在定义分组时已安装。先 `lsm add` 补上该 skill，再重新拉取。
- **目标已存在**：没有 `--force` 时算**跳过**，不算失败；需要覆盖时加 `lsm pull --group frontend --force`。

分组文件结构与加载时机见[配置文件参考](../reference/config.md)；作用域与分组位置的取舍见[核心概念](../explanation/concepts.md)。
