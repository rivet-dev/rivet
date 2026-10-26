use std::collections::HashMap;

use super::*;

fn test_config(releases_endpoint: String, auto_download: bool) -> EngineResolverConfig {
	EngineResolverConfig {
		endpoint: "http://127.0.0.1:1".to_owned(),
		explicit_binary_path: None,
		bind_host: None,
		bind_port: None,
		public_url: None,
		auto_download,
		version: "1.2.3-test".to_owned(),
		preview_ref: None,
		releases_endpoint,
	}
}

fn stamp_with_bind(bind_host: &str) -> EngineRuntimeStamp {
	EngineRuntimeStamp {
		pid: 1,
		endpoint: "http://127.0.0.1:6420".to_owned(),
		bind_host: bind_host.to_owned(),
		public_url: None,
	}
}

fn write_cached_engine(cache_root: &Path) -> PathBuf {
	let release = engine_release_binary(&test_config("http://127.0.0.1:1".to_owned(), false))
		.expect("engine release for host");
	let cached = release.cache_path(cache_root);
	std::fs::create_dir_all(cached.parent().expect("cached parent")).expect("create cache dir");
	std::fs::write(&cached, b"cached").expect("write cached binary");
	download::write_checksum_record(
		&download::checksum_record_path(&cached),
		&download::sha256_hex(b"cached"),
		6,
	)
	.expect("write checksum record");
	cached
}

#[test]
fn binds_loopback_only_detects_loopback_addresses() {
	assert!(stamp_with_bind("127.0.0.1").binds_loopback_only());
	assert!(stamp_with_bind("localhost").binds_loopback_only());
	assert!(stamp_with_bind("::1").binds_loopback_only());
}

#[test]
fn binds_loopback_only_false_for_reachable_addresses() {
	assert!(!stamp_with_bind("0.0.0.0").binds_loopback_only());
	assert!(!stamp_with_bind("::").binds_loopback_only());
	assert!(!stamp_with_bind("192.168.1.5").binds_loopback_only());
}

#[test]
fn resolve_bind_host_prefers_override_then_endpoint() {
	let mut config = test_config(String::new(), false);
	config.endpoint = "http://127.0.0.1:6420".to_owned();
	assert_eq!(resolve_bind_host(&config).unwrap(), "127.0.0.1");
	config.bind_host = Some("0.0.0.0".to_owned());
	assert_eq!(resolve_bind_host(&config).unwrap(), "0.0.0.0");
}

#[test]
fn engine_env_uses_development_defaults() {
	let env = engine_env(&test_config(String::new(), false)).expect("build engine env");
	let env = env.into_iter().collect::<HashMap<_, _>>();

	assert_eq!(env["RIVET__PEGBOARD__RETRY_RESET_DURATION"], "100");
	assert_eq!(env["RIVET__PEGBOARD__BASE_RETRY_TIMEOUT"], "100");
	assert_eq!(env["RIVET__PEGBOARD__RESCHEDULE_BACKOFF_MAX_EXPONENT"], "1");
	assert_eq!(env["RIVET__PEGBOARD__RUNNER_ELIGIBLE_THRESHOLD"], "5000");
	assert_eq!(env["RIVET__PEGBOARD__RUNNER_LOST_THRESHOLD"], "7000");
	assert_eq!(env["RIVET__PEGBOARD__ENVOY_ELIGIBLE_THRESHOLD"], "5000");
	assert_eq!(env["RIVET__PEGBOARD__ENVOY_LOST_THRESHOLD"], "7000");
	assert_eq!(env["RIVET__PEGBOARD__MIN_METADATA_POLL_INTERVAL"], "1000");
	assert_eq!(env["RIVET__AUTH__ADMIN_TOKEN"], "default");
	assert_eq!(env["RIVET__FEATURES__GUARD_GATEWAY_V3__MODE"], "on");
	assert_eq!(env["RIVET__FEATURES__GUARD_GATEWAY_V3__PERCENTAGE"], "100");
	assert_eq!(env["RIVET__RUNTIME__WORKER_SHUTDOWN_DURATION"], "1");
	assert_eq!(env["RIVET__RUNTIME__GUARD_SHUTDOWN_DURATION"], "1");
	assert_eq!(env["RIVET__RUNTIME__FORCE_SHUTDOWN_DURATION"], "2");
}

#[cfg(unix)]
#[test]
fn pid_is_alive_true_for_self_false_for_missing() {
	assert!(pid_is_alive(std::process::id()));
	// PID 0 targets the process group on unix, so use a very high, unused PID.
	assert!(!pid_is_alive(0x7FFF_FFFF));
}

#[test]
fn engine_release_binary_uses_versioned_release_path() {
	let release = engine_release_binary(&test_config(
		"https://releases.rivet.dev/".to_owned(),
		true,
	))
	.expect("engine release for host");
	assert_eq!(
		release.base_url,
		"https://releases.rivet.dev/rivet/1.2.3-test/engine"
	);
	assert!(release.artifact.starts_with("rivet-engine-"));
	assert_eq!(
		release.manifest_url(),
		"https://releases.rivet.dev/rivet/1.2.3-test/engine/SHA256SUMS"
	);
}

#[tokio::test]
async fn resolver_prefers_existing_engine_before_filesystem_paths() {
	let temp = tempfile::tempdir().expect("create temp dir");
	let local = temp
		.path()
		.join("target")
		.join("debug")
		.join(exe_name("rivet-engine"));
	std::fs::create_dir_all(local.parent().expect("local parent")).expect("create local dir");
	std::fs::write(&local, b"local").expect("write local binary");

	let resolved = resolve_engine_binary_after_probe(
		&test_config("http://127.0.0.1:1".to_owned(), false),
		true,
		&[temp.path().to_path_buf()],
		&temp.path().join("cache"),
	)
	.await
	.expect("resolve engine");

	assert_eq!(resolved, ResolvedEngine::Existing);
}

#[tokio::test]
async fn resolver_prefers_local_binary_before_cached_binary() {
	let temp = tempfile::tempdir().expect("create temp dir");
	let local = temp
		.path()
		.join("target")
		.join("debug")
		.join(exe_name("rivet-engine"));
	std::fs::create_dir_all(local.parent().expect("local parent")).expect("create local dir");
	std::fs::write(&local, b"local").expect("write local binary");
	let cache_root = temp.path().join("cache");
	write_cached_engine(&cache_root);

	let resolved = resolve_engine_binary_after_probe(
		&test_config("http://127.0.0.1:1".to_owned(), false),
		false,
		&[temp.path().to_path_buf()],
		&cache_root,
	)
	.await
	.expect("resolve engine");

	assert_eq!(resolved, ResolvedEngine::Binary(local));
}

#[tokio::test]
async fn resolver_reuses_cached_binary_without_download() {
	let temp = tempfile::tempdir().expect("create temp dir");
	let cache_root = temp.path().join("cache");
	let cached = write_cached_engine(&cache_root);

	let resolved = resolve_engine_binary_after_probe(
		&test_config("http://127.0.0.1:1".to_owned(), false),
		false,
		&[],
		&cache_root,
	)
	.await
	.expect("resolve engine");

	assert_eq!(resolved, ResolvedEngine::Binary(cached));
}

#[tokio::test]
async fn resolver_ignores_cached_binary_without_checksum_record() {
	let temp = tempfile::tempdir().expect("create temp dir");
	let cache_root = temp.path().join("cache");
	let cached = write_cached_engine(&cache_root);
	std::fs::remove_file(download::checksum_record_path(&cached)).expect("remove record");

	let error = resolve_engine_binary_after_probe(
		&test_config("http://127.0.0.1:1".to_owned(), false),
		false,
		&[],
		&cache_root,
	)
	.await
	.expect_err("unverified cache entry should not be used");
	assert!(error.to_string().contains("No usable engine binary was found"));
}

#[test]
fn engine_release_rejects_path_traversal_version() {
	let mut config = test_config("http://127.0.0.1:1".to_owned(), true);
	config.version = "../../etc".to_owned();
	let error = engine_release_binary(&config).expect_err("traversal version should fail");
	let error = rivet_error::RivetError::extract(&error);
	assert_eq!(error.group(), "binary_download");
	assert_eq!(error.code(), "invalid_version");
}

#[tokio::test]
async fn resolver_reports_actionable_error_without_binary_or_download() {
	let temp = tempfile::tempdir().expect("create temp dir");

	let error = resolve_engine_binary_after_probe(
		&test_config("http://127.0.0.1:1".to_owned(), false),
		false,
		&[],
		&temp.path().join("cache"),
	)
	.await
	.expect_err("missing binary should fail");
	let message = error.to_string();

	assert!(message.contains("No usable engine binary was found"));
	assert!(message.contains("Build `rivet-engine`"));
	assert!(message.contains("RIVET_ENGINE_BINARY_PATH"));
}

#[test]
fn engine_release_rejects_unpinned_versions() {
	for version in ["latest", "main", "next", "nightly"] {
		let config = test_config("http://127.0.0.1:1".to_owned(), true)
			.with_engine_version(Some(version.to_owned()));
		let error = engine_release_binary(&config).expect_err("alias should fail");
		let error = rivet_error::RivetError::extract(&error);
		assert_eq!(error.code(), "unpinned_version", "{version}");
		assert!(error.message().contains("`latest`"), "{}", error.message());
	}
}

#[test]
fn preview_builds_download_their_commit_engine() {
	let mut config = test_config("https://releases.rivet.dev/".to_owned(), true);
	config.preview_ref = Some("abc1234".to_owned());
	let release = engine_release_binary(&config).expect("preview release");
	assert_eq!(release.base_url, "https://releases.rivet.dev/rivet/abc1234/engine");
	assert_eq!(release.version, "abc1234");

	// An explicit version overrides the preview commit.
	let release = engine_release_binary(&config.with_engine_version(Some("2.3.30".to_owned())))
		.expect("pinned release");
	assert_eq!(release.base_url, "https://releases.rivet.dev/rivet/2.3.30/engine");
}

#[test]
fn missing_explicit_engine_binary_explains_how_to_fix_it() {
	let error = verify_binary_path(Path::new("/nonexistent/rivet-engine"))
		.expect_err("missing binary should fail");
	let error = rivet_error::RivetError::extract(&error);
	assert_eq!(error.group(), "engine");
	assert_eq!(error.code(), "binary_not_found");
	assert!(
		error.message().contains("RIVET_ENGINE_BINARY_PATH"),
		"resolution should name the setting to fix: {}",
		error.message()
	);
}

#[test]
fn engine_version_prefers_explicit_then_run_env_then_legacy_env() {
	let env = |pairs: &'static [(&'static str, &'static str)]| {
		move |key: &str| {
			pairs
				.iter()
				.find(|(name, _)| *name == key)
				.map(|(_, value)| (*value).to_owned())
		}
	};
	let both = &[
		("RIVET_RUN_ENGINE_VERSION", "2.3.29"),
		("RIVETKIT_ENGINE_VERSION", "2.3.28"),
	];

	assert_eq!(
		engine_version_override(Some("2.3.30".to_owned()), env(both)).as_deref(),
		Some("2.3.30")
	);
	assert_eq!(engine_version_override(None, env(both)).as_deref(), Some("2.3.29"));
	assert_eq!(
		engine_version_override(None, env(&[("RIVETKIT_ENGINE_VERSION", "2.3.28")])).as_deref(),
		Some("2.3.28")
	);
	assert_eq!(
		engine_version_override(Some(" ".to_owned()), env(&[("RIVET_RUN_ENGINE_VERSION", "")]))
			.as_deref(),
		None,
		"empty values fall through to the build default"
	);
}

#[test]
fn auto_download_disable_env_beats_explicit_config() {
	let env = |pairs: &'static [(&'static str, &'static str)]| {
		move |key: &str| {
			pairs
				.iter()
				.find(|(name, _)| *name == key)
				.map(|(_, value)| (*value).to_owned())
		}
	};
	assert!(download::resolve_auto_download(None, true, env(&[])));
	assert!(!download::resolve_auto_download(Some(false), true, env(&[])));
	assert!(!download::resolve_auto_download(
		None,
		true,
		env(&[("RIVETKIT_ENGINE_AUTO_DOWNLOAD", "0")])
	));
	assert!(download::resolve_auto_download(
		Some(true),
		true,
		env(&[("RIVETKIT_ENGINE_AUTO_DOWNLOAD", "0")])
	));
	assert!(!download::resolve_auto_download(
		Some(true),
		true,
		env(&[("RIVET_DISABLE_BINARY_DOWNLOAD", "1")])
	));
}
