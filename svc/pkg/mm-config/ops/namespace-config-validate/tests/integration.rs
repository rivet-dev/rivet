use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let res = op!([ctx] mm_config_namespace_config_validate {
		namespace_id: Some(Uuid::new_v4().into()),
		lobby_count_max: 10000000,
		max_players_per_client: 1,
		max_players_per_client_vpn: 1,
		max_players_per_client_proxy: 1,
		max_players_per_client_tor: 1,
		max_players_per_client_hosting: 1,
	})
	.await
	.unwrap();

	assert_eq!(res.errors.len(), 1, "validation failed");
}
