use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn version_prints_package_version_and_exits_zero() {
    Command::cargo_bin("lsm")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn help_mentions_binary_name_and_exits_zero() {
    Command::cargo_bin("lsm")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("lsm"));
}
