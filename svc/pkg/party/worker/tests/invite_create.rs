use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn basic(ctx: TestCtx) {
	let party_id = create_party(&ctx).await;

	let invite_id = Uuid::new_v4();
	msg!([ctx] party::msg::invite_create(party_id, invite_id) -> Result<party::msg::invite_create_complete, party::msg::invite_create_fail> {
		party_id: Some(party_id.into()),
		invite_id: Some(invite_id.into()),
		..Default::default()
	})
	.await
	.unwrap().unwrap();
}

#[worker_test]
async fn does_not_exist(ctx: TestCtx) {
	let party_id = Uuid::new_v4();
	let invite_id = Uuid::new_v4();
	let create_res = msg!([ctx] party::msg::invite_create(party_id, invite_id) -> Result<party::msg::invite_create_complete, party::msg::invite_create_fail> {
		party_id: Some(party_id.into()),
		invite_id: Some(invite_id.into()),
		..Default::default()
	})
	.await
	.unwrap().unwrap_err();
	assert_eq!(
		party::msg::invite_create_fail::ErrorCode::PartyDoesNotExist as i32,
		create_res.error_code
	);
}

#[worker_test]
async fn not_unique(ctx: TestCtx) {
	let party_id = create_party(&ctx).await;

	let namespace_id = Uuid::new_v4();
	let alias = Uuid::new_v4().to_string();

	let invite_id = Uuid::new_v4();
	msg!([ctx] party::msg::invite_create(party_id, invite_id) -> Result<party::msg::invite_create_complete, party::msg::invite_create_fail> {
		party_id: Some(party_id.into()),
		invite_id: Some(invite_id.into()),
		alias: Some(party::msg::invite_create::Alias {
			namespace_id: Some(namespace_id.into()),
			alias: alias.clone(),
		}),
		..Default::default()
	})
	.await
	.unwrap().unwrap();

	let invite_id = Uuid::new_v4();
	let create_res = msg!([ctx] party::msg::invite_create(party_id, invite_id) -> Result<party::msg::invite_create_complete, party::msg::invite_create_fail> {
		party_id: Some(party_id.into()),
		invite_id: Some(invite_id.into()),
		alias: Some(party::msg::invite_create::Alias {
			namespace_id: Some(namespace_id.into()),
			alias: alias.clone(),
		}),
		..Default::default()
	})
	.await
	.unwrap().unwrap_err();
	assert_eq!(
		party::msg::invite_create_fail::ErrorCode::AliasNotUnique as i32,
		create_res.error_code
	);
}

#[worker_test]
async fn preemptive_invite(ctx: TestCtx) {
	let party_id = Uuid::new_v4();
	let invite_id = Uuid::new_v4();
	msg!([ctx] party::msg::invite_create(party_id, invite_id) -> Result<party::msg::invite_create_complete, party::msg::invite_create_fail> {
		party_id: Some(party_id.into()),
		invite_id: Some(invite_id.into()),
		alias: Some(party::msg::invite_create::Alias {
			namespace_id: Some(Uuid::new_v4().into()),
			alias: Uuid::new_v4().to_string(),
		}),
		preemptive_party: true,
		..Default::default()
	})
	.await
	.unwrap().unwrap();
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
