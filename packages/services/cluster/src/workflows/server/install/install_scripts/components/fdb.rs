// See docs-internal/infrastructure/fdb/AVX.md
pub const FDB_VERSION: &str = "7.1.61";

pub fn install(initialize_immediately: bool) -> String {
	let mut script = include_str!("../files/fdb_install.sh")
		.replace("__FDB_VERSION__", FDB_VERSION)
		.replace(
			"__PROMETHEUS_PROXY_SCRIPT__",
			include_str!("../files/fdp_prometheus_proxy.py"),
		);

	if initialize_immediately {
		// Run script immediately
		script.push_str("systemctl start --no-block fdb_prometheus_proxy.service");
	}

	script
}

pub fn configure() -> String {
	include_str!("../files/fdb_configure.sh").to_string()
}
