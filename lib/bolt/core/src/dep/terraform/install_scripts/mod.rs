use crate::dep::terraform::servers::Server;
use anyhow::Result;

pub mod components;

pub fn gen(server: &Server) -> Result<String> {
	let mut script = Vec::new();
	script.push(components::common());
	script.push(components::node_exporter());
	script.push(components::sysctl());

	if server.pool_id == "gg" {
		script.push(components::traefik());
	}

	if server.pool_id == "job" {
		script.push(components::docker());
		script.push(components::cni_plugins());
		script.push(components::nomad(server));
	}

	// if server.pool_id == "ats" {
	// 	// script.push(components::ats());
	// }

	let joined = script.join("\n\necho \"======\"\n\n");
	Ok(format!("#!/usr/bin/env bash\nset -eu\n\n{joined}"))
}
