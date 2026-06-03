use std::borrow::Cow;

use universalpubsub::Subject;

pub const TRACING_CONFIG_SUBJECT: &str = "rivet.debug.tracing.config";

pub struct TracingConfigSubject;

impl std::fmt::Display for TracingConfigSubject {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		TRACING_CONFIG_SUBJECT.fmt(f)
	}
}

impl Subject for TracingConfigSubject {
	fn root<'a>() -> Option<Cow<'a, str>> {
		Some(Cow::Borrowed(TRACING_CONFIG_SUBJECT))
	}

	fn as_str(&self) -> Option<&str> {
		Some(TRACING_CONFIG_SUBJECT)
	}
}
