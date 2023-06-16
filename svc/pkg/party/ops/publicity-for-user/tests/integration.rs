use chirp_worker::prelude::*;
use proto::backend::{party::party::PublicityLevel, pkg::*};

#[worker_test]
async fn public_join(ctx: TestCtx) {
	let (party_id, user_a) = create_party(
		&ctx,
		party::msg::create::message::Publicity {
			public: Some(PublicityLevel::Join as i32),
			friends: Some(PublicityLevel::None as i32),
			teams: Some(PublicityLevel::None as i32),
		},
	)
	.await;
	let user_b = create_party_member(&ctx, party_id).await;
	let user_c = create_party_member(&ctx, party_id).await;

	let external_user = create_user(&ctx).await;
	assert_eq!(
		PublicityLevel::Join,
		check_publicity(&ctx, party_id, external_user).await
	);
}

#[worker_test]
async fn public_view(ctx: TestCtx) {
	let (party_id, user_a) = create_party(
		&ctx,
		party::msg::create::message::Publicity {
			public: Some(PublicityLevel::View as i32),
			friends: Some(PublicityLevel::Join as i32),
			teams: Some(PublicityLevel::Join as i32),
		},
	)
	.await;
	let user_b = create_party_member(&ctx, party_id).await;
	let user_c = create_party_member(&ctx, party_id).await;

	let external_user = create_user(&ctx).await;
	assert_eq!(
		PublicityLevel::View,
		check_publicity(&ctx, party_id, external_user).await
	);
}

#[worker_test]
async fn public_none(ctx: TestCtx) {
	let (party_id, user_a) = create_party(
		&ctx,
		party::msg::create::message::Publicity {
			public: Some(PublicityLevel::None as i32),
			friends: Some(PublicityLevel::View as i32),
			teams: Some(PublicityLevel::Join as i32),
		},
	)
	.await;
	let user_b = create_party_member(&ctx, party_id).await;
	let user_c = create_party_member(&ctx, party_id).await;

	let external_user = create_user(&ctx).await;
	assert_eq!(
		PublicityLevel::None,
		check_publicity(&ctx, party_id, external_user).await
	);
}

#[worker_test]
async fn team_member(ctx: TestCtx) {
	let (party_id, user_a) = create_party(
		&ctx,
		party::msg::create::message::Publicity {
			public: Some(PublicityLevel::None as i32),
			friends: Some(PublicityLevel::Join as i32),
			teams: Some(PublicityLevel::View as i32),
		},
	)
	.await;
	let user_b = create_party_member(&ctx, party_id).await;

	let external_user = create_user(&ctx).await;

	// Create team
	{
		let team_id = Uuid::new_v4();
		op!([ctx] faker_team {
			team_id: Some(team_id.into()),
			..Default::default()
		})
		.await
		.unwrap();

		msg!([ctx] team::msg::member_create(team_id, user_b) -> team::msg::member_create_complete {
			team_id: Some(team_id.into()),
			user_id: Some(user_b.into()),
			invitation: None,
		})
		.await
		.unwrap();
		msg!([ctx] team::msg::member_create(team_id, external_user) -> team::msg::member_create_complete {
		team_id: Some(team_id.into()),
		user_id: Some(external_user.into()),
		invitation: None,
	})
	.await
	.unwrap();
	}

	assert_eq!(
		PublicityLevel::View,
		check_publicity(&ctx, party_id, external_user).await
	);
}

#[worker_test]
async fn user_follow(ctx: TestCtx) {
	let (party_id, user_a) = create_party(
		&ctx,
		party::msg::create::message::Publicity {
			public: Some(PublicityLevel::None as i32),
			friends: Some(PublicityLevel::View as i32),
			teams: Some(PublicityLevel::Join as i32),
		},
	)
	.await;
	let user_b = create_party_member(&ctx, party_id).await;

	let external_user = create_user(&ctx).await;

	// Create follow
	op!([ctx] user_follow_toggle {
		follower_user_id: Some(external_user.into()),
		following_user_id: Some(user_b.into()),
		active: true,
	})
	.await
	.unwrap();
	op!([ctx] user_follow_toggle {
		follower_user_id: Some(user_b.into()),
		following_user_id: Some(external_user.into()),
		active: true,
	})
	.await
	.unwrap();

	assert_eq!(
		PublicityLevel::View,
		check_publicity(&ctx, party_id, external_user).await
	);
}

#[worker_test]
async fn conflicting_team_user(ctx: TestCtx) {
	let (party_id, user_a) = create_party(
		&ctx,
		party::msg::create::message::Publicity {
			public: Some(PublicityLevel::None as i32),
			friends: Some(PublicityLevel::View as i32),
			teams: Some(PublicityLevel::Join as i32),
		},
	)
	.await;
	let user_b = create_party_member(&ctx, party_id).await;

	let external_user = create_user(&ctx).await;

	// Create team
	{
		let team_id = Uuid::new_v4();
		op!([ctx] faker_team {
			team_id: Some(team_id.into()),
			..Default::default()
		})
		.await
		.unwrap();

		msg!([ctx] team::msg::member_create(team_id, user_b) -> team::msg::member_create_complete {
			team_id: Some(team_id.into()),
			user_id: Some(user_b.into()),
			invitation: None,
		})
		.await
		.unwrap();
		msg!([ctx] team::msg::member_create(team_id, external_user) -> team::msg::member_create_complete {
			team_id: Some(team_id.into()),
			user_id: Some(external_user.into()),
			invitation: None,
		})
		.await
		.unwrap();
	}

	// Create follow
	op!([ctx] user_follow_toggle {
		follower_user_id: Some(external_user.into()),
		following_user_id: Some(user_b.into()),
		active: true,
	})
	.await
	.unwrap();
	op!([ctx] user_follow_toggle {
		follower_user_id: Some(user_b.into()),
		following_user_id: Some(external_user.into()),
		active: true,
	})
	.await
	.unwrap();

	assert_eq!(
		PublicityLevel::Join,
		check_publicity(&ctx, party_id, external_user).await
	);
}

async fn create_party(
	ctx: &TestCtx,
	publicity: party::msg::create::message::Publicity,
) -> (Uuid, Uuid) {
	let leader_user_id = create_user(ctx).await;
	let party_id = Uuid::new_v4();
	msg!([ctx] party::msg::create(party_id) -> party::msg::create_complete {
		party_id: Some(party_id.into()),
		leader_user_id: Some(leader_user_id.into()),
		party_size: 4,
		publicity: Some(publicity),
		..Default::default()
	})
	.await
	.unwrap();

	(party_id, leader_user_id)
}

async fn create_user(ctx: &TestCtx) -> Uuid {
	let user_res = op!([ctx] faker_user {
		..Default::default()
	})
	.await
	.unwrap();
	let user_id = user_res.user_id.as_ref().unwrap().as_uuid();

	user_id
}

async fn create_party_member(ctx: &TestCtx, party_id: Uuid) -> Uuid {
	let user_id = create_user(ctx).await;
	msg!([ctx] party::msg::member_create(party_id, user_id) -> party::msg::member_create_complete {
		party_id: Some(party_id.into()),
		user_id: Some(user_id.into()),
		..Default::default()
	})
	.await
	.unwrap();

	user_id
}

async fn check_publicity(ctx: &TestCtx, party_id: Uuid, user_id: Uuid) -> PublicityLevel {
	let res = op!([ctx] party_publicity_for_user {
		user_id: Some(user_id.into()),
		party_ids: vec![party_id.into()],
	})
	.await
	.unwrap();
	let party = res.parties.first().unwrap();

	PublicityLevel::from_i32(party.publicity).unwrap()
}
