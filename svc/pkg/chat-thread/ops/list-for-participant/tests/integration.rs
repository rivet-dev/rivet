use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

/// Populates the database without populating dependent databases. This will not work for some
/// cases which query other services, such as for participants.
struct PopulateDb {
	team_id: Uuid,
	team_thread_id: Uuid,

	user_a_id: Uuid,
	user_b_id: Uuid,
	direct_thread_id: Uuid,
}

impl PopulateDb {
	async fn populate(
		ctx: &TestCtx,
		team_id: Uuid,
		user_a_id: Uuid,
		user_b_id: Uuid,
	) -> PopulateDb {
		let (user_a_id, user_b_id) = util::sort::id_pair(user_a_id, user_b_id);

		// Create threads
		let threads = vec![
			backend::chat::topic::Kind::Team(backend::chat::topic::Team {
				team_id: Some(team_id.into()),
			}),
			backend::chat::topic::Kind::Direct(backend::chat::topic::Direct {
				user_a_id: Some(user_a_id.into()),
				user_b_id: Some(user_b_id.into()),
			}),
		];
		let mut thread_ids = Vec::new();
		for kind in &threads {
			let res = op!([ctx] chat_thread_get_or_create_for_topic {
				topic: Some(backend::chat::Topic {
					kind: Some(kind.clone()),
				}),
			})
			.await
			.unwrap();
			thread_ids.push(res.thread_id.unwrap().as_uuid());
		}

		PopulateDb {
			team_id,
			team_thread_id: thread_ids[0],

			user_a_id,
			user_b_id,
			direct_thread_id: thread_ids[2],
		}
	}

	fn set_team(&mut self, team_id: common::Uuid) {
		self.team_id = team_id.as_uuid();
	}

	fn set_users(&mut self, user_a_id: common::Uuid, user_b_id: common::Uuid) {
		let (user_a_id, user_b_id) = util::sort::id_pair(user_a_id.as_uuid(), user_b_id.as_uuid());

		self.user_a_id = user_a_id;
		self.user_b_id = user_b_id;
	}
}

#[worker_test]
async fn empty(ctx: TestCtx) {
	let user_a = op!([ctx] faker_user {}).await.unwrap();
	let user_b = op!([ctx] faker_user {}).await.unwrap();

	// Team
	let raw_team_id = Uuid::new_v4();
	let team_id = Into::<common::Uuid>::into(raw_team_id);
	msg!([ctx] team::msg::create(raw_team_id) -> Result<team::msg::create_complete, team::msg::create_fail> {
			team_id: Some(team_id),
			display_name: util::faker::display_name(),
			owner_user_id: user_a.user_id,
		})
		.await
		.unwrap().unwrap();

	{
		let user_id = user_a.user_id.as_ref().unwrap().as_uuid();

		msg!([ctx] team::msg::member_create(raw_team_id, user_id) -> team::msg::member_create_complete {
				team_id: Some(team_id),
				user_id: user_a.user_id,
				invitation: None,
			})
			.await.unwrap();
	}

	// TODO:
	// // Party
	// op!([ctx] party_create {
	// 	party_id: Some(populate_db.party_id.into()),
	// 	activity: Some(backend::party::Activity {
	// 		kind: None,
	// 	}),
	// })
	// .await
	// .unwrap();

	// op!([ctx] user_presence_party_set {
	// 	user_id: user_a.user_id,
	// 	party_id: Some(populate_db.party_id.into()),
	// })
	// .await
	// .unwrap();

	let populate_db = PopulateDb::populate(
		&ctx,
		raw_team_id,
		Uuid::new_v4(),
		user_a.user_id.unwrap().as_uuid(),
		user_b.user_id.unwrap().as_uuid(),
	)
	.await;

	// Fetch the thread
	let res = op!([ctx] chat_thread_list_for_participant {
		user_id: user_a.user_id,
	})
	.await
	.unwrap();

	// Validate the threads
	assert_eq!(2, res.threads.len(), "wrong thread count");
	for thread in &res.threads {
		let thread_kind = thread.topic.clone().unwrap().kind.unwrap();
		match thread_kind {
			backend::chat::topic::Kind::Team(team) => {
				assert_eq!(populate_db.team_id, team.team_id.unwrap().as_uuid());
			}
			backend::chat::topic::Kind::Direct(direct) => {
				let (user_a_id, user_b_id) = util::sort::id_pair(
					direct.user_a_id.unwrap().as_uuid(),
					direct.user_b_id.unwrap().as_uuid(),
				);
				assert_eq!(user_a_id, populate_db.user_a_id);
				assert_eq!(user_b_id, populate_db.user_b_id);
			}
		}
	}
}
