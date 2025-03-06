// use chirp_worker::prelude::*;
// use proto::backend::{self, pkg::*};

// /// How many logs to create total.
// const BATCH_SIZE: usize = 64;

// /// How far aprat messages are created.
// const MESSAGE_INTERVAL: i64 = 1000;

// #[worker_test]
// async fn basic(ctx: TestCtx) {
// 	// let now = 100000i64;

// 	// todo!();

// 	// let run_id = Uuid::new_v4();
// 	// let task = "main".to_string();
// 	// let stream_type = backend::job::log::StreamType::StdOut;

// 	// // Insert entries
// 	// let entries = (0..TS_COUNT)
// 	// 	.flat_map(|i| backend::job::log::LogEntry {
// 	// 		ts: now + (i as i64 * MESSAGE_INTERVAL),
// 	// 		message: format!("Hello, {i}!").into_bytes(),
// 	// 	})
// 	// 	.collect::<Vec<_>>();
// 	// msg!([ctx] job_log::msg::entries(&run_id, "stdout") {
// 	// 	run_id: Some(run_id.into()),
// 	// 	stream_type: stream_type as i32,
// 	// 	entries: entries,
// 	// })
// 	// .await
// 	// .unwrap();

// 	// tokio::time::sleep(Duration::from_secs(1)).await;

// 	// // Before ts
// 	// {
// 	// 	let res = op!([ctx] job_log_read {
// 	// 		run_id: Some(run_id.into()),
// 	// 		stream_type: stream_type as i32,
// 	// 		count: 128,
// 	// 		query: Some(job_log::read::request::Query::BeforeTs(ts: now + 5 * MESSAGE_INTERVAL)),
// 	// 	})
// 	// 	.await
// 	// 	.unwrap();
// 	// 	assert_eq!(5 + 3, res.entries.len());
// 	// 	for (i, entry) in res.entries.iter().enumerate() {
// 	// 		assert_eq!(now + i as i64 * MESSAGE_INTERVAL, entry.ts, "i: {i}");
// 	// 	}
// 	// }

// 	// // After ts
// 	// {
// 	// 	let res = op!([ctx] job_log_read {
// 	// 		run_id: Some(run_id.into()),
// 	// 		stream_type: stream_type as i32,
// 	// 		count: 128,
// 	// 		query: Some(job_log::read::request::Query::AfterTs(now + 7 * MESSAGE_INTERVAL)),
// 	// 	})
// 	// 	.await
// 	// 	.unwrap();
// 	// 	assert_eq!(BATCH_SIZE - (7 + 2), res.entries.len());
// 	// 	for (i, entry) in res.entries.iter().enumerate() {
// 	// 		assert_eq!(
// 	// 			now + ((i - 2) + 8) as i64 * MESSAGE_INTERVAL,
// 	// 			entry.ts,
// 	// 			"i: {i}"
// 	// 		);
// 	// 	}
// 	// }

// 	// // All
// 	// {
// 	// 	let res = op!([ctx] job_log_read {
// 	// 		run_id: Some(run_id.into()),
// 	// 		task:
// 	// 		stream_type: stream_type as i32,
// 	// 		count: 128,
// 	// 		query: Some(job_log::read::request::Query::All(())),
// 	// 	})
// 	// 	.await
// 	// 	.unwrap();
// 	// 	assert_eq!(BATCH_SIZE, res.entries.len());
// 	// 	for (i, entry) in res.entries.iter().enumerate() {
// 	// 		assert_eq!(now + i as i64 * MESSAGE_INTERVAL, entry.ts, "i: {i}");
// 	// 	}
// 	// }
// }
