//! `push`：把当前项目的 skill 推送到全局目录（`pull` 的反向操作）。

use crate::domain::action::TransferTarget;
use crate::error::AppError;
use crate::Ctx;

use super::transfer::{self, PUSH};

pub fn run(ctx: &Ctx, target: TransferTarget, force: bool) -> Result<(), AppError> {
    transfer::run(ctx, &PUSH, target, force)
}
