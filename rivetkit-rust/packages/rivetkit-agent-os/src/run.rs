//! Actor run loop. Brings up an `AgentOs` VM lazily on the first action
//! that needs it, tears it down on `Sleep` / `Destroy`, dispatches
//! actions through [`actions::dispatch`].

use std::collections::HashMap;
use std::sync::Arc;

use agent_os_client::{
	AgentOs, MountPlugin, RootFilesystemConfig, RootFilesystemKind, SidecarJsBridgeCallback,
};
use anyhow::{Result, anyhow};
use bytes::Bytes;
use rivetkit::{Ctx, HttpCall, Response, RuntimeEvent, Start};
use serde::Serialize;
use serde_json::json;

use crate::actions;
use crate::actions::preview;
use crate::actor::{AgentOsActor, Vars};
use crate::config::AgentOsActorConfig;
use crate::persistence;

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
pub async fn run(config: Arc<AgentOsActorConfig>, mut start: Start<AgentOsActor>) -> Result<()> {
	let mut vm: Option<AgentOs> = None;
	// Ephemeral per-VM-lifetime state: the `external -> live` session remap and
	// the live event-capture pump tasks. Reconstructed on each wake; cleared on
	// teardown (see `Vars::clear`).
	let mut vars = Vars::default();

	// Ensure the agent-os SQLite persistence schema exists before handling any
	// events. Bare unit-test contexts can omit SQLite; production actor contexts
	// provide it and get the durable sqlite_vfs root below.
	if start.ctx.sql().is_enabled() {
		persistence::migrate_actor(&start.ctx).await?;
	}

	while let Some(event) = start.events.recv().await {
		match event {
			RuntimeEvent::Action(action) => {
				if let Err(error) = ensure_vm(&start.ctx, &config, &mut vm).await {
					tracing::error!(?error, "ensure_vm failed");
					action.err(error);
					continue;
				}
				let handle = vm.as_ref().expect("vm present after ensure_vm");
				actions::dispatch(&start.ctx, handle, &mut vars, action).await;
			}
			RuntimeEvent::Http(http) => proxy_preview(&start.ctx, vm.as_ref(), http).await,
			RuntimeEvent::QueueSend(queue) => queue.err(anyhow!("queue send not supported")),
			RuntimeEvent::WebSocketOpen(ws) => ws.reject(anyhow!("websocket not supported")),
			RuntimeEvent::ConnOpen(conn) => conn.accept(()),
			RuntimeEvent::ConnClosed(_) => {}
			RuntimeEvent::Subscribe(subscribe) => subscribe.allow(),
			RuntimeEvent::SerializeState(serialize) => serialize.skip(),
			RuntimeEvent::Sleep(sleep) => {
				// Cancel live event-capture pumps + drop the remap before the VM
				// goes away; both are reconstructed on wake.
				vars.clear();
				shutdown_vm(&start.ctx, &mut vm, "sleep").await;
				sleep.ok();
			}
			RuntimeEvent::Destroy(destroy) => {
				vars.clear();
				shutdown_vm(&start.ctx, &mut vm, "destroy").await;
				destroy.ok();
			}
		}
	}

	// Channel closed: best-effort cleanup if the run loop terminates while
	// a VM is still up.
	vars.clear();
	shutdown_vm(&start.ctx, &mut vm, "error").await;

	Ok(())
}

/// Proxy a `/preview/{token}/...` HTTP request to the guest port the token
/// was issued for. The first path segment after `/preview/` is the token;
/// the remainder is forwarded to the guest service via [`AgentOs::fetch`].
/// An unmatched path, an unknown or expired token, or a VM that is not yet
/// up all reply `404`.
async fn proxy_preview(ctx: &Ctx<AgentOsActor>, vm: Option<&AgentOs>, http: HttpCall) {
	let path = http
		.request()
		.map(|request| request.uri().path().to_owned())
		.unwrap_or_default();
	let Some(rest) = path.strip_prefix("/preview/") else {
		tracing::warn!(%path, "proxy_preview: path lacks /preview/ prefix");
		http.reply_status(404);
		return;
	};
	let (token, forward_path) = match rest.split_once('/') {
		Some((token, tail)) => (token.to_owned(), format!("/{tail}")),
		None => (rest.to_owned(), "/".to_owned()),
	};

	let port = match preview::resolve(ctx, &token).await {
		Ok(Some(port)) => port,
		Ok(None) => {
			tracing::warn!(token, "proxy_preview: token not found in persistence");
			http.reply_status(404);
			return;
		}
		Err(error) => {
			tracing::warn!(?error, "preview token resolve failed");
			http.reply_status(404);
			return;
		}
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
	let mut options = config.build_options();
	configure_actor_db_root(ctx, &mut options);
	let handle = AgentOs::create(options)
		.await
		.map_err(|error| anyhow!("agent-os vm bring-up failed: {error}"))?;
	*vm = Some(handle);
	ctx.broadcast("vmBooted", &VmBootedPayload {})?;
	Ok(())
}

fn configure_actor_db_root(ctx: &Ctx<AgentOsActor>, options: &mut agent_os_client::AgentOsConfig) {
	if !ctx.sql().is_enabled() {
		tracing::debug!("actor DB root disabled because ctx.sql is unavailable");
		return;
	}

	if options.root_filesystem == RootFilesystemConfig::default() {
		tracing::debug!("configuring actor DB sqlite_vfs root filesystem");
		options.root_filesystem = RootFilesystemConfig {
			kind: RootFilesystemKind::Native,
			native_plugin: Some(MountPlugin {
				id: "sqlite_vfs".to_owned(),
				config: Some(json!({
					"backend": "callback",
					"mountId": "rivetkit-agent-os-root",
				})),
			}),
			..RootFilesystemConfig::default()
		};
	} else {
		tracing::debug!(
			root_filesystem = ?options.root_filesystem,
			"keeping configured agent-os root filesystem"
		);
	}

	if options.sidecar_js_bridge_callback.is_none() {
		let ctx = ctx.clone();
		let callback: SidecarJsBridgeCallback = Arc::new(move |call| {
			let ctx = ctx.clone();
			Box::pin(async move { persistence::handle_sqlite_vfs_call(&ctx, call).await })
		});
		options.sidecar_js_bridge_callback = Some(callback);
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
	if let Err(error) = ctx.broadcast("vmShutdown", &VmShutdownPayload { reason }) {
		tracing::warn!(?error, reason, "vmShutdown broadcast failed");
	}
}
