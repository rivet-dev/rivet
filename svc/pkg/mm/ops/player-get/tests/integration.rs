use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn basic(ctx: TestCtx) {
	let lobby_res = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();
	let namespace_id = lobby_res.namespace_id.as_ref().unwrap().as_uuid();
	let lobby_id = lobby_res.lobby_id.as_ref().unwrap().as_uuid();

	let query_id = Uuid::new_v4();
	let player_id = Uuid::new_v4();
	msg!([ctx] @notrace mm::msg::lobby_find(namespace_id, query_id) -> Result<mm::msg::lobby_find_complete, mm::msg::lobby_find_fail> {
		namespace_id: Some(namespace_id.into()),
		query_id: Some(query_id.into()),
		join_kind: backend::matchmaker::query::JoinKind::Normal as i32,
		players: vec![
			mm::msg::lobby_find::Player {
				player_id: Some(player_id.into()),
				token_session_id: Some(Uuid::new_v4().into()),
				client_info: None,
			}
		],
		query: Some(mm::msg::lobby_find::message::Query::Direct(backend::matchmaker::query::Direct {
			lobby_id: Some(lobby_id.into()),
		})),
		..Default::default()
	}).await.unwrap().unwrap();

	let res = op!([ctx] mm_player_get {
		player_ids: vec![player_id.into()],
			..Default::default()
	})
	.await
	.unwrap();
	assert_eq!(1, res.players.len());
}
