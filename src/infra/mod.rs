//! 所有文件系统与环境访问。错误用本层枚举表达，不构造 `AppError`、不写文案。

pub mod config;
pub mod env;
pub mod frontmatter;
pub mod fsops;
pub mod paths;
pub mod scan;
