use std::{sync::Arc, time::Duration};

use sqlite_storage::compactor::{
	SQLITE_COMPACT_PAYLOAD_VERSION, SQLITE_COMPACT_SUBJECT, SqliteCompactPayload,
	SqliteCompactSubject, decode_compact_payload, encode_compact_payload, publish_compact_trigger,
};
use universalpubsub::{NextOutput, PubSub, driver::memory::MemoryDriver};

fn test_ups() -> PubSub {
	PubSub::new(Arc::new(MemoryDriver::new(
		"sqlite-storage-compactor-dispatch-test".to_string(),
	)))
}

#[test]
fn module_compiles() {}

#[test]
fn compact_subject_uses_constant_subject_string() {
	assert_eq!(SqliteCompactSubject.to_string(), SQLITE_COMPACT_SUBJECT);
	assert_eq!(SQLITE_COMPACT_SUBJECT, "sqlite.compact");
}

#[test]
fn compact_payload_round_trips_with_embedded_version() {
	for payload in [
		SqliteCompactPayload {
			actor_id: String::new(),
			commit_bytes_since_rollup: 0,
			read_bytes_since_rollup: 0,
		},
		SqliteCompactPayload {
			actor_id: "actor-a".to_string(),
			commit_bytes_since_rollup: u64::MAX,
			read_bytes_since_rollup: u64::MAX - 1,
		},
	] {
		let encoded = encode_compact_payload(payload.clone()).expect("payload should encode");
		assert_eq!(
			u16::from_le_bytes([encoded[0], encoded[1]]),
			SQLITE_COMPACT_PAYLOAD_VERSION
		);

		let decoded = decode_compact_payload(&encoded).expect("payload should decode");
		assert_eq!(decoded, payload);
	}
}

#[tokio::test]
async fn publish_compact_trigger_returns_unit_not_future() {
	let ups = test_ups();
	let _: () = publish_compact_trigger(&ups, "actor-1");
}

#[tokio::test(start_paused = true)]
async fn publish_compact_trigger_does_not_block_caller() {
	let ups = test_ups();
	let now = tokio::time::Instant::now();

	let _: () = publish_compact_trigger(&ups, "actor-1");

	assert_eq!(tokio::time::Instant::now(), now);
}

#[tokio::test]
async fn publish_compact_trigger_sends_fire_and_forget_ups_message() {
	let ups = test_ups();
	let mut sub = ups
		.queue_subscribe(SqliteCompactSubject, "compactor")
		.await
		.expect("subscriber should start");

	publish_compact_trigger(&ups, "actor-a");

	let msg = tokio::time::timeout(Duration::from_secs(1), sub.next())
		.await
		.expect("trigger should publish")
		.expect("subscriber should receive");

	let NextOutput::Message(msg) = msg else {
		panic!("subscriber unexpectedly unsubscribed");
	};
	let payload = decode_compact_payload(&msg.payload).expect("payload should decode");

	assert_eq!(
		payload,
		SqliteCompactPayload {
			actor_id: "actor-a".to_string(),
			commit_bytes_since_rollup: 0,
			read_bytes_since_rollup: 0,
		}
	);
}
