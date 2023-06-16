use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn basic(ctx: TestCtx) {
	let party_id = Uuid::new_v4();
	let user_id_a = Uuid::new_v4();
	let user_id_b = Uuid::new_v4();

	msg!([ctx] party::msg::create(party_id) -> party::msg::create_complete {
		party_id: Some(party_id.into()),
		leader_user_id: Some(user_id_a.into()),
		party_size: 4,
		..Default::default()
	})
	.await
	.unwrap();

	assert_eq!(user_id_a, get_leader(&ctx, party_id).await);

	msg!([ctx] party::msg::member_create(party_id, user_id_b) -> party::msg::member_create_complete {
		party_id: Some(party_id.into()),
		user_id: Some(user_id_b.into()),
		..Default::default()
	})
	.await
	.unwrap();

	msg!([ctx] party::msg::leader_set(party_id) -> party::msg::update {
		party_id: Some(party_id.into()),
		leader_user_id: Some(user_id_b.into()),
	})
	.await
	.unwrap();

	assert_eq!(user_id_b, get_leader(&ctx, party_id).await);
}

#[worker_test]
async fn random_leader(ctx: TestCtx) {
	let party_id = Uuid::new_v4();
	let user_id_a = Uuid::new_v4();
	let user_id_b = Uuid::new_v4();
	let user_id_c = Uuid::new_v4();

	msg!([ctx] party::msg::create(party_id) -> party::msg::create_complete {
		party_id: Some(party_id.into()),
		leader_user_id: Some(user_id_a.into()),
		party_size: 4,
		..Default::default()
	})
	.await
	.unwrap();
	msg!([ctx] party::msg::member_create(party_id, user_id_b) -> party::msg::member_create_complete {
		party_id: Some(party_id.into()),
		user_id: Some(user_id_b.into()),
		..Default::default()
	})
	.await
	.unwrap();
	msg!([ctx] party::msg::member_create(party_id, user_id_c) -> party::msg::member_create_complete {
		party_id: Some(party_id.into()),
		user_id: Some(user_id_c.into()),
		..Default::default()
	})
	.await
	.unwrap();

	// Set a new leader
	let update_sub = subscribe!([ctx] party::msg::update(party_id))
		.await
		.unwrap();
	msg!([ctx] party::msg::leader_set(party_id) {
		party_id: Some(party_id.into()),
		leader_user_id: Some(user_id_c.into()),
	})
	.await
	.unwrap();
	wait_for_leader(&ctx, update_sub, party_id, user_id_c).await;

	let update_sub = subscribe!([ctx] party::msg::update(party_id))
		.await
		.unwrap();
	msg!([ctx] party::msg::leader_set(party_id) {
		party_id: Some(party_id.into()),
		leader_user_id: None,
	})
	.await
	.unwrap();
	wait_for_leader(&ctx, update_sub, party_id, user_id_a).await;
}

#[worker_test]
async fn bad_party_member(ctx: TestCtx) {
	let party_id = Uuid::new_v4();
	let user_id_a = Uuid::new_v4();
	let user_id_b = Uuid::new_v4();

	msg!([ctx] party::msg::create(party_id) -> party::msg::create_complete {
		party_id: Some(party_id.into()),
		leader_user_id: Some(user_id_a.into()),
		party_size: 4,
		..Default::default()
	})
	.await
	.unwrap();

	msg!([ctx] party::msg::member_create(party_id, user_id_b) -> party::msg::member_create_complete {
		party_id: Some(party_id.into()),
		user_id: Some(user_id_b.into()),
		..Default::default()
	})
	.await
	.unwrap();

	// Attempt to set the leader to a nonexistent user
	msg!([ctx] party::msg::leader_set(party_id) {
		party_id: Some(party_id.into()),
		leader_user_id: Some(Uuid::new_v4().into()),
	})
	.await
	.unwrap();

	// There is no event to hook in to for this
	tokio::time::sleep(std::time::Duration::from_secs(1)).await;

	assert_eq!(
		user_id_a,
		get_leader(&ctx, party_id).await,
		"leader should not have changed"
	);
}

async fn get_leader(ctx: &TestCtx, party_id: Uuid) -> Uuid {
	let mut redis = ctx.redis_party().await.unwrap();
	let user_id = redis::cmd("JSON.RESP")
		.arg(util_party::key::party_config(party_id))
		.arg("leader_user_id")
		.query_async::<_, String>(&mut redis)
		.await
		.unwrap();
	util::uuid::parse(user_id.as_str()).unwrap()
}

async fn wait_for_leader(
	ctx: &TestCtx,
	mut party_update_sub: chirp_client::SubscriptionHandle<party::msg::update::Message>,
	party_id: Uuid,
	expected_leader_user_id: Uuid,
) {
	loop {
		party_update_sub.next().await.unwrap();

		let new_leader_user_id = get_leader(ctx, party_id).await;
		if new_leader_user_id == expected_leader_user_id {
			tracing::info!("found correct leader");
			break;
		} else {
			tracing::info!(
				?new_leader_user_id,
				?expected_leader_user_id,
				"still waiting for leader change"
			);
		}
	}
}
