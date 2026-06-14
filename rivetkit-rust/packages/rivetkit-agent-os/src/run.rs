//! Actor run loop. Brings up an `AgentOs` VM lazily on the first action
//! that needs it, tears it down on `Sleep` / `Destroy`, dispatches
//! actions through [`actions::dispatch`].

use std::collections::HashMap;
use std::sync::Arc;

use agent_os_client::AgentOs;
use anyhow::{Result, anyhow};
use bytes::Bytes;
use rivetkit::{Ctx, Event, HttpCall, Response, Start};
use serde::Serialize;

use crate::actions;
use crate::actions::preview::{self, PreviewStore};
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
	let mut previews: PreviewStore = PreviewStore::new();

	while let Some(event) = start.events.recv().await {
		match event {
			Event::Action(action) => {
				if let Err(error) = ensure_vm(&start.ctx, &config, &mut vm).await {
					action.err(error);
					continue;
				}
				let handle = vm.as_ref().expect("vm present after ensure_vm");
				actions::dispatch(handle, &mut previews, action).await;
			}
			Event::Http(http) => proxy_preview(vm.as_ref(), &mut previews, http).await,
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

/// Proxy a `/preview/{token}/...` HTTP request to the guest port the token
/// was issued for. The first path segment after `/preview/` is the token;
/// the remainder is forwarded to the guest service via [`AgentOs::fetch`].
/// An unmatched path, an unknown or expired token, or a VM that is not yet
/// up all reply `404`.
async fn proxy_preview(vm: Option<&AgentOs>, previews: &mut PreviewStore, http: HttpCall) {
	let path = http
		.request()
		.map(|request| request.uri().path().to_owned())
		.unwrap_or_default();
	let Some(rest) = path.strip_prefix("/preview/") else {
		http.reply_status(404);
		return;
	};
	let (token, forward_path) = match rest.split_once('/') {
		Some((token, tail)) => (token.to_owned(), format!("/{tail}")),
		None => (rest.to_owned(), "/".to_owned()),
	};

	let Some(port) = preview::resolve(previews, &token) else {
		http.reply_status(404);
		return;
	};
	let Some(vm) = vm else {
		http.reply_status(404);
		return;
	};

	let (request, reply) = match http.into_request() {
		Ok(pair) => pair,
		Err(error) => {
			tracing::warn!(?error, "preview request decode failed");
			return;
		}
	};
	let forward_uri: http::Uri = match forward_path.parse() {
		Ok(uri) => uri,
		Err(error) => {
			reply.reply_err(anyhow!("invalid preview path: {error}"));
			return;
		}
	};
	let (parts, body) = request.into_inner().into_parts();
	let mut forwarded = http::Request::new(Bytes::from(body));
	*forwarded.method_mut() = parts.method;
	*forwarded.uri_mut() = forward_uri;
	*forwarded.headers_mut() = parts.headers;

	match vm.fetch(port, forwarded).await {
		Ok(response) => {
			let status = response.status().as_u16();
			let mut headers: HashMap<String, String> = HashMap::new();
			for (name, value) in response.headers().iter() {
				headers.insert(
					name.as_str().to_owned(),
					String::from_utf8_lossy(value.as_bytes()).into_owned(),
				);
			}
			let body = response.into_body().to_vec();
			match Response::from_parts(status, headers, body) {
				Ok(response) => reply.reply(response),
				Err(error) => reply.reply_err(error),
			}
		}
		Err(error) => reply.reply_err(error),
	}
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
