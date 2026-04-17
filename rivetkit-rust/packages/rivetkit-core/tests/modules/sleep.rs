use super::*;

mod moved_tests {
	use std::time::Duration;

	use super::{CanSleep, SleepController};
	use crate::actor::context::ActorContext;
	use crate::ActorConfig;

	#[tokio::test]
	async fn can_sleep_requires_ready_and_started() {
		let ctx = ActorContext::default();

		assert_eq!(ctx.can_sleep().await, CanSleep::NotReady);

		ctx.set_ready(true);
		assert_eq!(ctx.can_sleep().await, CanSleep::NotReady);

		ctx.set_started(true);
		assert_eq!(ctx.can_sleep().await, CanSleep::Yes);
	}

	#[tokio::test]
	async fn can_sleep_blocks_for_active_regions_and_run_handler() {
		let ctx = ActorContext::default();
		ctx.set_ready(true);
		ctx.set_started(true);

		ctx.begin_keep_awake();
		assert_eq!(ctx.can_sleep().await, CanSleep::ActiveKeepAwake);
		ctx.end_keep_awake();

		ctx.begin_internal_keep_awake();
		assert_eq!(ctx.can_sleep().await, CanSleep::ActiveInternalKeepAwake);
		ctx.end_internal_keep_awake();

		ctx.set_run_handler_active(true);
		assert_eq!(ctx.can_sleep().await, CanSleep::ActiveRun);
		ctx.set_run_handler_active(false);

		assert_eq!(ctx.can_sleep().await, CanSleep::Yes);
	}

	#[tokio::test]
	async fn can_sleep_allows_run_handler_when_only_blocked_on_queue_wait() {
		let ctx = ActorContext::default();
		ctx.set_ready(true);
		ctx.set_started(true);
		ctx.set_run_handler_active(true);
		ctx.queue().set_wait_activity_callback(None);
		crate::actor::queue::tests::begin_sleep_test_wait(ctx.queue());

		assert_eq!(ctx.can_sleep().await, CanSleep::Yes);

		crate::actor::queue::tests::end_sleep_test_wait(ctx.queue());
	}

	#[tokio::test]
	async fn can_sleep_blocks_for_connections_disconnects_and_websocket_callbacks() {
		let ctx = ActorContext::default();
		ctx.set_ready(true);
		ctx.set_started(true);

		let conn = crate::ConnHandle::new("conn-1", Vec::new(), Vec::new(), false);
		ctx.add_conn(conn);
		assert_eq!(ctx.can_sleep().await, CanSleep::ActiveConnections);
		ctx.remove_conn("conn-1");

		ctx.begin_pending_disconnect();
		assert_eq!(ctx.can_sleep().await, CanSleep::PendingDisconnectCallbacks);
		ctx.end_pending_disconnect();

		ctx.begin_websocket_callback();
		assert_eq!(ctx.can_sleep().await, CanSleep::ActiveWebSocketCallbacks);
		ctx.end_websocket_callback();

		assert_eq!(ctx.can_sleep().await, CanSleep::Yes);
	}

	#[tokio::test]
	async fn reset_sleep_timer_requests_sleep_after_idle_timeout() {
		let ctx = ActorContext::default();
		ctx.configure_sleep(ActorConfig {
			sleep_timeout: Duration::from_millis(10),
			..ActorConfig::default()
		});
		ctx.set_ready(true);
		ctx.set_started(true);

		ctx.reset_sleep_timer();
		tokio::time::sleep(Duration::from_millis(25)).await;

		assert!(ctx.sleep_requested());
	}

	#[test]
	fn controller_tracks_shutdown_tasks() {
		let controller = SleepController::default();
		let runtime = tokio::runtime::Builder::new_current_thread()
			.enable_all()
			.build()
			.expect("runtime should build");

		runtime.block_on(async {
			let handle = tokio::spawn(async {});
			controller.track_shutdown_task(handle);
			assert_eq!(controller.shutdown_task_count(), 1);
			tokio::task::yield_now().await;
			assert_eq!(controller.shutdown_task_count(), 0);
		});
	}
}
