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
		ctx.set_started(true);

		ctx.sleep()
			.expect("sleep should succeed after started is set");
		ctx.destroy()
			.expect("destroy should succeed after started is set");

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

	#[tokio::test(start_paused = true)]
	async fn sleep_before_started_errors_with_actor_starting() {
		let ctx = ActorContext::new_for_sleep_tests("actor-sleep-before-started");

		let err = ctx
			.sleep()
			.expect_err("sleep should fail before started is set");
		let rivet_err = rivet_error::RivetError::extract(&err);
		assert_eq!(rivet_err.group(), "actor");
		assert_eq!(rivet_err.code(), "starting");
	}

	#[tokio::test(start_paused = true)]
	async fn destroy_before_started_errors_with_actor_starting() {
		let ctx = ActorContext::new_for_sleep_tests("actor-destroy-before-started");

		let err = ctx
			.destroy()
			.expect_err("destroy should fail before started is set");
		let rivet_err = rivet_error::RivetError::extract(&err);
		assert_eq!(rivet_err.group(), "actor");
		assert_eq!(rivet_err.code(), "starting");
	}

	#[tokio::test(start_paused = true)]
	async fn double_sleep_errors_with_actor_stopping() {
		let ctx = ActorContext::new_for_sleep_tests("actor-double-sleep");
		ctx.set_started(true);

		ctx.sleep()
			.expect("first sleep call should be accepted after startup");

		let err = ctx
			.sleep()
			.expect_err("second sleep call should fail as already requested");
		let rivet_err = rivet_error::RivetError::extract(&err);
		assert_eq!(rivet_err.group(), "actor");
		assert_eq!(rivet_err.code(), "stopping");
	}

	#[tokio::test(start_paused = true)]
	async fn double_destroy_errors_with_actor_stopping() {
		let ctx = ActorContext::new_for_sleep_tests("actor-double-destroy");
		ctx.set_started(true);

		ctx.destroy()
			.expect("first destroy call should be accepted after startup");

		let err = ctx
			.destroy()
			.expect_err("second destroy call should fail as already requested");
		let rivet_err = rivet_error::RivetError::extract(&err);
		assert_eq!(rivet_err.group(), "actor");
		assert_eq!(rivet_err.code(), "stopping");
	}

	// `set_prevent_sleep` is a deprecated no-op kept for NAPI bridge
	// compatibility. The exhaustive `CanSleep` match below is a build-time
	// guard against reintroducing a `PreventSleep` enum variant.
	#[tokio::test(start_paused = true)]
	#[allow(deprecated)]
	async fn set_prevent_sleep_is_a_deprecated_noop() {
		use crate::actor::sleep::CanSleep;

		let ctx = ActorContext::new_for_sleep_tests("actor-prevent-sleep-noop");
		ctx.set_started(true);

		ctx.set_prevent_sleep(true);
		match ctx.can_sleep().await {
			CanSleep::Yes
			| CanSleep::NotReady
			| CanSleep::NoSleep
			| CanSleep::ActiveHttpRequests
			| CanSleep::ActiveKeepAwake
			| CanSleep::ActiveInternalKeepAwake
			| CanSleep::ActiveRunHandler
			| CanSleep::ActiveDisconnectCallbacks
			| CanSleep::ActiveConnections
			| CanSleep::ActiveWebSocketCallbacks => {}
		}

		ctx.set_prevent_sleep(false);
	}

	#[tokio::test(start_paused = true)]
	async fn shutdown_deadline_token_aborts_select_awaiting_task() {
		// Mirrors the NAPI `RunGracefulCleanup` pattern: a task awaits user
		// work and the shutdown_deadline cancellation in a `tokio::select!`.
		// If `cancel_shutdown_deadline()` does not propagate to clones of the
		// token, the spawned task would hang and the test would time out.
		let ctx = ActorContext::new_for_sleep_tests("actor-shutdown-deadline");
		let token = ctx.shutdown_deadline_token();
		assert!(!token.is_cancelled());

		let aborted = Arc::new(std::sync::atomic::AtomicBool::new(false));
		let aborted_clone = aborted.clone();
		let task = tokio::spawn(async move {
			tokio::select! {
				_ = token.cancelled() => {
					aborted_clone.store(true, Ordering::SeqCst);
				}
				_ = futures::future::pending::<()>() => {}
			}
		});

		yield_now().await;
		assert!(!aborted.load(Ordering::SeqCst));

		ctx.cancel_shutdown_deadline();
		task.await.expect("select task should join after cancel");
		assert!(
			aborted.load(Ordering::SeqCst),
			"select-awaiting task must observe cancel via the cloned token"
		);
	}

	#[tokio::test(start_paused = true)]
	async fn sleep_after_grace_clears_started_returns_stopping_not_starting() {
		// Simulate the lifecycle state machine clearing `started` when it
		// transitions into SleepGrace. Calls into `sleep()` after that point
		// must surface `Stopping`, not `Starting`.
		let ctx = ActorContext::new_for_sleep_tests("actor-sleep-after-grace");
		ctx.set_started(true);

		ctx.sleep().expect("first sleep call should be accepted");

		// Lifecycle machine clears `started` on transition into SleepGrace.
		ctx.set_started(false);

		let err = ctx.sleep().expect_err("second sleep should fail");
		let rivet_err = rivet_error::RivetError::extract(&err);
		assert_eq!(rivet_err.group(), "actor");
		assert_eq!(
			rivet_err.code(),
			"stopping",
			"started=false during shutdown must surface stopping, not starting"
		);
	}
}
