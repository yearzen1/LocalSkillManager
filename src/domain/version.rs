//! release 版本号：`vMAJOR.MINOR.PATCH` 的解析与比较。
//!
//! 只做纯计算，不读文件、不读环境；当前版本由调用方以 `CARGO_PKG_VERSION` 传入。

use std::fmt;

/// 已解析的三段版本号；前导零按数值归一。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    major: u64,
    minor: u64,
    patch: u64,
}

/// 版本号解析失败。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionError {
    /// 不是 `[v]MAJOR.MINOR.PATCH`：段数不对、含非数字、带后缀或前后空白。
    Format { input: String },
    /// 某一段超出 `u64` 范围。
    TooLarge { input: String },
}

impl Version {
    /// 解析 `v0.3.0` 或 `0.3.0`；其余形式一律拒绝，不猜测、不截断。
    pub fn parse(_input: &str) -> Result<Self, VersionError> {
        unimplemented!("parse release version")
    }

    /// 规范化输出，不带 `v` 前缀（与 `CARGO_PKG_VERSION` 形式一致）。
    pub fn tag(&self) -> String {
        format!("v{self}")
    }
}

impl fmt::Display for Version {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unimplemented!("render release version")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> Version {
        Version::parse(input).expect("should be a valid release version")
    }

    #[test]
    fn accepts_with_and_without_the_v_prefix() {
        for input in ["v0.3.0", "0.3.0"] {
            let version = parse(input);
            assert_eq!(version.to_string(), "0.3.0");
            assert_eq!(version.tag(), "v0.3.0");
        }
    }

    #[test]
    fn rejects_everything_but_three_numeric_parts() {
        for input in [
            "", "v", "v0.3", "0.3.0.1", "v0.3.0-rc.1", "v0.3.0+build", "latest", "v0.3.x",
            "V0.3.0", " v0.3.0", "v0.3.0 ", "0..0", "v1.-1.0",
        ] {
            assert_eq!(
                Version::parse(input),
                Err(VersionError::Format {
                    input: input.to_string()
                })
            );
        }
    }

    #[test]
    fn compares_numeric_segments_in_order() {
        assert!(parse("v0.3.0") < parse("v0.10.0"));
        assert!(parse("v0.9.9") < parse("v0.10.0"));
        assert!(parse("v1.0.0") > parse("v0.99.99"));
        assert!(parse("v0.3.1") > parse("v0.3.0"));
        assert_eq!(parse("v0.3.0").cmp(&parse("0.3.0")), std::cmp::Ordering::Equal);
    }

    #[test]
    fn treats_leading_zeros_as_numbers() {
        assert_eq!(parse("v0.02.0"), parse("0.2.0"));
        assert_eq!(parse("v0000000001.0.0"), parse("v1.0.0"));
        assert_eq!(parse("v0.02.0").tag(), "v0.2.0");
    }

    #[test]
    fn rejects_segments_beyond_u64() {
        let input = "v18446744073709551616.0.0";
        assert_eq!(
            Version::parse(input),
            Err(VersionError::TooLarge {
                input: input.to_string()
            })
        );
    }
}
