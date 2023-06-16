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

	let member_res = op!([ctx] party_member_list {
		party_ids: vec![party_id.into()],
	})
	.await
	.unwrap();
	let party = member_res.parties.first().unwrap();
	assert_eq!(2, party.user_ids.len());
	let user_ids = party
		.user_ids
		.iter()
		.map(|x| x.as_uuid())
		.collect::<Vec<_>>();
	assert!(user_ids.contains(&user_a_id));
	assert!(user_ids.contains(&user_b_id));
}
