//! 命令编排层：只调用 `infra` 与 `output`，不直接碰 `std::fs`。

pub mod add;
pub mod config;
pub mod list;
pub mod pull;
pub mod push;
pub mod remove;
pub mod search;
pub mod upgrade;

use crate::domain::action::Action;
use crate::error::AppError;
use crate::Ctx;

/// 执行一个已解析的动作。
pub fn dispatch(ctx: &Ctx, action: Action) -> Result<(), AppError> {
    match action {
        Action::Add {
            source,
            scope,
            force,
        } => add::run(ctx, source, scope, force),
        Action::Config(command) => config::run(ctx, command),
        Action::List { scope, json } => list::run(ctx, scope, json),
        Action::Search { query, scope, json } => search::run(ctx, &query, scope, json),
        Action::Pull { target, force } => pull::run(ctx, target, force),
        Action::Push { target, force } => push::run(ctx, target, force),
        Action::Remove {
            scope,
            names,
            force,
        } => remove::run(ctx, scope, names, force),
        Action::Upgrade { check, force } => upgrade::run(ctx, check, force),
    }
}
