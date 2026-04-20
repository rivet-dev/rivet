use std::future::{self, Future};
use std::panic::AssertUnwindSafe;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
#[cfg(test)]
use std::sync::{Mutex, OnceLock};

use anyhow::{Context, Result, anyhow};
use futures::FutureExt;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::{JoinError, JoinHandle};
use tokio::time::{Duration, Instant, sleep, sleep_until, timeout};

use crate::actor::action::ActionDispatchError;
use crate::actor::callbacks::{
	ActorEvent, ActorStart, Reply, Request, Response, SerializeStateReason,
};
use crate::actor::connection::ConnHandle;
use crate::actor::context::ActorContext;
use crate::actor::diagnostics::record_actor_warning;
use crate::actor::factory::ActorFactory;
use crate::actor::metrics::ActorMetrics;
use crate::actor::state::{PERSIST_DATA_KEY, PersistedActor, decode_persisted_actor};
use crate::actor::task_types::{StateMutationReason, StopReason};
use crate::error::ActorLifecycle as ActorLifecycleError;
use crate::types::SaveStateOpts;
use crate::websocket::WebSocket;

pub type ActionDispatchResult = std::result::Result<Vec<u8>, ActionDispatchError>;
pub type HttpDispatchResult = Result<Response>;

const LONG_SHUTDOWN_DRAIN_WARNING_THRESHOLD: Duration = Duration::from_secs(1);
const INSPECTOR_SERIALIZE_STATE_INTERVAL: Duration = Duration::from_millis(50);
const INSPECTOR_OVERLAY_CHANNEL_CAPACITY: usize = 32;

pub(crate) const LIFECYCLE_INBOX_CHANNEL: &str = "lifecycle_inbox";
pub(crate) const DISPATCH_INBOX_CHANNEL: &str = "dispatch_inbox";
pub(crate) const LIFECYCLE_EVENT_INBOX_CHANNEL: &str = "lifecycle_event_inbox";
pub(crate) const ACTOR_EVENT_INBOX_CHANNEL: &str = "actor_event_inbox";
pub use crate::actor::task_types::LifecycleState;

#[cfg(test)]
#[path = "../../tests/modules/task.rs"]
mod tests;

#[cfg(test)]
type ShutdownCleanupHook = Arc<dyn Fn(&ActorContext, &'static str) + Send + Sync>;

#[cfg(test)]
static SHUTDOWN_CLEANUP_HOOK: OnceLock<Mutex<Option<ShutdownCleanupHook>>> =
	OnceLock::new();

#[cfg(test)]
pub(crate) struct ShutdownCleanupHookGuard;

#[cfg(test)]
type LifecycleEventHook = Arc<dyn Fn(&ActorContext, &LifecycleEvent) + Send + Sync>;

#[cfg(test)]
static LIFECYCLE_EVENT_HOOK: OnceLock<Mutex<Option<LifecycleEventHook>>> =
	OnceLock::new();

#[cfg(test)]
pub(crate) struct LifecycleEventHookGuard;

#[cfg(test)]
type ShutdownReplyHook = Arc<dyn Fn(&ActorContext, StopReason) + Send + Sync>;

#[cfg(test)]
static SHUTDOWN_REPLY_HOOK: OnceLock<Mutex<Option<ShutdownReplyHook>>> =
	OnceLock::new();

#[cfg(test)]
pub(crate) struct ShutdownReplyHookGuard;

#[cfg(test)]
pub(crate) fn install_shutdown_cleanup_hook(
	hook: ShutdownCleanupHook,
) -> ShutdownCleanupHookGuard {
	*SHUTDOWN_CLEANUP_HOOK
		.get_or_init(|| Mutex::new(None))
		.lock()
		.expect("shutdown cleanup hook lock poisoned") = Some(hook);
	ShutdownCleanupHookGuard
}

#[cfg(test)]
impl Drop for ShutdownCleanupHookGuard {
	fn drop(&mut self) {
		if let Some(hooks) = SHUTDOWN_CLEANUP_HOOK.get() {
			*hooks
				.lock()
				.expect("shutdown cleanup hook lock poisoned") = None;
		}
	}
}

#[cfg(test)]
fn run_shutdown_cleanup_hook(ctx: &ActorContext, reason: &'static str) {
	let hook = SHUTDOWN_CLEANUP_HOOK
		.get_or_init(|| Mutex::new(None))
		.lock()
		.expect("shutdown cleanup hook lock poisoned")
		.clone();
	if let Some(hook) = hook {
		hook(ctx, reason);
	}
}

#[cfg(test)]
pub(crate) fn install_lifecycle_event_hook(
	hook: LifecycleEventHook,
) -> LifecycleEventHookGuard {
	*LIFECYCLE_EVENT_HOOK
		.get_or_init(|| Mutex::new(None))
		.lock()
		.expect("lifecycle event hook lock poisoned") = Some(hook);
	LifecycleEventHookGuard
}

#[cfg(test)]
impl Drop for LifecycleEventHookGuard {
	fn drop(&mut self) {
		if let Some(hooks) = LIFECYCLE_EVENT_HOOK.get() {
			*hooks
				.lock()
				.expect("lifecycle event hook lock poisoned") = None;
		}
	}
}

#[cfg(test)]
fn run_lifecycle_event_hook(ctx: &ActorContext, event: &LifecycleEvent) {
	let hook = LIFECYCLE_EVENT_HOOK
		.get_or_init(|| Mutex::new(None))
		.lock()
		.expect("lifecycle event hook lock poisoned")
		.clone();
	if let Some(hook) = hook {
		hook(ctx, event);
	}
}

#[cfg(test)]
pub(crate) fn install_shutdown_reply_hook(
	hook: ShutdownReplyHook,
) -> ShutdownReplyHookGuard {
	*SHUTDOWN_REPLY_HOOK
		.get_or_init(|| Mutex::new(None))
		.lock()
		.expect("shutdown reply hook lock poisoned") = Some(hook);
	ShutdownReplyHookGuard
}

#[cfg(test)]
impl Drop for ShutdownReplyHookGuard {
	fn drop(&mut self) {
		if let Some(hooks) = SHUTDOWN_REPLY_HOOK.get() {
			*hooks
				.lock()
				.expect("shutdown reply hook lock poisoned") = None;
		}
	}
}

#[cfg(test)]
fn run_shutdown_reply_hook(ctx: &ActorContext, reason: StopReason) {
	let hook = SHUTDOWN_REPLY_HOOK
		.get_or_init(|| Mutex::new(None))
		.lock()
		.expect("shutdown reply hook lock poisoned")
		.clone();
	if let Some(hook) = hook {
		hook(ctx, reason);
	}
}

pub enum LifecycleCommand {
	Start {
		reply: oneshot::Sender<Result<()>>,
	},
	Stop {
		reason: StopReason,
		reply: oneshot::Sender<Result<()>>,
	},
	FireAlarm {
		reply: oneshot::Sender<Result<()>>,
	},
}

pub(crate) fn actor_channel_overloaded_error(
	channel: &'static str,
	capacity: usize,
	operation: &'static str,
	metrics: Option<&ActorMetrics>,
) -> anyhow::Error {
	if let Some(metrics) = metrics {
		match channel {
			LIFECYCLE_INBOX_CHANNEL => metrics.inc_lifecycle_inbox_overload(operation),
			DISPATCH_INBOX_CHANNEL => metrics.inc_dispatch_inbox_overload(operation),
			LIFECYCLE_EVENT_INBOX_CHANNEL => {
				metrics.inc_lifecycle_event_overload(operation)
			}
			_ => {}
		}
	}
	if let Some(metrics) = metrics {
		if let Some(suppression) =
			record_actor_warning(metrics.actor_id(), "actor_channel_overloaded")
		{
			tracing::warn!(
				actor_id = %suppression.actor_id,
				channel,
				capacity,
				operation,
				event = if channel == LIFECYCLE_EVENT_INBOX_CHANNEL {
					operation
				} else {
					""
				},
				per_actor_suppressed = suppression.per_actor_suppressed,
				global_suppressed = suppression.global_suppressed,
				"actor bounded channel overloaded"
			);
		}
	} else {
		tracing::warn!(
			channel,
			capacity,
			operation,
			"actor bounded channel overloaded"
		);
	}
	ActorLifecycleError::Overloaded {
		channel: channel.to_owned(),
		capacity,
		operation: operation.to_owned(),
	}
	.build()
}

pub(crate) fn try_send_lifecycle_command(
	sender: &mpsc::Sender<LifecycleCommand>,
	capacity: usize,
	operation: &'static str,
	command: LifecycleCommand,
	metrics: Option<&ActorMetrics>,
) -> Result<()> {
	let permit = sender.try_reserve().map_err(|_| {
		actor_channel_overloaded_error(
			LIFECYCLE_INBOX_CHANNEL,
			capacity,
			operation,
			metrics,
		)
	})?;
	permit.send(command);
	Ok(())
}

pub enum DispatchCommand {
	Action {
		name: String,
		args: Vec<u8>,
		conn: ConnHandle,
		reply: oneshot::Sender<Result<Vec<u8>>>,
	},
	Http {
		request: Request,
		reply: oneshot::Sender<HttpDispatchResult>,
	},
	OpenWebSocket {
		ws: WebSocket,
		request: Option<Request>,
		reply: oneshot::Sender<Result<()>>,
	},
	WorkflowHistory {
		reply: oneshot::Sender<Result<Option<Vec<u8>>>>,
	},
	WorkflowReplay {
		entry_id: Option<String>,
		reply: oneshot::Sender<Result<Option<Vec<u8>>>>,
	},
}

pub(crate) fn try_send_dispatch_command(
	sender: &mpsc::Sender<DispatchCommand>,
	capacity: usize,
	operation: &'static str,
	command: DispatchCommand,
	metrics: Option<&ActorMetrics>,
) -> Result<()> {
	let permit = sender.try_reserve().map_err(|_| {
		actor_channel_overloaded_error(
			DISPATCH_INBOX_CHANNEL,
			capacity,
			operation,
			metrics,
		)
	})?;
	permit.send(command);
	Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleEvent {
	StateMutated {
		reason: StateMutationReason,
	},
	ActivityDirty,
	SaveRequested {
		immediate: bool,
	},
	InspectorSerializeRequested,
	InspectorAttachmentsChanged,
	SleepTick,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShutdownPhase {
	SendingFinalize,
	AwaitingFinalizeReply,
	DrainingBefore,
	DisconnectingConns,
	DrainingAfter,
	AwaitingRunHandle,
	Finalizing,
	Done,
}

type ShutdownStep = Pin<Box<dyn Future<Output = Result<ShutdownPhase>> + Send>>;

pub struct ActorTask {
	pub actor_id: String,
	pub generation: u32,
	pub lifecycle_inbox: mpsc::Receiver<LifecycleCommand>,
	pub dispatch_inbox: mpsc::Receiver<DispatchCommand>,
	pub lifecycle_events: mpsc::Receiver<LifecycleEvent>,
	pub lifecycle: LifecycleState,
	pub factory: Arc<ActorFactory>,
	pub ctx: ActorContext,
	pub start_input: Option<Vec<u8>>,
	pub preload_persisted_actor: Option<PersistedActor>,
	actor_event_tx: Option<mpsc::Sender<ActorEvent>>,
	actor_event_rx: Option<mpsc::Receiver<ActorEvent>>,
	run_handle: Option<JoinHandle<Result<()>>>,
	inspector_attach_count: Arc<AtomicU32>,
	inspector_overlay_tx: broadcast::Sender<Arc<Vec<u8>>>,
	pub state_save_deadline: Option<Instant>,
	pub inspector_serialize_state_deadline: Option<Instant>,
	pub sleep_deadline: Option<Instant>,
	shutdown_phase: Option<ShutdownPhase>,
	shutdown_reason: Option<StopReason>,
	shutdown_deadline: Option<Instant>,
	shutdown_started_at: Option<Instant>,
	shutdown_replies: Vec<oneshot::Sender<Result<()>>>,
	shutdown_step: Option<ShutdownStep>,
	shutdown_finalize_reply: Option<oneshot::Receiver<Result<()>>>,
}

impl ActorTask {
	pub fn new(
		actor_id: String,
		generation: u32,
		lifecycle_inbox: mpsc::Receiver<LifecycleCommand>,
		dispatch_inbox: mpsc::Receiver<DispatchCommand>,
		lifecycle_events: mpsc::Receiver<LifecycleEvent>,
		factory: Arc<ActorFactory>,
		ctx: ActorContext,
		start_input: Option<Vec<u8>>,
		preload_persisted_actor: Option<PersistedActor>,
	) -> Self {
		let (actor_event_tx, actor_event_rx) =
			mpsc::channel(factory.config().lifecycle_event_inbox_capacity);
		let (inspector_overlay_tx, _) =
			broadcast::channel(INSPECTOR_OVERLAY_CHANNEL_CAPACITY);
		let inspector_attach_count = Arc::new(AtomicU32::new(0));
		ctx.configure_inspector_runtime(
			Arc::clone(&inspector_attach_count),
			inspector_overlay_tx.clone(),
		);
		let inspector_ctx = ctx.clone();
		let inspector_attach_count_for_hook = Arc::clone(&inspector_attach_count);
		ctx.on_request_save(Box::new(move |_immediate| {
			if inspector_attach_count_for_hook.load(Ordering::SeqCst) > 0 {
				inspector_ctx.notify_inspector_serialize_requested();
			}
		}));
		Self {
			actor_id,
			generation,
			lifecycle_inbox,
			dispatch_inbox,
			lifecycle_events,
			lifecycle: LifecycleState::default(),
			factory,
			ctx,
			start_input,
			preload_persisted_actor,
			actor_event_tx: Some(actor_event_tx),
			actor_event_rx: Some(actor_event_rx),
			run_handle: None,
			inspector_attach_count,
			inspector_overlay_tx,
			state_save_deadline: None,
			inspector_serialize_state_deadline: None,
			sleep_deadline: None,
			shutdown_phase: None,
			shutdown_reason: None,
			shutdown_deadline: None,
			shutdown_started_at: None,
			shutdown_replies: Vec::new(),
			shutdown_step: None,
			shutdown_finalize_reply: None,
		}
	}

	pub async fn run(mut self) -> Result<()> {
		loop {
			self.record_inbox_depths();
			tokio::select! {
				biased;
				// Bind the raw Option so a closed channel is logged, not silently swallowed by tokio::select!'s else arm.
				lifecycle_command = self.lifecycle_inbox.recv() => {
					match lifecycle_command {
						Some(command) => self.handle_lifecycle(command).await,
						None => {
							self.log_closed_channel(
								"lifecycle_inbox",
								"actor task terminating because lifecycle command inbox closed",
							);
							break;
						}
					}
				}
				lifecycle_event = self.lifecycle_events.recv() => {
					match lifecycle_event {
						Some(event) => self.handle_event(event).await,
						None => {
							self.log_closed_channel(
								"lifecycle_events",
								"actor task terminating because lifecycle event inbox closed",
							);
							break;
						}
					}
				}
				shutdown_outcome = Self::poll_shutdown_step(self.shutdown_step.as_mut()), if self.shutdown_step.is_some() => {
					self.on_shutdown_step_complete(shutdown_outcome);
				}
				dispatch_command = self.dispatch_inbox.recv(), if self.accepting_dispatch() => {
					match dispatch_command {
						Some(command) => self.handle_dispatch(command).await,
						None => {
							self.log_closed_channel(
								"dispatch_inbox",
								"actor task terminating because dispatch inbox closed",
							);
							break;
						}
					}
				}
				outcome = Self::wait_for_run_handle(self.run_handle.as_mut()), if self.run_handle.is_some() && self.shutdown_step.is_none() => {
					self.handle_run_handle_outcome(outcome);
				}
				_ = Self::state_save_tick(self.state_save_deadline), if self.state_save_timer_active() => {
					self.on_state_save_tick().await;
				}
				_ = Self::inspector_serialize_state_tick(self.inspector_serialize_state_deadline), if self.inspector_serialize_timer_active() => {
					self.on_inspector_serialize_state_tick().await;
				}
				_ = Self::sleep_tick(self.sleep_deadline), if self.sleep_timer_active() => {
					self.on_sleep_tick().await;
				}
			}

			if self.should_terminate() {
				break;
			}
		}

		self.record_inbox_depths();
		Ok(())
	}

	async fn handle_lifecycle(&mut self, command: LifecycleCommand) {
		match command {
			LifecycleCommand::Start { reply } => {
				let result = self.start_actor().await;
				let _ = reply.send(result);
			}
			LifecycleCommand::Stop { reason, reply } => {
				self.begin_stop(reason, reply).await;
			}
			LifecycleCommand::FireAlarm { reply } => {
				let result = self.fire_due_alarms().await;
				let _ = reply.send(result);
			}
		}
	}

	#[cfg_attr(not(test), allow(dead_code))]
	async fn handle_stop(&mut self, reason: StopReason) -> Result<()> {
		let (reply_tx, reply_rx) = oneshot::channel();
		self.begin_stop(reason, reply_tx).await;
		self.drive_shutdown_to_completion().await;
		reply_rx
			.await
			.expect("direct stop reply channel should remain open")
	}

	async fn begin_stop(
		&mut self,
		reason: StopReason,
		reply: oneshot::Sender<Result<()>>,
	) {
		match self.lifecycle {
			LifecycleState::Started => {
				self.register_shutdown_reply(reply);
				self.drain_accepted_dispatch().await;
				match reason {
					StopReason::Sleep => {
						self.transition_to(LifecycleState::SleepGrace);
						self.shutdown_for_sleep_grace().await;
					}
					StopReason::Destroy => {
						self.enter_shutdown_state_machine(StopReason::Destroy);
					}
				}
			}
			LifecycleState::SleepGrace => {
				let _ = reply.send(Ok(()));
			}
			LifecycleState::SleepFinalize | LifecycleState::Destroying => {
				self.register_shutdown_reply(reply);
			}
			LifecycleState::Terminated => {
				let _ = reply.send(Ok(()));
			}
			LifecycleState::Loading
			| LifecycleState::Migrating
			| LifecycleState::Waking
			| LifecycleState::Ready => {
				let _ = reply.send(Err(ActorLifecycleError::NotReady.build()));
			}
		}
	}

	async fn drain_accepted_dispatch(&mut self) {
		while self.lifecycle == LifecycleState::Started {
			let Ok(command) = self.dispatch_inbox.try_recv() else {
				break;
			};
			self.handle_dispatch(command).await;
		}
	}

	async fn handle_event(&mut self, event: LifecycleEvent) {
		#[cfg(test)]
		run_lifecycle_event_hook(&self.ctx, &event);
		match event {
			LifecycleEvent::StateMutated { .. } => {
				self.ctx.record_state_updated();
			}
			LifecycleEvent::ActivityDirty => {
				self.ctx.acknowledge_activity_dirty();
				self.reset_sleep_deadline().await;
			}
			LifecycleEvent::SaveRequested { immediate } => {
				self.schedule_state_save(immediate);
				self.sync_inspector_serialize_deadline();
			}
			LifecycleEvent::InspectorSerializeRequested
			| LifecycleEvent::InspectorAttachmentsChanged => {
				self.sync_inspector_serialize_deadline();
			}
			LifecycleEvent::SleepTick => {
				self.on_sleep_tick().await;
			}
		}
	}

	async fn handle_dispatch(&mut self, command: DispatchCommand) {
		if let Some(error) = self.dispatch_lifecycle_error() {
			self.reply_dispatch_error(command, error);
			return;
		}

		match command {
			DispatchCommand::Action {
				name,
				args,
				conn,
				reply,
			} => match self.reserve_actor_event("dispatch_action") {
				Ok(permit) => {
					permit.send(ActorEvent::Action {
						name,
						args,
						conn: Some(conn),
						reply: Reply::from(reply),
					});
				}
				Err(error) => {
					let _ = reply.send(Err(error));
				}
			},
			DispatchCommand::Http { request, reply } => {
				match self.reserve_actor_event("dispatch_http") {
					Ok(permit) => {
						permit.send(ActorEvent::HttpRequest {
							request,
							reply: Reply::from(reply),
						});
					}
					Err(error) => {
						let _ = reply.send(Err(error));
					}
				}
			}
			DispatchCommand::OpenWebSocket { ws, request, reply } => {
				match self.reserve_actor_event("dispatch_websocket_open") {
					Ok(permit) => {
						permit.send(ActorEvent::WebSocketOpen {
							ws,
							request,
							reply: Reply::from(reply),
						});
					}
					Err(error) => {
						let _ = reply.send(Err(error));
					}
				}
			}
			DispatchCommand::WorkflowHistory { reply } => {
				match self.reserve_actor_event("dispatch_workflow_history") {
					Ok(permit) => {
						permit.send(ActorEvent::WorkflowHistoryRequested {
							reply: Reply::from(reply),
						});
					}
					Err(error) => {
						let _ = reply.send(Err(error));
					}
				}
			}
			DispatchCommand::WorkflowReplay { entry_id, reply } => {
				match self.reserve_actor_event("dispatch_workflow_replay") {
					Ok(permit) => {
						permit.send(ActorEvent::WorkflowReplayRequested {
							entry_id,
							reply: Reply::from(reply),
						});
					}
					Err(error) => {
						let _ = reply.send(Err(error));
					}
				}
			}
		}
	}

	fn reserve_actor_event(
		&self,
		operation: &'static str,
	) -> Result<mpsc::OwnedPermit<ActorEvent>> {
		let sender = self
			.actor_event_tx
			.clone()
			.ok_or_else(|| ActorLifecycleError::NotReady.build())?;
		sender.try_reserve_owned().map_err(|_| {
			actor_channel_overloaded_error(
				ACTOR_EVENT_INBOX_CHANNEL,
				self.factory.config().lifecycle_event_inbox_capacity,
				operation,
				Some(self.ctx.metrics()),
			)
		})
	}

	fn reply_dispatch_error(
		&self,
		command: DispatchCommand,
		error: anyhow::Error,
	) {
		match command {
			DispatchCommand::Action { reply, .. } => {
				let _ = reply.send(Err(error));
			}
			DispatchCommand::Http { reply, .. } => {
				let _ = reply.send(Err(error));
			}
			DispatchCommand::OpenWebSocket { reply, .. } => {
				let _ = reply.send(Err(error));
			}
			DispatchCommand::WorkflowHistory { reply } => {
				let _ = reply.send(Err(error));
			}
			DispatchCommand::WorkflowReplay { reply, .. } => {
				let _ = reply.send(Err(error));
			}
		}
	}

	fn dispatch_lifecycle_error(&self) -> Option<anyhow::Error> {
		match self.lifecycle {
			LifecycleState::Started | LifecycleState::SleepGrace => None,
			LifecycleState::SleepFinalize => {
				self.ctx.warn_work_sent_to_stopping_instance("dispatch");
				Some(ActorLifecycleError::Stopping.build())
			}
			LifecycleState::Destroying | LifecycleState::Terminated => {
				self.ctx.warn_work_sent_to_stopping_instance("dispatch");
				Some(ActorLifecycleError::Destroying.build())
			}
			LifecycleState::Loading
			| LifecycleState::Migrating
			| LifecycleState::Waking
			| LifecycleState::Ready => {
				self.ctx.warn_self_call_risk("dispatch");
				Some(ActorLifecycleError::NotReady.build())
			}
		}
	}

	async fn start_actor(&mut self) -> Result<()> {
		if !self.ctx.started() {
			self.ctx.configure_sleep(self.factory.config().clone());
			self
				.ctx
				.configure_connection_runtime(self.factory.config().clone());
		}
		self.ensure_actor_event_channel();
		self
			.ctx
			.configure_actor_events(self.actor_event_tx.clone());

		let persisted = self.load_persisted_actor().await?;
		let is_new = !persisted.has_initialized;
		self.ctx.load_persisted_actor(persisted);
		self.ctx.set_has_initialized(true);
		self
			.ctx
			.persist_state(SaveStateOpts { immediate: true })
			.await
			.context("persist actor initialization")?;
		self
			.ctx
			.restore_hibernatable_connections()
			.await
			.context("restore hibernatable connections")?;
		Self::settle_hibernated_connections(self.ctx.clone())
			.await
			.context("settle hibernated connections")?;
		self.ctx.init_alarms();

		self.transition_to(LifecycleState::Started);
		self.spawn_run_handle(is_new);
		self.reset_sleep_deadline().await;
		self.ctx.drain_overdue_scheduled_events().await?;
		Ok(())
	}

	async fn load_persisted_actor(&mut self) -> Result<PersistedActor> {
		if let Some(preloaded) = self.preload_persisted_actor.take() {
			return Ok(preloaded);
		}

		match self.ctx.kv().get(PERSIST_DATA_KEY).await? {
			Some(bytes) => {
				decode_persisted_actor(&bytes).context("decode persisted actor startup data")
			}
			None => Ok(PersistedActor {
				input: self.start_input.clone(),
				..PersistedActor::default()
			}),
		}
	}

	fn ensure_actor_event_channel(&mut self) {
		if self.actor_event_tx.is_some() && self.actor_event_rx.is_some() {
			return;
		}

		let (actor_event_tx, actor_event_rx) =
			mpsc::channel(self.factory.config().lifecycle_event_inbox_capacity);
		self.actor_event_tx = Some(actor_event_tx);
		self.actor_event_rx = Some(actor_event_rx);
	}

	fn spawn_run_handle(&mut self, is_new: bool) {
		if self.run_handle.is_some() {
			return;
		}

		let Some(actor_events) = self.actor_event_rx.take() else {
			return;
		};
		let start = ActorStart {
			ctx: self.ctx.clone(),
			input: self.ctx.persisted_actor().input.clone(),
			snapshot: (!is_new).then(|| self.ctx.state()),
			hibernated: self
				.ctx
				.conns()
				.filter(|conn| conn.is_hibernatable())
				.map(|conn| {
					let bytes = conn.state();
					(conn, bytes)
				})
				.collect(),
			events: actor_events.into(),
		};
		let factory = self.factory.clone();
		self.run_handle = Some(tokio::spawn(async move {
			match AssertUnwindSafe(factory.start(start)).catch_unwind().await {
				Ok(result) => result,
				Err(_) => Err(anyhow!("actor run handler panicked")),
			}
		}));
	}

	async fn settle_hibernated_connections(ctx: ActorContext) -> Result<()> {
		let mut dead_conn_ids = Vec::new();
		for conn in ctx.conns().filter(|conn| conn.is_hibernatable()) {
			let hibernation = conn.hibernation();
			let Some(hibernation) = hibernation else {
				dead_conn_ids.push(conn.id().to_owned());
				continue;
			};
			let is_live = ctx.hibernated_connection_is_live(
				&hibernation.gateway_id,
				&hibernation.request_id,
			)?;
			if is_live {
				continue;
			}
			dead_conn_ids.push(conn.id().to_owned());
		}

		for conn_id in dead_conn_ids {
			ctx.request_hibernation_transport_removal(conn_id.clone());
			ctx.remove_conn(&conn_id);
		}

		Ok(())
	}

	async fn fire_due_alarms(&mut self) -> Result<()> {
		if !matches!(self.lifecycle, LifecycleState::Started) {
			return Ok(());
		}

		self.ctx.drain_overdue_scheduled_events().await
	}

	fn handle_run_handle_outcome(
		&mut self,
		outcome: std::result::Result<Result<()>, JoinError>,
	) {
		self.run_handle = None;
		self.state_save_deadline = None;
		self.inspector_serialize_state_deadline = None;
		self.close_actor_event_channel();

		match outcome {
			Ok(Ok(())) => {}
			Ok(Err(error)) => {
				tracing::error!(?error, "actor run handler failed");
			}
			Err(error) => {
				tracing::error!(?error, "actor run handler join failed");
			}
		}

		if self.ctx.destroy_requested() {
			self.transition_to(LifecycleState::Destroying);
			return;
		}

		if self.ctx.sleep_requested() {
			self.transition_to(LifecycleState::SleepFinalize);
			return;
		}

		if self.lifecycle == LifecycleState::Started {
			self.transition_to(LifecycleState::Terminated);
		}
	}

	async fn wait_for_run_handle(
		run_handle: Option<&mut JoinHandle<Result<()>>>,
	) -> std::result::Result<Result<()>, JoinError> {
		let Some(run_handle) = run_handle else {
			future::pending::<()>().await;
			unreachable!();
		};
		run_handle.await
	}

	fn close_actor_event_channel(&mut self) {
		self.actor_event_tx = None;
		self.ctx.configure_actor_events(None);
	}

	async fn shutdown_for_sleep_grace(&mut self) {
		let config = self.factory.config().clone();
		let shutdown_deadline = Instant::now() + config.effective_sleep_grace_period();
		self.sleep_deadline = None;
		self.ctx.cancel_sleep_timer();
		self.request_begin_sleep();

		let idle_wait_ctx = self.ctx.clone();
		let idle_wait = async move {
			idle_wait_ctx
				.wait_for_sleep_idle_window(shutdown_deadline)
				.await
		};
		tokio::pin!(idle_wait);
		loop {
			tokio::select! {
				biased;
				lifecycle_command = self.lifecycle_inbox.recv() => {
					match lifecycle_command {
						Some(LifecycleCommand::Start { reply }) => {
							let _ = reply.send(Err(ActorLifecycleError::Stopping.build()));
						}
						Some(LifecycleCommand::Stop { reason: StopReason::Sleep, reply }) => {
							let _ = reply.send(Ok(()));
						}
						Some(LifecycleCommand::Stop { reason: StopReason::Destroy, reply }) => {
							self.register_shutdown_reply(reply);
							self.enter_shutdown_state_machine(StopReason::Destroy);
							return;
						}
						Some(LifecycleCommand::FireAlarm { reply }) => {
							let result = self.fire_due_alarms().await;
							let _ = reply.send(result);
						}
						None => {
							self.log_closed_channel(
								"lifecycle_inbox",
								"actor task terminating because lifecycle command inbox closed",
							);
						}
					}
				}
				lifecycle_event = self.lifecycle_events.recv() => {
					match lifecycle_event {
						Some(event) => self.handle_event(event).await,
						None => {
							self.log_closed_channel(
								"lifecycle_events",
								"actor task terminating because lifecycle event inbox closed",
							);
						}
					}
				}
				dispatch_command = self.dispatch_inbox.recv() => {
					match dispatch_command {
						Some(command) => self.handle_dispatch(command).await,
						None => {
							self.log_closed_channel(
								"dispatch_inbox",
								"actor task terminating because dispatch inbox closed",
							);
						}
					}
				}
				outcome = Self::wait_for_run_handle(self.run_handle.as_mut()), if self.run_handle.is_some() => {
					self.handle_run_handle_outcome(outcome);
				}
				_ = Self::state_save_tick(self.state_save_deadline), if self.state_save_timer_active() => {
					self.on_state_save_tick().await;
				}
				_ = Self::inspector_serialize_state_tick(self.inspector_serialize_state_deadline), if self.inspector_serialize_timer_active() => {
					self.on_inspector_serialize_state_tick().await;
				}
				idle_ready = &mut idle_wait => {
					if !idle_ready {
						tracing::warn!(
							timeout_ms = config.effective_sleep_grace_period().as_millis() as u64,
							"sleep shutdown reached the idle wait deadline"
						);
					}
					break;
				}
			}
		}

		self.enter_shutdown_state_machine(StopReason::Sleep);
	}

	fn enter_shutdown_state_machine(&mut self, reason: StopReason) {
		let started_at = Instant::now();
		let deadline = started_at
			+ match reason {
				StopReason::Sleep => {
					self.transition_to(LifecycleState::SleepFinalize);
					self.factory.config().effective_sleep_grace_period()
				}
				StopReason::Destroy => {
					self.transition_to(LifecycleState::Destroying);
					for conn in self.ctx.conns() {
						if conn.is_hibernatable() {
							self
								.ctx
								.request_hibernation_transport_removal(conn.id().to_owned());
						}
					}
					self.factory.config().effective_on_destroy_timeout()
				}
			};
		self.shutdown_reason = Some(reason);
		self.shutdown_started_at = Some(started_at);
		self.shutdown_deadline = Some(deadline);
		self.shutdown_phase = None;
		self.shutdown_finalize_reply = None;
		self.state_save_deadline = None;
		self.inspector_serialize_state_deadline = None;
		self.sleep_deadline = None;
		self.ctx.cancel_sleep_timer();
		self.ctx.schedule().suspend_alarm_dispatch();
		self.ctx.cancel_local_alarm_timeouts();
		self.ctx.schedule().set_local_alarm_callback(None);
		self.install_shutdown_step(ShutdownPhase::SendingFinalize);
	}

	#[cfg_attr(not(test), allow(dead_code))]
	async fn drain_tracked_work(
		&mut self,
		reason: StopReason,
		phase: &'static str,
		deadline: Instant,
	) -> bool {
		Self::drain_tracked_work_with_ctx(self.ctx.clone(), reason, phase, deadline).await
	}

	fn register_shutdown_reply(&mut self, reply: oneshot::Sender<Result<()>>) {
		self.shutdown_replies.push(reply);
	}

	#[cfg_attr(not(test), allow(dead_code))]
	async fn drive_shutdown_to_completion(&mut self) {
		while self.shutdown_step.is_some() {
			let outcome = Self::poll_shutdown_step(self.shutdown_step.as_mut()).await;
			self.on_shutdown_step_complete(outcome);
		}
	}

	async fn poll_shutdown_step(
		step: Option<&mut ShutdownStep>,
	) -> Result<ShutdownPhase> {
		match step {
			Some(step) => step.await,
			None => future::pending().await,
		}
	}

	fn on_shutdown_step_complete(
		&mut self,
		outcome: Result<ShutdownPhase>,
	) {
		self.shutdown_step = None;
		match outcome {
			Ok(next) => self.install_shutdown_step(next),
			Err(error) => self.complete_shutdown(Err(error)),
		}
	}

	fn install_shutdown_step(&mut self, phase: ShutdownPhase) {
		self.shutdown_phase = Some(phase);
		let reason = self
			.shutdown_reason
			.expect("shutdown reason should be set before installing a step");
		let deadline = self
			.shutdown_deadline
			.expect("shutdown deadline should be set before installing a step");
		let reason_label = shutdown_reason_label(reason);

		self.shutdown_step = match phase {
			ShutdownPhase::SendingFinalize => {
				let actor_event_tx = self.actor_event_tx.clone();
				let (reply_tx, reply_rx) = oneshot::channel();
				self.shutdown_finalize_reply = Some(reply_rx);
				Some(Self::boxed_shutdown_step(phase, async move {
					if let Some(sender) = actor_event_tx {
						match sender.try_reserve_owned() {
							Ok(permit) => {
								let event = match reason {
									StopReason::Sleep => ActorEvent::FinalizeSleep {
										reply: Reply::from(reply_tx),
									},
									StopReason::Destroy => ActorEvent::Destroy {
										reply: Reply::from(reply_tx),
									},
								};
								permit.send(event);
							}
							Err(_) => {
								tracing::warn!(
									reason = reason_label,
									"failed to enqueue shutdown event"
								);
							}
						}
					}
					Ok(ShutdownPhase::AwaitingFinalizeReply)
				}))
			}
			ShutdownPhase::AwaitingFinalizeReply => {
				let reply_rx = self
					.shutdown_finalize_reply
					.take()
					.expect("shutdown finalize reply should be set before awaiting it");
				let timeout_duration = remaining_shutdown_budget(deadline);
				Some(Self::boxed_shutdown_step(phase, async move {
					match timeout(timeout_duration, reply_rx).await {
						Ok(Ok(Ok(()))) => {}
						Ok(Ok(Err(error))) => {
							tracing::error!(?error, reason = reason_label, "actor shutdown event failed");
						}
						Ok(Err(error)) => {
							tracing::error!(?error, reason = reason_label, "actor shutdown reply dropped");
						}
						Err(_) => {
							tracing::warn!(
								reason = reason_label,
								timeout_ms = timeout_duration.as_millis() as u64,
								"actor shutdown event timed out"
							);
						}
					}
					Ok(ShutdownPhase::DrainingBefore)
				}))
			}
			ShutdownPhase::DrainingBefore => {
				let ctx = self.ctx.clone();
				Some(Self::boxed_shutdown_step(phase, async move {
					if !Self::drain_tracked_work_with_ctx(
						ctx.clone(),
						reason,
						"before_disconnect",
						deadline,
					)
					.await
					{
						ctx.record_shutdown_timeout(reason);
						tracing::warn!(
							"{reason_label} shutdown timed out waiting for shutdown tasks"
						);
					}
					Ok(ShutdownPhase::DisconnectingConns)
				}))
			}
			ShutdownPhase::DisconnectingConns => {
				let ctx = self.ctx.clone();
				Some(Self::boxed_shutdown_step(phase, async move {
					Self::disconnect_for_shutdown_with_ctx(
						ctx,
						match reason {
							StopReason::Sleep => "actor sleeping",
							StopReason::Destroy => "actor destroyed",
						},
						matches!(reason, StopReason::Sleep),
					)
					.await?;
					Ok(ShutdownPhase::DrainingAfter)
				}))
			}
			ShutdownPhase::DrainingAfter => {
				let ctx = self.ctx.clone();
				Some(Self::boxed_shutdown_step(phase, async move {
					if !Self::drain_tracked_work_with_ctx(
						ctx.clone(),
						reason,
						"after_disconnect",
						deadline,
					)
					.await
					{
						ctx.record_shutdown_timeout(reason);
						tracing::warn!(
							"{reason_label} shutdown timed out after disconnect callbacks"
						);
					}
					Ok(ShutdownPhase::AwaitingRunHandle)
				}))
			}
			ShutdownPhase::AwaitingRunHandle => {
				self.close_actor_event_channel();
				let run_handle = self.run_handle.take();
				let timeout_duration = remaining_shutdown_budget(deadline);
				Some(Self::boxed_shutdown_step(phase, async move {
					if let Some(mut run_handle) = run_handle {
						tokio::select! {
							outcome = &mut run_handle => {
								match outcome {
									Ok(Ok(())) => {}
									Ok(Err(error)) => {
										tracing::error!(?error, "actor run handler failed during shutdown");
									}
									Err(error) => {
										tracing::error!(?error, "actor run handler join failed during shutdown");
									}
								}
							}
							_ = sleep(timeout_duration) => {
								run_handle.abort();
								tracing::warn!(
									reason = reason_label,
									timeout_ms = timeout_duration.as_millis() as u64,
									"actor run handler timed out during shutdown"
								);
							}
						}
					}
					Ok(ShutdownPhase::Finalizing)
				}))
			}
			ShutdownPhase::Finalizing => {
				let ctx = self.ctx.clone();
				Some(Self::boxed_shutdown_step(phase, async move {
					Self::finish_shutdown_cleanup_with_ctx(ctx, reason).await?;
					Ok(ShutdownPhase::Done)
				}))
			}
			ShutdownPhase::Done => {
				self.complete_shutdown(Ok(()));
				None
			}
		};
	}

	fn boxed_shutdown_step<F>(phase: ShutdownPhase, future: F) -> ShutdownStep
	where
		F: Future<Output = Result<ShutdownPhase>> + Send + 'static,
	{
		Box::pin(async move {
			match AssertUnwindSafe(future).catch_unwind().await {
				Ok(outcome) => outcome,
				Err(_) => Err(anyhow!("shutdown phase {phase:?} panicked")),
			}
		})
	}

	async fn drain_tracked_work_with_ctx(
		ctx: ActorContext,
		reason: StopReason,
		phase: &'static str,
		deadline: Instant,
	) -> bool {
		let started_at = Instant::now();
		tokio::select! {
			result = ctx.wait_for_shutdown_tasks(deadline) => result,
			_ = sleep(LONG_SHUTDOWN_DRAIN_WARNING_THRESHOLD) => {
				if ctx.wait_for_shutdown_tasks(Instant::now()).await {
					true
				} else {
					ctx.warn_long_shutdown_drain(
						reason.as_metric_label(),
						phase,
						Instant::now().duration_since(started_at),
					);
					ctx.wait_for_shutdown_tasks(deadline).await
				}
			}
		}
	}

	async fn disconnect_for_shutdown_with_ctx(
		ctx: ActorContext,
		reason: &'static str,
		preserve_hibernatable: bool,
	) -> Result<()> {
		let connections: Vec<_> = ctx.conns().collect();
		for conn in connections {
			if preserve_hibernatable && conn.is_hibernatable() {
				continue;
			}

			if let Err(error) = conn.disconnect(Some(reason)).await {
				tracing::error!(
					?error,
					conn_id = conn.id(),
					"failed to disconnect connection during shutdown"
				);
			}
		}

		Ok(())
	}

	async fn finish_shutdown_cleanup_with_ctx(
		ctx: ActorContext,
		reason: StopReason,
	) -> Result<()> {
		let reason_label = shutdown_reason_label(reason);
		ctx.teardown_sleep_controller().await;
		#[cfg(test)]
		run_shutdown_cleanup_hook(&ctx, reason_label);
		ctx.wait_for_pending_state_writes().await;
		ctx.schedule().sync_alarm_logged();
		ctx.schedule().wait_for_pending_alarm_writes().await;
		ctx
			.sql()
			.cleanup()
			.await
			.with_context(|| format!("cleanup sqlite during {reason_label} shutdown"))?;
		match reason {
			// Match the reference TS runtime: keep the persisted engine alarm armed
			// across sleep so the next instance still has a wake trigger, but abort
			// the local Tokio timer owned by the shutting-down instance.
			StopReason::Sleep => ctx.schedule().cancel_local_alarm_timeouts(),
			StopReason::Destroy => ctx.schedule().cancel_driver_alarm_logged(),
		}
		Ok(())
	}

	fn complete_shutdown(&mut self, result: Result<()>) {
		let reason = self.shutdown_reason.take();
		let started_at = self.shutdown_started_at.take();
		self.shutdown_deadline = None;
		self.shutdown_phase = None;
		self.shutdown_step = None;
		self.shutdown_finalize_reply = None;
		self.transition_to(LifecycleState::Terminated);

		if let Some(reason) = reason {
			if result.is_ok() {
				if let Some(started_at) = started_at {
					self.ctx.record_shutdown_wait(reason, started_at.elapsed());
				}
			}
			if matches!(reason, StopReason::Destroy) {
				self.ctx.mark_destroy_completed();
			}
			self.send_shutdown_replies(reason, &result);
		}
	}

	fn send_shutdown_replies(&mut self, _reason: StopReason, result: &Result<()>) {
		#[cfg(test)]
		run_shutdown_reply_hook(&self.ctx, _reason);

		for reply in self.shutdown_replies.drain(..) {
			let _ = reply.send(clone_shutdown_result(result));
		}
	}

	fn record_inbox_depths(&self) {
		self.ctx
			.metrics()
			.set_lifecycle_inbox_depth(self.lifecycle_inbox.len());
		self.ctx
			.metrics()
			.set_dispatch_inbox_depth(self.dispatch_inbox.len());
		self.ctx
			.metrics()
			.set_lifecycle_event_inbox_depth(self.lifecycle_events.len());
	}

	fn accepting_dispatch(&self) -> bool {
		matches!(
			self.lifecycle,
			LifecycleState::Started | LifecycleState::SleepGrace
		)
	}

	fn sleep_timer_active(&self) -> bool {
		self.sleep_deadline.is_some()
	}

	fn state_save_timer_active(&self) -> bool {
		self.state_save_deadline.is_some()
	}

	fn inspector_serialize_timer_active(&self) -> bool {
		self.inspector_serialize_state_deadline.is_some()
	}

	fn schedule_state_save(&mut self, immediate: bool) {
		if !matches!(
			self.lifecycle,
			LifecycleState::Started | LifecycleState::SleepGrace
		) || !self.ctx.save_requested()
		{
			self.state_save_deadline = None;
			return;
		}

		let next_deadline = self.ctx.save_deadline(immediate);
		self.state_save_deadline = Some(match self.state_save_deadline {
			Some(existing) => existing.min(next_deadline),
			None => next_deadline,
		});
	}

	async fn sleep_tick(deadline: Option<Instant>) {
		let Some(deadline) = deadline else {
			future::pending::<()>().await;
			return;
		};

		sleep_until(deadline).await;
	}

	async fn state_save_tick(deadline: Option<Instant>) {
		let Some(deadline) = deadline else {
			future::pending::<()>().await;
			return;
		};

		sleep_until(deadline).await;
	}

	async fn inspector_serialize_state_tick(deadline: Option<Instant>) {
		let Some(deadline) = deadline else {
			future::pending::<()>().await;
			return;
		};

		sleep_until(deadline).await;
	}

	async fn on_state_save_tick(&mut self) {
		self.state_save_deadline = None;
		self.inspector_serialize_state_deadline = None;
		if !matches!(
			self.lifecycle,
			LifecycleState::Started | LifecycleState::SleepGrace
		) || !self.ctx.save_requested()
		{
			return;
		}

		let save_request_revision = self.ctx.save_request_revision();
		let (reply_tx, reply_rx) = oneshot::channel();
		match self.reserve_actor_event("save_tick") {
			Ok(permit) => {
				permit.send(ActorEvent::SerializeState {
					reason: SerializeStateReason::Save,
					reply: Reply::from(reply_tx),
				});
			}
			Err(error) => {
				tracing::warn!(?error, "failed to enqueue save tick");
				self.schedule_state_save(true);
				return;
			}
		}

		match reply_rx.await {
			Ok(Ok(deltas)) => {
				self.broadcast_inspector_overlay(&deltas);
				if let Err(error) = self
					.ctx
					.save_state_with_revision(deltas, save_request_revision)
					.await
				{
					tracing::error!(?error, "failed to persist actor save tick");
					self.schedule_state_save(true);
					self.sync_inspector_serialize_deadline();
				} else if self.ctx.save_requested() {
					self.schedule_state_save(self.ctx.save_requested_immediate());
					self.sync_inspector_serialize_deadline();
				}
			}
			Ok(Err(error)) => {
				tracing::error!(?error, "actor save tick failed");
				self.schedule_state_save(true);
				self.sync_inspector_serialize_deadline();
			}
			Err(error) => {
				tracing::error!(?error, "actor save tick reply dropped");
				self.schedule_state_save(true);
				self.sync_inspector_serialize_deadline();
			}
		}
	}

	async fn on_inspector_serialize_state_tick(&mut self) {
		self.inspector_serialize_state_deadline = None;
		if !matches!(
			self.lifecycle,
			LifecycleState::Started | LifecycleState::SleepGrace
		)
			|| self.inspector_attach_count.load(Ordering::SeqCst) == 0
			|| !self.ctx.save_requested()
		{
			return;
		}

		let (reply_tx, reply_rx) = oneshot::channel();
		match self.reserve_actor_event("inspector_serialize_state") {
			Ok(permit) => {
				permit.send(ActorEvent::SerializeState {
					reason: SerializeStateReason::Inspector,
					reply: Reply::from(reply_tx),
				});
			}
			Err(error) => {
				tracing::warn!(?error, "failed to enqueue inspector serialize tick");
				self.sync_inspector_serialize_deadline();
				return;
			}
		}

		match reply_rx.await {
			Ok(Ok(deltas)) => {
				self.broadcast_inspector_overlay(&deltas);
			}
			Ok(Err(error)) => {
				tracing::error!(?error, "actor inspector serialize tick failed");
				self.sync_inspector_serialize_deadline();
			}
			Err(error) => {
				tracing::error!(?error, "actor inspector serialize tick reply dropped");
				self.sync_inspector_serialize_deadline();
			}
		}
	}

	async fn on_sleep_tick(&mut self) {
		self.sleep_deadline = None;
		if self.lifecycle != LifecycleState::Started {
			return;
		}

		if self.ctx.can_sleep().await == crate::actor::sleep::CanSleep::Yes {
			self.ctx.sleep();
		} else {
			self.reset_sleep_deadline().await;
		}
	}

	async fn reset_sleep_deadline(&mut self) {
		if self.lifecycle != LifecycleState::Started {
			self.sleep_deadline = None;
			return;
		}

		if self.ctx.can_sleep().await == crate::actor::sleep::CanSleep::Yes {
			self.sleep_deadline =
				Some(Instant::now() + self.factory.config().sleep_timeout);
		} else {
			self.sleep_deadline = None;
		}
	}

	fn sync_inspector_serialize_deadline(&mut self) {
		if !matches!(
			self.lifecycle,
			LifecycleState::Started | LifecycleState::SleepGrace
		)
			|| self.inspector_attach_count.load(Ordering::SeqCst) == 0
			|| !self.ctx.save_requested()
		{
			self.inspector_serialize_state_deadline = None;
			return;
		}

		self.inspector_serialize_state_deadline
			.get_or_insert_with(|| Instant::now() + INSPECTOR_SERIALIZE_STATE_INTERVAL);
	}

	fn broadcast_inspector_overlay(&self, deltas: &[crate::actor::callbacks::StateDelta]) {
		if self.inspector_attach_count.load(Ordering::SeqCst) == 0 || deltas.is_empty() {
			return;
		}

		let mut payload = Vec::new();
		if let Err(error) = ciborium::into_writer(deltas, &mut payload) {
			tracing::error!(?error, "failed to encode inspector overlay deltas");
			return;
		}

		let _ = self.inspector_overlay_tx.send(Arc::new(payload));
	}

	fn should_terminate(&self) -> bool {
		matches!(self.lifecycle, LifecycleState::Terminated)
	}

	fn log_closed_channel(&self, channel: &'static str, message: &'static str) {
		tracing::warn!(
			actor_id = %self.ctx.actor_id(),
			channel,
			reason = "all senders dropped",
			"{message}"
		);
	}

	fn transition_to(&mut self, lifecycle: LifecycleState) {
		self.lifecycle = lifecycle;
		match lifecycle {
			LifecycleState::Ready
			| LifecycleState::Started
			| LifecycleState::SleepGrace => self.ctx.set_ready(true),
			LifecycleState::Loading
			| LifecycleState::Migrating
			| LifecycleState::Waking
			| LifecycleState::SleepFinalize
			| LifecycleState::Destroying
			| LifecycleState::Terminated => self.ctx.set_ready(false),
		}

		self
			.ctx
			.set_started(matches!(lifecycle, LifecycleState::Started | LifecycleState::SleepGrace));
	}

	fn request_begin_sleep(&mut self) {
		if self.run_handle.is_none() {
			return;
		}

		match self.reserve_actor_event("begin_sleep") {
			Ok(permit) => {
				permit.send(ActorEvent::BeginSleep);
			}
			Err(error) => {
				tracing::warn!(?error, "failed to enqueue begin-sleep event");
			}
		}
	}
}

fn remaining_shutdown_budget(deadline: Instant) -> Duration {
	deadline.saturating_duration_since(Instant::now())
}

fn shutdown_reason_label(reason: StopReason) -> &'static str {
	match reason {
		StopReason::Sleep => "sleep",
		StopReason::Destroy => "destroy",
	}
}

fn clone_shutdown_result(result: &Result<()>) -> Result<()> {
	match result {
		Ok(()) => Ok(()),
		Err(error) => Err(anyhow!(error.to_string())),
	}
}
