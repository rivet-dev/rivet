use std::time::Duration;

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use rivetkit_core::ActorContext as CoreActorContext;
use rivetkit_core::actor::schedule::{
	CronFire as CoreCronFire, CronJobInfo as CoreCronJobInfo, ScheduleKind,
	ScheduledEventInfo as CoreScheduledEventInfo,
};

use crate::{NapiInvalidArgument, napi_anyhow_error};

#[napi(object)]
pub struct JsScheduledEventInfo {
	pub id: String,
	pub action: String,
	pub args: Buffer,
	pub run_at: i64,
}

#[napi(object)]
pub struct JsCronJobInfo {
	pub name: String,
	pub kind: String,
	pub action: String,
	pub args: Buffer,
	pub next_run_at: i64,
	pub last_run_at: Option<i64>,
	pub expression: Option<String>,
	pub timezone: Option<String>,
	pub interval_ms: Option<i64>,
	pub max_history: i64,
}

#[napi(object)]
pub struct JsScheduleErrorInfo {
	pub group: String,
	pub code: String,
	pub message: String,
	pub metadata: Option<serde_json::Value>,
}

#[napi(object)]
pub struct JsCronFire {
	pub action: String,
	pub scheduled_at: i64,
	pub fired_at: i64,
	pub finished_at: Option<i64>,
	pub result: String,
	pub error: Option<JsScheduleErrorInfo>,
}

#[napi]
pub struct Schedule {
	inner: CoreActorContext,
}

impl Schedule {
	pub(crate) fn new(inner: CoreActorContext) -> Self {
		Self { inner }
	}
}

#[napi]
impl Schedule {
	#[napi]
	pub async fn after(
		&self,
		duration_ms: i64,
		action_name: String,
		args: Buffer,
	) -> napi::Result<String> {
		let duration_ms = u64::try_from(duration_ms).map_err(|_| {
			napi_anyhow_error(
				NapiInvalidArgument {
					argument: "durationMs".to_owned(),
					reason: "must be non-negative".to_owned(),
				}
				.build(),
			)
		})?;
		self.inner
			.after(
				Duration::from_millis(duration_ms),
				&action_name,
				args.as_ref(),
			)
			.await
			.map_err(napi_anyhow_error)
	}

	#[napi]
	pub async fn at(
		&self,
		timestamp_ms: i64,
		action_name: String,
		args: Buffer,
	) -> napi::Result<String> {
		self.inner
			.at(timestamp_ms, &action_name, args.as_ref())
			.await
			.map_err(napi_anyhow_error)
	}

	#[napi]
	pub async fn cancel(&self, id: String) -> napi::Result<bool> {
		self.inner
			.cancel_schedule(&id)
			.await
			.map_err(napi_anyhow_error)
	}

	#[napi]
	pub async fn get(&self, id: String) -> napi::Result<Option<JsScheduledEventInfo>> {
		self.inner
			.get_scheduled_event(&id)
			.await
			.map(|event| event.map(JsScheduledEventInfo::from))
			.map_err(napi_anyhow_error)
	}

	#[napi]
	pub async fn list(&self) -> napi::Result<Vec<JsScheduledEventInfo>> {
		self.inner
			.list_scheduled_events()
			.await
			.map(|events| events.into_iter().map(Into::into).collect())
			.map_err(napi_anyhow_error)
	}

	#[napi]
	pub async fn cron_set(
		&self,
		name: String,
		expression: String,
		timezone: Option<String>,
		action_name: String,
		args: Buffer,
		max_history: Option<i64>,
	) -> napi::Result<()> {
		self.inner
			.cron_set(
				&name,
				&expression,
				timezone.as_deref(),
				&action_name,
				args.as_ref(),
				max_history,
			)
			.await
			.map_err(napi_anyhow_error)
	}

	#[napi]
	pub async fn cron_every(
		&self,
		name: String,
		interval_ms: i64,
		action_name: String,
		args: Buffer,
		max_history: Option<i64>,
	) -> napi::Result<()> {
		self.inner
			.cron_every(&name, interval_ms, &action_name, args.as_ref(), max_history)
			.await
			.map_err(napi_anyhow_error)
	}

	#[napi]
	pub async fn cron_get(&self, name: String) -> napi::Result<Option<JsCronJobInfo>> {
		self.inner
			.cron_get(&name)
			.await
			.map(|job| job.map(JsCronJobInfo::from))
			.map_err(napi_anyhow_error)
	}

	#[napi]
	pub async fn cron_list(&self) -> napi::Result<Vec<JsCronJobInfo>> {
		self.inner
			.cron_list()
			.await
			.map(|jobs| jobs.into_iter().map(Into::into).collect())
			.map_err(napi_anyhow_error)
	}

	#[napi]
	pub async fn cron_delete(&self, name: String) -> napi::Result<bool> {
		self.inner
			.cron_delete(&name)
			.await
			.map_err(napi_anyhow_error)
	}

	#[napi]
	pub async fn cron_history(
		&self,
		name: String,
		limit: Option<i64>,
	) -> napi::Result<Vec<JsCronFire>> {
		self.inner
			.cron_history(&name, limit)
			.await
			.map(|fires| fires.into_iter().map(Into::into).collect())
			.map_err(napi_anyhow_error)
	}
}

impl From<CoreScheduledEventInfo> for JsScheduledEventInfo {
	fn from(value: CoreScheduledEventInfo) -> Self {
		Self {
			id: value.id,
			action: value.action,
			args: Buffer::from(value.args),
			run_at: value.run_at,
		}
	}
}

impl From<CoreCronJobInfo> for JsCronJobInfo {
	fn from(value: CoreCronJobInfo) -> Self {
		Self {
			name: value.name,
			kind: match value.kind {
				ScheduleKind::At => "at",
				ScheduleKind::Cron => "cron",
				ScheduleKind::Every => "every",
			}
			.to_owned(),
			action: value.action,
			args: Buffer::from(value.args),
			next_run_at: value.next_run_at,
			last_run_at: value.last_run_at,
			expression: value.expression,
			timezone: value.timezone,
			interval_ms: value.interval_ms,
			max_history: value.max_history,
		}
	}
}

impl From<CoreCronFire> for JsCronFire {
	fn from(value: CoreCronFire) -> Self {
		Self {
			action: value.action,
			scheduled_at: value.scheduled_at,
			fired_at: value.fired_at,
			finished_at: value.finished_at,
			result: value.result,
				error: value.error.map(|error| JsScheduleErrorInfo {
					group: error.group,
					code: error.code,
					message: error.message,
					metadata: error.metadata,
				}),
		}
	}
}
