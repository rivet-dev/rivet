use super::*;

pub(crate) fn begin_sleep_test_wait(queue: &Queue) {
	queue
		.0
		.active_queue_wait_count
		.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
	queue.notify_wait_activity();
}

pub(crate) fn end_sleep_test_wait(queue: &Queue) {
	let previous = queue
		.0
		.active_queue_wait_count
		.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
	if previous == 0 {
		queue
			.0
			.active_queue_wait_count
			.store(0, std::sync::atomic::Ordering::SeqCst);
	}
	queue.notify_wait_activity();
}

mod moved_tests {
	use super::{
		CompletableQueueMessage, QueueMessage, QueueMetadata,
		decode_queue_message_key, decode_queue_metadata, encode_queue_metadata,
		make_queue_message_key,
	};
	use crate::actor::context::tests::new_with_kv;
	use crate::actor::queue::QueueNextOpts;

	const QUEUE_METADATA_HEX: &str = "04002a0000000000000007000000";
	const QUEUE_MESSAGE_HEX: &str =
		"0400036a6f6205a16178182ac80100000000000000000000";

	fn hex(bytes: &[u8]) -> String {
		bytes.iter().map(|byte| format!("{byte:02x}")).collect()
	}

	#[test]
	fn queue_message_keys_are_big_endian() {
		let first = make_queue_message_key(1);
		let second = make_queue_message_key(2);

		assert!(first < second);
		assert_eq!(super::QUEUE_METADATA_KEY, [5, 1, 1]);
		assert_eq!(
			first,
			vec![5, 1, 2, 0, 0, 0, 0, 0, 0, 0, 1],
		);
		assert_eq!(decode_queue_message_key(&first).expect("decode first"), 1);
		assert_eq!(decode_queue_message_key(&second).expect("decode second"), 2);
	}

	#[test]
	fn queue_metadata_round_trips_with_embedded_version() {
		let metadata = QueueMetadata {
			next_id: 42,
			size: 7,
		};

		let encoded = encode_queue_metadata(&metadata).expect("encode metadata");
		assert_eq!(hex(&encoded), QUEUE_METADATA_HEX);
		let decoded = decode_queue_metadata(&encoded).expect("decode metadata");

		assert_eq!(decoded, metadata);
	}

	#[test]
	fn queue_message_into_completable_requires_completion_handle() {
		let message = QueueMessage {
			id: 1,
			name: "tasks".into(),
			body: vec![1, 2, 3],
			created_at: 5,
			completion: None,
		};

		let error = message
			.into_completable()
			.expect_err("message should not be completable");

		assert!(error.to_string().contains("does not support completion"));
	}

	#[test]
	fn completable_message_round_trips_back_to_queue_message() {
		let completion = super::CompletionHandle::new(super::Queue::default(), 9);
		let message = CompletableQueueMessage {
			id: 9,
			name: "jobs".into(),
			body: vec![9],
			created_at: 11,
			completion,
		};

		let queue_message = message.into_message();
		assert!(queue_message.is_completable());
	}

	#[test]
	fn queue_message_hex_vector() {
		let encoded = super::encode_queue_message(&super::PersistedQueueMessage {
			name: "job".into(),
			body: vec![0xa1, 0x61, 0x78, 0x18, 0x2a],
			created_at: 456,
			failure_count: None,
			available_at: None,
			in_flight: None,
			in_flight_at: None,
		})
		.expect("encode queue message");

		assert_eq!(hex(&encoded), QUEUE_MESSAGE_HEX);
		let decoded = super::decode_queue_message(&encoded).expect("decode queue message");
		assert_eq!(decoded.name, "job");
		assert_eq!(decoded.body, vec![0xa1, 0x61, 0x78, 0x18, 0x2a]);
		assert_eq!(decoded.created_at, 456);
	}

	#[tokio::test]
	async fn queue_operations_update_prometheus_metrics() {
		let ctx = new_with_kv(
			"actor-1",
			"queue-metrics",
			Vec::new(),
			"local",
			crate::kv::tests::new_in_memory(),
		);

		ctx.queue()
			.send("jobs", b"payload")
			.await
			.expect("queue send should succeed");
		let message = ctx
			.queue()
			.next(QueueNextOpts::default())
			.await
			.expect("queue next should succeed")
			.expect("queue message should exist");
		assert_eq!(message.body, b"payload".to_vec());

		let metrics = ctx.render_metrics().expect("render metrics");
		let sent_line = metrics
			.lines()
			.find(|line| line.starts_with("queue_messages_sent_total"))
			.expect("sent metric line");
		let received_line = metrics
			.lines()
			.find(|line| line.starts_with("queue_messages_received_total"))
			.expect("received metric line");
		let depth_line = metrics
			.lines()
			.find(|line| line.starts_with("queue_depth"))
			.expect("depth metric line");

		assert!(sent_line.ends_with(" 1"));
		assert!(received_line.ends_with(" 1"));
		assert!(depth_line.ends_with(" 0"));
	}
}
