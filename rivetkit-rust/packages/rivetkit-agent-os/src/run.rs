//! Actor run loop. Brings up an `AgentOs` VM lazily on the first action
//! that needs it, tears it down on `Sleep` / `Destroy`, dispatches
//! actions through [`actions::dispatch`].

use std::sync::Arc;

use agent_os_client::{AgentOs, CronEvent};
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use rivetkit::{Ctx, Event, Start};
use serde::Serialize;

use crate::actions;
use crate::actor::AgentOsActor;
use crate::config::AgentOsActorConfig;

/// Empty payload type for the `vmBooted` broadcast.
#[derive(Serialize)]
struct VmBootedPayload {}

/// Payload for the `vmShutdown` broadcast. `reason` matches the TS actor:
/// `"sleep"`, `"destroy"`, or `"error"`.
#[derive(Serialize)]
struct VmShutdownPayload<'a> {
	reason: &'a str,
}

/// Run-loop entry function. Brings up the VM lazily on first event-driven
/// need and tears it down on `Sleep` / `Destroy`.
pub async fn run(
	config: Arc<AgentOsActorConfig>,
	mut start: Start<AgentOsActor>,
) -> Result<()> {
	let mut vm: Option<AgentOs> = None;

	while let Some(event) = start.events.recv().await {
		match event {
			Event::Action(action) => {
				if let Err(error) = ensure_vm(&start.ctx, &config, &mut vm).await {
					action.err(error);
					continue;
				}
				let handle = vm.as_ref().expect("vm present after ensure_vm");
				actions::dispatch(handle, &start.ctx, action).await;
			}
			Event::Http(http) => http.reply_status(404),
			Event::QueueSend(queue) => queue.err(anyhow!("queue send not supported")),
			Event::WebSocketOpen(ws) => ws.reject(anyhow!("websocket not supported")),
			Event::ConnOpen(conn) => conn.accept(()),
			Event::ConnClosed(_) => {}
			Event::Subscribe(subscribe) => subscribe.allow(),
			Event::SerializeState(serialize) => serialize.skip(),
			Event::Sleep(sleep) => {
				shutdown_vm(&start.ctx, &mut vm, "sleep").await;
				sleep.ok();
			}
			Event::Destroy(destroy) => {
				shutdown_vm(&start.ctx, &mut vm, "destroy").await;
				destroy.ok();
			}
			Event::WorkflowHistory(history) => history.reply_raw(None),
			Event::WorkflowReplay(replay) => replay.reply_raw(None),
		}
	}

	// Channel closed: best-effort cleanup if the run loop terminates while
	// a VM is still up.
	shutdown_vm(&start.ctx, &mut vm, "error").await;

	Ok(())
}

/// Bring up the VM if not already running. Broadcasts `vmBooted` on
/// first success.
async fn ensure_vm(
	ctx: &Ctx<AgentOsActor>,
	config: &Arc<AgentOsActorConfig>,
	vm: &mut Option<AgentOs>,
) -> Result<()> {
	if vm.is_some() {
		return Ok(());
	}
	let options = config.build_options();
	let handle = AgentOs::create(options)
		.await
		.map_err(|error| anyhow!("agent-os vm bring-up failed: {error}"))?;
	let receiver = handle.cron_events();
	*vm = Some(handle);
	// Broadcast payloads are wrapped in a 1-tuple so the rivetkit
	// client's TS-side `listener(...args)` spread receives the payload
	// as the first (and only) callback argument. Without the tuple
	// wrap, CBOR encodes the payload as a raw object and the spread
	// fails because objects aren't iterable.
	ctx.broadcast("vmBooted", &(VmBootedPayload {},))?;
	spawn_cron_event_forwarder(ctx, receiver);
	Ok(())
}

/// Subscribe to the VM's cron event broadcast channel and forward each
/// `Fire` / `Complete` / `Error` as a `cronEvent` actor broadcast. The
/// forwarder self-terminates when the underlying channel closes
/// (VM shutdown).
fn spawn_cron_event_forwarder(
	ctx: &Ctx<AgentOsActor>,
	mut receiver: tokio::sync::broadcast::Receiver<CronEvent>,
) {
	let ctx = ctx.clone();
	tokio::spawn(async move {
		loop {
			match receiver.recv().await {
				Ok(event) => {
					let payload = CronEventPayload::from(event);
					if let Err(error) = ctx.broadcast("cronEvent", &(payload,)) {
						tracing::warn!(?error, "cronEvent broadcast failed");
					}
				}
				Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
				Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
			}
		}
	});
}

/// Serializable shape for the `cronEvent` broadcast.
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum CronEventPayload {
	Fire {
		#[serde(rename = "jobId")]
		job_id: String,
		time: DateTime<Utc>,
	},
	Complete {
		#[serde(rename = "jobId")]
		job_id: String,
		time: DateTime<Utc>,
		#[serde(rename = "durationMs")]
		duration_ms: f64,
	},
	Error {
		#[serde(rename = "jobId")]
		job_id: String,
		time: DateTime<Utc>,
		error: String,
	},
}

impl From<CronEvent> for CronEventPayload {
	fn from(value: CronEvent) -> Self {
		match value {
			CronEvent::Fire { job_id, time } => Self::Fire { job_id, time },
			CronEvent::Complete {
				job_id,
				time,
				duration_ms,
			} => Self::Complete {
				job_id,
				time,
				duration_ms,
			},
			CronEvent::Error {
				job_id,
				time,
				error,
			} => Self::Error {
				job_id,
				time,
				error,
			},
		}
	}
}

/// Tear down the VM if running. Broadcasts `vmShutdown` after
/// `AgentOs::shutdown` completes (best-effort: shutdown errors are
/// logged but don't suppress the broadcast).
async fn shutdown_vm(ctx: &Ctx<AgentOsActor>, vm: &mut Option<AgentOs>, reason: &str) {
	let Some(handle) = vm.take() else {
		return;
	};
	if let Err(error) = handle.shutdown().await {
		tracing::warn!(?error, reason, "agent-os vm shutdown error");
	}
	if let Err(error) = ctx.broadcast("vmShutdown", &(VmShutdownPayload { reason },)) {
		tracing::warn!(?error, reason, "vmShutdown broadcast failed");
	}
}
