use std::collections::HashMap;

use chirp_worker::prelude::*;
use maplit::hashmap;
use proto::backend::{self, pkg::*};

// TODO: Test player limits
// TODO: Test all failure cases

#[worker_test]
async fn direct(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let lobby_res = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();

	let find_res = find(
		&ctx,
		FindRequest {
			namespace_id: lobby_res.namespace_id.as_ref().unwrap().as_uuid(),
			players: gen_players(1),
			query: mm::msg::lobby_find::message::Query::Direct(
				backend::matchmaker::query::Direct {
					lobby_id: lobby_res.lobby_id,
				},
			),
			user_id: None,
		},
	)
	.await
	.unwrap();

	assert_eq!(lobby_res.lobby_id, find_res.lobby_id);
}

#[worker_test]
async fn lobby_group_existing(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let lobby_res = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();

	let find_res = find(
		&ctx,
		FindRequest {
			namespace_id: lobby_res.namespace_id.as_ref().unwrap().as_uuid(),
			players: gen_players(1),
			query: mm::msg::lobby_find::message::Query::LobbyGroup(
				backend::matchmaker::query::LobbyGroup {
					lobby_group_ids: vec![lobby_res.lobby_group_id.unwrap()],
					region_ids: vec![lobby_res.region_id.unwrap()],
					auto_create: Some(backend::matchmaker::query::AutoCreate {
						lobby_group_id: lobby_res.lobby_group_id,
						region_id: lobby_res.region_id,
					}),
				},
			),
			user_id: None,
		},
	)
	.await
	.unwrap();

	assert_eq!(lobby_res.lobby_id, find_res.lobby_id, "found wrong lobby");
}

#[worker_test]
async fn direct_closed(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let lobby_res = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();
	let lobby_id = lobby_res.lobby_id.as_ref().unwrap().as_uuid();

	msg!([ctx] mm::msg::lobby_closed_set(lobby_id) -> mm::msg::lobby_closed_set_complete {
		lobby_id: Some(lobby_id.into()),
		is_closed: true,
	})
	.await
	.unwrap();

	find(
		&ctx,
		FindRequest {
			namespace_id: lobby_res.namespace_id.as_ref().unwrap().as_uuid(),
			players: gen_players(1),
			query: mm::msg::lobby_find::message::Query::Direct(
				backend::matchmaker::query::Direct {
					lobby_id: lobby_res.lobby_id,
				},
			),
			user_id: None,
		},
	)
	.await
	.unwrap();
}

#[worker_test]
async fn lobby_group_closed(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let lobby_res = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();
	let lobby_id = lobby_res.lobby_id.as_ref().unwrap().as_uuid();

	// Cannot find closed lobby
	msg!([ctx] mm::msg::lobby_closed_set(lobby_id) -> mm::msg::lobby_closed_set_complete {
		lobby_id: Some(lobby_id.into()),
		is_closed: true,
	})
	.await
	.unwrap();

	let err = find(
		&ctx,
		FindRequest {
			namespace_id: lobby_res.namespace_id.as_ref().unwrap().as_uuid(),
			players: gen_players(1),
			query: mm::msg::lobby_find::message::Query::LobbyGroup(
				backend::matchmaker::query::LobbyGroup {
					lobby_group_ids: vec![lobby_res.lobby_group_id.unwrap()],
					region_ids: vec![lobby_res.region_id.unwrap()],
					auto_create: None,
				},
			),
			user_id: None,
		},
	)
	.await
	.unwrap_err();

	assert_eq!(
		backend::matchmaker::lobby_find::ErrorCode::NoAvailableLobbies as i32,
		err.error_code
	);

	// Can find lobby once opened again
	msg!([ctx] mm::msg::lobby_closed_set(lobby_id) -> mm::msg::lobby_closed_set_complete {
		lobby_id: Some(lobby_id.into()),
		is_closed: false,
	})
	.await
	.unwrap();

	let res = find(
		&ctx,
		FindRequest {
			namespace_id: lobby_res.namespace_id.as_ref().unwrap().as_uuid(),
			players: gen_players(1),
			query: mm::msg::lobby_find::message::Query::LobbyGroup(
				backend::matchmaker::query::LobbyGroup {
					lobby_group_ids: vec![lobby_res.lobby_group_id.unwrap()],
					region_ids: vec![lobby_res.region_id.unwrap()],
					auto_create: None,
				},
			),
			user_id: None,
		},
	)
	.await
	.unwrap();
	assert_eq!(lobby_res.lobby_id, res.lobby_id);
}

#[worker_test]
async fn lobby_group_closed_auto_create(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let lobby_res = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();
	let lobby_id = lobby_res.lobby_id.as_ref().unwrap().as_uuid();

	msg!([ctx] mm::msg::lobby_closed_set(lobby_id) -> mm::msg::lobby_closed_set_complete {
		lobby_id: Some(lobby_id.into()),
		is_closed: true,
	})
	.await
	.unwrap();

	let res = find(
		&ctx,
		FindRequest {
			namespace_id: lobby_res.namespace_id.as_ref().unwrap().as_uuid(),
			players: gen_players(1),
			query: mm::msg::lobby_find::message::Query::LobbyGroup(
				backend::matchmaker::query::LobbyGroup {
					lobby_group_ids: vec![lobby_res.lobby_group_id.unwrap()],
					region_ids: vec![lobby_res.region_id.unwrap()],
					auto_create: Some(backend::matchmaker::query::AutoCreate {
						lobby_group_id: lobby_res.lobby_group_id,
						region_id: lobby_res.region_id,
					}),
				},
			),
			user_id: None,
		},
	)
	.await
	.unwrap();
	assert_ne!(lobby_res.lobby_id, res.lobby_id);
}

#[worker_test]
async fn lobby_crash_immediate(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let lobby_group = create_lobby_group(&ctx, Some(backend::faker::Image::FailImmediately)).await;

	let err = find(
		&ctx,
		FindRequest {
			namespace_id: lobby_group.namespace_id,
			players: gen_players(1),
			query: mm::msg::lobby_find::message::Query::LobbyGroup(
				backend::matchmaker::query::LobbyGroup {
					lobby_group_ids: vec![lobby_group.lobby_group_id.into()],
					region_ids: vec![lobby_group.region_id.into()],
					auto_create: Some(backend::matchmaker::query::AutoCreate {
						lobby_group_id: Some(lobby_group.lobby_group_id.into()),
						region_id: Some(lobby_group.region_id.into()),
					}),
				},
			),
			user_id: None,
		},
	)
	.await
	.unwrap_err();

	assert_eq!(
		backend::matchmaker::lobby_find::ErrorCode::LobbyStoppedPrematurely as i32,
		err.error_code
	);
}

#[worker_test]
async fn max_players_per_client(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let lobby_res = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();

	let max_players_per_client = 3;
	op!([ctx] mm_config_namespace_config_set {
		namespace_id: lobby_res.namespace_id,
		lobby_count_max: 4,
		max_players_per_client: max_players_per_client,
		max_players_per_client_vpn: max_players_per_client,
		max_players_per_client_proxy: max_players_per_client,
		max_players_per_client_tor: max_players_per_client,
		max_players_per_client_hosting: max_players_per_client,
	})
	.await
	.unwrap();

	let fake_ip = util::faker::ip_addr_v4();
	tracing::info!(%fake_ip, "fake ip");

	for i in 0..(max_players_per_client + 2) {
		tracing::info!(i, "find iter");

		let res = find(
			&ctx,
			FindRequest {
				namespace_id: lobby_res.namespace_id.as_ref().unwrap().as_uuid(),
				players: vec![mm::msg::lobby_find::Player {
					player_id: Some(Uuid::new_v4().into()),
					token_session_id: Some(Uuid::new_v4().into()),
					client_info: Some(backend::net::ClientInfo {
						user_agent: Some("Test".into()),
						remote_address: Some(fake_ip.to_string()),
					}),
				}],
				query: mm::msg::lobby_find::message::Query::Direct(
					backend::matchmaker::query::Direct {
						lobby_id: lobby_res.lobby_id,
					},
				),
				user_id: None,
			},
		)
		.await;
		if i >= max_players_per_client {
			let err = res.unwrap_err();
			assert_eq!(
				backend::matchmaker::lobby_find::ErrorCode::TooManyPlayersFromSource as i32,
				err.error_code
			);
		} else {
			let _ = res.unwrap();
		}
	}
}

#[worker_test]
async fn lobby_group_auto_create(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let lobby_group = create_lobby_group(&ctx, None).await;

	find(
		&ctx,
		FindRequest {
			namespace_id: lobby_group.namespace_id,
			players: gen_players(1),
			query: mm::msg::lobby_find::message::Query::LobbyGroup(
				backend::matchmaker::query::LobbyGroup {
					lobby_group_ids: vec![lobby_group.lobby_group_id.into()],
					region_ids: vec![lobby_group.region_id.into()],
					auto_create: Some(backend::matchmaker::query::AutoCreate {
						lobby_group_id: Some(lobby_group.lobby_group_id.into()),
						region_id: Some(lobby_group.region_id.into()),
					}),
				},
			),
			user_id: None,
		},
	)
	.await
	.unwrap();
}

#[worker_test]
async fn lobby_group_no_auto_create(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let lobby_group = create_lobby_group(&ctx, None).await;

	let err = find(
		&ctx,
		FindRequest {
			namespace_id: lobby_group.namespace_id,
			players: gen_players(1),
			query: mm::msg::lobby_find::message::Query::LobbyGroup(
				backend::matchmaker::query::LobbyGroup {
					lobby_group_ids: vec![lobby_group.lobby_group_id.into()],
					region_ids: vec![lobby_group.region_id.into()],
					auto_create: None,
				},
			),
			user_id: None,
		},
	)
	.await
	.unwrap_err();

	assert_eq!(
		backend::matchmaker::lobby_find::ErrorCode::NoAvailableLobbies as i32,
		err.error_code
	);
}

#[worker_test]
async fn join_disabled(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let _user_id = Uuid::new_v4();

	let (namespace_id, lobby_id) = gen_disabled_lobby(&ctx).await;

	let err = find(
		&ctx,
		FindRequest {
			namespace_id,
			players: gen_players(1),
			query: mm::msg::lobby_find::message::Query::Direct(
				backend::matchmaker::query::Direct {
					lobby_id: Some(lobby_id.into()),
				},
			),
			user_id: None,
		},
	)
	.await
	.unwrap_err();

	assert_eq!(
		backend::matchmaker::lobby_find::ErrorCode::JoinDisabled as i32,
		err.error_code
	);
}

#[worker_test]
async fn guest_verification(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let user_id = Uuid::new_v4();

	let (namespace_id, lobby_id) =
		gen_verification_lobby(&ctx, backend::matchmaker::IdentityRequirement::Guest, None).await;

	let err = find(
		&ctx,
		FindRequest {
			namespace_id,
			players: gen_players(1),
			query: mm::msg::lobby_find::message::Query::Direct(
				backend::matchmaker::query::Direct {
					lobby_id: Some(lobby_id.into()),
				},
			),
			user_id: None,
		},
	)
	.await
	.unwrap_err();

	assert_eq!(
		backend::matchmaker::lobby_find::ErrorCode::IdentityRequired as i32,
		err.error_code
	);

	let _find_res = find(
		&ctx,
		FindRequest {
			namespace_id,
			players: gen_players(1),
			query: mm::msg::lobby_find::message::Query::Direct(
				backend::matchmaker::query::Direct {
					lobby_id: Some(lobby_id.into()),
				},
			),
			user_id: Some(user_id),
		},
	)
	.await
	.unwrap();
}

#[worker_test]
async fn registered_verification(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let user_res = op!([ctx] faker_user {}).await.unwrap();
	let user_id = user_res.user_id.unwrap().as_uuid();

	let (namespace_id, lobby_id) = gen_verification_lobby(
		&ctx,
		backend::matchmaker::IdentityRequirement::Registered,
		None,
	)
	.await;

	let err = find(
		&ctx,
		FindRequest {
			namespace_id,
			players: gen_players(1),
			query: mm::msg::lobby_find::message::Query::Direct(
				backend::matchmaker::query::Direct {
					lobby_id: Some(lobby_id.into()),
				},
			),
			user_id: None,
		},
	)
	.await
	.unwrap_err();

	assert_eq!(
		backend::matchmaker::lobby_find::ErrorCode::IdentityRequired as i32,
		err.error_code
	);

	let err = find(
		&ctx,
		FindRequest {
			namespace_id,
			players: gen_players(1),
			query: mm::msg::lobby_find::message::Query::Direct(
				backend::matchmaker::query::Direct {
					lobby_id: Some(lobby_id.into()),
				},
			),
			user_id: Some(user_id),
		},
	)
	.await
	.unwrap_err();

	assert_eq!(
		backend::matchmaker::lobby_find::ErrorCode::RegistrationRequired as i32,
		err.error_code
	);

	let email = util::faker::email();
	op!([ctx] user_identity_create {
		user_id: user_res.user_id,
		identity: Some(backend::user_identity::Identity {
			kind: Some(backend::user_identity::identity::Kind::Email(
				backend::user_identity::identity::Email {
					email: email.clone(),
				}
			)),
		}),
	})
	.await
	.unwrap();

	let _find_res = find(
		&ctx,
		FindRequest {
			namespace_id,
			players: gen_players(1),
			query: mm::msg::lobby_find::message::Query::Direct(
				backend::matchmaker::query::Direct {
					lobby_id: Some(lobby_id.into()),
				},
			),
			user_id: Some(user_id),
		},
	)
	.await
	.unwrap();
}

#[worker_test]
async fn bypass_verification(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let _user_id = Uuid::new_v4();

	let (namespace_id, lobby_id) = gen_verification_lobby(
		&ctx,
		backend::matchmaker::IdentityRequirement::Registered,
		None,
	)
	.await;

	let _find_res = find_with_verification(
		&ctx,
		FindRequest {
			namespace_id,
			players: gen_players(1),
			query: mm::msg::lobby_find::message::Query::Direct(
				backend::matchmaker::query::Direct {
					lobby_id: Some(lobby_id.into()),
				},
			),
			user_id: None,
		},
		None,
		true,
	)
	.await
	.unwrap();
}

// TODO: Find way to actually verify user data
#[worker_test]
async fn external_verification(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let user_id = Uuid::new_v4();

	let (namespace_id, lobby_id) = gen_verification_lobby(
		&ctx,
		backend::matchmaker::IdentityRequirement::None,
		Some(backend::matchmaker::VerificationConfig {
			url: "https://httpstat.us/403".to_string(),
			headers: IntoIterator::into_iter([("accept".to_string(), "text/plain".to_string())])
				.collect::<HashMap<_, _>>(),
		}),
	)
	.await;

	let err = find(
		&ctx,
		FindRequest {
			namespace_id,
			players: gen_players(1),
			query: mm::msg::lobby_find::message::Query::Direct(
				backend::matchmaker::query::Direct {
					lobby_id: Some(lobby_id.into()),
				},
			),
			user_id: None,
		},
	)
	.await
	.unwrap_err();

	assert_eq!(
		backend::matchmaker::lobby_find::ErrorCode::VerificationFailed as i32,
		err.error_code
	);

	let (namespace_id, lobby_id) = gen_verification_lobby(
		&ctx,
		backend::matchmaker::IdentityRequirement::None,
		Some(backend::matchmaker::VerificationConfig {
			url: "https://httpstat.us/200".to_string(),
			headers: IntoIterator::into_iter([("accept".to_string(), "text/plain".to_string())])
				.collect::<HashMap<_, _>>(),
		}),
	)
	.await;

	let _find_res = find(
		&ctx,
		FindRequest {
			namespace_id,
			players: gen_players(1),
			query: mm::msg::lobby_find::message::Query::Direct(
				backend::matchmaker::query::Direct {
					lobby_id: Some(lobby_id.into()),
				},
			),
			user_id: Some(user_id),
		},
	)
	.await
	.unwrap();
}

#[worker_test]
async fn tagged(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let lobby_group = create_lobby_group(&ctx, None).await;

	let find_res1 = find_with_tags(
		&ctx,
		FindRequest {
			namespace_id: lobby_group.namespace_id,
			players: gen_players(1),
			query: mm::msg::lobby_find::message::Query::LobbyGroup(
				backend::matchmaker::query::LobbyGroup {
					lobby_group_ids: vec![lobby_group.lobby_group_id.into()],
					region_ids: vec![lobby_group.region_id.into()],
					auto_create: Some(backend::matchmaker::query::AutoCreate {
						lobby_group_id: Some(lobby_group.lobby_group_id.into()),
						region_id: Some(lobby_group.region_id.into()),
					}),
				},
			),
			user_id: None,
		},
		hashmap! {
			"mytag".to_string() => "1234".to_string(),
		},
	)
	.await
	.unwrap();

	let find_res2 = find_with_tags(
		&ctx,
		FindRequest {
			namespace_id: lobby_group.namespace_id,
			players: gen_players(1),
			query: mm::msg::lobby_find::message::Query::LobbyGroup(
				backend::matchmaker::query::LobbyGroup {
					lobby_group_ids: vec![lobby_group.lobby_group_id.into()],
					region_ids: vec![lobby_group.region_id.into()],
					auto_create: Some(backend::matchmaker::query::AutoCreate {
						lobby_group_id: Some(lobby_group.lobby_group_id.into()),
						region_id: Some(lobby_group.region_id.into()),
					}),
				},
			),
			user_id: None,
		},
		hashmap! {
			"mytag".to_string() => "1234".to_string(),
		},
	)
	.await
	.unwrap();

	assert_eq!(find_res1.lobby_id, find_res2.lobby_id, "found wrong lobby");

	let find_res3 = find_with_tags(
		&ctx,
		FindRequest {
			namespace_id: lobby_group.namespace_id,
			players: gen_players(1),
			query: mm::msg::lobby_find::message::Query::LobbyGroup(
				backend::matchmaker::query::LobbyGroup {
					lobby_group_ids: vec![lobby_group.lobby_group_id.into()],
					region_ids: vec![lobby_group.region_id.into()],
					auto_create: Some(backend::matchmaker::query::AutoCreate {
						lobby_group_id: Some(lobby_group.lobby_group_id.into()),
						region_id: Some(lobby_group.region_id.into()),
					}),
				},
			),
			user_id: None,
		},
		hashmap! {
			"mytag".to_string() => "1234".to_string(),
			"othertag".to_string() => "foobar".to_string(),
		},
	)
	.await
	.unwrap();

	assert_ne!(find_res2.lobby_id, find_res3.lobby_id, "found wrong lobby");
}

#[worker_test]
async fn tagged_no_auto_create(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let lobby_group = create_lobby_group(&ctx, None).await;

	let err = find_with_tags(
		&ctx,
		FindRequest {
			namespace_id: lobby_group.namespace_id,
			players: gen_players(1),
			query: mm::msg::lobby_find::message::Query::LobbyGroup(
				backend::matchmaker::query::LobbyGroup {
					lobby_group_ids: vec![lobby_group.lobby_group_id.into()],
					region_ids: vec![lobby_group.region_id.into()],
					auto_create: None,
				},
			),
			user_id: None,
		},
		hashmap! {
			"mytag".to_string() => "1234".to_string(),
		},
	)
	.await
	.unwrap_err();

	assert_eq!(
		backend::matchmaker::lobby_find::ErrorCode::NoAvailableLobbies as i32,
		err.error_code
	);
}

#[worker_test]
async fn tagged_multiple_game_modes(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let lobby_group = create_lobby_group(&ctx, None).await;

	let _find_res1 = find_with_tags(
		&ctx,
		FindRequest {
			namespace_id: lobby_group.namespace_id,
			players: gen_players(2),
			query: mm::msg::lobby_find::message::Query::LobbyGroup(
				backend::matchmaker::query::LobbyGroup {
					lobby_group_ids: vec![lobby_group.lobby_group_id.into()],
					region_ids: vec![lobby_group.region_id.into()],
					auto_create: Some(backend::matchmaker::query::AutoCreate {
						lobby_group_id: Some(lobby_group.lobby_group_id.into()),
						region_id: Some(lobby_group.region_id.into()),
					}),
				},
			),
			user_id: None,
		},
		hashmap! {
			"mytag".to_string() => "1234".to_string(),
		},
	)
	.await
	.unwrap();

	let find_res2 = find_with_tags(
		&ctx,
		FindRequest {
			namespace_id: lobby_group.namespace_id,
			players: gen_players(1),
			query: mm::msg::lobby_find::message::Query::LobbyGroup(
				backend::matchmaker::query::LobbyGroup {
					lobby_group_ids: vec![lobby_group.lobby_group_id2.into()],
					region_ids: vec![lobby_group.region_id.into()],
					auto_create: Some(backend::matchmaker::query::AutoCreate {
						lobby_group_id: Some(lobby_group.lobby_group_id2.into()),
						region_id: Some(lobby_group.region_id.into()),
					}),
				},
			),
			user_id: None,
		},
		hashmap! {
			"mytag".to_string() => "4321".to_string(),
		},
	)
	.await
	.unwrap();

	// This should iterate over the both of the previously created lobbies and skip the first one because
	// it's tag doesn't match.
	let find_res3 = find_with_tags(
		&ctx,
		FindRequest {
			namespace_id: lobby_group.namespace_id,
			players: gen_players(1),
			query: mm::msg::lobby_find::message::Query::LobbyGroup(
				backend::matchmaker::query::LobbyGroup {
					lobby_group_ids: vec![
						lobby_group.lobby_group_id.into(),
						lobby_group.lobby_group_id2.into(),
					],
					region_ids: vec![lobby_group.region_id.into()],
					auto_create: None,
				},
			),
			user_id: None,
		},
		hashmap! {
			"mytag".to_string() => "4321".to_string(),
		},
	)
	.await
	.unwrap();

	assert_eq!(find_res2.lobby_id, find_res3.lobby_id, "found wrong lobby");
}

#[worker_test]
async fn tagged_multiple_lobbies(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let lobby_group = create_lobby_group(&ctx, None).await;

	let _find_res1 = find_with_tags(
		&ctx,
		FindRequest {
			namespace_id: lobby_group.namespace_id,
			players: gen_players(2),
			query: mm::msg::lobby_find::message::Query::LobbyGroup(
				backend::matchmaker::query::LobbyGroup {
					lobby_group_ids: vec![lobby_group.lobby_group_id.into()],
					region_ids: vec![lobby_group.region_id.into()],
					auto_create: Some(backend::matchmaker::query::AutoCreate {
						lobby_group_id: Some(lobby_group.lobby_group_id.into()),
						region_id: Some(lobby_group.region_id.into()),
					}),
				},
			),
			user_id: None,
		},
		hashmap! {
			"mytag".to_string() => "1234".to_string(),
		},
	)
	.await
	.unwrap();

	let find_res2 = find_with_tags(
		&ctx,
		FindRequest {
			namespace_id: lobby_group.namespace_id,
			players: gen_players(1),
			query: mm::msg::lobby_find::message::Query::LobbyGroup(
				backend::matchmaker::query::LobbyGroup {
					lobby_group_ids: vec![lobby_group.lobby_group_id.into()],
					region_ids: vec![lobby_group.region_id.into()],
					auto_create: Some(backend::matchmaker::query::AutoCreate {
						lobby_group_id: Some(lobby_group.lobby_group_id.into()),
						region_id: Some(lobby_group.region_id.into()),
					}),
				},
			),
			user_id: None,
		},
		hashmap! {
			"mytag".to_string() => "4321".to_string(),
		},
	)
	.await
	.unwrap();

	// This should iterate over the both of the previously created lobbies and skip the first one because
	// it's tag doesn't match.
	let find_res3 = find_with_tags(
		&ctx,
		FindRequest {
			namespace_id: lobby_group.namespace_id,
			players: gen_players(1),
			query: mm::msg::lobby_find::message::Query::LobbyGroup(
				backend::matchmaker::query::LobbyGroup {
					lobby_group_ids: vec![lobby_group.lobby_group_id.into()],
					region_ids: vec![lobby_group.region_id.into()],
					auto_create: None,
				},
			),
			user_id: None,
		},
		hashmap! {
			"mytag".to_string() => "4321".to_string(),
		},
	)
	.await
	.unwrap();

	assert_eq!(find_res2.lobby_id, find_res3.lobby_id, "found wrong lobby");
}

#[worker_test]
async fn tagged_subset(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let lobby_group = create_lobby_group(&ctx, None).await;

	let find_res1 = find_with_tags(
		&ctx,
		FindRequest {
			namespace_id: lobby_group.namespace_id,
			players: gen_players(1),
			query: mm::msg::lobby_find::message::Query::LobbyGroup(
				backend::matchmaker::query::LobbyGroup {
					lobby_group_ids: vec![lobby_group.lobby_group_id.into()],
					region_ids: vec![lobby_group.region_id.into()],
					auto_create: Some(backend::matchmaker::query::AutoCreate {
						lobby_group_id: Some(lobby_group.lobby_group_id.into()),
						region_id: Some(lobby_group.region_id.into()),
					}),
				},
			),
			user_id: None,
		},
		hashmap! {
			"mytag".to_string() => "1234".to_string(),
			"othertag".to_string() => "foobar".to_string(),
		},
	)
	.await
	.unwrap();

	// Subset tags will still match
	let find_res2 = find_with_tags(
		&ctx,
		FindRequest {
			namespace_id: lobby_group.namespace_id,
			players: gen_players(1),
			query: mm::msg::lobby_find::message::Query::LobbyGroup(
				backend::matchmaker::query::LobbyGroup {
					lobby_group_ids: vec![lobby_group.lobby_group_id.into()],
					region_ids: vec![lobby_group.region_id.into()],
					auto_create: None,
				},
			),
			user_id: None,
		},
		hashmap! {
			"mytag".to_string() => "1234".to_string(),
		},
	)
	.await
	.unwrap();

	assert_eq!(find_res1.lobby_id, find_res2.lobby_id, "found wrong lobby");
}

#[worker_test]
async fn dynamic_max_players(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let lobby_group = create_lobby_group(&ctx, None).await;

	let find_res = find_with_dynamic_max_players(
		&ctx,
		FindRequest {
			namespace_id: lobby_group.namespace_id,
			players: gen_players(1),
			query: mm::msg::lobby_find::message::Query::LobbyGroup(
				backend::matchmaker::query::LobbyGroup {
					lobby_group_ids: vec![lobby_group.lobby_group_id.into()],
					region_ids: vec![lobby_group.region_id.into()],
					auto_create: Some(backend::matchmaker::query::AutoCreate {
						lobby_group_id: Some(lobby_group.lobby_group_id.into()),
						region_id: Some(lobby_group.region_id.into()),
					}),
				},
			),
			user_id: None,
		},
		1,
	)
	.await
	.unwrap();

	let err = find(
		&ctx,
		FindRequest {
			namespace_id: lobby_group.namespace_id,
			players: gen_players(1),
			query: mm::msg::lobby_find::message::Query::Direct(
				backend::matchmaker::query::Direct {
					lobby_id: find_res.lobby_id,
				},
			),
			user_id: None,
		},
	)
	.await
	.unwrap_err();

	assert_eq!(
		backend::matchmaker::lobby_find::ErrorCode::LobbyFull as i32,
		err.error_code
	);
}

fn gen_players(count: usize) -> Vec<mm::msg::lobby_find::Player> {
	let mut players = Vec::new();
	for _ in 0..count {
		players.push(mm::msg::lobby_find::Player {
			player_id: Some(Uuid::new_v4().into()),
			token_session_id: Some(Uuid::new_v4().into()),
			client_info: Some(backend::net::ClientInfo {
				user_agent: Some("Test".into()),
				remote_address: Some(util::faker::ip_addr_v4().to_string()),
			}),
		});
	}
	players
}

async fn gen_verification_lobby(
	ctx: &TestCtx,
	identity_requirement: backend::matchmaker::IdentityRequirement,
	verification: Option<backend::matchmaker::VerificationConfig>,
) -> (Uuid, Uuid) {
	let region_res = op!([ctx] faker_region {}).await.unwrap();
	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();
	let namespace_id = game_res.namespace_ids.first().unwrap();

	let build_res = op!([ctx] faker_build {
		game_id: game_res.game_id,
		image: backend::faker::Image::MmLobbyAutoReady as i32,
	})
	.await
	.unwrap();

	let version_create_res = op!([ctx] faker_game_version {
		game_id: game_res.game_id,
		override_lobby_groups: Some(faker::game_version::request::OverrideLobbyGroups {
			lobby_groups: vec![backend::matchmaker::LobbyGroup {
				name_id: "test-1".into(),

				regions: vec![
					backend::matchmaker::lobby_group::Region {
						region_id: region_res.region_id,
						tier_name_id: util_mm::test::TIER_NAME_ID.to_owned(),
						idle_lobbies: Some(backend::matchmaker::lobby_group::IdleLobbies {
							min_idle_lobbies: 0,
							max_idle_lobbies: 2,
						}),
					},
				],

				max_players_normal: 8,
				max_players_direct: 10,
				max_players_party: 12,
				listable: true,
				taggable: false,
				allow_dynamic_max_players: false,

				runtime: Some(
					backend::matchmaker::lobby_runtime::Docker {
						build_id: build_res.build_id,
						args: Vec::new(),
						env_vars: Vec::new(),
						network_mode:
							backend::matchmaker::lobby_runtime::NetworkMode::Bridge
								as i32,
						ports: Vec::new(),
					}
					.into(),
				),

				actions: Some(backend::matchmaker::lobby_group::Actions {
					find: Some(backend::matchmaker::FindConfig {
						enabled: true,
						identity_requirement: identity_requirement as i32,
						verification: verification.clone(),
					}),
					join: Some(backend::matchmaker::JoinConfig {
						enabled: true,
						identity_requirement: identity_requirement as i32,
						verification,
					}),
					create: None,
				})
			}],
		}),
	})
	.await
	.unwrap();

	let lobby_res = op!([ctx] faker_mm_lobby {
		namespace_id: Some(*namespace_id),
		version_id: version_create_res.version_id,
		..Default::default()
	})
	.await
	.unwrap();

	(
		lobby_res.namespace_id.unwrap().as_uuid(),
		lobby_res.lobby_id.unwrap().as_uuid(),
	)
}

async fn gen_disabled_lobby(ctx: &TestCtx) -> (Uuid, Uuid) {
	let region_res = op!([ctx] faker_region {}).await.unwrap();
	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();
	let namespace_id = game_res.namespace_ids.first().unwrap();

	let build_res = op!([ctx] faker_build {
		game_id: game_res.game_id,
		image: backend::faker::Image::MmLobbyAutoReady as i32,
	})
	.await
	.unwrap();

	let version_create_res = op!([ctx] faker_game_version {
		game_id: game_res.game_id,
		override_lobby_groups: Some(faker::game_version::request::OverrideLobbyGroups {
			lobby_groups: vec![backend::matchmaker::LobbyGroup {
				name_id: "test-1".into(),

				regions: vec![
					backend::matchmaker::lobby_group::Region {
						region_id: region_res.region_id,
						tier_name_id: util_mm::test::TIER_NAME_ID.to_owned(),
						idle_lobbies: Some(backend::matchmaker::lobby_group::IdleLobbies {
							min_idle_lobbies: 0,
							max_idle_lobbies: 2,
						}),
					},
				],

				max_players_normal: 8,
				max_players_direct: 10,
				max_players_party: 12,
				listable: true,
				taggable: false,
				allow_dynamic_max_players: false,

				runtime: Some(
					backend::matchmaker::lobby_runtime::Docker {
						build_id: build_res.build_id,
						args: Vec::new(),
						env_vars: Vec::new(),
						network_mode:
							backend::matchmaker::lobby_runtime::NetworkMode::Bridge
								as i32,
						ports: Vec::new(),
					}
					.into(),
				),

				actions: Some(backend::matchmaker::lobby_group::Actions {
					find: Some(backend::matchmaker::FindConfig {
						enabled: false,
						identity_requirement: backend::matchmaker::IdentityRequirement::None as i32,
						verification: None,
					}),
					join: Some(backend::matchmaker::JoinConfig {
						enabled: false,
						identity_requirement: backend::matchmaker::IdentityRequirement::None as i32,
						verification: None,
					}),
					create: None,
				})
			}],
		}),
	})
	.await
	.unwrap();

	let lobby_res = op!([ctx] faker_mm_lobby {
		namespace_id: Some(*namespace_id),
		version_id: version_create_res.version_id,
		..Default::default()
	})
	.await
	.unwrap();

	(
		lobby_res.namespace_id.unwrap().as_uuid(),
		lobby_res.lobby_id.unwrap().as_uuid(),
	)
}

struct TestLobbyGroup {
	lobby_group_id: Uuid,
	lobby_group_id2: Uuid,
	#[allow(unused)]
	version_id: Uuid,
	namespace_id: Uuid,
	region_id: Uuid,
}

async fn create_lobby_group(ctx: &TestCtx, image: Option<backend::faker::Image>) -> TestLobbyGroup {
	let region_res = op!([ctx] faker_region {}).await.unwrap();
	let region_id = region_res.region_id.as_ref().unwrap().as_uuid();

	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();
	let namespace_id = game_res.namespace_ids.first().unwrap().as_uuid();

	let build_res = op!([ctx] faker_build {
		game_id: game_res.game_id,
		image: image.unwrap_or(backend::faker::Image::MmLobbyAutoReady) as i32,
	})
	.await
	.unwrap();

	let game_version_res = op!([ctx] faker_game_version {
		game_id: game_res.game_id,
		override_lobby_groups: Some(faker::game_version::request::OverrideLobbyGroups {
			lobby_groups: vec![backend::matchmaker::LobbyGroup {
				name_id: "faker-lg".into(),

				regions: vec![backend::matchmaker::lobby_group::Region {
					region_id: region_res.region_id,
					tier_name_id: util_mm::test::TIER_NAME_ID.to_owned(),
					idle_lobbies: None,
				}],
				max_players_normal: 8,
				max_players_direct: 10,
				max_players_party: 12,
				listable: true,
				taggable: false,
				allow_dynamic_max_players: false,

				runtime: Some(backend::matchmaker::lobby_runtime::Docker {
					// We can't use `curlimages/curl` here because it doesn't allow for
					// variable interpolation, so we need a container that has a proper shell
					// that we can inject variables with.
					build_id: build_res.build_id,
					args: vec![],
					env_vars: vec![backend::matchmaker::lobby_runtime::EnvVar {
						key: "HELLO".into(),
						value: "world".into(),
					}],
					network_mode: backend::matchmaker::lobby_runtime::NetworkMode::Bridge as i32,
					ports: vec![backend::matchmaker::lobby_runtime::Port {
						label: "1234".into(),
						target_port: Some(1234),
						port_range: None,
						proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Https as i32,
						proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
					}],
				}.into()),

				actions: None,
			}, backend::matchmaker::LobbyGroup {
				name_id: "faker-lg2".into(),

				regions: vec![backend::matchmaker::lobby_group::Region {
					region_id: region_res.region_id,
					tier_name_id: util_mm::test::TIER_NAME_ID.to_owned(),
					idle_lobbies: None,
				}],
				max_players_normal: 8,
				max_players_direct: 10,
				max_players_party: 12,
				listable: true,
				taggable: false,
				allow_dynamic_max_players: false,

				runtime: Some(backend::matchmaker::lobby_runtime::Docker {
					build_id: build_res.build_id,
					args: vec![],
					env_vars: vec![backend::matchmaker::lobby_runtime::EnvVar {
						key: "HELLO".into(),
						value: "world".into(),
					}],
					network_mode: backend::matchmaker::lobby_runtime::NetworkMode::Bridge as i32,
					ports: vec![backend::matchmaker::lobby_runtime::Port {
						label: "1234".into(),
						target_port: Some(1234),
						port_range: None,
						proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Https as i32,
						proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
					}],
				}.into()),

				actions: None,
			}],
		}),
		..Default::default()
	})
	.await
	.unwrap();
	let version_id = game_version_res.version_id.as_ref().unwrap().as_uuid();

	let version_get_res = op!([ctx] mm_config_version_get {
		version_ids: vec![version_id.into()],
	})
	.await
	.unwrap();
	let version = version_get_res.versions.first().unwrap();
	let config_meta = version.config_meta.as_ref().unwrap();
	let lobby_group = config_meta.lobby_groups.first().unwrap();
	let lobby_group_id = lobby_group.lobby_group_id.as_ref().unwrap().as_uuid();

	let lobby_group2 = config_meta.lobby_groups.get(1).unwrap();
	let lobby_group_id2 = lobby_group2.lobby_group_id.as_ref().unwrap().as_uuid();

	op!([ctx] game_namespace_version_set {
		namespace_id: Some(namespace_id.into()),
		version_id: Some(version_id.into()),
	})
	.await
	.unwrap();

	TestLobbyGroup {
		lobby_group_id,
		lobby_group_id2,
		version_id,
		namespace_id,
		region_id,
	}
}

// TODO: Split into multiple functions
struct FindRequest {
	namespace_id: Uuid,
	players: Vec<mm::msg::lobby_find::Player>,
	query: mm::msg::lobby_find::message::Query,
	user_id: Option<Uuid>,
}

async fn find(
	ctx: &TestCtx,
	req: FindRequest,
) -> Result<
	chirp_client::message::ReceivedMessage<mm::msg::lobby_find_complete::Message>,
	chirp_client::message::ReceivedMessage<mm::msg::lobby_find_fail::Message>,
> {
	let query_id = Uuid::new_v4();
	msg!([ctx] @notrace mm::msg::lobby_find(req.namespace_id, query_id) -> Result<mm::msg::lobby_find_complete, mm::msg::lobby_find_fail> {
		namespace_id: Some(req.namespace_id.into()),
		query_id: Some(query_id.into()),
		join_kind: backend::matchmaker::query::JoinKind::Normal as i32,
		players: req.players,
		query: Some(req.query),

		user_id: req.user_id.map(Into::into),
		verification_data_json: None,
		bypass_verification: false,
		tags: HashMap::new(),
		dynamic_max_players: None,

		debug: None,
	})
	.await
	.unwrap()
}

async fn find_with_verification(
	ctx: &TestCtx,
	req: FindRequest,
	verification_data_json: Option<String>,
	bypass_verification: bool,
) -> Result<
	chirp_client::message::ReceivedMessage<mm::msg::lobby_find_complete::Message>,
	chirp_client::message::ReceivedMessage<mm::msg::lobby_find_fail::Message>,
> {
	let query_id = Uuid::new_v4();
	msg!([ctx] @notrace mm::msg::lobby_find(req.namespace_id, query_id) -> Result<mm::msg::lobby_find_complete, mm::msg::lobby_find_fail> {
		namespace_id: Some(req.namespace_id.into()),
		query_id: Some(query_id.into()),
		join_kind: backend::matchmaker::query::JoinKind::Normal as i32,
		players: req.players,
		query: Some(req.query),

		user_id: req.user_id.map(Into::into),
		verification_data_json: verification_data_json,
		bypass_verification: bypass_verification,
		tags: HashMap::new(),
		dynamic_max_players: None,

		debug: None,
	})
	.await
	.unwrap()
}

async fn find_with_tags(
	ctx: &TestCtx,
	req: FindRequest,
	tags: HashMap<String, String>,
) -> Result<
	chirp_client::message::ReceivedMessage<mm::msg::lobby_find_complete::Message>,
	chirp_client::message::ReceivedMessage<mm::msg::lobby_find_fail::Message>,
> {
	let query_id = Uuid::new_v4();
	msg!([ctx] @notrace mm::msg::lobby_find(req.namespace_id, query_id) -> Result<mm::msg::lobby_find_complete, mm::msg::lobby_find_fail> {
		namespace_id: Some(req.namespace_id.into()),
		query_id: Some(query_id.into()),
		join_kind: backend::matchmaker::query::JoinKind::Normal as i32,
		players: req.players,
		query: Some(req.query),

		user_id: req.user_id.map(Into::into),
		verification_data_json: None,
		bypass_verification: false,
		tags: tags,
		dynamic_max_players: None,

		debug: None,
	})
	.await
	.unwrap()
}

async fn find_with_dynamic_max_players(
	ctx: &TestCtx,
	req: FindRequest,
	dynamic_max_players: u32,
) -> Result<
	chirp_client::message::ReceivedMessage<mm::msg::lobby_find_complete::Message>,
	chirp_client::message::ReceivedMessage<mm::msg::lobby_find_fail::Message>,
> {
	let query_id = Uuid::new_v4();
	msg!([ctx] @notrace mm::msg::lobby_find(req.namespace_id, query_id) -> Result<mm::msg::lobby_find_complete, mm::msg::lobby_find_fail> {
		namespace_id: Some(req.namespace_id.into()),
		query_id: Some(query_id.into()),
		join_kind: backend::matchmaker::query::JoinKind::Normal as i32,
		players: req.players,
		query: Some(req.query),

		user_id: req.user_id.map(Into::into),
		verification_data_json: None,
		bypass_verification: false,
		tags: HashMap::new(),
		dynamic_max_players: Some(dynamic_max_players),

		debug: None,
	})
	.await
	.unwrap()
}
