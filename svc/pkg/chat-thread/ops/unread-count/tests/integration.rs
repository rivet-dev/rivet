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
			direct_thread_id: thread_ids[2],
		}
	}
}

#[worker_test]
async fn empty(ctx: TestCtx) {
	let populate_db = PopulateDb::populate(&ctx).await;

	let user_res = op!([ctx] faker_user {}).await.unwrap();
	let user_id = user_res.user_id.unwrap();

	let res = op!([ctx] chat_thread_get_or_create_for_topic {
		topic: Some(backend::chat::Topic {
			kind: Some(backend::chat::topic::Kind::Team(backend::chat::topic::Team {
				team_id: Some(Uuid::new_v4().into()),
			})),
		}),
	})
	.await
	.unwrap();
	let thread_id = res.thread_id.unwrap().as_uuid();

	{
		let chat_message_id = Uuid::new_v4();
		msg!([ctx] chat_message::msg::create(thread_id, chat_message_id) -> chat_thread::msg::update(thread_id) {
			chat_message_id: Some(chat_message_id.into()),
			thread_id: Some(thread_id.into()),
			send_ts: util::timestamp::now(),
			body: Some(backend::chat::MessageBody {
				kind: Some(backend::chat::message_body::Kind::Text(backend::chat::message_body::Text {
					sender_user_id: Some(Uuid::new_v4().into()),
					body: "abc123".to_owned(),
				}))
			}),
		})
		.await
		.unwrap();
	}

	msg!([ctx] chat::msg::last_read_ts_set(&user_id, thread_id) -> chat::msg::last_read_ts_update {
		user_id: user_res.user_id,
		thread_id: Some(thread_id.into()),
		last_read_ts: util::timestamp::now(),
	})
	.await
	.unwrap();

	{
		let chat_message_id = Uuid::new_v4();
		msg!([ctx] chat_message::msg::create(thread_id, chat_message_id) -> chat_thread::msg::update(thread_id) {
			chat_message_id: Some(chat_message_id.into()),
			thread_id: Some(thread_id.into()),
			send_ts: util::timestamp::now(),
			body: Some(backend::chat::MessageBody {
				kind: Some(backend::chat::message_body::Kind::Text(backend::chat::message_body::Text {
					sender_user_id: Some(Uuid::new_v4().into()),
					body: "abc123".to_owned(),
				}))
			}),
		})
		.await
		.unwrap();
	}

	let res = op!([ctx] chat_thread_unread_count {
		user_id: user_res.user_id,
		thread_ids: vec![thread_id.into()],
		read_ts_threads: Vec::new(),
	})
	.await
	.unwrap();

	let unread_count = res.threads.first().unwrap().unread_count;
	tracing::info!(?unread_count);
	assert_eq!(unread_count, 1, "invalid unread count");
}
