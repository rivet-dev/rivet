//! Cron actions. The client's `CronJobOptions` / `CronAction` /
//! `CronJobInfo` are not serde types (they carry closures), so we define
//! serde DTOs here and map to/from the client types.

use agent_os_client::{AgentOs, CronAction, CronJobOptions, CronOverlap};
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

/// `{ type: "exec", command, args }` | `{ type: "session", agentType, prompt }`.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum CronActionDto {
	Exec {
		command: String,
		#[serde(default)]
		args: Vec<String>,
	},
	Session {
		agent_type: String,
		prompt: String,
	},
}

/// Options object for `scheduleCron(...)`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CronJobOptionsDto {
	#[serde(default)]
	pub id: Option<String>,
	pub schedule: String,
	pub action: CronActionDto,
	#[serde(default)]
	pub overlap: Option<CronOverlap>,
}

/// `{ id }` returned by `scheduleCron`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledCronDto {
	pub id: String,
}

/// One entry returned by `listCronJobs`. `last_run` / `next_run` are
/// epoch-millis timestamps serialized as `f64` so they cross the napi
/// boundary as JS `number`s (not `BigInt`s), matching the core API.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CronJobInfoDto {
	pub id: String,
	pub schedule: String,
	pub overlap: CronOverlap,
	pub last_run: Option<f64>,
	pub next_run: Option<f64>,
}

fn to_action(dto: CronActionDto) -> CronAction {
	match dto {
		CronActionDto::Exec { command, args } => CronAction::Exec { command, args },
		CronActionDto::Session { agent_type, prompt } => CronAction::Session {
			agent_type,
			prompt,
			options: None,
		},
	}
}

pub fn schedule_cron(vm: &AgentOs, dto: CronJobOptionsDto) -> Result<ScheduledCronDto> {
	let options = CronJobOptions {
		id: dto.id,
		schedule: dto.schedule,
		action: to_action(dto.action),
		overlap: dto.overlap,
	};
	let handle = vm.schedule_cron(options).map_err(|e| anyhow!(e))?;
	Ok(ScheduledCronDto { id: handle.id })
}

pub fn list_cron_jobs(vm: &AgentOs) -> Vec<CronJobInfoDto> {
	vm.list_cron_jobs()
		.into_iter()
		.map(|info| CronJobInfoDto {
			id: info.id,
			schedule: info.schedule,
			overlap: info.overlap,
			last_run: info.last_run.map(|t| t.timestamp_millis() as f64),
			next_run: info.next_run.map(|t| t.timestamp_millis() as f64),
		})
		.collect()
}

pub fn cancel_cron_job(vm: &AgentOs, id: &str) {
	vm.cancel_cron_job(id);
}
