pub const OK_SERVER_PORT: usize = 9999;

pub fn install(initialize_immediately: bool) -> String {
	let mut script = include_str!("../files/ok_server.sh")
		.replace("__OK_SERVER_PORT__", &OK_SERVER_PORT.to_string());

	if initialize_immediately {
		// Run script immediately
		script.push_str("systemctl start --no-block ok_server.service");
	}

	script
}
