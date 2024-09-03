use std::collections::HashMap;

use chirp_workflow::prelude::*;
use ds::types;
use rivet_operation::prelude::proto::backend;
use serde_json::json;

pub struct Setup {
	pub env_id: Uuid,
	pub cluster_id: Uuid,
	pub datacenter_id: Uuid,
	pub image_id: Uuid,
}

impl Setup {
	pub async fn init(ctx: &TestCtx) -> Self {
		let region_res = op!([ctx] faker_region {}).await.unwrap();
		let region_id = region_res.region_id.as_ref().unwrap().as_uuid();

		let game_res = op!([ctx] faker_game {
			..Default::default()
		})
		.await
		.unwrap();

		let build_res = op!([ctx] faker_build {
			env_id: game_res.prod_env_id,
			image: backend::faker::Image::DsEcho as i32,
		})
		.await
		.unwrap();

		// Pick an existing cluster
		let cluster_id = ctx
			.op(cluster::ops::list::Input {})
			.await
			.unwrap()
			.cluster_ids
			.first()
			.unwrap()
			.to_owned();

		Setup {
			env_id: game_res.prod_env_id.unwrap().as_uuid(),
			cluster_id,
			datacenter_id: region_id,
			image_id: build_res.build_id.unwrap().as_uuid(),
		}
	}

	/// Create bridge server
	pub async fn create_bridge_server(&self, ctx: &TestCtx) -> Uuid {
		self.create_server_inner(ctx, types::NetworkMode::Bridge)
			.await
	}

	/// Create host server
	pub async fn create_host_server(&self, ctx: &TestCtx) -> Uuid {
		self.create_server_inner(ctx, types::NetworkMode::Host)
			.await
	}

	async fn create_server_inner(&self, ctx: &TestCtx, network_mode: types::NetworkMode) -> Uuid {
		let env = vec![
			("HTTP_PORT".to_string(), 8001.to_string()),
			("TCP_PORT".to_string(), 8002.to_string()),
			("UDP_PORT".to_string(), 8003.to_string()),
		]
		.into_iter()
		.collect();

		let ports = vec![
			(
				"test-http".to_string(),
				ds::workflows::server::Port {
					internal_port: Some(8001),
					routing: types::Routing::GameGuard {
						protocol: types::GameGuardProtocol::Http,
					},
				},
			),
			(
				"test-tcp".to_string(),
				ds::workflows::server::Port {
					internal_port: Some(8002),
					routing: types::Routing::GameGuard {
						protocol: types::GameGuardProtocol::Tcp,
					},
				},
			),
			(
				"test-udp".to_string(),
				ds::workflows::server::Port {
					internal_port: Some(8003),
					routing: types::Routing::GameGuard {
						protocol: types::GameGuardProtocol::Udp,
					},
				},
			),
		]
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
			env_id: self.env_id,
			cluster_id: self.cluster_id,
			datacenter_id: self.datacenter_id,
			resources: ds::types::ServerResources {
				cpu_millicores: 100,
				memory_mib: 200,
			},
			kill_timeout_ms: 0,
			tags: HashMap::new(),
			args: Vec::new(),
			environment: env,
			image_id: self.image_id,
			network_mode,
			network_ports: ports,
		})
		.tag("server_id", server_id)
		.dispatch()
		.await
		.unwrap();

		sub.next().await.unwrap();

		// Sleep for 5 seconds
		tokio::time::sleep(std::time::Duration::from_secs(5)).await;

		server_id
	}
}
