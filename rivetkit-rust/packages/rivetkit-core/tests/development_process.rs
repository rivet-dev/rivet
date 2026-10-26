use std::collections::HashMap;

use super::*;

fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
	let vars = pairs
		.iter()
		.map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
		.collect::<HashMap<_, _>>();
	move |key| vars.get(key).cloned()
}

fn config() -> ServeConfig {
	ServeConfig {
		endpoint: "http://127.0.0.1:6420".to_owned(),
		..ServeConfig::default()
	}
}

#[test]
fn defaults_download_and_run_services_without_requiring_them() {
	let settings = LocalProcessSettings::resolve_with(&config(), env(&[])).expect("resolve");
	assert!(settings.auto_download, "Rust and TypeScript hosts download by default");
	assert!(settings.start_services);
	assert!(!settings.services_required);
	assert_eq!(settings.engine_version, None);
	assert_eq!(settings.engine_host, None);
	assert_eq!(settings.engine_port, None);
	assert_eq!(settings.engine_binary_path, None);
	assert_eq!(settings.services_binary_path, None);
}

#[test]
fn environment_fills_unset_config() {
	let settings = LocalProcessSettings::resolve_with(
		&config(),
		env(&[
			("RIVET_RUN_ENGINE_VERSION", "2.3.29"),
			("RIVET_RUN_ENGINE_HOST", "0.0.0.0"),
			("RIVET_RUN_ENGINE_PORT", "7000"),
			("RIVET_ENGINE_BINARY_PATH", "/opt/rivet-engine"),
			("RIVET_SERVICES_BINARY", "/opt/rivet-services"),
			("RIVETKIT_ENGINE_AUTO_DOWNLOAD", "0"),
		]),
	)
	.expect("resolve");
	assert_eq!(settings.engine_version.as_deref(), Some("2.3.29"));
	assert_eq!(settings.engine_host.as_deref(), Some("0.0.0.0"));
	assert_eq!(settings.engine_port, Some(7000));
	assert_eq!(
		settings.engine_binary_path,
		Some(PathBuf::from("/opt/rivet-engine"))
	);
	assert_eq!(
		settings.services_binary_path,
		Some(PathBuf::from("/opt/rivet-services"))
	);
	assert!(!settings.auto_download);
}

#[test]
fn explicit_config_beats_environment() {
	let config = ServeConfig {
		engine_version: Some("2.3.30".to_owned()),
		engine_host: Some("127.0.0.1".to_owned()),
		engine_port: Some(6500),
		start_services: Some(false),
		engine_auto_download: Some(true),
		..config()
	};
	let settings = LocalProcessSettings::resolve_with(
		&config,
		env(&[
			("RIVET_RUN_ENGINE_VERSION", "2.3.29"),
			("RIVETKIT_ENGINE_VERSION", "2.3.28"),
			("RIVET_RUN_ENGINE_HOST", "0.0.0.0"),
			("RIVET_RUN_ENGINE_PORT", "7000"),
			("RIVET_RUN_SERVICES", "1"),
			("RIVETKIT_ENGINE_AUTO_DOWNLOAD", "0"),
		]),
	)
	.expect("resolve");
	assert_eq!(settings.engine_version.as_deref(), Some("2.3.30"));
	assert_eq!(settings.engine_host.as_deref(), Some("127.0.0.1"));
	assert_eq!(settings.engine_port, Some(6500));
	assert!(!settings.start_services);
	assert!(settings.auto_download);
}

#[test]
fn disable_download_env_beats_explicit_config() {
	let config = ServeConfig {
		engine_auto_download: Some(true),
		..config()
	};
	let settings = LocalProcessSettings::resolve_with(
		&config,
		env(&[("RIVET_DISABLE_BINARY_DOWNLOAD", "1")]),
	)
	.expect("resolve");
	assert!(!settings.auto_download);
}

#[test]
fn run_services_env_is_an_explicit_choice() {
	let on = LocalProcessSettings::resolve_with(&config(), env(&[("RIVET_RUN_SERVICES", "1")]))
		.expect("resolve");
	assert!(on.start_services);
	assert!(on.services_required, "an explicit request fails on unsupported platforms");

	let off = LocalProcessSettings::resolve_with(&config(), env(&[("RIVET_RUN_SERVICES", "0")]))
		.expect("resolve");
	assert!(!off.start_services);

	let requested = ServeConfig {
		start_services: Some(true),
		..config()
	};
	let requested = LocalProcessSettings::resolve_with(&requested, env(&[])).expect("resolve");
	assert!(requested.start_services);
	assert!(requested.services_required);
}

#[test]
fn empty_environment_values_are_ignored() {
	let settings = LocalProcessSettings::resolve_with(
		&config(),
		env(&[
			("RIVET_RUN_SERVICES", ""),
			("RIVET_RUN_ENGINE_PORT", " "),
			("RIVET_RUN_ENGINE_VERSION", ""),
			("RIVET_ENGINE_BINARY_PATH", ""),
		]),
	)
	.expect("resolve");
	assert!(settings.start_services);
	assert!(!settings.services_required);
	assert_eq!(settings.engine_port, None);
	assert_eq!(settings.engine_version, None);
	assert_eq!(settings.engine_binary_path, None);
}

#[test]
fn rejects_an_invalid_engine_port() {
	let error =
		LocalProcessSettings::resolve_with(&config(), env(&[("RIVET_RUN_ENGINE_PORT", "abc")]))
			.expect_err("invalid port should fail");
	assert!(
		error.to_string().contains("RIVET_RUN_ENGINE_PORT"),
		"error should name the variable: {error}"
	);
}
