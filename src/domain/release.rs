//! release 元数据与平台资产选择：纯数据，不联网、不写盘。

use serde_json::Value;

use crate::domain::version::{Version, VersionError};

/// 平台到资产名的映射（`upgrade.md` §2.2）。`os`/`arch` 取 `std::env::consts` 的原始值。
pub fn asset_name(_os: &str, _arch: &str) -> Result<String, ReleaseError> {
    unimplemented!("map platform to asset name")
}

/// 一个可下载的 release 资产。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    pub name: String,
    pub url: String,
}

/// 一次 release 的元数据：只保留升级需要的字段。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    tag: Version,
    assets: Vec<Asset>,
    duplicate_assets: Vec<String>,
}

/// 元数据解析或平台映射失败。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReleaseError {
    /// 响应不是合法 JSON，或顶层不是对象。
    Json { detail: String },
    /// 缺少必需字段。
    MissingField { field: String },
    /// 字段存在但类型不对。
    InvalidField { field: String },
    /// `tag_name` 不是合法的 release 版本号。
    InvalidTag { source: VersionError },
    /// 平台不在命名表内。
    UnsupportedPlatform { os: String, arch: String },
}

impl Release {
    /// 解析 GitHub release 元数据；未知字段忽略。
    pub fn parse_metadata(_json: &str) -> Result<Self, ReleaseError> {
        unimplemented!("parse release metadata")
    }

    /// release 的版本号。
    pub fn tag(&self) -> Version {
        self.tag
    }

    /// 按资产名精确查找；不存在返回 `None`。
    pub fn asset(&self, name: &str) -> Option<&Asset> {
        self.assets.iter().find(|asset| asset.name == name)
    }

    /// 重复出现的资产名（保留首次出现顺序）；只取第一个，不阻断升级。
    pub fn duplicate_assets(&self) -> &[String] {
        &self.duplicate_assets
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_every_documented_platform_to_an_asset_name() {
        let cases = [
            ("windows", "x86_64", "lsm-windows-x64.exe"),
            ("windows", "aarch64", "lsm-windows-arm64.exe"),
            ("linux", "x86_64", "lsm-linux-x64"),
            ("linux", "aarch64", "lsm-linux-arm64"),
            ("macos", "aarch64", "lsm-macos-arm64"),
            ("macos", "x86_64", "lsm-macos-x64"),
        ];

        for (os, arch, expected) in cases {
            assert_eq!(asset_name(os, arch), Ok(expected.to_string()), "{os}/{arch}");
        }
    }

    #[test]
    fn rejects_platforms_outside_the_table() {
        for (os, arch) in [
            ("freebsd", "x86_64"),
            ("windows", "x86"),
            ("Windows", "x86_64"),
            ("", ""),
        ] {
            assert_eq!(
                asset_name(os, arch),
                Err(ReleaseError::UnsupportedPlatform {
                    os: os.to_string(),
                    arch: arch.to_string(),
                })
            );
        }
    }

    #[test]
    fn parses_tag_and_assets_and_ignores_unknown_fields() {
        let json = r#"{
            "tag_name": "v0.3.0",
            "draft": false,
            "assets": [
                {
                    "name": "lsm-windows-x64.exe",
                    "browser_download_url": "https://example.test/lsm-windows-x64.exe",
                    "size": 1024
                }
            ]
        }"#;

        let release = Release::parse_metadata(json).expect("should parse");

        assert_eq!(release.tag().tag(), "v0.3.0");
        assert_eq!(
            release.asset("lsm-windows-x64.exe"),
            Some(&Asset {
                name: "lsm-windows-x64.exe".to_string(),
                url: "https://example.test/lsm-windows-x64.exe".to_string(),
            })
        );
        assert!(release.asset("lsm-linux-x64").is_none());
        assert!(release.duplicate_assets().is_empty());
    }

    #[test]
    fn rejects_non_json_bodies() {
        for body in ["", "not json", "[]"] {
            assert!(
                matches!(
                    Release::parse_metadata(body),
                    Err(ReleaseError::Json { .. })
                ),
                "{body}"
            );
        }
    }

    #[test]
    fn reports_missing_fields_by_name() {
        let cases = [
            (r#"{ "assets": [] }"#, "tag_name"),
            (r#"{ "tag_name": "v0.3.0" }"#, "assets"),
            (
                r#"{ "tag_name": "v0.3.0", "assets": [ { "browser_download_url": "https://example.test/x" } ] }"#,
                "assets[0].name",
            ),
            (
                r#"{ "tag_name": "v0.3.0", "assets": [ { "name": "lsm-windows-x64.exe" } ] }"#,
                "assets[0].browser_download_url",
            ),
        ];

        for (json, field) in cases {
            assert_eq!(
                Release::parse_metadata(json),
                Err(ReleaseError::MissingField {
                    field: field.to_string()
                }),
                "{json}"
            );
        }
    }

    #[test]
    fn reports_fields_with_the_wrong_type() {
        let cases = [
            (r#"{ "tag_name": 3, "assets": [] }"#, "tag_name"),
            (r#"{ "tag_name": "v0.3.0", "assets": {} }"#, "assets"),
            (r#"{ "tag_name": "v0.3.0", "assets": [ 1 ] }"#, "assets[0]"),
            (
                r#"{ "tag_name": "v0.3.0", "assets": [ { "name": 1, "browser_download_url": "https://example.test/x" } ] }"#,
                "assets[0].name",
            ),
        ];

        for (json, field) in cases {
            assert_eq!(
                Release::parse_metadata(json),
                Err(ReleaseError::InvalidField {
                    field: field.to_string()
                }),
                "{json}"
            );
        }
    }

    #[test]
    fn rejects_tags_that_are_not_release_versions() {
        let json = r#"{ "tag_name": "v0.3", "assets": [] }"#;

        assert_eq!(
            Release::parse_metadata(json),
            Err(ReleaseError::InvalidTag {
                source: VersionError::Format {
                    input: "v0.3".to_string()
                }
            })
        );
    }

    #[test]
    fn keeps_the_first_of_duplicate_assets_and_records_the_rest() {
        let json = r#"{
            "tag_name": "v0.3.0",
            "assets": [
                { "name": "lsm-windows-x64.exe", "browser_download_url": "https://example.test/first" },
                { "name": "lsm-windows-x64.exe", "browser_download_url": "https://example.test/second" }
            ]
        }"#;

        let release = Release::parse_metadata(json).expect("should parse");

        assert_eq!(
            release.asset("lsm-windows-x64.exe").map(|asset| &asset.url[..]),
            Some("https://example.test/first")
        );
        assert_eq!(release.duplicate_assets(), ["lsm-windows-x64.exe"]);
    }
}
