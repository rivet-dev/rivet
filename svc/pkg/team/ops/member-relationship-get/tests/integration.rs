use std::collections::HashSet;

use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn basic(ctx: TestCtx) {
	let user_a = Uuid::new_v4();
	let user_b = Uuid::new_v4();
	let user_c = Uuid::new_v4();

	let team_a = Uuid::new_v4();
	let team_b = Uuid::new_v4();
	let team_c = Uuid::new_v4();

	let all_user_ids = [user_a, user_b, user_c];
	let all_team_ids = vec![team_a, team_b, team_c];

	// Create fake teams
	for &team_id in &all_team_ids {
		op!([ctx] faker_team {
			team_id: Some(team_id.into()),
			..Default::default()
		})
		.await
		.unwrap();
	}

	// Create memberships
	let memberships = [
		(user_a, team_a),
		(user_a, team_b),
		(user_b, team_c),
		(user_c, team_a),
		(user_c, team_b),
	];
	for &(user_id, team_id) in memberships.iter() {
		msg!([ctx] team::msg::member_create(team_id, user_id) -> team::msg::member_create_complete {
			team_id: Some(team_id.into()),
			user_id: Some(user_id.into()),
			invitation: None,
		})
		.await
		.unwrap();
	}

	let tests = all_user_ids
		.iter()
		.flat_map(|&this_user| {
			all_user_ids
				.iter()
				.map(move |&other_user| util::sort::id_pair(this_user, other_user))
		})
		.collect::<HashSet<_>>();
	let test_users = tests
		.iter()
		.map(
			|&(this_user, other_user)| team::member_relationship_get::request::User {
				this_user_id: Some(this_user.into()),
				other_user_id: Some(other_user.into()),
			},
		)
		.collect();
	let res = op!([ctx] team_member_relationship_get {
		users: test_users,
	})
	.await
	.unwrap();

	assert_eq!(tests.len(), res.users.len());

	res.users.iter().for_each(|relationship| {
		let this_user_id = relationship.this_user_id.unwrap().as_uuid();
		let other_user_id = relationship.other_user_id.unwrap().as_uuid();

		let res_shared_team_ids = relationship
			.shared_team_ids
			.iter()
			.map(|x| x.as_uuid())
			.collect::<HashSet<Uuid>>();

		let this_team_ids = memberships
			.iter()
			.filter(|m| m.0 == this_user_id)
			.map(|m| m.1)
			.collect::<HashSet<Uuid>>();
		let shared_team_ids = memberships
			.iter()
			.filter(|m| m.0 == other_user_id)
			.filter(|m| this_team_ids.contains(&m.1))
			.map(|m| m.1)
			.collect::<HashSet<Uuid>>();

		assert_eq!(shared_team_ids, res_shared_team_ids, "bad shared team ids");
	});
}
