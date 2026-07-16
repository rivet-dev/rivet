use std::fmt;
use std::sync::Arc;

use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ActorErrorEvent {
	Action { name: String, scheduled: bool },
	Hook { name: String },
	Queue { name: String },
	Internal { kind: InternalErrorKind },
	Fatal { phase: FatalPhase },
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InternalErrorKind {
	Persist,
	Alarm,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FatalPhase {
	Run,
	Shutdown,
}

#[derive(Clone, Debug)]
pub struct ErrorReport {
	pub event: ActorErrorEvent,
	pub group: String,
	pub code: String,
	pub message: String,
	pub metadata: Option<serde_json::Value>,
	pub raw_error_ref: Option<u64>,
}

pub type OnErrorHook = Arc<dyn Fn(ErrorReport) + Send + Sync>;

#[derive(Debug)]
pub struct HookName(pub &'static str);

impl fmt::Display for HookName {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.0)
	}
}

impl std::error::Error for HookName {}

#[derive(Debug)]
pub struct RawErrorRef(pub u64);

impl fmt::Display for RawErrorRef {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "raw error ref {}", self.0)
	}
}

impl std::error::Error for RawErrorRef {}
