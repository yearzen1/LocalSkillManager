//! 结构化错误：只存数据与退出码，不含文案与渲染。

mod mapping;

use crate::i18n::MsgKey;

/// 错误类别决定退出码。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// 用法错误，退出码 2。
    Usage,
    /// 运行时错误，退出码 1。
    Runtime,
}

/// 一条待渲染的消息：键 + 参数。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Msg {
    pub key: MsgKey,
    pub args: Vec<String>,
}

impl Msg {
    pub fn new(key: MsgKey) -> Self {
        Self {
            key,
            args: Vec::new(),
        }
    }

    pub fn with_args(key: MsgKey, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            key,
            args: args.into_iter().map(Into::into).collect(),
        }
    }
}

/// 统一错误出口的数据形态。`anyhow` 只作为底层 `source` 保留，不直接进用户输出。
#[derive(Debug)]
pub struct AppError {
    pub kind: ErrorKind,
    pub msg: Msg,
    pub hint: Option<Msg>,
    pub source: Option<anyhow::Error>,
}

impl AppError {
    pub fn usage(key: MsgKey) -> Self {
        Self::new(ErrorKind::Usage, Msg::new(key))
    }

    pub fn runtime(key: MsgKey) -> Self {
        Self::new(ErrorKind::Runtime, Msg::new(key))
    }

    fn new(kind: ErrorKind, msg: Msg) -> Self {
        Self {
            kind,
            msg,
            hint: None,
            source: None,
        }
    }

    pub fn with_args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.msg.args = args.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_hint(mut self, key: MsgKey) -> Self {
        self.hint = Some(Msg::new(key));
        self
    }

    pub fn with_hint_args(
        mut self,
        key: MsgKey,
        args: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.hint = Some(Msg::with_args(key, args));
        self
    }

    pub fn with_source(mut self, source: anyhow::Error) -> Self {
        self.source = Some(source);
        self
    }

    /// `Usage` → 2，`Runtime` → 1。
    pub fn exit_code(&self) -> u8 {
        match self.kind {
            ErrorKind::Usage => 2,
            ErrorKind::Runtime => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_errors_exit_two_and_runtime_errors_exit_one() {
        assert_eq!(AppError::usage(MsgKey::UsageMissingScope).exit_code(), 2);
        assert_eq!(AppError::runtime(MsgKey::FsSymlink).exit_code(), 1);
    }

    #[test]
    fn carries_message_and_hint_args_in_order() {
        let err = AppError::runtime(MsgKey::SkillAlreadyExists)
            .with_args(["my-skill", "/tmp/skills/my-skill"])
            .with_hint(MsgKey::SkillAlreadyExistsHint);

        assert_eq!(err.msg.key, MsgKey::SkillAlreadyExists);
        assert_eq!(err.msg.args, ["my-skill", "/tmp/skills/my-skill"]);
        assert_eq!(
            err.hint.as_ref().unwrap().key,
            MsgKey::SkillAlreadyExistsHint
        );
        assert!(err.source.is_none());
    }

    #[test]
    fn keeps_source_chain() {
        let err = AppError::runtime(MsgKey::FsIo)
            .with_args(["path", "denied"])
            .with_source(anyhow::anyhow!("denied"));

        assert_eq!(err.source.unwrap().to_string(), "denied");
    }
}
