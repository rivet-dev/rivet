//! Cron actions. Currently wraps `scheduleCron` / `listCronJobs` /
//! `cancelCronJob`. The cron-events broadcast wire-up (sidecar →
//! `ctx.broadcast("cronEvent", ...)`) lands in the same followup that
//! adds process output/exit broadcasts; for now the actions
//! arms are sufficient to satisfy the driver-test surface.

use agent_os_client::{
	AgentOs, CronAction, CronJobHandle, CronJobInfo, CronJobOptions, CronOverlap,
};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// `scheduleCron(options)` — port of [`AgentOs::schedule_cron`].
/// Returns the assigned job id; the caller can use it with
/// `cancelCronJob` later.
pub fn schedule_cron(
	vm: &AgentOs,
	options: ScheduleCronOptionsArg,
) -> Result<CronJobHandleDto> {
	let handle: CronJobHandle = vm
		.schedule_cron(options.into_native())
		.map_err(anyhow::Error::from)?;
	Ok(CronJobHandleDto { id: handle.id })
}

/// `listCronJobs()` — port of [`AgentOs::list_cron_jobs`]. Returns
/// snapshot info for every scheduled job, mapped to a serializable
/// shape.
pub fn list_cron_jobs(vm: &AgentOs) -> Vec<CronJobInfoDto> {
	vm.list_cron_jobs().into_iter().map(CronJobInfoDto::from).collect()
}

/// `cancelCronJob(id)` — port of [`AgentOs::cancel_cron_job`]. No-op
/// if the id is unknown; mirrors the upstream's never-errors contract.
pub fn cancel_cron_job(vm: &AgentOs, id: &str) {
	vm.cancel_cron_job(id);
}

// ---------------------------------------------------------------------------
// Action argument / reply DTOs
// ---------------------------------------------------------------------------

/// Input shape for `scheduleCron`. `action` is a tagged union matching
/// the JS shape `{type: "exec" | "session", ...}`. Callback actions
/// can't cross the wire so they're intentionally absent.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleCronOptionsArg {
	pub schedule: String,
	pub action: CronActionArg,
	#[serde(default)]
	pub id: Option<String>,
	#[serde(default)]
	pub overlap: Option<CronOverlap>,
}

impl ScheduleCronOptionsArg {
	fn into_native(self) -> CronJobOptions {
		CronJobOptions {
			id: self.id,
			schedule: self.schedule,
			action: self.action.into_native(),
			overlap: self.overlap,
		}
	}
}

/// Tagged union mirroring the JS `CronAction` discriminator. Only the
/// wire-friendly variants are accepted; `Callback` is host-only and
/// cannot be expressed across the bridge.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum CronActionArg {
	Exec {
		command: String,
		#[serde(default)]
		args: Vec<String>,
	},
	Session {
		#[serde(rename = "agentType")]
		agent_type: String,
		prompt: String,
	},
}

impl CronActionArg {
	fn into_native(self) -> CronAction {
		match self {
			Self::Exec { command, args } => CronAction::Exec { command, args },
			Self::Session {
				agent_type,
				prompt,
			} => CronAction::Session {
				agent_type,
				prompt,
				options: None,
			},
		}
	}
}

/// Reply for `scheduleCron`. Just the assigned id — the JS surface
/// pulls `{ id }` out of the handle.
#[derive(Serialize)]
pub struct CronJobHandleDto {
	pub id: String,
}

/// Serializable mirror of [`CronJobInfo`] for the listCronJobs reply.
/// camelCase to match the JS-side field names. Numeric timestamps go
/// out as ISO-8601 strings via chrono's serde impl.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CronJobInfoDto {
	pub id: String,
	pub schedule: String,
	pub action: CronActionDto,
	pub overlap: CronOverlap,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub last_run: Option<DateTime<Utc>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub next_run: Option<DateTime<Utc>>,
	pub run_count: u64,
	pub running: bool,
}

impl From<CronJobInfo> for CronJobInfoDto {
	fn from(value: CronJobInfo) -> Self {
		Self {
			id: value.id,
			schedule: value.schedule,
			action: CronActionDto::from(value.action),
			overlap: value.overlap,
			last_run: value.last_run,
			next_run: value.next_run,
			run_count: value.run_count,
			running: value.running,
		}
	}
}

/// Outbound mirror of [`CronAction`] for listCronJobs. `Callback` is
/// represented as a bare tag with no payload since the closure isn't
/// serializable; consumers that care can distinguish on the tag.
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum CronActionDto {
	Exec {
		command: String,
		args: Vec<String>,
	},
	Session {
		#[serde(rename = "agentType")]
		agent_type: String,
		prompt: String,
	},
	Callback,
}

impl From<CronAction> for CronActionDto {
	fn from(value: CronAction) -> Self {
		match value {
			CronAction::Exec { command, args } => Self::Exec { command, args },
			CronAction::Session {
				agent_type, prompt, ..
			} => Self::Session { agent_type, prompt },
			CronAction::Callback { .. } => Self::Callback,
		}
	}
}
