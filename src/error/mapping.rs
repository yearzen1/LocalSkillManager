//! 唯一的「各层窄错误 → `AppError`」适配点，hint 也在这里配置。

use crate::domain::lang::LangError;
use crate::domain::meta::MetaError;
use crate::domain::name::NameError;
use crate::domain::release::ReleaseError;
use crate::domain::version::VersionError;
use crate::i18n::MsgKey;
use crate::infra::config::ConfigError;
use crate::infra::frontmatter::FrontmatterError;
use crate::infra::fsops::FsError;
use crate::infra::net::NetError;
use crate::infra::paths::PathError;

use super::AppError;

impl From<LangError> for AppError {
    fn from(err: LangError) -> Self {
        match err {
            LangError::Unsupported(raw) => AppError::usage(MsgKey::UsageInvalidLanguage)
                .with_args([raw])
                .with_hint(MsgKey::UsageInvalidLanguageHint),
        }
    }
}

/// `copy` 的 name 入参校验：用法错误。
impl From<NameError> for AppError {
    fn from(err: NameError) -> Self {
        match err {
            NameError::Format { input } => AppError::usage(MsgKey::SkillNameInvalid)
                .with_args([input])
                .with_hint(MsgKey::SkillNameInvalidHint),
            NameError::TooLong { len, .. } => {
                AppError::usage(MsgKey::SkillNameTooLong).with_args([len.to_string()])
            }
            NameError::Reserved { input } => {
                AppError::usage(MsgKey::SkillNameReserved).with_args([input])
            }
        }
    }
}

impl From<MetaError> for AppError {
    fn from(err: MetaError) -> Self {
        match err {
            MetaError::DescriptionEmpty => AppError::runtime(MsgKey::SkillDescriptionEmpty),
            MetaError::DescriptionTooLong { len } => {
                AppError::runtime(MsgKey::SkillDescriptionTooLong).with_args([len.to_string()])
            }
        }
    }
}

impl From<FrontmatterError> for AppError {
    fn from(err: FrontmatterError) -> Self {
        match err {
            FrontmatterError::Io { path, source } => {
                let detail = source.to_string();
                AppError::runtime(MsgKey::FsIo)
                    .with_args([path.display().to_string(), detail])
                    .with_source(source)
            }
            FrontmatterError::InvalidUtf8 { path } => {
                AppError::runtime(MsgKey::SkillFrontmatterUtf8)
                    .with_args([path.display().to_string()])
            }
            FrontmatterError::MissingFrontmatter { path } => {
                AppError::runtime(MsgKey::SkillFrontmatterMissing)
                    .with_args([path.display().to_string()])
                    .with_hint(MsgKey::SkillFrontmatterMissingHint)
            }
            FrontmatterError::MissingField { path, field } => {
                AppError::runtime(MsgKey::SkillFieldMissing)
                    .with_args([path.display().to_string(), field.to_string()])
            }
            FrontmatterError::InvalidYaml { path, source } => {
                let detail = source.to_string();
                AppError::runtime(MsgKey::SkillFrontmatterYaml)
                    .with_args([path.display().to_string(), detail])
                    .with_hint(MsgKey::SkillFrontmatterYamlHint)
                    .with_source(source)
            }
            FrontmatterError::InvalidName { source, .. } => name_error_runtime(source),
            FrontmatterError::InvalidMeta { source, .. } => source.into(),
        }
    }
}

/// frontmatter 里的 name 非法属于运行时错误（文件内容问题），与 `copy` 入参的用法错误区分。
fn name_error_runtime(err: NameError) -> AppError {
    match err {
        NameError::Format { input } => AppError::runtime(MsgKey::SkillNameInvalid)
            .with_args([input])
            .with_hint(MsgKey::SkillNameInvalidHint),
        NameError::TooLong { len, .. } => {
            AppError::runtime(MsgKey::SkillNameTooLong).with_args([len.to_string()])
        }
        NameError::Reserved { input } => {
            AppError::runtime(MsgKey::SkillNameReserved).with_args([input])
        }
    }
}

impl From<FsError> for AppError {
    fn from(err: FsError) -> Self {
        match err {
            FsError::Io { path, source } => {
                let detail = source.to_string();
                AppError::runtime(MsgKey::FsIo)
                    .with_args([path.display().to_string(), detail])
                    .with_source(source)
            }
            FsError::SymlinkUnsupported { path } => {
                AppError::runtime(MsgKey::FsSymlink).with_args([path.display().to_string()])
            }
            FsError::AlreadyExists { path } => {
                let name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default();
                AppError::runtime(MsgKey::SkillAlreadyExists)
                    .with_args([name, path.display().to_string()])
                    .with_hint(MsgKey::SkillAlreadyExistsHint)
            }
            FsError::SourceMissing { path } => {
                AppError::runtime(MsgKey::FsSourceMissing).with_args([path.display().to_string()])
            }
            FsError::SourceNotExpectedType { path } => {
                AppError::runtime(MsgKey::FsSourceType).with_args([path.display().to_string()])
            }
        }
    }
}

impl From<PathError> for AppError {
    fn from(err: PathError) -> Self {
        match err {
            PathError::HomeUnavailable => AppError::runtime(MsgKey::PathHomeUnavailable)
                .with_hint(MsgKey::PathHomeUnavailableHint),
            PathError::ConfigDirUnavailable => AppError::runtime(MsgKey::PathConfigDirUnavailable)
                .with_hint(MsgKey::PathConfigDirUnavailableHint),
        }
    }
}

impl From<ConfigError> for AppError {
    fn from(err: ConfigError) -> Self {
        match err {
            ConfigError::Path(path_error) => path_error.into(),
            ConfigError::Io { path, source } => {
                let detail = source.to_string();
                AppError::runtime(MsgKey::FsIo)
                    .with_args([path.display().to_string(), detail])
                    .with_source(source)
            }
            ConfigError::Write { path, source } => {
                let detail = source.to_string();
                AppError::runtime(MsgKey::ConfigWriteFailed)
                    .with_args([format!("{}: {detail}", path.display())])
                    .with_source(source)
            }
            ConfigError::Parse { path, source } => {
                let shown = path.display().to_string();
                AppError::runtime(MsgKey::ConfigParseFailed)
                    .with_args([shown.clone()])
                    .with_hint_args(MsgKey::ConfigFixHint, [shown])
                    .with_source(source)
            }
            ConfigError::InvalidValue { path, raw } => {
                let shown = path.display().to_string();
                AppError::runtime(MsgKey::ConfigInvalidValue)
                    .with_args([raw])
                    .with_hint_args(MsgKey::ConfigFixHint, [shown])
            }
        }
    }
}

/// release tag 不合法属于运行时错误（发布端数据问题），与 CLI 入参无关。
impl From<VersionError> for AppError {
    fn from(err: VersionError) -> Self {
        let input = match err {
            VersionError::Format { input } | VersionError::TooLarge { input } => input,
        };
        AppError::runtime(MsgKey::UpgradeInvalidVersion)
            .with_args([input])
            .with_hint(MsgKey::UpgradeInvalidVersionHint)
    }
}

/// 取元数据失败：限流单独提示，其余带上原因。
impl From<NetError> for AppError {
    fn from(err: NetError) -> Self {
        match err {
            NetError::RateLimited => rate_limited(),
            NetError::Parse { source } => release_error(source),
            other => fetch_failed(net_failure_detail(&other)),
        }
    }
}

fn rate_limited() -> AppError {
    AppError::runtime(MsgKey::UpgradeRateLimited).with_hint(MsgKey::UpgradeRateLimitedHint)
}

fn fetch_failed(detail: String) -> AppError {
    AppError::runtime(MsgKey::UpgradeFetchFailed)
        .with_args([detail])
        .with_hint(MsgKey::UpgradeFetchFailedHint)
}

/// 下载失败：带上资产名，便于用户按名字手动下载。
pub(crate) fn download_failed(asset: &str, err: NetError) -> AppError {
    match err {
        NetError::RateLimited => rate_limited(),
        other => AppError::runtime(MsgKey::UpgradeDownloadFailed)
            .with_args([asset.to_string(), net_failure_detail(&other)])
            .with_hint(MsgKey::UpgradeDownloadFailedHint),
    }
}

/// 冒烟校验失败：文案固定，不暴露子进程细节（`upgrade.md` §7）。
pub(crate) fn verification_failed() -> AppError {
    AppError::runtime(MsgKey::UpgradeVerifyFailed).with_hint(MsgKey::UpgradeVerifyFailedHint)
}

fn net_failure_detail(err: &NetError) -> String {
    match err {
        NetError::Status { code } => format!("HTTP {code}"),
        NetError::Network { detail } => detail.clone(),
        NetError::RateLimited => "HTTP 403".to_string(),
        NetError::Parse { source } => match source {
            ReleaseError::Json { detail } => detail.clone(),
            ReleaseError::MissingField { field } => format!("missing field `{field}`"),
            ReleaseError::InvalidField { field } => format!("invalid field `{field}`"),
            ReleaseError::InvalidTag { source } => format!("invalid release version: {source:?}"),
            ReleaseError::UnsupportedPlatform { os, arch } => {
                format!("unsupported platform {os}-{arch}")
            }
        },
    }
}

/// 元数据本身的问题都归到「取不到 release」，只有 tag 不合法有专门文案。
fn release_error(err: ReleaseError) -> AppError {
    match err {
        ReleaseError::InvalidTag { source } => source.into(),
        ReleaseError::Json { detail } => fetch_failed(detail),
        ReleaseError::MissingField { field } => fetch_failed(format!("missing field `{field}`")),
        ReleaseError::InvalidField { field } => fetch_failed(format!("invalid field `{field}`")),
        // 资产选择尚未接入，先按取元数据失败处理，等拼接资产名时再细化。
        ReleaseError::UnsupportedPlatform { os, arch } => {
            fetch_failed(format!("unsupported platform {os}-{arch}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::error::ErrorKind;

    #[test]
    fn copy_name_errors_are_usage_errors() {
        let app: AppError = NameError::Format {
            input: "..".to_string(),
        }
        .into();

        assert_eq!(app.kind, ErrorKind::Usage);
        assert_eq!(app.exit_code(), 2);
        assert_eq!(app.msg.key, MsgKey::SkillNameInvalid);
        assert_eq!(app.msg.args, [".."]);
        assert_eq!(
            app.hint.as_ref().map(|hint| hint.key),
            Some(MsgKey::SkillNameInvalidHint)
        );
    }

    #[test]
    fn frontmatter_name_errors_are_runtime_errors() {
        let app: AppError = FrontmatterError::InvalidName {
            path: PathBuf::from("SKILL.md"),
            source: NameError::Reserved {
                input: "con".to_string(),
            },
        }
        .into();

        assert_eq!(app.kind, ErrorKind::Runtime);
        assert_eq!(app.exit_code(), 1);
        assert_eq!(app.msg.key, MsgKey::SkillNameReserved);
    }

    #[test]
    fn missing_frontmatter_carries_template_hint() {
        let app: AppError = FrontmatterError::MissingFrontmatter {
            path: PathBuf::from("SKILL.md"),
        }
        .into();

        assert_eq!(app.msg.key, MsgKey::SkillFrontmatterMissing);
        assert_eq!(
            app.hint.as_ref().map(|hint| hint.key),
            Some(MsgKey::SkillFrontmatterMissingHint)
        );
    }

    #[test]
    fn missing_field_names_the_field() {
        let app: AppError = FrontmatterError::MissingField {
            path: PathBuf::from("SKILL.md"),
            field: "description",
        }
        .into();

        assert_eq!(app.msg.key, MsgKey::SkillFieldMissing);
        assert_eq!(app.msg.args, ["SKILL.md", "description"]);
    }

    #[test]
    fn symlink_and_source_errors_keep_paths() {
        let app: AppError = FsError::SymlinkUnsupported {
            path: PathBuf::from("/tmp/link"),
        }
        .into();
        assert_eq!(app.msg.key, MsgKey::FsSymlink);
        assert_eq!(app.msg.args, ["/tmp/link"]);

        let app: AppError = FsError::SourceMissing {
            path: PathBuf::from("/tmp/none"),
        }
        .into();
        assert_eq!(app.msg.key, MsgKey::FsSourceMissing);
    }

    #[test]
    fn already_exists_hint_points_to_force() {
        let app: AppError = FsError::AlreadyExists {
            path: PathBuf::from("/tmp/skills/my-skill"),
        }
        .into();

        assert_eq!(app.msg.key, MsgKey::SkillAlreadyExists);
        assert_eq!(app.msg.args, ["my-skill", "/tmp/skills/my-skill"]);
        assert_eq!(
            app.hint.as_ref().map(|hint| hint.key),
            Some(MsgKey::SkillAlreadyExistsHint)
        );
    }

    #[test]
    fn path_errors_hint_the_override_variable() {
        let app: AppError = PathError::HomeUnavailable.into();
        assert_eq!(app.msg.key, MsgKey::PathHomeUnavailable);
        assert_eq!(
            app.hint.as_ref().map(|hint| hint.key),
            Some(MsgKey::PathHomeUnavailableHint)
        );

        let app: AppError = PathError::ConfigDirUnavailable.into();
        assert_eq!(app.msg.key, MsgKey::PathConfigDirUnavailable);
    }

    #[test]
    fn invalid_yaml_hints_the_fix() {
        let app: AppError = FrontmatterError::InvalidYaml {
            path: PathBuf::from("SKILL.md"),
            source: anyhow::anyhow!("mapping values are not allowed in this context"),
        }
        .into();

        assert_eq!(app.msg.key, MsgKey::SkillFrontmatterYaml);
        assert_eq!(
            app.hint.as_ref().map(|hint| hint.key),
            Some(MsgKey::SkillFrontmatterYamlHint)
        );
    }

    #[test]
    fn lang_errors_are_usage_errors_with_allowed_values() {
        let app: AppError = LangError::Unsupported("fr".to_string()).into();

        assert_eq!(app.kind, ErrorKind::Usage);
        assert_eq!(app.msg.key, MsgKey::UsageInvalidLanguage);
        assert_eq!(
            app.hint.as_ref().map(|hint| hint.key),
            Some(MsgKey::UsageInvalidLanguageHint)
        );
    }

    #[test]
    fn invalid_release_versions_are_runtime_errors_with_the_raw_input() {
        let app: AppError = VersionError::Format {
            input: "v0.3.0-rc.1".to_string(),
        }
        .into();

        assert_eq!(app.kind, ErrorKind::Runtime);
        assert_eq!(app.exit_code(), 1);
        assert_eq!(app.msg.key, MsgKey::UpgradeInvalidVersion);
        assert_eq!(app.msg.args, ["v0.3.0-rc.1"]);
        assert_eq!(
            app.hint.as_ref().map(|hint| hint.key),
            Some(MsgKey::UpgradeInvalidVersionHint)
        );

        let app: AppError = VersionError::TooLarge {
            input: "v18446744073709551616.0.0".to_string(),
        }
        .into();
        assert_eq!(app.msg.args, ["v18446744073709551616.0.0"]);
    }

    #[test]
    fn rate_limits_get_their_own_message() {
        let app: AppError = NetError::RateLimited.into();

        assert_eq!(app.exit_code(), 1);
        assert_eq!(app.msg.key, MsgKey::UpgradeRateLimited);
        assert_eq!(
            app.hint.as_ref().map(|hint| hint.key),
            Some(MsgKey::UpgradeRateLimitedHint)
        );
    }

    #[test]
    fn other_fetch_failures_carry_the_reason() {
        let app: AppError = NetError::Status { code: 404 }.into();
        assert_eq!(app.msg.key, MsgKey::UpgradeFetchFailed);
        assert_eq!(app.msg.args, ["HTTP 404"]);
        assert_eq!(
            app.hint.as_ref().map(|hint| hint.key),
            Some(MsgKey::UpgradeFetchFailedHint)
        );

        let app: AppError = NetError::Parse {
            source: ReleaseError::MissingField {
                field: "tag_name".to_string(),
            },
        }
        .into();
        assert_eq!(app.msg.key, MsgKey::UpgradeFetchFailed);
        assert_eq!(app.msg.args, ["missing field `tag_name`"]);
    }
}
