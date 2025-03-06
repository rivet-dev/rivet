use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();
	let ns_id = game_res.namespace_ids.first().unwrap();

	op!([ctx] mm_config_namespace_config_set {
		namespace_id: Some(*ns_id),
		lobby_count_max: 123,
		max_players_per_client: 3,
		max_players_per_client_vpn: 4,
		max_players_per_client_proxy: 5,
		max_players_per_client_tor: 6,
		max_players_per_client_hosting: 7,
	})
	.await
	.unwrap();

	let get_res = op!([ctx] mm_config_namespace_get {
		namespace_ids: vec![*ns_id],
	})
	.await
	.unwrap();
	let ns_config = get_res.namespaces.first().unwrap().config.as_ref().unwrap();
	assert_eq!(123, ns_config.lobby_count_max, "not updated");
	assert_eq!(3, ns_config.max_players_per_client, "not updated");
	assert_eq!(4, ns_config.max_players_per_client_vpn, "not updated");
	assert_eq!(5, ns_config.max_players_per_client_proxy, "not updated");
	assert_eq!(6, ns_config.max_players_per_client_tor, "not updated");
	assert_eq!(7, ns_config.max_players_per_client_hosting, "not updated");
}
