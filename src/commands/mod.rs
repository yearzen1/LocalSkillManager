//! 命令编排层：只调用 `infra` 与 `output`，不直接碰 `std::fs`。

pub mod add;
pub mod config;
pub mod copy;
pub mod list;

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
        Action::Copy { name, force } => copy::run(ctx, name, force),
    }
}
