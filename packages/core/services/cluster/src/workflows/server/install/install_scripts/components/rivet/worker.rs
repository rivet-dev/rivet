use chirp_workflow::prelude::*;

use super::{TUNNEL_API_EDGE_PORT, super::{fdb::FDB_VERSION}};

pub async fn install(
	config: &rivet_config::Config,
) -> GlobalResult<String> {
	let provision_config = &config.server()?.rivet.provision()?;

	Ok(include_str!("../../files/rivet_worker_install.sh")
		.replace(
			"__EDGE_SERVER_BINARY_URL__",
			provision_config.edge_server_binary_url.as_ref(),
		)
		.replace("__FDB_VERSION__", FDB_VERSION))
}

pub fn configure(
	config: &rivet_config::Config,
) -> GlobalResult<String> {
	let server_config = config.server()?;
	let provision_config = server_config.rivet.provision()?;

	let origin_api =
		util::url::to_string_without_slash(&config.server()?.rivet.api_public.public_origin());

	
	use rivet_config::config::*;
	let edge_config = Root {
		server: Server {
			rivet: Rivet {
				edge: Edge {
					cluster_id: todo!("templated by hook"),
					datacenter_id: todo!("templated by hook"),
				},
				rest todo
			},
			foundationdb: todo!("service discovery"),
			cockroachdb: todo!("tunnel"),
			redis: todo!("tunnel"),
			clickhouse: todo!("tunnel"),
			s3: todo!("tunnel"),
			nats: todo!("tunnel"),
			jwt: todo!(),
		},
	};
	let edge_config_json = serde_json::to_value(&edge_config)?;

	todo!("replace cluster id and dc id in edge_config_json with ___CLUSTER_ID___ etc");
	
	Ok(include_str!("../../files/rivet_worker_configure.sh")
		.replace(
			"__RIVET_EDGE_CONFIG__",
			&serde_json::to_string(&edge_config)?,
		)
		.replace(
			"__TUNNEL_API_EDGE_API__",
			&format!("http://127.0.0.1:{TUNNEL_API_EDGE_PORT}"),
		)
	)
}
