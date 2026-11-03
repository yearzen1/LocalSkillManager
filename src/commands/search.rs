//! `search`：按 name / description 子串搜索某个作用域的 skill。

use crate::domain::scope::Scope;
use crate::error::{AppError, Msg};
use crate::i18n::MsgKey;
use crate::infra::{paths, scan};
use crate::output;
use crate::Ctx;

pub fn run(ctx: &Ctx, query: &str, scope: Scope, json: bool) -> Result<(), AppError> {
    let directory = paths::skills_dir(&ctx.env, scope, &ctx.cwd)?;
    let entries = scan::scan_skills(&directory)?;

    let needle = query.to_lowercase();
    let matches: Vec<_> = entries
        .into_iter()
        .filter(|entry| {
            entry.name.to_lowercase().contains(&needle)
                || entry.description.to_lowercase().contains(&needle)
        })
        .collect();

    if json {
        output::emit_stdout(&format!("{}\n", output::list::render_json(&matches)));
    } else if matches.is_empty() {
        output::emit_line(
            ctx.lang,
            &Msg::with_args(MsgKey::SearchNoMatches, [query, scope.label()]),
        );
    } else {
        output::emit_stdout(&output::list::render_text(ctx.lang, &matches));
    }

    Ok(())
}
