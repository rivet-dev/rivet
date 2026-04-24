//! Actor lifecycle task orchestration.
//!
//! `ActorTask` deliberately uses four separate bounded `mpsc` receivers instead
//! of one tagged command queue:
//!
//! - `lifecycle_inbox` carries trusted registry/envoy lifecycle commands:
//!   start, stop, destroy, and driver-alarm wakeups.
//! - `dispatch_inbox` carries client-facing actor work such as actions, raw
//!   HTTP, raw WebSockets, and inspector workflow requests.
//! - `lifecycle_events` carries internal subsystem signals from
//!   `ActorContext`: save requests, activity changes, inspector attach changes,
//!   and sleep ticks.
//! - `actor_event_rx` feeds the user runtime adapter with actor events after
//!   `ActorTask` accepts dispatch work.
//!
//! Keeping these queues split gives the task loop explicit back-pressure and
//! priority boundaries. Client dispatch can fill its own bounded inbox without
//! starving lifecycle stop/destroy commands, while internal save/sleep/inspector
//! events do not compete with untrusted client traffic. The main `tokio::select!`
//! is biased so lifecycle commands are observed first, then internal lifecycle
//! events, then dispatch and timers. During sleep grace, the same priority keeps
//! lifecycle handling live while still draining accepted dispatch replies before
//! final teardown.
//!
//! Producers reserve capacity with `try_reserve` before constructing channel
//! work. Overload paths therefore fail fast with `actor.overloaded`, record the
//! specific inbox metric (`lifecycle_inbox`, `dispatch_inbox`,
//! `lifecycle_event_inbox`, or `actor_event_inbox`), and avoid orphaning reply
//! oneshots. The sender topology follows the trust boundary: registry/envoy owns
//! lifecycle and dispatch senders, core subsystems enqueue lifecycle events
//! through `ActorContext`, and only `ActorTask` forwards accepted work into the
//! actor-event stream consumed by user code.

use std::future;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
#[cfg(test)]
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};

use anyhow::{Context, Result, anyhow};
use futures::FutureExt;
#[cfg(test)]
use parking_lot::Mutex;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::{JoinError, JoinHandle};
use tokio::time::{Duration, Instant, sleep_until, timeout};
use tracing::Instrument;

use crate::actor::action::ActionDispatchError;
use crate::actor::connection::ConnHandle;
use crate::actor::context::ActorContext;
use crate::actor::diagnostics::record_actor_warning;
use crate::actor::factory::ActorFactory;
use crate::actor::lifecycle_hooks::{ActorEvents, ActorStart, Reply};
use crate::actor::messages::{
	ActorEvent, QueueSendResult, Request, Response, SerializeStateReason, StateDelta,
};
use crate::actor::metrics::ActorMetrics;
use crate::actor::preload::{PreloadedKv, PreloadedPersistedActor};
use crate::actor::state::{
	LAST_PUSHED_ALARM_KEY, PERSIST_DATA_KEY, PersistedActor, decode_last_pushed_alarm,
	decode_persisted_actor,
};
use crate::actor::task_types::StopReason;
use crate::error::{ActorLifecycle as ActorLifecycleError, ActorRuntime};
use crate::types::{SaveStateOpts, format_actor_key};
use crate::websocket::WebSocket;

pub type ActionDispatchResult = std::result::Result<Vec<u8>, ActionDispatchError>;
pub type HttpDispatchResult = Result<Response>;

const SERIALIZE_STATE_SHUTDOWN_SANITY_CAP: Duration = Duration::from_secs(30);
#[cfg(test)]
const LONG_SHUTDOWN_DRAIN_WARNING_THRESHOLD: Duration = Duration::from_secs(1);
const INSPECTOR_SERIALIZE_STATE_INTERVAL: Duration = Duration::from_millis(50);
const INSPECTOR_OVERLAY_CHANNEL_CAPACITY: usize = 32;

pub(crate) const LIFECYCLE_INBOX_CHANNEL: &str = "lifecycle_inbox";
pub(crate) const DISPATCH_INBOX_CHANNEL: &str = "dispatch_inbox";
pub(crate) const LIFECYCLE_EVENT_INBOX_CHANNEL: &str = "lifecycle_event_inbox";
pub use crate::actor::task_types::LifecycleState;

#[cfg(test)]
#[path = "../../tests/modules/task.rs"]
mod tests;

#[cfg(test)]
type ShutdownCleanupHook = Arc<dyn Fn(&ActorContext, &'static str) + Send + Sync>;

#[cfg(test)]
// Forced-sync: test hooks are installed and cleared from synchronous guard APIs.
static SHUTDOWN_CLEANUP_HOOK: OnceLock<Mutex<Option<ShutdownCleanupHook>>> = OnceLock::new();

#[cfg(test)]
pub(crate) struct ShutdownCleanupHookGuard;

#[cfg(test)]
type ShutdownReplyHook = Arc<dyn Fn(&ActorContext, StopReason) + Send + Sync>;

#[cfg(test)]
// Forced-sync: test hooks are installed and cleared from synchronous guard APIs.
static SHUTDOWN_REPLY_HOOK: OnceLock<Mutex<Option<ShutdownReplyHook>>> = OnceLock::new();

#[cfg(test)]
pub(crate) struct ShutdownReplyHookGuard;

#[cfg(test)]
pub(crate) fn install_shutdown_cleanup_hook(hook: ShutdownCleanupHook) -> ShutdownCleanupHookGuard {
	*SHUTDOWN_CLEANUP_HOOK
		.get_or_init(|| Mutex::new(None))
		.lock() = Some(hook);
	ShutdownCleanupHookGuard
}

#[cfg(test)]
impl Drop for ShutdownCleanupHookGuard {
	fn drop(&mut self) {
		if let Some(hooks) = SHUTDOWN_CLEANUP_HOOK.get() {
			*hooks.lock() = None;
		}
	}
}

#[cfg(test)]
fn run_shutdown_cleanup_hook(ctx: &ActorContext, reason: &'static str) {
	let hook = SHUTDOWN_CLEANUP_HOOK
		.get_or_init(|| Mutex::new(None))
		.lock()
		.clone();
	if let Some(hook) = hook {
		hook(ctx, reason);
	}
}

#[cfg(test)]
pub(crate) fn install_shutdown_reply_hook(hook: ShutdownReplyHook) -> ShutdownReplyHookGuard {
	*SHUTDOWN_REPLY_HOOK.get_or_init(|| Mutex::new(None)).lock() = Some(hook);
	ShutdownReplyHookGuard
}

#[cfg(test)]
impl Drop for ShutdownReplyHookGuard {
	fn drop(&mut self) {
		if let Some(hooks) = SHUTDOWN_REPLY_HOOK.get() {
			*hooks.lock() = None;
		}
	}
}

#[cfg(test)]
fn run_shutdown_reply_hook(ctx: &ActorContext, reason: StopReason) {
	let hook = SHUTDOWN_REPLY_HOOK
		.get_or_init(|| Mutex::new(None))
		.lock()
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

impl LifecycleCommand {
	fn kind(&self) -> &'static str {
		match self {
			Self::Start { .. } => "start",
			Self::Stop { .. } => "stop",
			Self::FireAlarm { .. } => "fire_alarm",
		}
	}

	fn stop_reason(&self) -> Option<&'static str> {
		match self {
			Self::Stop { reason, .. } => Some(shutdown_reason_label(*reason)),
			_ => None,
		}
	}
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
			LIFECYCLE_EVENT_INBOX_CHANNEL => metrics.inc_lifecycle_event_overload(operation),
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
	// Reserve capacity before sending so overload paths can return
	// `actor.overloaded` without waiting or constructing more channel-owned work.
	// Lifecycle callers also avoid creating reply oneshots when a full inbox would
	// immediately orphan them.
	let permit = sender.try_reserve().map_err(|_| {
		actor_channel_overloaded_error(LIFECYCLE_INBOX_CHANNEL, capacity, operation, metrics)
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
	QueueSend {
		name: String,
		body: Vec<u8>,
		conn: ConnHandle,
		request: Request,
		wait: bool,
		timeout_ms: Option<u64>,
		reply: oneshot::Sender<Result<QueueSendResult>>,
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

impl DispatchCommand {
	fn kind(&self) -> &'static str {
		match self {
			Self::Action { .. } => "action",
			Self::QueueSend { .. } => "queue_send",
			Self::Http { .. } => "http",
			Self::OpenWebSocket { .. } => "open_websocket",
			Self::WorkflowHistory { .. } => "workflow_history",
			Self::WorkflowReplay { .. } => "workflow_replay",
		}
	}
}

pub(crate) fn try_send_dispatch_command(
	sender: &mpsc::Sender<DispatchCommand>,
	capacity: usize,
	operation: &'static str,
	command: DispatchCommand,
	metrics: Option<&ActorMetrics>,
) -> Result<()> {
	// Match lifecycle command backpressure semantics: capacity is checked before
	// handing the value to the channel, which keeps reject paths cheap and avoids
	// `try_send` returning a fully built command that must be discarded.
	let permit = sender.try_reserve().map_err(|_| {
		actor_channel_overloaded_error(DISPATCH_INBOX_CHANNEL, capacity, operation, metrics)
	})?;
	permit.send(command);
	Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleEvent {
	SaveRequested { immediate: bool },
	InspectorSerializeRequested,
	InspectorAttachmentsChanged,
	SleepTick,
}

impl LifecycleEvent {
	fn kind(&self) -> &'static str {
		match self {
			Self::SaveRequested { .. } => "save_requested",
			Self::InspectorSerializeRequested => "inspector_serialize_requested",
			Self::InspectorAttachmentsChanged => "inspector_attachments_changed",
			Self::SleepTick => "sleep_tick",
		}
	}
}

enum LiveExit {
	Shutdown { reason: StopReason },
	Terminated,
}

struct SleepGraceState {
	deadline: Instant,
	reason: StopReason,
}

struct PersistedStartup {
	actor: PersistedActor,
	last_pushed_alarm: Option<i64>,
}

struct PendingLifecycleReply {
	command: &'static str,
	reason: Option<&'static str>,
	reply: oneshot::Sender<Result<()>>,
}

pub struct ActorTask {
	// === IDENTITY ===
	pub actor_id: String,
	pub generation: u32,

	// === INBOX CHANNELS ===
	/// Lifecycle commands (Start / Stop / FireAlarm) sent by the registry
	/// in response to engine-driven `EnvoyCallbacks` from the envoy client.
	pub lifecycle_inbox: mpsc::Receiver<LifecycleCommand>,
	/// Client-originated work sent by `RegistryDispatcher` in
	/// `registry/dispatch.rs` (Action, OpenWebSocket, Workflow*) and
	/// `registry/http.rs` (Http, QueueSend).
	pub dispatch_inbox: mpsc::Receiver<DispatchCommand>,
	/// Internal self-events the actor enqueues onto itself via `ActorContext`
	/// hooks (save/inspector/activity notifications from
	/// `actor/state.rs`, `actor/connection.rs`, `actor/context.rs`).
	pub lifecycle_events: mpsc::Receiver<LifecycleEvent>,

	// === RUNTIME STATE ===
	pub lifecycle: LifecycleState,
	pub factory: Arc<ActorFactory>,
	pub ctx: ActorContext,

	// === STARTUP ===
	pub start_input: Option<Vec<u8>>,
	/// Optional persisted snapshot supplied by the registry to skip the
	/// initial KV fetch. Tri-state: `NoBundle` falls back to KV,
	/// `BundleExistsButEmpty` means fresh actor defaults, `Some` decodes
	/// the persisted actor.
	preload_persisted_actor: PreloadedPersistedActor,
	/// Optional preloaded KV entries (e.g. `[1]`, `[2] + conn_id`,
	/// `[5, 1, *]`) supplied alongside `preload_persisted_actor` so startup
	/// avoids extra round trips.
	preloaded_kv: Option<PreloadedKv>,

	// === USER RUNTIME BRIDGE ===
	/// Sends `ActorEvent`s from core subsystems and `ActorTask` to the
	/// user runtime adapter.
	actor_event_tx: Option<mpsc::UnboundedSender<ActorEvent>>,
	/// Receiver half. Not consumed by `ActorTask`. `spawn_run_handle`
	/// `take()`s it and hands it to the user `run` handler via `ActorStart`
	/// so the runtime adapter (e.g. NAPI receive loop) drains events there.
	actor_event_rx: Option<mpsc::UnboundedReceiver<ActorEvent>>,
	/// Join handle for the user `run` task spawned by `spawn_run_handle`.
	/// Awaited as a `select!` arm; cleared on shutdown abort/await.
	run_handle: Option<JoinHandle<Result<()>>>,

	// === INSPECTOR ===
	/// Live count of attached inspector websockets. Read from request-save
	/// hooks to decide whether to debounce a `SerializeState { Inspector }`.
	inspector_attach_count: Arc<AtomicU32>,
	/// Live `StateDelta` stream broadcast to attached inspector WebSockets
	/// so their snapshot stays in sync without re-fetching.
	inspector_overlay_tx: broadcast::Sender<Arc<Vec<u8>>>,

	// === TIMERS ===
	/// Next deadline at which `on_state_save_tick` should flush a deferred
	/// state save. Cleared while no save is requested.
	pub state_save_deadline: Option<Instant>,
	/// Next deadline at which an inspector-driven `SerializeState` should
	/// fire. Debounces inspector overlay refreshes.
	pub inspector_serialize_state_deadline: Option<Instant>,
	/// Next deadline at which the actor becomes eligible for sleep if it
	/// stays idle. Cleared on activity and during sleep grace.
	pub sleep_deadline: Option<Instant>,

	// === SHUTDOWN ===
	/// The single lifecycle reply for shutdown. Engine actor2 sends at most
	/// one Stop command per actor instance; duplicates are a protocol bug.
	shutdown_reply: Option<PendingLifecycleReply>,
	/// Active sleep-grace idle wait. Polled by the main loop so grace keeps the
	/// same inbox/timer handling as the started actor.
	sleep_grace: Option<SleepGraceState>,
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
		let (actor_event_tx, actor_event_rx) = mpsc::unbounded_channel();
		let (inspector_overlay_tx, _) = broadcast::channel(INSPECTOR_OVERLAY_CHANNEL_CAPACITY);
		let inspector_attach_count = Arc::new(AtomicU32::new(0));
		ctx.configure_inspector_runtime(
			Arc::clone(&inspector_attach_count),
			inspector_overlay_tx.clone(),
		);
		let inspector_ctx = ctx.clone();
		let inspector_attach_count_for_hook = Arc::clone(&inspector_attach_count);
		ctx.on_request_save(Box::new(move |_opts| {
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
			preload_persisted_actor: preload_persisted_actor.into(),
			preloaded_kv: None,
			actor_event_tx: Some(actor_event_tx),
			actor_event_rx: Some(actor_event_rx),
			run_handle: None,
			inspector_attach_count,
			inspector_overlay_tx,
			state_save_deadline: None,
			inspector_serialize_state_deadline: None,
			sleep_deadline: None,
			shutdown_reply: None,
			sleep_grace: None,
		}
	}

	pub(crate) fn with_preloaded_kv(mut self, preloaded_kv: Option<PreloadedKv>) -> Self {
		self.preloaded_kv = preloaded_kv;
		self
	}

	pub(crate) fn with_preloaded_persisted_actor(
		mut self,
		preload_persisted_actor: PreloadedPersistedActor,
	) -> Self {
		self.preload_persisted_actor = preload_persisted_actor;
		self
	}

	#[tracing::instrument(
		skip_all,
		fields(
			actor_id = %self.actor_id,
			generation = self.generation,
			actor_key = %format_actor_key(self.ctx.key()),
		),
	)]
	pub async fn run(mut self) -> Result<()> {
		let exit = self.run_live().await;
		let LiveExit::Shutdown { reason } = exit else {
			self.record_inbox_depths();
			return Ok(());
		};

		let result = match AssertUnwindSafe(self.run_shutdown(reason))
			.catch_unwind()
			.await
		{
			Ok(result) => result,
			Err(_) => Err(anyhow!("shutdown panicked during {reason:?}")),
		};
		self.deliver_shutdown_reply(reason, &result);
		self.transition_to(LifecycleState::Terminated);
		self.record_inbox_depths();
		result
	}

	async fn run_live(&mut self) -> LiveExit {
		let activity_notify = self.ctx.sleep_activity_notify();

		loop {
			if self.ctx.acknowledge_activity_dirty() {
				if let Some(exit) = self.on_activity_signal().await {
					return exit;
				}
			}
			self.record_inbox_depths();
			tokio::select! {
				biased;
				lifecycle_command = self.lifecycle_inbox.recv() => {
					match lifecycle_command {
						Some(command) => {
							if let Some(exit) = self.handle_lifecycle(command).await {
								return exit;
							}
						}
						None => {
							self.log_closed_channel(
								"lifecycle_inbox",
								"actor task terminating because lifecycle command inbox closed",
							);
							return LiveExit::Terminated;
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
							return LiveExit::Terminated;
						}
					}
				}
				_ = activity_notify.notified() => {
					self.ctx.acknowledge_activity_dirty();
					if let Some(exit) = self.on_activity_signal().await {
						return exit;
					}
				}
				_ = Self::sleep_grace_tick(self.sleep_grace.as_ref().map(|grace| grace.deadline)), if self.sleep_grace.is_some() => {
					if let Some(exit) = self.on_sleep_grace_deadline().await {
						return exit;
					}
				}
				dispatch_command = self.dispatch_inbox.recv(), if self.accepting_dispatch() => {
					match dispatch_command {
						Some(command) => self.handle_dispatch(command).await,
						None => {
							self.log_closed_channel(
								"dispatch_inbox",
								"actor task terminating because dispatch inbox closed",
							);
							return LiveExit::Terminated;
						}
					}
				}
				outcome = Self::wait_for_run_handle(self.run_handle.as_mut()), if self.run_handle.is_some() => {
					if let Some(exit) = self.handle_run_handle_outcome(outcome) {
						return exit;
					}
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
				return LiveExit::Terminated;
			}
		}
	}

	async fn handle_lifecycle(&mut self, command: LifecycleCommand) -> Option<LiveExit> {
		let command_kind = command.kind();
		let reason = command.stop_reason();
		self.log_lifecycle_command_received(command_kind, reason);
		if matches!(
			self.lifecycle,
			LifecycleState::SleepGrace | LifecycleState::DestroyGrace
		) {
			return self
				.handle_sleep_grace_lifecycle(command, command_kind, reason)
				.await;
		}
		match command {
			LifecycleCommand::Start { reply } => {
				let result = self.start_actor().await;
				self.reply_lifecycle_command(command_kind, reason, reply, result);
				None
			}
			LifecycleCommand::Stop { reason, reply } => {
				self.begin_stop(
					reason,
					command_kind,
					Some(shutdown_reason_label(reason)),
					reply,
				)
				.await
			}
			LifecycleCommand::FireAlarm { reply } => {
				let result = self.fire_due_alarms().await;
				self.reply_lifecycle_command(command_kind, reason, reply, result);
				None
			}
		}
	}

	async fn handle_sleep_grace_lifecycle(
		&mut self,
		command: LifecycleCommand,
		command_kind: &'static str,
		command_reason: Option<&'static str>,
	) -> Option<LiveExit> {
		match command {
			LifecycleCommand::Start { reply } => {
				self.reply_lifecycle_command(
					command_kind,
					command_reason,
					reply,
					Err(ActorLifecycleError::Stopping.build()),
				);
				None
			}
			LifecycleCommand::Stop { reason, reply } => {
				let current_reason = self.sleep_grace.as_ref().map(|grace| grace.reason);
				if current_reason != Some(reason) {
					debug_assert!(false, "engine actor2 sends one Stop per actor instance");
					tracing::warn!(
						actor_id = %self.ctx.actor_id(),
						reason = shutdown_reason_label(reason),
						current_reason = ?current_reason,
						"conflicting Stop during grace, ignoring"
					);
				}
				self.reply_lifecycle_command(command_kind, command_reason, reply, Ok(()));
				None
			}
			LifecycleCommand::FireAlarm { reply } => {
				let result = self.fire_due_alarms().await;
				self.reply_lifecycle_command(command_kind, command_reason, reply, result);
				None
			}
		}
	}

	#[cfg(test)]
	async fn handle_stop(&mut self, reason: StopReason) -> Result<()> {
		let (reply_tx, reply_rx) = oneshot::channel();
		self.register_shutdown_reply("stop", Some(shutdown_reason_label(reason)), reply_tx);
		self.begin_grace(reason).await;
		loop {
			if self.ctx.acknowledge_activity_dirty() {
				if let Some(exit) = self.on_activity_signal().await {
					let LiveExit::Shutdown { reason } = exit else {
						return Ok(());
					};
					let result = match AssertUnwindSafe(self.run_shutdown(reason))
						.catch_unwind()
						.await
					{
						Ok(result) => result,
						Err(_) => Err(anyhow!("shutdown panicked during {reason:?}")),
					};
					self.deliver_shutdown_reply(reason, &result);
					self.transition_to(LifecycleState::Terminated);
					return match reply_rx.await {
						Ok(result) => result,
						Err(_) => Err(ActorLifecycleError::DroppedReply.build()),
					};
				}
			}

			let Some(deadline) = self.sleep_grace.as_ref().map(|grace| grace.deadline) else {
				return Err(anyhow!("stop grace ended without shutdown exit"));
			};
			let activity_notify = self.ctx.sleep_activity_notify();
			let activity = activity_notify.notified();
			tokio::pin!(activity);

			tokio::select! {
				_ = &mut activity => {}
				_ = Self::sleep_grace_tick(Some(deadline)) => {
					if let Some(exit) = self.on_sleep_grace_deadline().await {
						let LiveExit::Shutdown { reason } = exit else {
							return Ok(());
						};
						let result = match AssertUnwindSafe(self.run_shutdown(reason))
							.catch_unwind()
							.await
						{
							Ok(result) => result,
							Err(_) => Err(anyhow!("shutdown panicked during {reason:?}")),
						};
						self.deliver_shutdown_reply(reason, &result);
						self.transition_to(LifecycleState::Terminated);
						return match reply_rx.await {
							Ok(result) => result,
							Err(_) => Err(ActorLifecycleError::DroppedReply.build()),
						};
					}
				}
			}
		}
	}

	async fn begin_stop(
		&mut self,
		reason: StopReason,
		command: &'static str,
		command_reason: Option<&'static str>,
		reply: oneshot::Sender<Result<()>>,
	) -> Option<LiveExit> {
		match self.lifecycle {
			LifecycleState::Started => {
				self.register_shutdown_reply(command, command_reason, reply);
				self.drain_accepted_dispatch().await;
				self.begin_grace(reason).await;
				self.try_finish_grace()
			}
			LifecycleState::SleepGrace | LifecycleState::DestroyGrace => {
				let current_reason = self.sleep_grace.as_ref().map(|grace| grace.reason);
				if current_reason == Some(reason) {
					self.reply_lifecycle_command(command, command_reason, reply, Ok(()));
					None
				} else {
					debug_assert!(false, "engine actor2 sends one Stop per actor instance");
					tracing::warn!(
						actor_id = %self.ctx.actor_id(),
						reason = shutdown_reason_label(reason),
					current_reason = ?current_reason,
						"conflicting Stop during grace, ignoring"
					);
					self.reply_lifecycle_command(command, command_reason, reply, Ok(()));
					None
				}
			}
			LifecycleState::SleepFinalize | LifecycleState::Destroying => {
				debug_assert!(false, "engine actor2 sends one Stop per actor instance");
				tracing::warn!(
					actor_id = %self.ctx.actor_id(),
					reason = shutdown_reason_label(reason),
					"duplicate Stop after shutdown started, ignoring"
				);
				self.reply_lifecycle_command(command, command_reason, reply, Ok(()));
				None
			}
			LifecycleState::Terminated => {
				self.reply_lifecycle_command(command, command_reason, reply, Ok(()));
				None
			}
			LifecycleState::Loading => {
				self.reply_lifecycle_command(
					command,
					command_reason,
					reply,
					Err(ActorLifecycleError::NotReady.build()),
				);
				None
			}
		}
	}

	async fn drain_accepted_dispatch(&mut self) {
		while self.accepting_dispatch() {
			let Ok(command) = self.dispatch_inbox.try_recv() else {
				break;
			};
			self.handle_dispatch(command).await;
		}
	}

	async fn begin_grace(&mut self, reason: StopReason) {
		tracing::debug!(
			actor_id = %self.ctx.actor_id(),
			reason = shutdown_reason_label(reason),
			"actor grace shutdown started"
		);
		self.ctx.suspend_alarm_dispatch();
		self.ctx.cancel_local_alarm_timeouts();
		self.ctx.set_local_alarm_callback(None);
		self.transition_to(match reason {
			StopReason::Sleep => LifecycleState::SleepGrace,
			StopReason::Destroy => LifecycleState::DestroyGrace,
		});
		self.start_grace(reason);
		self.emit_grace_events(reason);
	}

	fn emit_grace_events(&mut self, reason: StopReason) {
		let conns: Vec<_> = self.ctx.conns().collect();
		for conn in conns {
			let hibernatable_sleep = matches!(reason, StopReason::Sleep) && conn.is_hibernatable();
			if hibernatable_sleep {
				self.ctx.request_hibernation_transport_save(conn.id());
				continue;
			}
			self.ctx.begin_core_dispatched_hook();
			let reply = self.core_dispatched_hook_reply("disconnect_conn");
			let conn_id = conn.id().to_owned();
			if let Err(error) = self.send_actor_event(
				"grace_disconnect_conn",
				ActorEvent::DisconnectConn { conn_id, reply },
			) {
				tracing::error!(?error, "failed to enqueue disconnect cleanup event");
			}
		}

		self.ctx.begin_core_dispatched_hook();
		let reply = self.core_dispatched_hook_reply("run_graceful_cleanup");
		if let Err(error) = self.send_actor_event(
			"grace_run_cleanup",
			ActorEvent::RunGracefulCleanup { reason, reply },
		) {
			tracing::error!(?error, "failed to enqueue run cleanup event");
		}
		self.ctx.reset_sleep_timer();
	}

	fn core_dispatched_hook_reply(&self, operation: &'static str) -> Reply<()> {
		let (tx, rx) = oneshot::channel();
		let ctx = self.ctx.clone();
		tokio::spawn(
			async move {
				match rx.await {
					Ok(Ok(())) => {}
					Ok(Err(error)) => {
						tracing::error!(?error, operation, "core dispatched hook failed");
					}
					Err(error) => {
						tracing::error!(?error, operation, "core dispatched hook reply dropped");
					}
				}
				ctx.mark_core_dispatched_hook_completed();
			}
			.in_current_span(),
		);
		tx.into()
	}

	async fn handle_event(&mut self, event: LifecycleEvent) {
		tracing::debug!(
			actor_id = %self.ctx.actor_id(),
			event = event.kind(),
			"actor lifecycle event drained"
		);
		match event {
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
		let command_kind = command.kind();
		tracing::debug!(
			actor_id = %self.ctx.actor_id(),
			command = command_kind,
			"actor dispatch command received"
		);
		if let Some(error) = self.dispatch_lifecycle_error() {
			self.reply_dispatch_error(command, error);
			self.log_dispatch_command_handled(command_kind, "rejected_lifecycle");
			return;
		}

		match command {
			DispatchCommand::Action {
				name,
				args,
				conn,
				reply,
			} => {
				tracing::info!(
					actor_id = %self.ctx.actor_id(),
					action_name = %name,
					conn_id = ?conn.id(),
					args_len = args.len(),
					"actor task: handling DispatchCommand::Action"
				);
				let (tracked_reply_tx, tracked_reply_rx) = oneshot::channel();
				let action_name_for_log = name.clone();
				match self.send_actor_event(
					"dispatch_action",
					ActorEvent::Action {
						name,
						args,
						conn: Some(conn),
						reply: Reply::from(tracked_reply_tx),
					},
				) {
					Ok(()) => {
						tracing::info!(
							actor_id = %self.ctx.actor_id(),
							action_name = %action_name_for_log,
							"actor task: ActorEvent::Action enqueued"
						);
						self.log_dispatch_command_handled(command_kind, "enqueued");
						let actor_id = self.ctx.actor_id().to_owned();
						self.ctx.wait_until(async move {
							match tracked_reply_rx.await {
								Ok(result) => {
									tracing::info!(
										actor_id = %actor_id,
										action_name = %action_name_for_log,
										ok = result.is_ok(),
										"actor task: tracked reply received, forwarding"
									);
									let _ = reply.send(result);
								}
								Err(_) => {
									tracing::warn!(
										actor_id = %actor_id,
										action_name = %action_name_for_log,
										"actor task: tracked reply dropped before completion"
									);
									let _ =
										reply.send(Err(ActorLifecycleError::DroppedReply.build()));
								}
							}
						});
					}
					Err(error) => {
						tracing::warn!(
							actor_id = %self.ctx.actor_id(),
							action_name = %action_name_for_log,
							?error,
							"actor task: failed to enqueue ActorEvent::Action"
						);
						let _ = reply.send(Err(error));
						self.log_dispatch_command_handled(command_kind, "enqueue_failed");
					}
				}
			}
			DispatchCommand::QueueSend {
				name,
				body,
				conn,
				request,
				wait,
				timeout_ms,
				reply,
			} => match self.send_actor_event(
				"dispatch_queue_send",
				ActorEvent::QueueSend {
					name,
					body,
					conn,
					request,
					wait,
					timeout_ms,
					reply: Reply::from(reply),
				},
			) {
				Ok(()) => {
					self.log_dispatch_command_handled(command_kind, "enqueued");
				}
				Err(_error) => {
					self.log_dispatch_command_handled(command_kind, "enqueue_failed");
				}
			},
			DispatchCommand::Http { request, reply } => {
				match self.send_actor_event(
					"dispatch_http",
					ActorEvent::HttpRequest {
						request,
						reply: Reply::from(reply),
					},
				) {
					Ok(()) => {
						self.log_dispatch_command_handled(command_kind, "enqueued");
					}
					Err(_error) => {
						self.log_dispatch_command_handled(command_kind, "enqueue_failed");
					}
				}
			}
			DispatchCommand::OpenWebSocket { ws, request, reply } => {
				match self.send_actor_event(
					"dispatch_websocket_open",
					ActorEvent::WebSocketOpen {
						ws,
						request,
						reply: Reply::from(reply),
					},
				) {
					Ok(()) => {
						self.log_dispatch_command_handled(command_kind, "enqueued");
					}
					Err(_error) => {
						self.log_dispatch_command_handled(command_kind, "enqueue_failed");
					}
				}
			}
			DispatchCommand::WorkflowHistory { reply } => {
				match self.send_actor_event(
					"dispatch_workflow_history",
					ActorEvent::WorkflowHistoryRequested {
						reply: Reply::from(reply),
					},
				) {
					Ok(()) => {
						self.log_dispatch_command_handled(command_kind, "enqueued");
					}
					Err(_error) => {
						self.log_dispatch_command_handled(command_kind, "enqueue_failed");
					}
				}
			}
			DispatchCommand::WorkflowReplay { entry_id, reply } => {
				match self.send_actor_event(
					"dispatch_workflow_replay",
					ActorEvent::WorkflowReplayRequested {
						entry_id,
						reply: Reply::from(reply),
					},
				) {
					Ok(()) => {
						self.log_dispatch_command_handled(command_kind, "enqueued");
					}
					Err(_error) => {
						self.log_dispatch_command_handled(command_kind, "enqueue_failed");
					}
				}
			}
		}
	}

	fn log_dispatch_command_handled(&self, command: &'static str, outcome: &'static str) {
		tracing::debug!(
			actor_id = %self.ctx.actor_id(),
			command,
			outcome,
			"actor dispatch command handled"
		);
	}

	fn send_actor_event(&self, operation: &'static str, event: ActorEvent) -> Result<()> {
		let sender = self
			.actor_event_tx
			.as_ref()
			.ok_or_else(|| ActorLifecycleError::NotReady.build())?;
		tracing::debug!(
			actor_id = %self.ctx.actor_id(),
			operation,
			event = event.kind(),
			"actor event enqueued"
		);
		sender
			.send(event)
			.map_err(|_| ActorLifecycleError::NotReady.build())
	}

	fn reply_dispatch_error(&self, command: DispatchCommand, error: anyhow::Error) {
		match command {
			DispatchCommand::Action { reply, .. } => {
				let _ = reply.send(Err(error));
			}
			DispatchCommand::QueueSend { reply, .. } => {
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
			LifecycleState::Started => None,
			LifecycleState::SleepGrace
			| LifecycleState::SleepFinalize
			| LifecycleState::DestroyGrace => {
				self.ctx.warn_work_sent_to_stopping_instance("dispatch");
				Some(ActorLifecycleError::Stopping.build())
			}
			LifecycleState::Destroying | LifecycleState::Terminated => {
				self.ctx.warn_work_sent_to_stopping_instance("dispatch");
				Some(ActorLifecycleError::Destroying.build())
			}
			LifecycleState::Loading => {
				self.ctx.warn_self_call_risk("dispatch");
				Some(ActorLifecycleError::NotReady.build())
			}
		}
	}

	async fn start_actor(&mut self) -> Result<()> {
		let startup_started_at = Instant::now();
		let actor_id = self.ctx.actor_id().to_owned();
		if !self.ctx.started() {
			self.ctx.configure_sleep(self.factory.config().clone());
			self.ctx
				.configure_connection_runtime(self.factory.config().clone());
		}
		self.ensure_actor_event_channel();
		self.ctx.configure_actor_events(self.actor_event_tx.clone());
		self.ctx.configure_queue_preload(self.preloaded_kv.clone());

		let load_state_started_at = Instant::now();
		let persisted = self.load_persisted_startup().await?;
		tracing::debug!(
			actor_id = %actor_id,
			duration_ms = duration_ms_f64(load_state_started_at.elapsed()),
			"perf internal: loadStateMs"
		);
		let is_new = !persisted.actor.has_initialized;
		self.ctx.load_persisted_actor(persisted.actor);
		self.ctx.load_last_pushed_alarm(persisted.last_pushed_alarm);
		self.ctx.set_has_initialized(true);
		self.ctx
			.persist_state(SaveStateOpts { immediate: true })
			.await
			.context("persist actor initialization")?;
		let init_inspector_token_started_at = Instant::now();
		crate::inspector::init_inspector_token(&self.ctx)
			.await
			.context("initialize inspector token")?;
		tracing::debug!(
			actor_id = %actor_id,
			duration_ms = duration_ms_f64(init_inspector_token_started_at.elapsed()),
			"perf internal: initInspectorTokenMs"
		);
		self.ctx
			.restore_hibernatable_connections_with_preload(self.preloaded_kv.as_ref())
			.await
			.context("restore hibernatable connections")?;
		Self::settle_hibernated_connections(self.ctx.clone())
			.await
			.context("settle hibernated connections")?;
		self.ctx.init_alarms();

		self.transition_to(LifecycleState::Started);
		self.spawn_run_handle(is_new).await?;
		self.reset_sleep_deadline().await;
		self.ctx.drain_overdue_scheduled_events().await?;
		tracing::debug!(
			actor_id = %actor_id,
			duration_ms = duration_ms_f64(startup_started_at.elapsed()),
			is_new,
			"perf internal: startupTotalMs"
		);
		Ok(())
	}

	async fn load_persisted_startup(&mut self) -> Result<PersistedStartup> {
		match std::mem::take(&mut self.preload_persisted_actor) {
			PreloadedPersistedActor::Some(preloaded) => {
				return Ok(PersistedStartup {
					actor: preloaded,
					last_pushed_alarm: Self::load_last_pushed_alarm(self.ctx.kv().clone()).await?,
				});
			}
			PreloadedPersistedActor::BundleExistsButEmpty => {
				return Ok(PersistedStartup {
					actor: PersistedActor {
						input: self.start_input.clone(),
						..PersistedActor::default()
					},
					last_pushed_alarm: None,
				});
			}
			PreloadedPersistedActor::NoBundle => {}
		}

		let mut values = self
			.ctx
			.kv()
			.batch_get(&[PERSIST_DATA_KEY, LAST_PUSHED_ALARM_KEY])
			.await
			.context("load persisted actor startup data")?
			.into_iter();
		let actor = match values.next().flatten() {
			Some(bytes) => {
				decode_persisted_actor(&bytes).context("decode persisted actor startup data")
			}
			None => Ok(PersistedActor {
				input: self.start_input.clone(),
				..PersistedActor::default()
			}),
		}?;
		let last_pushed_alarm = values
			.next()
			.flatten()
			.map(|bytes| decode_last_pushed_alarm(&bytes))
			.transpose()
			.context("decode persisted last pushed alarm")?
			.flatten();

		Ok(PersistedStartup {
			actor,
			last_pushed_alarm,
		})
	}

	async fn load_last_pushed_alarm(kv: crate::kv::Kv) -> Result<Option<i64>> {
		kv.get(LAST_PUSHED_ALARM_KEY)
			.await
			.context("load persisted last pushed alarm")?
			.map(|bytes| decode_last_pushed_alarm(&bytes))
			.transpose()
			.context("decode persisted last pushed alarm")
			.map(Option::flatten)
	}

	fn ensure_actor_event_channel(&mut self) {
		if self.actor_event_tx.is_some() && self.actor_event_rx.is_some() {
			return;
		}

		let (actor_event_tx, actor_event_rx) = mpsc::unbounded_channel();
		self.actor_event_tx = Some(actor_event_tx);
		self.actor_event_rx = Some(actor_event_rx);
	}

	async fn spawn_run_handle(&mut self, is_new: bool) -> Result<()> {
		if self.run_handle.is_some() {
			return Ok(());
		}

		let Some(actor_events) = self.actor_event_rx.take() else {
			return Ok(());
		};
		let requires_manual_startup_ready = self.factory.requires_manual_startup_ready();
		let (startup_ready_tx, startup_ready_rx) = if requires_manual_startup_ready {
			let (tx, rx) = oneshot::channel();
			(Some(tx), Some(rx))
		} else {
			(None, None)
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
			events: ActorEvents::new(self.ctx.actor_id().to_owned(), actor_events),
			startup_ready: startup_ready_tx,
		};
		let factory = self.factory.clone();
		self.run_handle = Some(tokio::spawn(
			async move {
				match AssertUnwindSafe(factory.start(start)).catch_unwind().await {
					Ok(result) => result,
					Err(_) => Err(ActorRuntime::Panicked {
						operation: "run handler".to_owned(),
					}
					.build()),
				}
			}
			.in_current_span(),
		));
		if let Some(startup_ready_rx) = startup_ready_rx {
			startup_ready_rx
				.await
				.context("receive runtime startup ready reply")?
				.context("runtime startup preamble")?;
		}
		Ok(())
	}

	async fn settle_hibernated_connections(ctx: ActorContext) -> Result<()> {
		let actor_id = ctx.actor_id().to_owned();
		let mut dead_conn_ids = Vec::new();
		for conn in ctx.conns().filter(|conn| conn.is_hibernatable()) {
			let hibernation = conn.hibernation();
			let Some(hibernation) = hibernation else {
				tracing::debug!(
					actor_id = %actor_id,
					conn_id = conn.id(),
					outcome = "dead_missing_hibernation_metadata",
					"hibernated connection settled"
				);
				dead_conn_ids.push(conn.id().to_owned());
				continue;
			};
			let is_live = ctx
				.hibernated_connection_is_live(&hibernation.gateway_id, &hibernation.request_id)?;
			if is_live {
				tracing::debug!(
					actor_id = %actor_id,
					conn_id = conn.id(),
					outcome = "live",
					"hibernated connection settled"
				);
				continue;
			}
			tracing::debug!(
				actor_id = %actor_id,
				conn_id = conn.id(),
				outcome = "dead_not_live",
				"hibernated connection settled"
			);
			dead_conn_ids.push(conn.id().to_owned());
		}

		for conn_id in dead_conn_ids {
			ctx.request_hibernation_transport_removal(conn_id.clone());
			ctx.remove_conn(&conn_id);
			tracing::debug!(
				actor_id = %actor_id,
				conn_id = %conn_id,
				"dead hibernated connection removed"
			);
		}

		Ok(())
	}

	async fn fire_due_alarms(&mut self) -> Result<()> {
		if !matches!(
			self.lifecycle,
			LifecycleState::Started | LifecycleState::SleepGrace | LifecycleState::DestroyGrace
		) {
			return Ok(());
		}

		self.ctx.drain_overdue_scheduled_events().await
	}

	fn handle_run_handle_outcome(
		&mut self,
		outcome: std::result::Result<Result<()>, JoinError>,
	) -> Option<LiveExit> {
		self.run_handle = None;
		let clean_exit = match outcome {
			Ok(Ok(())) => true,
			Ok(Err(error)) => {
				tracing::error!(?error, "actor run handler failed");
				false
			}
			Err(error) => {
				tracing::error!(?error, "actor run handler join failed");
				false
			}
		};

		if clean_exit && self.lifecycle == LifecycleState::Started {
			tracing::debug!(
				actor_id = %self.ctx.actor_id(),
				"actor run handler exited cleanly while awaiting engine stop"
			);
			return None;
		}

		if self.lifecycle == LifecycleState::Started {
			self.transition_to(LifecycleState::Terminated);
		}

		self.ctx.reset_sleep_timer();
		self.state_save_deadline = None;
		self.inspector_serialize_state_deadline = None;
		self.close_actor_event_channel();

		None
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

	fn start_grace(&mut self, reason: StopReason) {
		let grace_period = match reason {
			StopReason::Sleep => self.factory.config().effective_sleep_grace_period(),
			StopReason::Destroy => self.factory.config().effective_on_destroy_timeout(),
		};
		self.sleep_deadline = None;
		self.ctx.cancel_sleep_timer();
		self.ctx.cancel_abort_signal_for_sleep();
		self.sleep_grace = Some(SleepGraceState {
			deadline: Instant::now() + grace_period,
			reason,
		});
		self.ctx.reset_sleep_timer();
	}

	async fn sleep_grace_tick(deadline: Option<Instant>) {
		let Some(deadline) = deadline else {
			future::pending::<()>().await;
			return;
		};

		sleep_until(deadline).await;
	}

	async fn on_activity_signal(&mut self) -> Option<LiveExit> {
		match self.lifecycle {
			LifecycleState::Started => {
				self.reset_sleep_deadline().await;
				None
			}
			LifecycleState::SleepGrace | LifecycleState::DestroyGrace => self.try_finish_grace(),
			_ => None,
		}
	}

	fn try_finish_grace(&mut self) -> Option<LiveExit> {
		let Some(grace) = self.sleep_grace.as_ref() else {
			return None;
		};
		if self.ctx.can_finalize_sleep() {
			let reason = grace.reason;
			self.sleep_grace = None;
			return Some(LiveExit::Shutdown { reason });
		}
		None
	}

	async fn on_sleep_grace_deadline(&mut self) -> Option<LiveExit> {
		let Some(grace) = self.sleep_grace.take() else {
			return None;
		};
		if let Some(run_handle) = self.run_handle.as_mut() {
			run_handle.abort();
		}
		self.ctx.record_shutdown_timeout(grace.reason);
		tracing::warn!(
			reason = shutdown_reason_label(grace.reason),
			deadline_missed_by_ms = Instant::now()
				.saturating_duration_since(grace.deadline)
				.as_millis() as u64,
			"actor shutdown reached the grace deadline"
		);
		Some(LiveExit::Shutdown {
			reason: grace.reason,
		})
	}

	async fn join_aborted_run_handle(&mut self) {
		let Some(mut run_handle) = self.run_handle.take() else {
			return;
		};
		match (&mut run_handle).await {
			Ok(Ok(())) => {}
			Ok(Err(error)) => {
				tracing::error!(?error, "actor run handler failed during shutdown");
			}
			Err(error) => {
				if !error.is_cancelled() {
					tracing::error!(?error, "actor run handler join failed during shutdown");
				}
			}
		};
	}

	#[cfg(test)]
	async fn drain_tracked_work(
		&mut self,
		reason: StopReason,
		phase: &'static str,
		deadline: Instant,
	) -> bool {
		Self::drain_tracked_work_with_ctx(self.ctx.clone(), reason, phase, deadline).await
	}

	#[cfg(test)]
	async fn drain_tracked_work_with_ctx(
		ctx: ActorContext,
		reason: StopReason,
		phase: &'static str,
		deadline: Instant,
	) -> bool {
		let started_at = Instant::now();
		tokio::select! {
			result = ctx.wait_for_shutdown_tasks(deadline) => result,
			_ = tokio::time::sleep(LONG_SHUTDOWN_DRAIN_WARNING_THRESHOLD) => {
				if ctx.wait_for_shutdown_tasks(Instant::now()).await {
					true
				} else {
					tracing::warn!(
						actor_id = %ctx.actor_id(),
						reason = reason.as_metric_label(),
						phase,
						elapsed_ms = Instant::now().duration_since(started_at).as_millis() as u64,
						"actor shutdown drain is taking longer than expected"
					);
					ctx.wait_for_shutdown_tasks(deadline).await
				}
			}
		}
	}

	fn log_lifecycle_command_received(&self, command: &'static str, reason: Option<&'static str>) {
		tracing::debug!(
			actor_id = %self.ctx.actor_id(),
			command,
			reason,
			"actor lifecycle command received"
		);
	}

	fn reply_lifecycle_command(
		&self,
		command: &'static str,
		reason: Option<&'static str>,
		reply: oneshot::Sender<Result<()>>,
		result: Result<()>,
	) {
		let outcome = result_outcome(&result);
		let delivered = reply.send(result).is_ok();
		tracing::debug!(
			actor_id = %self.ctx.actor_id(),
			command,
			reason,
			outcome,
			delivered,
			"actor lifecycle command replied"
		);
	}

	fn register_shutdown_reply(
		&mut self,
		command: &'static str,
		reason: Option<&'static str>,
		reply: oneshot::Sender<Result<()>>,
	) {
		if self.shutdown_reply.is_some() {
			debug_assert!(false, "engine actor2 sends one Stop per actor instance");
			tracing::warn!(
				actor_id = %self.ctx.actor_id(),
				command,
				reason,
				"duplicate Stop after shutdown reply was registered, dropping new reply"
			);
			return;
		}
		self.shutdown_reply = Some(PendingLifecycleReply {
			command,
			reason,
			reply,
		});
	}

	fn deliver_shutdown_reply(&mut self, reason: StopReason, result: &Result<()>) {
		#[cfg(test)]
		run_shutdown_reply_hook(&self.ctx, reason);

		let Some(pending) = self.shutdown_reply.take() else {
			return;
		};
		let outcome = result_outcome(result);
		let delivered = pending.reply.send(clone_shutdown_result(result)).is_ok();
		tracing::debug!(
			actor_id = %self.ctx.actor_id(),
			command = pending.command,
			reason = pending.reason,
			shutdown_reason = shutdown_reason_label(reason),
			outcome,
			delivered,
			"actor lifecycle command replied"
		);
	}

	async fn run_shutdown(&mut self, reason: StopReason) -> Result<()> {
		self.sleep_grace = None;
		let started_at = Instant::now();
		self.state_save_deadline = None;
		self.inspector_serialize_state_deadline = None;
		self.sleep_deadline = None;
		self.transition_to(match reason {
			StopReason::Sleep => LifecycleState::SleepFinalize,
			StopReason::Destroy => LifecycleState::Destroying,
		});
		self.save_final_state().await?;
		self.close_actor_event_channel();
		self.join_aborted_run_handle().await;
		Self::finish_shutdown_cleanup_with_ctx(self.ctx.clone(), reason).await?;
		if matches!(reason, StopReason::Destroy) {
			self.ctx.mark_destroy_completed();
		}
		self.ctx.record_shutdown_wait(reason, started_at.elapsed());
		Ok(())
	}

	async fn save_final_state(&mut self) -> Result<()> {
		let (reply_tx, reply_rx) = oneshot::channel();
		if let Err(error) = self.send_actor_event(
			"shutdown_serialize_state",
			ActorEvent::SerializeState {
				reason: SerializeStateReason::Save,
				reply: Reply::from(reply_tx),
			},
		) {
			tracing::error!(?error, "shutdown serialize-state enqueue failed");
			return self.ctx.save_state(Vec::new()).await;
		}

		let deltas = match timeout(SERIALIZE_STATE_SHUTDOWN_SANITY_CAP, reply_rx).await {
			Ok(Ok(Ok(deltas))) => deltas,
			Ok(Ok(Err(error))) => {
				tracing::error!(?error, "serializeState callback returned error");
				Vec::new()
			}
			Ok(Err(error)) => {
				tracing::error!(?error, "serializeState reply dropped");
				Vec::new()
			}
			Err(_) => {
				tracing::error!("serializeState timed out");
				Vec::new()
			}
		};

		self.ctx.save_state(deltas).await
	}

	async fn finish_shutdown_cleanup_with_ctx(ctx: ActorContext, reason: StopReason) -> Result<()> {
		let reason_label = shutdown_reason_label(reason);
		let actor_id = ctx.actor_id().to_owned();
		ctx.teardown_sleep_state().await;
		tracing::debug!(
			actor_id = %actor_id,
			reason = reason_label,
			step = "teardown_sleep_state",
			"actor shutdown cleanup step completed"
		);
		#[cfg(test)]
		run_shutdown_cleanup_hook(&ctx, reason_label);
		ctx.wait_for_pending_state_writes().await;
		tracing::debug!(
			actor_id = %actor_id,
			reason = reason_label,
			step = "wait_for_pending_state_writes",
			"actor shutdown cleanup step completed"
		);
		ctx.sync_alarm_logged();
		tracing::debug!(
			actor_id = %actor_id,
			reason = reason_label,
			step = "sync_alarm",
			"actor shutdown cleanup step completed"
		);
		ctx.wait_for_pending_alarm_writes().await;
		tracing::debug!(
			actor_id = %actor_id,
			reason = reason_label,
			step = "wait_for_pending_alarm_writes",
			"actor shutdown cleanup step completed"
		);
		ctx.sql()
			.cleanup()
			.await
			.with_context(|| format!("cleanup sqlite during {reason_label} shutdown"))?;
		tracing::debug!(
			actor_id = %actor_id,
			reason = reason_label,
			step = "cleanup_sqlite",
			"actor shutdown cleanup step completed"
		);
		match reason {
			// Match the reference TS runtime: keep the persisted engine alarm armed
			// across sleep so the next instance still has a wake trigger, but abort
			// the local Tokio timer owned by the shutting-down instance.
			StopReason::Sleep => {
				ctx.cancel_local_alarm_timeouts();
				tracing::debug!(
					actor_id = %actor_id,
					reason = reason_label,
					step = "cancel_local_alarm_timeouts",
					"actor shutdown cleanup step completed"
				);
			}
			StopReason::Destroy => {
				ctx.cancel_driver_alarm_logged();
				tracing::debug!(
					actor_id = %actor_id,
					reason = reason_label,
					step = "cancel_driver_alarm",
					"actor shutdown cleanup step completed"
				);
			}
		}
		Ok(())
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
			LifecycleState::Started | LifecycleState::SleepGrace | LifecycleState::DestroyGrace
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
		match self.send_actor_event(
			"save_tick",
			ActorEvent::SerializeState {
				reason: SerializeStateReason::Save,
				reply: Reply::from(reply_tx),
			},
		) {
			Ok(()) => {}
			Err(error) => {
				tracing::warn!(?error, "failed to enqueue save tick");
				self.schedule_state_save(true);
				return;
			}
		}

		match reply_rx.await {
			Ok(Ok(deltas)) => {
				let serialized_bytes = state_delta_payload_bytes(&deltas);
				tracing::debug!(
					actor_id = %self.ctx.actor_id(),
					reason = SerializeStateReason::Save.label(),
					delta_count = deltas.len(),
					serialized_bytes,
					save_request_revision,
					"actor serializeState completed"
				);
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
		) || self.inspector_attach_count.load(Ordering::SeqCst) == 0
			|| !self.ctx.save_requested()
		{
			return;
		}

		let (reply_tx, reply_rx) = oneshot::channel();
		match self.send_actor_event(
			"inspector_serialize_state",
			ActorEvent::SerializeState {
				reason: SerializeStateReason::Inspector,
				reply: Reply::from(reply_tx),
			},
		) {
			Ok(()) => {}
			Err(error) => {
				tracing::warn!(?error, "failed to enqueue inspector serialize tick");
				self.sync_inspector_serialize_deadline();
				return;
			}
		}

		match reply_rx.await {
			Ok(Ok(deltas)) => {
				tracing::debug!(
					actor_id = %self.ctx.actor_id(),
					reason = SerializeStateReason::Inspector.label(),
					delta_count = deltas.len(),
					serialized_bytes = state_delta_payload_bytes(&deltas),
					"actor serializeState completed"
				);
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

		let can_sleep = self.ctx.can_sleep().await;
		if can_sleep == crate::actor::sleep::CanSleep::Yes {
			tracing::debug!(
				actor_id = %self.ctx.actor_id(),
				sleep_timeout_ms = self.factory.config().sleep_timeout.as_millis() as u64,
				"sleep idle deadline elapsed"
			);
			self.ctx.sleep();
		} else {
			tracing::warn!(
				actor_id = %self.ctx.actor_id(),
				reason = ?can_sleep,
				"sleep idle deadline elapsed but actor stayed awake"
			);
			self.reset_sleep_deadline().await;
		}
	}

	async fn reset_sleep_deadline(&mut self) {
		if self.lifecycle != LifecycleState::Started {
			self.sleep_deadline = None;
			tracing::debug!(
				actor_id = %self.ctx.actor_id(),
				lifecycle = ?self.lifecycle,
				"sleep activity reset skipped outside started state"
			);
			return;
		}

		let can_sleep = self.ctx.can_sleep().await;
		if can_sleep == crate::actor::sleep::CanSleep::Yes {
			let deadline = Instant::now() + self.factory.config().sleep_timeout;
			self.sleep_deadline = Some(deadline);
			tracing::debug!(
				actor_id = %self.ctx.actor_id(),
				sleep_timeout_ms = self.factory.config().sleep_timeout.as_millis() as u64,
				"sleep activity reset"
			);
		} else {
			self.sleep_deadline = None;
			tracing::debug!(
				actor_id = %self.ctx.actor_id(),
				reason = ?can_sleep,
				"sleep activity reset skipped"
			);
		}
	}

	fn sync_inspector_serialize_deadline(&mut self) {
		if !matches!(
			self.lifecycle,
			LifecycleState::Started | LifecycleState::SleepGrace
		) || self.inspector_attach_count.load(Ordering::SeqCst) == 0
			|| !self.ctx.save_requested()
		{
			self.inspector_serialize_state_deadline = None;
			return;
		}

		self.inspector_serialize_state_deadline
			.get_or_insert_with(|| Instant::now() + INSPECTOR_SERIALIZE_STATE_INTERVAL);
	}

	fn broadcast_inspector_overlay(&self, deltas: &[StateDelta]) {
		if self.inspector_attach_count.load(Ordering::SeqCst) == 0 || deltas.is_empty() {
			return;
		}

		let mut payload = Vec::new();
		if let Err(error) = ciborium::into_writer(deltas, &mut payload) {
			tracing::error!(?error, "failed to encode inspector overlay deltas");
			return;
		}

		let payload = Arc::new(payload);
		let payload_bytes = payload.len();
		match self.inspector_overlay_tx.send(payload) {
			Ok(receiver_count) => {
				tracing::debug!(
					actor_id = %self.ctx.actor_id(),
					delta_count = deltas.len(),
					payload_bytes,
					receiver_count,
					"inspector overlay broadcast"
				);
			}
			Err(error) => {
				tracing::debug!(
					actor_id = %self.ctx.actor_id(),
					delta_count = deltas.len(),
					payload_bytes,
					error = ?error,
					"inspector overlay broadcast dropped"
				);
			}
		}
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
		let old = self.lifecycle;
		tracing::info!(
			actor_id = %self.ctx.actor_id(),
			old = ?old,
			new = ?lifecycle,
			"actor lifecycle transition"
		);
		self.lifecycle = lifecycle;
		match lifecycle {
			LifecycleState::Started => self.ctx.set_ready(true),
			LifecycleState::Loading
			| LifecycleState::SleepGrace
			| LifecycleState::SleepFinalize
			| LifecycleState::DestroyGrace
			| LifecycleState::Destroying
			| LifecycleState::Terminated => self.ctx.set_ready(false),
		}

		self.ctx
			.set_started(matches!(lifecycle, LifecycleState::Started));
	}
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
		Err(error) => {
			let error = rivet_error::RivetError::extract(error);
			Err(anyhow::Error::new(error))
		}
	}
}

fn result_outcome<T>(result: &Result<T>) -> &'static str {
	match result {
		Ok(_) => "ok",
		Err(_) => "error",
	}
}

fn state_delta_payload_bytes(deltas: &[StateDelta]) -> usize {
	deltas.iter().map(StateDelta::payload_len).sum()
}

fn duration_ms_f64(duration: Duration) -> f64 {
	duration.as_secs_f64() * 1000.0
}
