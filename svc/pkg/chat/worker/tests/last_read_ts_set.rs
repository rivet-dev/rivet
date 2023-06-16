use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn empty(ctx: TestCtx) {
	let user_res = op!([ctx] faker_user {}).await.unwrap();

	let res = op!([ctx] chat_thread_get_or_create_for_topic {
		topic: Some(backend::chat::Topic {
			kind: Some(backend::chat::topic::Kind::Direct(
				backend::chat::topic::Direct {
					user_a_id: Some(Uuid::new_v4().into()),
					user_b_id: Some(Uuid::new_v4().into()),
				},
			)),
		}),
	})
	.await
	.unwrap();
	let thread_id = res.thread_id.unwrap().as_uuid();

	let last_read_ts = util::timestamp::now();

	msg!([ctx] chat::msg::last_read_ts_set(user_res.user_id.as_ref().unwrap(), thread_id) -> chat::msg::last_read_ts_update {
		user_id: user_res.user_id,
		thread_id: Some(thread_id.into()),
		last_read_ts: last_read_ts,
	})
	.await
	.unwrap();

	let res = op!([ctx] chat_last_read_ts_get {
		user_id: user_res.user_id,
		thread_ids: vec![thread_id.into()]
	})
	.await
	.unwrap();

	assert_eq!(res.threads.first().unwrap().last_read_ts, last_read_ts);
}
