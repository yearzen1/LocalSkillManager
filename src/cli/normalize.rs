//! 短选项归一化：`-a/-l/-c/-g/-p` → 完整子命令形式。纯函数，不碰 I/O。

use std::ffi::OsString;

/// 归一化失败，全部是用法错误（退出码 2）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NormalizeError {
    /// 多个动作标志同时出现。
    MultipleActions,
    /// `-a` / `-c` 缺少参数。
    MissingActionValue { flag: &'static str },
    /// `-g` 与 `-p` 同时出现。
    MultipleScopes,
    /// `add` / `list` 缺少作用域。
    MissingScope { action: &'static str },
    /// `-c` 与 `-g` / `-p` 同现。
    CopyWithScope,
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
            Some("-c") | Some("--copy") => {
                let value = tokens
                    .next()
                    .ok_or(NormalizeError::MissingActionValue { flag: "-c" })?;
                actions.push(("copy", Some(value)));
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
        .any(|token| matches!(token, "add" | "list" | "copy"));
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

    if action == "copy" && !scopes.is_empty() {
        return Err(NormalizeError::CopyWithScope);
    }

    if action != "copy" && scopes.is_empty() {
        return Err(NormalizeError::MissingScope { action });
    }

    let mut normalized = vec![OsString::from(action)];
    if let Some(value) = value {
        normalized.push(value);
    }
    if let Some(scope) = scopes.first() {
        normalized.push(OsString::from(*scope));
    }
    normalized.extend(passthrough);

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
    fn maps_copy_without_scope() {
        assert_eq!(run(&["-c", "foo"]).unwrap(), ["copy", "foo"]);
        assert_eq!(
            run(&["-c", "foo", "--force"]).unwrap(),
            ["copy", "foo", "--force"]
        );
    }

    #[test]
    fn rejects_multiple_actions() {
        assert_eq!(
            run(&["-l", "-c", "foo"]).unwrap_err(),
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
    fn rejects_missing_action_value() {
        assert_eq!(
            run(&["-a"]).unwrap_err(),
            NormalizeError::MissingActionValue { flag: "-a" }
        );
        assert_eq!(
            run(&["-c"]).unwrap_err(),
            NormalizeError::MissingActionValue { flag: "-c" }
        );
    }

    #[test]
    fn rejects_missing_scope() {
        assert_eq!(
            run(&["-a", "foo.md"]).unwrap_err(),
            NormalizeError::MissingScope { action: "add" }
        );
        assert_eq!(
            run(&["-l"]).unwrap_err(),
            NormalizeError::MissingScope { action: "list" }
        );
    }

    #[test]
    fn rejects_scope_for_copy() {
        assert_eq!(
            run(&["-c", "foo", "-g"]).unwrap_err(),
            NormalizeError::CopyWithScope
        );
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
