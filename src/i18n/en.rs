//! 英文文案。

use super::MsgKey;

pub(super) fn text(key: MsgKey) -> &'static str {
    match key {
        MsgKey::ErrorPrefix => "error:",
        MsgKey::HintPrefix => "hint:",
        MsgKey::WarningPrefix => "warning:",
        MsgKey::HelpAbout => "Install, pull, and push agent skills under .agents/skills",
        MsgKey::HelpUsage => "Usage:",
        MsgKey::HelpCommands => "Commands:",
        MsgKey::HelpOptions => "Options:",
        MsgKey::HelpArguments => "Arguments:",
        MsgKey::HelpPossibleValues => "[possible values: ",
        MsgKey::HelpMoreInfo => "For more information, try '--help'.",
        MsgKey::HelpFlagHelp => "Print help",
        MsgKey::VersionFlagHelp => "Print version",
        MsgKey::LangHelp => "Output language [en, zh]",
        MsgKey::NameAmbiguous => "'{0}' matches more than one skill in {1}: {2}",
        MsgKey::NameAmbiguousHint => "use a longer prefix, or run `lsm list {0}`",
        MsgKey::NameSuggestion => "did you mean: {0}?",

        MsgKey::UsageInvalidLanguage => "invalid language '{0}'",
        MsgKey::UsageInvalidLanguageHint => "use one of: en, zh",
        MsgKey::UsageInvalidScope => "invalid scope '{0}'",
        MsgKey::UsageInvalidScopeHint => "use one of: global, project",
        MsgKey::UsageMissingSubcommand => "a subcommand is required",
        MsgKey::UsageMultipleActions => "only one of -a/-l/-c may be used",
        MsgKey::UsageMissingActionValue => "'{0}' requires a value",
        MsgKey::UsageMultipleScopes => "-g and -p cannot be combined",
        MsgKey::UsageRemoveMissingName => "remove needs at least one skill name",
        MsgKey::UsageRemoveMissingNameHint => {
            "to remove a skill named 'global' or 'project', write `lsm remove <name> <scope>`"
        }
        MsgKey::UsagePullTargetMissing => "{0} needs a skill name, --group, or --all",
        MsgKey::UsagePullTargetConflict => {
            "{0} takes a skill name, --group, or --all, but not more than one"
        }
        MsgKey::UsageEmptyQuery => "search needs a non-empty query",
        MsgKey::UsageMixedForms => "cannot mix short flags with subcommands",
        MsgKey::UsageHintForms => "examples: lsm -a foo.md -g | lsm add foo.md global",
        MsgKey::UsageMissingArgument => "missing required argument '{0}'",
        MsgKey::UsageGroupItemMissingHint => {
            "Windows PowerShell splats a leading @, so write group references as '@group'"
        }
        MsgKey::UsageInvalidValue => "invalid value for '{0}'",
        MsgKey::UsageUnknownArgument => "unknown argument '{0}'",
        MsgKey::UsageUnexpectedArgument => "unexpected argument '{0}'",
        MsgKey::UsageInvalidSubcommand => "invalid subcommand '{0}'",
        MsgKey::UsageInvalidArguments => "{0}",
        MsgKey::UsageHelpHint => "run `lsm --help` for usage",

        MsgKey::AddAbout => "Install a skill into global or project",
        MsgKey::AddSourceArg => "Path to a skill directory or a .md file",
        MsgKey::AddScopeArg => "Where to install: global or project (default: project)",
        MsgKey::AddForceFlag => "Overwrite an existing skill",
        MsgKey::AddInstalled => "installed {0} -> {1}",
        MsgKey::AddNestedTarget => "destination is inside source directory",
        MsgKey::AddNestedTargetHint => "choose a source directory outside {0}",
        MsgKey::AddGitWarning => "source contains .git directory, it will be copied",
        MsgKey::AddUnsupportedSource => "source must be a skill directory or a .md file: {0}",

        MsgKey::ListAbout => "List installed skills",
        MsgKey::ListScopeArg => "Where to look: global or project (default: project)",
        MsgKey::ListJsonFlag => "Print machine-readable JSON",
        MsgKey::ListEmpty => "No skills found in {0}",
        MsgKey::ListHeaderName => "NAME",
        MsgKey::ListHeaderDescription => "DESCRIPTION",

        MsgKey::SearchAbout => "Search installed skills by name or description",
        MsgKey::SearchQueryArg => "Text to match against name and description",
        MsgKey::SearchScopeArg => "Where to search: global or project (default: project)",
        MsgKey::SearchJsonFlag => "Print machine-readable JSON",
        MsgKey::SearchNameFlag => "Match skill names only",
        MsgKey::SearchDescriptionFlag => "Match descriptions only",
        MsgKey::SearchNoMatches => "No skills matched '{0}' in {1}",

        MsgKey::PullAbout => "Pull global skills into the current project",
        MsgKey::PullNameArg => "Skill names to pull",
        MsgKey::PullAllFlag => "Pull every global skill",
        MsgKey::PullGroupFlag => "Pull every skill in a group (repeatable)",
        MsgKey::PullAllEmpty => "no global skills to pull",
        MsgKey::PullAllSkipped => "skipped {0}: already exists in project",
        MsgKey::PullAllSummary => "pulled {0}, skipped {1}, failed {2}",
        MsgKey::PullAllFailed => "{0} skill(s) failed to pull",
        MsgKey::PullAllFailedHint => "fix the skills above, then run `lsm pull --all` again",
        MsgKey::PullBatchFailed => "{0} skill(s) failed to pull",
        MsgKey::PullBatchFailedHint => "fix the skills above, then run `lsm pull` again",
        MsgKey::PullForceFlag => "Overwrite an existing skill",
        MsgKey::PullCopied => "pulled {0} -> {1}",
        MsgKey::PullNotFound => "skill '{0}' not found in global",
        MsgKey::PullNotFoundHint => "run `lsm search {0} global` to see similar skills",
        MsgKey::PullSourceInvalid => "global skill '{0}' is invalid: {1}",
        MsgKey::PullSourceInvalidHint => "fix the source skill or reinstall it",
        MsgKey::PullSamePath => "source and destination are the same path",
        MsgKey::PullSamePathHint => "run this command from a different directory",

        MsgKey::PushAbout => "Push project skills into the global directory",
        MsgKey::PushNameArg => "Skill names to push",
        MsgKey::PushAllFlag => "Push every project skill",
        MsgKey::PushForceFlag => "Overwrite an existing global skill",
        MsgKey::PushAllEmpty => "no project skills to push",
        MsgKey::PushAllSkipped => "skipped {0}: already exists in global",
        MsgKey::PushAllSummary => "pushed {0}, skipped {1}, failed {2}",
        MsgKey::PushAllFailed => "{0} skill(s) failed to push",
        MsgKey::PushAllFailedHint => "fix the skills above, then run `lsm push --all` again",
        MsgKey::PushPushed => "pushed {0} -> {1}",
        MsgKey::PushNotFound => "skill '{0}' not found in project",
        MsgKey::PushNotFoundHint => "run `lsm list project` to see available skills",
        MsgKey::PushSourceInvalid => "project skill '{0}' is invalid: {1}",
        MsgKey::PushSourceInvalidHint => "fix the source skill or reinstall it",
        MsgKey::PushSamePath => "source and destination are the same path",
        MsgKey::PushSamePathHint => "run this command from a different directory",

        MsgKey::RemoveAbout => "Remove a skill from global or project",
        MsgKey::RemoveNameArg => {
            "Skill names to remove, optionally followed by global or project (default: project)"
        }
        MsgKey::RemoveForceFlag => "Remove even if the directory is not a valid skill",
        MsgKey::RemoveRemoved => "removed {0} -> {1}",
        MsgKey::RemoveNotFound => "skill '{0}' not found in {1}",
        MsgKey::RemoveNotFoundHint => "run `lsm search {0} {1}` to see similar skills",
        MsgKey::RemoveNotSkill => "'{0}' is not a valid skill: {1}",
        MsgKey::RemoveNotSkillHint => "pass --force to remove it anyway",
        MsgKey::RemoveLeftover => "temporary directory could not be removed: {0}",
        MsgKey::RemoveBatchSummary => "removed {0}, failed {1}",
        MsgKey::RemoveBatchFailed => "{0} skill(s) failed to remove",
        MsgKey::RemoveBatchFailedHint => "fix the skills above, then run `lsm remove` again",

        MsgKey::ConfigAbout => "Manage persistent settings",
        MsgKey::ConfigGetAbout => "Print a saved setting",
        MsgKey::ConfigSetAbout => "Save a setting",
        MsgKey::ConfigKeyLangHelp => "Configuration key: lang",
        MsgKey::ConfigValueArg => "Language value: en or zh",
        MsgKey::ConfigSaved => "lang = {0} (saved to {1})",
        MsgKey::ConfigInvalidValue => "invalid language '{0}' (allowed: en, zh)",
        MsgKey::ConfigParseFailed => "cannot parse config file: {0}",
        MsgKey::ConfigWriteFailed => "failed to write config file: {0}",
        MsgKey::ConfigUnknownKey => "unknown config key '{0}'",
        MsgKey::ConfigFixHint => "fix or delete {0}",

        MsgKey::GroupAbout => "Manage skill groups",
        MsgKey::GroupListAbout => "List skill groups",
        MsgKey::GroupSetAbout => "Create or replace a skill group",
        MsgKey::GroupAddAbout => "Append skills to a group",
        MsgKey::GroupRemoveAbout => "Remove skills from a group",
        MsgKey::GroupDeleteAbout => "Delete skill groups",
        MsgKey::GroupNameArg => "Group name",
        MsgKey::GroupSetForceFlag => "Overwrite an existing group",
        MsgKey::GroupDeleteForceFlag => "Delete even if other groups reference it",
        MsgKey::GroupSkillArg => "Skills or @group references",
        MsgKey::GroupJsonFlag => "Print machine-readable JSON",
        MsgKey::GroupResolvedFlag => "Expand @group references",
        MsgKey::GroupInvalid => "invalid group name '{0}'",
        MsgKey::GroupInvalidHint => "group names use lowercase letters, digits and single hyphens",
        MsgKey::GroupTooLong => "group name is too long: {0} > 64 characters",
        MsgKey::GroupReserved => "group name '{0}' is reserved on Windows",
        MsgKey::GroupAlreadyExists => "group '{0}' already exists",
        MsgKey::GroupNotFound => "group '{0}' not found",
        MsgKey::GroupNotFoundHint => "run `lsm group list` to see available groups",
        MsgKey::GroupLowercaseHint => "did you mean '{0}'? group names must be lowercase",
        MsgKey::GroupMemberMissing => "skill '{0}' is not installed in global",
        MsgKey::GroupMemberMissingHint => "run `lsm list global` to see installed skills",
        MsgKey::GroupMembersFailed => {
            "{0} skill(s) are unavailable in global; the group was not written"
        }
        MsgKey::GroupMembersFailedHint => "fix the skills above, then run the command again",
        MsgKey::GroupAmbiguous => "group '{0}' matches more than one group: {1}",
        MsgKey::GroupAmbiguousHint => "use a longer prefix, or run `lsm group list`",
        MsgKey::GroupItemNotInGroup => "'{0}' is not in group '{1}'",
        MsgKey::GroupRemoveFailed => {
            "{0} item(s) are not in group '{1}'; the group was not changed"
        }
        MsgKey::GroupRemoveFailedHint => "fix the names above, then run the command again",
        MsgKey::GroupSetFailed => "{0} item(s) could not be resolved; the group was not changed",
        MsgKey::GroupSetFailedHint => "fix the names above, then run the command again",
        MsgKey::GroupListFailed => "{0} group(s) could not be expanded",
        MsgKey::GroupListFailedHint => {
            "fix the @group references above, then run the command again"
        }
        MsgKey::GroupMemberRemoved => "removed '{0}' from group '{1}'",
        MsgKey::GroupRefMissing => "group '{0}' references undefined group '{1}'",
        MsgKey::GroupRefMissingHint => "run `lsm group list` to see defined groups",
        MsgKey::GroupCycle => "group reference cycle: {0}",
        MsgKey::GroupCycleHint => "remove one of the references in groups.toml",
        MsgKey::GroupSelfReference => "group '{0}' cannot include itself",
        MsgKey::GroupDeleteReferenced => "group '{0}' is referenced by: {1}",
        MsgKey::GroupDeleteReferencedHint => "use --force to delete it and remove the references",
        MsgKey::GroupRefStripped => "removed reference '@{0}' from group '{1}'",
        MsgKey::GroupEmpty => "group '{0}' is empty",
        MsgKey::GroupEmptyHint => "add members with `lsm group add {0} <skill>...`",
        MsgKey::GroupNoGroups => "No groups found.",
        MsgKey::GroupLine => "{0}: {1}",
        MsgKey::GroupEmptyMembers => "(empty)",
        MsgKey::GroupSaved => "group '{0}' saved ({1} skills) -> {2}",
        MsgKey::GroupRemoved => "removed group '{0}'",
        MsgKey::GroupBatchSummary => "removed {0}, failed {1}",
        MsgKey::GroupBatchFailed => "{0} group(s) failed to remove",
        MsgKey::GroupBatchFailedHint => "fix the groups above, then run `lsm group delete` again",
        MsgKey::GroupParseFailed => "cannot parse groups file: {0}",
        MsgKey::GroupWriteFailed => "failed to write groups file: {0}",
        MsgKey::GroupFileInvalid => "invalid groups file {0}: {1}",
        MsgKey::GroupLegacyLeftover => "could not remove [groups] from {0}; it is now ignored",

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
        MsgKey::SkillFrontmatterYamlHint => {
            "quote the value or use a >- block scalar when it contains \": \""
        }
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

        MsgKey::UpgradeInvalidVersion => "invalid release version '{0}'",
        MsgKey::UpgradeInvalidVersionHint => "expected a version like v0.3.0",
        MsgKey::UpgradeAbout => "Update lsm itself",
        MsgKey::UpgradeCheckFlag => "Only check for a newer release",
        MsgKey::UpgradeForceFlag => "Reinstall the same version or downgrade",
        MsgKey::UpgradeVerboseFlag => "Print each upgrade step to stderr",
        MsgKey::UpgradeProgressFlag => {
            "Draw a download progress bar on stderr (also prints each step)"
        }
        MsgKey::UpgradeUpToDate => "lsm {0} is up to date",
        MsgKey::UpgradeAvailable => "new version available: {0} (current {1})",
        MsgKey::UpgradeCurrentNewer => "lsm {0} is newer than {1}",
        MsgKey::UpgradeUpgraded => "upgraded lsm {0} -> {1} ({2})",
        MsgKey::UpgradeDowngraded => "downgraded lsm {0} -> {1} ({2})",
        MsgKey::UpgradeFetchFailed => "cannot fetch latest release: {0}",
        MsgKey::UpgradeFetchFailedHint => {
            "set LSM_UPGRADE_API to a mirror or download the binary manually from the releases page"
        }
        MsgKey::UpgradeRateLimited => "github api rate limit reached",
        MsgKey::UpgradeRateLimitedHint => {
            "wait a few minutes and retry, or download the binary manually from the releases page"
        }
        MsgKey::UpgradeDownloadFailed => "cannot download {0}: {1}",
        MsgKey::UpgradeDownloadFailedHint => "download the binary manually from the releases page",
        MsgKey::UpgradeVerifyFailed => "downloaded binary failed verification",
        MsgKey::UpgradeVerifyFailedHint => "check the release assets and try again later",
        MsgKey::UpgradeNoAsset => "release {0} has no asset for {1}",
        MsgKey::UpgradeNoAssetHint => "expected {0}",
        MsgKey::UpgradeReplaceFailed => "cannot replace {0}: {1}",
        MsgKey::UpgradeReplaceFailedHint => {
            "run from a writable directory or download the binary manually from the releases page"
        }
        MsgKey::UpgradeBackupLeftover => {
            "old binary left at {0}, it will be cleaned on next upgrade"
        }
        MsgKey::UpgradeCargoInstallWarning => {
            "lsm is installed by cargo, consider cargo install --force local-skill-manager"
        }
        MsgKey::UpgradeCheckingRelease => "checking for the latest release...",
        MsgKey::UpgradeDownloading => "downloading {0} ({1})...",
        MsgKey::UpgradeDownloadingUnknown => "downloading {0}...",
        MsgKey::UpgradeDownloadProgress => "downloading {0} {1} {2}%  {3}/{4}",
        MsgKey::UpgradeDownloadProgressUnknown => "downloading {0} {1}",
        MsgKey::UpgradeVerifying => "verifying the downloaded binary...",
        MsgKey::UpgradeReplacing => "replacing {0}...",
    }
}
