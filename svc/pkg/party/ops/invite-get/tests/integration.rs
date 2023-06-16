use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn basic(ctx: TestCtx) {
	let party_id = Uuid::new_v4();
	msg!([ctx] party::msg::create(party_id) -> party::msg::create_complete {
		party_id: Some(party_id.into()),
		leader_user_id: Some(Uuid::new_v4().into()),
		party_size: 4,
		..Default::default()
	})
	.await
	.unwrap();

	let invite_id = Uuid::new_v4();
	let namespace_id = Uuid::new_v4();
	let invite_alias = Uuid::new_v4().to_string();
	msg!([ctx] party::msg::invite_create(party_id, invite_id) -> Result<party::msg::invite_create_complete, party::msg::invite_create_fail> {
		party_id: Some(party_id.into()),
		invite_id: Some(invite_id.into()),
		alias: Some(party::msg::invite_create::Alias {
			namespace_id: Some(namespace_id.into()),
			alias: invite_alias.clone(),
		}),
		..Default::default()
	})
	.await
	.unwrap().unwrap();

	let get_res = op!([ctx] party_invite_get {
		invite_ids: vec![Uuid::new_v4().into(), invite_id.into()],
	})
	.await
	.unwrap();
	assert_eq!(1, get_res.invites.len(), "wrong invite count");
	let invite = &get_res.invites[0];
	assert_eq!(party_id, invite.party_id.as_ref().unwrap().as_uuid());
	let alias = invite.alias.as_ref().expect("missing alias");
	assert_eq!(namespace_id, alias.namespace_id.as_ref().unwrap().as_uuid());
	assert_eq!(invite_alias, alias.alias);
}
