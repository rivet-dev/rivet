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
	AgentOs, CreateSessionOptions, PromptResult, SessionId, SessionInfo,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// `createSession(agentType, options)` — port of [`AgentOs::create_session`].
/// Returns the [`SessionId`] `{ sessionId }` directly; the upstream type
/// already serializes with camelCase.
pub async fn create_session(
	vm: &AgentOs,
	agent_type: &str,
	options: CreateSessionOptionsDto,
) -> Result<SessionId> {
	vm.create_session(agent_type, options.into_native()).await
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
