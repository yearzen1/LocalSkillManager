//! `pull`：把全局 skill 复制到当前项目。

use crate::domain::action::TransferTarget;
use crate::error::AppError;
use crate::Ctx;

use super::transfer::{self, PULL};

pub fn run(ctx: &Ctx, target: TransferTarget, force: bool) -> Result<(), AppError> {
    transfer::run(ctx, &PULL, target, force)
}
