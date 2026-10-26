use super::*;

fn config(binary_path: Option<PathBuf>) -> ServicesProcessConfig {
	ServicesProcessConfig {
		binary_path,
		endpoint: "http://127.0.0.1:6420".to_owned(),
		token: Some("dev".to_owned()),
		namespace: "default".to_owned(),
		pool_name: SERVICES_POOL_NAME.to_owned(),
		engine_protocol_version: 7,
		auto_download: false,
	}
}

#[test]
fn parses_structured_version_output() {
	let output: ServicesVersionOutput = serde_json::from_str(
		r#"{"name":"rivet-services","version":"0.2.0-rc.1","rivetkitVersion":"2.3.11","protocolVersion":7}"#,
	)
	.expect("version output should parse");
	assert_eq!(output.name, "rivet-services");
	assert_eq!(output.version, "0.2.0-rc.1");
	assert_eq!(output.rivetkit_version, "2.3.11");
	assert_eq!(output.protocol_version, 7);
}

#[tokio::test]
async fn missing_binary_is_a_structured_error() {
	let error = ServicesProcessManager::start(config(Some(PathBuf::from(
		"/nonexistent/rivet-services",
	))))
	.await
	.expect_err("missing binary should fail");

	let error = rivet_error::RivetError::extract(&error);
	assert_eq!(error.group(), "services");
	assert_eq!(error.code(), "binary_not_found");
	assert!(
		error.message().contains("RIVET_SERVICES_BINARY"),
		"resolution should name the setting to fix: {}",
		error.message()
	);
}

#[test]
fn rejects_a_newer_protocol() {
	let error =
		validate_protocol_version(8, 7).expect_err("newer Services protocol should fail");
	let error = rivet_error::RivetError::extract(&error);
	assert_eq!(error.group(), "services");
	assert_eq!(error.code(), "protocol_mismatch");
}

#[test]
fn accepts_an_equal_or_older_protocol() {
	validate_protocol_version(7, 7).expect("equal protocol should be compatible");
	validate_protocol_version(6, 7).expect("older protocol should be compatible");
}

#[test]
fn services_release_uses_pinned_version_path() {
	let release = services_release_binary("0.2.3", "https://releases.rivet.dev/")
		.expect("Services release for host");
	assert_eq!(
		release.base_url,
		"https://releases.rivet.dev/services/0.2.3/services"
	);
	assert!(release.artifact.starts_with("rivet-services-"));
	assert_eq!(
		release.manifest_url(),
		"https://releases.rivet.dev/services/0.2.3/services/SHA256SUMS"
	);
}

#[tokio::test]
async fn explicit_binary_path_skips_download() {
	let path = PathBuf::from("/nonexistent/rivet-services");
	let resolved = resolve_services_binary(&config(Some(path.clone())))
		.await
		.expect("explicit path resolves");
	assert_eq!(resolved, path);
}

#[test]
fn services_release_rejects_path_traversal_version() {
	let error = services_release_binary("../0.2.3", "https://releases.rivet.dev")
		.expect_err("traversal version should fail");
	let error = rivet_error::RivetError::extract(&error);
	assert_eq!(error.group(), "binary_download");
	assert_eq!(error.code(), "invalid_version");
}

#[test]
fn skips_services_only_when_unsupported_and_not_requested() {
	assert!(should_skip_services(false, false, false));
	assert!(!should_skip_services(true, false, false), "explicit Services keeps the hard error");
	assert!(!should_skip_services(false, true, false), "supported platforms start Services");
	assert!(!should_skip_services(false, false, true), "a provided binary is used");
}

#[test]
fn services_release_rejects_unpinned_version() {
	let error = services_release_binary("latest", "https://releases.rivet.dev")
		.expect_err("alias should fail");
	let error = rivet_error::RivetError::extract(&error);
	assert_eq!(error.code(), "unpinned_version");
}

#[test]
fn binary_unavailable_names_the_setting_that_disabled_downloads() {
	// This error is only returned when RIVETKIT_ENGINE_AUTO_DOWNLOAD turned
	// downloads off. RIVET_DISABLE_BINARY_DOWNLOAD reports its own error.
	let error = ServicesProcessError::BinaryUnavailable {
		version: SERVICES_VERSION.to_owned(),
	}
	.build();
	let message = rivet_error::RivetError::extract(&error).message().to_owned();
	assert!(
		message.contains("RIVETKIT_ENGINE_AUTO_DOWNLOAD=1"),
		"resolution should name the setting to change: {message}"
	);
	assert!(message.contains(SERVICES_VERSION), "message should name the version: {message}");
	assert!(
		!message.contains("RIVET_DISABLE_BINARY_DOWNLOAD"),
		"this error is never caused by RIVET_DISABLE_BINARY_DOWNLOAD: {message}"
	);
}
