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

	let get_res = op!([ctx] party_get {
		party_ids: vec![Uuid::new_v4().into(), party_id.into(), Uuid::new_v4().into()],
	})
	.await
	.unwrap();
	assert_eq!(1, get_res.parties.len());

	let party = get_res.parties.first().unwrap();
	assert_eq!(
		party.party_id.as_ref().unwrap().as_uuid(),
		party_id,
		"wrong party id"
	);
}
