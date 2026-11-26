//! Errors the adapter produces itself, in the same groups and codes as napi so
//! clients see one set of errors whatever language hosts the actor.

use std::time::Duration;

use rivet_error::{MacroMarker, RivetError, RivetErrorKind, RivetErrorSchema};
use rivetkit_core::ActorLifecycle;
use rivetkit_core::error::ActorRuntime;

static ACTION_TIMED_OUT: RivetErrorSchema = RivetErrorSchema {
	group: "actor",
	code: "action_timed_out",
	default_message: "Action timed out",
	meta_type: None,
	_macro_marker: MacroMarker { _private: () },
};

static CALLBACK_TIMED_OUT: RivetErrorSchema = RivetErrorSchema {
	group: "actor",
	code: "callback_timed_out",
	default_message: "Lifecycle callback timed out",
	meta_type: None,
	_macro_marker: MacroMarker { _private: () },
};

fn with_kind(
	kind: RivetErrorKind,
	message: String,
	meta: Option<serde_json::Value>,
) -> anyhow::Error {
	anyhow::Error::new(RivetError {
		kind,
		meta: meta.and_then(|value| serde_json::value::to_raw_value(&value).ok()),
		message: Some(message),
		actor: None,
	})
}

pub fn action_timed_out(duration: Duration) -> anyhow::Error {
	with_kind(
		RivetErrorKind::Static(&ACTION_TIMED_OUT),
		format!("action timed out after {} ms", duration.as_millis()),
		None,
	)
}

pub fn callback_timed_out(callback: &str, duration: Duration) -> anyhow::Error {
	let duration_ms = u64::try_from(duration.as_millis()).unwrap_or(u64::MAX);
	with_kind(
		RivetErrorKind::Static(&CALLBACK_TIMED_OUT),
		format!("callback `{callback}` timed out after {duration_ms} ms"),
		Some(serde_json::json!({ "callback_name": callback, "duration_ms": duration_ms })),
	)
}

pub fn cut_off_at_shutdown(callback: &str) -> anyhow::Error {
	with_kind(
		RivetErrorKind::Static(&CALLBACK_TIMED_OUT),
		format!("callback `{callback}` was cut off at the shutdown deadline"),
		Some(serde_json::json!({ "callback_name": callback })),
	)
}

pub fn action_not_found(name: String) -> anyhow::Error {
	rivetkit_core::error::action_not_found(name)
}

pub fn stopping() -> anyhow::Error {
	ActorLifecycle::Stopping.build()
}

pub fn not_configured(component: &str) -> anyhow::Error {
	ActorRuntime::NotConfigured {
		component: component.to_owned(),
	}
	.build()
}

pub fn queue_handler_not_found(name: String) -> anyhow::Error {
	ActorRuntime::NotFound {
		resource: "queue handler".to_owned(),
		id: name,
	}
	.build()
}

/// A Rivet error with a caller-chosen group and code.
pub fn rivet_error(group: &str, code: &str, message: impl Into<String>) -> anyhow::Error {
	let message = message.into();
	with_kind(
		RivetErrorKind::Dynamic {
			group: group.to_owned(),
			code: code.to_owned(),
			default_message: message.clone(),
		},
		message,
		None,
	)
}

/// Same as [`rivet_error`], with metadata clients can read.
pub fn rivet_error_with_meta(
	group: &str,
	code: &str,
	message: impl Into<String>,
	meta: Option<serde_json::Value>,
) -> anyhow::Error {
	let message = message.into();
	with_kind(
		RivetErrorKind::Dynamic {
			group: group.to_owned(),
			code: code.to_owned(),
			default_message: message.clone(),
		},
		message,
		meta,
	)
}

/// Bounds `future` by `duration`, failing with the error `on_timeout` builds.
pub async fn bounded<T>(
	duration: Duration,
	future: impl std::future::Future<Output = anyhow::Result<T>>,
	on_timeout: impl FnOnce() -> anyhow::Error,
) -> anyhow::Result<T> {
	match tokio::time::timeout(duration, future).await {
		Ok(result) => result,
		Err(_) => Err(on_timeout()),
	}
}
