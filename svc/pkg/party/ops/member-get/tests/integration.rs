use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn basic(ctx: TestCtx) {
	let party_id = Uuid::new_v4();
	let user_a_id = Uuid::new_v4();
	let user_b_id = Uuid::new_v4();

	msg!([ctx] party::msg::create(party_id) -> party::msg::create_complete {
		party_id: Some(party_id.into()),
		leader_user_id: Some(user_a_id.into()),
		party_size: 4,
		..Default::default()
	})
	.await
	.unwrap();

	msg!([ctx] party::msg::member_create(party_id, user_b_id) -> party::msg::member_create_complete {
		party_id: Some(party_id.into()),
		user_id: Some(user_b_id.into()),
		..Default::default()
	})
	.await
	.unwrap();

	let get_res = op!([ctx] party_member_get {
		user_ids: vec![Uuid::new_v4().into(), user_a_id.into(), user_b_id.into()],
	})
	.await
	.unwrap();
	assert_eq!(2, get_res.party_members.len());
}
