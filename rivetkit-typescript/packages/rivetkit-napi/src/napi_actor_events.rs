use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::Result;
use parking_lot::Mutex;
use rivet_error::{MacroMarker, RivetError as RivetTransportError, RivetErrorSchema};
use rivetkit_core::{
	ActorContext as CoreActorContext, ActorEvent, ActorEvents, ActorLifecycle, ActorStart,
	QueueSendResult, QueueSendStatus, Reply, SerializeStateReason, StateDelta,
};
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
use tokio::task::JoinHandle;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use crate::NapiInvalidState;
#[cfg(test)]
use crate::actor_context::EndReason;
use crate::actor_context::{ActorContext, RegisteredTask, state_deltas_from_payload};
use crate::actor_factory::{
	ActionPayload, AdapterConfig, BeforeActionResponsePayload, BeforeConnectPayload,
	BeforeSubscribePayload, CallbackBindings, ConnectionPayload, CreateConnStatePayload,
	CreateStatePayload, HttpRequestPayload, LifecyclePayload, MigratePayload, QueueSendPayload,
	SerializeStatePayload, WebSocketPayload, WorkflowHistoryPayload, WorkflowReplayPayload,
	call_buffer, call_optional_buffer, call_queue_send, call_request, call_state_delta_payload,
	call_void,
};

// Restart hooks are synchronous callback slots; the guard is only held while
// swapping task handles, never while awaiting a task shutdown.
type RunHandlerSlot = Arc<Mutex<Option<JoinHandle<()>>>>;

struct RunHandlerActiveGuard {
	ctx: CoreActorContext,
}

struct DispatchCancelGuard {
	token: CancellationToken,
}

impl RunHandlerActiveGuard {
	fn new(ctx: CoreActorContext) -> Self {
		ctx.begin_run_handler();
		Self { ctx }
	}
}

impl DispatchCancelGuard {
	fn new() -> Self {
		Self {
			token: CancellationToken::new(),
		}
	}

	fn token(&self) -> CancellationToken {
		self.token.clone()
	}
}

impl Drop for RunHandlerActiveGuard {
	fn drop(&mut self) {
		self.ctx.end_run_handler();
	}
}

impl Drop for DispatchCancelGuard {
	fn drop(&mut self) {
		self.token.cancel();
	}
}

static ACTION_TIMED_OUT_SCHEMA: RivetErrorSchema = RivetErrorSchema {
	group: "actor",
	code: "action_timed_out",
	default_message: "Action timed out",
	meta_type: None,
	_macro_marker: MacroMarker { _private: () },
};

static CALLBACK_TIMED_OUT_SCHEMA: RivetErrorSchema = RivetErrorSchema {
	group: "actor",
	code: "callback_timed_out",
	default_message: "Lifecycle callback timed out",
	meta_type: None,
	_macro_marker: MacroMarker { _private: () },
};

pub(crate) async fn run_adapter_loop(
	bindings: Arc<CallbackBindings>,
	config: Arc<AdapterConfig>,
	start: ActorStart,
) -> Result<()> {
	let ActorStart {
		ctx: core_ctx,
		input,
		snapshot,
		hibernated,
		mut events,
		startup_ready,
	} = start;

	let ctx = ActorContext::new(core_ctx.clone());
	ctx.reset_runtime_shared_state();
	let abort = CancellationToken::new();
	ctx.attach_napi_abort_token(abort.clone());
	let (registered_task_tx, mut registered_task_rx) = unbounded_channel();
	ctx.attach_task_sender(registered_task_tx);

	let dirty = Arc::new(AtomicBool::new(false));
	core_ctx.on_request_save(Box::new({
		let dirty = Arc::clone(&dirty);
		move |_opts| {
			dirty.store(true, Ordering::Release);
		}
	}));

	let mut tasks = JoinSet::new();
	let run_handler = match run_preamble(
		&bindings,
		config.as_ref(),
		&ctx,
		input.as_deref(),
		snapshot,
		hibernated,
	)
	.await
	{
		Ok(run_handler) => {
			if let Some(reply) = startup_ready {
				let _ = reply.send(Ok(()));
			}
			run_handler
		}
		Err(error) => {
			if let Some(reply) = startup_ready {
				let startup_error = anyhow::Error::new(RivetTransportError::extract(&error));
				let _ = reply.send(Err(startup_error));
			}
			return Err(error);
		}
	};

	run_event_loop(
		&bindings,
		config.as_ref(),
		&ctx,
		&abort,
		&mut tasks,
		&mut registered_task_rx,
		&dirty,
		&mut events,
	)
	.await;

	stop_run_handler(&run_handler).await;
	abort.cancel();
	drain_tasks(&mut tasks, &mut registered_task_rx).await;
	Ok(())
}

async fn run_event_loop(
	bindings: &Arc<CallbackBindings>,
	config: &AdapterConfig,
	ctx: &ActorContext,
	abort: &CancellationToken,
	tasks: &mut JoinSet<()>,
	registered_task_rx: &mut UnboundedReceiver<RegisteredTask>,
	dirty: &Arc<AtomicBool>,
	events: &mut ActorEvents,
) {
	while let Some(event) = events.recv().await {
		pump_registered_tasks(tasks, registered_task_rx);
		dispatch_event(
			event,
			bindings,
			config,
			ctx,
			abort,
			tasks,
			registered_task_rx,
			dirty,
		)
		.await;
		if ctx.has_end_reason() {
			break;
		}
	}
}

async fn run_preamble(
	bindings: &CallbackBindings,
	config: &AdapterConfig,
	ctx: &ActorContext,
	input: Option<&[u8]>,
	snapshot: Option<Vec<u8>>,
	hibernated: Vec<(rivetkit_core::ConnHandle, Vec<u8>)>,
) -> Result<RunHandlerSlot> {
	let is_new = snapshot.is_none();

	if is_new {
		if let Some(callback) = &bindings.create_state {
			let bytes = with_timeout(
				"createState",
				config.create_state_timeout,
				call_create_state(callback, ctx, input),
			)
			.await?;
			ctx.set_state_initial(bytes)?;
		}
		if let Some(callback) = &bindings.on_create {
			with_timeout(
				"onCreate",
				config.on_create_timeout,
				call_on_create(callback, ctx, input),
			)
			.await?;
		}
		ctx.mark_has_initialized_and_flush().await?;
	} else {
		let snapshot = snapshot.ok_or_else(|| {
			NapiInvalidState {
				state: "actor wake snapshot".to_owned(),
				reason: "wake path did not include a persisted snapshot".to_owned(),
			}
			.build()
		})?;
		ctx.set_state_initial(snapshot)?;
		for (conn, bytes) in hibernated {
			ctx.restore_hibernatable_conn(conn, bytes)?;
		}
	}

	if let Some(callback) = &bindings.create_vars {
		with_timeout(
			"createVars",
			config.create_vars_timeout,
			call_create_vars(callback, ctx),
		)
		.await?;
	}

	if let Some(callback) = &bindings.on_migrate {
		with_timeout(
			"onMigrate",
			config.on_migrate_timeout,
			call_on_migrate(callback, ctx, is_new),
		)
		.await?;
	}

	ctx.init_alarms().await?;
	ctx.mark_ready_internal();

	if let Some(callback) = &bindings.on_wake {
		with_timeout(
			"onWake",
			config.on_wake_timeout,
			call_on_wake(callback, ctx),
		)
		.await?;
	}

	if let Some(callback) = &bindings.on_before_actor_start {
		with_timeout(
			"onBeforeActorStart",
			config.on_before_actor_start_timeout,
			call_on_before_actor_start(callback, ctx),
		)
		.await?;
	}

	ctx.mark_started_internal()?;
	let run_handler = configure_run_handler(bindings, ctx);
	if bindings.run.is_some() {
		tokio::task::yield_now().await;
	}
	ctx.drain_overdue_scheduled_events().await?;

	Ok(run_handler)
}

fn configure_run_handler(bindings: &CallbackBindings, ctx: &ActorContext) -> RunHandlerSlot {
	let run_handler = Arc::new(Mutex::new(None));
	let Some(callback) = bindings.run.as_ref().cloned() else {
		return run_handler;
	};

	let restart_slot = Arc::clone(&run_handler);
	let restart_ctx = ctx.clone();
	let restart_callback = callback.clone();

	ctx.attach_run_restart(move || {
		let mut guard = restart_slot.lock();
		if let Some(handle) = guard.take() {
			handle.abort();
		}
		*guard = Some(spawn_run_handler(
			restart_callback.clone(),
			restart_ctx.clone(),
		));
		Ok(())
	});

	{
		let mut guard = run_handler.lock();
		*guard = Some(spawn_run_handler(callback, ctx.clone()));
	}

	run_handler
}

#[tracing::instrument(
	skip_all,
	fields(actor_id = %ctx.inner().actor_id()),
)]
pub(crate) async fn dispatch_event(
	event: ActorEvent,
	bindings: &Arc<CallbackBindings>,
	config: &AdapterConfig,
	ctx: &ActorContext,
	abort: &CancellationToken,
	tasks: &mut JoinSet<()>,
	_registered_task_rx: &mut UnboundedReceiver<RegisteredTask>,
	dirty: &Arc<AtomicBool>,
) {
	let _ = dirty;

	match event {
		ActorEvent::Action {
			name,
			args,
			conn,
			reply,
		} => {
			tracing::info!(
				actor_id = %ctx.inner().actor_id(),
				action_name = %name,
				args_len = args.len(),
				has_conn = conn.is_some(),
				"napi: dispatching ActorEvent::Action to JS"
			);
			let Some(callback) = bindings.actions.get(&name).cloned() else {
				tracing::warn!(
					actor_id = %ctx.inner().actor_id(),
					action_name = %name,
					"napi: no action callback registered",
				);
				reply.send(Err(action_not_found(name)));
				return;
			};
			let on_before_action_response = bindings.on_before_action_response.clone();
			let timeout = config.action_timeout;
			let ctx = ctx.clone();

			spawn_reply(tasks, abort.clone(), reply, async move {
				tracing::info!(action_name = %name, "napi: invoking action JS callback");
				let output = with_dispatch_cancel_token(|cancel_token| {
					with_structured_timeout(
						"actor",
						"action_timed_out",
						"Action timed out",
						None,
						timeout,
						call_action(
							&callback,
							&ctx,
							conn,
							name.clone(),
							args.clone(),
							Some(cancel_token),
						),
					)
				})
				.await?;
				tracing::info!(
					action_name = %name,
					output_len = output.len(),
					"napi: action JS callback returned"
				);

				if let Some(callback) = on_before_action_response {
					with_structured_timeout(
						"actor",
						"action_timed_out",
						"Action timed out",
						None,
						timeout,
						call_on_before_action_response(&callback, &ctx, name, args, output),
					)
					.await
				} else {
					Ok(output)
				}
			});
		}
		ActorEvent::HttpRequest { request, reply } => {
			let Some(callback) = bindings.on_request.clone() else {
				reply.send(Err(missing_callback("onRequest")));
				return;
			};
			let ctx = ctx.clone();
			let timeout = config.on_request_timeout;
			spawn_reply(tasks, abort.clone(), reply, async move {
				with_dispatch_cancel_token(|cancel_token| {
					with_structured_timeout(
						"actor",
						"action_timed_out",
						"Action timed out",
						None,
						timeout,
						async move {
							call_http_request(&callback, &ctx, request, Some(cancel_token)).await
						},
					)
				})
				.await
			});
		}
		ActorEvent::QueueSend {
			name,
			body,
			conn,
			request,
			wait,
			timeout_ms,
			reply,
		} => {
			let Some(callback) = bindings.on_queue_send.clone() else {
				reply.send(Err(missing_callback("onQueueSend")));
				return;
			};
			let ctx = ctx.clone();
			let timeout = config.on_request_timeout;
			spawn_reply(tasks, abort.clone(), reply, async move {
				with_dispatch_cancel_token(|cancel_token| {
					with_structured_timeout(
						"actor",
						"action_timed_out",
						"Action timed out",
						None,
						timeout,
						async move {
							let result = call_queue_send(
								"onQueueSend",
								&callback,
								QueueSendPayload {
									ctx: ctx.inner().clone(),
									conn,
									request,
									name,
									body,
									wait,
									timeout_ms,
									cancel_token: Some(cancel_token),
								},
							)
							.await?;
							let status = match result.status.as_str() {
								"completed" => QueueSendStatus::Completed,
								"timedOut" => QueueSendStatus::TimedOut,
								other => {
									return Err(NapiInvalidState {
										state: "queue send status".to_owned(),
										reason: format!("invalid status `{other}`"),
									}
									.build());
								}
							};
							Ok(QueueSendResult {
								status,
								response: result.response.map(|buffer| buffer.to_vec()),
							})
						},
					)
				})
				.await
			});
		}
		ActorEvent::WebSocketOpen { ws, request, reply } => {
			let Some(callback) = bindings.on_websocket.clone() else {
				reply.send(Ok(()));
				return;
			};
			let ctx = ctx.clone();
			spawn_reply(tasks, abort.clone(), reply, async move {
				call_on_websocket(&callback, &ctx, ws, request).await
			});
		}
		ActorEvent::ConnectionOpen {
			conn,
			params,
			request,
			reply,
		} => {
			let on_before_connect = bindings.on_before_connect.clone();
			let create_conn_state = bindings.create_conn_state.clone();
			let on_connect = bindings.on_connect.clone();
			let timeout = config.on_before_connect_timeout;
			let connect_timeout = config.on_connect_timeout;
			let create_conn_state_timeout = config.create_conn_state_timeout;
			let ctx = ctx.clone();

			spawn_reply(tasks, abort.clone(), reply, async move {
				if let Some(callback) = on_before_connect {
					with_timeout(
						"onBeforeConnect",
						timeout,
						call_on_before_connect(&callback, &ctx, params.clone(), request.clone()),
					)
					.await?;
				}

				if let Some(callback) = create_conn_state {
					let state = with_timeout(
						"createConnState",
						create_conn_state_timeout,
						call_create_conn_state(
							&callback,
							&ctx,
							conn.clone(),
							params.clone(),
							request.clone(),
						),
					)
					.await?;
					ctx.set_conn_state_initial(&conn, state)?;
				}

				if let Some(callback) = on_connect {
					with_timeout(
						"onConnect",
						connect_timeout,
						call_on_connect(&callback, &ctx, conn, request),
					)
					.await?;
				}

				Ok(())
			});
		}
		ActorEvent::ConnectionClosed { conn } => {
			let Some(callback) = bindings.on_disconnect_final.clone() else {
				return;
			};
			let ctx = ctx.clone();
			let actor_id = ctx.inner().actor_id().to_owned();
			let timeout = config.on_connect_timeout;
			spawn_task(tasks, abort.clone(), actor_id, async move {
				with_timeout(
					"onDisconnect",
					timeout,
					call_on_disconnect_final(&callback, &ctx, conn),
				)
				.await
			});
		}
		ActorEvent::SubscribeRequest {
			conn,
			event_name,
			reply,
		} => {
			let Some(callback) = bindings.on_before_subscribe.clone() else {
				reply.send(Ok(()));
				return;
			};
			let ctx = ctx.clone();
			let timeout = config.on_before_connect_timeout;
			spawn_reply(tasks, abort.clone(), reply, async move {
				with_timeout(
					"onBeforeSubscribe",
					timeout,
					call_on_before_subscribe(&callback, &ctx, conn, event_name),
				)
				.await
			});
		}
		ActorEvent::SerializeState { reason, reply } => {
			reply.send(maybe_serialize(bindings.as_ref(), ctx, dirty.as_ref(), reason).await);
		}
		ActorEvent::RunGracefulCleanup { reason, reply } => {
			let callback = match reason {
				rivetkit_core::actor::StopReason::Sleep => bindings.on_sleep.clone(),
				rivetkit_core::actor::StopReason::Destroy => bindings.on_destroy.clone(),
			};
			let ctx = ctx.clone();
			tasks.spawn(async move {
				let result: Result<()> = async {
					if let Some(callback) = callback {
						match reason {
							rivetkit_core::actor::StopReason::Sleep => {
								call_on_sleep(&callback, &ctx).await
							}
							rivetkit_core::actor::StopReason::Destroy => {
								call_on_destroy(&callback, &ctx).await
							}
						}?;
					}
					Ok(())
				}
				.await;
				if let Err(error) = result {
					tracing::error!(
						actor_id = %ctx.inner().actor_id(),
						?error,
						"graceful cleanup callback failed",
					);
				}
				reply.send(Ok(()));
			});
		}
		ActorEvent::DisconnectConn { conn_id, reply } => {
			let callback = bindings.on_disconnect_final.clone();
			let ctx = ctx.clone();
			tasks.spawn(async move {
				let result: Result<()> = async {
					let conn = { ctx.inner().conns().find(|conn| conn.id() == conn_id) };
					if let Some(conn) = conn {
						if let Some(callback) = callback {
							call_on_disconnect_final(&callback, &ctx, conn.clone()).await?;
						}
						ctx.inner().disconnect_conn(conn_id).await?;
					}
					Ok(())
				}
				.await;
				if let Err(error) = result {
					tracing::error!(
						actor_id = %ctx.inner().actor_id(),
						?error,
						"disconnect cleanup callback failed",
					);
				}
				reply.send(Ok(()));
			});
		}
		ActorEvent::WorkflowHistoryRequested { reply } => {
			let Some(callback) = bindings.get_workflow_history.clone() else {
				reply.send(Ok(None));
				return;
			};
			let ctx = ctx.clone();
			spawn_reply(tasks, abort.clone(), reply, async move {
				call_workflow_history(&callback, &ctx).await
			});
		}
		ActorEvent::WorkflowReplayRequested { entry_id, reply } => {
			let Some(callback) = bindings.replay_workflow.clone() else {
				reply.send(Ok(None));
				return;
			};
			let ctx = ctx.clone();
			spawn_reply(tasks, abort.clone(), reply, async move {
				call_workflow_replay(&callback, &ctx, entry_id).await
			});
		}
	}
}

async fn maybe_serialize(
	bindings: &CallbackBindings,
	ctx: &ActorContext,
	dirty: &AtomicBool,
	reason: SerializeStateReason,
) -> Result<Vec<StateDelta>> {
	// The adapter dirty bit is consumed only by persistence-bound serialization.
	// Inspector snapshots feed the live overlay and must not steal a pending save.
	maybe_serialize_with(
		bindings,
		ctx,
		dirty,
		reason,
		|bindings, ctx, reason| async move { call_serialize_state(bindings, ctx, reason).await },
	)
	.await
}

async fn maybe_serialize_with<'a, F, Fut>(
	bindings: &'a CallbackBindings,
	ctx: &'a ActorContext,
	dirty: &AtomicBool,
	reason: SerializeStateReason,
	serialize: F,
) -> Result<Vec<StateDelta>>
where
	F: FnOnce(&'a CallbackBindings, &'a ActorContext, &'static str) -> Fut,
	Fut: std::future::Future<Output = Result<Vec<StateDelta>>> + 'a,
{
	if reason != SerializeStateReason::Inspector && !dirty.swap(false, Ordering::AcqRel) {
		return Ok(Vec::new());
	}

	serialize(bindings, ctx, serialize_state_reason_name(reason)).await
}

async fn call_serialize_state(
	bindings: &CallbackBindings,
	ctx: &ActorContext,
	reason: &'static str,
) -> Result<Vec<StateDelta>> {
	let callback = bindings
		.serialize_state
		.as_ref()
		.ok_or_else(|| missing_callback("serializeState"))?;
	let payload = call_state_delta_payload(
		"serializeState",
		callback,
		SerializeStatePayload {
			ctx: ctx.inner().clone(),
			reason: reason.to_owned(),
		},
	)
	.await?;
	Ok(state_deltas_from_payload(payload))
}

fn serialize_state_reason_name(reason: SerializeStateReason) -> &'static str {
	match reason {
		SerializeStateReason::Save => "save",
		SerializeStateReason::Inspector => "inspector",
	}
}

#[allow(dead_code)]
pub(crate) fn spawn_reply<T, F>(
	tasks: &mut JoinSet<()>,
	abort: CancellationToken,
	reply: Reply<T>,
	work: F,
) where
	T: Send + 'static,
	F: std::future::Future<Output = Result<T>> + Send + 'static,
{
	tasks.spawn(async move {
		tokio::select! {
			_ = abort.cancelled() => {
				reply.send(Err(actor_shutting_down()));
			}
			result = work => {
				reply.send(result);
			}
		}
	});
}

fn spawn_task<F>(tasks: &mut JoinSet<()>, abort: CancellationToken, actor_id: String, work: F)
where
	F: std::future::Future<Output = Result<()>> + Send + 'static,
{
	tasks.spawn(async move {
		tokio::select! {
			_ = abort.cancelled() => {}
			result = work => {
				if let Err(error) = result {
					tracing::error!(actor_id, ?error, "napi background callback failed");
				}
			}
		}
	});
}

pub(crate) async fn drain_tasks(
	tasks: &mut JoinSet<()>,
	registered_task_rx: &mut UnboundedReceiver<RegisteredTask>,
) {
	loop {
		pump_registered_tasks(tasks, registered_task_rx);

		if tasks.is_empty() {
			break;
		}

		if let Some(result) = tasks.join_next().await {
			if let Err(error) = result {
				tracing::error!(?error, "napi background task failed to join");
			}
		}
	}
}

fn pump_registered_tasks(
	tasks: &mut JoinSet<()>,
	registered_task_rx: &mut UnboundedReceiver<RegisteredTask>,
) {
	while let Ok(task) = registered_task_rx.try_recv() {
		tasks.spawn(task);
	}
}

async fn stop_run_handler(run_handler: &RunHandlerSlot) {
	let handle = {
		let mut guard = run_handler.lock();
		guard.take()
	};

	if let Some(handle) = handle {
		handle.abort();
		let _ = handle.await;
	}
}

async fn with_timeout<T, F>(callback_name: &str, duration: Duration, future: F) -> Result<T>
where
	F: std::future::Future<Output = Result<T>>,
{
	with_structured_timeout(
		"actor",
		"callback_timed_out",
		format!(
			"callback `{callback_name}` timed out after {} ms",
			duration.as_millis()
		),
		callback_timeout_metadata(callback_name, duration),
		duration,
		future,
	)
	.await
}

async fn with_structured_timeout<T, F>(
	group: &'static str,
	code: &'static str,
	message: impl Into<String>,
	meta: Option<Box<serde_json::value::RawValue>>,
	duration: Duration,
	future: F,
) -> Result<T>
where
	F: std::future::Future<Output = Result<T>>,
{
	let message = message.into();
	let schema = structured_timeout_schema(group, code, &message);
	tokio::time::timeout(duration, future)
		.await
		.map_err(|_| structured_timeout_error(schema, message, meta))?
}

fn structured_timeout_schema(
	group: &'static str,
	code: &'static str,
	message: &str,
) -> &'static RivetErrorSchema {
	match (group, code) {
		("actor", "action_timed_out") => &ACTION_TIMED_OUT_SCHEMA,
		("actor", "callback_timed_out") => &CALLBACK_TIMED_OUT_SCHEMA,
		_ => Box::leak(Box::new(RivetErrorSchema {
			group,
			code,
			default_message: Box::leak(message.to_owned().into_boxed_str()),
			meta_type: None,
			_macro_marker: MacroMarker { _private: () },
		})),
	}
}

fn structured_timeout_error(
	schema: &'static RivetErrorSchema,
	message: impl Into<String>,
	meta: Option<Box<serde_json::value::RawValue>>,
) -> anyhow::Error {
	anyhow::Error::new(RivetTransportError {
		schema,
		meta,
		message: Some(message.into()),
	})
}

fn callback_timeout_metadata(
	callback_name: &str,
	duration: Duration,
) -> Option<Box<serde_json::value::RawValue>> {
	let duration_ms = u64::try_from(duration.as_millis()).unwrap_or(u64::MAX);
	let metadata = serde_json::json!({
		"callback_name": callback_name,
		"duration_ms": duration_ms,
	});

	serde_json::value::to_raw_value(&metadata).ok()
}

fn spawn_run_handler(
	callback: crate::actor_factory::CallbackTsfn<LifecyclePayload>,
	ctx: ActorContext,
) -> JoinHandle<()> {
	let run_handler_active = RunHandlerActiveGuard::new(ctx.inner().clone());
	tokio::spawn(async move {
		let _run_handler_active = run_handler_active;
		match call_run(&callback, &ctx).await {
			Ok(()) => {
				tracing::debug!(
					actor_id = %ctx.inner().actor_id(),
					"napi run handler exited cleanly"
				);
			}
			Err(error) => {
				tracing::error!(
					actor_id = %ctx.inner().actor_id(),
					?error,
					"napi run handler failed"
				);
			}
		}
	})
}

async fn call_create_state(
	callback: &crate::actor_factory::CallbackTsfn<CreateStatePayload>,
	ctx: &ActorContext,
	input: Option<&[u8]>,
) -> Result<Vec<u8>> {
	call_buffer(
		"createState",
		callback,
		CreateStatePayload {
			ctx: ctx.inner().clone(),
			input: input.map(|input| input.to_vec()),
		},
	)
	.await
}

async fn call_on_create(
	callback: &crate::actor_factory::CallbackTsfn<CreateStatePayload>,
	ctx: &ActorContext,
	input: Option<&[u8]>,
) -> Result<()> {
	call_void(
		"onCreate",
		callback,
		CreateStatePayload {
			ctx: ctx.inner().clone(),
			input: input.map(|input| input.to_vec()),
		},
	)
	.await
}

async fn call_create_vars(
	callback: &crate::actor_factory::CallbackTsfn<LifecyclePayload>,
	ctx: &ActorContext,
) -> Result<()> {
	call_void(
		"createVars",
		callback,
		LifecyclePayload {
			ctx: ctx.inner().clone(),
		},
	)
	.await
}

async fn call_on_migrate(
	callback: &crate::actor_factory::CallbackTsfn<MigratePayload>,
	ctx: &ActorContext,
	is_new: bool,
) -> Result<()> {
	call_void(
		"onMigrate",
		callback,
		MigratePayload {
			ctx: ctx.inner().clone(),
			is_new,
		},
	)
	.await
}

async fn call_on_wake(
	callback: &crate::actor_factory::CallbackTsfn<LifecyclePayload>,
	ctx: &ActorContext,
) -> Result<()> {
	call_void(
		"onWake",
		callback,
		LifecyclePayload {
			ctx: ctx.inner().clone(),
		},
	)
	.await
}

async fn call_on_before_actor_start(
	callback: &crate::actor_factory::CallbackTsfn<LifecyclePayload>,
	ctx: &ActorContext,
) -> Result<()> {
	call_void(
		"onBeforeActorStart",
		callback,
		LifecyclePayload {
			ctx: ctx.inner().clone(),
		},
	)
	.await
}

async fn call_on_sleep(
	callback: &crate::actor_factory::CallbackTsfn<LifecyclePayload>,
	ctx: &ActorContext,
) -> Result<()> {
	call_void(
		"onSleep",
		callback,
		LifecyclePayload {
			ctx: ctx.inner().clone(),
		},
	)
	.await
}

async fn call_on_destroy(
	callback: &crate::actor_factory::CallbackTsfn<LifecyclePayload>,
	ctx: &ActorContext,
) -> Result<()> {
	call_void(
		"onDestroy",
		callback,
		LifecyclePayload {
			ctx: ctx.inner().clone(),
		},
	)
	.await
}

async fn call_run(
	callback: &crate::actor_factory::CallbackTsfn<LifecyclePayload>,
	ctx: &ActorContext,
) -> Result<()> {
	call_void(
		"run",
		callback,
		LifecyclePayload {
			ctx: ctx.inner().clone(),
		},
	)
	.await
}

async fn call_action(
	callback: &crate::actor_factory::CallbackTsfn<ActionPayload>,
	ctx: &ActorContext,
	conn: Option<rivetkit_core::ConnHandle>,
	name: String,
	args: Vec<u8>,
	cancel_token: Option<CancellationToken>,
) -> Result<Vec<u8>> {
	let callback_name = format!("actions.{name}");
	call_buffer(
		callback_name.as_str(),
		callback,
		ActionPayload {
			ctx: ctx.inner().clone(),
			conn,
			name,
			args,
			cancel_token,
		},
	)
	.await
}

async fn call_on_before_action_response(
	callback: &crate::actor_factory::CallbackTsfn<BeforeActionResponsePayload>,
	ctx: &ActorContext,
	name: String,
	args: Vec<u8>,
	output: Vec<u8>,
) -> Result<Vec<u8>> {
	call_buffer(
		"onBeforeActionResponse",
		callback,
		BeforeActionResponsePayload {
			ctx: ctx.inner().clone(),
			name,
			args,
			output,
		},
	)
	.await
}

async fn call_http_request(
	callback: &crate::actor_factory::CallbackTsfn<HttpRequestPayload>,
	ctx: &ActorContext,
	request: rivetkit_core::Request,
	cancel_token: Option<CancellationToken>,
) -> Result<rivetkit_core::Response> {
	call_request(
		"onRequest",
		callback,
		HttpRequestPayload {
			ctx: ctx.inner().clone(),
			request,
			cancel_token,
		},
	)
	.await
}

async fn with_dispatch_cancel_token<T, F, Fut>(work: F) -> Result<T>
where
	F: FnOnce(CancellationToken) -> Fut,
	Fut: std::future::Future<Output = Result<T>>,
{
	let guard = DispatchCancelGuard::new();
	work(guard.token()).await
}

async fn call_on_websocket(
	callback: &crate::actor_factory::CallbackTsfn<WebSocketPayload>,
	ctx: &ActorContext,
	ws: rivetkit_core::WebSocket,
	request: Option<rivetkit_core::Request>,
) -> Result<()> {
	call_void(
		"onWebSocket",
		callback,
		WebSocketPayload {
			ctx: ctx.inner().clone(),
			ws,
			request,
		},
	)
	.await
}

async fn call_on_before_subscribe(
	callback: &crate::actor_factory::CallbackTsfn<BeforeSubscribePayload>,
	ctx: &ActorContext,
	conn: rivetkit_core::ConnHandle,
	event_name: String,
) -> Result<()> {
	call_void(
		"onBeforeSubscribe",
		callback,
		BeforeSubscribePayload {
			ctx: ctx.inner().clone(),
			conn,
			event_name,
		},
	)
	.await
}

async fn call_workflow_history(
	callback: &crate::actor_factory::CallbackTsfn<WorkflowHistoryPayload>,
	ctx: &ActorContext,
) -> Result<Option<Vec<u8>>> {
	call_optional_buffer(
		"getWorkflowHistory",
		callback,
		WorkflowHistoryPayload {
			ctx: ctx.inner().clone(),
		},
	)
	.await
}

async fn call_workflow_replay(
	callback: &crate::actor_factory::CallbackTsfn<WorkflowReplayPayload>,
	ctx: &ActorContext,
	entry_id: Option<String>,
) -> Result<Option<Vec<u8>>> {
	call_optional_buffer(
		"replayWorkflow",
		callback,
		WorkflowReplayPayload {
			ctx: ctx.inner().clone(),
			entry_id,
		},
	)
	.await
}

async fn call_on_before_connect(
	callback: &crate::actor_factory::CallbackTsfn<BeforeConnectPayload>,
	ctx: &ActorContext,
	params: Vec<u8>,
	request: Option<rivetkit_core::Request>,
) -> Result<()> {
	call_void(
		"onBeforeConnect",
		callback,
		BeforeConnectPayload {
			ctx: ctx.inner().clone(),
			params,
			request,
		},
	)
	.await
}

async fn call_create_conn_state(
	callback: &crate::actor_factory::CallbackTsfn<CreateConnStatePayload>,
	ctx: &ActorContext,
	conn: rivetkit_core::ConnHandle,
	params: Vec<u8>,
	request: Option<rivetkit_core::Request>,
) -> Result<Vec<u8>> {
	call_buffer(
		"createConnState",
		callback,
		CreateConnStatePayload {
			ctx: ctx.inner().clone(),
			conn,
			params,
			request,
		},
	)
	.await
}

async fn call_on_connect(
	callback: &crate::actor_factory::CallbackTsfn<ConnectionPayload>,
	ctx: &ActorContext,
	conn: rivetkit_core::ConnHandle,
	request: Option<rivetkit_core::Request>,
) -> Result<()> {
	call_void(
		"onConnect",
		callback,
		ConnectionPayload {
			ctx: ctx.inner().clone(),
			conn,
			request,
		},
	)
	.await
}

async fn call_on_disconnect_final(
	callback: &crate::actor_factory::CallbackTsfn<ConnectionPayload>,
	ctx: &ActorContext,
	conn: rivetkit_core::ConnHandle,
) -> Result<()> {
	ctx.inner()
		.with_disconnect_callback(|| async {
			call_void(
				"onDisconnect",
				callback,
				ConnectionPayload {
					ctx: ctx.inner().clone(),
					conn,
					request: None,
				},
			)
			.await
		})
		.await
}

fn action_not_found(name: String) -> anyhow::Error {
	let schema = Box::leak(Box::new(RivetErrorSchema {
		group: "actor",
		code: "action_not_found",
		default_message: "Action not found",
		meta_type: None,
		_macro_marker: MacroMarker { _private: () },
	}));

	anyhow::Error::new(RivetTransportError {
		schema,
		meta: None,
		message: Some(format!("Action `{name}` was not found.")),
	})
}

fn actor_shutting_down() -> anyhow::Error {
	ActorLifecycle::Stopping.build()
}

fn missing_callback(name: &str) -> anyhow::Error {
	NapiInvalidState {
		state: format!("callback {name}"),
		reason: "not configured".to_owned(),
	}
	.build()
}

#[cfg(test)]
mod tests {
	use std::collections::HashMap;
	use std::sync::Arc as StdArc;
	use std::time::Duration;

	use rivet_error::RivetError as RivetTransportError;
	use rivetkit_core::Kv;
	use rivetkit_core::actor::state::{PERSIST_DATA_KEY, PersistedActor};
	use tokio::sync::oneshot;

	use super::*;

	fn test_adapter_config() -> AdapterConfig {
		let timeout = Duration::from_secs(1);
		AdapterConfig {
			create_state_timeout: timeout,
			on_create_timeout: timeout,
			create_vars_timeout: timeout,
			on_migrate_timeout: timeout,
			on_wake_timeout: timeout,
			on_before_actor_start_timeout: timeout,
			create_conn_state_timeout: timeout,
			on_before_connect_timeout: timeout,
			on_connect_timeout: timeout,
			action_timeout: timeout,
			on_request_timeout: timeout,
		}
	}

	fn empty_bindings() -> CallbackBindings {
		CallbackBindings {
			create_state: None,
			on_create: None,
			create_conn_state: None,
			create_vars: None,
			on_migrate: None,
			on_wake: None,
			on_before_actor_start: None,
			on_sleep: None,
			on_destroy: None,
			on_before_connect: None,
			on_connect: None,
			on_disconnect_final: None,
			on_before_subscribe: None,
			actions: HashMap::new(),
			on_before_action_response: None,
			on_queue_send: None,
			on_request: None,
			on_websocket: None,
			run: None,
			get_workflow_history: None,
			replay_workflow: None,
			serialize_state: None,
		}
	}

	fn assert_error_code(error: anyhow::Error, code: &str) {
		let error = RivetTransportError::extract(&error);
		assert_eq!(error.code(), code);
	}

	#[tokio::test(flavor = "current_thread")]
	async fn with_dispatch_cancel_token_cleans_up_after_success() {
		let cancel_token = with_dispatch_cancel_token(|cancel_token| async move {
			Ok::<_, anyhow::Error>(cancel_token)
		})
		.await
		.expect("successful dispatch should resolve");

		assert!(cancel_token.is_cancelled());
	}

	#[tokio::test(flavor = "current_thread")]
	async fn with_dispatch_cancel_token_cleans_up_after_panic() {
		let seen_cancel_token = StdArc::new(parking_lot::Mutex::new(None));

		let join_error = tokio::spawn({
			let seen_cancel_token = StdArc::clone(&seen_cancel_token);
			async move {
				let _ = with_dispatch_cancel_token(|cancel_token| async move {
					*seen_cancel_token.lock() = Some(cancel_token);
					panic!("dispatch panic");
					#[allow(unreachable_code)]
					Ok::<(), anyhow::Error>(())
				})
				.await;
			}
		})
		.await
		.expect_err("panic dispatch should panic");

		assert!(join_error.is_panic());
		let cancel_token = seen_cancel_token
			.lock()
			.clone()
			.expect("panic path should observe the dispatch token");
		assert!(cancel_token.is_cancelled());
	}

	#[tokio::test(flavor = "current_thread")]
	async fn with_dispatch_cancel_token_does_not_leak_under_mixed_load() {
		for i in 0..1000 {
			if i % 2 == 0 {
				let cancel_token = with_dispatch_cancel_token(|cancel_token| async move {
					Ok::<_, anyhow::Error>(cancel_token)
				})
				.await
				.expect("successful dispatch should resolve");
				assert!(cancel_token.is_cancelled());
				continue;
			}

			let join_error = tokio::spawn(async move {
				let _ = with_dispatch_cancel_token(|_| async move {
					panic!("dispatch panic");
					#[allow(unreachable_code)]
					Ok::<(), anyhow::Error>(())
				})
				.await;
			})
			.await
			.expect_err("panic dispatch should panic");

			assert!(join_error.is_panic());
		}
	}

	#[tokio::test]
	async fn action_dispatch_missing_action_returns_not_found() {
		let bindings = Arc::new(empty_bindings());
		let config = test_adapter_config();
		let core_ctx =
			rivetkit_core::ActorContext::new("actor-missing-action", "actor", Vec::new(), "local");
		let ctx = ActorContext::new(core_ctx);
		let abort = CancellationToken::new();
		let dirty = Arc::new(AtomicBool::new(false));
		let mut tasks = JoinSet::new();
		let (_registered_task_tx, mut registered_task_rx) = unbounded_channel();
		let (tx, rx) = oneshot::channel();

		dispatch_event(
			ActorEvent::Action {
				name: "missing".to_owned(),
				args: vec![1, 2, 3],
				conn: None,
				reply: tx.into(),
			},
			&bindings,
			&config,
			&ctx,
			&abort,
			&mut tasks,
			&mut registered_task_rx,
			&dirty,
		)
		.await;

		drain_tasks(&mut tasks, &mut registered_task_rx).await;

		let error = rx
			.await
			.expect("action reply should resolve")
			.expect_err("missing action should error");
		let error = RivetTransportError::extract(&error);
		assert_eq!(error.code(), "action_not_found");
	}

	#[tokio::test]
	async fn subscribe_request_without_guard_is_allowed() {
		let bindings = Arc::new(empty_bindings());
		let config = test_adapter_config();
		let core_ctx =
			rivetkit_core::ActorContext::new("actor-subscribe", "actor", Vec::new(), "local");
		let ctx = ActorContext::new(core_ctx);
		let abort = CancellationToken::new();
		let dirty = Arc::new(AtomicBool::new(false));
		let mut tasks = JoinSet::new();
		let (_registered_task_tx, mut registered_task_rx) = unbounded_channel();
		let (tx, rx) = oneshot::channel();
		let conn = rivetkit_core::ConnHandle::new("conn-subscribe", Vec::new(), Vec::new(), false);

		dispatch_event(
			ActorEvent::SubscribeRequest {
				conn,
				event_name: "ping".to_owned(),
				reply: tx.into(),
			},
			&bindings,
			&config,
			&ctx,
			&abort,
			&mut tasks,
			&mut registered_task_rx,
			&dirty,
		)
		.await;

		drain_tasks(&mut tasks, &mut registered_task_rx).await;

		rx.await
			.expect("subscribe reply should resolve")
			.expect("subscribe without guard should be allowed");
	}

	#[tokio::test]
	async fn connection_open_without_callbacks_is_allowed() {
		let bindings = Arc::new(empty_bindings());
		let config = test_adapter_config();
		let core_ctx =
			rivetkit_core::ActorContext::new("actor-connection-open", "actor", Vec::new(), "local");
		let ctx = ActorContext::new(core_ctx);
		let abort = CancellationToken::new();
		let dirty = Arc::new(AtomicBool::new(false));
		let mut tasks = JoinSet::new();
		let (_registered_task_tx, mut registered_task_rx) = unbounded_channel();
		let (tx, rx) = oneshot::channel();
		let conn = rivetkit_core::ConnHandle::new("conn-open", vec![1, 2, 3], Vec::new(), false);

		dispatch_event(
			ActorEvent::ConnectionOpen {
				conn: conn.clone(),
				params: vec![4, 5, 6],
				request: None,
				reply: tx.into(),
			},
			&bindings,
			&config,
			&ctx,
			&abort,
			&mut tasks,
			&mut registered_task_rx,
			&dirty,
		)
		.await;

		drain_tasks(&mut tasks, &mut registered_task_rx).await;

		rx.await
			.expect("connection-open reply should resolve")
			.expect("connection-open without callbacks should be allowed");
		assert!(conn.state().is_empty());
	}

	#[tokio::test]
	async fn workflow_requests_without_callbacks_return_none() {
		let bindings = Arc::new(empty_bindings());
		let config = test_adapter_config();
		let core_ctx =
			rivetkit_core::ActorContext::new("actor-workflow", "actor", Vec::new(), "local");
		let ctx = ActorContext::new(core_ctx);
		let abort = CancellationToken::new();
		let dirty = Arc::new(AtomicBool::new(false));
		let mut tasks = JoinSet::new();
		let (_registered_task_tx, mut registered_task_rx) = unbounded_channel();
		let (history_tx, history_rx) = oneshot::channel();
		let (replay_tx, replay_rx) = oneshot::channel();

		dispatch_event(
			ActorEvent::WorkflowHistoryRequested {
				reply: history_tx.into(),
			},
			&bindings,
			&config,
			&ctx,
			&abort,
			&mut tasks,
			&mut registered_task_rx,
			&dirty,
		)
		.await;
		dispatch_event(
			ActorEvent::WorkflowReplayRequested {
				entry_id: Some("step-1".to_owned()),
				reply: replay_tx.into(),
			},
			&bindings,
			&config,
			&ctx,
			&abort,
			&mut tasks,
			&mut registered_task_rx,
			&dirty,
		)
		.await;

		drain_tasks(&mut tasks, &mut registered_task_rx).await;

		assert_eq!(
			history_rx
				.await
				.expect("workflow history reply should resolve")
				.expect("workflow history should succeed"),
			None
		);
		assert_eq!(
			replay_rx
				.await
				.expect("workflow replay reply should resolve")
				.expect("workflow replay should succeed"),
			None
		);
	}

	#[tokio::test]
	async fn spawn_reply_sends_stopping_when_abort_is_cancelled() {
		let mut tasks = JoinSet::new();
		let (_registered_task_tx, mut registered_task_rx) = unbounded_channel();
		let abort = CancellationToken::new();
		let (tx, rx) = oneshot::channel();

		spawn_reply(&mut tasks, abort.clone(), tx.into(), async move {
			tokio::time::sleep(Duration::from_secs(60)).await;
			Ok::<_, anyhow::Error>(Vec::<u8>::new())
		});

		abort.cancel();
		drain_tasks(&mut tasks, &mut registered_task_rx).await;

		let error = rx
			.await
			.expect("abort reply should resolve")
			.expect_err("abort should return an error");
		let error = RivetTransportError::extract(&error);
		assert_eq!(error.code(), "stopping");
	}

	#[tokio::test]
	async fn callback_timeout_returns_structured_error_with_metadata() {
		let timeout = Duration::from_millis(10);
		let error = with_timeout("onWake", timeout, std::future::pending::<Result<()>>())
			.await
			.expect_err("callback timeout should fail");
		let error = RivetTransportError::extract(&error);
		assert_eq!(error.group(), "actor");
		assert_eq!(error.code(), "callback_timed_out");
		assert_eq!(
			error.message(),
			format!(
				"callback `onWake` timed out after {} ms",
				timeout.as_millis()
			)
		);
		assert_eq!(
			error.metadata(),
			Some(serde_json::json!({
				"callback_name": "onWake",
				"duration_ms": timeout.as_millis() as u64,
			}))
		);
	}

	#[tokio::test]
	async fn structured_timeout_returns_action_timeout_error() {
		let error = with_structured_timeout(
			"actor",
			"action_timed_out",
			"Action timed out",
			None,
			Duration::from_millis(10),
			std::future::pending::<Result<()>>(),
		)
		.await
		.expect_err("structured timeout should fail");
		let error = RivetTransportError::extract(&error);
		assert_eq!(error.group(), "actor");
		assert_eq!(error.code(), "action_timed_out");
		assert_eq!(error.message(), "Action timed out");
	}

	#[tokio::test]
	async fn run_adapter_loop_resets_stale_shared_end_reason_before_wake() {
		let bindings = Arc::new(empty_bindings());
		let config = Arc::new(test_adapter_config());
		let core_ctx =
			rivetkit_core::ActorContext::new("actor-wake-reset", "actor", Vec::new(), "local");
		let stale_ctx = ActorContext::new(core_ctx.clone());
		stale_ctx.mark_ready_internal();
		stale_ctx
			.mark_started_internal()
			.expect("stale context should mark started");
		stale_ctx.set_end_reason(EndReason::Sleep);

		let (events_tx, events_rx) = unbounded_channel();
		let (first_tx, first_rx) = oneshot::channel();
		let (second_tx, second_rx) = oneshot::channel();

		events_tx
			.send(ActorEvent::Action {
				name: "missing-first".to_owned(),
				args: Vec::new(),
				conn: None,
				reply: first_tx.into(),
			})
			.expect("first action event should send");
		events_tx
			.send(ActorEvent::Action {
				name: "missing-second".to_owned(),
				args: Vec::new(),
				conn: None,
				reply: second_tx.into(),
			})
			.expect("second action event should send");
		drop(events_tx);

		run_adapter_loop(
			bindings,
			config,
			ActorStart {
				ctx: core_ctx,
				input: None,
				snapshot: Some(Vec::new()),
				hibernated: Vec::new(),
				events: ActorEvents::from(events_rx),
				startup_ready: None,
			},
		)
		.await
		.expect("adapter loop should finish cleanly");

		let first_error = first_rx
			.await
			.expect("first action reply should resolve")
			.expect_err("missing action should error");
		assert_error_code(first_error, "action_not_found");

		let second_error = second_rx
			.await
			.expect("second action reply should resolve")
			.expect_err("second missing action should error");
		assert_error_code(second_error, "action_not_found");
		assert_eq!(stale_ctx.take_end_reason(), None);
	}

	#[tokio::test]
	async fn preamble_marks_initialized_and_reloads_as_wake() {
		let kv = Kv::new_in_memory();
		let config = test_adapter_config();
		let bindings = empty_bindings();

		let first_core_ctx = rivetkit_core::ActorContext::new_with_kv(
			"actor-preamble-first",
			"actor",
			Vec::new(),
			"local",
			kv.clone(),
		);
		let first_ctx = ActorContext::new(first_core_ctx.clone());
		first_ctx
			.set_state_initial(vec![9, 9, 9])
			.expect("initial state should set");

		run_preamble(&bindings, &config, &first_ctx, None, None, Vec::new())
			.await
			.expect("first-create preamble should succeed");

		let persisted_bytes = kv
			.get(PERSIST_DATA_KEY)
			.await
			.expect("persisted actor read should succeed")
			.expect("persisted actor bytes should exist");
		let embedded_version = u16::from_le_bytes([persisted_bytes[0], persisted_bytes[1]]);
		assert!(matches!(embedded_version, 3 | 4));
		let persisted: PersistedActor =
			serde_bare::from_slice(&persisted_bytes[2..]).expect("persisted actor should decode");
		assert!(persisted.has_initialized);

		let second_core_ctx = rivetkit_core::ActorContext::new_with_kv(
			"actor-preamble-second",
			"actor",
			Vec::new(),
			"local",
			kv.clone(),
		);
		let second_ctx = ActorContext::new(second_core_ctx);
		let snapshot = persisted.has_initialized.then_some(persisted.state.clone());
		assert!(snapshot.is_some());

		run_preamble(&bindings, &config, &second_ctx, None, snapshot, Vec::new())
			.await
			.expect("wake preamble should succeed");

		assert_eq!(second_ctx.inner().state(), vec![9, 9, 9]);
	}

	#[tokio::test]
	async fn maybe_serialize_skips_save_when_adapter_is_clean() {
		let bindings = empty_bindings();
		let core_ctx =
			rivetkit_core::ActorContext::new("actor-serialize-clean", "actor", Vec::new(), "local");
		let ctx = ActorContext::new(core_ctx);
		let dirty = AtomicBool::new(false);

		let deltas = maybe_serialize(&bindings, &ctx, &dirty, SerializeStateReason::Save)
			.await
			.expect("clean save serialize should not fail");

		assert!(deltas.is_empty());
		assert!(!dirty.load(Ordering::Acquire));
	}

	#[tokio::test]
	async fn maybe_serialize_inspector_does_not_consume_pending_save() {
		let bindings = empty_bindings();
		let core_ctx = rivetkit_core::ActorContext::new(
			"actor-serialize-inspector",
			"actor",
			Vec::new(),
			"local",
		);
		let ctx = ActorContext::new(core_ctx);
		let dirty = AtomicBool::new(true);
		let calls = Arc::new(Mutex::new(Vec::new()));

		let inspector_deltas = maybe_serialize_with(
			&bindings,
			&ctx,
			&dirty,
			SerializeStateReason::Inspector,
			|_, _, reason| {
				let calls = Arc::clone(&calls);
				async move {
					calls.lock().push(reason);
					Ok(vec![StateDelta::ActorState(vec![1, 2, 3])])
				}
			},
		)
		.await
		.expect("inspector serialize should succeed");

		assert_eq!(inspector_deltas.len(), 1);
		assert!(dirty.load(Ordering::Acquire));

		let save_deltas = maybe_serialize_with(
			&bindings,
			&ctx,
			&dirty,
			SerializeStateReason::Save,
			|_, _, reason| {
				let calls = Arc::clone(&calls);
				async move {
					calls.lock().push(reason);
					Ok(vec![StateDelta::ActorState(vec![4, 5, 6])])
				}
			},
		)
		.await
		.expect("save serialize should still run after inspector");

		assert_eq!(save_deltas.len(), 1);
		assert!(!dirty.load(Ordering::Acquire));
		assert_eq!(*calls.lock(), vec!["inspector", "save"]);
	}
}
