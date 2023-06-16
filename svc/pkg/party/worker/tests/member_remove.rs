use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use redis::AsyncCommands;

#[worker_test]
async fn basic(ctx: TestCtx) {
	let mut redis = ctx.redis_party().await.unwrap();

	let (party_id, leader_user_id) = create_party(&ctx).await;
	let user_id = create_party_member(&ctx, party_id).await;

	assert!(redis
		.exists::<_, bool>(util_party::key::party_member_config(user_id))
		.await
		.unwrap());

	msg!([ctx] party::msg::member_remove(party_id, user_id) -> party::msg::member_remove_complete {
		party_id: Some(party_id.into()),
		user_id: Some(user_id.into()),
		..Default::default()
	})
	.await
	.unwrap();

	assert!(
		!redis
			.exists::<_, bool>(util_party::key::party_member_config(user_id))
			.await
			.unwrap(),
		"party member not removed"
	);
}

#[worker_test]
async fn leader_transfer(ctx: TestCtx) {
	let mut redis = ctx.redis_party().await.unwrap();

	let (party_id, leader_user_id) = create_party(&ctx).await;
	let user_id = create_party_member(&ctx, party_id).await;

	assert_eq!(
		leader_user_id.to_string(),
		redis::cmd("JSON.RESP")
			.arg(util_party::key::party_config(party_id))
			.arg("leader_user_id")
			.query_async::<_, String>(&mut redis)
			.await
			.unwrap()
	);

	// Remove the leader, will transfer to other user
	let mut leader_set_sub = subscribe!([ctx] party::msg::leader_set(party_id))
		.await
		.unwrap();
	msg!([ctx] party::msg::member_remove(party_id, leader_user_id) -> party::msg::member_remove_complete {
			party_id: Some(party_id.into()),
			user_id: Some(leader_user_id.into()),
			..Default::default()
		})
		.await
		.unwrap();
	leader_set_sub.next().await.unwrap();

	assert_eq!(
		user_id.to_string(),
		redis::cmd("JSON.RESP")
			.arg(util_party::key::party_config(party_id))
			.arg("leader_user_id")
			.query_async::<_, String>(&mut redis)
			.await
			.unwrap(),
		"party leader not transferred"
	);
}

#[worker_test]
async fn destroy_party(ctx: TestCtx) {
	let redis = ctx.redis_party().await.unwrap();

	let (party_id, leader_user_id) = create_party(&ctx).await;
	let user_id = create_party_member(&ctx, party_id).await;

	let mut party_destroy_sub = subscribe!([ctx] party::msg::destroy(party_id))
		.await
		.unwrap();
	msg!([ctx] party::msg::member_remove(party_id, leader_user_id) -> party::msg::member_remove_complete {
			party_id: Some(party_id.into()),
			user_id: Some(leader_user_id.into()),
			..Default::default()
		})
		.await
		.unwrap();
	msg!([ctx] party::msg::member_remove(party_id, user_id) -> party::msg::member_remove_complete {
		party_id: Some(party_id.into()),
		user_id: Some(user_id.into()),
		..Default::default()
	})
	.await
	.unwrap();
	party_destroy_sub.next().await.unwrap();
}

async fn create_party(ctx: &TestCtx) -> (Uuid, Uuid) {
	let party_id = Uuid::new_v4();
	let leader_user_id = Uuid::new_v4();
	msg!([ctx] party::msg::create(party_id) -> party::msg::create_complete {
		party_id: Some(party_id.into()),
		leader_user_id: Some(leader_user_id.into()),
		party_size: 4,
		..Default::default()
	})
	.await
	.unwrap();

	(party_id, leader_user_id)
}

async fn create_party_member(ctx: &TestCtx, party_id: Uuid) -> Uuid {
	let user_id = Uuid::new_v4();
	msg!([ctx] party::msg::member_create(party_id, user_id) -> party::msg::member_create_complete {
		party_id: Some(party_id.into()),
		user_id: Some(user_id.into()),
		..Default::default()
	})
	.await
	.unwrap();

	user_id
}
