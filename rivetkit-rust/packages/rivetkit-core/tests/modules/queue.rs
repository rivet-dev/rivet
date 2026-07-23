use super::*;

#[path = "../metrics_helpers.rs"]
mod metrics_helpers;

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
		QueueMetadata, decode_queue_metadata, encode_queue_metadata,
	};
	use crate::actor::context::tests::new_with_kv;
	use crate::actor::keys::{
		QUEUE_METADATA_KEY, decode_queue_message_key, make_queue_message_key,
	};
	use crate::actor::queue::{
		QueueMessageStatus, QueueNextOpts, QueueSendOpts, QueueWaitOpts,
	};
	use tokio::time::{Duration, sleep};
	use tokio_util::sync::CancellationToken;

	use super::metrics_helpers::{metric_line_for_actor, render_global_metrics};

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
		assert_eq!(QUEUE_METADATA_KEY, [5, 1, 1]);
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
			"queue-metrics-actor",
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

		let metrics = render_global_metrics();
		let sent_line = metrics
			.lines()
			.find(|line| metric_line_for_actor(line, "rivetkit_actor_queue_messages_sent_total", "queue-metrics"))
			.expect("sent metric line");
		let received_line = metrics
			.lines()
			.find(|line| metric_line_for_actor(line, "rivetkit_actor_queue_messages_received_total", "queue-metrics"))
			.expect("received metric line");

		assert!(sent_line.ends_with(" 1"));
		assert!(received_line.ends_with(" 1"));
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
	async fn send_returns_stable_deduplicated_receipt() {
		let ctx = new_with_kv(
			"actor-1",
			"queue-deduplicated-receipt",
			Vec::new(),
			"local",
			crate::kv::tests::new_in_memory(),
		);

		let first = ctx
			.queue()
			.send_with_opts(
				"jobs",
				b"payload",
				QueueSendOpts {
					dedupe_key: Some("order-1".into()),
					delay: None,
				},
			)
			.await
			.expect("first send");
		let second = ctx
			.queue()
			.send_with_opts(
				"jobs",
				b"different payload is ignored during dedupe window",
				QueueSendOpts {
					dedupe_key: Some("order-1".into()),
					delay: None,
				},
			)
			.await
			.expect("deduplicated send");

		assert!(!first.deduplicated);
		assert!(second.deduplicated);
		assert_eq!(first.id, second.id);
		assert!(matches!(
			ctx.queue().queue_status(&first.id).await.expect("status"),
			QueueMessageStatus::Queued { attempts: 0, .. }
		));
	}

	#[tokio::test]
	async fn raw_receive_consumes_message_and_records_terminal_receipt() {
		let ctx = new_with_kv(
			"actor-1",
			"queue-raw-consumed-receipt",
			Vec::new(),
			"local",
			crate::kv::tests::new_in_memory(),
		);
		let receipt = ctx
			.queue()
			.send("jobs", b"payload")
			.await
			.expect("send");
		let message = ctx
			.queue()
			.next(QueueNextOpts::default())
			.await
			.expect("receive")
			.expect("message");
		assert_eq!(message.receipt_id, receipt.id);
		assert!(matches!(
			ctx.queue().queue_status(&receipt.id).await.expect("status"),
			QueueMessageStatus::Consumed { .. }
		));
	}

	#[tokio::test]
	async fn wait_for_names_available_does_not_consume_the_message() {
		let ctx = new_with_kv(
			"actor-1",
			"queue-wait-available",
			Vec::new(),
			"local",
			crate::kv::tests::new_in_memory(),
		);
		ctx.queue().send("target", b"payload").await.expect("send");

		ctx.queue()
			.wait_for_names_available(
				vec!["target".into()],
				QueueWaitOpts {
					timeout: Some(Duration::ZERO),
					signal: None,
				},
			)
			.await
			.expect("message should be available");

		let message = ctx
			.queue()
			.next(QueueNextOpts::default())
			.await
			.expect("receive")
			.expect("wait must not consume");
		assert_eq!(message.body, b"payload");
	}

	#[tokio::test]
	async fn raw_receive_skips_delayed_messages() {
		let ctx = new_with_kv(
			"actor-1",
			"queue-delayed-raw",
			Vec::new(),
			"local",
			crate::kv::tests::new_in_memory(),
		);
		let receipt = ctx
			.queue()
			.send_with_opts(
				"jobs",
				b"later",
				QueueSendOpts {
					dedupe_key: None,
					delay: Some(Duration::from_secs(60)),
				},
			)
			.await
			.expect("send delayed");

		let message = ctx
			.queue()
			.next(QueueNextOpts {
				timeout: Some(Duration::ZERO),
				..QueueNextOpts::default()
			})
			.await
			.expect("receive");
		assert!(message.is_none());
		assert!(matches!(
			ctx.queue().queue_status(&receipt.id).await.expect("status"),
			QueueMessageStatus::Delayed { attempts: 0, .. }
		));
	}
}
