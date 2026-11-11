//! `upgrade`：比较当前版本与最新 release。下载与替换在后续切片接入。

use std::cmp::Ordering;
use std::path::{Path, PathBuf};

use crate::domain::lang::Lang;
use crate::domain::release::{self, Asset, Release};
use crate::domain::version::Version;
use crate::error::{self, AppError, Msg};
use crate::i18n::{tr_args, MsgKey};
use crate::infra::{fsops, net, process};
use crate::output;
use crate::Ctx;

/// 版本比较的结论（`upgrade.md` §1）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Check {
    /// 已是最新。
    UpToDate { current: Version },
    /// release 比当前新。
    Available { current: Version, latest: Version },
    /// 当前版本更新（latest 更旧）。
    CurrentIsNewer { current: Version, latest: Version },
}

/// 编译期版本；`CARGO_PKG_VERSION` 由 `Cargo.toml` 保证合法。
pub fn current_version() -> Version {
    Version::parse(env!("CARGO_PKG_VERSION")).expect("CARGO_PKG_VERSION is a valid version")
}

/// `--check`：取最新 release 后比版本，不下载、不写盘。
pub fn check(ctx: &Ctx) -> Result<Check, AppError> {
    let current = current_version();
    let release = net::fetch_latest_release(ctx.env.upgrade_api.as_deref(), current)?;

    Ok(decide(current, release.tag()))
}

/// 纯比较：不联网、不排版。
pub fn decide(current: Version, latest: Version) -> Check {
    match latest.cmp(&current) {
        Ordering::Greater => Check::Available { current, latest },
        Ordering::Equal => Check::UpToDate { current },
        Ordering::Less => Check::CurrentIsNewer { current, latest },
    }
}

/// `upgrade [--check] [--force] [--verbose] [--progress]` 的命令入口。
pub fn run(
    ctx: &Ctx,
    check: bool,
    force: bool,
    verbose: bool,
    progress: bool,
) -> Result<(), AppError> {
    let mut reporter = Reporter::new(ctx.lang, verbose || progress, progress);
    let outcome = perform(ctx, check, force, &mut reporter);
    reporter.finish();

    match outcome? {
        Outcome::Report(verdict) => {
            output::emit_stdout(&format!("{}\n", render(verdict, ctx.lang)));
        }
        Outcome::Installed {
            current,
            latest,
            exe,
        } => {
            let key = if latest < current {
                MsgKey::UpgradeDowngraded
            } else {
                MsgKey::UpgradeUpgraded
            };
            output::emit_stdout(&format!(
                "{}\n",
                tr_args(
                    ctx.lang,
                    key,
                    &[
                        &current.to_string(),
                        &latest.tag(),
                        &exe.display().to_string()
                    ]
                )
            ));
        }
    }

    Ok(())
}

/// stdout 上要打印的那一行结果。
enum Outcome {
    /// 只报告版本关系，不安装。
    Report(Check),
    /// 二进制已替换。
    Installed {
        current: Version,
        latest: Version,
        exe: PathBuf,
    },
}

/// 中间步骤：联网、下载、校验、替换。过程输出只经 `reporter` 进 stderr。
fn perform(
    ctx: &Ctx,
    check: bool,
    force: bool,
    reporter: &mut Reporter,
) -> Result<Outcome, AppError> {
    let current = current_version();
    reporter.stage(MsgKey::UpgradeCheckingRelease, &[]);
    let release = net::fetch_latest_release(ctx.env.upgrade_api.as_deref(), current)?;
    let latest = release.tag();
    let verdict = decide(current, latest);

    let install = !check && (force || matches!(verdict, Check::Available { .. }));
    if !install {
        return Ok(Outcome::Report(verdict));
    }

    let exe = process::current_exe()?;
    fsops::clean_stale_backups(&exe);

    let asset = select_asset(&release)?;
    let temp = fsops::upgrade_temp_path(&exe)?;
    fsops::discard(&temp);

    if let Err(err) = download_with_report(asset, &temp, latest, reporter) {
        reporter.finish();
        return Err(err);
    }

    reporter.stage(MsgKey::UpgradeReplacing, &[&exe.display().to_string()]);
    let report = fsops::replace_self(&exe, &temp)
        .map_err(|err| error::replace_failed(&exe, &err.to_string()))?;

    if let Some(backup) = report.leftover_backup {
        output::warn(
            ctx.lang,
            &Msg::with_args(
                MsgKey::UpgradeBackupLeftover,
                [backup.display().to_string()],
            ),
        );
    }

    if is_cargo_install(&exe, ctx) {
        output::warn(ctx.lang, &Msg::new(MsgKey::UpgradeCargoInstallWarning));
    }

    Ok(Outcome::Installed {
        current,
        latest,
        exe,
    })
}

/// 过程反馈的开关与状态；默认全关时不产生任何中间输出（Q59）。
struct Reporter {
    lang: Lang,
    verbose: bool,
    progress: bool,
    /// `--verbose` 的下载行是否已经打印。
    started: bool,
    /// 已画出的整数百分比；`None` 表示还没有进度行。
    drawn: Option<u64>,
    /// 上一次绘制时的已收字节与行宽（用于未知总长节流与清尾）。
    last_bytes: u64,
    last_width: usize,
}

impl Reporter {
    fn new(lang: Lang, verbose: bool, progress: bool) -> Self {
        Self {
            lang,
            verbose,
            progress,
            started: false,
            drawn: None,
            last_bytes: 0,
            last_width: 0,
        }
    }

    fn silent() -> Self {
        Self::new(Lang::En, false, false)
    }

    /// 阶段提示：只有开了 `--verbose` 才写，一行一条、无前缀。
    fn stage(&self, key: MsgKey, args: &[&str]) {
        if !self.verbose {
            return;
        }
        output::status(self.lang, &Msg::with_args(key, args.iter().copied()));
    }

    /// 下载进度：`--progress` 重绘进度行，否则 `--verbose` 只打一行开始提示。
    fn tick(&mut self, asset: &Asset, tick: net::Tick) {
        if self.progress {
            let due = match tick.total {
                Some(total) => self.drawn != Some(output::percent(tick.received, total)),
                None => {
                    self.drawn.is_none()
                        || tick.received.saturating_sub(self.last_bytes) >= 256 * 1024
                }
            };
            if due {
                self.draw(asset, tick);
            }
            return;
        }

        if self.verbose && !self.started {
            self.started = true;
            match tick.total {
                Some(total) => self.stage(
                    MsgKey::UpgradeDownloading,
                    &[&asset.name, &output::bytes(total)],
                ),
                None => self.stage(MsgKey::UpgradeDownloadingUnknown, &[&asset.name]),
            }
        }
    }

    fn draw(&mut self, asset: &Asset, tick: net::Tick) {
        let text = progress_text(self.lang, &asset.name, tick.received, tick.total);

        // 用空格补到上一次的宽度，避免末尾残留旧字符。
        let width = text.chars().count();
        let pad = self.last_width.saturating_sub(width);
        output::progress(&format!("{text}{}", " ".repeat(pad)));

        self.drawn = tick
            .total
            .map(|total| output::percent(tick.received, total));
        self.last_bytes = tick.received;
        self.last_width = width;
    }

    /// 下载结束：给进度行补换行。
    fn download_finished(&mut self) {
        if self.progress {
            self.finish();
        }
    }

    /// 幂等收尾：有打开的进度行时补一个换行，让后续输出另起一行。
    fn finish(&mut self) {
        if self.drawn.is_some() {
            output::progress_end();
            self.drawn = None;
            self.last_bytes = 0;
            self.last_width = 0;
        }
    }
}

/// 进度行的纯文本：有总长时画条并报百分比，否则只报已下载字节。
fn progress_text(lang: Lang, asset: &str, received: u64, total: Option<u64>) -> String {
    match total {
        Some(total) => tr_args(
            lang,
            MsgKey::UpgradeDownloadProgress,
            &[
                asset,
                &output::bar(received, total),
                &output::percent(received, total).to_string(),
                &output::bytes(received),
                &output::bytes(total),
            ],
        ),
        None => tr_args(
            lang,
            MsgKey::UpgradeDownloadProgressUnknown,
            &[asset, &output::bytes(received)],
        ),
    }
}

/// 当前平台的资产：命名表里没有该组合、或 release 里没有这个名字，都按「缺资产」报。
fn select_asset(release: &Release) -> Result<&Asset, AppError> {
    let (os, arch) = (std::env::consts::OS, std::env::consts::ARCH);
    let platform = format!("{os}-{arch}");

    let name = match release::asset_name(os, arch) {
        Ok(name) => name,
        Err(_) => return Err(error::no_asset(&release.tag().tag(), &platform, &platform)),
    };

    release
        .asset(&name)
        .ok_or_else(|| error::no_asset(&release.tag().tag(), &platform, &name))
}

/// `$CARGO_HOME/bin` 下的副本：升级能成功，但 cargo 下次安装会把它覆盖回去。
fn is_cargo_install(exe: &Path, ctx: &Ctx) -> bool {
    ctx.env
        .cargo_bin_dir
        .as_ref()
        .is_some_and(|dir| exe.starts_with(dir))
}

/// 结论对应的单行文案。
pub fn render(check: Check, lang: Lang) -> String {
    match check {
        Check::UpToDate { current } => {
            tr_args(lang, MsgKey::UpgradeUpToDate, &[&current.to_string()])
        }
        Check::Available { current, latest } => tr_args(
            lang,
            MsgKey::UpgradeAvailable,
            &[&latest.tag(), &current.to_string()],
        ),
        Check::CurrentIsNewer { current, latest } => tr_args(
            lang,
            MsgKey::UpgradeCurrentNewer,
            &[&current.to_string(), &latest.tag()],
        ),
    }
}

/// 冒烟校验：`<新文件> --version` 的 stdout 必须含 `lsm <期望版本>`。
pub fn verify_binary(path: &Path, expected: Version) -> Result<(), AppError> {
    let stdout = process::version_output(path).map_err(|_| error::verification_failed())?;

    verify_output(&stdout, expected)
}

/// 纯文本校验，便于单测。
pub fn verify_output(stdout: &str, expected: Version) -> Result<(), AppError> {
    if stdout.contains(&format!("lsm {expected}")) {
        Ok(())
    } else {
        Err(error::verification_failed())
    }
}

/// 下载到 `dest` 并校验；任一步失败都删掉 `dest`。
pub fn download_and_verify(asset: &Asset, dest: &Path, expected: Version) -> Result<(), AppError> {
    download_with_report(asset, dest, expected, &mut Reporter::silent())
}

/// 带过程反馈的下载与校验。
fn download_with_report(
    asset: &Asset,
    dest: &Path,
    expected: Version,
    reporter: &mut Reporter,
) -> Result<(), AppError> {
    let result = net::download_asset(&asset.url, dest, &mut |tick| reporter.tick(asset, tick))
        .map_err(|err| error::download_failed(&asset.name, err))
        .and_then(|()| {
            reporter.download_finished();
            reporter.stage(MsgKey::UpgradeVerifying, &[]);
            verify_binary(dest, expected)
        });

    if result.is_err() {
        fsops::discard(dest);
    }

    result
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use tempfile::TempDir;

    use super::*;
    use crate::domain::lang::Lang;
    use crate::infra::env::Env;
    use crate::infra::net::stub::Stub;

    fn context(endpoint: &str, dir: &TempDir) -> Ctx {
        Ctx {
            env: Env {
                home: Some(dir.path().join("agents")),
                config_dir: Some(dir.path().join("config")),
                lang: None,
                upgrade_api: Some(endpoint.to_string()),
                cargo_bin_dir: None,
            },
            lang: Lang::En,
            cwd: dir.path().to_path_buf(),
            lang_explicit: false,
        }
    }

    fn release_json(tag: &str) -> String {
        format!(r#"{{ "tag_name": "{tag}", "assets": [] }}"#)
    }

    /// 递归列出目录内容；`--check` 前后必须完全一致。
    fn snapshot(root: &Path) -> Vec<PathBuf> {
        fn walk(dir: &Path, found: &mut Vec<PathBuf>) {
            let Ok(entries) = fs::read_dir(dir) else {
                return;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                found.push(path.clone());
                if path.is_dir() {
                    walk(&path, found);
                }
            }
        }

        let mut found = Vec::new();
        walk(root, &mut found);
        found.sort();
        found
    }

    #[test]
    fn reports_up_to_date_without_touching_the_disk() {
        let dir = TempDir::new().expect("temp dir");
        let before = snapshot(dir.path());
        let stub = Stub::start(200, release_json(env!("CARGO_PKG_VERSION")));
        let ctx = context(stub.endpoint(), &dir);

        let check = check(&ctx).expect("should check");

        assert_eq!(
            check,
            Check::UpToDate {
                current: current_version()
            }
        );
        assert_eq!(
            render(check, Lang::En),
            format!("lsm {} is up to date", current_version())
        );

        stub.request();
        assert_eq!(snapshot(dir.path()), before);
    }

    #[test]
    fn reports_a_newer_release_without_downloading() {
        let dir = TempDir::new().expect("temp dir");
        let before = snapshot(dir.path());
        let stub = Stub::start(200, release_json("v99.0.0"));
        let ctx = context(stub.endpoint(), &dir);

        let check = check(&ctx).expect("should check");

        assert_eq!(
            check,
            Check::Available {
                current: current_version(),
                latest: Version::parse("v99.0.0").unwrap()
            }
        );
        assert_eq!(
            render(check, Lang::En),
            format!(
                "new version available: v99.0.0 (current {})",
                current_version()
            )
        );

        stub.request();
        assert_eq!(snapshot(dir.path()), before);
    }

    #[test]
    fn reports_a_current_version_that_is_newer_than_the_release() {
        let check = decide(current_version(), Version::parse("v0.0.1").unwrap());

        assert_eq!(
            check,
            Check::CurrentIsNewer {
                current: current_version(),
                latest: Version::parse("v0.0.1").unwrap()
            }
        );
        assert_eq!(
            render(check, Lang::En),
            format!("lsm {} is newer than v0.0.1", current_version())
        );
    }

    #[test]
    fn rate_limits_become_runtime_errors() {
        let dir = TempDir::new().expect("temp dir");
        let stub = Stub::start(403, "");
        let ctx = context(stub.endpoint(), &dir);

        let err = check(&ctx).expect_err("should fail");

        assert_eq!(err.exit_code(), 1);
        assert_eq!(err.msg.key, MsgKey::UpgradeRateLimited);
        stub.request();
    }

    #[test]
    fn accepts_version_output_that_carries_the_expected_version() {
        let expected = current_version();

        assert!(verify_output(&format!("lsm {expected}\n"), expected).is_ok());
    }

    #[test]
    fn rejects_version_output_with_another_version() {
        let err = verify_output("lsm 0.0.1\n", Version::parse("v0.0.2").unwrap())
            .expect_err("should reject");

        assert_eq!(err.exit_code(), 1);
        assert_eq!(err.msg.key, MsgKey::UpgradeVerifyFailed);
    }

    #[test]
    fn rejects_empty_version_output() {
        let err = verify_output("", current_version()).expect_err("should reject");

        assert_eq!(err.msg.key, MsgKey::UpgradeVerifyFailed);
    }

    #[test]
    fn progress_text_shows_the_bar_percent_and_sizes() {
        assert_eq!(
            progress_text(Lang::En, "lsm-linux-x64", 5, Some(10)),
            "downloading lsm-linux-x64 [##########----------] 50%  5 B/10 B"
        );
    }

    #[test]
    fn progress_text_falls_back_to_received_bytes_without_a_total() {
        assert_eq!(
            progress_text(Lang::En, "lsm-linux-x64", 5, None),
            "downloading lsm-linux-x64 5 B"
        );
    }
}
