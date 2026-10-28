//! 命令编排层：只调用 `infra` 与 `output`，不直接碰 `std::fs`。

pub mod config;

use crate::domain::action::Action;
use crate::error::AppError;
use crate::Ctx;

/// 执行一个已解析的动作。
pub fn dispatch(ctx: &Ctx, action: Action) -> Result<(), AppError> {
    match action {
        Action::Config(command) => config::run(ctx, command),
        // add / list / copy 在后续切片接入。
        Action::Add { .. } | Action::List { .. } | Action::Copy { .. } => Ok(()),
    }
}
