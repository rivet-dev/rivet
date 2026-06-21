//! Agent session actions: create an ACP agent session, send prompts,
//! and close it. Ports of [`AgentOs::create_session`] / `prompt` /
//! `close_session`.
//!
//! Session metadata is persisted to the actor's SQLite database
//! (`agent_os_sessions`, with streamed events in `agent_os_session_events`)
//! via `ctx.db_*`, so the set of sessions survives actor sleep/wake. The live
//! ACP session itself lives in the VM and is recreated on demand.

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use agent_os_client::{AgentOs, CreateSessionOptions, ResumeSessionOptions};
use anyhow::{Result, anyhow};
use futures::StreamExt;
use rivetkit::Ctx;
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};

use crate::actor::{AgentOsActor, Vars};
use crate::persistence::{
	insert_session_event, query_rows, reconstruct_transcript_to_file, run_stmt,
};

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

/// One row of `listPersistedSessions`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedSessionDto {
	pub session_id: String,
	pub agent_type: String,
	pub created_at: f64,
}

fn now_ms() -> i64 {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map(|d| d.as_millis() as i64)
		.unwrap_or(0)
}

/// Subscribe to the live `session/update` stream for `live_session_id` and
/// spawn a task that persists each event under `external_session_id` (spec §5).
///
/// The subscription is broadcast-backed, so aborting the spawned task — which
/// drops the stream — is the unsubscribe. The handle is tracked in
/// [`Vars::capture_tasks`] keyed by the live id so it can be cancelled on close
/// / sleep / destroy. Re-subscribing for the same live id first aborts any
/// existing pump so we never run two pumps for one session.
fn spawn_event_capture(
	ctx: &Ctx<AgentOsActor>,
	vm: &AgentOs,
	vars: &mut Vars,
	external_session_id: &str,
	live_session_id: &str,
) {
	let (mut stream, subscription) = match vm.on_session_event(live_session_id) {
		Ok(sub) => sub,
		Err(error) => {
			tracing::warn!(?error, live_session_id, "on_session_event subscribe failed");
			return;
		}
	};
	// Replace any existing pump for this live id.
	if let Some(old) = vars.capture_tasks.remove(live_session_id) {
		old.abort();
	}
	let ctx = ctx.clone();
	let external = external_session_id.to_owned();
	let handle = tokio::spawn(async move {
		// Keep the RAII guard alive for the lifetime of the pump; dropping the
		// stream (on abort / channel close) is the unsubscribe.
		let _subscription = subscription;
		while let Some(notification) = stream.next().await {
			let event_json = match serde_json::to_string(&notification) {
				Ok(json) => json,
				Err(error) => {
					tracing::warn!(?error, "failed to encode captured session event");
					continue;
				}
			};
			if let Err(error) = insert_session_event(&ctx, &external, &event_json).await {
				tracing::warn!(?error, external, "failed to persist captured session event");
			}
		}
	});
	vars.capture_tasks
		.insert(live_session_id.to_owned(), handle);
}

pub async fn create_session(
	ctx: &Ctx<AgentOsActor>,
	vm: &AgentOs,
	vars: &mut Vars,
	agent_type: &str,
	dto: CreateSessionOptionsDto,
) -> Result<SessionIdDto> {
	// Capture cwd + env BEFORE they move into `options`, so the resume fallback
	// `session/new` can rehydrate the same working dir + environment (spec §12b).
	let persist_cwd = dto.cwd.clone();
	let persist_env = serde_json::to_string(&dto.env).ok();
	let options = CreateSessionOptions {
		cwd: dto.cwd,
		env: dto.env,
		skip_os_instructions: dto.skip_os_instructions,
		additional_instructions: dto.additional_instructions,
		..CreateSessionOptions::default()
	};
	let session_id = vm.create_session(agent_type, options).await?.session_id;
	// Persist session metadata so the set of sessions survives sleep/wake. Capture the REAL
	// agent capabilities + info (not a `"{}"` placeholder) so the resume path can capability-gate
	// the native `session/load` tier after a wake, when the live session is gone. See
	// `resume_session` for how these are read back.
	let capabilities = vm
		.get_session_capabilities(&session_id)
		.and_then(|caps| serde_json::to_string(&caps).ok())
		.unwrap_or_else(|| "{}".to_owned());
	let agent_info = vm
		.get_session_agent_info(&session_id)
		.and_then(|info| serde_json::to_string(&info).ok());
	run_stmt(
		ctx,
		"INSERT OR REPLACE INTO agent_os_sessions \
		 (session_id, agent_type, capabilities, agent_info, created_at, cwd, env) \
		 VALUES (?, ?, ?, ?, ?, ?, ?)",
		&[
			json!(session_id),
			json!(agent_type),
			json!(capabilities),
			agent_info.map(JsonValue::String).unwrap_or(JsonValue::Null),
			json!(now_ms()),
			persist_cwd
				.map(JsonValue::String)
				.unwrap_or(JsonValue::Null),
			persist_env
				.map(JsonValue::String)
				.unwrap_or(JsonValue::Null),
		],
	)
	.await?;
	// At create time `external == live`; capture every `session/update` for this
	// session under the external id (spec §3/§5).
	spawn_event_capture(ctx, vm, vars, &session_id, &session_id);
	Ok(SessionIdDto { session_id })
}

pub async fn send_prompt(
	ctx: &Ctx<AgentOsActor>,
	vm: &AgentOs,
	vars: &mut Vars,
	session_id: &str,
	text: &str,
) -> Result<PromptResultDto> {
	// Lazy-resume trigger (spec §8): a prompt for a session that is persisted in
	// `agent_os_sessions` but absent from `Vars.live_sessions` means the VM was
	// recreated since the session was last live — resume it before forwarding.
	// `session_id` here is the client-facing `external_session_id`.
	//
	// Canonical resume state-machine documentation lives on the sidecar handler
	// in `crates/agent-os-sidecar/src/acp_extension.rs` (spec §6); this is just
	// the actor-side trigger that drives it.
	if !vars.live_sessions.contains_key(session_id) && !is_session_live(vm, session_id) {
		if session_is_persisted(ctx, session_id).await? {
			resume_session(ctx, vm, vars, session_id).await?;
		}
	}

	// Record the outbound prompt text as a synthetic `user_prompt` event BEFORE
	// the prompt streams, so the transcript turn ordering is correct (the prompt
	// row precedes the agent `session/update` rows for this turn). Stored under
	// the stable external id (spec §4/§5).
	let prompt_event = json!({
		"method": "user_prompt",
		"params": { "text": text },
	});
	if let Err(error) = insert_session_event(ctx, session_id, &prompt_event.to_string()).await {
		tracing::warn!(?error, session_id, "failed to persist user_prompt event");
	}

	// Forward to the live id (== external for native/not-yet-resumed sessions).
	let live_session_id = vars.live_id(session_id).to_owned();
	let result = vm.prompt(&live_session_id, text).await?;
	Ok(PromptResultDto { text: result.text })
}

pub async fn close_session(
	ctx: &Ctx<AgentOsActor>,
	vm: &AgentOs,
	vars: &mut Vars,
	session_id: &str,
) -> Result<()> {
	// Stop event capture + drop the remap for this external session.
	let live_session_id = vars.live_id(session_id).to_owned();
	if let Some(task) = vars.capture_tasks.remove(&live_session_id) {
		task.abort();
	}
	vars.live_sessions.remove(session_id);
	let persisted = session_is_persisted(ctx, session_id).await?;
	if is_session_live(vm, &live_session_id) {
		vm.close_session(&live_session_id).map_err(|e| anyhow!(e))?;
	} else if !persisted {
		// Preserve the unknown-session error for callers that close something that
		// is neither live nor durably persisted.
		vm.close_session(&live_session_id).map_err(|e| anyhow!(e))?;
	}
	// Drop persisted metadata + events (explicit, since SQLite FK cascade is
	// only enforced when `PRAGMA foreign_keys = ON`).
	run_stmt(
		ctx,
		"DELETE FROM agent_os_session_events WHERE session_id = ?",
		&[json!(session_id)],
	)
	.await?;
	run_stmt(
		ctx,
		"DELETE FROM agent_os_sessions WHERE session_id = ?",
		&[json!(session_id)],
	)
	.await?;
	Ok(())
}

/// List the sessions persisted for this actor (`listPersistedSessions`).
pub async fn list_persisted_sessions(ctx: &Ctx<AgentOsActor>) -> Result<Vec<PersistedSessionDto>> {
	let rows = query_rows(
		ctx,
		"SELECT session_id, agent_type, created_at FROM agent_os_sessions \
		 ORDER BY created_at",
		&[],
	)
	.await?;
	Ok(rows
		.into_iter()
		.map(|row| PersistedSessionDto {
			session_id: row
				.get("session_id")
				.and_then(|v| v.as_str())
				.unwrap_or_default()
				.to_owned(),
			agent_type: row
				.get("agent_type")
				.and_then(|v| v.as_str())
				.unwrap_or_default()
				.to_owned(),
			created_at: row.get("created_at").and_then(|v| v.as_i64()).unwrap_or(0) as f64,
		})
		.collect())
}

/// Return the persisted ACP events for a session, ordered by sequence
/// (`getSessionEvents`). Each event is the stored JSON-RPC notification.
pub async fn get_session_events(
	ctx: &Ctx<AgentOsActor>,
	session_id: &str,
) -> Result<Vec<JsonValue>> {
	let rows = query_rows(
		ctx,
		"SELECT event FROM agent_os_session_events WHERE session_id = ? ORDER BY seq",
		&[json!(session_id)],
	)
	.await?;
	Ok(rows
		.into_iter()
		.filter_map(|row| {
			row.get("event")
				.and_then(|v| v.as_str())
				.and_then(|raw| serde_json::from_str::<JsonValue>(raw).ok())
		})
		.collect())
}

/// True when an ACP session with this id is currently live in the VM.
fn is_session_live(vm: &AgentOs, session_id: &str) -> bool {
	vm.list_sessions()
		.iter()
		.any(|info| info.session_id == session_id)
}

/// True when `external_session_id` has a persisted registry row in
/// `agent_os_sessions` (so it is resumable).
async fn session_is_persisted(ctx: &Ctx<AgentOsActor>, external_session_id: &str) -> Result<bool> {
	let rows = query_rows(
		ctx,
		"SELECT session_id FROM agent_os_sessions WHERE session_id = ? LIMIT 1",
		&[json!(external_session_id)],
	)
	.await?;
	Ok(!rows.is_empty())
}

/// Persisted registry row needed to resume a session: agent type, parsed
/// capabilities (`{}` if absent), and the original create-time cwd + env.
struct SessionRegistryRow {
	agent_type: String,
	#[allow(dead_code)]
	capabilities: JsonValue,
	cwd: Option<String>,
	env: BTreeMap<String, String>,
}

/// Read the persisted registry row for a session (agent_type, capabilities, and
/// the create-time cwd + env) so resume can rehydrate it faithfully.
async fn read_session_registry(
	ctx: &Ctx<AgentOsActor>,
	external_session_id: &str,
) -> Result<SessionRegistryRow> {
	let rows = query_rows(
		ctx,
		"SELECT agent_type, capabilities, cwd, env FROM agent_os_sessions WHERE session_id = ? LIMIT 1",
		&[json!(external_session_id)],
	)
	.await?;
	let row = rows
		.into_iter()
		.next()
		.ok_or_else(|| anyhow!("no persisted session {external_session_id} to resume"))?;
	let agent_type = row
		.get("agent_type")
		.and_then(|v| v.as_str())
		.unwrap_or_default()
		.to_owned();
	let capabilities = row
		.get("capabilities")
		.and_then(|v| v.as_str())
		.and_then(|raw| serde_json::from_str::<JsonValue>(raw).ok())
		.unwrap_or_else(|| json!({}));
	let cwd = row
		.get("cwd")
		.and_then(|v| v.as_str())
		.map(|s| s.to_owned());
	let env = row
		.get("env")
		.and_then(|v| v.as_str())
		.and_then(|raw| serde_json::from_str::<BTreeMap<String, String>>(raw).ok())
		.unwrap_or_default();
	Ok(SessionRegistryRow {
		agent_type,
		capabilities,
		cwd,
		env,
	})
}

/// Resume a persisted-but-not-live session in the freshly recreated VM
/// (spec §6/§8). Reads the registry caps, reconstructs the transcript file from
/// `agent_os_session_events`, calls the sidecar resume orchestration via the
/// client, records the `external -> live` remap, and starts event capture for
/// the live session.
///
/// The canonical resume state machine (native `session/load`/`resume` tier with
/// the `unknown_session` fallthrough, then the universal `session/new` +
/// transcript-preamble fallback) lives on the sidecar handler in
/// `crates/agent-os-sidecar/src/acp_extension.rs` (spec §6). This actor function
/// only supplies the durable inputs (caps + transcript path) and records the
/// remap the sidecar returns.
pub async fn resume_session(
	ctx: &Ctx<AgentOsActor>,
	vm: &AgentOs,
	vars: &mut Vars,
	external_session_id: &str,
) -> Result<()> {
	let registry = read_session_registry(ctx, external_session_id).await?;

	// Disposable on-demand render of the canonical event log; handed to the
	// sidecar so a fallback agent can read prior context with its file tools.
	let transcript_path = reconstruct_transcript_to_file(ctx, external_session_id).await?;

	// Call the sidecar resume orchestration through the client. The contract is
	// `AcpResumeSessionRequest { sessionId, agentType, transcriptPath?, cwd, env }`
	// (spec §6); it returns the live session id (== external for the native tier,
	// a new id for the `session/new` fallback) plus the tier that ran (`mode`).
	//
	// Rehydrate with the ORIGINAL create-time cwd + env (spec §12b) so the fallback
	// `session/new` keeps the same working dir + environment instead of defaulting.
	let result = vm
		.resume_session(
			external_session_id,
			&registry.agent_type,
			ResumeSessionOptions {
				transcript_path: Some(transcript_path),
				cwd: registry.cwd,
				env: registry.env,
			},
		)
		.await?;
	tracing::info!(
		external_session_id,
		live_session_id = %result.session_id,
		mode = %result.mode,
		"resumed persisted session"
	);
	let live_session_id = result.session_id;

	// The remap lives SOLELY in the actor (spec §3): record external -> live so
	// subsequent prompts route to the live session, and start event capture for the
	// live session under the stable external id.
	//
	// NOTE on capture ordering (review SEV-3): we subscribe AFTER resume returns, on
	// purpose. For the native `session/load` tier the agent replays prior history as
	// `session/update` notifications DURING the load — those are already persisted in
	// `agent_os_session_events` from the original session, so subscribing afterward
	// correctly avoids RE-capturing (duplicating) the replay. For the `session/new`
	// fallback there is no replay. New post-resume updates are captured normally.
	vars.live_sessions
		.insert(external_session_id.to_owned(), live_session_id.clone());
	spawn_event_capture(ctx, vm, vars, external_session_id, &live_session_id);
	Ok(())
}
