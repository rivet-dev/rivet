mod moved_tests {
	use futures::StreamExt;
	use tokio_util::sync::CancellationToken;

	use crate::queue::{QueueStreamExt, QueueStreamOpts};
	use rivetkit_core::{ActorContext, Kv};

	#[tokio::test]
	async fn queue_stream_yields_messages_through_stream_ext_combinators() {
		let ctx = ActorContext::new_with_kv(
			"actor-id",
			"test",
			Vec::new(),
			"local",
			Kv::new_in_memory(),
		);
		let queue = ctx.queue();

		queue.send("alpha", br#"{"value":1}"#).await.expect("send alpha");
		queue.send("beta", br#"{"value":2}"#).await.expect("send beta");

		let names = queue
			.stream(QueueStreamOpts::default())
			.map(|message| message.name)
			.take(2)
			.collect::<Vec<_>>()
			.await;

		assert_eq!(names, vec!["alpha".to_owned(), "beta".to_owned()]);
	}

	#[tokio::test]
	async fn queue_stream_honors_name_filters() {
		let ctx = ActorContext::new_with_kv(
			"actor-id",
			"test",
			Vec::new(),
			"local",
			Kv::new_in_memory(),
		);
		let queue = ctx.queue();

		queue.send("skip", b"1").await.expect("send skip");
		queue.send("keep", b"2").await.expect("send keep");

		let message = queue
			.stream(QueueStreamOpts {
				names: Some(vec!["keep".to_owned()]),
				signal: None,
			})
			.next()
			.await
			.expect("filtered stream should yield");

		assert_eq!(message.name, "keep");
		assert_eq!(message.body, b"2");
	}

	#[tokio::test]
	async fn queue_stream_ends_when_cancellation_signal_is_already_fired() {
		let ctx = ActorContext::new_with_kv(
			"actor-id",
			"test",
			Vec::new(),
			"local",
			Kv::new_in_memory(),
		);
		let queue = ctx.queue();
		let signal = CancellationToken::new();
		signal.cancel();

		let next = queue
			.stream(QueueStreamOpts {
				names: None,
				signal: Some(signal),
			})
			.next()
			.await;

		assert!(next.is_none());
	}
}
