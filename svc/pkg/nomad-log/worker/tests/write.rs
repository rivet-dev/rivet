use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn basic(ctx: TestCtx) {
	let alloc = Uuid::new_v4().to_string();
	let task = Uuid::new_v4().to_string();
	let now = util::timestamp::now();
	let messages = vec![b"Message 1", b"Message 2", b"Message 3", b"Message 4"];
	msg!([ctx] nomad_log::msg::entries(&alloc, &task, "stdout") {
		alloc: alloc.clone(),
		task: task.clone(),
		stream_type: backend::nomad_log::StreamType::StdOut as i32,
		entries: vec![
			backend::nomad_log::LogEntry {
				ts: now,
				idx: 0,
				message: messages[0].to_vec(),
			},
			backend::nomad_log::LogEntry {
				ts: now,
				idx: 1,
				message: messages[1].to_vec(),
			},
			backend::nomad_log::LogEntry {
				ts: now,
				idx: 2,
				message: messages[2].to_vec(),
			},
			backend::nomad_log::LogEntry {
				ts: now + 10,
				idx: 0,
				message: messages[3].to_vec(),
			},
		],
	})
	.await
	.unwrap();

	// HACK: Message at the end of a test will get killed since it is spawned
	// int he background
	tokio::time::sleep(std::time::Duration::from_secs(1)).await;

	// TODO: Add back when we can read from ClickHouse from tests

	// let query_messages = loop {
	// 	tokio::time::sleep(Duration::from_secs(1)).await;

	// 	tracing::info!("querying messages");
	// 	let query_messages = scylla
	// 		.query(
	// 			"SELECT message FROM logs WHERE alloc = ? AND task = ? AND stream_type = 0 ORDER BY ts ASC, idx ASC",
	// 			(&alloc, &task),
	// 		)
	// 		.await
	// 		.unwrap()
	// 		.rows_typed::<(Vec<u8>,)>()
	// 		.unwrap()
	// 		.collect::<Result<Vec<_>, _>>()
	// 		.unwrap();
	// 	if !query_messages.is_empty() {
	// 		break query_messages;
	// 	} else {
	// 		tracing::info!("logs not written to db yet");
	// 	}
	// };

	// let query_messages = query_messages.iter().map(|x| &x.0[..]).collect::<Vec<_>>();
	// assert_eq!(
	// 	messages, query_messages,
	// 	"messages do not match sent messages"
	// );
}

// TODO: Add back when we can read from ClickHouse from tests
// #[worker_test]
// async fn stress(ctx: TestCtx) {
// 	let scylla = ctx.scylla().await.unwrap();

// 	let alloc = Uuid::new_v4().to_string();
// 	let task = Uuid::new_v4().to_string();
// 	let now = util::timestamp::now();
// 	let messages = vec![b"Message 1", b"Message 2", b"Message 3", b"Message 4"];
// 	msg!([ctx] nomad_log::msg::entries(&alloc, &task, "stdout") {
// 		alloc: alloc.clone(),
// 		task: task.clone(),
// 		stream_type: backend::nomad_log::StreamType::StdOut as i32,
// 		entries: vec![
// 			backend::nomad_log::LogEntry {
// 				ts: now,
// 				idx: 0,
// 				message: messages[0].to_vec(),
// 			},
// 			backend::nomad_log::LogEntry {
// 				ts: now,
// 				idx: 1,
// 				message: messages[1].to_vec(),
// 			},
// 			backend::nomad_log::LogEntry {
// 				ts: now,
// 				idx: 2,
// 				message: messages[2].to_vec(),
// 			},
// 			backend::nomad_log::LogEntry {
// 				ts: now + 10,
// 				idx: 0,
// 				message: messages[3].to_vec(),
// 			},
// 		],
// 	})
// 	.await
// 	.unwrap();

// 	let query_messages = loop {
// 		tokio::time::sleep(Duration::from_secs(1)).await;

// 		tracing::info!("querying messages");
// 		let query_messages = scylla
// 			.query(
// 				"SELECT message FROM logs WHERE alloc = ? AND task = ? AND stream_type = 0 ORDER BY ts ASC, idx ASC",
// 				(&alloc, &task),
// 			)
// 			.await
// 			.unwrap()
// 			.rows_typed::<(Vec<u8>,)>()
// 			.unwrap()
// 			.collect::<Result<Vec<_>, _>>()
// 			.unwrap();
// 		if !query_messages.is_empty() {
// 			break query_messages;
// 		} else {
// 			tracing::info!("logs not written to db yet");
// 		}
// 	};

// 	let query_messages = query_messages.iter().map(|x| &x.0[..]).collect::<Vec<_>>();
// 	assert_eq!(
// 		messages, query_messages,
// 		"messages do not match sent messages"
// 	);
// }
