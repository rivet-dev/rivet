use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn basic(ctx: TestCtx) {
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

	let lookup_res = op!([ctx] party_invite_alias_lookup {
		namespace_id: Some(namespace_id.into()),
		alias: alias.clone(),
	})
	.await
	.unwrap();
	let lookup_invite_id = lookup_res
		.invite_id
		.as_ref()
		.expect("invite should be found")
		.as_uuid();
	assert_eq!(invite_id, lookup_invite_id, "invite id does not match");
}

#[worker_test]
async fn not_found(ctx: TestCtx) {
	let lookup_res = op!([ctx] party_invite_alias_lookup {
		namespace_id: Some(Uuid::new_v4().into()),
		alias: Uuid::new_v4().to_string(),
	})
	.await
	.unwrap();
	assert!(lookup_res.invite_id.is_none(), "invite should not be found");
}

#[worker_test]
async fn not_found_in_different_ns(ctx: TestCtx) {
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

	let lookup_res = op!([ctx] party_invite_alias_lookup {
		namespace_id: Some(Uuid::new_v4().into()),
		alias: alias.clone(),
	})
	.await
	.unwrap();
	assert!(lookup_res.invite_id.is_none(), "invite should not be found");
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
