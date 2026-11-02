//! 下载与冒烟校验的集成测试：用真实的 `lsm` 二进制当载荷。

use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::thread::{self, JoinHandle};

use lsm::commands::upgrade::{download_and_verify, verify_binary};
use lsm::domain::release::{Asset, Release};
use lsm::domain::version::Version;
use tempfile::TempDir;

fn current() -> Version {
    Version::parse(env!("CARGO_PKG_VERSION")).expect("valid crate version")
}

fn built_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_lsm"))
}

/// 单次响应桩：回一段固定字节，充当资产下载。
fn serve(body: Vec<u8>) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind stub");
    let address = listener.local_addr().expect("stub address");

    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut buffer = [0_u8; 4096];
        let _ = stream.read(&mut buffer).expect("read request");

        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(header.as_bytes()).expect("write header");
        stream.write_all(&body).expect("write body");
        stream.flush().expect("flush");
    });

    (format!("http://{address}/asset"), handle)
}

fn asset_from(url: &str) -> Asset {
    let json = format!(
        r#"{{ "tag_name": "v{}", "assets": [ {{ "name": "lsm-test", "browser_download_url": "{url}" }} ] }}"#,
        current()
    );
    let release = Release::parse_metadata(&json).expect("metadata");
    release.asset("lsm-test").expect("asset").clone()
}

#[test]
fn verifies_the_binary_under_test() {
    verify_binary(&built_binary(), current()).expect("the real binary prints its version");
}

#[test]
fn rejects_a_version_that_does_not_match() {
    let err = verify_binary(&built_binary(), Version::parse("v99.0.0").unwrap())
        .expect_err("mismatched version must fail");
    assert_eq!(err.exit_code(), 1);
}

#[test]
fn verifies_a_downloaded_copy_of_the_binary() {
    let dir = TempDir::new().expect("temp dir");
    let dest = dir.path().join("payload.exe");
    let bytes = fs::read(built_binary()).expect("read built binary");
    let (url, handle) = serve(bytes.clone());

    download_and_verify(&asset_from(&url), &dest, current()).expect("should verify");

    assert_eq!(fs::read(&dest).expect("payload"), bytes);
    handle.join().expect("stub thread");
}

#[test]
fn removes_the_download_when_the_version_does_not_match() {
    let dir = TempDir::new().expect("temp dir");
    let dest = dir.path().join("payload.exe");
    let bytes = fs::read(built_binary()).expect("read built binary");
    let (url, handle) = serve(bytes);

    let err = download_and_verify(&asset_from(&url), &dest, Version::parse("v99.0.0").unwrap())
        .expect_err("mismatched version must fail");

    assert_eq!(err.exit_code(), 1);
    assert!(!dest.exists(), "failed verification must clean up");
    handle.join().expect("stub thread");
}

#[test]
fn removes_the_download_when_the_payload_cannot_run() {
    let dir = TempDir::new().expect("temp dir");
    let dest = dir.path().join("payload.exe");
    let (url, handle) = serve(b"not a binary".to_vec());

    let err = download_and_verify(&asset_from(&url), &dest, current())
        .expect_err("garbage payload must fail");

    assert_eq!(err.exit_code(), 1);
    assert!(!dest.exists(), "failed verification must clean up");
    handle.join().expect("stub thread");
}
