//! 升级的网络出口：取 release 元数据。只有本模块允许访问网络。

use std::time::Duration;

use crate::domain::release::{Release, ReleaseError};
use crate::domain::version::Version;

/// 默认元数据端点（`upgrade.md` §3）。
pub const DEFAULT_API: &str =
    "https://api.github.com/repos/yearzen1/LocalSkillManager/releases/latest";

/// 元数据请求超时（Q36）。
const METADATA_TIMEOUT: Duration = Duration::from_secs(30);

/// 资产下载超时（Q36）；下载能力由升级切片接入。
pub const ASSET_TIMEOUT: Duration = Duration::from_secs(300);

/// 取元数据时的失败分类。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetError {
    /// HTTP 403/429：GitHub 限流。
    RateLimited,
    /// 其余非 2xx 状态码。
    Status { code: u16 },
    /// 连接、超时、TLS、非法 URL 等传输层失败。
    Network { detail: String },
    /// 响应不是可用的 release 元数据。
    Parse { source: ReleaseError },
}

/// 取 `releases/latest`，解析为 [`Release`]。
pub fn fetch_latest_release(endpoint: Option<&str>, current: Version) -> Result<Release, NetError> {
    let url = endpoint_url(endpoint)?;
    let agent = agent(METADATA_TIMEOUT);

    let response = agent
        .get(url)
        .header("User-Agent", &format!("lsm/{current}"))
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|err| NetError::Network {
            detail: err.to_string(),
        })?;

    let status = response.status().as_u16();
    match status {
        200..=299 => {}
        403 | 429 => return Err(NetError::RateLimited),
        code => return Err(NetError::Status { code }),
    }

    let mut response = response;
    let body = response
        .body_mut()
        .read_to_string()
        .map_err(|err| NetError::Network {
            detail: err.to_string(),
        })?;

    Release::parse_metadata(&body).map_err(|source| NetError::Parse { source })
}

/// 空串与 `None` 回落到默认端点；其余必须是绝对 `http(s)` URL。
fn endpoint_url(endpoint: Option<&str>) -> Result<&str, NetError> {
    match endpoint {
        None | Some("") => Ok(DEFAULT_API),
        Some(value) if value.starts_with("http://") || value.starts_with("https://") => Ok(value),
        Some(value) => Err(NetError::Network {
            detail: format!("LSM_UPGRADE_API is not an absolute http(s) URL: {value}"),
        }),
    }
}

/// 一个请求一个 Agent：超时按 Q36 固定，重定向用库默认的 10 次。
fn agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .http_status_as_error(false)
        .build()
        .new_agent()
}

#[cfg(test)]
pub(crate) mod stub {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread::{self, JoinHandle};

    /// 单次响应桩：接一个连接，读完请求头后回固定响应。
    pub(crate) struct Stub {
        endpoint: String,
        handle: JoinHandle<String>,
    }

    impl Stub {
        pub(crate) fn start(status: u16, body: impl Into<String>) -> Self {
            let body = body.into();
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind stub");
            let address = listener.local_addr().expect("stub address");

            let handle = thread::spawn(move || {
                let (mut stream, _) = listener.accept().expect("accept");
                let mut buffer = [0_u8; 4096];
                let read = stream.read(&mut buffer).expect("read request");
                let request = String::from_utf8_lossy(&buffer[..read]).into_owned();

                let response = format!(
                    "HTTP/1.1 {status} Stub\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).expect("write");
                stream.flush().expect("flush");
                request
            });

            Self {
                endpoint: format!("http://{address}/api/releases/latest"),
                handle,
            }
        }

        pub(crate) fn endpoint(&self) -> &str {
            &self.endpoint
        }

        /// 等桩线程结束，返回它收到的请求。
        pub(crate) fn request(self) -> String {
            self.handle.join().expect("stub thread")
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;

    use super::stub::Stub;
    use super::*;

    fn current() -> Version {
        Version::parse("0.3.0").expect("valid version")
    }

    const OK_BODY: &str = r#"{
        "tag_name": "v0.3.1",
        "assets": [
            { "name": "lsm-windows-x64.exe", "browser_download_url": "https://example.test/lsm" }
        ]
    }"#;

    #[test]
    fn parses_a_200_response_and_sends_the_documented_headers() {
        let stub = Stub::start(200, OK_BODY);

        let release = fetch_latest_release(Some(stub.endpoint()), current()).expect("should fetch");

        assert_eq!(release.tag().tag(), "v0.3.1");
        assert!(release.asset("lsm-windows-x64.exe").is_some());

        let request = stub.request().to_lowercase();
        assert!(
            request.starts_with("get /api/releases/latest http/1.1"),
            "{request}"
        );
        assert!(request.contains("user-agent: lsm/0.3.0"), "{request}");
        assert!(
            request.contains("accept: application/vnd.github+json"),
            "{request}"
        );
    }

    #[test]
    fn maps_rate_limit_statuses() {
        for status in [403, 429] {
            let stub = Stub::start(status, "");
            assert_eq!(
                fetch_latest_release(Some(stub.endpoint()), current()),
                Err(NetError::RateLimited),
                "{status}"
            );
            stub.request();
        }
    }

    #[test]
    fn maps_other_error_statuses() {
        for status in [404, 500] {
            let stub = Stub::start(status, "");
            assert_eq!(
                fetch_latest_release(Some(stub.endpoint()), current()),
                Err(NetError::Status { code: status }),
                "{status}"
            );
            stub.request();
        }
    }

    #[test]
    fn reports_bodies_that_are_not_metadata() {
        let stub = Stub::start(200, "not json");

        assert!(matches!(
            fetch_latest_release(Some(stub.endpoint()), current()),
            Err(NetError::Parse {
                source: ReleaseError::Json { .. }
            })
        ));
        stub.request();
    }

    #[test]
    fn reports_unreachable_endpoints_as_network_failures() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let address = listener.local_addr().expect("address");
        drop(listener);

        let endpoint = format!("http://{address}/api");
        assert!(matches!(
            fetch_latest_release(Some(&endpoint), current()),
            Err(NetError::Network { .. })
        ));
    }

    #[test]
    fn uses_the_default_endpoint_unless_overridden() {
        assert_eq!(endpoint_url(None), Ok(DEFAULT_API));
        assert_eq!(endpoint_url(Some("")), Ok(DEFAULT_API));
        assert_eq!(
            endpoint_url(Some("http://127.0.0.1:1/api")),
            Ok("http://127.0.0.1:1/api")
        );
        assert_eq!(
            endpoint_url(Some("https://mirror.test/release.json")),
            Ok("https://mirror.test/release.json")
        );
    }

    #[test]
    fn rejects_endpoints_that_are_not_absolute_http_urls() {
        for endpoint in ["ftp://mirror.test/r.json", "mirror.test/r.json", "/r.json"] {
            assert!(matches!(
                endpoint_url(Some(endpoint)),
                Err(NetError::Network { .. })
            ));
        }
    }
}
