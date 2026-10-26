use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use super::*;

const ARTIFACT: &str = "rivet-test-x86_64";
const BODY: &[u8] = b"#!/bin/sh\necho test binary\n";

#[derive(Clone)]
enum Route {
	Ok(Vec<u8>),
	/// Advertises the full length but closes the connection after `sent` bytes.
	Truncated { body: Vec<u8>, sent: usize },
	/// Returns `status` for the first `failures` requests, then `body`.
	FailThenOk {
		status: u16,
		failures: usize,
		seen: Arc<AtomicUsize>,
		body: Vec<u8>,
	},
}

struct TestServer {
	base_url: String,
	artifact_requests: Arc<AtomicUsize>,
}

async fn spawn_server(routes: HashMap<String, Route>) -> TestServer {
	let listener = TcpListener::bind("127.0.0.1:0")
		.await
		.expect("bind download server");
	let addr = listener.local_addr().expect("download server address");
	let routes = Arc::new(routes);
	let artifact_requests = Arc::new(AtomicUsize::new(0));
	let counter = artifact_requests.clone();
	tokio::spawn(async move {
		loop {
			let Ok((mut socket, _)) = listener.accept().await else {
				return;
			};
			let routes = routes.clone();
			let counter = counter.clone();
			tokio::spawn(async move {
				let mut buffer = [0_u8; 4096];
				let n = socket.read(&mut buffer).await.unwrap_or(0);
				let request = String::from_utf8_lossy(&buffer[..n]);
				let path = request.split_whitespace().nth(1).unwrap_or("/").to_owned();
				if path.ends_with(ARTIFACT) {
					counter.fetch_add(1, Ordering::SeqCst);
				}
				let route = match routes.get(&path) {
					Some(Route::FailThenOk {
						status,
						failures,
						seen,
						body,
					}) => {
						if seen.fetch_add(1, Ordering::SeqCst) < *failures {
							let response = format!(
								"HTTP/1.1 {status} Error\r\nretry-after: 0\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"
							);
							let _ = socket.write_all(response.as_bytes()).await;
							let _ = socket.shutdown().await;
							return;
						}
						Some(Route::Ok(body.clone()))
					}
					other => other.cloned(),
				};
				match route {
					Some(Route::FailThenOk { .. }) => {}
					Some(Route::Ok(body)) => {
						let header = format!(
							"HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
							body.len()
						);
						let _ = socket.write_all(header.as_bytes()).await;
						// Write in small chunks so concurrent clients overlap.
						for chunk in body.chunks(8) {
							let _ = socket.write_all(chunk).await;
							tokio::task::yield_now().await;
						}
					}
					Some(Route::Truncated { body, sent }) => {
						let header = format!(
							"HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
							body.len()
						);
						let _ = socket.write_all(header.as_bytes()).await;
						let _ = socket.write_all(&body[..sent]).await;
					}
					None => {
						let _ = socket
							.write_all(
								b"HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
							)
							.await;
					}
				}
				let _ = socket.shutdown().await;
			});
		}
	});
	TestServer {
		base_url: format!("http://{addr}/rivet/1.2.3/test"),
		artifact_requests,
	}
}

fn manifest_for(body: &[u8]) -> Vec<u8> {
	format!(
		"{}  other-artifact\n{}  {ARTIFACT}\n",
		sha256_hex(b"other"),
		sha256_hex(body)
	)
	.into_bytes()
}

fn routes(manifest: Vec<u8>, artifact: Route) -> HashMap<String, Route> {
	HashMap::from([
		("/rivet/1.2.3/test/SHA256SUMS".to_owned(), Route::Ok(manifest)),
		(format!("/rivet/1.2.3/test/{ARTIFACT}"), artifact),
	])
}

fn release(base_url: &str) -> ReleaseBinary {
	ReleaseBinary {
		display_name: "Rivet Test",
		cache_name: "test",
		version: "1.2.3".to_owned(),
		platform: "linux-x64".to_owned(),
		base_url: base_url.to_owned(),
		artifact: ARTIFACT.to_owned(),
		file_name: "rivet-test".to_owned(),
		override_env: "RIVET_TEST_BINARY",
		alternative: "skip the test.",
	}
}

fn error_code(error: &anyhow::Error) -> String {
	let error = rivet_error::RivetError::extract(error);
	format!("{}.{}", error.group(), error.code())
}

fn cache_dir_entries(cache: &Path) -> Vec<String> {
	let dir = release("http://unused").cache_path(cache);
	let Some(parent) = dir.parent() else {
		return Vec::new();
	};
	let Ok(entries) = std::fs::read_dir(parent) else {
		return Vec::new();
	};
	let mut names = entries
		.flatten()
		.map(|entry| entry.file_name().to_string_lossy().into_owned())
		.filter(|name| !name.ends_with(".lock") && !name.ends_with(".sha256"))
		.collect::<Vec<_>>();
	names.sort();
	names
}

#[test]
fn parses_sha256sums_manifest() {
	let manifest = format!(
		"{}  a\n{} *b\nnot-a-checksum c\n",
		sha256_hex(b"a"),
		sha256_hex(b"b")
	);
	assert_eq!(
		checksum_for_artifact(&manifest, "b"),
		Some(sha256_hex(b"b"))
	);
	assert_eq!(checksum_for_artifact(&manifest, "c"), None);
	assert_eq!(checksum_for_artifact(&manifest, "missing"), None);
}

#[tokio::test]
async fn downloads_verifies_and_caches() {
	let server = spawn_server(routes(manifest_for(BODY), Route::Ok(BODY.to_vec()))).await;
	let cache = tempfile::tempdir().expect("create cache dir");
	let release = release(&server.base_url);

	let path = ensure_release_binary(&release, cache.path(), true, "test")
		.await
		.expect("download binary");
	assert_eq!(path, release.cache_path(cache.path()));
	assert_eq!(std::fs::read(&path).expect("read binary"), BODY);
	#[cfg(unix)]
	{
		use std::os::unix::fs::PermissionsExt;
		let mode = std::fs::metadata(&path).expect("metadata").permissions().mode();
		assert_eq!(mode & 0o111, 0o111, "binary must be executable");
	}

	// A second resolution uses the cache, even with downloads disabled.
	let again = ensure_release_binary(&release, cache.path(), false, "test")
		.await
		.expect("reuse cached binary");
	assert_eq!(again, path);
	assert_eq!(server.artifact_requests.load(Ordering::SeqCst), 1);
	assert_eq!(cache_dir_entries(cache.path()), vec!["rivet-test".to_owned()]);
}

#[tokio::test]
async fn rejects_checksum_mismatch_without_caching() {
	let server = spawn_server(routes(
		manifest_for(b"different bytes"),
		Route::Ok(BODY.to_vec()),
	))
	.await;
	let cache = tempfile::tempdir().expect("create cache dir");
	let release = release(&server.base_url);

	let error = ensure_release_binary(&release, cache.path(), true, "test")
		.await
		.expect_err("checksum mismatch should fail");
	assert_eq!(error_code(&error), "binary_download.checksum_mismatch");
	assert!(error.to_string().contains("RIVET_TEST_BINARY"));
	assert!(!release.cache_path(cache.path()).exists());
	assert!(cache_dir_entries(cache.path()).is_empty());
}

#[tokio::test]
async fn rejects_artifact_missing_from_manifest() {
	let manifest = format!("{}  other-artifact\n", sha256_hex(b"other")).into_bytes();
	let server = spawn_server(routes(manifest, Route::Ok(BODY.to_vec()))).await;
	let cache = tempfile::tempdir().expect("create cache dir");

	let error = ensure_release_binary(&release(&server.base_url), cache.path(), true, "test")
		.await
		.expect_err("missing checksum should fail");
	assert_eq!(error_code(&error), "binary_download.checksum_missing");
	assert_eq!(server.artifact_requests.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn reports_http_404_with_url() {
	let server = spawn_server(HashMap::new()).await;
	let cache = tempfile::tempdir().expect("create cache dir");
	let release = release(&server.base_url);

	let error = ensure_release_binary(&release, cache.path(), true, "test")
		.await
		.expect_err("404 should fail");
	assert_eq!(error_code(&error), "binary_download.http_status");
	let message = error.to_string();
	assert!(message.contains("HTTP 404"), "{message}");
	assert!(message.contains(&release.manifest_url()), "{message}");
}

#[tokio::test]
async fn discards_partial_download() {
	let body = vec![7_u8; 4096];
	let server = spawn_server(routes(
		manifest_for(&body),
		Route::Truncated { body, sent: 1000 },
	))
	.await;
	let cache = tempfile::tempdir().expect("create cache dir");
	let release = release(&server.base_url);

	let error = ensure_release_binary(&release, cache.path(), true, "test")
		.await
		.expect_err("partial download should fail");
	assert_eq!(error_code(&error), "binary_download.incomplete");
	assert!(!release.cache_path(cache.path()).exists());
	assert!(cache_dir_entries(cache.path()).is_empty());
}

#[tokio::test]
async fn refuses_to_download_when_disabled() {
	let cache = tempfile::tempdir().expect("create cache dir");
	let error = ensure_release_binary(&release("http://127.0.0.1:1"), cache.path(), false, "test")
		.await
		.expect_err("disabled download should fail");
	assert_eq!(error_code(&error), "binary_download.disabled");
	let message = error.to_string();
	assert!(message.contains(DISABLE_DOWNLOAD_ENV), "{message}");
	assert!(message.contains("RIVET_TEST_BINARY"), "{message}");
}

#[tokio::test]
async fn reports_unreachable_server() {
	let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
	let addr = listener.local_addr().expect("addr");
	drop(listener);
	let cache = tempfile::tempdir().expect("create cache dir");

	let error = ensure_release_binary(
		&release(&format!("http://{addr}/rivet/1.2.3/test")),
		cache.path(),
		true,
		"test",
	)
	.await
	.expect_err("unreachable server should fail");
	assert_eq!(error_code(&error), "binary_download.request_failed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_first_run_downloads_once() {
	let body = vec![42_u8; 256 * 1024];
	let server = spawn_server(routes(manifest_for(&body), Route::Ok(body.clone()))).await;
	let cache = tempfile::tempdir().expect("create cache dir");
	let release = release(&server.base_url);

	let mut tasks = Vec::new();
	for _ in 0..8 {
		let release = release.clone();
		let cache = cache.path().to_path_buf();
		tasks.push(tokio::spawn(async move {
			ensure_release_binary(&release, &cache, true, "test").await
		}));
	}
	for task in tasks {
		let path = task.await.expect("join").expect("download binary");
		assert_eq!(std::fs::read(&path).expect("read binary"), body);
	}
	assert_eq!(server.artifact_requests.load(Ordering::SeqCst), 1);
	assert_eq!(cache_dir_entries(cache.path()), vec!["rivet-test".to_owned()]);
}

#[test]
fn removes_stale_partial_downloads() {
	let dir = tempfile::tempdir().expect("create dir");
	std::fs::write(dir.path().join(".rivet-test.123.download"), b"partial").expect("write");
	std::fs::write(dir.path().join("rivet-test"), b"keep").expect("write");
	remove_stale_downloads(dir.path(), "rivet-test");
	assert!(!dir.path().join(".rivet-test.123.download").exists());
	assert!(dir.path().join("rivet-test").exists());
}

#[tokio::test]
async fn redownloads_truncated_cached_binary() {
	let server = spawn_server(routes(manifest_for(BODY), Route::Ok(BODY.to_vec()))).await;
	let cache = tempfile::tempdir().expect("create cache dir");
	let release = release(&server.base_url);
	let path = ensure_release_binary(&release, cache.path(), true, "test")
		.await
		.expect("download binary");

	std::fs::write(&path, b"").expect("truncate cached binary");
	assert!(!release.is_cached(cache.path()));
	ensure_release_binary(&release, cache.path(), false, "test")
		.await
		.expect_err("corrupt cache must not be used without a download");

	let again = ensure_release_binary(&release, cache.path(), true, "test")
		.await
		.expect("redownload binary");
	assert_eq!(std::fs::read(&again).expect("read binary"), BODY);
	assert!(release.is_cached(cache.path()));
	assert_eq!(server.artifact_requests.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn ignores_cached_binary_without_checksum_record() {
	let cache = tempfile::tempdir().expect("create cache dir");
	let release = release("http://127.0.0.1:1");
	let path = release.cache_path(cache.path());
	std::fs::create_dir_all(path.parent().expect("parent")).expect("create dir");
	std::fs::write(&path, BODY).expect("write binary");

	assert!(!release.is_cached(cache.path()));
	let error = ensure_release_binary(&release, cache.path(), false, "test")
		.await
		.expect_err("unverified binary should not be used");
	assert_eq!(error_code(&error), "binary_download.disabled");
}

#[tokio::test]
async fn rejects_versions_that_escape_the_cache() {
	let cache = tempfile::tempdir().expect("create cache dir");
	for version in ["..", "../1.2.3", "1.2.3/../../x", "", ".hidden", "1.2.3\\x"] {
		let mut release = release("http://127.0.0.1:1");
		release.version = version.to_owned();
		let error = ensure_release_binary(&release, cache.path(), true, "test")
			.await
			.expect_err("invalid version should fail");
		assert_eq!(error_code(&error), "binary_download.invalid_version", "{version}");
	}
	validate_version("Rivet Test", "2.3.30-rc.1+build_5").expect("valid version");
}

#[tokio::test]
async fn lock_wait_times_out_with_lock_path() {
	let dir = tempfile::tempdir().expect("create dir");
	let lock_path = dir.path().join("rivet-test.lock");
	let release = release("http://127.0.0.1:1");
	let held = acquire_lock(&release, &lock_path, Duration::from_secs(1))
		.await
		.expect("acquire first lock");

	let error = acquire_lock(&release, &lock_path, Duration::from_millis(100))
		.await
		.err()
		.expect("contended lock should time out");
	assert_eq!(error_code(&error), "binary_download.lock_timeout");
	assert!(error.to_string().contains(&lock_path.display().to_string()));

	drop(held);
	acquire_lock(&release, &lock_path, Duration::from_secs(5))
		.await
		.expect("lock is free after release");
}

#[tokio::test]
async fn cancelled_download_removes_temp_file() {
	let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
	let addr = listener.local_addr().expect("addr");
	let body = vec![1_u8; 64];
	let manifest = manifest_for(&body);
	tokio::spawn(async move {
		loop {
			let Ok((mut socket, _)) = listener.accept().await else {
				return;
			};
			let manifest = manifest.clone();
			tokio::spawn(async move {
				let mut buffer = [0_u8; 4096];
				let n = socket.read(&mut buffer).await.unwrap_or(0);
				let request = String::from_utf8_lossy(&buffer[..n]).into_owned();
				if request.contains("SHA256SUMS") {
					let header = format!(
						"HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
						manifest.len()
					);
					let _ = socket.write_all(header.as_bytes()).await;
					let _ = socket.write_all(&manifest).await;
					return;
				}
				// Send part of the artifact, then stall so the download stays open.
				let _ = socket
					.write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 64\r\nconnection: close\r\n\r\n")
					.await;
				let _ = socket.write_all(&[1_u8; 16]).await;
				std::future::pending::<()>().await;
			});
		}
	});

	let cache = tempfile::tempdir().expect("create cache dir");
	let release = release(&format!("http://{addr}/rivet/1.2.3/test"));
	let cache_root = cache.path().to_path_buf();
	let task = {
		let release = release.clone();
		tokio::spawn(async move { ensure_release_binary(&release, &cache_root, true, "test").await })
	};
	let directory = release
		.cache_path(cache.path())
		.parent()
		.expect("parent")
		.to_path_buf();
	// The download runs in another task and signals nothing, so poll for its temp file.
	tokio::time::timeout(Duration::from_secs(10), async {
		while !std::fs::read_dir(&directory)
			.map(|entries| {
				entries
					.flatten()
					.any(|entry| entry.file_name().to_string_lossy().ends_with(".download"))
			})
			.unwrap_or(false)
		{
			tokio::time::sleep(Duration::from_millis(10)).await;
		}
	})
	.await
	.expect("download should create a temp file");
	task.abort();
	let _ = task.await;

	assert!(cache_dir_entries(cache.path()).is_empty());
}

fn fail_then_ok(status: u16, failures: usize, body: &[u8]) -> Route {
	Route::FailThenOk {
		status,
		failures,
		seen: Arc::new(AtomicUsize::new(0)),
		body: body.to_vec(),
	}
}

#[tokio::test]
async fn retries_server_errors_then_succeeds() {
	let server = spawn_server(routes(manifest_for(BODY), fail_then_ok(503, 2, BODY))).await;
	let cache = tempfile::tempdir().expect("create cache dir");
	let release = release(&server.base_url);

	let path = ensure_release_binary(&release, cache.path(), true, "test")
		.await
		.expect("download after retries");
	assert_eq!(std::fs::read(&path).expect("read binary"), BODY);
	assert_eq!(server.artifact_requests.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn gives_up_after_three_retries() {
	let server = spawn_server(routes(manifest_for(BODY), fail_then_ok(502, 100, BODY))).await;
	let cache = tempfile::tempdir().expect("create cache dir");
	let release = release(&server.base_url);

	let error = ensure_release_binary(&release, cache.path(), true, "test")
		.await
		.expect_err("persistent server error should fail");
	assert_eq!(error_code(&error), "binary_download.http_status");
	assert_eq!(server.artifact_requests.load(Ordering::SeqCst), 4);
	assert!(cache_dir_entries(cache.path()).is_empty());
}

#[tokio::test]
async fn does_not_retry_missing_release_or_bad_checksum() {
	let server = spawn_server(routes(manifest_for(BODY), fail_then_ok(404, 100, BODY))).await;
	let cache = tempfile::tempdir().expect("create cache dir");
	let error = ensure_release_binary(&release(&server.base_url), cache.path(), true, "test")
		.await
		.expect_err("404 should fail");
	assert_eq!(error_code(&error), "binary_download.http_status");
	assert_eq!(server.artifact_requests.load(Ordering::SeqCst), 1);

	let server = spawn_server(routes(
		manifest_for(b"different bytes"),
		Route::Ok(BODY.to_vec()),
	))
	.await;
	let error = ensure_release_binary(&release(&server.base_url), cache.path(), true, "test")
		.await
		.expect_err("checksum mismatch should fail");
	assert_eq!(error_code(&error), "binary_download.checksum_mismatch");
	assert_eq!(server.artifact_requests.load(Ordering::SeqCst), 1);
}

#[test]
fn classifies_transient_errors() {
	let binary = release("http://127.0.0.1:1");
	let status = |status: u16| {
		check_status_code(&binary, "http://x", StatusCode::from_u16(status).expect("status"))
			.expect_err("error status")
	};
	assert!(is_retryable(&status(500)));
	assert!(is_retryable(&status(503)));
	assert!(is_retryable(&status(429)));
	assert!(!is_retryable(&status(404)));
	assert!(!is_retryable(&status(403)));
	assert!(is_retryable(&request_error(&binary, "http://x", "connection refused")));
	assert!(is_retryable(&incomplete_or_request_error(
		&binary,
		"http://x",
		1,
		Some(2),
		"reset".to_owned()
	)));
	assert!(!is_retryable(&io_error(&binary, "write", Path::new("/x"), "disk full")));
}

#[test]
fn detects_filesystems_without_locking() {
	assert!(lock_unsupported(&std::io::Error::from(std::io::ErrorKind::Unsupported)));
	#[cfg(unix)]
	{
		assert!(lock_unsupported(&std::io::Error::from_raw_os_error(libc::ENOLCK)));
		assert!(lock_unsupported(&std::io::Error::from_raw_os_error(libc::EOPNOTSUPP)));
		assert!(!lock_unsupported(&std::io::Error::from_raw_os_error(libc::EACCES)));
	}
}

#[test]
fn rejects_unpinned_versions() {
	for version in ["latest", "main", "stable"] {
		let error = validate_pinned_version("Rivet Test", version).expect_err("alias");
		assert_eq!(error_code(&error), "binary_download.unpinned_version", "{version}");
	}
	validate_pinned_version("Rivet Test", "2.3.30-rc.1").expect("pinned version");
}

#[tokio::test]
async fn retries_rate_limited_downloads_then_succeeds() {
	let server = spawn_server(routes(manifest_for(BODY), fail_then_ok(429, 2, BODY))).await;
	let cache = tempfile::tempdir().expect("create cache dir");

	let path = ensure_release_binary(&release(&server.base_url), cache.path(), true, "test")
		.await
		.expect("download after rate limiting");
	assert_eq!(std::fs::read(&path).expect("read binary"), BODY);
	assert_eq!(server.artifact_requests.load(Ordering::SeqCst), 3);
}

#[test]
fn honors_retry_after_up_to_the_cap() {
	let headers = |retry_after: &str| {
		let mut headers = reqwest::header::HeaderMap::new();
		headers.insert(
			reqwest::header::RETRY_AFTER,
			retry_after.parse().expect("header value"),
		);
		headers
	};
	let limited = StatusCode::TOO_MANY_REQUESTS;

	assert_eq!(retry_after(limited, &headers("0")), Some(Duration::ZERO));
	assert_eq!(retry_after(limited, &headers("3600")), Some(MAX_RETRY_AFTER));
	// The HTTP-date form falls back to exponential backoff.
	assert_eq!(retry_after(limited, &headers("Wed, 21 Oct 2026 07:28:00 GMT")), None);
	assert_eq!(retry_after(limited, &reqwest::header::HeaderMap::new()), None);
	assert_eq!(
		retry_after(StatusCode::SERVICE_UNAVAILABLE, &headers("5")),
		None,
		"only rate-limited responses honor Retry-After"
	);
}
