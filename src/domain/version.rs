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
    pub fn parse(input: &str) -> Result<Self, VersionError> {
        let body = input.strip_prefix('v').unwrap_or(input);
        let mut parts = body.split('.');

        let major = parse_segment(parts.next(), input)?;
        let minor = parse_segment(parts.next(), input)?;
        let patch = parse_segment(parts.next(), input)?;
        if parts.next().is_some() {
            return Err(VersionError::Format {
                input: input.to_string(),
            });
        }

        Ok(Self {
            major,
            minor,
            patch,
        })
    }

    /// 规范化输出，不带 `v` 前缀（与 `CARGO_PKG_VERSION` 形式一致）。
    pub fn tag(&self) -> String {
        format!("v{self}")
    }
}

/// 单个数值段：必须非空且全是 ASCII 数字。
fn parse_segment(segment: Option<&str>, input: &str) -> Result<u64, VersionError> {
    let Some(segment) = segment else {
        return Err(VersionError::Format {
            input: input.to_string(),
        });
    };

    if segment.is_empty() || !segment.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(VersionError::Format {
            input: input.to_string(),
        });
    }

    segment.parse().map_err(|_| VersionError::TooLarge {
        input: input.to_string(),
    })
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
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
            "",
            "v",
            "v0.3",
            "0.3.0.1",
            "v0.3.0-rc.1",
            "v0.3.0+build",
            "latest",
            "v0.3.x",
            "V0.3.0",
            " v0.3.0",
            "v0.3.0 ",
            "0..0",
            "v1.-1.0",
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
        assert_eq!(
            parse("v0.3.0").cmp(&parse("0.3.0")),
            std::cmp::Ordering::Equal
        );
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
