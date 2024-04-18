use chirp_worker::prelude::*;

use super::TUNNEL_API_INTERNAL_PORT;

pub fn create_hook(initialize_immediately: bool) -> GlobalResult<String> {
	let mut script = include_str!("../files/rivet_create_hook.sh").to_string();

	if initialize_immediately {
		script.push_str("systemctl start rivet_hook\n");
	}

	Ok(script)
}

pub fn fetch_info(server_token: &str) -> GlobalResult<String> {
	Ok(include_str!("../files/rivet_fetch_info.sh")
		.replace("__SERVER_TOKEN__", server_token)
		.replace(
			"__TUNNEL_API_INTERNAL_PORT__",
			&TUNNEL_API_INTERNAL_PORT.to_string(),
		))
}

pub fn fetch_tls(
	initialize_immediately: bool,
	server_token: &str,
	traefik_instance_name: &str,
) -> GlobalResult<String> {
	let mut script = include_str!("../files/rivet_fetch_tls.sh")
		.replace("__NAME__", traefik_instance_name)
		.replace("__SERVER_TOKEN__", server_token)
		.replace(
			"__TUNNEL_API_INTERNAL_PORT__",
			&TUNNEL_API_INTERNAL_PORT.to_string(),
		);

	if initialize_immediately {
		script.push_str("systemctl start rivet_fetch_tls.timer\n");
	}

	Ok(script)
}
