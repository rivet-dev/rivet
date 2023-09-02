use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn set_lobby_state(ctx: TestCtx) {
	let lobby = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();
	let lobby_id = lobby.lobby_id.unwrap();

	let json = Some(r#"{ "foo": "bar" }"#.to_string());
	msg!([ctx] mm::msg::lobby_state_set(lobby_id) -> mm::msg::lobby_state_set_complete {
		lobby_id: Some(lobby_id),
		state_json: json.clone(),
	})
	.await
	.unwrap();

	let res = op!([ctx] mm_lobby_state_get {
		lobby_ids: vec![lobby_id],
	})
	.await
	.unwrap();
	let lobby = res.lobbies.first().unwrap();

	assert_eq!(json, lobby.state_json);
}
