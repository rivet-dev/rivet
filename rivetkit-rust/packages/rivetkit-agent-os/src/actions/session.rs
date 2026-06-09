//! Session actions. Wraps `AgentOs::create_session` / `prompt` / lifecycle
//! methods. Public action surface intentionally mirrors the JS-facing
//! names from the rivetkit demo:
//!
//! ```ignore
//! const session = await agent.createSession("pi", { env: { ... } });
//! const response = await agent.sendPrompt(session.sessionId, "...");
//! ```

use std::collections::BTreeMap;

use agent_os_client::{
	AgentOs, CreateSessionOptions, JsonRpcNotification, PromptResult, SessionId,
	SessionInfo,
};
use anyhow::Result;
use futures::StreamExt;
use rivetkit::Ctx;
use serde::{Deserialize, Serialize};

use crate::actor::AgentOsActor;

/// `createSession(agentType, options)` — port of [`AgentOs::create_session`].
/// On success, also spawns a background task that subscribes to the
/// session's event stream and rebroadcasts each notification as a
/// `sessionEvent` actor event via `ctx.broadcast(...)`. The subscription
/// task self-terminates when the session is destroyed (the stream
/// closes) or when the VM is dropped.
pub async fn create_session(
	vm: &AgentOs,
	ctx: &Ctx<AgentOsActor>,
	agent_type: &str,
	options: CreateSessionOptionsDto,
) -> Result<SessionId> {
	let session = vm.create_session(agent_type, options.into_native()).await?;
	spawn_session_event_forwarder(vm, ctx, &session.session_id);
	Ok(session)
}

/// Spawn a detached task that forwards `session/update` notifications
/// from the agent-os event stream to actor subscribers via
/// `ctx.broadcast("sessionEvent", payload)`. Errors during subscription
/// or broadcast are logged but don't propagate — failing to wire events
/// shouldn't fail the session creation.
fn spawn_session_event_forwarder(
	vm: &AgentOs,
	ctx: &Ctx<AgentOsActor>,
	session_id: &str,
) {
	let (stream, subscription) = match vm.on_session_event(session_id) {
		Ok(pair) => pair,
		Err(error) => {
			tracing::warn!(
				?error,
				session_id,
				"failed to subscribe to session events; sessionEvent broadcasts disabled"
			);
			return;
		}
	};
	let ctx = ctx.clone();
	let session_id_owned = session_id.to_owned();
	tracing::info!(
		session_id = %session_id_owned,
		"session-event forwarder spawned"
	);
	tokio::spawn(async move {
		// Hold the subscription handle alive for the lifetime of the
		// forwarder task; dropping it cancels the underlying stream.
		let _subscription = subscription;
		let mut stream = stream;
		let mut event_count: u64 = 0;
		while let Some(notification) = stream.next().await {
			event_count += 1;
			let payload = SessionEventPayload {
				session_id: &session_id_owned,
				event: &notification,
			};
			tracing::info!(
				session_id = %session_id_owned,
				event_count,
				method = %notification.method,
				"forwarding session event"
			);
			if let Err(error) = ctx.broadcast("sessionEvent", &payload) {
				tracing::warn!(
					?error,
					session_id = %session_id_owned,
					"sessionEvent broadcast failed"
				);
			}
		}
		tracing::info!(
			session_id = %session_id_owned,
			event_count,
			"session-event forwarder exiting; stream closed"
		);
	});
}

#[derive(Serialize)]
struct SessionEventPayload<'a> {
	#[serde(rename = "sessionId")]
	session_id: &'a str,
	event: &'a JsonRpcNotification,
}

/// `sendPrompt(sessionId, text)` — port of [`AgentOs::prompt`]. Replies
/// with a [`PromptReplyDto`] carrying the response text plus the full
/// JSON-RPC envelope for callers that need protocol-level access.
pub async fn send_prompt(
	vm: &AgentOs,
	session_id: &str,
	text: &str,
) -> Result<PromptReplyDto> {
	vm.prompt(session_id, text).await.map(PromptReplyDto::from)
}

/// `listSessions()` — port of [`AgentOs::list_sessions`] (sync). Returns
/// the in-memory session registry summaries.
pub fn list_sessions(vm: &AgentOs) -> Vec<SessionInfo> {
	vm.list_sessions()
}

/// `destroySession(sessionId)` — port of [`AgentOs::destroy_session`].
/// Tears down the session in the sidecar.
pub async fn destroy_session(vm: &AgentOs, session_id: &str) -> Result<()> {
	vm.destroy_session(session_id).await
}

/// `closeSession(sessionId)` — port of [`AgentOs::close_session`] (sync).
/// Local cleanup only; complements `destroySession` for cases where
/// only the local handle needs releasing.
pub fn close_session(vm: &AgentOs, session_id: &str) -> Result<()> {
	vm.close_session(session_id).map_err(anyhow::Error::from)
}

// ---------------------------------------------------------------------------
// Action argument / reply DTOs
// ---------------------------------------------------------------------------

/// Serializable mirror of [`CreateSessionOptions`] for the camelCase TS
/// boundary. The upstream type isn't `Deserialize`, so this DTO bridges
/// the wire shape and converts.
///
/// Phase 3a minimum: `cwd`, `env`, `additionalInstructions` are honored.
/// `mcpServers` and `skipOsInstructions` are accepted but currently
/// passed through with defaults; full handling lands when the
/// corresponding TS surface is built out.
#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CreateSessionOptionsDto {
	pub cwd: Option<String>,
	pub env: BTreeMap<String, String>,
	pub additional_instructions: Option<String>,
	pub skip_os_instructions: bool,
}

impl CreateSessionOptionsDto {
	fn into_native(self) -> CreateSessionOptions {
		CreateSessionOptions {
			cwd: self.cwd,
			env: self.env,
			mcp_servers: Vec::new(),
			skip_os_instructions: self.skip_os_instructions,
			additional_instructions: self.additional_instructions,
		}
	}
}

/// Serializable reply for `sendPrompt`. Surfaces both the convenient
/// `text` field (what the demo wants) and the full JSON-RPC response
/// envelope so callers that need raw protocol-level data have it.
#[derive(Serialize)]
pub struct PromptReplyDto {
	pub text: String,
	pub response: agent_os_client::JsonRpcResponse,
}

impl From<PromptResult> for PromptReplyDto {
	fn from(value: PromptResult) -> Self {
		Self {
			text: value.text,
			response: value.response,
		}
	}
}
