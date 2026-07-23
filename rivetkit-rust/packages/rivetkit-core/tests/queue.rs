use super::*;

mod moved_tests {
	use super::{
		QueueMessageStatus, QueueNextBatchOpts, QueueNextOpts, QueueSendOpts, QueueWaitOpts,
		queue_backoff_with_jitter,
	};

	use crate::actor::context::ActorContext;
	use crate::actor::keys::{
		QUEUE_METADATA_KEY, decode_queue_message_key, make_queue_message_key,
	};
	use crate::kv::Kv;
	use std::time::Duration;
	use tokio::task::yield_now;
	use tokio_util::sync::CancellationToken;

	fn test_queue() -> ActorContext {
		ActorContext::new_with_kv(
			"actor-queue",
			"queue-test",
			Vec::new(),
			"local",
			Kv::new_in_memory(),
		)
	}

	fn assert_actor_aborted(error: anyhow::Error) {
		let error = rivet_error::RivetError::extract(&error);
		assert_eq!(error.group(), "actor");
		assert_eq!(error.code(), "aborted");
	}

	#[tokio::test]
	async fn next_batch_filters_and_limits_messages_in_enqueue_order() {
		let queue = test_queue();
		crate::actor::internal_storage::schema::ensure_internal_schema(queue.sql())
			.await
			.expect("initialize queue storage");
		for (name, body) in [
			("ignored", b"first".as_slice()),
			("target", b"second".as_slice()),
			("target", b"third".as_slice()),
			("target", b"fourth".as_slice()),
		] {
			queue.send(name, body).await.expect("send queue message");
		}

		let selected = queue
			.next_batch(QueueNextBatchOpts {
				names: Some(vec!["target".into()]),
				count: 2,
				timeout: None,
				signal: None,
			})
			.await
			.expect("receive filtered queue batch");
		assert_eq!(
			selected
				.into_iter()
				.map(|message| message.body)
				.collect::<Vec<_>>(),
			vec![b"second".to_vec(), b"third".to_vec()]
		);

		let remaining = queue.inspect_messages().await.expect("inspect queue");
		assert_eq!(
			remaining
				.into_iter()
				.map(|message| message.body)
				.collect::<Vec<_>>(),
			vec![b"first".to_vec(), b"fourth".to_vec()]
		);
	}

	#[tokio::test]
	async fn next_batch_supports_large_name_filters_without_sql_bind_expansion() {
		let queue = test_queue();
		crate::actor::internal_storage::schema::ensure_internal_schema(queue.sql())
			.await
			.expect("initialize queue storage");
		let target = format!("queue-{:04}-{}", 1_099, "x".repeat(112));
		queue
			.send(&target, b"selected")
			.await
			.expect("send queue message");
		let names = (0..1_100)
			.map(|index| format!("queue-{index:04}-{}", "x".repeat(112)))
			.collect();

		let selected = queue
			.next_batch(QueueNextBatchOpts {
				names: Some(names),
				count: 1,
				timeout: None,
				signal: None,
			})
			.await
			.expect("receive from large name filter");

		assert_eq!(selected.len(), 1);
		assert_eq!(selected[0].name, target);
		assert_eq!(selected[0].body, b"selected");
	}

	#[test]
	fn queue_message_keys_are_big_endian() {
		let first = make_queue_message_key(1);
		let second = make_queue_message_key(2);

		assert!(first < second);
		assert_eq!(QUEUE_METADATA_KEY, [5, 1, 1]);
		assert_eq!(first, vec![5, 1, 2, 0, 0, 0, 0, 0, 0, 0, 1]);
		assert_eq!(decode_queue_message_key(&first).expect("decode first"), 1);
		assert_eq!(decode_queue_message_key(&second).expect("decode second"), 2);
	}

	#[tokio::test]
	async fn wait_for_names_returns_aborted_when_signal_is_already_cancelled() {
		let queue = test_queue();
		let signal = CancellationToken::new();
		signal.cancel();

		let error = queue
			.wait_for_names(
				vec!["missing".to_owned()],
				QueueWaitOpts {
					signal: Some(signal),
					..Default::default()
				},
			)
			.await
			.expect_err("already-cancelled waits should abort immediately");

		assert_actor_aborted(error);
	}

	#[tokio::test(start_paused = true)]
	async fn wait_for_names_returns_aborted_when_signal_cancels_during_wait() {
		let queue = test_queue();
		let signal = CancellationToken::new();
		let wait_signal = signal.clone();
		let wait_queue = queue.clone();

		let wait = tokio::spawn(async move {
			wait_queue
				.wait_for_names(
					vec!["missing".to_owned()],
					QueueWaitOpts {
						timeout: Some(Duration::from_secs(60)),
						signal: Some(wait_signal),
						..Default::default()
					},
				)
				.await
		});

		yield_now().await;
		signal.cancel();

		let error = wait
			.await
			.expect("wait task should join")
			.expect_err("cancelled waits should abort");

		assert_actor_aborted(error);
	}

	#[tokio::test(start_paused = true)]
	async fn next_returns_aborted_when_actor_signal_cancels_during_wait() {
		let queue = test_queue();

		let wait = tokio::spawn({
			let queue = queue.clone();
			async move {
				queue
					.next(QueueNextOpts {
						names: Some(vec!["missing".to_owned()]),
						timeout: Some(Duration::from_secs(60)),
						..Default::default()
					})
					.await
			}
		});

		yield_now().await;
		queue.cancel_actor_abort_signal();

		let error = wait
			.await
			.expect("wait task should join")
			.expect_err("cancelled actor waits should abort");

		assert_actor_aborted(error);
	}

	#[tokio::test]
	async fn dedupe_key_returns_the_original_receipt() {
		let queue = test_queue();
		let options = QueueSendOpts {
			dedupe_key: Some("order-1".to_owned()),
			delay: None,
		};
		let first = queue
			.send_with_opts("jobs", b"first", options.clone())
			.await
			.expect("first send");
		let duplicate = queue
			.send_with_opts("jobs", b"duplicate", options)
			.await
			.expect("duplicate send");

		assert!(!first.deduplicated);
		assert!(duplicate.deduplicated);
		assert_eq!(first.id, duplicate.id);
		assert!(matches!(
			queue.queue_status(&first.id).await.expect("queue status"),
			QueueMessageStatus::Queued { attempts: 0, .. }
		));
	}

	#[tokio::test]
	async fn raw_receive_records_consumed_status() {
		let queue = test_queue();
		let receipt = queue.send("jobs", b"payload").await.expect("send");
		let message = queue
			.next(QueueNextOpts::default())
			.await
			.expect("receive")
			.expect("queued message");

		assert_eq!(message.receipt_id, receipt.id);
		assert!(matches!(
			queue
				.queue_status(&receipt.id)
				.await
				.expect("consumed status"),
			QueueMessageStatus::Consumed { .. }
		));
	}

	#[tokio::test]
	async fn zero_delay_is_immediately_queued_not_delayed() {
		let queue = test_queue();
		let receipt = queue
			.send_with_opts(
				"jobs",
				b"payload",
				QueueSendOpts {
					dedupe_key: None,
					delay: Some(Duration::ZERO),
				},
			)
			.await
			.expect("send");

		assert!(matches!(
			queue.queue_status(&receipt.id).await.expect("queue status"),
			QueueMessageStatus::Queued { attempts: 0, .. }
		));
	}

	#[tokio::test]
	async fn refresh_queue_metadata_repairs_a_stale_admission_counter() {
		let queue = test_queue();
		let mut config = crate::actor::config::ActorConfig::default();
		config.max_queue_size = 1;
		queue.configure_queue(config);
		queue.send("jobs", b"first").await.expect("first send");

		// Simulate cancellation after a durable mutation but before the matching
		// in-memory update in a prior actor generation.
		queue.decrement_queue_size(1).await;
		queue
			.refresh_queue_metadata()
			.await
			.expect("refresh queue metadata");

		let error = queue
			.send("jobs", b"second")
			.await
			.expect_err("persisted queue depth should still enforce admission");
		let error = rivet_error::RivetError::extract(&error);
		assert_eq!(error.group(), "queue");
		assert_eq!(error.code(), "full");
	}

	#[test]
	fn retry_jitter_never_exceeds_configured_maximum() {
		let definition = crate::actor::config::QueueDefinition {
			name: "jobs".to_owned(),
			on_message: true,
			on_dead_letter: false,
			timeout: Duration::from_secs(30),
			max_attempts: 3,
			backoff_initial: Duration::from_secs(30),
			backoff_factor: 2.0,
			backoff_max: Duration::from_secs(30),
			backoff_jitter: true,
		};

		assert_eq!(
			queue_backoff_with_jitter(&definition, 3, 1.5),
			Duration::from_secs(30)
		);
	}
}
