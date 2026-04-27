mod moved_tests {
	use std::sync::Arc;
	use std::sync::atomic::{AtomicUsize, Ordering};

	use crate::actor::context::ActorContext;
	use parking_lot::Mutex as DropMutex;
	use rivet_util::async_counter::AsyncCounter;
	use tokio::sync::oneshot;
	use tokio::task::yield_now;
	use tokio::time::{Duration, Instant, advance};
	use tracing::field::{Field, Visit};
	use tracing::{Event, Subscriber};
	use tracing_subscriber::layer::{Context as LayerContext, Layer};
	use tracing_subscriber::prelude::*;
	use tracing_subscriber::registry::Registry;

	#[derive(Default)]
	struct MessageVisitor {
		message: Option<String>,
	}

	impl Visit for MessageVisitor {
		fn record_str(&mut self, field: &Field, value: &str) {
			if field.name() == "message" {
				self.message = Some(value.to_owned());
			}
		}

		fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
			if field.name() == "message" {
				self.message = Some(format!("{value:?}").trim_matches('"').to_owned());
			}
		}
	}

	#[derive(Clone)]
	struct ShutdownTaskRefusedLayer {
		count: Arc<AtomicUsize>,
	}

	impl<S> Layer<S> for ShutdownTaskRefusedLayer
	where
		S: Subscriber,
	{
		fn on_event(&self, event: &Event<'_>, _ctx: LayerContext<'_, S>) {
			if *event.metadata().level() != tracing::Level::WARN {
				return;
			}

			let mut visitor = MessageVisitor::default();
			event.record(&mut visitor);
			if visitor.message.as_deref()
				== Some("shutdown task spawned after teardown; aborting immediately")
			{
				self.count.fetch_add(1, Ordering::SeqCst);
			}
		}
	}

	struct NotifyOnDrop(DropMutex<Option<oneshot::Sender<()>>>);

	impl NotifyOnDrop {
		fn new(sender: oneshot::Sender<()>) -> Self {
			Self(DropMutex::new(Some(sender)))
		}
	}

	impl Drop for NotifyOnDrop {
		fn drop(&mut self) {
			if let Some(sender) = self.0.lock().take() {
				let _ = sender.send(());
			}
		}
	}

	#[tokio::test(start_paused = true)]
	async fn shutdown_task_counter_reaches_zero_after_completion() {
		let ctx = ActorContext::new_for_sleep_tests("actor-shutdown-complete");
		let (done_tx, done_rx) = oneshot::channel();

		ctx.track_shutdown_task(async move {
			let _ = done_tx.send(());
		});

		done_rx.await.expect("shutdown task should complete");
		yield_now().await;

		assert_eq!(ctx.shutdown_task_count(), 0);
		assert!(
			ctx.0
				.sleep
				.work
				.shutdown_counter
				.wait_zero(Instant::now() + Duration::from_millis(1))
				.await
		);
	}

	#[tokio::test(start_paused = true)]
	async fn shutdown_task_counter_reaches_zero_after_panic() {
		let ctx = ActorContext::new_for_sleep_tests("actor-shutdown-panic");

		ctx.track_shutdown_task(async move {
			panic!("boom");
		});

		yield_now().await;
		yield_now().await;

		assert_eq!(ctx.shutdown_task_count(), 0);
		assert!(
			ctx.0
				.sleep
				.work
				.shutdown_counter
				.wait_zero(Instant::now() + Duration::from_millis(1))
				.await
		);
	}

	#[tokio::test(start_paused = true)]
	async fn teardown_aborts_tracked_shutdown_tasks() {
		let ctx = ActorContext::new_for_sleep_tests("actor-shutdown-teardown");
		let (drop_tx, drop_rx) = oneshot::channel();
		let (_never_tx, never_rx) = oneshot::channel::<()>();
		let notify = NotifyOnDrop::new(drop_tx);

		ctx.track_shutdown_task(async move {
			let _notify = notify;
			let _ = never_rx.await;
		});

		assert_eq!(ctx.shutdown_task_count(), 1);

		ctx.teardown_sleep_state().await;
		advance(Duration::from_millis(1)).await;

		drop_rx
			.await
			.expect("teardown should abort the tracked task");
		assert_eq!(ctx.shutdown_task_count(), 0);
	}

	#[tokio::test(start_paused = true)]
	async fn track_shutdown_task_refuses_spawns_after_teardown() {
		let ctx = ActorContext::new_for_sleep_tests("actor-shutdown-refuse");
		let warning_count = Arc::new(AtomicUsize::new(0));
		let subscriber = Registry::default().with(ShutdownTaskRefusedLayer {
			count: warning_count.clone(),
		});
		let _guard = tracing::subscriber::set_default(subscriber);

		ctx.teardown_sleep_state().await;
		ctx.track_shutdown_task(async move {
			panic!("post-teardown shutdown task should never spawn");
		});
		yield_now().await;

		assert_eq!(ctx.shutdown_task_count(), 0);
		assert_eq!(warning_count.load(Ordering::SeqCst), 1);
	}

	#[tokio::test(start_paused = true)]
	async fn sleep_then_destroy_signal_tasks_do_not_leak_after_teardown() {
		let ctx = ActorContext::new_for_sleep_tests("actor-sleep-destroy");

		ctx.sleep();
		ctx.destroy();

		assert_eq!(
			ctx.shutdown_task_count(),
			2,
			"sleep and destroy bridge work should be tracked before it runs"
		);

		ctx.teardown_sleep_state().await;
		advance(Duration::from_millis(1)).await;

		assert_eq!(ctx.shutdown_task_count(), 0);
	}

	#[tokio::test(start_paused = true)]
	async fn sleep_idle_window_without_work_returns_next_tick() {
		let ctx = ActorContext::new_for_sleep_tests("actor-sleep-idle");

		let waiter = tokio::spawn({
			let ctx = ctx.clone();
			async move {
				ctx.wait_for_sleep_idle_window(Instant::now() + Duration::from_secs(1))
					.await
			}
		});

		yield_now().await;

		assert!(
			waiter.is_finished(),
			"idle wait should not poll in 10ms slices"
		);
		assert!(waiter.await.expect("idle waiter should join"));
	}

	#[tokio::test(start_paused = true)]
	async fn sleep_idle_window_waits_for_http_counter_zero_transition() {
		let ctx = ActorContext::new_for_sleep_tests("actor-http-idle");
		let counter = Arc::new(AsyncCounter::new());
		counter.register_zero_notify(&ctx.0.sleep.work.idle_notify);
		counter.register_change_notify(&ctx.sleep_activity_notify());
		*ctx.0.sleep.http_request_counter.lock() = Some(counter.clone());

		counter.increment();
		let waiter = tokio::spawn({
			let ctx = ctx.clone();
			async move {
				ctx.wait_for_sleep_idle_window(Instant::now() + Duration::from_secs(1))
					.await
			}
		});

		yield_now().await;
		assert!(
			!waiter.is_finished(),
			"http request drain should stay blocked while the counter is non-zero"
		);

		counter.decrement();
		advance(Duration::from_millis(1)).await;
		yield_now().await;
		assert!(waiter.await.expect("http idle waiter should join"));
	}

	#[tokio::test(start_paused = true)]
	async fn http_request_idle_wait_uses_zero_notify() {
		let ctx = ActorContext::new_for_sleep_tests("actor-http-zero-notify");
		let counter = Arc::new(AsyncCounter::new());
		counter.register_zero_notify(&ctx.0.sleep.work.idle_notify);
		*ctx.0.sleep.http_request_counter.lock() = Some(counter.clone());

		counter.increment();
		let waiter = tokio::spawn({
			let ctx = ctx.clone();
			async move {
				ctx.wait_for_http_requests_idle().await;
			}
		});

		yield_now().await;
		assert!(
			!waiter.is_finished(),
			"http request idle wait should block while the counter is non-zero"
		);

		counter.decrement();
		yield_now().await;

		assert!(
			waiter.is_finished(),
			"http request idle wait should wake on the zero notification"
		);
		waiter.await.expect("http idle waiter should join");
	}

	#[tokio::test(start_paused = true)]
	async fn sleep_idle_window_waits_for_websocket_callback_zero_transition() {
		let ctx = ActorContext::new_for_sleep_tests("actor-websocket-idle");
		let guard = ctx.websocket_callback_region();

		let waiter = tokio::spawn({
			let ctx = ctx.clone();
			async move {
				ctx.wait_for_sleep_idle_window(Instant::now() + Duration::from_secs(1))
					.await
			}
		});

		yield_now().await;
		assert!(
			!waiter.is_finished(),
			"websocket callback drain should stay blocked while the counter is non-zero"
		);

		drop(guard);
		advance(Duration::from_millis(1)).await;
		yield_now().await;
		assert!(waiter.await.expect("websocket idle waiter should join"));
	}
}
