# Release notes

> 上级：[发布规则](release.md) ｜ 相关：[测试与交付](impl/quality.md)
>
> 本文件保存发布正文模板，并用 v0.2.0 作参考样例。**不逐版本累积实例**：以后发版只复用模板，不往这里追加历史。

## 1. 模板

写作要求：

- 中文在前、English 在后，两段逐条对应，不各写一套。
- 只写用户能观察到的行为与退出码，不写内部实现、文件路径或提交列表。
- 失败语义必须写清：什么算跳过、什么算失败、什么情况退 2。

````markdown
## 中文

### <功能名>

- `lsm <命令>`：一句话说明它做什么。
- 边界与失败语义：跳过 / 失败 / 互斥，逐条写清。
- 退出码：成功 0，用法错误 2，运行时失败 1。
- 短选项写法（如有）：`lsm -x ...`。

### 其他

- 文档、验收脚本等配套改动。

## English

### <feature>

- `lsm <command>`: one-line summary.
- Boundaries and failure semantics: skips / failures / conflicts.
- Exit codes: 0 on success, 2 on usage errors, 1 on runtime failures.
- Short form (if any): `lsm -x ...`.

### Also

- Docs, acceptance script and other supporting changes.
````

## 2. 参考样例：v0.2.0

### 中文

#### `copy --all` 批量复制

- `lsm copy --all [--force]`：把全局 skills 目录下的全部合法 skill 复制到当前项目。
- 目标已存在且未加 `--force` 时记为**跳过**并继续；源非法或 I/O 失败记为**失败**并继续，逐条报错。
- 结束打印「成功 / 跳过 / 失败」计数：任一失败退出码 1，只有跳过或全部成功退出码 0，全局为空退 0 并提示。
- `--all` 与 `<name>` 互斥（同时给出退出码 2）；短选项写法 `lsm -c --all`。

#### `remove` 卸载

- `lsm remove <name> <global|project> [--force]`：从指定作用域卸载 skill。
- 无 `--force` 时只删合法 skill 目录（存在 `SKILL.md` 且其 `name` 与目录名一致）；其余必须显式加 `--force`。
- 删除是原子的：先把目录 rename 到同级 `.tmp-remove-*`，再递归删除；rename 失败时原目录保持完整。
- 目标不存在退出码 1；`name` 非法退出码 2。
- 短选项写法 `lsm -r <name> -g|-p [--force]`。

#### 其他

- 两份 README、`docs/impl/storage.md`、`docs/impl/cli.md` 与 `scripts/acceptance.sh`（9 步）同步更新。
- `upgrade` 仍为 planned，未包含在本次发布中。

### English

#### `copy --all`

- `lsm copy --all [--force]` copies every valid global skill into the current project.
- Existing targets are **skipped** unless `--force`; invalid skills or I/O errors are reported as **failures** and the batch keeps going.
- Prints `copied / skipped / failed` counts: exits 1 if any skill failed, 0 when only skipped or all succeeded, and 0 with a hint when the global scope is empty.
- `--all` and `<name>` are mutually exclusive (exit 2 when both are given). Short form: `lsm -c --all`.

#### `remove`

- `lsm remove <name> <global|project> [--force]` uninstalls a skill from the chosen scope.
- Without `--force` only a valid skill directory is deleted (`SKILL.md` present and its `name` matching the directory); anything else needs `--force`.
- Deletion is atomic: the target is renamed to a sibling `.tmp-remove-*` first, then removed; if the rename fails the original stays intact.
- A missing target exits 1; an invalid `name` exits 2.
- Short form: `lsm -r <name> -g|-p [--force]`.

#### Also

- Both READMEs, `docs/impl/storage.md`, `docs/impl/cli.md` and `scripts/acceptance.sh` (9 steps) were updated.
- `upgrade` remains planned and is not part of this release.
