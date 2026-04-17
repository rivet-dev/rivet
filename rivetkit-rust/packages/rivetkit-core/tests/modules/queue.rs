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
		CompletableQueueMessage, QueueMessage, QueueMetadata, decode_queue_message_key,
		decode_queue_metadata, encode_queue_metadata, make_queue_message_key,
	};
	use crate::actor::context::tests::new_with_kv;
	use crate::actor::queue::{EnqueueAndWaitOpts, QueueNextOpts, QueueWaitOpts};
	use tokio::time::{Duration, sleep};
	use tokio_util::sync::CancellationToken;

	const QUEUE_METADATA_HEX: &str = "04002a0000000000000007000000";
	const QUEUE_MESSAGE_HEX: &str = "0400036a6f6205a16178182ac80100000000000000000000";

	fn hex(bytes: &[u8]) -> String {
		bytes.iter().map(|byte| format!("{byte:02x}")).collect()
	}

	#[test]
	fn queue_message_keys_are_big_endian() {
		let first = make_queue_message_key(1);
		let second = make_queue_message_key(2);

		assert!(first < second);
		assert_eq!(super::QUEUE_METADATA_KEY, [5, 1, 1]);
		assert_eq!(first, vec![5, 1, 2, 0, 0, 0, 0, 0, 0, 0, 1],);
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

	#[tokio::test]
	async fn wait_for_names_skips_non_matching_messages() {
		let ctx = new_with_kv(
			"actor-1",
			"queue-wait-for-names",
			Vec::new(),
			"local",
			crate::kv::tests::new_in_memory(),
		);

		ctx.queue()
			.send("ignored", b"first")
			.await
			.expect("send ignored message");
		ctx.queue()
			.send("target", b"second")
			.await
			.expect("send target message");

		let message = ctx
			.queue()
			.wait_for_names(vec!["target".into()], QueueWaitOpts::default())
			.await
			.expect("wait for names should receive target");
		assert_eq!(message.name, "target");
		assert_eq!(message.body, b"second".to_vec());

		let remaining = ctx
			.queue()
			.next(QueueNextOpts::default())
			.await
			.expect("queue next should succeed")
			.expect("ignored message should remain in queue");
		assert_eq!(remaining.name, "ignored");
		assert_eq!(remaining.body, b"first".to_vec());
	}

	#[tokio::test]
	async fn wait_for_names_returns_timeout_error() {
		let ctx = new_with_kv(
			"actor-1",
			"queue-wait-timeout",
			Vec::new(),
			"local",
			crate::kv::tests::new_in_memory(),
		);

		let error = ctx
			.queue()
			.wait_for_names(
				vec!["missing".into()],
				QueueWaitOpts {
					timeout: Some(Duration::from_millis(0)),
					signal: None,
					completable: false,
				},
			)
			.await
			.expect_err("wait for names should time out");
		let error = rivet_error::RivetError::extract(&error);
		assert_eq!(error.group(), "queue");
		assert_eq!(error.code(), "timed_out");
	}

	#[tokio::test]
	async fn wait_for_names_tracks_active_waits_until_signal_abort() {
		let ctx = new_with_kv(
			"actor-1",
			"queue-wait-signal-abort",
			Vec::new(),
			"local",
			crate::kv::tests::new_in_memory(),
		);
		let signal = CancellationToken::new();
		let queue = ctx.queue().clone();
		let signal_for_task = signal.clone();

		let wait_task = tokio::spawn(async move {
			queue
				.wait_for_names(
					vec!["missing".into()],
					QueueWaitOpts {
						timeout: Some(Duration::from_secs(5)),
						signal: Some(signal_for_task),
						completable: false,
					},
				)
				.await
		});

		for _ in 0..20 {
			if ctx.queue().active_queue_wait_count() == 1 {
				break;
			}
			sleep(Duration::from_millis(10)).await;
		}
		assert_eq!(ctx.queue().active_queue_wait_count(), 1);

		signal.cancel();

		let error = wait_task
			.await
			.expect("wait task should join")
			.expect_err("wait should abort");
		let error = rivet_error::RivetError::extract(&error);
		assert_eq!(error.group(), "actor");
		assert_eq!(error.code(), "aborted");
		assert_eq!(ctx.queue().active_queue_wait_count(), 0);
	}

	#[tokio::test]
	async fn enqueue_and_wait_returns_completion_response() {
		let ctx = new_with_kv(
			"actor-1",
			"queue-enqueue-and-wait",
			Vec::new(),
			"local",
			crate::kv::tests::new_in_memory(),
		);

		let consumer_queue = ctx.queue().clone();
		let consumer = tokio::spawn(async move {
			let message = consumer_queue
				.next(QueueNextOpts {
					names: Some(vec!["jobs".into()]),
					timeout: Some(Duration::from_secs(1)),
					signal: None,
					completable: true,
				})
				.await
				.expect("receive completable queue message")
				.expect("queue message should exist");
			message
				.complete(Some(b"done".to_vec()))
				.await
				.expect("complete message");
		});

		let response = ctx
			.queue()
			.enqueue_and_wait(
				"jobs",
				b"payload",
				EnqueueAndWaitOpts {
					timeout: Some(Duration::from_secs(1)),
					signal: None,
				},
			)
			.await
			.expect("enqueue_and_wait should succeed");

		consumer.await.expect("consumer join");
		assert_eq!(response, Some(b"done".to_vec()));
	}

	#[tokio::test]
	async fn enqueue_and_wait_returns_timeout_error() {
		let ctx = new_with_kv(
			"actor-1",
			"queue-enqueue-and-wait-timeout",
			Vec::new(),
			"local",
			crate::kv::tests::new_in_memory(),
		);

		let error = ctx
			.queue()
			.enqueue_and_wait(
				"jobs",
				b"payload",
				EnqueueAndWaitOpts {
					timeout: Some(Duration::from_millis(0)),
					signal: None,
				},
			)
			.await
			.expect_err("enqueue_and_wait should time out");
		let error = rivet_error::RivetError::extract(&error);
		assert_eq!(error.group(), "queue");
		assert_eq!(error.code(), "timed_out");
	}

	#[tokio::test]
	async fn enqueue_and_wait_returns_abort_error_when_signal_is_cancelled() {
		let ctx = new_with_kv(
			"actor-1",
			"queue-enqueue-and-wait-abort",
			Vec::new(),
			"local",
			crate::kv::tests::new_in_memory(),
		);
		let signal = CancellationToken::new();
		signal.cancel();

		let error = ctx
			.queue()
			.enqueue_and_wait(
				"jobs",
				b"payload",
				EnqueueAndWaitOpts {
					timeout: Some(Duration::from_secs(1)),
					signal: Some(signal),
				},
			)
			.await
			.expect_err("enqueue_and_wait should abort");
		let error = rivet_error::RivetError::extract(&error);
		assert_eq!(error.group(), "actor");
		assert_eq!(error.code(), "aborted");
	}
}
