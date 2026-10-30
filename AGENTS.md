## 使用tdd、grill、intent-lock、conventional-commit skill

## 改 CLI 的顺序（命令与参数）

命令与参数的唯一真源是 `docs/impl/cli.md` §2.1 的状态表。改动 CLI 时必须按此顺序，顺序颠倒即为文档漂移：

1. 先改 `docs/impl/cli.md` §2.1 的状态列：新功能写 `planned`，实现完成后改成 `implemented`。
2. 再改 `src/cli/spec.rs`（clap 结构）与 `src/domain/action.rs`（`Action`），文案走 `src/i18n/{en,zh}.rs`。
3. 再补 `tests/cli.rs` 的用例；改完实现后同步把状态列改成 `implemented`。
4. 最后改两份 README，且**只写 `implemented` 的命令**，不描述 `planned` 功能。

`tests/cli.rs::cli_reference_table_matches_binary` 会强制这条约定：`implemented` 行必须 `lsm <command> --help` 退出 0，`planned` 行必须是未知子命令（退出 2）。状态列写错，`cargo test` 就会红。
