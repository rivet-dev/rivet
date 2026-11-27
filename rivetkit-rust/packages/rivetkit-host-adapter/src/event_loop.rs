//! The per-actor event loop, ported from `rivetkit-napi/src/napi_actor_events.rs`
//! without actor state (deprecated in 3.0; durable data lives in SQLite).
//!
//! Start: `on_migrate` → `create_vars` → `on_create` (first start) → `on_fork`
//! (forked first start) → `on_wake`, each bounded by its timeout. This is the
//! Rust SDK's order: the host's per-start object exists before any user hook.
//!
//! Run: every event is handed to the host in arrival order (see the crate docs)
//! and its reply awaited on its own task, so a slow action never blocks the loop
//! or later events. Ordering and mutual exclusion of user code are the host's job.
//!
//! Stop: core cancels its abort signal (the host hears `on_abort`), disconnects
//! connections, sends `RunGracefulCleanup` (`on_sleep`/`on_destroy`, bounded by
//! the shutdown deadline), then closes the event channel. In-flight calls finish
//! inside the grace period; once the channel closes, anything still running is
//! answered with `actor/stopping`, and the host gets `released`.

use std::sync::Arc;

use anyhow::Result;
use rivet_error::RivetError;
use rivetkit_core::actor::ShutdownKind;
use rivetkit_core::{
	ActorConfig, ActorContext, ActorEvent, ActorFactory, ActorHttpResponse,
	HTTP_BODY_STREAM_CHANNEL_CAPACITY, Reply, Response, StreamingResponse,
};
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use crate::config::AdapterConfig;
use crate::errors::{self, bounded};
use crate::host::{ActionCall, HostActor, HostFuture, HostInstance, HttpReply};
use crate::http::ResponseBody;

/// Builds a core factory that runs `host` through this adapter.
pub fn actor_factory(
	config: ActorConfig,
	adapter: AdapterConfig,
	host: Arc<dyn HostActor>,
) -> ActorFactory {
	let adapter = Arc::new(adapter);
	let streaming_bodies = adapter.hooks.on_request;
	let factory = ActorFactory::new_with_manual_startup_ready(config, move |start| {
		let host = host.clone();
		let adapter = adapter.clone();
		Box::pin(run_actor(host, adapter, start))
	});
	if streaming_bodies {
		factory.with_streaming_request_body()
	} else {
		factory
	}
}

/// Tells the host the actor is gone however the loop exits.
struct ReleaseGuard(Arc<dyn HostInstance>);

impl Drop for ReleaseGuard {
	fn drop(&mut self) {
		self.0.released();
	}
}

pub async fn run_actor(
	host: Arc<dyn HostActor>,
	config: Arc<AdapterConfig>,
	start: rivetkit_core::ActorStart,
) -> Result<()> {
	let rivetkit_core::ActorStart {
		ctx,
		input,
		is_new,
		snapshot: _,
		hibernated: _,
		fork,
		mut events,
		startup_ready,
	} = start;

	let started = async {
		let instance = host.start(&ctx)?;
		let guard = ReleaseGuard(instance.clone());
		preamble(&config, &ctx, instance.as_ref(), input, is_new, fork).await?;
		Ok::<_, anyhow::Error>((instance, guard))
	}
	.await;
	let (instance, _guard) = match started {
		Ok(started) => {
			if let Some(reply) = startup_ready {
				let _ = reply.send(Ok(()));
			}
			started
		}
		Err(error) => {
			if let Some(reply) = startup_ready {
				let _ = reply.send(Err(anyhow::Error::new(RivetError::extract(&error))));
			}
			return Err(error);
		}
	};

	// Cancelled only after the event channel closes, so in-flight calls get the
	// whole grace period.
	let abort = CancellationToken::new();
	let mut tasks = JoinSet::new();
	if config.hooks.on_abort {
		let instance = instance.clone();
		let signal = ctx.actor_abort_signal();
		let abort = abort.clone();
		tasks.spawn(async move {
			tokio::select! {
				_ = signal.cancelled() => instance.on_abort(),
				_ = abort.cancelled() => {}
			}
		});
	}

	let loop_cx = LoopCx {
		config: &config,
		ctx: &ctx,
		instance: &instance,
		abort: &abort,
	};
	loop {
		tokio::select! {
			// Reap finished tasks; a JoinSet keeps each one until joined.
			Some(result) = tasks.join_next(), if !tasks.is_empty() => log_join(result),
			event = events.recv() => match event {
				Some(event) => loop_cx.dispatch(event, &mut tasks),
				None => break,
			},
		}
	}

	abort.cancel();
	while let Some(result) = tasks.join_next().await {
		log_join(result);
	}
	Ok(())
}

fn log_join(result: Result<(), tokio::task::JoinError>) {
	if let Err(error) = result
		&& !error.is_cancelled()
	{
		tracing::error!(?error, "host adapter task failed");
	}
}

async fn preamble(
	config: &AdapterConfig,
	ctx: &ActorContext,
	instance: &dyn HostInstance,
	input: Option<Vec<u8>>,
	is_new: bool,
	fork: Option<rivetkit_core::ActorForkStart>,
) -> Result<()> {
	let hooks = &config.hooks;
	if hooks.on_migrate {
		hook(
			ctx,
			"onMigrate",
			config.on_migrate_timeout,
			instance.on_migrate(is_new),
		)
		.await?;
	}
	hook(
		ctx,
		"createVars",
		config.create_vars_timeout,
		instance.create_vars(),
	)
	.await?;
	if is_new && hooks.on_create {
		hook(
			ctx,
			"onCreate",
			config.on_create_timeout,
			instance.on_create(input),
		)
		.await?;
	}
	// Core decides when a fork's hook is due: once, on the fork's first start,
	// before overdue schedules drain.
	if let Some(fork) = fork
		&& hooks.on_fork
	{
		hook(
			ctx,
			"onFork",
			config.on_create_timeout,
			instance.on_fork(fork),
		)
		.await?;
	}
	if hooks.on_wake {
		hook(ctx, "onWake", config.on_wake_timeout, instance.on_wake()).await?;
	}
	Ok(())
}

async fn hook(
	ctx: &ActorContext,
	name: &'static str,
	timeout: std::time::Duration,
	future: HostFuture<()>,
) -> Result<()> {
	ctx.time_runtime_callback(
		name,
		bounded(timeout, future, || {
			errors::callback_timed_out(name, timeout)
		}),
	)
	.await
}

struct LoopCx<'a> {
	config: &'a AdapterConfig,
	ctx: &'a ActorContext,
	instance: &'a Arc<dyn HostInstance>,
	abort: &'a CancellationToken,
}

impl LoopCx<'_> {
	/// Answers `reply` with `work`, or with `actor/stopping` if the actor finishes
	/// shutting down first.
	fn spawn_reply<T: Send + 'static>(
		&self,
		tasks: &mut JoinSet<()>,
		reply: Reply<T>,
		work: impl std::future::Future<Output = Result<T>> + Send + 'static,
	) {
		let abort = self.abort.clone();
		tasks.spawn(async move {
			tokio::select! {
				_ = abort.cancelled() => reply.send(Err(errors::stopping())),
				result = work => reply.send(result),
			}
		});
	}

	fn dispatch(&self, event: ActorEvent, tasks: &mut JoinSet<()>) {
		let hooks = &self.config.hooks;
		match event {
			ActorEvent::Action {
				name,
				args,
				conn,
				scheduled_fire,
				reply,
				..
			} => {
				let timeout = self.config.action_timeout;
				// Delivered now, in arrival order; the reply is awaited on a task.
				let future = self.instance.action(ActionCall {
					name,
					args,
					conn,
					scheduled: scheduled_fire.is_some(),
				});
				self.spawn_reply(tasks, reply, async move {
					bounded(timeout, future, || errors::action_timed_out(timeout)).await
				});
			}
			ActorEvent::HttpRequest { request, reply, .. } => {
				if !hooks.on_request {
					reply.send(Err(errors::not_configured("onRequest")));
					return;
				}
				// The writer exists before the host runs, so it can stream before
				// it replies.
				let (tx, rx) = mpsc::channel(HTTP_BODY_STREAM_CHANNEL_CAPACITY);
				let future = self.instance.on_request(request, ResponseBody::new(tx));
				let timeout = self.config.on_request_timeout;
				self.spawn_reply(tasks, reply, async move {
					let reply =
						bounded(timeout, future, || errors::action_timed_out(timeout)).await?;
					http_response(reply, rx)
				});
			}
			ActorEvent::QueueSend { name, reply, .. } => {
				reply.send(Err(errors::queue_handler_not_found(name)));
			}
			ActorEvent::WebSocketOpen {
				conn,
				ws,
				request,
				reply,
			} => {
				if !hooks.on_websocket {
					reply.send(Err(errors::not_configured("onWebSocket")));
					return;
				}
				let future = self.instance.on_websocket(&conn, ws, request.as_ref());
				let timeout = self.config.on_connect_timeout;
				self.spawn_reply(tasks, reply, async move {
					bounded(timeout, future, || {
						errors::callback_timed_out("onWebSocket", timeout)
					})
					.await
				});
			}
			ActorEvent::ConnectionPreflight {
				conn,
				params,
				request,
				reply,
			} => {
				if !hooks.on_before_connect {
					reply.send(Ok(()));
					return;
				}
				let future = self
					.instance
					.on_before_connect(&conn, params, request.as_ref());
				let timeout = self.config.on_before_connect_timeout;
				self.spawn_reply(tasks, reply, async move {
					bounded(timeout, future, || {
						errors::callback_timed_out("onBeforeConnect", timeout)
					})
					.await
				});
			}
			ActorEvent::ConnectionOpen {
				conn,
				request,
				reply,
			} => {
				if !hooks.on_connect {
					reply.send(Ok(()));
					return;
				}
				let future = self.instance.on_connect(&conn, request.as_ref());
				let timeout = self.config.on_connect_timeout;
				self.spawn_reply(tasks, reply, async move {
					bounded(timeout, future, || {
						errors::callback_timed_out("onConnect", timeout)
					})
					.await
				});
			}
			ActorEvent::ConnectionClosed { conn } => {
				if !hooks.on_disconnect {
					return;
				}
				let future = self.instance.on_disconnect(&conn);
				let timeout = self.config.on_connect_timeout;
				let ctx = self.ctx.clone();
				let abort = self.abort.clone();
				tasks.spawn(async move {
					// Keeps the actor awake until the callback returns.
					let work = ctx.with_disconnect_callback(|| {
						bounded(timeout, future, || {
							errors::callback_timed_out("onDisconnect", timeout)
						})
					});
					tokio::select! {
						_ = abort.cancelled() => {}
						result = work => if let Err(error) = result {
							tracing::error!(?error, "onDisconnect failed");
						},
					}
				});
			}
			ActorEvent::SubscribeRequest {
				conn,
				event_name,
				reply,
			} => {
				if !hooks.on_subscribe {
					reply.send(Ok(()));
					return;
				}
				let future = self.instance.on_subscribe(&conn, &event_name);
				let timeout = self.config.on_before_connect_timeout;
				self.spawn_reply(tasks, reply, async move {
					bounded(timeout, future, || {
						errors::callback_timed_out("onSubscribe", timeout)
					})
					.await
				});
			}
			// No framework-managed state: nothing to persist.
			ActorEvent::SerializeState { reply, .. } => reply.send(Ok(Vec::new())),
			ActorEvent::RunGracefulCleanup { reason, reply } => {
				let (name, wanted, future) = match reason {
					ShutdownKind::Sleep => ("onSleep", hooks.on_sleep, self.instance.on_sleep()),
					ShutdownKind::Destroy => {
						("onDestroy", hooks.on_destroy, self.instance.on_destroy())
					}
				};
				if !wanted {
					reply.send(Ok(()));
					return;
				}
				// Bounded by core's shutdown deadline so the final save still runs.
				let deadline = self.ctx.shutdown_deadline_token();
				tasks.spawn(async move {
					let result = tokio::select! {
						result = future => result,
						_ = deadline.cancelled() => Err(errors::cut_off_at_shutdown(name)),
					};
					if let Err(error) = result {
						tracing::error!(?error, callback = name, "shutdown hook failed");
					}
					reply.send(Ok(()));
				});
			}
			ActorEvent::DisconnectConn {
				conn_id,
				reason,
				reply,
			} => {
				let ctx = self.ctx.clone();
				let conn = ctx.conns().find(|conn| conn.id() == conn_id);
				let instance = hooks.on_disconnect.then(|| self.instance.clone());
				tasks.spawn(async move {
					if let Some(conn) = conn {
						// The transport closes before the callback runs, so a slow
						// callback cannot hide the reason from the client.
						if let Err(error) = ctx.disconnect_conn_with_reason(conn_id, reason).await {
							tracing::error!(?error, "disconnect failed");
						}
						if let Some(instance) = instance
							&& let Err(error) = instance.on_disconnect(&conn).await
						{
							tracing::error!(?error, "onDisconnect failed");
						}
					}
					reply.send(Ok(()));
				});
			}
			ActorEvent::WorkflowHistoryRequested { reply } => {
				reply.send(Err(errors::not_configured("workflow history")));
			}
			ActorEvent::WorkflowReplayRequested { reply, .. } => {
				reply.send(Err(errors::not_configured("workflow replay")));
			}
			// No `run` handler (deprecated): nothing to wake.
			ActorEvent::RunWake { reply, .. } => reply.send(Ok(())),
		}
	}
}

fn http_response(
	reply: HttpReply,
	rx: mpsc::Receiver<rivetkit_core::ResponseChunk>,
) -> Result<ActorHttpResponse> {
	if reply.streamed {
		StreamingResponse::from_parts(reply.status, reply.headers, rx)
			.map(ActorHttpResponse::Stream)
	} else {
		Response::from_parts(reply.status, reply.headers, reply.body)
			.map(ActorHttpResponse::Buffered)
	}
}
