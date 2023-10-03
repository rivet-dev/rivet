use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn empty(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let lobby_res = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();
	let namespace_id = lobby_res.namespace_id.unwrap();
	let lobby_id = lobby_res.lobby_id.unwrap();

	let player_ids = std::iter::repeat_with(Uuid::new_v4)
		.take(3)
		.collect::<Vec<_>>();
	for player_id in &player_ids {
		let query_id = Uuid::new_v4();

		msg!([ctx] @notrace mm::msg::lobby_find(namespace_id, query_id) -> Result<mm::msg::lobby_find_complete, mm::msg::lobby_find_fail> {
			namespace_id: Some(namespace_id),
			query_id: Some(query_id.into()),
			join_kind: backend::matchmaker::query::JoinKind::Normal as i32,
			players: vec![mm::msg::lobby_find::Player {
				player_id: Some((*player_id).into()),
				token_session_id: Some(Uuid::new_v4().into()),
				client_info:None,
			}],
			query: Some(mm::msg::lobby_find::message::Query::Direct(backend::matchmaker::query::Direct {
				lobby_id: Some(lobby_id),
			})),
			..Default::default()
		})
		.await
		.unwrap().unwrap();
	}

	let res = op!([ctx] mm_player_count_for_namespace {
		namespace_ids: vec![namespace_id],
	})
	.await
	.unwrap();
	assert_eq!(3, res.namespaces[0].player_count);

	for (i, player_id) in player_ids.iter().enumerate() {
		msg!([ctx] mm::msg::player_remove(player_id) -> mm::msg::player_remove_complete{
			player_id: Some((*player_id).into()),
			lobby_id: Some(lobby_id),
			..Default::default()
		})
		.await
		.unwrap();

		let res = op!([ctx] mm_player_count_for_namespace {
			namespace_ids: vec![namespace_id],
		})
		.await
		.unwrap();
		assert_eq!(3 - i as u32 - 1, res.namespaces[0].player_count);
	}
}
