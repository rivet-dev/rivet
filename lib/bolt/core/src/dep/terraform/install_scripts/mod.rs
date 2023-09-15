use crate::{context::ProjectContext, dep::terraform::servers::Server};
use anyhow::Result;
use maplit::hashmap;

use crate::dep::terraform;

pub mod components;

pub async fn gen(ctx: &ProjectContext, server: &Server) -> Result<String> {
	let mut script = Vec::new();
	script.push(components::common());
	script.push(components::node_exporter());
	script.push(components::sysctl());

	if server.pool_id == "gg" {
		// TODO: Only do this if TLS plan applied
		let tls = terraform::output::read_tls(ctx).await;

		script.push(components::traefik());
		script.push(components::traefik_instance(components::TraefikInstance {
			name: "game-guard".into(),
			static_config: "TODO".into(),
			dynamic_config: "TODO".into(),
			tls_certs: hashmap! {
				"letsencrypt_rivet_job".into() => (*tls.tls_cert_letsencrypt_rivet_job).clone(),
			},
		}));
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
