use chirp_worker::prelude::*;
use proto::backend;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let res = op!([ctx] game_token_development_validate {
		hostname: util::faker::ident(),
		lobby_ports: vec![backend::matchmaker::lobby_runtime::Port {
			label: "default".to_owned(),
			target_port: Some(80),
			port_range: None,
			proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Http as i32,
			proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
		}]
	})
	.await
	.unwrap();

	assert_eq!(res.errors.len(), 0, "validation failed");
}
