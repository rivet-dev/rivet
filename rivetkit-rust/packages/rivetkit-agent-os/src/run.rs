//! Actor run loop. Brings up an `AgentOs` VM lazily on the first action
//! that needs it, tears it down on `Sleep` / `Destroy`, dispatches
//! actions through [`actions::dispatch`].

use std::sync::Arc;

use agent_os_client::AgentOs;
use anyhow::{Result, anyhow};
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
				actions::dispatch(handle, action).await;
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
	*vm = Some(handle);
	ctx.broadcast("vmBooted", &VmBootedPayload {})?;
	Ok(())
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
	if let Err(error) = ctx.broadcast("vmShutdown", &VmShutdownPayload { reason }) {
		tracing::warn!(?error, reason, "vmShutdown broadcast failed");
	}
}
