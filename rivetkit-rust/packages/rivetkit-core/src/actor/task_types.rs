use std::{any::Any, fmt};

use anyhow::Result;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleState {
	#[default]
	Loading,
	Started,
	SleepGrace,
	SleepFinalize,
	DestroyGrace,
	Destroying,
	Terminated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownKind {
	Sleep,
	Destroy,
}

impl ShutdownKind {
	pub(crate) fn as_metric_label(self) -> &'static str {
		match self {
			ShutdownKind::Sleep => "sleep",
			ShutdownKind::Destroy => "destroy",
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserTaskKind {
	Action,
	Http,
	WebSocketLifetime,
	WebSocketCallback,
	QueueWait,
	ScheduledAction,
	DisconnectCallback,
	WaitUntil,
	SleepFinalize,
	DestroyRequest,
}

impl UserTaskKind {
	pub(crate) const ALL: [Self; 10] = [
		Self::Action,
		Self::Http,
		Self::WebSocketLifetime,
		Self::WebSocketCallback,
		Self::QueueWait,
		Self::ScheduledAction,
		Self::DisconnectCallback,
		Self::WaitUntil,
		Self::SleepFinalize,
		Self::DestroyRequest,
	];

	pub(crate) fn as_metric_label(self) -> &'static str {
		match self {
			Self::Action => "action",
			Self::Http => "http",
			Self::WebSocketLifetime => "websocket_lifetime",
			Self::WebSocketCallback => "websocket_callback",
			Self::QueueWait => "queue_wait",
			Self::ScheduledAction => "scheduled_action",
			Self::DisconnectCallback => "disconnect_callback",
			Self::WaitUntil => "wait_until",
			Self::SleepFinalize => "sleep_finalize",
			Self::DestroyRequest => "destroy_request",
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateMutationReason {
	InternalReplace,
	ScheduledEventsUpdate,
	InputSet,
	HasInitialized,
}

impl StateMutationReason {
	pub(crate) const ALL: [Self; 4] = [
		Self::InternalReplace,
		Self::ScheduledEventsUpdate,
		Self::InputSet,
		Self::HasInitialized,
	];

	pub(crate) fn as_metric_label(self) -> &'static str {
		match self {
			Self::InternalReplace => "internal_replace",
			Self::ScheduledEventsUpdate => "scheduled_events_update",
			Self::InputSet => "input_set",
			Self::HasInitialized => "has_initialized",
		}
	}
}

pub enum ActorChildOutcome {
	UserTaskFinished {
		kind: UserTaskKind,
		result: Result<()>,
	},
	UserTaskPanicked {
		kind: UserTaskKind,
		payload: Box<dyn Any + Send>,
	},
}

impl fmt::Debug for ActorChildOutcome {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			ActorChildOutcome::UserTaskFinished { kind, result } => f
				.debug_struct("UserTaskFinished")
				.field("kind", kind)
				.field("result", result)
				.finish(),
			ActorChildOutcome::UserTaskPanicked { kind, .. } => f
				.debug_struct("UserTaskPanicked")
				.field("kind", kind)
				.field("payload", &"<panic payload>")
				.finish(),
		}
	}
}
