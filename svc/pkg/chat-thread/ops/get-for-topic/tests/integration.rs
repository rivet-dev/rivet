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
		let (user_a_id, user_b_id) = util::sort::id_pair(Uuid::new_v4(), Uuid::new_v4());

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
			direct_thread_id: thread_ids[2],
		}
	}
}

#[worker_test]
async fn team(ctx: TestCtx) {
	let populate_db = PopulateDb::populate(&ctx).await;

	// Fetch the thread
	let res = op!([ctx] chat_thread_get_for_topic {
		topics: vec![backend::chat::Topic {
			kind: Some(backend::chat::topic::Kind::Team(
				backend::chat::topic::Team {
					team_id: Some(populate_db.team_id.into()),
				},
			)),
		}],
	})
	.await
	.unwrap();

	// Validate thread
	assert!(!res.threads.is_empty(), "missing team thread");
	if let backend::chat::topic::Kind::Team(t) = res
		.threads
		.first()
		.unwrap()
		.topic
		.clone()
		.unwrap()
		.kind
		.unwrap()
	{
		assert_eq!(populate_db.team_id, t.team_id.unwrap().as_uuid());
	} else {
		panic!()
	}
}

#[worker_test]
async fn direct(ctx: TestCtx) {
	let populate_db = PopulateDb::populate(&ctx).await;

	// Fetch the thread
	let res = op!([ctx] chat_thread_get_for_topic {
		topics: vec![
			backend::chat::Topic {
				kind: Some(backend::chat::topic::Kind::Direct(
					backend::chat::topic::Direct {
						user_a_id: Some(populate_db.user_a_id.into()),
						user_b_id: Some(populate_db.user_b_id.into()),
					},
				)),
			},
		],
	})
	.await
	.unwrap();

	// Validate thread
	assert!(!res.threads.is_empty(), "missing direct thread");
	if let backend::chat::topic::Kind::Direct(t) = res
		.threads
		.first()
		.unwrap()
		.topic
		.clone()
		.unwrap()
		.kind
		.unwrap()
	{
		assert_eq!(populate_db.user_a_id, t.user_a_id.unwrap().as_uuid());
		assert_eq!(populate_db.user_b_id, t.user_b_id.unwrap().as_uuid());
	} else {
		panic!()
	}
}
