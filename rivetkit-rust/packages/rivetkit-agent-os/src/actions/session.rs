//! Agent session actions: create an ACP agent session, send prompts,
//! and close it. Ports of [`AgentOs::create_session`] / `prompt` /
//! `close_session`.

use std::collections::BTreeMap;

use agent_os_client::{AgentOs, CreateSessionOptions};
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

/// Options object for `createSession(agentType, options?)`.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSessionOptionsDto {
	#[serde(default)]
	pub cwd: Option<String>,
	#[serde(default)]
	pub env: BTreeMap<String, String>,
	#[serde(default)]
	pub skip_os_instructions: bool,
	#[serde(default)]
	pub additional_instructions: Option<String>,
}

/// `{ sessionId }` returned by `createSession`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionIdDto {
	pub session_id: String,
}

/// Result of `sendPrompt` exposed to the TS client.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptResultDto {
	pub text: String,
}

pub async fn create_session(
	vm: &AgentOs,
	agent_type: &str,
	dto: CreateSessionOptionsDto,
) -> Result<SessionIdDto> {
	let options = CreateSessionOptions {
		cwd: dto.cwd,
		env: dto.env,
		skip_os_instructions: dto.skip_os_instructions,
		additional_instructions: dto.additional_instructions,
		..CreateSessionOptions::default()
	};
	let session_id = vm.create_session(agent_type, options).await?;
	Ok(SessionIdDto {
		session_id: session_id.session_id,
	})
}

pub async fn send_prompt(vm: &AgentOs, session_id: &str, text: &str) -> Result<PromptResultDto> {
	let result = vm.prompt(session_id, text).await?;
	Ok(PromptResultDto { text: result.text })
}

pub fn close_session(vm: &AgentOs, session_id: &str) -> Result<()> {
	vm.close_session(session_id).map_err(|e| anyhow!(e))
}
