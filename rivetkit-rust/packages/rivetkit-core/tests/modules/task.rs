mod moved_tests {
	use std::collections::{BTreeMap, HashMap};
	use std::path::PathBuf;
	use std::process::Command;
	use std::sync::Arc;
	use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
	use std::sync::{Mutex, OnceLock};
	use std::task::Poll;
	use std::time::Duration;

	use futures::{FutureExt, poll};
	use rivet_envoy_client::config::{
		BoxFuture as EnvoyBoxFuture, EnvoyCallbacks, EnvoyConfig, HttpRequest, HttpResponse,
		WebSocketHandler, WebSocketSender,
	};
	use rivet_envoy_client::context::{SharedContext, WsTxMessage};
	use rivet_envoy_client::envoy::ToEnvoyMessage;
	use rivet_envoy_client::handle::EnvoyHandle;
	use rivet_envoy_client::protocol;
	use tokio::sync::{Mutex as AsyncMutex, mpsc, oneshot};
	use tokio::task::yield_now;
	use tokio::time::{Instant, advance, sleep, timeout};
	use tracing::field::{Field, Visit};
	use tracing::{Event, Subscriber};
	use tracing_subscriber::layer::{Context as LayerContext, Layer};
	use tracing_subscriber::prelude::*;
	use tracing_subscriber::registry::Registry;

	use crate::actor::connection::{
		ConnHandle, HibernatableConnectionMetadata, decode_persisted_connection,
		make_connection_key,
	};
	use crate::actor::context::tests::new_with_kv;
	use crate::actor::messages::{ActorEvent, SerializeStateReason, StateDelta};
	use crate::actor::preload::PreloadedPersistedActor;
	use crate::actor::state::{
		LAST_PUSHED_ALARM_KEY, PERSIST_DATA_KEY, PersistedActor, PersistedScheduleEvent,
		RequestSaveOpts, decode_last_pushed_alarm, decode_persisted_actor,
		encode_last_pushed_alarm, encode_persisted_actor,
	};
	use crate::actor::task::{
		ActorTask, DispatchCommand, LONG_SHUTDOWN_DRAIN_WARNING_THRESHOLD, LifecycleCommand,
		LifecycleEvent, LifecycleState, LiveExit,
	};
	use crate::actor::task_types::ShutdownKind;
	use crate::kv::tests::new_in_memory;
	use crate::{ActorConfig, ActorContext, ActorFactory};

	fn test_hook_lock() -> &'static AsyncMutex<()> {
		static LOCK: OnceLock<AsyncMutex<()>> = OnceLock::new();
		LOCK.get_or_init(|| AsyncMutex::new(()))
	}

	async fn wait_for_count(counter: &AtomicUsize, expected: usize) {
		for _ in 0..50 {
			if counter.load(Ordering::SeqCst) >= expected {
				return;
			}
			sleep(Duration::from_millis(10)).await;
		}

		assert_eq!(counter.load(Ordering::SeqCst), expected);
	}

	async fn wait_for_state(ctx: &ActorContext, expected: &[u8]) {
		for _ in 0..50 {
			if ctx.state() == expected {
				return;
			}
			sleep(Duration::from_millis(10)).await;
		}

		assert_eq!(ctx.state(), expected);
	}

	async fn drain_lifecycle_events(task: &mut ActorTask) {
		while let Ok(event) = task.lifecycle_events.try_recv() {
			task.handle_event(event).await;
		}
	}

	fn save_tick_factory(save_ticks: Arc<AtomicUsize>) -> Arc<ActorFactory> {
		Arc::new(ActorFactory::new(
			ActorConfig {
				state_save_interval: Duration::from_millis(50),
				..ActorConfig::default()
			},
			move |start| {
				let save_ticks = save_ticks.clone();
				Box::pin(async move {
					let mut events = start.events;
					while let Some(event) = events.recv().await {
						match event {
							ActorEvent::SerializeState {
								reason: SerializeStateReason::Save,
								reply,
							} => {
								let next = save_ticks.fetch_add(1, Ordering::SeqCst) + 1;
								reply.send(Ok(vec![StateDelta::ActorState(vec![next as u8])]));
							}
							ActorEvent::BeginSleep => {}
							ActorEvent::FinalizeSleep { reply } | ActorEvent::Destroy { reply } => {
								reply.send(Ok(()));
								break;
							}
							_ => {}
						}
					}
					Ok(())
				})
			},
		))
	}

	fn noop_factory() -> Arc<ActorFactory> {
		Arc::new(ActorFactory::new(Default::default(), |_start| {
			Box::pin(async move { Ok(()) })
		}))
	}

	fn new_task(ctx: ActorContext) -> ActorTask {
		new_task_with_factory(ctx, noop_factory())
	}

	fn new_task_with_senders(
		ctx: ActorContext,
	) -> (
		ActorTask,
		mpsc::Sender<LifecycleCommand>,
		mpsc::Sender<DispatchCommand>,
		mpsc::Sender<LifecycleEvent>,
	) {
		let (lifecycle_tx, lifecycle_rx) = mpsc::channel(4);
		let (dispatch_tx, dispatch_rx) = mpsc::channel(4);
		let (events_tx, events_rx) = mpsc::channel(4);
		(
			ActorTask::new(
				"actor-drain".into(),
				0,
				lifecycle_rx,
				dispatch_rx,
				events_rx,
				noop_factory(),
				ctx,
				None,
				None,
			),
			lifecycle_tx,
			dispatch_tx,
			events_tx,
		)
	}

	fn new_task_with_factory(ctx: ActorContext, factory: Arc<ActorFactory>) -> ActorTask {
		let (_lifecycle_tx, lifecycle_rx) = mpsc::channel(4);
		let (_dispatch_tx, dispatch_rx) = mpsc::channel(4);
		let (_events_tx, events_rx) = mpsc::channel(4);
		ActorTask::new(
			"actor-drain".into(),
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

	struct IdleEnvoyCallbacks;

	impl EnvoyCallbacks for IdleEnvoyCallbacks {
		fn on_actor_start(
			&self,
			_handle: EnvoyHandle,
			_actor_id: String,
			_generation: u32,
			_config: protocol::ActorConfig,
			_preloaded_kv: Option<protocol::PreloadedKv>,
			_sqlite_startup_data: Option<protocol::SqliteStartupData>,
		) -> EnvoyBoxFuture<anyhow::Result<()>> {
			Box::pin(async { Ok(()) })
		}

		fn on_shutdown(&self) {}

		fn fetch(
			&self,
			_handle: EnvoyHandle,
			_actor_id: String,
			_gateway_id: protocol::GatewayId,
			_request_id: protocol::RequestId,
			_request: HttpRequest,
		) -> EnvoyBoxFuture<anyhow::Result<HttpResponse>> {
			Box::pin(async { anyhow::bail!("fetch should not run in task tests") })
		}

		fn websocket(
			&self,
			_handle: EnvoyHandle,
			_actor_id: String,
			_gateway_id: protocol::GatewayId,
			_request_id: protocol::RequestId,
			_request: HttpRequest,
			_path: String,
			_headers: HashMap<String, String>,
			_is_hibernatable: bool,
			_is_restoring_hibernatable: bool,
			_sender: WebSocketSender,
		) -> EnvoyBoxFuture<anyhow::Result<WebSocketHandler>> {
			Box::pin(async { anyhow::bail!("websocket should not run in task tests") })
		}

		fn can_hibernate(
			&self,
			_actor_id: &str,
			_gateway_id: &protocol::GatewayId,
			_request_id: &protocol::RequestId,
			_request: &HttpRequest,
		) -> EnvoyBoxFuture<anyhow::Result<bool>> {
			Box::pin(async { Ok(false) })
		}
	}

	fn test_envoy_handle() -> (EnvoyHandle, mpsc::UnboundedReceiver<ToEnvoyMessage>) {
		let (envoy_tx, envoy_rx) = mpsc::unbounded_channel();
		let shared = Arc::new(SharedContext {
			config: EnvoyConfig {
				version: 1,
				endpoint: "http://127.0.0.1:1".to_string(),
				token: None,
				namespace: "test".to_string(),
				pool_name: "test".to_string(),
				prepopulate_actor_names: HashMap::new(),
				metadata: None,
				not_global: true,
				debug_latency_ms: None,
				callbacks: Arc::new(IdleEnvoyCallbacks),
			},
			envoy_key: "test-envoy".to_string(),
			envoy_tx,
			actors: Arc::new(Mutex::new(HashMap::new())),
			live_tunnel_requests: Arc::new(Mutex::new(HashMap::new())),
			pending_hibernation_restores: Arc::new(Mutex::new(HashMap::new())),
			ws_tx: Arc::new(tokio::sync::Mutex::new(
				None::<mpsc::UnboundedSender<WsTxMessage>>,
			)),
			protocol_metadata: Arc::new(tokio::sync::Mutex::new(None)),
			shutting_down: AtomicBool::new(false),
			stopped_tx: tokio::sync::watch::channel(true).0,
		});

		(EnvoyHandle::from_shared(shared), envoy_rx)
	}

	fn recv_alarm_now(
		rx: &mut mpsc::UnboundedReceiver<ToEnvoyMessage>,
		expected_actor_id: &str,
		expected_generation: Option<u32>,
	) -> Option<i64> {
		match rx.try_recv() {
			Ok(ToEnvoyMessage::SetAlarm {
				actor_id,
				generation,
				alarm_ts,
				ack_tx,
			}) => {
				assert_eq!(actor_id, expected_actor_id);
				assert_eq!(generation, expected_generation);
				if let Some(ack_tx) = ack_tx {
					let _ = ack_tx.send(());
				}
				alarm_ts
			}
			Ok(_) => panic!("expected set_alarm envoy message"),
			Err(error) => panic!("expected set_alarm envoy message, got {error:?}"),
		}
	}

	fn assert_no_alarm(rx: &mut mpsc::UnboundedReceiver<ToEnvoyMessage>) {
		assert!(matches!(
			rx.try_recv(),
			Err(mpsc::error::TryRecvError::Empty)
		));
	}

	fn shutdown_ack_factory(config: ActorConfig) -> Arc<ActorFactory> {
		Arc::new(ActorFactory::new(config, move |start| {
			Box::pin(async move {
				let mut events = start.events;
				while let Some(event) = events.recv().await {
					match event {
						ActorEvent::SerializeState { reply, .. } => {
							reply.send(Ok(Vec::new()));
						}
						ActorEvent::RunGracefulCleanup { reply, .. } => {
							reply.send(Ok(()));
							break;
						}
						_ => {}
					}
				}
				Ok(())
			})
		}))
	}

	fn detached_cleanup_after_clean_run_factory(
		sleep_count: Arc<AtomicUsize>,
		destroy_count: Arc<AtomicUsize>,
		run_returned_tx: oneshot::Sender<()>,
		cleanup_tx: oneshot::Sender<ShutdownKind>,
	) -> Arc<ActorFactory> {
		let run_returned_tx = Arc::new(Mutex::new(Some(run_returned_tx)));
		let cleanup_tx = Arc::new(Mutex::new(Some(cleanup_tx)));
		Arc::new(ActorFactory::new(ActorConfig::default(), move |start| {
			let sleep_count = sleep_count.clone();
			let destroy_count = destroy_count.clone();
			let run_returned_tx = run_returned_tx.clone();
			let cleanup_tx = cleanup_tx.clone();
			Box::pin(async move {
				let mut events = start.events;
				tokio::spawn(async move {
					while let Some(event) = events.recv().await {
						match event {
							ActorEvent::SerializeState { reply, .. } => {
								reply.send(Ok(Vec::new()));
							}
							ActorEvent::RunGracefulCleanup { reason, reply } => {
								match reason {
									ShutdownKind::Sleep => {
										sleep_count.fetch_add(1, Ordering::SeqCst);
									}
									ShutdownKind::Destroy => {
										destroy_count.fetch_add(1, Ordering::SeqCst);
									}
								}
								reply.send(Ok(()));
								if let Some(tx) = cleanup_tx
									.lock()
									.expect("cleanup sender lock poisoned")
									.take()
								{
									let _ = tx.send(reason);
								}
								break;
							}
							_ => {}
						}
					}
				});
				if let Some(tx) = run_returned_tx
					.lock()
					.expect("run returned sender lock poisoned")
					.take()
				{
					let _ = tx.send(());
				}
				Ok(())
			})
		}))
	}

	fn sleep_grace_factory(
		config: ActorConfig,
		begin_sleep_count: Arc<AtomicUsize>,
		destroy_count: Arc<AtomicUsize>,
		action_count: Arc<AtomicUsize>,
	) -> Arc<ActorFactory> {
		Arc::new(ActorFactory::new(config, move |start| {
			let begin_sleep_count = begin_sleep_count.clone();
			let destroy_count = destroy_count.clone();
			let action_count = action_count.clone();
			Box::pin(async move {
				let mut events = start.events;
				while let Some(event) = events.recv().await {
					match event {
						ActorEvent::Action { reply, .. } => {
							action_count.fetch_add(1, Ordering::SeqCst);
							reply.send(Ok(vec![7, 7, 7]));
						}
						ActorEvent::RunGracefulCleanup { reason, reply } => {
							match reason {
								ShutdownKind::Sleep => {
									begin_sleep_count.fetch_add(1, Ordering::SeqCst);
								}
								ShutdownKind::Destroy => {
									destroy_count.fetch_add(1, Ordering::SeqCst);
								}
							}
							reply.send(Ok(()));
						}
						_ => {}
					}
				}
				Ok(())
			})
		}))
	}

	#[derive(Default)]
	struct MessageVisitor {
		message: Option<String>,
		channel: Option<String>,
		actor_id: Option<String>,
		reason: Option<String>,
		command: Option<String>,
		event: Option<String>,
		outcome: Option<String>,
		old: Option<String>,
		new: Option<String>,
	}

	impl Visit for MessageVisitor {
		fn record_str(&mut self, field: &Field, value: &str) {
			match field.name() {
				"message" => self.message = Some(value.to_owned()),
				"channel" => self.channel = Some(value.to_owned()),
				"actor_id" => self.actor_id = Some(value.to_owned()),
				"reason" => self.reason = Some(value.to_owned()),
				"command" => self.command = Some(value.to_owned()),
				"event" => self.event = Some(value.to_owned()),
				"outcome" => self.outcome = Some(value.to_owned()),
				"old" => self.old = Some(value.to_owned()),
				"new" => self.new = Some(value.to_owned()),
				_ => {}
			}
		}

		fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
			match field.name() {
				"message" => {
					self.message = Some(format!("{value:?}").trim_matches('"').to_owned());
				}
				"channel" => {
					self.channel = Some(format!("{value:?}").trim_matches('"').to_owned());
				}
				"actor_id" => {
					self.actor_id = Some(format!("{value:?}").trim_matches('"').to_owned());
				}
				"reason" => {
					self.reason = Some(format!("{value:?}").trim_matches('"').to_owned());
				}
				"command" => {
					self.command = Some(format!("{value:?}").trim_matches('"').to_owned());
				}
				"event" => {
					self.event = Some(format!("{value:?}").trim_matches('"').to_owned());
				}
				"outcome" => {
					self.outcome = Some(format!("{value:?}").trim_matches('"').to_owned());
				}
				"old" => {
					self.old = Some(format!("{value:?}").trim_matches('"').to_owned());
				}
				"new" => {
					self.new = Some(format!("{value:?}").trim_matches('"').to_owned());
				}
				_ => {}
			}
		}
	}

	#[derive(Clone, Debug)]
	struct ActorTaskLog {
		level: tracing::Level,
		actor_id: Option<String>,
		message: Option<String>,
		command: Option<String>,
		event: Option<String>,
		outcome: Option<String>,
		old: Option<String>,
		new: Option<String>,
	}

	#[derive(Clone, Debug, PartialEq, Eq)]
	struct ClosedChannelWarning {
		actor_id: String,
		channel: String,
		reason: String,
		message: String,
	}

	#[derive(Clone)]
	struct LongShutdownDrainWarningLayer {
		count: Arc<AtomicUsize>,
	}

	#[derive(Clone)]
	struct ShutdownTaskRefusedWarningLayer {
		count: Arc<AtomicUsize>,
	}

	#[derive(Clone)]
	struct ClosedChannelWarningLayer {
		records: Arc<Mutex<Vec<ClosedChannelWarning>>>,
	}

	#[derive(Clone)]
	struct ActorTaskLogLayer {
		records: Arc<Mutex<Vec<ActorTaskLog>>>,
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

	impl<S> Layer<S> for LongShutdownDrainWarningLayer
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
				== Some("actor shutdown drain is taking longer than expected")
			{
				self.count.fetch_add(1, Ordering::SeqCst);
			}
		}
	}

	impl<S> Layer<S> for ShutdownTaskRefusedWarningLayer
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

	impl<S> Layer<S> for ClosedChannelWarningLayer
	where
		S: Subscriber,
	{
		fn on_event(&self, event: &Event<'_>, _ctx: LayerContext<'_, S>) {
			if *event.metadata().level() != tracing::Level::WARN {
				return;
			}

			let mut visitor = MessageVisitor::default();
			event.record(&mut visitor);
			if visitor.reason.as_deref() != Some("all senders dropped") {
				return;
			}

			let Some(actor_id) = visitor.actor_id else {
				return;
			};
			let Some(channel) = visitor.channel else {
				return;
			};
			let Some(reason) = visitor.reason else {
				return;
			};
			let Some(message) = visitor.message else {
				return;
			};

			self.records
				.lock()
				.expect("closed-channel warning lock poisoned")
				.push(ClosedChannelWarning {
					actor_id,
					channel,
					reason,
					message,
				});
		}
	}

	impl<S> Layer<S> for ActorTaskLogLayer
	where
		S: Subscriber,
	{
		fn on_event(&self, event: &Event<'_>, _ctx: LayerContext<'_, S>) {
			let mut visitor = MessageVisitor::default();
			event.record(&mut visitor);
			self.records
				.lock()
				.expect("actor-task log lock poisoned")
				.push(ActorTaskLog {
					level: *event.metadata().level(),
					actor_id: visitor.actor_id,
					message: visitor.message,
					command: visitor.command,
					event: visitor.event,
					outcome: visitor.outcome,
					old: visitor.old,
					new: visitor.new,
				});
		}
	}

	async fn poll_until_ready(
		future: &mut std::pin::Pin<&mut impl std::future::Future<Output = bool>>,
	) -> bool {
		for _ in 0..5 {
			match poll!(future.as_mut()) {
				Poll::Ready(result) => return result,
				Poll::Pending => yield_now().await,
			}
		}

		panic!("future should be ready");
	}

	enum ClosedChannelCase {
		LifecycleInbox,
		LifecycleEvents,
		DispatchInbox,
	}

	impl ClosedChannelCase {
		fn actor_id(&self) -> &'static str {
			match self {
				Self::LifecycleInbox => "actor-channel-lifecycle",
				Self::LifecycleEvents => "actor-channel-events",
				Self::DispatchInbox => "actor-channel-dispatch",
			}
		}
	}

	async fn run_task_with_closed_channel(case: ClosedChannelCase) -> Vec<ClosedChannelWarning> {
		let _hook_lock = test_hook_lock().lock().await;
		let ctx = new_with_kv(
			case.actor_id(),
			"task-run",
			Vec::new(),
			"local",
			new_in_memory(),
		);
		let (mut task, lifecycle_tx, dispatch_tx, events_tx) = new_task_with_senders(ctx);
		let warnings = Arc::new(Mutex::new(Vec::new()));
		let subscriber = Registry::default().with(ClosedChannelWarningLayer {
			records: warnings.clone(),
		});
		let _guard = tracing::subscriber::set_default(subscriber);

		match case {
			ClosedChannelCase::LifecycleInbox => drop(lifecycle_tx),
			ClosedChannelCase::LifecycleEvents => drop(events_tx),
			ClosedChannelCase::DispatchInbox => {
				task.lifecycle = crate::actor::task::LifecycleState::Started;
				drop(dispatch_tx);
			}
		}

		let run = task.run();
		tokio::pin!(run);
		let Poll::Ready(result) = poll!(run.as_mut()) else {
			panic!("task run should exit immediately after the channel closes");
		};
		result.expect("task run should exit cleanly");

		warnings
			.lock()
			.expect("closed-channel warnings lock poisoned")
			.clone()
	}

	fn managed_test_conn(
		ctx: &ActorContext,
		id: &str,
		is_hibernatable: bool,
		disconnects: Arc<Mutex<Vec<String>>>,
	) -> ConnHandle {
		let conn = ConnHandle::new(id, Vec::new(), Vec::new(), is_hibernatable);
		let ctx = ctx.clone();
		let conn_id = id.to_owned();
		conn.configure_disconnect_handler(Some(Arc::new(move |_reason| {
			let ctx = ctx.clone();
			let conn_id = conn_id.clone();
			let disconnects = disconnects.clone();
			Box::pin(async move {
				disconnects
					.lock()
					.expect("disconnect log lock poisoned")
					.push(conn_id.clone());
				if is_hibernatable {
					ctx.request_hibernation_transport_removal(conn_id.clone());
				}
				ctx.remove_conn(&conn_id);
				Ok(())
			})
		})));
		conn.configure_transport_disconnect_handler(conn.managed_disconnect_handler().ok());
		conn
	}

	fn configure_live_hibernated_pairs(
		ctx: &ActorContext,
		pairs: impl IntoIterator<Item = (&'static [u8], &'static [u8])>,
	) {
		ctx.set_hibernated_connection_liveness_override(
			pairs
				.into_iter()
				.map(|(gateway_id, request_id)| (gateway_id.to_vec(), request_id.to_vec())),
		);
	}

	#[tokio::test]
	async fn save_tick_respects_debounce_and_immediate_requests() {
		let ctx = new_with_kv("actor-1", "task-save", Vec::new(), "local", new_in_memory());
		let (_lifecycle_tx, lifecycle_rx) = mpsc::channel(4);
		let (_dispatch_tx, dispatch_rx) = mpsc::channel(4);
		let (events_tx, events_rx) = mpsc::channel(4);
		ctx.configure_lifecycle_events(Some(events_tx));

		let save_ticks = Arc::new(AtomicUsize::new(0));
		let mut task = ActorTask::new(
			"actor-1".into(),
			0,
			lifecycle_rx,
			dispatch_rx,
			events_rx,
			save_tick_factory(save_ticks.clone()),
			ctx.clone(),
			None,
			None,
		);

		let (start_tx, start_rx) = oneshot::channel();
		task.handle_lifecycle(LifecycleCommand::Start { reply: start_tx })
			.await;
		start_rx
			.await
			.expect("start reply should send")
			.expect("start should succeed");

		ctx.request_save(RequestSaveOpts::default());
		task.handle_event(crate::actor::task::LifecycleEvent::SaveRequested { immediate: false })
			.await;
		let debounce_deadline = task
			.state_save_deadline
			.expect("debounced save deadline should exist");
		assert!(debounce_deadline > tokio::time::Instant::now());
		sleep(Duration::from_millis(20)).await;
		assert_eq!(save_ticks.load(Ordering::SeqCst), 0);

		sleep(Duration::from_millis(40)).await;
		task.on_state_save_tick().await;
		wait_for_count(&save_ticks, 1).await;
		wait_for_state(&ctx, &[1]).await;

		ctx.request_save(RequestSaveOpts {
			immediate: true,
			max_wait_ms: None,
		});
		task.handle_event(crate::actor::task::LifecycleEvent::SaveRequested { immediate: true })
			.await;
		let immediate_deadline = task
			.state_save_deadline
			.expect("immediate save deadline should exist");
		assert!(immediate_deadline <= tokio::time::Instant::now() + Duration::from_millis(5));
		task.on_state_save_tick().await;
		wait_for_count(&save_ticks, 2).await;
		wait_for_state(&ctx, &[2]).await;

		task.handle_stop(crate::actor::task_types::ShutdownKind::Destroy)
			.await
			.expect("stop should succeed");
	}

	#[tokio::test]
	async fn inspector_attach_threshold_arms_and_clears_serialize_debounce() {
		let ctx = new_with_kv(
			"actor-inspector",
			"task-inspector",
			Vec::new(),
			"local",
			new_in_memory(),
		);
		let (_lifecycle_tx, lifecycle_rx) = mpsc::channel(4);
		let (_dispatch_tx, dispatch_rx) = mpsc::channel(4);
		let (events_tx, events_rx) = mpsc::channel(4);
		ctx.configure_lifecycle_events(Some(events_tx));

		let save_ticks = Arc::new(AtomicUsize::new(0));
		let mut task = ActorTask::new(
			"actor-inspector".into(),
			0,
			lifecycle_rx,
			dispatch_rx,
			events_rx,
			save_tick_factory(save_ticks),
			ctx.clone(),
			None,
			None,
		);

		let (start_tx, start_rx) = oneshot::channel();
		task.handle_lifecycle(LifecycleCommand::Start { reply: start_tx })
			.await;
		start_rx
			.await
			.expect("start reply should send")
			.expect("start should succeed");

		ctx.request_save(RequestSaveOpts::default());
		drain_lifecycle_events(&mut task).await;
		assert!(task.state_save_deadline.is_some());
		assert!(task.inspector_serialize_state_deadline.is_none());

		let inspector_guard = ctx
			.inspector_attach()
			.expect("inspector runtime should be configured");
		drain_lifecycle_events(&mut task).await;
		assert_eq!(ctx.inspector_attach_count(), 1);
		assert!(task.inspector_serialize_state_deadline.is_some());

		drop(inspector_guard);
		drain_lifecycle_events(&mut task).await;
		assert_eq!(ctx.inspector_attach_count(), 0);
		assert!(task.inspector_serialize_state_deadline.is_none());

		task.handle_stop(ShutdownKind::Destroy)
			.await
			.expect("destroy stop should succeed");
	}

	#[tokio::test]
	async fn inspector_serialize_tick_broadcasts_overlay_without_persisting_kv() {
		let kv = new_in_memory();
		let ctx = new_with_kv(
			"actor-overlay",
			"task-overlay",
			Vec::new(),
			"local",
			kv.clone(),
		);
		let (_lifecycle_tx, lifecycle_rx) = mpsc::channel(4);
		let (_dispatch_tx, dispatch_rx) = mpsc::channel(4);
		let (events_tx, events_rx) = mpsc::channel(4);
		ctx.configure_lifecycle_events(Some(events_tx));

		let factory = Arc::new(ActorFactory::new(Default::default(), move |start| {
			Box::pin(async move {
				let mut events = start.events;
				while let Some(event) = events.recv().await {
					match event {
						ActorEvent::SerializeState {
							reason: SerializeStateReason::Inspector,
							reply,
						} => {
							reply.send(Ok(vec![StateDelta::ActorState(vec![9, 9, 9])]));
						}
						ActorEvent::BeginSleep => {}
						ActorEvent::FinalizeSleep { reply } | ActorEvent::Destroy { reply } => {
							reply.send(Ok(()));
							break;
						}
						_ => {}
					}
				}
				Ok(())
			})
		}));

		let mut task = ActorTask::new(
			"actor-overlay".into(),
			0,
			lifecycle_rx,
			dispatch_rx,
			events_rx,
			factory,
			ctx.clone(),
			None,
			None,
		);

		let mut inspector_rx = ctx
			.subscribe_inspector()
			.expect("inspector runtime should be configured");
		let (start_tx, start_rx) = oneshot::channel();
		task.handle_lifecycle(LifecycleCommand::Start { reply: start_tx })
			.await;
		start_rx
			.await
			.expect("start reply should send")
			.expect("start should succeed");

		let _inspector_guard = ctx
			.inspector_attach()
			.expect("inspector runtime should be configured");
		ctx.request_save(RequestSaveOpts::default());
		drain_lifecycle_events(&mut task).await;
		assert!(task.inspector_serialize_state_deadline.is_some());

		task.on_inspector_serialize_state_tick().await;

		let overlay = inspector_rx
			.recv()
			.await
			.expect("inspector overlay should broadcast");
		let deltas: Vec<StateDelta> =
			ciborium::from_reader(overlay.as_slice()).expect("overlay payload should decode");
		assert_eq!(deltas, vec![StateDelta::ActorState(vec![9, 9, 9])]);
		assert!(ctx.save_requested());

		let persisted_actor = kv
			.get(PERSIST_DATA_KEY)
			.await
			.expect("persisted actor lookup should succeed")
			.expect("persisted actor should exist");
		let persisted_actor =
			decode_persisted_actor(&persisted_actor).expect("persisted actor should decode");
		assert_eq!(persisted_actor.state, Vec::<u8>::new());
		assert_eq!(ctx.state(), Vec::<u8>::new());

		task.handle_stop(ShutdownKind::Destroy)
			.await
			.expect("destroy stop should succeed");
	}

	#[tokio::test]
	async fn save_tick_cancels_pending_inspector_deadline_and_broadcasts_overlay() {
		let ctx = new_with_kv(
			"actor-save-overlay",
			"task-save-overlay",
			Vec::new(),
			"local",
			new_in_memory(),
		);
		let (_lifecycle_tx, lifecycle_rx) = mpsc::channel(4);
		let (_dispatch_tx, dispatch_rx) = mpsc::channel(4);
		let (events_tx, events_rx) = mpsc::channel(4);
		ctx.configure_lifecycle_events(Some(events_tx));

		let save_ticks = Arc::new(AtomicUsize::new(0));
		let mut task = ActorTask::new(
			"actor-save-overlay".into(),
			0,
			lifecycle_rx,
			dispatch_rx,
			events_rx,
			save_tick_factory(save_ticks),
			ctx.clone(),
			None,
			None,
		);

		let mut inspector_rx = ctx
			.subscribe_inspector()
			.expect("inspector runtime should be configured");
		let (start_tx, start_rx) = oneshot::channel();
		task.handle_lifecycle(LifecycleCommand::Start { reply: start_tx })
			.await;
		start_rx
			.await
			.expect("start reply should send")
			.expect("start should succeed");

		let _inspector_guard = ctx
			.inspector_attach()
			.expect("inspector runtime should be configured");
		ctx.request_save(RequestSaveOpts::default());
		drain_lifecycle_events(&mut task).await;
		assert!(task.state_save_deadline.is_some());
		assert!(task.inspector_serialize_state_deadline.is_some());

		task.on_state_save_tick().await;

		assert!(task.inspector_serialize_state_deadline.is_none());
		let overlay = inspector_rx
			.recv()
			.await
			.expect("save tick should broadcast inspector overlay");
		let deltas: Vec<StateDelta> =
			ciborium::from_reader(overlay.as_slice()).expect("overlay payload should decode");
		assert_eq!(deltas, vec![StateDelta::ActorState(vec![1])]);
		wait_for_state(&ctx, &[1]).await;

		task.handle_stop(ShutdownKind::Destroy)
			.await
			.expect("destroy stop should succeed");
	}

	#[tokio::test]
	async fn save_tick_reschedules_when_request_save_arrives_during_in_flight_reply() {
		let ctx = new_with_kv(
			"actor-race",
			"task-race",
			Vec::new(),
			"local",
			new_in_memory(),
		);
		let (_lifecycle_tx, lifecycle_rx) = mpsc::channel(4);
		let (_dispatch_tx, dispatch_rx) = mpsc::channel(4);
		let (events_tx, events_rx) = mpsc::channel(4);
		ctx.configure_lifecycle_events(Some(events_tx));

		let save_ticks = Arc::new(AtomicUsize::new(0));
		let factory = Arc::new(ActorFactory::new(Default::default(), {
			let save_ticks = save_ticks.clone();
			move |start| {
				let save_ticks = save_ticks.clone();
				Box::pin(async move {
					let ctx = start.ctx.clone();
					let mut events = start.events;
					while let Some(event) = events.recv().await {
						match event {
							ActorEvent::SerializeState {
								reason: SerializeStateReason::Save,
								reply,
							} => {
								let tick = save_ticks.fetch_add(1, Ordering::SeqCst) + 1;
								if tick == 1 {
									ctx.request_save(RequestSaveOpts::default());
								}
								reply.send(Ok(vec![StateDelta::ActorState(vec![tick as u8])]));
							}
							ActorEvent::BeginSleep => {}
							ActorEvent::FinalizeSleep { reply } | ActorEvent::Destroy { reply } => {
								reply.send(Ok(()));
								break;
							}
							_ => {}
						}
					}
					Ok(())
				})
			}
		}));

		let mut task = ActorTask::new(
			"actor-race".into(),
			0,
			lifecycle_rx,
			dispatch_rx,
			events_rx,
			factory,
			ctx.clone(),
			None,
			None,
		);

		let (start_tx, start_rx) = oneshot::channel();
		task.handle_lifecycle(LifecycleCommand::Start { reply: start_tx })
			.await;
		start_rx
			.await
			.expect("start reply should send")
			.expect("start should succeed");

		ctx.request_save(RequestSaveOpts::default());
		task.handle_event(crate::actor::task::LifecycleEvent::SaveRequested { immediate: false })
			.await;
		task.on_state_save_tick().await;

		wait_for_count(&save_ticks, 1).await;
		wait_for_state(&ctx, &[1]).await;
		assert!(
			task.state_save_deadline.is_some(),
			"a second save tick should be scheduled"
		);
		assert!(ctx.save_requested());

		task.on_state_save_tick().await;
		wait_for_count(&save_ticks, 2).await;
		wait_for_state(&ctx, &[2]).await;
		assert!(!ctx.save_requested());

		task.handle_stop(ShutdownKind::Destroy)
			.await
			.expect("destroy stop should succeed");
	}

	#[tokio::test]
	async fn sleep_shutdown_persists_actor_and_hibernation_deltas() {
		let kv = new_in_memory();
		let ctx = new_with_kv("actor-sleep", "task-sleep", Vec::new(), "local", kv.clone());
		let (_lifecycle_tx, lifecycle_rx) = mpsc::channel(4);
		let (_dispatch_tx, dispatch_rx) = mpsc::channel(4);
		let (events_tx, events_rx) = mpsc::channel(4);
		ctx.configure_lifecycle_events(Some(events_tx));

		let disconnects = Arc::new(Mutex::new(Vec::<String>::new()));
		let normal_conn = managed_test_conn(&ctx, "conn-normal", false, disconnects.clone());
		ctx.add_conn(normal_conn.clone());
		let hibernating_conn =
			managed_test_conn(&ctx, "conn-hibernating", true, disconnects.clone());
		hibernating_conn.configure_hibernation(Some(HibernatableConnectionMetadata {
			gateway_id: *b"gate",
			request_id: *b"req1",
			server_message_index: 1,
			client_message_index: 2,
			request_path: "/ws".to_owned(),
			request_headers: BTreeMap::from([("x-test".to_owned(), "true".to_owned())]),
		}));
		ctx.add_conn(hibernating_conn.clone());
		configure_live_hibernated_pairs(&ctx, [(b"gate".as_slice(), b"req1".as_slice())]);

		let hibernating_conn_id = hibernating_conn.id().to_owned();
		let factory = Arc::new(ActorFactory::new(
			ActorConfig {
				sleep_grace_period: Duration::from_millis(200),
				sleep_grace_period_overridden: true,
				..ActorConfig::default()
			},
			move |start| {
				let hibernating_conn_id = hibernating_conn_id.clone();
				Box::pin(async move {
					let ctx = start.ctx.clone();
					let mut events = start.events;
					while let Some(event) = events.recv().await {
						match event {
							ActorEvent::SerializeState { reply, .. } => {
								reply.send(Ok(vec![
									StateDelta::ActorState(vec![4, 5, 6]),
									StateDelta::ConnHibernation {
										conn: hibernating_conn_id.clone(),
										bytes: vec![9, 8, 7],
									},
								]));
							}
							ActorEvent::DisconnectConn { conn_id, reply } => {
								ctx.disconnect_conn(conn_id)
									.await
									.expect("sleep shutdown should disconnect conns");
								reply.send(Ok(()));
							}
							ActorEvent::RunGracefulCleanup { reply, .. } => {
								reply.send(Ok(()));
							}
							_ => {}
						}
					}
					Ok(())
				})
			},
		));

		let mut task = ActorTask::new(
			"actor-sleep".into(),
			0,
			lifecycle_rx,
			dispatch_rx,
			events_rx,
			factory,
			ctx.clone(),
			None,
			None,
		);
		let (start_tx, start_rx) = oneshot::channel();
		task.handle_lifecycle(LifecycleCommand::Start { reply: start_tx })
			.await;
		start_rx
			.await
			.expect("start reply should send")
			.expect("start should succeed");

		task.handle_stop(ShutdownKind::Sleep)
			.await
			.expect("sleep stop should succeed");

		let persisted_actor = kv
			.get(PERSIST_DATA_KEY)
			.await
			.expect("persisted actor lookup should succeed")
			.expect("persisted actor should exist");
		let persisted_actor =
			decode_persisted_actor(&persisted_actor).expect("persisted actor should decode");
		assert_eq!(persisted_actor.state, vec![4, 5, 6]);

		let persisted_conn = kv
			.get(&make_connection_key(hibernating_conn.id()))
			.await
			.expect("persisted connection lookup should succeed")
			.expect("persisted connection should exist");
		let persisted_conn = decode_persisted_connection(&persisted_conn)
			.expect("persisted connection should decode");
		assert_eq!(persisted_conn.state, vec![9, 8, 7]);
		assert_eq!(
			disconnects
				.lock()
				.expect("disconnect log lock poisoned")
				.as_slice(),
			["conn-normal"]
		);
		let remaining_conns: Vec<_> = ctx.conns().collect();
		assert_eq!(remaining_conns.len(), 1);
		assert_eq!(remaining_conns[0].id(), hibernating_conn.id());
	}

	#[tokio::test]
	async fn sleep_shutdown_waits_for_on_state_change_before_final_save() {
		let kv = new_in_memory();
		let ctx = new_with_kv(
			"actor-sleep-state-change",
			"task-sleep-state-change",
			Vec::new(),
			"local",
			kv.clone(),
		);
		ctx.set_state_initial(vec![1]);

		let (_lifecycle_tx, lifecycle_rx) = mpsc::channel(4);
		let (_dispatch_tx, dispatch_rx) = mpsc::channel(4);
		let (events_tx, events_rx) = mpsc::channel(4);
		ctx.configure_lifecycle_events(Some(events_tx));

		let factory = Arc::new(ActorFactory::new(
			ActorConfig {
				action_timeout: Duration::from_millis(500),
				sleep_grace_period: Duration::from_millis(500),
				sleep_grace_period_overridden: true,
				..ActorConfig::default()
			},
			|start| {
				Box::pin(async move {
					let ctx = start.ctx.clone();
					let mut events = start.events;
					while let Some(event) = events.recv().await {
						match event {
							ActorEvent::SerializeState { reply, .. } => {
								reply.send(Ok(vec![StateDelta::ActorState(ctx.state())]));
							}
							ActorEvent::RunGracefulCleanup { reply, .. } => {
								reply.send(Ok(()));
							}
							_ => {}
						}
					}
					Ok(())
				})
			},
		));

		let mut task = ActorTask::new(
			"actor-sleep-state-change".into(),
			0,
			lifecycle_rx,
			dispatch_rx,
			events_rx,
			factory,
			ctx.clone(),
			None,
			None,
		);
		let (start_tx, start_rx) = oneshot::channel();
		task.handle_lifecycle(LifecycleCommand::Start { reply: start_tx })
			.await;
		start_rx
			.await
			.expect("start reply should send")
			.expect("start should succeed");

		let on_state_change = ctx.begin_on_state_change();
		let callback_ctx = ctx.clone();
		tokio::spawn(async move {
			sleep(Duration::from_millis(25)).await;
			callback_ctx.set_state_initial(vec![8]);
			drop(on_state_change);
		});

		task.handle_stop(ShutdownKind::Sleep)
			.await
			.expect("sleep stop should succeed");

		let persisted_actor = kv
			.get(PERSIST_DATA_KEY)
			.await
			.expect("persisted actor lookup should succeed")
			.expect("persisted actor should exist");
		let persisted_actor =
			decode_persisted_actor(&persisted_actor).expect("persisted actor should decode");
		assert_eq!(persisted_actor.state, vec![8]);
	}

	#[tokio::test]
	async fn destroy_shutdown_disconnects_hibernating_connections_after_final_delta_flush() {
		let kv = new_in_memory();
		let ctx = new_with_kv(
			"actor-destroy",
			"task-destroy",
			Vec::new(),
			"local",
			kv.clone(),
		);
		let (_lifecycle_tx, lifecycle_rx) = mpsc::channel(4);
		let (_dispatch_tx, dispatch_rx) = mpsc::channel(4);
		let (events_tx, events_rx) = mpsc::channel(4);
		ctx.configure_lifecycle_events(Some(events_tx));

		let disconnects = Arc::new(Mutex::new(Vec::<String>::new()));
		let normal_conn = managed_test_conn(&ctx, "conn-normal", false, disconnects.clone());
		ctx.add_conn(normal_conn);
		let hibernating_conn =
			managed_test_conn(&ctx, "conn-hibernating", true, disconnects.clone());
		hibernating_conn.configure_hibernation(Some(HibernatableConnectionMetadata {
			gateway_id: *b"gate",
			request_id: *b"req1",
			server_message_index: 1,
			client_message_index: 2,
			request_path: "/ws".to_owned(),
			request_headers: BTreeMap::new(),
		}));
		ctx.add_conn(hibernating_conn.clone());
		configure_live_hibernated_pairs(&ctx, [(b"gate".as_slice(), b"req1".as_slice())]);

		let factory = Arc::new(ActorFactory::new(
			ActorConfig {
				sleep_grace_period: Duration::from_millis(200),
				sleep_grace_period_overridden: true,
				on_destroy_timeout: Duration::from_millis(100),
				..ActorConfig::default()
			},
			move |start| {
				Box::pin(async move {
					let ctx = start.ctx.clone();
					let mut events = start.events;
					while let Some(event) = events.recv().await {
						match event {
							ActorEvent::SerializeState { reply, .. } => {
								reply.send(Ok(vec![StateDelta::ActorState(vec![7, 7, 7])]));
							}
							ActorEvent::DisconnectConn { conn_id, reply } => {
								ctx.disconnect_conn(conn_id)
									.await
									.expect("destroy shutdown should disconnect conns");
								reply.send(Ok(()));
							}
							ActorEvent::RunGracefulCleanup { reply, .. } => {
								reply.send(Ok(()));
							}
							_ => {}
						}
					}
					Ok(())
				})
			},
		));

		let mut task = ActorTask::new(
			"actor-destroy".into(),
			0,
			lifecycle_rx,
			dispatch_rx,
			events_rx,
			factory,
			ctx.clone(),
			None,
			None,
		);
		let (start_tx, start_rx) = oneshot::channel();
		task.handle_lifecycle(LifecycleCommand::Start { reply: start_tx })
			.await;
		start_rx
			.await
			.expect("start reply should send")
			.expect("start should succeed");

		task.handle_stop(ShutdownKind::Destroy)
			.await
			.expect("destroy stop should succeed");

		let persisted_actor = kv
			.get(PERSIST_DATA_KEY)
			.await
			.expect("persisted actor lookup should succeed")
			.expect("persisted actor should exist");
		let persisted_actor =
			decode_persisted_actor(&persisted_actor).expect("persisted actor should decode");
		assert_eq!(persisted_actor.state, vec![7, 7, 7]);

		let mut disconnects = disconnects
			.lock()
			.expect("disconnect log lock poisoned")
			.clone();
		disconnects.sort();
		assert_eq!(
			disconnects,
			vec!["conn-hibernating".to_owned(), "conn-normal".to_owned()]
		);
		assert!(
			kv.get(&make_connection_key(hibernating_conn.id()))
				.await
				.expect("persisted connection lookup should succeed")
				.is_none()
		);
		assert!(ctx.conns().is_empty());
	}

	#[tokio::test]
	async fn action_dispatch_uses_optional_conn_and_alarms_use_none() {
		let ctx = new_with_kv(
			"actor-action",
			"task-action",
			Vec::new(),
			"local",
			new_in_memory(),
		);
		let (_lifecycle_tx, lifecycle_rx) = mpsc::channel(4);
		let (_dispatch_tx, dispatch_rx) = mpsc::channel(4);
		let (events_tx, events_rx) = mpsc::channel(4);
		ctx.configure_lifecycle_events(Some(events_tx));

		let seen_conns = Arc::new(Mutex::new(Vec::<Option<String>>::new()));
		let seen_conns_for_entry = seen_conns.clone();
		let factory = Arc::new(ActorFactory::new(Default::default(), move |start| {
			let seen_conns = seen_conns_for_entry.clone();
			Box::pin(async move {
				let mut events = start.events;
				while let Some(event) = events.recv().await {
					match event {
						ActorEvent::Action {
							name, conn, reply, ..
						} => {
							seen_conns
								.lock()
								.expect("action log lock poisoned")
								.push(conn.as_ref().map(|conn| conn.id().to_owned()));
							reply.send(Ok(name.into_bytes()));
						}
						ActorEvent::BeginSleep => {}
						ActorEvent::FinalizeSleep { reply } | ActorEvent::Destroy { reply } => {
							reply.send(Ok(()));
							break;
						}
						_ => {}
					}
				}
				Ok(())
			})
		}));

		let mut task = ActorTask::new(
			"actor-action".into(),
			0,
			lifecycle_rx,
			dispatch_rx,
			events_rx,
			factory,
			ctx,
			None,
			None,
		);
		let (start_tx, start_rx) = oneshot::channel();
		task.handle_lifecycle(LifecycleCommand::Start { reply: start_tx })
			.await;
		start_rx
			.await
			.expect("start reply should send")
			.expect("start should succeed");

		let client_conn = ConnHandle::new("conn-client", Vec::new(), Vec::new(), false);
		let (reply_tx, reply_rx) = oneshot::channel();
		task.handle_dispatch(DispatchCommand::Action {
			name: "client-action".to_owned(),
			args: Vec::new(),
			conn: client_conn,
			reply: reply_tx,
		})
		.await;
		assert_eq!(
			reply_rx
				.await
				.expect("client action reply should send")
				.expect("client action should succeed"),
			b"client-action".to_vec(),
		);

		let mut persisted = task.ctx.persisted_actor();
		persisted.scheduled_events.push(PersistedScheduleEvent {
			event_id: "event-1".to_owned(),
			timestamp_ms: 0,
			action: "alarm-action".to_owned(),
			args: Vec::new(),
		});
		task.ctx.load_persisted_actor(PersistedActor {
			scheduled_events: persisted.scheduled_events,
			..persisted
		});
		task.ctx
			.drain_overdue_scheduled_events()
			.await
			.expect("scheduled actions should drain");
		for _ in 0..50 {
			if seen_conns.lock().expect("action log lock poisoned").len() >= 2 {
				break;
			}
			sleep(Duration::from_millis(10)).await;
		}

		assert_eq!(
			seen_conns.lock().expect("action log lock poisoned").clone(),
			vec![Some("conn-client".to_owned()), None],
		);

		task.handle_stop(ShutdownKind::Destroy)
			.await
			.expect("destroy stop should succeed");
	}

	#[tokio::test]
	async fn wake_start_hibernated_does_not_refire_connection_open() {
		let kv = new_in_memory();
		let seed_ctx = new_with_kv("actor-wake", "task-wake", Vec::new(), "local", kv.clone());
		let seed_conn = ConnHandle::new("conn-hibernating", Vec::new(), Vec::new(), true);
		seed_conn.configure_hibernation(Some(HibernatableConnectionMetadata {
			gateway_id: *b"gate",
			request_id: *b"req1",
			server_message_index: 4,
			client_message_index: 8,
			request_path: "/ws".to_owned(),
			request_headers: BTreeMap::new(),
		}));
		seed_ctx.add_conn(seed_conn.clone());
		seed_ctx
			.save_state(vec![StateDelta::ConnHibernation {
				conn: seed_conn.id().into(),
				bytes: vec![3, 2, 1],
			}])
			.await
			.expect("seed hibernation should persist");

		let ctx = new_with_kv("actor-wake", "task-wake", Vec::new(), "local", kv.clone());
		let (_lifecycle_tx, lifecycle_rx) = mpsc::channel(4);
		let (_dispatch_tx, dispatch_rx) = mpsc::channel(4);
		let (events_tx, events_rx) = mpsc::channel(4);
		ctx.configure_lifecycle_events(Some(events_tx));
		configure_live_hibernated_pairs(&ctx, [(b"gate".as_slice(), b"req1".as_slice())]);
		let (started_tx, started_rx) = oneshot::channel();
		let started_tx = Arc::new(Mutex::new(Some(started_tx)));
		let factory = Arc::new(ActorFactory::new(Default::default(), move |start| {
			let started_tx = started_tx.clone();
			Box::pin(async move {
				let mut events = start.events;
				started_tx
					.lock()
					.expect("started sender lock poisoned")
					.take()
					.expect("started sender should exist")
					.send((
						start.hibernated.len(),
						start.hibernated[0].1.clone(),
						events.try_recv().is_none(),
					))
					.expect("started info should send");
				while let Some(event) = events.recv().await {
					match event {
						ActorEvent::BeginSleep => {}
						ActorEvent::FinalizeSleep { reply } | ActorEvent::Destroy { reply } => {
							reply.send(Ok(()));
							break;
						}
						ActorEvent::ConnectionOpen { .. } => {
							panic!("hibernated connection should not refire ConnectionOpen");
						}
						_ => {}
					}
				}
				Ok(())
			})
		}));

		let mut task = ActorTask::new(
			"actor-wake".into(),
			0,
			lifecycle_rx,
			dispatch_rx,
			events_rx,
			factory,
			ctx.clone(),
			None,
			None,
		);
		let (start_tx, start_rx) = oneshot::channel();
		task.handle_lifecycle(LifecycleCommand::Start { reply: start_tx })
			.await;
		start_rx
			.await
			.expect("start reply should send")
			.expect("start should succeed");

		let (hibernated_count, bytes, no_initial_event) =
			started_rx.await.expect("start info should send");
		assert_eq!(hibernated_count, 1);
		assert_eq!(bytes, vec![3, 2, 1]);
		assert!(no_initial_event);

		task.handle_stop(ShutdownKind::Sleep)
			.await
			.expect("sleep stop should succeed");
	}

	#[tokio::test]
	async fn workflow_requests_dispatch_through_actor_events() {
		let ctx = new_with_kv(
			"actor-workflow",
			"task-workflow",
			Vec::new(),
			"local",
			new_in_memory(),
		);
		let (_lifecycle_tx, lifecycle_rx) = mpsc::channel(4);
		let (_dispatch_tx, dispatch_rx) = mpsc::channel(4);
		let (events_tx, events_rx) = mpsc::channel(4);
		ctx.configure_lifecycle_events(Some(events_tx));

		let (started_tx, started_rx) = oneshot::channel();
		let started_tx = Arc::new(Mutex::new(Some(started_tx)));
		let workflow_requests = Arc::new(Mutex::new(Vec::<Option<String>>::new()));
		let workflow_requests_for_entry = workflow_requests.clone();
		let history_payload = vec![4, 2];
		let replay_payload = vec![9, 9, 1];
		let factory = Arc::new(ActorFactory::new(Default::default(), move |start| {
			let started_tx = started_tx.clone();
			let workflow_requests = workflow_requests_for_entry.clone();
			let history_payload = history_payload.clone();
			let replay_payload = replay_payload.clone();
			Box::pin(async move {
				let mut events = start.events;
				started_tx
					.lock()
					.expect("started sender lock poisoned")
					.take()
					.expect("started sender should exist")
					.send(events.try_recv().is_none())
					.expect("started info should send");
				while let Some(event) = events.recv().await {
					match event {
						ActorEvent::WorkflowHistoryRequested { reply } => {
							workflow_requests
								.lock()
								.expect("workflow request log lock poisoned")
								.push(None);
							reply.send(Ok(Some(history_payload.clone())));
						}
						ActorEvent::WorkflowReplayRequested { entry_id, reply } => {
							workflow_requests
								.lock()
								.expect("workflow request log lock poisoned")
								.push(entry_id.clone());
							reply.send(Ok(Some(replay_payload.clone())));
						}
						ActorEvent::BeginSleep => {}
						ActorEvent::FinalizeSleep { reply } | ActorEvent::Destroy { reply } => {
							reply.send(Ok(()));
							break;
						}
						_ => {}
					}
				}
				Ok(())
			})
		}));

		let mut task = ActorTask::new(
			"actor-workflow".into(),
			0,
			lifecycle_rx,
			dispatch_rx,
			events_rx,
			factory,
			ctx.clone(),
			None,
			None,
		);
		let (start_tx, start_rx) = oneshot::channel();
		task.handle_lifecycle(LifecycleCommand::Start { reply: start_tx })
			.await;
		start_rx
			.await
			.expect("start reply should send")
			.expect("start should succeed");
		assert!(
			started_rx.await.expect("started info should send"),
			"workflow events should only arrive on explicit request"
		);

		let (history_tx, history_rx) = oneshot::channel();
		task.handle_dispatch(DispatchCommand::WorkflowHistory { reply: history_tx })
			.await;
		assert_eq!(
			history_rx
				.await
				.expect("workflow history reply should send")
				.expect("workflow history should succeed"),
			Some(vec![4, 2]),
		);

		let (replay_tx, replay_rx) = oneshot::channel();
		task.handle_dispatch(DispatchCommand::WorkflowReplay {
			entry_id: Some("entry-123".to_owned()),
			reply: replay_tx,
		})
		.await;
		assert_eq!(
			replay_rx
				.await
				.expect("workflow replay reply should send")
				.expect("workflow replay should succeed"),
			Some(vec![9, 9, 1]),
		);
		assert_eq!(
			workflow_requests
				.lock()
				.expect("workflow request log lock poisoned")
				.clone(),
			vec![None, Some("entry-123".to_owned())],
		);

		task.handle_stop(ShutdownKind::Destroy)
			.await
			.expect("destroy stop should succeed");
	}

	#[tokio::test]
	async fn hibernation_transport_updates_flush_only_on_save_tick() {
		let kv = new_in_memory();
		let ctx = new_with_kv("actor-hws", "task-hws", Vec::new(), "local", kv.clone());

		let (_lifecycle_tx, lifecycle_rx) = mpsc::channel(4);
		let (_dispatch_tx, dispatch_rx) = mpsc::channel(4);
		let (events_tx, events_rx) = mpsc::channel(4);
		ctx.configure_lifecycle_events(Some(events_tx));

		let factory = Arc::new(ActorFactory::new(Default::default(), move |start| {
			Box::pin(async move {
				let mut events = start.events;
				while let Some(event) = events.recv().await {
					match event {
						ActorEvent::SerializeState {
							reason: SerializeStateReason::Save,
							reply,
						} => {
							reply.send(Ok(Vec::new()));
						}
						ActorEvent::BeginSleep => {}
						ActorEvent::FinalizeSleep { reply } | ActorEvent::Destroy { reply } => {
							reply.send(Ok(()));
							break;
						}
						_ => {}
					}
				}
				Ok(())
			})
		}));

		let mut task = ActorTask::new(
			"actor-hws".into(),
			0,
			lifecycle_rx,
			dispatch_rx,
			events_rx,
			factory,
			ctx.clone(),
			None,
			None,
		);
		let (start_tx, start_rx) = oneshot::channel();
		task.handle_lifecycle(LifecycleCommand::Start { reply: start_tx })
			.await;
		start_rx
			.await
			.expect("start reply should send")
			.expect("start should succeed");

		let disconnects = Arc::new(Mutex::new(Vec::<String>::new()));
		let conn = managed_test_conn(&ctx, "conn-hibernating", true, disconnects);
		conn.configure_hibernation(Some(HibernatableConnectionMetadata {
			gateway_id: *b"gate",
			request_id: *b"req1",
			server_message_index: 1,
			client_message_index: 2,
			request_path: "/ws".to_owned(),
			request_headers: BTreeMap::new(),
		}));
		ctx.add_conn(conn.clone());
		ctx.save_state(vec![StateDelta::ConnHibernation {
			conn: conn.id().into(),
			bytes: vec![9, 8, 7],
		}])
		.await
		.expect("seed hibernation should persist");
		assert_eq!(kv.test_apply_batch_call_count(), 1);

		conn.set_server_message_index(7);
		ctx.request_hibernation_transport_save(conn.id());
		assert_eq!(kv.test_apply_batch_call_count(), 1);
		let persisted_before = kv
			.get(&make_connection_key(conn.id()))
			.await
			.expect("persisted connection lookup should succeed")
			.expect("persisted connection should exist");
		let persisted_before = decode_persisted_connection(&persisted_before)
			.expect("persisted connection should decode");
		assert_eq!(persisted_before.server_message_index, 1);

		task.handle_event(crate::actor::task::LifecycleEvent::SaveRequested { immediate: false })
			.await;
		task.on_state_save_tick().await;

		assert_eq!(kv.test_apply_batch_call_count(), 2);
		let persisted_after = kv
			.get(&make_connection_key(conn.id()))
			.await
			.expect("persisted connection lookup should succeed")
			.expect("persisted connection should exist");
		let persisted_after = decode_persisted_connection(&persisted_after)
			.expect("persisted connection should decode");
		assert_eq!(persisted_after.server_message_index, 7);

		task.handle_stop(ShutdownKind::Destroy)
			.await
			.expect("destroy stop should succeed");
	}

	#[tokio::test(start_paused = true)]
	async fn destroy_waits_for_tracked_schedule_persistence() {
		let kv = new_in_memory();
		let ctx = new_with_kv(
			"actor-schedule-destroy",
			"task-schedule-destroy",
			Vec::new(),
			"local",
			kv.clone(),
		);

		let (_lifecycle_tx, lifecycle_rx) = mpsc::channel(4);
		let (_dispatch_tx, dispatch_rx) = mpsc::channel(4);
		let (events_tx, events_rx) = mpsc::channel(4);
		ctx.configure_lifecycle_events(Some(events_tx));

		let factory = Arc::new(ActorFactory::new(Default::default(), move |start| {
			Box::pin(async move {
				let ctx = start.ctx.clone();
				let mut events = start.events;
				while let Some(event) = events.recv().await {
					match event {
						ActorEvent::RunGracefulCleanup { reason, reply } => {
							if reason == ShutdownKind::Destroy {
								ctx.after(Duration::from_secs(60), "after-destroy", &[1, 2, 3]);
							}
							reply.send(Ok(()));
							break;
						}
						_ => {}
					}
				}
				Ok(())
			})
		}));

		let mut task = ActorTask::new(
			"actor-schedule-destroy".into(),
			0,
			lifecycle_rx,
			dispatch_rx,
			events_rx,
			factory,
			ctx.clone(),
			None,
			None,
		);
		let (start_tx, start_rx) = oneshot::channel();
		task.handle_lifecycle(LifecycleCommand::Start { reply: start_tx })
			.await;
		start_rx
			.await
			.expect("start reply should send")
			.expect("start should succeed");

		task.handle_stop(ShutdownKind::Destroy)
			.await
			.expect("destroy stop should succeed");

		let actor_bytes = kv
			.get(PERSIST_DATA_KEY)
			.await
			.expect("persisted actor lookup should succeed")
			.expect("scheduled event should be persisted before shutdown returns");
		let persisted =
			decode_persisted_actor(&actor_bytes).expect("persisted actor should decode");
		assert_eq!(persisted.scheduled_events.len(), 1);
		assert_eq!(persisted.scheduled_events[0].action, "after-destroy");
		assert_eq!(persisted.scheduled_events[0].args, vec![1, 2, 3]);
	}

	#[tokio::test]
	async fn startup_uses_empty_preloaded_persisted_actor_without_fallback_get() {
		let kv = new_in_memory();
		let ctx = new_with_kv(
			"actor-preload-empty",
			"task-preload-empty",
			Vec::new(),
			"local",
			kv.clone(),
		);
		let mut task = new_task(ctx.clone())
			.with_preloaded_persisted_actor(PreloadedPersistedActor::BundleExistsButEmpty);
		let (start_tx, start_rx) = oneshot::channel();

		task.handle_lifecycle(LifecycleCommand::Start { reply: start_tx })
			.await;
		start_rx
			.await
			.expect("start reply should send")
			.expect("start should succeed");

		assert_eq!(kv.test_batch_get_call_count(), 0);
		assert!(ctx.persisted_actor().has_initialized);
	}

	#[tokio::test]
	async fn startup_skips_future_alarm_push_when_last_pushed_matches() {
		let kv = new_in_memory();
		let future_ts = 4_102_444_800_000;
		let persisted = PersistedActor {
			has_initialized: true,
			scheduled_events: vec![PersistedScheduleEvent {
				event_id: "evt-future".to_owned(),
				timestamp_ms: future_ts,
				action: "tick".to_owned(),
				args: Vec::new(),
			}],
			..PersistedActor::default()
		};
		kv.put(
			PERSIST_DATA_KEY,
			&encode_persisted_actor(&persisted).expect("persisted actor should encode"),
		)
		.await
		.expect("persisted actor should seed");
		kv.put(
			LAST_PUSHED_ALARM_KEY,
			&encode_last_pushed_alarm(Some(future_ts)).expect("last pushed alarm should encode"),
		)
		.await
		.expect("last pushed alarm should seed");

		let ctx = new_with_kv(
			"actor-startup-skip-alarm",
			"task-startup-skip-alarm",
			Vec::new(),
			"local",
			kv,
		);
		let (handle, mut rx) = test_envoy_handle();
		ctx.configure_envoy(handle, Some(11));
		let mut task = new_task(ctx);
		let (start_tx, start_rx) = oneshot::channel();

		task.handle_lifecycle(LifecycleCommand::Start { reply: start_tx })
			.await;
		start_rx
			.await
			.expect("start reply should send")
			.expect("start should succeed");

		assert_no_alarm(&mut rx);
	}

	#[tokio::test]
	async fn startup_persists_last_pushed_alarm_after_future_alarm_push() {
		let kv = new_in_memory();
		let future_ts = 4_102_444_900_000;
		let persisted = PersistedActor {
			has_initialized: true,
			scheduled_events: vec![PersistedScheduleEvent {
				event_id: "evt-future".to_owned(),
				timestamp_ms: future_ts,
				action: "tick".to_owned(),
				args: Vec::new(),
			}],
			..PersistedActor::default()
		};
		kv.put(
			PERSIST_DATA_KEY,
			&encode_persisted_actor(&persisted).expect("persisted actor should encode"),
		)
		.await
		.expect("persisted actor should seed");
		kv.put(
			LAST_PUSHED_ALARM_KEY,
			&encode_last_pushed_alarm(Some(future_ts + 1))
				.expect("last pushed alarm should encode"),
		)
		.await
		.expect("last pushed alarm should seed");

		let ctx = new_with_kv(
			"actor-startup-push-alarm",
			"task-startup-push-alarm",
			Vec::new(),
			"local",
			kv.clone(),
		);
		let (handle, mut rx) = test_envoy_handle();
		ctx.configure_envoy(handle, Some(12));
		let mut task = new_task(ctx.clone());
		let (start_tx, start_rx) = oneshot::channel();

		task.handle_lifecycle(LifecycleCommand::Start { reply: start_tx })
			.await;
		start_rx
			.await
			.expect("start reply should send")
			.expect("start should succeed");

		assert_eq!(
			recv_alarm_now(&mut rx, "actor-startup-push-alarm", Some(12)),
			Some(future_ts)
		);
		ctx.wait_for_pending_alarm_writes().await;
		let encoded = kv
			.get(LAST_PUSHED_ALARM_KEY)
			.await
			.expect("last pushed alarm lookup should succeed")
			.expect("last pushed alarm should be persisted");
		assert_eq!(
			decode_last_pushed_alarm(&encoded).expect("last pushed alarm should decode"),
			Some(future_ts)
		);
	}

	#[tokio::test]
	async fn fire_due_alarms_dispatches_overdue_work_during_sleep_grace() {
		let ctx = new_with_kv(
			"actor-sleep-grace-alarm",
			"task-sleep-grace-alarm",
			Vec::new(),
			"local",
			new_in_memory(),
		);
		let (events_tx, mut events_rx) = mpsc::unbounded_channel();
		ctx.configure_actor_events(Some(events_tx));
		ctx.load_persisted_actor(PersistedActor {
			scheduled_events: vec![PersistedScheduleEvent {
				event_id: "evt-overdue".to_owned(),
				timestamp_ms: 0,
				action: "tick".to_owned(),
				args: vec![1, 2, 3],
			}],
			..PersistedActor::default()
		});

		let mut task = new_task(ctx.clone());
		task.lifecycle = LifecycleState::SleepGrace;

		task.fire_due_alarms()
			.await
			.expect("sleep grace alarm fire should not fail");

		let dispatched = tokio::time::timeout(Duration::from_secs(1), events_rx.recv())
			.await
			.expect("sleep grace scheduled action should dispatch before timeout")
			.expect("actor event channel should stay open");
		match dispatched {
			ActorEvent::Action { reply, .. } => reply.send(Ok(Vec::new())),
			other => panic!("expected scheduled action dispatch, got {}", other.kind()),
		}
		assert!(
			ctx.next_event().is_none(),
			"overdue alarm should be consumed after dispatch"
		);
	}

	#[tokio::test(start_paused = true)]
	async fn sleep_shutdown_preserves_driver_alarm_after_cleanup() {
		let ctx = new_with_kv(
			"actor-sleep-alarm-preserve",
			"task-sleep-alarm-preserve",
			Vec::new(),
			"local",
			new_in_memory(),
		);
		let (mut task, lifecycle_tx, _dispatch_tx, _events_tx) = new_task_with_senders(ctx.clone());
		task.factory = shutdown_ack_factory(ActorConfig {
			sleep_grace_period: Duration::from_secs(5),
			sleep_grace_period_overridden: true,
			..ActorConfig::default()
		});
		let run = tokio::spawn(task.run());

		let (start_tx, start_rx) = oneshot::channel();
		lifecycle_tx
			.send(LifecycleCommand::Start { reply: start_tx })
			.await
			.expect("start command should send");
		start_rx
			.await
			.expect("start reply should send")
			.expect("start should succeed");

		ctx.after(Duration::from_secs(60), "wake", &[]);

		let (stop_tx, stop_rx) = oneshot::channel();
		lifecycle_tx
			.send(LifecycleCommand::Stop {
				reason: ShutdownKind::Sleep,
				reply: stop_tx,
			})
			.await
			.expect("sleep stop should send");
		stop_rx
			.await
			.expect("sleep stop reply should send")
			.expect("sleep stop should succeed");
		run.await
			.expect("task run should finish")
			.expect("task run should succeed");

		assert_eq!(ctx.test_driver_alarm_cancel_count(), 0);
	}

	#[tokio::test(start_paused = true)]
	async fn destroy_shutdown_still_clears_driver_alarm_after_cleanup() {
		let ctx = new_with_kv(
			"actor-destroy-alarm-clear",
			"task-destroy-alarm-clear",
			Vec::new(),
			"local",
			new_in_memory(),
		);
		let (mut task, lifecycle_tx, _dispatch_tx, _events_tx) = new_task_with_senders(ctx.clone());
		task.factory = shutdown_ack_factory(ActorConfig {
			on_destroy_timeout: Duration::from_secs(5),
			..ActorConfig::default()
		});
		let run = tokio::spawn(task.run());

		let (start_tx, start_rx) = oneshot::channel();
		lifecycle_tx
			.send(LifecycleCommand::Start { reply: start_tx })
			.await
			.expect("start command should send");
		start_rx
			.await
			.expect("start reply should send")
			.expect("start should succeed");

		ctx.after(Duration::from_secs(60), "wake", &[]);

		let (stop_tx, stop_rx) = oneshot::channel();
		lifecycle_tx
			.send(LifecycleCommand::Stop {
				reason: ShutdownKind::Destroy,
				reply: stop_tx,
			})
			.await
			.expect("destroy stop should send");
		stop_rx
			.await
			.expect("destroy stop reply should send")
			.expect("destroy stop should succeed");
		run.await
			.expect("task run should finish")
			.expect("task run should succeed");

		assert_eq!(ctx.test_driver_alarm_cancel_count(), 1);
	}

	#[tokio::test(start_paused = true)]
	async fn sleep_shutdown_without_in_flight_work_finishes_under_baseline() {
		let ctx = new_with_kv(
			"actor-sleep-fast",
			"task-sleep-fast",
			Vec::new(),
			"local",
			new_in_memory(),
		);
		let (mut task, lifecycle_tx, _dispatch_tx, _events_tx) = new_task_with_senders(ctx.clone());
		task.factory = shutdown_ack_factory(ActorConfig {
			sleep_grace_period: Duration::from_secs(5),
			sleep_grace_period_overridden: true,
			..ActorConfig::default()
		});
		let run = tokio::spawn(task.run());

		let (start_tx, start_rx) = oneshot::channel();
		lifecycle_tx
			.send(LifecycleCommand::Start { reply: start_tx })
			.await
			.expect("start command should send");
		start_rx
			.await
			.expect("start reply should send")
			.expect("start should succeed");

		let (stop_tx, stop_rx) = oneshot::channel();
		lifecycle_tx
			.send(LifecycleCommand::Stop {
				reason: ShutdownKind::Sleep,
				reply: stop_tx,
			})
			.await
			.expect("sleep stop should send");
		stop_rx
			.await
			.expect("sleep stop reply should send")
			.expect("sleep stop should succeed");
		run.await
			.expect("task run should finish")
			.expect("task run should succeed");
	}

	#[tokio::test(start_paused = true)]
	async fn sleep_shutdown_waits_for_keep_awake_work_then_finishes_next_tick() {
		let ctx = new_with_kv(
			"actor-sleep-keep-awake",
			"task-sleep-keep-awake",
			Vec::new(),
			"local",
			new_in_memory(),
		);
		let (mut task, lifecycle_tx, _dispatch_tx, _events_tx) = new_task_with_senders(ctx.clone());
		task.factory = shutdown_ack_factory(ActorConfig {
			sleep_grace_period: Duration::from_secs(5),
			sleep_grace_period_overridden: true,
			..ActorConfig::default()
		});
		let run = tokio::spawn(task.run());

		let (start_tx, start_rx) = oneshot::channel();
		lifecycle_tx
			.send(LifecycleCommand::Start { reply: start_tx })
			.await
			.expect("start command should send");
		start_rx
			.await
			.expect("start reply should send")
			.expect("start should succeed");

		let (release_tx, release_rx) = oneshot::channel();
		let keep_awake = tokio::spawn({
			let ctx = ctx.clone();
			async move {
				ctx.keep_awake(async move {
					let _ = release_rx.await;
				})
				.await
			}
		});
		yield_now().await;
		assert_eq!(ctx.keep_awake_count(), 1);

		let (stop_tx, stop_rx) = oneshot::channel();
		lifecycle_tx
			.send(LifecycleCommand::Stop {
				reason: ShutdownKind::Sleep,
				reply: stop_tx,
			})
			.await
			.expect("sleep stop should send");
		let stop = tokio::spawn(async move { stop_rx.await });
		yield_now().await;
		assert!(
			!stop.is_finished(),
			"sleep shutdown should stay blocked while keep-awake work is active"
		);

		release_tx.send(()).expect("release should send");
		stop.await
			.expect("sleep stop join should succeed")
			.expect("sleep stop reply should send")
			.expect("sleep stop should succeed");
		keep_awake
			.await
			.expect("keep-awake task should finish after release");
		run.await
			.expect("task run should finish")
			.expect("task run should succeed");
	}

	#[tokio::test(start_paused = true)]
	async fn destroy_shutdown_times_out_at_deadline_and_aborts_stuck_shutdown_task() {
		let ctx = new_with_kv(
			"actor-destroy-timeout",
			"task-destroy-timeout",
			Vec::new(),
			"local",
			new_in_memory(),
		);
		let destroy_timeout = Duration::from_millis(100);
		let mut task = new_task_with_factory(
			ctx.clone(),
			shutdown_ack_factory(ActorConfig {
				on_destroy_timeout: destroy_timeout,
				..ActorConfig::default()
			}),
		);

		let (start_tx, start_rx) = oneshot::channel();
		task.handle_lifecycle(LifecycleCommand::Start { reply: start_tx })
			.await;
		start_rx
			.await
			.expect("start reply should send")
			.expect("start should succeed");

		let (drop_tx, drop_rx) = oneshot::channel();
		let (_never_tx, never_rx) = oneshot::channel::<()>();
		ctx.wait_until(async move {
			let _notify = NotifyOnDrop::new(drop_tx);
			let _ = never_rx.await;
		});
		yield_now().await;

		let stop = tokio::spawn(async move { task.handle_stop(ShutdownKind::Destroy).await });
		yield_now().await;
		assert!(
			!stop.is_finished(),
			"destroy shutdown should wait for the destroy deadline while tracked work is stuck"
		);

		advance(destroy_timeout - Duration::from_millis(1)).await;
		yield_now().await;
		assert!(
			!stop.is_finished(),
			"destroy shutdown should still be waiting before the configured deadline"
		);

		advance(Duration::from_millis(1)).await;
		stop.await
			.expect("destroy stop join should succeed")
			.expect("destroy stop should succeed after timing out tracked work");
		drop_rx
			.await
			.expect("destroy teardown should abort the stuck shutdown task");
	}

	#[tokio::test(start_paused = true)]
	async fn ctx_wait_until_during_finish_shutdown_cleanup_refused_without_leak() {
		let _hook_lock = test_hook_lock().lock().await;
		let ctx = new_with_kv(
			"actor-sleep-cleanup-race",
			"task-sleep-cleanup-race",
			Vec::new(),
			"local",
			new_in_memory(),
		);
		let (hook_tx, hook_rx) = oneshot::channel();
		let (drop_tx, mut drop_rx) = oneshot::channel();
		let warning_count = Arc::new(AtomicUsize::new(0));
		let subscriber = Registry::default().with(ShutdownTaskRefusedWarningLayer {
			count: warning_count.clone(),
		});
		let _guard = tracing::subscriber::set_default(subscriber);
		let _cleanup_hook = crate::actor::task::install_shutdown_cleanup_hook(Arc::new({
			let hook_tx = Arc::new(Mutex::new(Some(hook_tx)));
			let drop_tx = Arc::new(Mutex::new(Some(drop_tx)));
			move |ctx, reason| {
				if ctx.actor_id() != "actor-sleep-cleanup-race" {
					return;
				}
				assert_eq!(reason, "sleep");
				let notify = NotifyOnDrop::new(
					drop_tx
						.lock()
						.expect("sleep drop notify lock poisoned")
						.take()
						.expect("sleep drop notify should only be taken once"),
				);
				let (_never_tx, never_rx) = oneshot::channel::<()>();
				ctx.wait_until(async move {
					let _notify = notify;
					let _ = never_rx.await;
				});
				if let Some(tx) = hook_tx
					.lock()
					.expect("sleep cleanup hook lock poisoned")
					.take()
				{
					let _ = tx.send(());
				}
			}
		}));
		let (mut task, lifecycle_tx, _dispatch_tx, _events_tx) = new_task_with_senders(ctx.clone());
		task.factory = shutdown_ack_factory(ActorConfig::default());
		let run = tokio::spawn(task.run());

		let (start_tx, start_rx) = oneshot::channel();
		lifecycle_tx
			.send(LifecycleCommand::Start { reply: start_tx })
			.await
			.expect("start command should send");
		start_rx
			.await
			.expect("start reply should send")
			.expect("start should succeed");

		let (stop_tx, stop_rx) = oneshot::channel();
		lifecycle_tx
			.send(LifecycleCommand::Stop {
				reason: ShutdownKind::Sleep,
				reply: stop_tx,
			})
			.await
			.expect("sleep stop command should send");
		hook_rx.await.expect("sleep cleanup hook should fire");
		assert_eq!(
			drop_rx
				.try_recv()
				.expect("refused sleep wait_until future should drop immediately"),
			()
		);
		assert_eq!(warning_count.load(Ordering::SeqCst), 1);
		stop_rx
			.await
			.expect("sleep stop reply should send")
			.expect("sleep shutdown should succeed");
		assert!(
			ctx.wait_for_shutdown_tasks(Instant::now() + Duration::from_millis(1))
				.await
		);
		run.await
			.expect("task run should finish")
			.expect("task run should succeed");
	}

	#[tokio::test(start_paused = true)]
	async fn destroy_shutdown_concurrent_wait_until_refused() {
		let _hook_lock = test_hook_lock().lock().await;
		let ctx = new_with_kv(
			"actor-destroy-cleanup-race",
			"task-destroy-cleanup-race",
			Vec::new(),
			"local",
			new_in_memory(),
		);
		let (hook_tx, hook_rx) = oneshot::channel();
		let (drop_tx, mut drop_rx) = oneshot::channel();
		let destroy_completed = Arc::new(AtomicUsize::new(0));
		let warning_count = Arc::new(AtomicUsize::new(0));
		let subscriber = Registry::default().with(ShutdownTaskRefusedWarningLayer {
			count: warning_count.clone(),
		});
		let _guard = tracing::subscriber::set_default(subscriber);
		let _cleanup_hook = crate::actor::task::install_shutdown_cleanup_hook(Arc::new({
			let hook_tx = Arc::new(Mutex::new(Some(hook_tx)));
			let drop_tx = Arc::new(Mutex::new(Some(drop_tx)));
			let destroy_completed = destroy_completed.clone();
			move |ctx, reason| {
				if ctx.actor_id() != "actor-destroy-cleanup-race" {
					return;
				}
				assert_eq!(reason, "destroy");
				assert_eq!(
					destroy_completed.load(Ordering::SeqCst),
					0,
					"destroy completion should not resolve before destroy cleanup finishes"
				);
				let notify = NotifyOnDrop::new(
					drop_tx
						.lock()
						.expect("destroy drop notify lock poisoned")
						.take()
						.expect("destroy drop notify should only be taken once"),
				);
				let (_never_tx, never_rx) = oneshot::channel::<()>();
				ctx.wait_until(async move {
					let _notify = notify;
					let _ = never_rx.await;
				});
				if let Some(tx) = hook_tx
					.lock()
					.expect("destroy cleanup hook lock poisoned")
					.take()
				{
					let _ = tx.send(());
				}
			}
		}));
		let (mut task, lifecycle_tx, _dispatch_tx, _events_tx) = new_task_with_senders(ctx.clone());
		task.factory = shutdown_ack_factory(ActorConfig::default());
		let run = tokio::spawn(task.run());

		let (start_tx, start_rx) = oneshot::channel();
		lifecycle_tx
			.send(LifecycleCommand::Start { reply: start_tx })
			.await
			.expect("start command should send");
		start_rx
			.await
			.expect("start reply should send")
			.expect("start should succeed");

		let destroy_wait = tokio::spawn({
			let ctx = ctx.clone();
			let destroy_completed = destroy_completed.clone();
			async move {
				ctx.wait_for_destroy_completion_public().await;
				destroy_completed.store(1, Ordering::SeqCst);
			}
		});
		yield_now().await;
		assert!(
			!destroy_wait.is_finished(),
			"destroy completion should not fire before shutdown begins"
		);

		let (stop_tx, stop_rx) = oneshot::channel();
		lifecycle_tx
			.send(LifecycleCommand::Stop {
				reason: ShutdownKind::Destroy,
				reply: stop_tx,
			})
			.await
			.expect("destroy stop command should send");
		hook_rx.await.expect("destroy cleanup hook should fire");
		assert_eq!(
			drop_rx
				.try_recv()
				.expect("refused destroy wait_until future should drop immediately"),
			()
		);
		assert_eq!(warning_count.load(Ordering::SeqCst), 1);
		stop_rx
			.await
			.expect("destroy stop reply should send")
			.expect("destroy shutdown should succeed");
		assert!(
			ctx.wait_for_shutdown_tasks(Instant::now() + Duration::from_millis(1))
				.await
		);
		destroy_wait
			.await
			.expect("destroy completion waiter should join");
		assert_eq!(destroy_completed.load(Ordering::SeqCst), 1);
		run.await
			.expect("task run should finish")
			.expect("task run should succeed");
	}

	#[tokio::test]
	async fn sleep_grace_keeps_dispatch_open_and_second_sleep_is_idempotent() {
		let ctx = new_with_kv(
			"actor-sleep-grace-dispatch",
			"task-sleep-grace-dispatch",
			Vec::new(),
			"local",
			new_in_memory(),
		);
		let (mut task, lifecycle_tx, dispatch_tx, _events_tx) = new_task_with_senders(ctx.clone());
		let begin_sleep_count = Arc::new(AtomicUsize::new(0));
		let destroy_count = Arc::new(AtomicUsize::new(0));
		let action_count = Arc::new(AtomicUsize::new(0));
		task.factory = sleep_grace_factory(
			ActorConfig {
				sleep_grace_period: Duration::from_secs(5),
				sleep_grace_period_overridden: true,
				..ActorConfig::default()
			},
			begin_sleep_count.clone(),
			destroy_count.clone(),
			action_count.clone(),
		);
		let run = tokio::spawn(task.run());

		let (start_tx, start_rx) = oneshot::channel();
		lifecycle_tx
			.send(LifecycleCommand::Start { reply: start_tx })
			.await
			.expect("start command should send");
		start_rx
			.await
			.expect("start reply should send")
			.expect("start should succeed");

		let (release_tx, release_rx) = oneshot::channel();
		let keep_awake = tokio::spawn({
			let ctx = ctx.clone();
			async move {
				ctx.keep_awake(async move {
					let _ = release_rx.await;
				})
				.await
			}
		});
		yield_now().await;

		let (sleep_tx, sleep_rx) = oneshot::channel();
		lifecycle_tx
			.send(LifecycleCommand::Stop {
				reason: ShutdownKind::Sleep,
				reply: sleep_tx,
			})
			.await
			.expect("sleep stop should send");
		wait_for_count(&begin_sleep_count, 1).await;
		assert!(ctx.actor_aborted());

		let (sleep_again_tx, sleep_again_rx) = oneshot::channel();
		lifecycle_tx
			.send(LifecycleCommand::Stop {
				reason: ShutdownKind::Sleep,
				reply: sleep_again_tx,
			})
			.await
			.expect("second sleep stop should send");
		sleep_again_rx
			.await
			.expect("second sleep reply should send")
			.expect("second sleep should no-op");
		assert_eq!(begin_sleep_count.load(Ordering::SeqCst), 1);

		let (action_tx, action_rx) = oneshot::channel();
		dispatch_tx
			.send(DispatchCommand::Action {
				name: "ping".to_owned(),
				args: Vec::new(),
				conn: ConnHandle::new("conn-grace", Vec::new(), Vec::new(), false),
				reply: action_tx,
			})
			.await
			.expect("action should send during sleep grace");
		let _error = action_rx
			.await
			.expect("action reply should send")
			.expect_err("sleep grace should reject new dispatch");
		assert_eq!(action_count.load(Ordering::SeqCst), 0);
		assert_eq!(destroy_count.load(Ordering::SeqCst), 0);

		release_tx.send(()).expect("keep-awake release should send");
		sleep_rx
			.await
			.expect("sleep reply should send")
			.expect("sleep should succeed");
		keep_awake
			.await
			.expect("keep-awake task should finish after release");
		assert_eq!(destroy_count.load(Ordering::SeqCst), 0);
		run.await
			.expect("task run should finish")
			.expect("task run should succeed");
	}

	#[cfg(not(debug_assertions))]
	#[tokio::test]
	async fn duplicate_destroy_during_sleep_grace_is_acked_and_ignored_in_release() {
		let ctx = new_with_kv(
			"actor-sleep-grace-destroy",
			"task-sleep-grace-destroy",
			Vec::new(),
			"local",
			new_in_memory(),
		);
		let (mut task, lifecycle_tx, _dispatch_tx, _events_tx) = new_task_with_senders(ctx.clone());
		let begin_sleep_count = Arc::new(AtomicUsize::new(0));
		let destroy_count = Arc::new(AtomicUsize::new(0));
		let action_count = Arc::new(AtomicUsize::new(0));
		task.factory = sleep_grace_factory(
			ActorConfig {
				sleep_grace_period: Duration::from_secs(5),
				sleep_grace_period_overridden: true,
				on_destroy_timeout: Duration::from_secs(5),
				..ActorConfig::default()
			},
			begin_sleep_count.clone(),
			destroy_count.clone(),
			action_count,
		);
		let run = tokio::spawn(task.run());

		let (start_tx, start_rx) = oneshot::channel();
		lifecycle_tx
			.send(LifecycleCommand::Start { reply: start_tx })
			.await
			.expect("start command should send");
		start_rx
			.await
			.expect("start reply should send")
			.expect("start should succeed");

		let (release_tx, release_rx) = oneshot::channel();
		let keep_awake = tokio::spawn({
			let ctx = ctx.clone();
			async move {
				ctx.keep_awake(async move {
					let _ = release_rx.await;
				})
				.await
			}
		});
		yield_now().await;

		let (sleep_tx, sleep_rx) = oneshot::channel();
		lifecycle_tx
			.send(LifecycleCommand::Stop {
				reason: ShutdownKind::Sleep,
				reply: sleep_tx,
			})
			.await
			.expect("sleep stop should send");
		wait_for_count(&begin_sleep_count, 1).await;

		let (destroy_tx, destroy_rx) = oneshot::channel();
		lifecycle_tx
			.send(LifecycleCommand::Stop {
				reason: ShutdownKind::Destroy,
				reply: destroy_tx,
			})
			.await
			.expect("destroy stop should send");
		destroy_rx
			.await
			.expect("destroy reply should send")
			.expect("duplicate destroy should be ignored");

		release_tx.send(()).expect("keep-awake release should send");
		sleep_rx
			.await
			.expect("sleep reply should send")
			.expect("sleep should succeed after grace completes");
		keep_awake
			.await
			.expect("keep-awake task should finish after release");
		assert_eq!(begin_sleep_count.load(Ordering::SeqCst), 1);
		assert_eq!(destroy_count.load(Ordering::SeqCst), 0);
		run.await
			.expect("task run should finish")
			.expect("task run should succeed");
	}

	#[tokio::test(start_paused = true)]
	async fn inline_shutdown_panic_returns_error_instead_of_crashing_task_loop() {
		let _hook_lock = test_hook_lock().lock().await;
		let ctx = new_with_kv(
			"actor-shutdown-step-panic",
			"task-shutdown-step-panic",
			Vec::new(),
			"local",
			new_in_memory(),
		);
		let _cleanup_hook =
			crate::actor::task::install_shutdown_cleanup_hook(Arc::new(move |ctx, _reason| {
				if ctx.actor_id() != "actor-shutdown-step-panic" {
					return;
				}
				panic!("boom");
			}));
		let (mut task, lifecycle_tx, _dispatch_tx, _events_tx) = new_task_with_senders(ctx.clone());
		task.factory = shutdown_ack_factory(ActorConfig::default());
		let run = tokio::spawn(task.run());

		let (start_tx, start_rx) = oneshot::channel();
		lifecycle_tx
			.send(LifecycleCommand::Start { reply: start_tx })
			.await
			.expect("start command should send");
		start_rx
			.await
			.expect("start reply should send")
			.expect("start should succeed");

		let (stop_tx, stop_rx) = oneshot::channel();
		lifecycle_tx
			.send(LifecycleCommand::Stop {
				reason: ShutdownKind::Destroy,
				reply: stop_tx,
			})
			.await
			.expect("destroy stop should send");
		let error = stop_rx
			.await
			.expect("destroy stop reply should send")
			.expect_err("shutdown panic should surface as an error reply");
		assert!(
			error.to_string().contains("internal_error"),
			"unexpected shutdown panic error: {error:#}"
		);
		let task_error = run
			.await
			.expect("task run should finish")
			.expect_err("task should return shutdown panic error");
		assert!(
			task_error
				.to_string()
				.contains("shutdown panicked during Destroy"),
			"unexpected task shutdown panic error: {task_error:#}"
		);
	}

	#[tokio::test(start_paused = true)]
	async fn destroy_marks_completion_before_shutdown_reply_is_sent() {
		let _hook_lock = test_hook_lock().lock().await;
		let ctx = new_with_kv(
			"actor-destroy-reply-order",
			"task-destroy-reply-order",
			Vec::new(),
			"local",
			new_in_memory(),
		);
		let hook_count = Arc::new(AtomicUsize::new(0));
		let _reply_hook = crate::actor::task::install_shutdown_reply_hook(Arc::new({
			let hook_count = hook_count.clone();
			move |ctx, reason| {
				if ctx.actor_id() != "actor-destroy-reply-order" {
					return;
				}
				if reason == ShutdownKind::Destroy {
					hook_count.fetch_add(1, Ordering::SeqCst);
					assert!(
						ctx.wait_for_destroy_completion_public()
							.now_or_never()
							.is_some(),
						"destroy completion should already be visible when the shutdown reply is sent"
					);
				}
			}
		}));
		let (mut task, lifecycle_tx, _dispatch_tx, _events_tx) = new_task_with_senders(ctx.clone());
		task.factory = shutdown_ack_factory(ActorConfig::default());
		let run = tokio::spawn(task.run());

		let (start_tx, start_rx) = oneshot::channel();
		lifecycle_tx
			.send(LifecycleCommand::Start { reply: start_tx })
			.await
			.expect("start command should send");
		start_rx
			.await
			.expect("start reply should send")
			.expect("start should succeed");

		let (stop_tx, stop_rx) = oneshot::channel();
		lifecycle_tx
			.send(LifecycleCommand::Stop {
				reason: ShutdownKind::Destroy,
				reply: stop_tx,
			})
			.await
			.expect("destroy stop should send");
		stop_rx
			.await
			.expect("destroy stop reply should send")
			.expect("destroy stop should succeed");
		assert_eq!(hook_count.load(Ordering::SeqCst), 1);
		run.await
			.expect("task run should finish")
			.expect("task run should succeed");
	}

	#[tokio::test]
	async fn clean_run_exit_still_dispatches_on_sleep_when_stop_arrives() {
		assert_clean_run_exit_stop_dispatches_cleanup(ShutdownKind::Sleep).await;
	}

	#[tokio::test]
	async fn clean_run_exit_still_dispatches_on_destroy_when_stop_arrives() {
		assert_clean_run_exit_stop_dispatches_cleanup(ShutdownKind::Destroy).await;
	}

	async fn assert_clean_run_exit_stop_dispatches_cleanup(reason: ShutdownKind) {
		let actor_id = match reason {
			ShutdownKind::Sleep => "actor-clean-run-sleep-stop",
			ShutdownKind::Destroy => "actor-clean-run-destroy-stop",
		};
		let ctx = new_with_kv(
			actor_id,
			"task-clean-run",
			Vec::new(),
			"local",
			new_in_memory(),
		);
		let sleep_count = Arc::new(AtomicUsize::new(0));
		let destroy_count = Arc::new(AtomicUsize::new(0));
		let (run_returned_tx, run_returned_rx) = oneshot::channel();
		let (cleanup_tx, cleanup_rx) = oneshot::channel();
		let mut task = new_task_with_factory(
			ctx.clone(),
			detached_cleanup_after_clean_run_factory(
				sleep_count.clone(),
				destroy_count.clone(),
				run_returned_tx,
				cleanup_tx,
			),
		);

		let (start_tx, start_rx) = oneshot::channel();
		task.handle_lifecycle(LifecycleCommand::Start { reply: start_tx })
			.await;
		start_rx
			.await
			.expect("start reply should send")
			.expect("start should succeed");
		timeout(Duration::from_secs(2), run_returned_rx)
			.await
			.expect("clean run should return")
			.expect("run returned signal should send");

		let outcome = ActorTask::wait_for_run_handle(task.run_handle.as_mut()).await;
		assert!(task.handle_run_handle_outcome(outcome).is_none());
		assert_eq!(task.lifecycle, LifecycleState::Started);

		let (stop_tx, stop_rx) = oneshot::channel();
		task.handle_lifecycle(LifecycleCommand::Stop {
			reason,
			reply: stop_tx,
		})
		.await;
		assert_eq!(
			timeout(Duration::from_secs(2), cleanup_rx)
				.await
				.expect("grace cleanup should run after Stop")
				.expect("cleanup signal should send"),
			reason
		);
		assert_eq!(
			sleep_count.load(Ordering::SeqCst),
			usize::from(matches!(reason, ShutdownKind::Sleep))
		);
		assert_eq!(
			destroy_count.load(Ordering::SeqCst),
			usize::from(matches!(reason, ShutdownKind::Destroy))
		);

		timeout(Duration::from_secs(2), async {
			while ctx.core_dispatched_hook_count() != 0 {
				yield_now().await;
			}
		})
		.await
		.expect("core-dispatched cleanup hook should complete");

		let exit = task
			.try_finish_grace()
			.expect("grace should finish after cleanup hook completes");
		let LiveExit::Shutdown {
			reason: shutdown_reason,
		} = exit
		else {
			panic!("grace should transition to shutdown");
		};
		assert_eq!(shutdown_reason, reason);
		let result = task.run_shutdown(shutdown_reason).await;
		task.deliver_shutdown_reply(shutdown_reason, &result);
		task.transition_to(LifecycleState::Terminated);
		result.expect("shutdown should succeed");
		stop_rx
			.await
			.expect("stop reply should send")
			.expect("stop should succeed");
	}

	#[tokio::test]
	async fn self_initiated_sleep_waits_for_stop_reply_before_shutdown_finishes() {
		let _hook_lock = test_hook_lock().lock().await;
		let ctx = new_with_kv(
			"actor-self-sleep-run-return",
			"task-self-sleep-run-return",
			Vec::new(),
			"local",
			new_in_memory(),
		);
		let cleanup_count = Arc::new(AtomicUsize::new(0));
		let _cleanup_hook = crate::actor::task::install_shutdown_cleanup_hook(Arc::new({
			let cleanup_count = cleanup_count.clone();
			move |ctx, reason| {
				if ctx.actor_id() == "actor-self-sleep-run-return" && reason == "sleep" {
					cleanup_count.fetch_add(1, Ordering::SeqCst);
				}
			}
		}));
		let factory = Arc::new(ActorFactory::new(ActorConfig::default(), move |start| {
			Box::pin(async move {
				start.ctx.sleep()?;
				Ok(())
			})
		}));
		let (mut task, lifecycle_tx, _dispatch_tx, _events_tx) = new_task_with_senders(ctx.clone());
		task.factory = factory;
		let run = tokio::spawn(task.run());

		let (start_tx, start_rx) = oneshot::channel();
		lifecycle_tx
			.send(LifecycleCommand::Start { reply: start_tx })
			.await
			.expect("start command should send");
		start_rx
			.await
			.expect("start reply should send")
			.expect("start should succeed");

		yield_now().await;
		assert_eq!(ctx.sleep_request_count(), 1);
		assert!(
			!run.is_finished(),
			"self-initiated sleep should stay live until the Stop arrives"
		);
		let (stop_tx, stop_rx) = oneshot::channel();
		lifecycle_tx
			.send(LifecycleCommand::Stop {
				reason: ShutdownKind::Sleep,
				reply: stop_tx,
			})
			.await
			.expect("sleep stop should send");
		stop_rx
			.await
			.expect("sleep stop reply should send")
			.expect("sleep stop should succeed");

		timeout(Duration::from_secs(2), run)
			.await
			.expect("self-initiated sleep shutdown should finish after Stop")
			.expect("task join should succeed")
			.expect("task run should succeed");
		assert_eq!(ctx.sleep_request_count(), 1);
		assert_eq!(cleanup_count.load(Ordering::SeqCst), 1);
	}

	#[tokio::test]
	async fn self_initiated_destroy_waits_for_stop_reply_and_marks_complete() {
		let _hook_lock = test_hook_lock().lock().await;
		let ctx = new_with_kv(
			"actor-self-destroy-run-return",
			"task-self-destroy-run-return",
			Vec::new(),
			"local",
			new_in_memory(),
		);
		let cleanup_count = Arc::new(AtomicUsize::new(0));
		let _cleanup_hook = crate::actor::task::install_shutdown_cleanup_hook(Arc::new({
			let cleanup_count = cleanup_count.clone();
			move |ctx, reason| {
				if ctx.actor_id() == "actor-self-destroy-run-return" && reason == "destroy" {
					cleanup_count.fetch_add(1, Ordering::SeqCst);
				}
			}
		}));
		let factory = Arc::new(ActorFactory::new(ActorConfig::default(), move |start| {
			Box::pin(async move {
				start.ctx.destroy()?;
				Ok(())
			})
		}));
		let (mut task, lifecycle_tx, _dispatch_tx, _events_tx) = new_task_with_senders(ctx.clone());
		task.factory = factory;
		let run = tokio::spawn(task.run());

		let (start_tx, start_rx) = oneshot::channel();
		lifecycle_tx
			.send(LifecycleCommand::Start { reply: start_tx })
			.await
			.expect("start command should send");
		start_rx
			.await
			.expect("start reply should send")
			.expect("start should succeed");

		yield_now().await;
		assert!(ctx.destroy_requested());
		assert!(
			!run.is_finished(),
			"self-initiated destroy should stay live until the Stop arrives"
		);
		let (stop_tx, stop_rx) = oneshot::channel();
		lifecycle_tx
			.send(LifecycleCommand::Stop {
				reason: ShutdownKind::Destroy,
				reply: stop_tx,
			})
			.await
			.expect("destroy stop should send");
		stop_rx
			.await
			.expect("destroy stop reply should send")
			.expect("destroy stop should succeed");

		timeout(Duration::from_secs(2), run)
			.await
			.expect("self-initiated destroy shutdown should finish after Stop")
			.expect("task join should succeed")
			.expect("task run should succeed");
		assert_eq!(cleanup_count.load(Ordering::SeqCst), 1);
		assert!(
			ctx.wait_for_destroy_completion_public()
				.now_or_never()
				.is_some(),
			"destroy completion should be marked after self-initiated shutdown"
		);
	}

	#[test]
	fn event_driven_drain_grep_gate_script_passes() {
		let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
		let script = manifest_dir.join("scripts/check-event-driven-drains.sh");
		let output = Command::new("bash")
			.arg(&script)
			.current_dir(&manifest_dir)
			.output()
			.expect("grep gate script should run");
		assert!(
			output.status.success(),
			"grep gate script failed\nstdout:\n{}\nstderr:\n{}",
			String::from_utf8_lossy(&output.stdout),
			String::from_utf8_lossy(&output.stderr)
		);
	}

	#[tokio::test(start_paused = true)]
	async fn drain_tracked_work_before_warning_threshold_does_not_emit_warning() {
		let _hook_lock = test_hook_lock().lock().await;
		let ctx = new_with_kv(
			"actor-drain-fast",
			"task-drain-fast",
			Vec::new(),
			"local",
			new_in_memory(),
		);
		let mut task = new_task(ctx.clone());
		let warning_count = Arc::new(AtomicUsize::new(0));
		let subscriber = Registry::default().with(LongShutdownDrainWarningLayer {
			count: warning_count.clone(),
		});
		let _guard = tracing::subscriber::set_default(subscriber);

		let (release_tx, release_rx) = oneshot::channel();
		ctx.wait_until(async move {
			let _ = release_rx.await;
		});

		let drain = task.drain_tracked_work(
			ShutdownKind::Destroy,
			"before_disconnect",
			Instant::now() + Duration::from_secs(5),
		);
		tokio::pin!(drain);
		assert!(poll!(drain.as_mut()).is_pending());

		advance(LONG_SHUTDOWN_DRAIN_WARNING_THRESHOLD - Duration::from_millis(1)).await;
		yield_now().await;
		assert!(poll!(drain.as_mut()).is_pending());
		assert_eq!(warning_count.load(Ordering::SeqCst), 0);

		release_tx.send(()).expect("release should send");
		assert!(poll_until_ready(&mut drain).await);
		assert_eq!(warning_count.load(Ordering::SeqCst), 0);
	}

	#[tokio::test(start_paused = true)]
	async fn drain_tracked_work_warns_once_after_threshold_then_finishes() {
		let _hook_lock = test_hook_lock().lock().await;
		let ctx = new_with_kv(
			"actor-drain-slow",
			"task-drain-slow",
			Vec::new(),
			"local",
			new_in_memory(),
		);
		let mut task = new_task(ctx.clone());
		let warning_count = Arc::new(AtomicUsize::new(0));
		let subscriber = Registry::default().with(LongShutdownDrainWarningLayer {
			count: warning_count.clone(),
		});
		let _guard = tracing::subscriber::set_default(subscriber);

		let (release_tx, release_rx) = oneshot::channel();
		ctx.wait_until(async move {
			let _ = release_rx.await;
		});

		let drain = task.drain_tracked_work(
			ShutdownKind::Sleep,
			"after_disconnect",
			Instant::now() + Duration::from_secs(5),
		);
		tokio::pin!(drain);
		assert!(poll!(drain.as_mut()).is_pending());

		advance(LONG_SHUTDOWN_DRAIN_WARNING_THRESHOLD + Duration::from_millis(1)).await;
		yield_now().await;
		assert!(poll!(drain.as_mut()).is_pending());
		assert_eq!(warning_count.load(Ordering::SeqCst), 1);

		advance(Duration::from_secs(2)).await;
		yield_now().await;
		assert!(poll!(drain.as_mut()).is_pending());
		assert_eq!(warning_count.load(Ordering::SeqCst), 1);

		release_tx.send(()).expect("release should send");
		assert!(poll_until_ready(&mut drain).await);
		assert_eq!(warning_count.load(Ordering::SeqCst), 1);
	}

	#[tokio::test]
	async fn run_logs_and_terminates_when_lifecycle_inbox_closes() {
		let warnings = run_task_with_closed_channel(ClosedChannelCase::LifecycleInbox).await;
		assert_eq!(
			warnings,
			vec![ClosedChannelWarning {
				actor_id: "actor-channel-lifecycle".to_owned(),
				channel: "lifecycle_inbox".to_owned(),
				reason: "all senders dropped".to_owned(),
				message: "actor task terminating because lifecycle command inbox closed".to_owned(),
			}]
		);
	}

	#[tokio::test]
	async fn run_logs_and_terminates_when_lifecycle_events_close() {
		let warnings = run_task_with_closed_channel(ClosedChannelCase::LifecycleEvents).await;
		assert_eq!(
			warnings,
			vec![ClosedChannelWarning {
				actor_id: "actor-channel-events".to_owned(),
				channel: "lifecycle_events".to_owned(),
				reason: "all senders dropped".to_owned(),
				message: "actor task terminating because lifecycle event inbox closed".to_owned(),
			}]
		);
	}

	#[tokio::test]
	async fn run_logs_and_terminates_when_dispatch_inbox_closes() {
		let warnings = run_task_with_closed_channel(ClosedChannelCase::DispatchInbox).await;
		assert_eq!(
			warnings,
			vec![ClosedChannelWarning {
				actor_id: "actor-channel-dispatch".to_owned(),
				channel: "dispatch_inbox".to_owned(),
				reason: "all senders dropped".to_owned(),
				message: "actor task terminating because dispatch inbox closed".to_owned(),
			}]
		);
	}

	#[tokio::test(flavor = "current_thread")]
	async fn actor_task_logs_lifecycle_dispatch_and_actor_event_flow() {
		let _hook_lock = test_hook_lock().lock().await;
		let ctx = new_with_kv(
			"actor-log-flow",
			"task-log-flow",
			Vec::new(),
			"local",
			new_in_memory(),
		);
		let (lifecycle_tx, lifecycle_rx) = mpsc::channel(4);
		let (dispatch_tx, dispatch_rx) = mpsc::channel(4);
		let (events_tx, events_rx) = mpsc::channel(4);
		ctx.configure_lifecycle_events(Some(events_tx));
		let factory = Arc::new(ActorFactory::new(Default::default(), |start| {
			Box::pin(async move {
				let mut events = start.events;
				while let Some(event) = events.recv().await {
					match event {
						ActorEvent::Action { reply, .. } => {
							reply.send(Ok(vec![1]));
						}
						ActorEvent::RunGracefulCleanup { reply, .. } => {
							reply.send(Ok(()));
							break;
						}
						_ => {}
					}
				}
				Ok(())
			})
		}));
		let task = ActorTask::new(
			"actor-log-flow".into(),
			0,
			lifecycle_rx,
			dispatch_rx,
			events_rx,
			factory,
			ctx,
			None,
			None,
		);
		let records = Arc::new(Mutex::new(Vec::new()));
		let subscriber = Registry::default().with(ActorTaskLogLayer {
			records: records.clone(),
		});
		let _guard = tracing::subscriber::set_default(subscriber);
		let run = tokio::spawn(task.run());

		let (start_tx, start_rx) = oneshot::channel();
		lifecycle_tx
			.send(LifecycleCommand::Start { reply: start_tx })
			.await
			.expect("start command should send");
		start_rx
			.await
			.expect("start reply should send")
			.expect("start should succeed");

		let (action_tx, action_rx) = oneshot::channel();
		dispatch_tx
			.send(DispatchCommand::Action {
				name: "ping".to_owned(),
				args: Vec::new(),
				conn: ConnHandle::new("conn-log-flow", Vec::new(), Vec::new(), false),
				reply: action_tx,
			})
			.await
			.expect("dispatch command should send");
		assert_eq!(
			action_rx
				.await
				.expect("dispatch reply should send")
				.expect("dispatch should succeed"),
			vec![1]
		);

		let (stop_tx, stop_rx) = oneshot::channel();
		lifecycle_tx
			.send(LifecycleCommand::Stop {
				reason: ShutdownKind::Destroy,
				reply: stop_tx,
			})
			.await
			.expect("stop command should send");
		stop_rx
			.await
			.expect("stop reply should send")
			.expect("stop should succeed");
		run.await
			.expect("task join should succeed")
			.expect("task should succeed");

		let logs = records
			.lock()
			.expect("actor-task log lock poisoned")
			.clone();
		assert!(logs.iter().any(|log| {
			log.level == tracing::Level::INFO
				&& log.actor_id.as_deref() == Some("actor-log-flow")
				&& log.message.as_deref() == Some("actor lifecycle transition")
				&& log.new.as_deref() == Some("Started")
		}));
		assert!(logs.iter().any(|log| {
			log.level == tracing::Level::DEBUG
				&& log.actor_id.as_deref() == Some("actor-log-flow")
				&& log.message.as_deref() == Some("actor lifecycle command received")
				&& log.command.as_deref() == Some("start")
		}));
		assert!(logs.iter().any(|log| {
			log.level == tracing::Level::DEBUG
				&& log.actor_id.as_deref() == Some("actor-log-flow")
				&& log.message.as_deref() == Some("actor event enqueued")
				&& log.event.as_deref() == Some("action")
		}));
		assert!(logs.iter().any(|log| {
			log.level == tracing::Level::DEBUG
				&& log.actor_id.as_deref() == Some("actor-log-flow")
				&& log.message.as_deref() == Some("actor event drained")
				&& log.event.as_deref() == Some("action")
		}));
	}

	#[tokio::test]
	async fn disconnect_hibernatable_connection_reaps_on_next_atomic_flush() {
		let kv = new_in_memory();
		let ctx = new_with_kv(
			"actor-disconnect",
			"task-disconnect",
			Vec::new(),
			"local",
			kv.clone(),
		);
		let disconnects = Arc::new(Mutex::new(Vec::<String>::new()));
		let conn = managed_test_conn(&ctx, "conn-hibernating", true, disconnects);
		conn.configure_hibernation(Some(HibernatableConnectionMetadata {
			gateway_id: *b"gate",
			request_id: *b"req1",
			server_message_index: 1,
			client_message_index: 2,
			request_path: "/ws".to_owned(),
			request_headers: BTreeMap::new(),
		}));
		ctx.add_conn(conn.clone());
		ctx.save_state(vec![StateDelta::ConnHibernation {
			conn: conn.id().into(),
			bytes: vec![1, 2, 3],
		}])
		.await
		.expect("seed hibernation should persist");
		assert_eq!(kv.test_batch_delete_call_count(), 0);

		conn.disconnect(Some("bye"))
			.await
			.expect("disconnect should succeed");
		assert!(ctx.conns().is_empty());

		ctx.save_state(Vec::new())
			.await
			.expect("flush should persist pending removal");

		assert_eq!(kv.test_batch_delete_call_count(), 0);
		let last_batch = kv
			.test_last_apply_batch()
			.expect("last apply batch should be recorded");
		assert_eq!(last_batch.puts, vec![]);
		assert_eq!(last_batch.deletes, vec![make_connection_key(conn.id())]);
		assert!(
			kv.get(&make_connection_key(conn.id()))
				.await
				.expect("persisted connection lookup should succeed")
				.is_none()
		);
	}

	#[tokio::test]
	async fn wake_start_filters_disconnected_hibernated_connections_and_reaps_them() {
		let kv = new_in_memory();
		let seed_ctx = new_with_kv(
			"actor-wake-prune",
			"task-wake",
			Vec::new(),
			"local",
			kv.clone(),
		);
		let live_conn = ConnHandle::new("conn-live", Vec::new(), Vec::new(), true);
		live_conn.configure_hibernation(Some(HibernatableConnectionMetadata {
			gateway_id: *b"gliv",
			request_id: *b"rliv",
			server_message_index: 4,
			client_message_index: 8,
			request_path: "/ws".to_owned(),
			request_headers: BTreeMap::new(),
		}));
		let stale_conn = ConnHandle::new("conn-stale", Vec::new(), Vec::new(), true);
		stale_conn.configure_hibernation(Some(HibernatableConnectionMetadata {
			gateway_id: *b"gstl",
			request_id: *b"rstl",
			server_message_index: 5,
			client_message_index: 9,
			request_path: "/ws".to_owned(),
			request_headers: BTreeMap::new(),
		}));
		seed_ctx.add_conn(live_conn.clone());
		seed_ctx.add_conn(stale_conn.clone());
		seed_ctx
			.save_state(vec![
				StateDelta::ConnHibernation {
					conn: live_conn.id().into(),
					bytes: vec![3, 2, 1],
				},
				StateDelta::ConnHibernation {
					conn: stale_conn.id().into(),
					bytes: vec![6, 5, 4],
				},
			])
			.await
			.expect("seed hibernations should persist");

		let ctx = new_with_kv(
			"actor-wake-prune",
			"task-wake",
			Vec::new(),
			"local",
			kv.clone(),
		);
		let (_lifecycle_tx, lifecycle_rx) = mpsc::channel(4);
		let (_dispatch_tx, dispatch_rx) = mpsc::channel(4);
		let (events_tx, events_rx) = mpsc::channel(4);
		ctx.configure_lifecycle_events(Some(events_tx));
		configure_live_hibernated_pairs(&ctx, [(b"gliv".as_slice(), b"rliv".as_slice())]);

		let (started_tx, started_rx) = oneshot::channel();
		let started_tx = Arc::new(Mutex::new(Some(started_tx)));
		let factory = Arc::new(ActorFactory::new(Default::default(), move |start| {
			let started_tx = started_tx.clone();
			Box::pin(async move {
				let mut events = start.events;
				let hibernated: Vec<(String, Vec<u8>)> = start
					.hibernated
					.into_iter()
					.map(|(conn, bytes)| (conn.id().to_owned(), bytes))
					.collect();
				started_tx
					.lock()
					.expect("started sender lock poisoned")
					.take()
					.expect("started sender should exist")
					.send(hibernated)
					.expect("started info should send");
				while let Some(event) = events.recv().await {
					match event {
						ActorEvent::SerializeState {
							reason: SerializeStateReason::Save,
							reply,
						} => {
							reply.send(Ok(Vec::new()));
						}
						ActorEvent::BeginSleep => {}
						ActorEvent::FinalizeSleep { reply } | ActorEvent::Destroy { reply } => {
							reply.send(Ok(()));
							break;
						}
						ActorEvent::ConnectionOpen { .. } => {
							panic!("hibernated connection should not refire ConnectionOpen");
						}
						_ => {}
					}
				}
				Ok(())
			})
		}));

		let mut task = ActorTask::new(
			"actor-wake-prune".into(),
			0,
			lifecycle_rx,
			dispatch_rx,
			events_rx,
			factory,
			ctx.clone(),
			None,
			None,
		);
		let (start_tx, start_rx) = oneshot::channel();
		task.handle_lifecycle(LifecycleCommand::Start { reply: start_tx })
			.await;
		start_rx
			.await
			.expect("start reply should send")
			.expect("start should succeed");

		let hibernated = started_rx.await.expect("start info should send");
		assert_eq!(hibernated, vec![("conn-live".to_owned(), vec![3, 2, 1])]);

		task.handle_event(crate::actor::task::LifecycleEvent::SaveRequested { immediate: false })
			.await;
		task.on_state_save_tick().await;

		let last_batch = kv
			.test_last_apply_batch()
			.expect("last apply batch should be recorded");
		assert_eq!(last_batch.deletes, vec![make_connection_key("conn-stale")]);
		assert!(
			kv.get(&make_connection_key("conn-stale"))
				.await
				.expect("persisted connection lookup should succeed")
				.is_none()
		);

		task.handle_stop(ShutdownKind::Sleep)
			.await
			.expect("sleep stop should succeed");
	}

	#[tokio::test]
	async fn wake_start_reaps_dead_hibernated_connections_without_engine_registration() {
		let kv = new_in_memory();
		let seed_ctx = new_with_kv(
			"actor-wake-dead",
			"task-wake",
			Vec::new(),
			"local",
			kv.clone(),
		);
		let dead_conn = ConnHandle::new("conn-dead", Vec::new(), Vec::new(), true);
		dead_conn.configure_hibernation(Some(HibernatableConnectionMetadata {
			gateway_id: *b"gded",
			request_id: *b"rded",
			server_message_index: 7,
			client_message_index: 11,
			request_path: "/ws".to_owned(),
			request_headers: BTreeMap::new(),
		}));
		seed_ctx.add_conn(dead_conn.clone());
		seed_ctx
			.save_state(vec![StateDelta::ConnHibernation {
				conn: dead_conn.id().into(),
				bytes: vec![9, 8, 7],
			}])
			.await
			.expect("seed hibernation should persist");

		let ctx = new_with_kv(
			"actor-wake-dead",
			"task-wake",
			Vec::new(),
			"local",
			kv.clone(),
		);
		let (_lifecycle_tx, lifecycle_rx) = mpsc::channel(4);
		let (_dispatch_tx, dispatch_rx) = mpsc::channel(4);
		let (events_tx, events_rx) = mpsc::channel(4);
		ctx.configure_lifecycle_events(Some(events_tx));
		ctx.set_hibernated_connection_liveness_override(std::iter::empty());

		let (started_tx, started_rx) = oneshot::channel();
		let started_tx = Arc::new(Mutex::new(Some(started_tx)));
		let factory = Arc::new(ActorFactory::new(Default::default(), move |start| {
			let started_tx = started_tx.clone();
			Box::pin(async move {
				let mut events = start.events;
				started_tx
					.lock()
					.expect("started sender lock poisoned")
					.take()
					.expect("started sender should exist")
					.send(
						start
							.hibernated
							.into_iter()
							.map(|(conn, _)| conn.id().to_owned())
							.collect::<Vec<_>>(),
					)
					.expect("started info should send");
				while let Some(event) = events.recv().await {
					match event {
						ActorEvent::SerializeState {
							reason: SerializeStateReason::Save,
							reply,
						} => {
							reply.send(Ok(Vec::new()));
						}
						ActorEvent::BeginSleep => {}
						ActorEvent::FinalizeSleep { reply } | ActorEvent::Destroy { reply } => {
							reply.send(Ok(()));
							break;
						}
						ActorEvent::ConnectionOpen { .. } => {
							panic!("dead hibernated connection should not refire ConnectionOpen");
						}
						_ => {}
					}
				}
				Ok(())
			})
		}));

		let mut task = ActorTask::new(
			"actor-wake-dead".into(),
			0,
			lifecycle_rx,
			dispatch_rx,
			events_rx,
			factory,
			ctx.clone(),
			None,
			None,
		);
		let (start_tx, start_rx) = oneshot::channel();
		task.handle_lifecycle(LifecycleCommand::Start { reply: start_tx })
			.await;
		start_rx
			.await
			.expect("start reply should send")
			.expect("start should succeed");

		assert_eq!(
			started_rx.await.expect("start info should send"),
			Vec::<String>::new()
		);
		assert!(ctx.conns().is_empty());

		task.handle_event(crate::actor::task::LifecycleEvent::SaveRequested { immediate: false })
			.await;
		task.on_state_save_tick().await;

		let last_batch = kv
			.test_last_apply_batch()
			.expect("last apply batch should be recorded");
		assert_eq!(last_batch.deletes, vec![make_connection_key("conn-dead")]);
		assert!(
			kv.get(&make_connection_key("conn-dead"))
				.await
				.expect("persisted connection lookup should succeed")
				.is_none()
		);

		task.handle_stop(ShutdownKind::Sleep)
			.await
			.expect("sleep stop should succeed");
	}
}
