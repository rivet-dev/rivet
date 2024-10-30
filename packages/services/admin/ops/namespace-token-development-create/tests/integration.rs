use chirp_worker::prelude::*;
use proto::backend;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();
	let namespace_id = game_res.namespace_ids.first().unwrap();

	op!([ctx] cloud_namespace_token_development_create {
		namespace_id: Some(*namespace_id),
		hostname: "hostname".to_owned(),
		lobby_ports: vec![backend::matchmaker::lobby_runtime::Port {
			label: "test".into(),
			target_port: Some(80),
			port_range: None,
			proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Https as i32,
			proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
		}],
	})
	.await
	.unwrap();
}
