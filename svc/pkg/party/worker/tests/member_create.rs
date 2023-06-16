use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn basic(ctx: TestCtx) {
	let (party_id, leader_user_id) = create_party(&ctx, 4).await;
	let user_id_a = add_member(&ctx, party_id).await.unwrap();
	let user_id_b = add_member(&ctx, party_id).await.unwrap();

	let redis = ctx.redis_party().await.unwrap();
	let party_members = op!([ctx] party_member_list {
		party_ids: vec![party_id.into()],
	})
	.await
	.unwrap();
	assert_eq!(
		3,
		party_members.parties.first().unwrap().user_ids.len(),
		"wrong member count"
	);
}

#[worker_test]
async fn party_does_not_exist(ctx: TestCtx) {
	assert_eq!(
		Err(party::msg::member_create_fail::ErrorCode::PartyDoesNotExist),
		add_member(&ctx, Uuid::new_v4()).await,
		"party should be full"
	);
}

#[worker_test]
async fn party_full(ctx: TestCtx) {
	let (party_id, leader_user_id) = create_party(&ctx, 4).await;
	let user_id_a = add_member(&ctx, party_id).await.unwrap();
	let user_id_b = add_member(&ctx, party_id).await.unwrap();
	let user_id_c = add_member(&ctx, party_id).await.unwrap();
	assert_eq!(
		Err(party::msg::member_create_fail::ErrorCode::PartyFull),
		add_member(&ctx, party_id).await,
		"party should be full"
	);
}

#[worker_test]
async fn already_in_party(ctx: TestCtx) {
	let (party_id, leader_user_id) = create_party(&ctx, 4).await;
	let user_id_a = add_member(&ctx, party_id).await.unwrap();
	assert_eq!(
		Err(party::msg::member_create_fail::ErrorCode::AlreadyInParty),
		add_member_with_user_id(&ctx, party_id, user_id_a).await,
	);
}

async fn create_party(ctx: &TestCtx, party_size: u32) -> (Uuid, Uuid) {
	let party_id = Uuid::new_v4();
	let leader_user_id = Uuid::new_v4();
	msg!([ctx] party::msg::create(party_id) -> party::msg::create_complete {
		party_id: Some(party_id.into()),
		leader_user_id: Some(leader_user_id.into()),
		party_size: party_size,
		..Default::default()
	})
	.await
	.unwrap();

	(party_id, leader_user_id)
}

async fn add_member(
	ctx: &TestCtx,
	party_id: Uuid,
) -> Result<Uuid, party::msg::member_create_fail::ErrorCode> {
	let user_id = Uuid::new_v4();
	add_member_with_user_id(ctx, party_id, user_id)
		.await
		.map(|_| user_id)
}

async fn add_member_with_user_id(
	ctx: &TestCtx,
	party_id: Uuid,
	user_id: Uuid,
) -> Result<(), party::msg::member_create_fail::ErrorCode> {
	let res = msg!([ctx] party::msg::member_create(party_id, user_id) -> Result<party::msg::member_create_complete, party::msg::member_create_fail> {
		party_id: Some(party_id.into()),
		user_id: Some(user_id.into()),
		..Default::default()
	})
	.await
	.unwrap();
	match res {
		Ok(_) => {
			tracing::info!("party member created");
			Ok(())
		}
		Err(msg) => {
			tracing::info!("party member failed to create");
			Err(party::msg::member_create_fail::ErrorCode::from_i32(msg.error_code).unwrap())
		}
	}
}
