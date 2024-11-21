use chirp_workflow::prelude::*;

pub fn install(initialize_immediately: bool) -> String {
	let mut script = include_str!("../files/fdb_install.sh").replace(
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
