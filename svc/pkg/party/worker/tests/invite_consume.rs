use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn global(ctx: TestCtx) {
	let party_id = create_party(&ctx).await;

	let invite_id = Uuid::new_v4();
	msg!([ctx] party::msg::invite_create(party_id, invite_id) -> party::msg::invite_create_complete {
		party_id: Some(party_id.into()),
		invite_id: Some(invite_id.into()),
		..Default::default()
	})
	.await
	.unwrap();

	let query_id = Uuid::new_v4();
	let consume_res = msg!([ctx] party::msg::invite_consume(query_id) -> Result<party::msg::invite_consume_complete, party::msg::invite_consume_fail> {
		query_id: Some(query_id.into()),
		invite_id: Some(invite_id.into()),
	}).await.unwrap().unwrap();
	assert_eq!(party_id, consume_res.party_id.as_ref().unwrap().as_uuid());
}

#[worker_test]
async fn not_found(ctx: TestCtx) {
	let query_id = Uuid::new_v4();
	let consume_res = msg!([ctx] party::msg::invite_consume(query_id) -> Result<party::msg::invite_consume_complete, party::msg::invite_consume_fail> {
		query_id: Some(query_id.into()),
		invite_id: Some(Uuid::new_v4().into()),
	}).await.unwrap().unwrap_err();
	assert_eq!(
		party::msg::invite_consume_fail::ErrorCode::InviteNotFound as i32,
		consume_res.error_code
	);
}

async fn create_party(ctx: &TestCtx) -> Uuid {
	let party_id = Uuid::new_v4();
	msg!([ctx] party::msg::create(party_id) -> party::msg::create_complete {
		party_id: Some(party_id.into()),
		leader_user_id: Some(Uuid::new_v4().into()),
		party_size: 4,
		..Default::default()
	})
	.await
	.unwrap();

	party_id
}

// TODO: Test that global and namespaced codes don't interfere
