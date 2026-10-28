//! 英文文案。

use super::MsgKey;

pub(super) fn text(key: MsgKey) -> &'static str {
    match key {
        MsgKey::ErrorPrefix => "error:",
        MsgKey::HintPrefix => "hint:",
        MsgKey::WarningPrefix => "warning:",
        MsgKey::HelpAbout => "Install and copy agent skills under .agents/skills",
        MsgKey::HelpUsage => "Usage:",
        MsgKey::HelpCommands => "Commands:",
        MsgKey::HelpOptions => "Options:",
        MsgKey::HelpArguments => "Arguments:",
        MsgKey::HelpMoreInfo => "For more information, try '--help'.",
        MsgKey::HelpFlagHelp => "Print help",
        MsgKey::VersionFlagHelp => "Print version",
        MsgKey::LangHelp => "Output language [en, zh]",

        MsgKey::UsageInvalidLanguage => "invalid language '{0}'",
        MsgKey::UsageInvalidLanguageHint => "use one of: en, zh",
        MsgKey::UsageInvalidScope => "invalid scope '{0}'",
        MsgKey::UsageInvalidScopeHint => "use one of: global, project",
        MsgKey::UsageMissingSubcommand => "a subcommand is required",
        MsgKey::UsageMultipleActions => "only one of -a/-l/-c may be used",
        MsgKey::UsageMissingActionValue => "'{0}' requires a value",
        MsgKey::UsageMultipleScopes => "-g and -p cannot be combined",
        MsgKey::UsageMissingScope => "'{0}' requires -g or -p",
        MsgKey::UsageCopyWithScope => "copy does not take -g or -p",
        MsgKey::UsageMixedForms => "cannot mix short flags with subcommands",
        MsgKey::UsageHintForms => "examples: lsm -a foo.md -g | lsm add foo.md global",
        MsgKey::UsageMissingArgument => "missing required argument '{0}'",
        MsgKey::UsageInvalidValue => "invalid value for '{0}'",
        MsgKey::UsageUnknownArgument => "unknown argument '{0}'",
        MsgKey::UsageUnexpectedArgument => "unexpected argument '{0}'",
        MsgKey::UsageInvalidSubcommand => "invalid subcommand '{0}'",
        MsgKey::UsageInvalidArguments => "{0}",
        MsgKey::UsageHelpHint => "run `lsm --help` for usage",

        MsgKey::AddAbout => "Install a skill into global or project",
        MsgKey::AddSourceArg => "Path to a skill directory or a .md file",
        MsgKey::AddScopeArg => "Where to install: global or project",
        MsgKey::AddForceFlag => "Overwrite an existing skill",
        MsgKey::AddInstalled => "installed {0} -> {1}",
        MsgKey::AddNestedTarget => "destination is inside source directory",
        MsgKey::AddNestedTargetHint => "choose a source directory outside {0}",
        MsgKey::AddGitWarning => "source contains .git directory, it will be copied",
        MsgKey::AddUnsupportedSource => "source must be a skill directory or a .md file: {0}",

        MsgKey::ListAbout => "List installed skills",
        MsgKey::ListScopeArg => "Where to look: global or project",
        MsgKey::ListJsonFlag => "Print machine-readable JSON",
        MsgKey::ListEmpty => "No skills found in {0}",
        MsgKey::ListHeaderName => "NAME",
        MsgKey::ListHeaderDescription => "DESCRIPTION",

        MsgKey::CopyAbout => "Copy a global skill into the current project",
        MsgKey::CopyNameArg => "Skill name to copy",
        MsgKey::CopyForceFlag => "Overwrite an existing skill",
        MsgKey::CopyCopied => "copied {0} -> {1}",
        MsgKey::CopyNotFound => "skill '{0}' not found in global",
        MsgKey::CopyNotFoundHint => "run `lsm list global` to see available skills",
        MsgKey::CopySourceInvalid => "global skill '{0}' is invalid: {1}",
        MsgKey::CopySourceInvalidHint => "fix the source skill or reinstall it",
        MsgKey::CopySamePath => "source and destination are the same path",
        MsgKey::CopySamePathHint => "run this command from a different directory",

        MsgKey::ConfigAbout => "Manage persistent settings",
        MsgKey::ConfigGetAbout => "Print a saved setting",
        MsgKey::ConfigSetAbout => "Save a setting",
        MsgKey::ConfigKeyLangHelp => "Output language",
        MsgKey::ConfigValueArg => "Language value: en or zh",
        MsgKey::ConfigSaved => "lang = {0} (saved to {1})",
        MsgKey::ConfigInvalidValue => "invalid language '{0}' (allowed: en, zh)",
        MsgKey::ConfigParseFailed => "cannot parse config file: {0}",
        MsgKey::ConfigWriteFailed => "failed to write config file: {0}",
        MsgKey::ConfigUnknownKey => "unknown config key '{0}'",
        MsgKey::ConfigFixHint => "fix or delete {0}",

        MsgKey::SkillAlreadyExists => "skill '{0}' already exists at {1}",
        MsgKey::SkillAlreadyExistsHint => "use --force to overwrite",
        MsgKey::SkillNameInvalid => "invalid skill name '{0}'",
        MsgKey::SkillNameInvalidHint => {
            "skill names use lowercase letters, digits and single hyphens"
        }
        MsgKey::SkillNameTooLong => "skill name is too long: {0} > 64 characters",
        MsgKey::SkillNameReserved => "skill name '{0}' is reserved on Windows",
        MsgKey::SkillDescriptionEmpty => "skill description must not be empty",
        MsgKey::SkillDescriptionTooLong => "skill description is too long: {0} > 1024 characters",
        MsgKey::SkillFrontmatterMissing => "{0}: missing frontmatter block",
        MsgKey::SkillFrontmatterMissingHint => {
            "add a '---' block with name and description at the top of SKILL.md"
        }
        MsgKey::SkillFrontmatterUtf8 => "{0}: SKILL.md is not valid UTF-8",
        MsgKey::SkillFrontmatterYaml => "{0}: invalid YAML frontmatter: {1}",
        MsgKey::SkillFieldMissing => "{0}: missing required field '{1}'",
        MsgKey::SkillDirMismatch => "directory name '{0}' does not match frontmatter name '{1}'",
        MsgKey::SkillDirMismatchHint => "rename the directory to '{0}'",

        MsgKey::FsIo => "cannot access {0}: {1}",
        MsgKey::FsSymlink => "symbolic links are not supported: {0}",
        MsgKey::FsSourceMissing => "source path does not exist: {0}",
        MsgKey::FsSourceType => "source has the wrong type: {0}",
        MsgKey::FsBackupLeftover => "failed to remove backup directory: {0}",

        MsgKey::PathHomeUnavailable => "cannot determine the home directory",
        MsgKey::PathHomeUnavailableHint => "set LSM_HOME",
        MsgKey::PathConfigDirUnavailable => "cannot determine the config directory",
        MsgKey::PathConfigDirUnavailableHint => "set LSM_CONFIG_DIR",
    }
}
