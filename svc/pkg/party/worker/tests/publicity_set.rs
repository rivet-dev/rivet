use chirp_worker::prelude::*;
use proto::backend::{party::party::PublicityLevel, pkg::*};

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

	msg!([ctx] party::msg::publicity_set(party_id) -> party::msg::update {
		party_id: Some(party_id.into()),
		public: Some(PublicityLevel::None as i32),
		friends: Some(PublicityLevel::View as i32),
		teams: Some(PublicityLevel::Join as i32),
	})
	.await
	.unwrap();

	let get_res = op!([ctx] party_get {
		party_ids: vec![party_id.into()],
	})
	.await
	.unwrap();
	let party = get_res.parties.first().unwrap();
	let publicity = party.publicity.as_ref().unwrap();
	assert_eq!(
		PublicityLevel::None,
		PublicityLevel::from_i32(publicity.public).unwrap()
	);
	assert_eq!(
		PublicityLevel::View,
		PublicityLevel::from_i32(publicity.friends).unwrap()
	);
	assert_eq!(
		PublicityLevel::Join,
		PublicityLevel::from_i32(publicity.teams).unwrap()
	);
}

#[worker_test]
async fn defaults(ctx: TestCtx) {
	let party_id = Uuid::new_v4();
	msg!([ctx] party::msg::create(party_id) -> party::msg::create_complete {
		party_id: Some(party_id.into()),
		leader_user_id: Some(Uuid::new_v4().into()),
		party_size: 4,
		..Default::default()
	})
	.await
	.unwrap();

	msg!([ctx] party::msg::publicity_set(party_id) -> party::msg::update {
		party_id: Some(party_id.into()),
		public: Some(PublicityLevel::None as i32),
		friends: None,
		teams: None,
	})
	.await
	.unwrap();

	let get_res = op!([ctx] party_get {
		party_ids: vec![party_id.into()],
	})
	.await
	.unwrap();
	let party = get_res.parties.first().unwrap();
	let publicity = party.publicity.as_ref().unwrap();
	assert_eq!(
		PublicityLevel::None,
		PublicityLevel::from_i32(publicity.public).unwrap()
	);
	assert_eq!(
		PublicityLevel::Join,
		PublicityLevel::from_i32(publicity.friends).unwrap()
	);
	assert_eq!(
		PublicityLevel::View,
		PublicityLevel::from_i32(publicity.teams).unwrap()
	);
}
