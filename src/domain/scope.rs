//! 安装作用域。

/// skill 的安装位置。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scope {
    Global,
    Project,
}

impl Scope {
    /// 稳定的展示名，同时用于 CLI 入参与输出。
    pub fn label(self) -> &'static str {
        match self {
            Scope::Global => "global",
            Scope::Project => "project",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_match_cli_values() {
        assert_eq!(Scope::Global.label(), "global");
        assert_eq!(Scope::Project.label(), "project");
    }
}
