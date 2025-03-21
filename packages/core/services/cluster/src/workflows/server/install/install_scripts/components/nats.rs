use chirp_workflow::prelude::*;

use super::rivet::TUNNEL_API_EDGE_PORT;

const NATS_VERSION: &str = "2.10.22";

pub fn install(config: &rivet_config::Config) -> GlobalResult<String> {
	let nats = &config.server()?.nats;

	let mut script = include_str!("../files/nats.sh").replace("__VERSION__", NATS_VERSION);

	if let (Some(username), Some(password)) = (&nats.username, &nats.password) {
		script = script.replace("__USERNAME__", username);
		script = script.replace("__PASSWORD__", password.read());
	}

	Ok(script)
}

pub fn fetch_routes(config: &rivet_config::Config, server_token: &str) -> GlobalResult<String> {
	let nats = &config.server()?.nats;

	let mut script = include_str!("../files/rivet_fetch_nats_routes.sh")
		.replace("__SERVER_TOKEN__", server_token)
		.replace(
			"__TUNNEL_API_EDGE_API__",
			&format!("http://127.0.0.1:{TUNNEL_API_EDGE_PORT}"),
		);

	if let (Some(username), Some(password)) = (&nats.username, &nats.password) {
		script = script.replace("__USERNAME__", username);
		script = script.replace("__PASSWORD__", password.read());
	}

	Ok(script)
}
