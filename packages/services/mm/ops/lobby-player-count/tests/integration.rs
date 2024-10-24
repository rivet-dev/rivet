use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

struct TestPlayer {
	namespace_id: Uuid,
	lobby_id: Uuid,
	player_id: Uuid,
	registered: bool,
}

#[worker_test]
async fn empty(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let lobby_a = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();
	let lobby_a_namespace_id = lobby_a.namespace_id.as_ref().unwrap().as_uuid();
	let lobby_a_id = lobby_a.lobby_id.as_ref().unwrap().as_uuid();

	let lobby_b = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();
	let lobby_b_namespace_id = lobby_a.namespace_id.as_ref().unwrap().as_uuid();
	let lobby_b_id = lobby_b.lobby_id.as_ref().unwrap().as_uuid();

	let players = vec![
		TestPlayer {
			namespace_id: lobby_a_namespace_id,
			lobby_id: lobby_a_id,
			player_id: Uuid::new_v4(),
			registered: true,
		},
		TestPlayer {
			namespace_id: lobby_a_namespace_id,
			lobby_id: lobby_a_id,
			player_id: Uuid::new_v4(),
			registered: true,
		},
		TestPlayer {
			namespace_id: lobby_a_namespace_id,
			lobby_id: lobby_a_id,
			player_id: Uuid::new_v4(),
			registered: false,
		},
		TestPlayer {
			namespace_id: lobby_b_namespace_id,
			lobby_id: lobby_b_id,
			player_id: Uuid::new_v4(),
			registered: true,
		},
	];

	for player in &players {
		let query_id = Uuid::new_v4();
		msg!([ctx] @notrace mm::msg::lobby_find(player.namespace_id, query_id) -> Result<mm::msg::lobby_find_complete, mm::msg::lobby_find_fail> {
			namespace_id: Some(player.namespace_id.into()),
			query_id: Some(query_id.into()),
			join_kind: backend::matchmaker::query::JoinKind::Normal as i32,
			players: vec![mm::msg::lobby_find::Player {
				player_id: Some(player.player_id.into()),
				token_session_id: Some(Uuid::new_v4().into()),
				client_info:None,
			}],
			query: Some(mm::msg::lobby_find::message::Query::Direct(backend::matchmaker::query::Direct {
				lobby_id: Some(player.lobby_id.into()),
			})),
			..Default::default()
		})
		.await
		.unwrap().unwrap();

		if player.registered {
			msg!([ctx] mm::msg::player_register(player.player_id) -> mm::msg::player_register_complete {
				player_id: Some(player.player_id.into()),
				lobby_id: Some(player.lobby_id.into()),
			})
			.await
			.unwrap();
		}
	}

	let res = op!([ctx] mm_lobby_player_count {
		lobby_ids: vec![
			lobby_a.lobby_id.unwrap(),
			lobby_b.lobby_id.unwrap(),
			Uuid::new_v4().into(),
		],
	})
	.await
	.unwrap();

	assert_eq!(3, res.lobbies.len());

	let lobby_a_res = res
		.lobbies
		.iter()
		.find(|l| l.lobby_id == lobby_a.lobby_id)
		.expect("missing lobby");
	let lobby_b_res = res
		.lobbies
		.iter()
		.find(|l| l.lobby_id == lobby_b.lobby_id)
		.expect("missing lobby");

	assert_eq!(3, lobby_a_res.total_player_count);
	assert_eq!(2, lobby_a_res.registered_player_count);
	assert_eq!(1, lobby_b_res.total_player_count);
	assert_eq!(1, lobby_b_res.registered_player_count);
}
