//! 短选项归一化：`-a/-l/-r/-g/-p` → 完整子命令形式。纯函数，不碰 I/O。

use std::ffi::OsString;

/// 归一化失败，全部是用法错误（退出码 2）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NormalizeError {
    /// 多个动作标志同时出现。
    MultipleActions,
    /// `-a` / `-r` 缺少参数。
    MissingActionValue { flag: &'static str },
    /// `-g` 与 `-p` 同时出现。
    MultipleScopes,
    /// `add` / `list` / `remove` 缺少作用域。
    MissingScope { action: &'static str },
    /// 短选项与完整子命令混用。
    MixedForms,
}

/// 把缩减版 argv 改写成完整版；无需改写时原样返回。
pub fn normalize(argv: Vec<OsString>) -> Result<Vec<OsString>, NormalizeError> {
    // `config` 子命令永远原样交给 clap。
    if argv.iter().any(|token| token == "config") {
        return Ok(argv);
    }

    let mut actions: Vec<(&'static str, Option<OsString>)> = Vec::new();
    let mut scopes: Vec<&'static str> = Vec::new();
    let mut passthrough: Vec<OsString> = Vec::new();

    let mut tokens = argv.into_iter();
    while let Some(token) = tokens.next() {
        match token.to_str() {
            Some("-a") | Some("--add") => {
                let value = tokens
                    .next()
                    .ok_or(NormalizeError::MissingActionValue { flag: "-a" })?;
                actions.push(("add", Some(value)));
            }
            Some("-r") | Some("--remove") => {
                let value = tokens
                    .next()
                    .ok_or(NormalizeError::MissingActionValue { flag: "-r" })?;
                actions.push(("remove", Some(value)));
            }
            Some("-l") | Some("--list") => actions.push(("list", None)),
            Some("-g") | Some("--global") => scopes.push("global"),
            Some("-p") | Some("--project") => scopes.push("project"),
            _ => passthrough.push(token),
        }
    }

    if actions.is_empty() && scopes.is_empty() {
        return Ok(passthrough);
    }

    if actions.len() > 1 {
        return Err(NormalizeError::MultipleActions);
    }

    if scopes.len() > 1 {
        return Err(NormalizeError::MultipleScopes);
    }

    let has_subcommand = passthrough
        .iter()
        .filter_map(|token| token.to_str())
        .any(|token| {
            matches!(
                token,
                "add" | "list" | "search" | "pull" | "push" | "remove"
            )
        });
    if has_subcommand && !(actions.is_empty() && scopes.is_empty()) {
        return Err(NormalizeError::MixedForms);
    }

    let Some((action, value)) = actions.pop() else {
        // 只有 -g/-p：交回 clap 报未知参数。
        passthrough.insert(
            0,
            OsString::from(if scopes[0] == "global" { "-g" } else { "-p" }),
        );
        return Ok(passthrough);
    };

    if scopes.is_empty() {
        return Err(NormalizeError::MissingScope { action });
    }

    let mut normalized = vec![OsString::from(action)];
    if let Some(value) = value {
        normalized.push(value);
    }
    // `remove <name>... <scope>`：scope 必须留在最后一个位置参数。
    if action == "remove" {
        normalized.extend(passthrough);
        if let Some(scope) = scopes.first() {
            normalized.push(OsString::from(*scope));
        }
    } else {
        if let Some(scope) = scopes.first() {
            normalized.push(OsString::from(*scope));
        }
        normalized.extend(passthrough);
    }

    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(values: &[&str]) -> Result<Vec<String>, NormalizeError> {
        normalize(values.iter().map(OsString::from).collect()).map(|argv| {
            argv.iter()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect()
        })
    }

    #[test]
    fn maps_add_with_scope() {
        assert_eq!(
            run(&["-a", "foo.md", "-g"]).unwrap(),
            ["add", "foo.md", "global"]
        );
        assert_eq!(
            run(&["-a", "foo.md", "-p", "--force"]).unwrap(),
            ["add", "foo.md", "project", "--force"]
        );
        assert_eq!(
            run(&["--add", "foo.md", "--global"]).unwrap(),
            ["add", "foo.md", "global"]
        );
    }

    #[test]
    fn maps_list_with_scope_and_extras() {
        assert_eq!(run(&["-l", "-g"]).unwrap(), ["list", "global"]);
        assert_eq!(
            run(&["-l", "-p", "--json"]).unwrap(),
            ["list", "project", "--json"]
        );
    }

    #[test]
    fn rejects_multiple_actions() {
        assert_eq!(
            run(&["-l", "-r", "foo"]).unwrap_err(),
            NormalizeError::MultipleActions
        );
    }

    #[test]
    fn rejects_multiple_scopes() {
        assert_eq!(
            run(&["-g", "-p"]).unwrap_err(),
            NormalizeError::MultipleScopes
        );
    }

    #[test]
    fn maps_remove_with_scope() {
        assert_eq!(
            run(&["-r", "foo", "-p"]).unwrap(),
            ["remove", "foo", "project"]
        );
        assert_eq!(
            run(&["-r", "foo", "-g", "--force"]).unwrap(),
            ["remove", "foo", "--force", "global"]
        );
        assert_eq!(
            run(&["-r", "a", "b", "-p"]).unwrap(),
            ["remove", "a", "b", "project"]
        );
    }

    #[test]
    fn rejects_missing_action_value() {
        assert_eq!(
            run(&["-a"]).unwrap_err(),
            NormalizeError::MissingActionValue { flag: "-a" }
        );
        assert_eq!(
            run(&["-r"]).unwrap_err(),
            NormalizeError::MissingActionValue { flag: "-r" }
        );
    }

    #[test]
    fn defaults_scope_to_project() {
        assert_eq!(
            run(&["-a", "foo.md"]).unwrap(),
            ["add", "foo.md", "project"]
        );
        assert_eq!(run(&["-l"]).unwrap(), ["list", "project"]);
        assert_eq!(run(&["-r", "foo"]).unwrap(), ["remove", "foo", "project"]);
        assert_eq!(
            run(&["-r", "a", "b"]).unwrap(),
            ["remove", "a", "b", "project"]
        );
        // 显式作用域仍然覆盖默认值。
        assert_eq!(
            run(&["-a", "foo.md", "-g"]).unwrap(),
            ["add", "foo.md", "global"]
        );
    }

    #[test]
    fn leaves_removed_copy_flag_untouched() {
        // `-c` 随 `copy` 一起移除：归一化不再识别，交给 clap 报未知参数。
        assert_eq!(run(&["-c", "foo"]).unwrap(), ["-c", "foo"]);
        assert_eq!(run(&["-c", "--all"]).unwrap(), ["-c", "--all"]);
    }

    #[test]
    fn rejects_mixed_forms() {
        assert_eq!(
            run(&["add", "foo.md", "-g"]).unwrap_err(),
            NormalizeError::MixedForms
        );
        assert_eq!(
            run(&["list", "-p"]).unwrap_err(),
            NormalizeError::MixedForms
        );
    }

    #[test]
    fn leaves_full_form_and_help_untouched() {
        assert_eq!(run(&["list", "global"]).unwrap(), ["list", "global"]);
        assert_eq!(run(&["--help"]).unwrap(), ["--help"]);
        assert_eq!(run(&["-V"]).unwrap(), ["-V"]);
    }

    #[test]
    fn never_rewrites_config_subcommand() {
        assert_eq!(
            run(&["config", "set", "lang", "zh"]).unwrap(),
            ["config", "set", "lang", "zh"]
        );
        assert_eq!(
            run(&["-a", "foo.md", "-g", "config"]).unwrap(),
            ["-a", "foo.md", "-g", "config"]
        );
    }
}
