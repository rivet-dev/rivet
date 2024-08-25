use chirp_workflow::prelude::*;
use ds::types;
// use rivet_api::{apis::*, models};
use rivet_operation::prelude::proto::backend;
use serde_json::json;

#[workflow_test]
async fn server_create(ctx: TestCtx) {
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

	ctx.dispatch_tagged_workflow(
		&json!({
			"server_id": server_id,
		}),
		ds::workflows::server::Input {
			server_id,
			env_id: *env_id,
			cluster_id,
			datacenter_id: faker_region.region_id.unwrap().as_uuid(),
			resources: ds::types::ServerResources {
				cpu_millicores: 100,
				memory_mib: 200,
			},
			kill_timeout_ms: 0,
			tags: vec![(String::from("test"), String::from("123"))]
				.into_iter()
				.collect(),
			args: Vec::new(),
			environment: env,
			image_id: build_res.build_id.unwrap().as_uuid(),
			network_mode: types::NetworkMode::Bridge,
			network_ports: ports,
		},
	)
	.await
	.unwrap();

	sub.next().await.unwrap();

	// Async sleep for 5 seconds
	tokio::time::sleep(std::time::Duration::from_secs(5)).await;

	let server = ctx
		.op(ds::ops::server::get::Input {
			server_ids: vec![server_id],
		})
		.await
		.unwrap()
		.servers
		.into_iter()
		.next()
		.unwrap();

	let hostname = server
		.network_ports
		.get("testing2")
		.unwrap()
		.public_hostname
		.as_ref()
		.expect("no public hostname");

	// Echo body
	let random_body = Uuid::new_v4().to_string();
	let client = reqwest::Client::new();
	let res = client
		.post(format!("http://{hostname}"))
		.body(random_body.clone())
		.send()
		.await
		.unwrap()
		.error_for_status()
		.unwrap();
	let res_text = res.text().await.unwrap();
	assert_eq!(random_body, res_text, "echoed wrong response");

	// assert_eq!(game_res.prod_env_id.unwrap(), server.env_id.unwrap().as_uuid());
}
