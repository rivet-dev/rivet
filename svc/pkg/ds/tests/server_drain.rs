use chirp_workflow::prelude::*;
use ds::types;
use rivet_operation::prelude::proto::backend;
use serde_json::json;

#[workflow_test]
async fn server_drain(ctx: TestCtx) {
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

	let build_res = op!([ctx] faker_build {
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
		("HTTP_PORT".to_string(), 28234.to_string()),
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
	// Collect into hashmap
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
		runtime: ds::types::GameRuntime::Pegboard,
		resources: ds::types::ServerResources {
			cpu_millicores: 50,
			memory_mib: 50,
		},
		kill_timeout_ms: 10000,
		tags: vec![(String::from("test"), String::from("123"))]
			.into_iter()
			.collect(),
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

	ctx.signal(ds::workflows::server::Drain {
		drain_timeout: 30000,
	})
	.tag("server_id", server_id)
	.send()
	.await
	.unwrap();

	tokio::time::sleep(std::time::Duration::from_secs(5)).await;

	ctx.signal(ds::workflows::server::Undrain {})
		.tag("server_id", server_id)
		.send()
		.await
		.unwrap();

	ctx.signal(ds::workflows::server::Drain { drain_timeout: 0 })
		.tag("server_id", server_id)
		.send()
		.await
		.unwrap();
}
