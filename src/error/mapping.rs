//! 唯一的「各层窄错误 → `AppError`」适配点，hint 也在这里配置。

use crate::domain::lang::LangError;
use crate::domain::meta::MetaError;
use crate::domain::name::NameError;
use crate::i18n::MsgKey;
use crate::infra::frontmatter::FrontmatterError;
use crate::infra::fsops::FsError;
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
    fn lang_errors_are_usage_errors_with_allowed_values() {
        let app: AppError = LangError::Unsupported("fr".to_string()).into();

        assert_eq!(app.kind, ErrorKind::Usage);
        assert_eq!(app.msg.key, MsgKey::UsageInvalidLanguage);
        assert_eq!(
            app.hint.as_ref().map(|hint| hint.key),
            Some(MsgKey::UsageInvalidLanguageHint)
        );
    }
}
