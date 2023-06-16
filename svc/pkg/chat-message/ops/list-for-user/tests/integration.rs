use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

/// Random timestamp to execute tests from.
const NOW: i64 = 1658880140904;

const TEST_MESSAGE_COUNT: usize = 25;

#[worker_test]
async fn basic(ctx: TestCtx) {
	let sender_user_id = Uuid::new_v4();

	let mut thread_ids = Vec::new();
	for _ in 0..3 {
		let res = op!([ctx] chat_thread_get_or_create_for_topic {
			topic: Some(backend::chat::Topic {
				kind: Some(backend::chat::topic::Kind::Team(backend::chat::topic::Team {
					team_id: Some(Uuid::new_v4().into()),
				})),
			}),
		})
		.await
		.unwrap();
		thread_ids.push(res.thread_id.unwrap().as_uuid());
	}

	let chat_message_ids = thread_ids
		.iter()
		.flat_map(|thread_id| {
			(0..TEST_MESSAGE_COUNT)
				.map(|_| (Uuid::new_v4(), *thread_id))
				.collect::<Vec<(_, _)>>()
		})
		.collect::<Vec<(Uuid, Uuid)>>();

	let mut futs = Vec::new();
	for (i, (chat_message_id, thread_id)) in chat_message_ids.iter().enumerate() {
		let ctx = ctx.clone();

		futs.push(async move {
			msg!([ctx] chat_message::msg::create(thread_id, chat_message_id) -> chat_message::msg::create_complete {
				chat_message_id: Some((*chat_message_id).into()),
				thread_id: Some((*thread_id).into()),
				send_ts: gen_msg_ts(i),
				body: Some(backend::chat::MessageBody {
					kind: Some(backend::chat::message_body::Kind::Text(backend::chat::message_body::Text {
						sender_user_id: Some(sender_user_id.into()),
						body: "abc123".to_owned(),
					}))
				}),
			})
			.await
		});
	}
	futures_util::future::try_join_all(futs).await.unwrap();

	// Query before
	{
		let res = op!([ctx] chat_message_list_for_user {
			user_id: Some(sender_user_id.into()),
			// Choose timestamp long beyond the last sent message
			ts: gen_msg_ts(TEST_MESSAGE_COUNT) + util::duration::days(30),
			count: 99999,
			query_direction: chat_message::list_for_user::request::QueryDirection::Before as i32,
		})
		.await
		.unwrap();
		assert_eq!(
			TEST_MESSAGE_COUNT * 3,
			res.messages.len(),
			"invalid message count before"
		);
	}

	// Query after
	{
		let res = op!([ctx] chat_message_list_for_user {
			user_id: Some(sender_user_id.into()),
			ts: gen_msg_ts(0) - util::duration::days(30),
			count: 99999,
			query_direction: chat_message::list_for_user::request::QueryDirection::After as i32,
		})
		.await
		.unwrap();
		assert_eq!(
			TEST_MESSAGE_COUNT * 3,
			res.messages.len(),
			"invalid message count after"
		);
	}

	// Query before and after
	{
		let query_i = (TEST_MESSAGE_COUNT * 3) / 2;
		let query_count = 4;
		let res = op!([ctx] chat_message_list_for_user {
			user_id: Some(sender_user_id.into()),
			ts: gen_msg_ts(query_i),
			count: query_count,
			query_direction: chat_message::list_for_user::request::QueryDirection::BeforeAndAfter as i32,
		})
		.await
		.unwrap();
		assert_eq!(
			query_count as usize * 2,
			res.messages.len(),
			"invalid message count before & after"
		);

		let mut messages = res.messages;
		messages.sort_by_key(|msg| msg.send_ts);

		messages
			.iter()
			.zip((query_i - query_count as usize + 1)..=(query_i + query_count as usize - 1))
			.for_each(|(msg, i)| {
				assert_eq!(gen_msg_ts(i), msg.send_ts, "message sent at wrong ts");
			});
	}
}

/// Generates the timestamp that a message was sent at for a given index.
fn gen_msg_ts(i: usize) -> i64 {
	NOW + i as i64 * 100
}
