mod moved_tests {
	use std::sync::atomic::{AtomicUsize, Ordering};
	use std::sync::{Arc, Mutex};
	use std::time::Duration;

	use tokio::sync::{mpsc, oneshot};
	use tokio::task::yield_now;
	use tokio::time::advance;

	use crate::actor::messages::{ActorEvent, StateDelta};
	use crate::actor::preload::PreloadedPersistedActor;
	use crate::actor::state::PersistedActor;
	use crate::actor::task::{ActorTask, LifecycleCommand};
	use crate::actor::task_types::ShutdownKind;
	use crate::{ActorConfig, ActorContext, ActorFactory};

	fn new_task_with_factory(ctx: ActorContext, factory: Arc<ActorFactory>) -> ActorTask {
		let (_lifecycle_tx, lifecycle_rx) = mpsc::channel(4);
		let (_dispatch_tx, dispatch_rx) = mpsc::channel(4);
		let (_events_tx, events_rx) = mpsc::channel(4);
		ActorTask::new(
			ctx.actor_id().to_owned(),
			0,
			lifecycle_rx,
			dispatch_rx,
			events_rx,
			factory,
			ctx,
			None,
			None,
		)
	}

	struct NotifyOnDrop(Mutex<Option<oneshot::Sender<()>>>);

	impl NotifyOnDrop {
		fn new(sender: oneshot::Sender<()>) -> Self {
			Self(Mutex::new(Some(sender)))
		}
	}

	impl Drop for NotifyOnDrop {
		fn drop(&mut self) {
			if let Some(sender) = self.0.lock().expect("drop notify lock poisoned").take() {
				let _ = sender.send(());
			}
		}
	}

	#[tokio::test]
	async fn startup_loads_preloaded_state_and_input_before_run_handler() {
		let ctx = crate::actor::context::tests::new_with_kv(
			"actor-preloaded-startup",
			"task-lifecycle",
			Vec::new(),
			"local",
			crate::kv::tests::new_in_memory(),
		);
		let (observed_send, observed_rx) = oneshot::channel();
		let observed_tx = Arc::new(Mutex::new(Some(observed_send)));

		let factory = Arc::new(ActorFactory::new(Default::default(), move |start| {
			let observed_tx = observed_tx.clone();
			Box::pin(async move {
				let mut events = start.events;
				observed_tx
					.lock()
					.expect("observed lock poisoned")
					.take()
					.expect("observed sender should exist")
					.send((
						start.input,
						start.snapshot,
						start.ctx.state(),
						start.ctx.persisted_actor().has_initialized,
						events.try_recv().is_none(),
					))
					.expect("startup observation should send");

				while let Some(event) = events.recv().await {
					match event {
						ActorEvent::SerializeState { reply, .. } => {
							reply.send(Ok(vec![StateDelta::ActorState(start.ctx.state())]));
						}
						ActorEvent::RunGracefulCleanup { reply, .. } => {
							reply.send(Ok(()));
						}
						_ => {}
					}
				}
				Ok(())
			})
		}));
		let mut task = ActorTask::new(
			"actor-preloaded-startup".to_owned(),
			0,
			mpsc::channel(4).1,
			mpsc::channel(4).1,
			mpsc::channel(4).1,
			factory,
			ctx.clone(),
			Some(vec![7, 7, 7]),
			None,
		)
		.with_preloaded_persisted_actor(PreloadedPersistedActor::Some(PersistedActor {
			input: Some(vec![1, 2, 3]),
			has_initialized: false,
			state: vec![9, 8, 7],
			scheduled_events: Vec::new(),
		}));

		let (start_tx, start_rx) = oneshot::channel();
		task.handle_lifecycle(LifecycleCommand::Start { reply: start_tx })
			.await;
		start_rx
			.await
			.expect("start reply should send")
			.expect("start should succeed");

		let (input, snapshot, state, has_initialized, no_initial_event) =
			observed_rx.await.expect("startup observation should send");
		assert_eq!(input, Some(vec![1, 2, 3]));
		assert_eq!(snapshot, None);
		assert_eq!(state, vec![9, 8, 7]);
		assert!(has_initialized);
		assert!(no_initial_event);

		let run_handle = task.run_handle.take().expect("run handle should exist");
		run_handle.abort();
		let _ = run_handle.await;
	}

	#[tokio::test(start_paused = true)]
	async fn sleep_shutdown_aborts_stuck_run_handler_at_grace_deadline() {
		let ctx = crate::actor::context::tests::new_with_kv(
			"actor-stuck-run-grace",
			"task-lifecycle",
			Vec::new(),
			"local",
			crate::kv::tests::new_in_memory(),
		);
		let grace_period = Duration::from_millis(100);
		let dropped_count = Arc::new(AtomicUsize::new(0));
		let (drop_tx, drop_rx) = oneshot::channel();
		let drop_tx = Arc::new(Mutex::new(Some(drop_tx)));
		let dropped_count_for_factory = dropped_count.clone();
		let factory = Arc::new(ActorFactory::new(
			ActorConfig {
				sleep_grace_period: grace_period,
				sleep_grace_period_overridden: true,
				..ActorConfig::default()
			},
			move |_start| {
				let dropped_count = dropped_count_for_factory.clone();
				let drop_tx = drop_tx.clone();
				Box::pin(async move {
					let sender = drop_tx
						.lock()
						.expect("drop sender lock poisoned")
						.take()
						.expect("drop sender should exist");
					let _notify = NotifyOnDrop::new(sender);
					dropped_count.fetch_add(1, Ordering::SeqCst);
					std::future::pending::<()>().await;
					Ok(())
				})
			},
		));
		let mut task = new_task_with_factory(ctx, factory);

		let (start_tx, start_rx) = oneshot::channel();
		task.handle_lifecycle(LifecycleCommand::Start { reply: start_tx })
			.await;
		start_rx
			.await
			.expect("start reply should send")
			.expect("start should succeed");
		yield_now().await;
		assert_eq!(dropped_count.load(Ordering::SeqCst), 1);

		let stop = tokio::spawn(async move { task.handle_stop(ShutdownKind::Sleep).await });
		yield_now().await;
		assert!(
			!stop.is_finished(),
			"sleep shutdown should wait for the shared grace deadline"
		);

		advance(grace_period - Duration::from_millis(1)).await;
		yield_now().await;
		assert!(
			!stop.is_finished(),
			"stuck run handler should not be aborted before the grace deadline"
		);

		advance(Duration::from_millis(1)).await;
		stop.await
			.expect("sleep stop join should succeed")
			.expect("sleep stop should succeed after grace timeout");
		drop_rx
			.await
			.expect("stuck run handler should be aborted at the grace deadline");
	}
}
