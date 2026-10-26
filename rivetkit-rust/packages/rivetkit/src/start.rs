use std::any::{Any, TypeId};
use std::future::Future;
use std::io::Cursor;
use std::marker::PhantomData;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use futures::FutureExt;
use rivet_error::RivetError;
use rivetkit_core::actor::ShutdownKind;
use rivetkit_core::error::{ActorLifecycle, ActorRuntime, action_not_found};
use rivetkit_core::{ActorEvent, ActorEvents, ActorStart, QueueSendResult, QueueSendStatus, Reply};
use serde::de::DeserializeOwned;
use tokio::{sync::OwnedSemaphorePermit, task::JoinHandle};
use tokio_util::sync::CancellationToken;

use crate::{
	action::ActionSet,
	actor::{Actor, HttpCallbackAdmission, HttpCallbackClass},
	context::{ConnCtx, Ctx},
	event::RuntimeEvent,
	queue::QueueSet,
};

#[derive(Debug)]
pub struct Start<A: Actor> {
	pub ctx: Ctx<A>,
	pub input: Input<A>,
	pub is_new: bool,
	pub snapshot: Snapshot,
	pub hibernated: Vec<Hibernated<A>>,
	pub events: Events<A>,
	#[doc(hidden)]
	pub startup_ready: Option<tokio::sync::oneshot::Sender<Result<()>>>,
}

#[derive(Debug)]
pub struct Input<A: Actor> {
	bytes: Option<Vec<u8>>,
	_p: PhantomData<fn() -> A>,
}

impl<A: Actor> Input<A> {
	pub fn is_present(&self) -> bool {
		self.bytes.is_some()
	}

	pub fn decode(&self) -> Result<A::Input> {
		match self.bytes.as_deref() {
			Some(bytes) => decode_cbor(bytes, "actor input"),
			None if TypeId::of::<A::Input>() == TypeId::of::<()>() => {
				let unit: Box<dyn Any> = Box::new(());
				Ok(*unit
					.downcast::<A::Input>()
					.expect("unit input type id should downcast"))
			}
			None => Err(ActorRuntime::MissingInput.build()),
		}
	}

	pub fn decode_or<F>(&self, f: F) -> Result<A::Input>
	where
		F: FnOnce() -> A::Input,
	{
		match self.bytes.as_deref() {
			Some(bytes) => decode_cbor(bytes, "actor input"),
			None => Ok(f()),
		}
	}

	pub fn decode_or_default(&self) -> Result<A::Input>
	where
		A::Input: Default,
	{
		self.decode_or(A::Input::default)
	}

	pub fn raw(&self) -> Option<&[u8]> {
		self.bytes.as_deref()
	}
}

#[derive(Debug)]
pub struct Snapshot {
	is_new: bool,
	bytes: Option<Vec<u8>>,
}

impl Snapshot {
	pub fn is_new(&self) -> bool {
		self.is_new
	}

	pub fn decode<S>(&self) -> Result<Option<S>>
	where
		S: DeserializeOwned,
	{
		let Some(bytes) = self.bytes.as_deref().filter(|bytes| !bytes.is_empty()) else {
			return Ok(None);
		};
		decode_cbor(bytes, "actor snapshot").map(Some)
	}

	pub fn decode_or_default<S>(&self) -> Result<S>
	where
		S: DeserializeOwned + Default,
	{
		Ok(self.decode()?.unwrap_or_default())
	}

	pub fn raw(&self) -> Option<&[u8]> {
		self.bytes.as_deref()
	}
}

#[derive(Debug)]
pub struct Hibernated<A: Actor> {
	pub conn: ConnCtx<A>,
}

#[derive(Debug)]
pub struct Events<A: Actor> {
	ctx: Ctx<A>,
	rx: ActorEvents,
	_p: PhantomData<fn() -> A>,
}

impl<A: Actor> Events<A> {
	pub(crate) async fn recv_raw(&mut self) -> Option<ActorEvent> {
		self.rx.recv().await
	}

	pub async fn recv(&mut self) -> Option<RuntimeEvent<A>> {
		loop {
			let event = self.rx.recv().await?;
			if let Some(event) = self.handle_runtime_event(event).await {
				return Some(wrap_event(event));
			}
		}
	}

	pub fn try_recv(&mut self) -> Option<RuntimeEvent<A>> {
		while let Some(event) = self.rx.try_recv() {
			if let Some(event) = self.handle_runtime_event_sync(event) {
				return Some(wrap_event(event));
			}
		}
		None
	}

	async fn handle_runtime_event(&self, event: ActorEvent) -> Option<ActorEvent> {
		match event {
			ActorEvent::ConnectionOpen { reply, .. } => {
				reply.send(Ok(()));
				None
			}
			ActorEvent::DisconnectConn { conn_id, reply } => {
				reply.send(self.ctx.disconnect_conn(&conn_id).await);
				None
			}
			ActorEvent::RunWake { reply, .. } => {
				reply.send(Ok(()));
				None
			}
			event => Some(event),
		}
	}

	fn handle_runtime_event_sync(&self, event: ActorEvent) -> Option<ActorEvent> {
		match event {
			ActorEvent::ConnectionOpen { reply, .. } => {
				reply.send(Ok(()));
				None
			}
			ActorEvent::DisconnectConn { conn_id, reply } => {
				let ctx = self.ctx.clone();
				tokio::spawn(async move {
					reply.send(ctx.disconnect_conn(&conn_id).await);
				});
				None
			}
			ActorEvent::RunWake { reply, .. } => {
				reply.send(Ok(()));
				None
			}
			event => Some(event),
		}
	}
}

pub async fn run_actor<A: Actor>(start: Start<A>) -> Result<()> {
	let Start {
		ctx,
		input,
		is_new,
		snapshot,
		hibernated: _,
		mut events,
		startup_ready,
	} = start;

	// Run the whole startup phase (input decode, state creation, create,
	// on_create, on_start) as one fallible unit so a failure is forwarded to
	// the runtime handshake as the real cause instead of being dropped. Without
	// this, an input decode error would drop `startup_ready`, surfacing only a
	// generic closed-channel error rather than the actual failure. The failure
	// itself is logged by rivetkit-core when it drains the run handle.
	let startup = async {
		let state = match snapshot.decode()? {
			Some(state) => state,
			// Absent input falls back to the input type's default, matching
			// rivetkit-typescript where createState receives undefined input.
			None => A::create_state(&ctx, input.decode_or_default()?).await?,
		};
		ctx.set_initial_state(state);

		let actor = Arc::new(A::create(&ctx).await?);
		if is_new {
			actor.clone().on_create(ctx.clone()).await?;
		}
		actor.clone().on_start(ctx.clone()).await?;
		Ok::<_, anyhow::Error>(actor)
	}
	.await;

	let actor = match startup {
		Ok(actor) => {
			if let Some(reply) = startup_ready {
				let _ = reply.send(Ok(()));
			}
			actor
		}
		Err(error) => {
			if let Some(reply) = startup_ready {
				let _ = reply.send(Err(anyhow::Error::new(RivetError::extract(&error))));
			}
			return Err(error);
		}
	};

	let run_cancel = CancellationToken::new();
	let run_task = spawn_run_task(actor.clone(), ctx.clone(), run_cancel.clone());
	let http_pools = HttpCallbackPools::for_actor::<A>();

	while let Some(event) = events.recv_raw().await {
		let should_stop =
			handle_actor_event(actor.clone(), ctx.clone(), http_pools.as_ref(), event).await?;
		if should_stop {
			break;
		}
	}

	stop_run_task(&ctx, run_cancel, run_task).await
}

struct HttpCallbackPools {
	standard: Arc<tokio::sync::Semaphore>,
	live: Arc<tokio::sync::Semaphore>,
}

impl HttpCallbackPools {
	fn for_actor<A: Actor>() -> Option<Self> {
		A::CONCURRENT_HTTP_CALLBACKS.then(|| Self {
			standard: Arc::new(tokio::sync::Semaphore::new(
				A::MAX_CONCURRENT_HTTP_CALLBACKS,
			)),
			live: Arc::new(tokio::sync::Semaphore::new(
				A::MAX_CONCURRENT_LIVE_HTTP_CALLBACK_STARTS,
			)),
		})
	}

	fn try_acquire(&self, class: HttpCallbackClass) -> Option<OwnedSemaphorePermit> {
		match class {
			HttpCallbackClass::Standard => self.standard.clone().try_acquire_owned().ok(),
			HttpCallbackClass::Live => self.live.clone().try_acquire_owned().ok(),
		}
	}
}

struct HttpCallbackAdmissionGuard(Option<Box<dyn FnOnce() + Send + 'static>>);

impl HttpCallbackAdmissionGuard {
	fn new(release: Option<Box<dyn FnOnce() + Send + 'static>>) -> Self {
		Self(release)
	}
}

impl Drop for HttpCallbackAdmissionGuard {
	fn drop(&mut self) {
		if let Some(release) = self.0.take() {
			release();
		}
	}
}

fn spawn_run_task<A: Actor>(
	actor: Arc<A>,
	ctx: Ctx<A>,
	cancel: CancellationToken,
) -> JoinHandle<Result<()>> {
	tokio::spawn(async move {
		let result = tokio::select! {
			_ = cancel.cancelled() => return Ok(()),
			result = AssertUnwindSafe(actor.run(ctx.clone())).catch_unwind() => result,
		};
		let result = result.unwrap_or_else(|_| Err(anyhow::anyhow!("actor run handler panicked")));
		if let Err(error) = &result {
			report_run_error(&ctx, error);
		}
		result
	})
}

/// Reports a `run` failure to the engine as an errored stop. The event loop in
/// `run_actor` does not observe the run task until shutdown, so the report
/// must happen here for the engine to learn about the crash promptly. Errors
/// that surface after the abort signal fires are shutdown-induced (cancelled
/// waits during sleep or destroy) and are not crashes.
fn report_run_error<A: Actor>(ctx: &Ctx<A>, error: &anyhow::Error) {
	if ctx.abort_signal().is_cancelled() {
		tracing::debug!(?error, "actor run task failed during shutdown");
		return;
	}
	if let Err(stop_error) = ctx.stop_with_error(format!("{error:#}")) {
		if ctx.inner().is_destroy_requested() {
			tracing::debug!(
				?stop_error,
				run_error = ?error,
				"actor run failed while a stop was already requested"
			);
		} else {
			tracing::error!(
				?stop_error,
				run_error = ?error,
				"failed to report actor run error as errored stop"
			);
		}
	}
}

async fn stop_run_task<A: Actor>(
	ctx: &Ctx<A>,
	cancel: CancellationToken,
	mut task: JoinHandle<Result<()>>,
) -> Result<()> {
	ctx.inner().cancel_actor_abort_signal();
	match tokio::time::timeout(Duration::from_millis(100), &mut task).await {
		Ok(result) => result.context("join actor run task")?,
		Err(_) => {
			cancel.cancel();
			task.await.context("join actor run task")?
		}
	}
}

async fn handle_actor_event<A: Actor>(
	actor: Arc<A>,
	ctx: Ctx<A>,
	http_pools: Option<&HttpCallbackPools>,
	event: ActorEvent,
) -> Result<bool> {
	match event {
		ActorEvent::Action {
			name,
			args,
			conn,
			reply,
			..
		} => {
			let handler_ctx = ctx.with_conn(conn.map(ConnCtx::from));
			match <A::Actions as ActionSet<A>>::dispatch(
				actor,
				handler_ctx.clone(),
				name.as_str(),
				args.as_slice(),
			) {
				Some(future) => {
					spawn_action_reply(handler_ctx, reply, future);
				}
				None => {
					reply.send(Err(action_not_found(name)));
				}
			}
		}
		ActorEvent::HttpRequest {
			request,
			invocation_telemetry: _,
			reply,
		} => {
			let ctx = ctx.with_request(Some(request.clone()));
			if let Some(http_pools) = http_pools {
				let class = A::classify_http_request(&request);
				let Some(permit) = http_pools.try_acquire(class) else {
					reply.send(
						rivetkit_core::Response::from_parts(429, Default::default(), Vec::new())
							.map(rivetkit_core::ActorHttpResponse::Buffered),
					);
					return Ok(false);
				};

				let admission = actor.admit_http_request(&ctx, &request);
				let release = match admission {
					HttpCallbackAdmission::Untracked => None,
					HttpCallbackAdmission::Tracked { release } => Some(release),
					HttpCallbackAdmission::Reject(error) => {
						reply.send(Err(error));
						return Ok(false);
					}
				};
				let future = actor.on_fetch_response(ctx.clone(), request);
				spawn_http_reply(ctx, reply, future, permit, release);
			} else {
				reply.send(actor.on_fetch_response(ctx, request).await);
			}
		}
		ActorEvent::QueueSend {
			name,
			body,
			conn,
			request,
			reply,
			..
		} => {
			let handler_ctx = ctx
				.with_request(Some(request))
				.with_conn(Some(ConnCtx::from(conn)));
			match <A::Queue as QueueSet<A>>::dispatch(
				actor,
				handler_ctx.clone(),
				name.as_str(),
				body.as_slice(),
			) {
				Some(future) => {
					spawn_queue_reply(handler_ctx, reply, future);
				}
				None => {
					reply.send(Err(ActorRuntime::NotFound {
						resource: "queue handler".to_owned(),
						id: name,
					}
					.build()));
				}
			}
		}
		ActorEvent::WebSocketOpen {
			ws, request, reply, ..
		} => {
			let ctx = ctx.with_request(request.clone());
			reply.send(
				actor
					.on_websocket(ctx, ws, request.unwrap_or_default())
					.await,
			);
		}
		ActorEvent::ConnectionPreflight {
			conn,
			params,
			request,
			reply,
			..
		} => {
			let ctx = ctx.with_request(request);
			let result = async {
				let params = decode_conn_params::<A>(&params)?;
				actor
					.clone()
					.on_before_connect(ctx.clone(), &params)
					.await?;
				let conn_state = actor.clone().create_conn_state(ctx.clone(), params).await?;
				let conn = ConnCtx::from(conn);
				conn.set_state(&conn_state)?;
				actor.on_connect(ctx, conn).await
			}
			.await;
			reply.send(result);
		}
		ActorEvent::ConnectionOpen { reply, .. } => {
			reply.send(Ok(()));
		}
		ActorEvent::ConnectionClosed { conn } => {
			actor.on_disconnect(ctx, ConnCtx::from(conn)).await;
		}
		ActorEvent::SubscribeRequest {
			conn,
			event_name,
			reply,
		} => {
			reply.send(
				actor
					.on_subscribe(ctx, ConnCtx::from(conn), event_name)
					.await,
			);
		}
		ActorEvent::SerializeState { reply, .. } => {
			let result = async {
				if ctx.state_dirty() {
					actor.on_state_change(ctx.clone()).await?;
				}
				let delta = ctx.encode_state_delta()?;
				ctx.clear_state_dirty();
				Ok(vec![delta])
			}
			.await;
			reply.send(result);
		}
		ActorEvent::RunGracefulCleanup { reason, reply } => {
			let result = match reason {
				ShutdownKind::Sleep => actor.on_sleep(ctx).await,
				ShutdownKind::Destroy => actor.on_destroy(ctx).await,
			};
			reply.send(result);
			// Keep the loop alive so the finalize-phase `SerializeState` can capture state the hook wrote; core ends it by closing the channel.
		}
		ActorEvent::DisconnectConn { conn_id, reply } => {
			reply.send(ctx.disconnect_conn(&conn_id).await);
		}
		ActorEvent::WorkflowHistoryRequested { reply } => {
			reply.send(Err(not_configured("workflow history")));
		}
		ActorEvent::WorkflowReplayRequested { reply, .. } => {
			reply.send(Err(not_configured("workflow replay")));
		}
		ActorEvent::RunWake { reply, .. } => {
			reply.send(Ok(()));
		}
	}

	Ok(false)
}

fn spawn_http_reply<A: Actor>(
	ctx: Ctx<A>,
	reply: Reply<rivetkit_core::ActorHttpResponse>,
	future: impl Future<Output = Result<rivetkit_core::ActorHttpResponse>> + Send + 'static,
	permit: OwnedSemaphorePermit,
	release: Option<Box<dyn FnOnce() + Send + 'static>>,
) {
	ctx.register_task(async move {
		let _permit = permit;
		let _admission = HttpCallbackAdmissionGuard::new(release);
		reply.send(future.await);
	});
}

fn spawn_action_reply<A: Actor>(
	ctx: Ctx<A>,
	reply: Reply<Vec<u8>>,
	future: crate::action::BoxActionFuture,
) {
	tokio::spawn(async move {
		let abort = ctx.abort_signal();
		tokio::select! {
			_ = abort.cancelled() => {
				reply.send(Err(ActorLifecycle::Stopping.build()));
			}
			result = future => {
				reply.send(result);
			}
		}
	});
}

fn spawn_queue_reply<A: Actor>(
	ctx: Ctx<A>,
	reply: Reply<QueueSendResult>,
	future: crate::queue::BoxQueueFuture,
) {
	tokio::spawn(async move {
		let abort = ctx.abort_signal();
		let result = tokio::select! {
			_ = abort.cancelled() => Err(ActorLifecycle::Stopping.build()),
			result = future => result.map(|response| QueueSendResult {
				status: QueueSendStatus::Completed,
				response,
			}),
		};
		reply.send(result);
	});
}

fn not_configured(component: impl Into<String>) -> anyhow::Error {
	ActorRuntime::NotConfigured {
		component: component.into(),
	}
	.build()
}

#[doc(hidden)]
pub fn wrap_start<A: Actor>(core_start: ActorStart) -> Result<Start<A>> {
	let ActorStart {
		ctx,
		input,
		is_new,
		snapshot,
		hibernated,
		events,
		startup_ready,
	} = core_start;

	let hibernated = hibernated
		.into_iter()
		.map(|(conn, bytes)| Hibernated {
			conn: ConnCtx::from({
				conn.set_state(bytes);
				conn
			}),
		})
		.collect();

	let ctx = Ctx::new(ctx);

	Ok(Start {
		ctx: ctx.clone(),
		input: Input {
			bytes: input,
			_p: PhantomData,
		},
		is_new,
		snapshot: Snapshot {
			is_new,
			bytes: snapshot,
		},
		hibernated,
		events: Events {
			ctx,
			rx: events,
			_p: PhantomData,
		},
		startup_ready,
	})
}

fn wrap_event<A: Actor>(event: ActorEvent) -> RuntimeEvent<A> {
	RuntimeEvent::from_core(event)
}

fn decode_cbor<T: DeserializeOwned>(bytes: &[u8], label: &str) -> Result<T> {
	ciborium::from_reader(Cursor::new(bytes)).with_context(|| format!("decode {label} from cbor"))
}

fn decode_conn_params<A: Actor>(bytes: &[u8]) -> Result<A::ConnParams> {
	if bytes.is_empty() || bytes == [0xf6] {
		return Ok(A::ConnParams::default());
	}
	decode_cbor(bytes, "connection params")
}


