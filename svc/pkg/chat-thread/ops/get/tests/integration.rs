use chirp_worker::prelude::*;
use proto::backend;

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
	async fn populate(ctx: &TestCtx) -> Self {
		let team_id = Uuid::new_v4();
		let user_a_id = Uuid::new_v4();
		let user_b_id = Uuid::new_v4();
		let (user_a_id, user_b_id) = util::sort::id_pair(user_a_id, user_b_id);

		// Create threads
		let topics = vec![
			backend::chat::topic::Kind::Team(backend::chat::topic::Team {
				team_id: Some(team_id.into()),
			}),
			backend::chat::topic::Kind::Direct(backend::chat::topic::Direct {
				user_a_id: Some(user_a_id.into()),
				user_b_id: Some(user_b_id.into()),
			}),
		];
		let mut thread_ids = Vec::new();
		for kind in &topics {
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
			direct_thread_id: thread_ids[1],
		}
	}
}

#[worker_test]
async fn empty(ctx: TestCtx) {
	let populate_db = PopulateDb::populate(&ctx).await;

	// Fetch the threads
	let res = op!([ctx] chat_thread_get {
		thread_ids: vec![
			// Existing
			populate_db.team_thread_id.into(),
			// Nonexistent
			Uuid::new_v4().into(),
		],
	})
	.await
	.unwrap();

	// Validate the threads
	assert_eq!(2, res.threads.len(), "wrong thread count");
	let team_thread = res
		.threads
		.iter()
		.filter_map(|t| {
			if let Some(backend::chat::topic::Kind::Team(t)) = &t.topic.as_ref().unwrap().kind {
				Some(t)
			} else {
				None
			}
		})
		.next()
		.expect("missing group thread");
	assert_eq!(populate_db.team_id, team_thread.team_id.unwrap().as_uuid());
}
