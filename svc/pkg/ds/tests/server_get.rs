use std::collections::HashMap;

use chirp_workflow::prelude::*;
use ds::types;
use rivet_operation::prelude::proto::backend;
use serde_json::json;

#[workflow_test]
async fn server_get(ctx: TestCtx) {
	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();
	let env_id = game_res.prod_env_id.unwrap();

	// Pick an existing cluster
	let cluster_id = ctx
		.op(cluster::ops::list::Input {})
		.await
		.unwrap()
		.cluster_ids
		.first()
		.unwrap()
		.to_owned();

	let build_res: backend::pkg::faker::build::Response = op!([ctx] faker_build {
		env_id: Some(env_id),
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
		("HTTP_PORT".to_string(), "28234".to_string()),
	]
	.into_iter()
	.collect();

	let ports = vec![(
		"testing2".to_string(),
		ds::workflows::server::Port {
			internal_port: Some(28234),
			routing: types::Routing::GameGuard {
				protocol: types::GameGuardProtocol::Http,
			},
		},
	)]
	.into_iter()
	.collect();

	let server_id = Uuid::new_v4();

	let mut sub = ctx
		.subscribe::<ds::workflows::server::CreateComplete>(&json!({
			"server_id": server_id,
		}))
		.await
		.unwrap();

	ctx.workflow(ds::workflows::server::Input {
		server_id,
		env_id: *env_id,
		datacenter_id: faker_region.region_id.unwrap().as_uuid(),
		cluster_id,
		client: ds::types::GameClient::Nomad,
		resources: ds::types::ServerResources {
			cpu_millicores: 100,
			memory_mib: 200,
		},
		kill_timeout_ms: 0,
		tags: HashMap::new(),
		root_user_enabled: false,
		args: Vec::new(),
		environment: env,
		image_id: build_res.build_id.unwrap().as_uuid(),
		network_mode: types::NetworkMode::Bridge,
		network_ports: ports,
	})
	.tag("server_id", server_id)
	.dispatch()
	.await
	.unwrap();

	sub.next().await.unwrap();

	ctx.op(ds::ops::server::get::Input {
		server_ids: vec![server_id],
	})
	.await
	.unwrap()
	.servers
	.first()
	.unwrap();
}
