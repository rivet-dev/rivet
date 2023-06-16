use std::collections::HashSet;

use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn basic(ctx: TestCtx) {
	let redis = ctx.redis_party().await.unwrap();

	let (party_id, leader_user_id) = create_party(&ctx).await;
	let user_id_a = create_party_member(&ctx, party_id).await;
	let user_id_b = create_party_member(&ctx, party_id).await;
	let mut all_party_members = vec![leader_user_id, user_id_a, user_id_b]
		.into_iter()
		.collect::<HashSet<Uuid>>();

	let mut member_remove_sub = subscribe!([ctx] party::msg::member_remove(party_id, "*"))
		.await
		.unwrap();
	msg!([ctx] party::msg::destroy(party_id) -> party::msg::destroy_complete {
		party_id: Some(party_id.into()),
	})
	.await
	.unwrap();

	while !all_party_members.is_empty() {
		let remove_msg = member_remove_sub.next().await.unwrap();
		let removed_user_id = remove_msg.user_id.as_ref().unwrap().as_uuid();
		all_party_members.remove(&removed_user_id);
		tracing::info!(?removed_user_id, "removed user");
	}
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
