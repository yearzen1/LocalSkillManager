//! `list`：扫描指定作用域并输出文本表格或 JSON。

use crate::domain::scope::Scope;
use crate::error::AppError;
use crate::infra::{paths, scan};
use crate::output;
use crate::Ctx;

pub fn run(ctx: &Ctx, scope: Scope, json: bool) -> Result<(), AppError> {
    let directory = paths::skills_dir(&ctx.env, scope, &ctx.cwd)?;
    let entries = scan::scan_skills(&directory)?;

    if json {
        output::emit_stdout(&format!("{}\n", output::list::render_json(&entries)));
    } else if entries.is_empty() {
        output::emit_stdout(&format!(
            "{}\n",
            output::list::empty_message(ctx.lang, &directory)
        ));
    } else {
        output::emit_stdout(&output::list::render_text(ctx.lang, &entries));
    }

    Ok(())
}
