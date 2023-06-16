use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

use std::time::Duration;

/// How many logs to create total.
const BATCH_SIZE: usize = 64;

/// How many idx per timestamp;
const IDX_COUNT: usize = 4;

/// How many unique timestamps there are.
const TS_COUNT: usize = BATCH_SIZE / IDX_COUNT;

/// How far aprat messages are created.
const MESSAGE_INTERVAL: i64 = 1000;

#[worker_test]
async fn basic(ctx: TestCtx) {
	let now = 100000i64;

	let alloc = Uuid::new_v4().to_string();
	let task = Uuid::new_v4().to_string();
	let stream_type = backend::nomad_log::StreamType::StdOut;

	// Insert entries
	let entries = (0..TS_COUNT)
		.flat_map(|i| {
			(0..IDX_COUNT).map(move |idx| backend::nomad_log::LogEntry {
				ts: now + (i as i64 * MESSAGE_INTERVAL),
				idx: idx as u32,
				message: format!("Hello, {i}@{idx}!").into_bytes(),
			})
		})
		.collect::<Vec<_>>();
	msg!([ctx] nomad_log::msg::entries(&alloc, &task, "stdout") {
		alloc: alloc.clone(),
		task: task.clone(),
		stream_type: stream_type as i32,
		entries: entries,
	})
	.await
	.unwrap();

	tokio::time::sleep(Duration::from_secs(1)).await;

	// Before ts
	{
		let res = op!([ctx] nomad_log_read {
			alloc: alloc.clone(),
			task: task.clone(),
			stream_type: stream_type as i32,
			count: 128,
			query: Some(nomad_log::read::request::Query::BeforeTs(nomad_log::read::request::TimestampQuery {
				ts: now + 5 * MESSAGE_INTERVAL,
				idx: 2,
			})),
		})
		.await
		.unwrap();
		assert_eq!(5 * IDX_COUNT + 3, res.entries.len());
		for (i, entry) in res.entries.iter().enumerate() {
			assert_eq!(
				now + (i / IDX_COUNT) as i64 * MESSAGE_INTERVAL,
				entry.ts,
				"i: {i}"
			);
		}
	}

	// After ts
	{
		let res = op!([ctx] nomad_log_read {
			alloc: alloc.clone(),
			task: task.clone(),
			stream_type: stream_type as i32,
			count: 128,
			query: Some(nomad_log::read::request::Query::AfterTs(nomad_log::read::request::TimestampQuery {
				ts: now + 7 * MESSAGE_INTERVAL,
				idx: 2,
			})),
		})
		.await
		.unwrap();
		assert_eq!(BATCH_SIZE - (7 * IDX_COUNT + 2), res.entries.len());
		for (i, entry) in res.entries.iter().enumerate() {
			assert_eq!(
				now + ((i - 2) / IDX_COUNT + 8) as i64 * MESSAGE_INTERVAL,
				entry.ts,
				"i: {i}"
			);
		}
	}

	// All
	{
		let res = op!([ctx] nomad_log_read {
			alloc: alloc.clone(),
			task: task.clone(),
			stream_type: stream_type as i32,
			count: 128,
			query: Some(nomad_log::read::request::Query::All(())),
		})
		.await
		.unwrap();
		assert_eq!(BATCH_SIZE, res.entries.len());
		for (i, entry) in res.entries.iter().enumerate() {
			assert_eq!(
				now + (i / IDX_COUNT) as i64 * MESSAGE_INTERVAL,
				entry.ts,
				"i: {i}"
			);
		}
	}
}
