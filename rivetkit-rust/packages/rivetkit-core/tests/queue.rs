use super::*;

mod moved_tests {
	use super::{
		PersistedQueueMessage, QueueMetadata, QueueNextOpts, QueueWaitOpts, encode_queue_message,
		encode_queue_metadata,
	};

	use crate::actor::context::ActorContext;
	use crate::actor::keys::{
		QUEUE_MESSAGES_PREFIX, QUEUE_METADATA_KEY, decode_queue_message_key, make_queue_message_key,
	};
	use crate::actor::preload::PreloadedKv;
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
	async fn inspect_messages_uses_preloaded_queue_entries_when_present() {
		let queue = ActorContext::new_with_kv(
			"actor-queue",
			"queue-preload",
			Vec::new(),
			"local",
			Kv::default(),
		);
		let metadata = QueueMetadata {
			next_id: 8,
			size: 1,
		};
		let persisted = PersistedQueueMessage {
			name: "preloaded".to_owned(),
			body: b"body".to_vec(),
			created_at: 42,
			failure_count: None,
			available_at: None,
			in_flight: None,
			in_flight_at: None,
		};
		queue.configure_preload(Some(PreloadedKv::new_with_requested_get_keys(
			[
				(
					QUEUE_METADATA_KEY.to_vec(),
					encode_queue_metadata(&metadata).expect("metadata should encode"),
				),
				(
					make_queue_message_key(7),
					encode_queue_message(&persisted).expect("message should encode"),
				),
			],
			vec![QUEUE_METADATA_KEY.to_vec()],
			vec![QUEUE_MESSAGES_PREFIX.to_vec()],
		)));

		let messages = queue
			.inspect_messages()
			.await
			.expect("queue should initialize from preload without touching kv");

		assert_eq!(messages.len(), 1);
		assert_eq!(messages[0].id, 7);
		assert_eq!(messages[0].name, "preloaded");
		assert_eq!(messages[0].body, b"body");
		assert_eq!(*queue.0.queue_metadata.lock().await, metadata);
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
		queue.mark_destroy_requested();

		let error = wait
			.await
			.expect("wait task should join")
			.expect_err("cancelled actor waits should abort");

		assert_actor_aborted(error);
	}
}
