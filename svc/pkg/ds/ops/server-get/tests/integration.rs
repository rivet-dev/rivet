use std::collections::HashMap;

use chirp_worker::prelude::*;
use proto::backend::{self, pkg::dynamic_servers};

#[worker_test]
async fn server_get(ctx: TestCtx) {
	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();
	let game_id = game_res.game_id.unwrap();

	// Pick an existing cluster
	let cluster_id = op!([ctx] cluster_list {})
		.await
		.unwrap()
		.cluster_ids
		.first()
		.unwrap()
		.to_owned();

	let build_res: backend::pkg::faker::build::Response = op!([ctx] faker_build {
		game_id: Some(game_id),
		image: backend::faker::Image::DsEcho as i32,
	})
	.await
	.unwrap();

	let faker_region = op!([ctx] faker_region {}).await.unwrap();

	let env = vec![
		("some_envkey_test".to_string(), "2134523".to_string()),
		(
			"some_other_envkey_test".to_string(),
			"4325234356".to_string(),
		),
	]
	.into_iter()
	.collect();

	let ports = vec![(
		"testing2".to_string(),
		dynamic_servers::server_create::Port {
			internal_port: Some(28234),
			routing: Some(dynamic_servers::server_create::port::Routing::GameGuard(
				backend::ds::GameGuardRouting { protocol: 0 },
			)),
		},
	)]
	// Collect into hashmap
	.into_iter()
	.collect();

	let server = op!([ctx] ds_server_create {
		game_id: Some(game_id),
		cluster_id: Some(cluster_id),
		datacenter_id: faker_region.region_id,
		resources: Some(proto::backend::ds::ServerResources { cpu_millicores: 100, memory_mib: 200 }),
		kill_timeout_ms: 0,
		tags: HashMap::new(),
		args: Vec::new(),
		environment: env,
		image_id: Some(build_res.build_id.unwrap()),
		network_mode: 0,
		network_ports: ports,
	})
	.await
	.unwrap()
	.server
	.unwrap();

	let server_res = op!([ctx] ds_server_get {
		server_ids: vec![server.server_id.unwrap()],
	})
	.await
	.unwrap();
}
