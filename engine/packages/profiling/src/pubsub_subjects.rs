use std::borrow::Cow;

use universalpubsub::Subject;

pub const PROFILE_CONFIG_SUBJECT: &str = "rivet.debug.profile.config";

pub struct ProfileConfigSubject;

impl std::fmt::Display for ProfileConfigSubject {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		PROFILE_CONFIG_SUBJECT.fmt(f)
	}
}

impl Subject for ProfileConfigSubject {
	fn root<'a>() -> Option<Cow<'a, str>> {
		Some(Cow::Borrowed(PROFILE_CONFIG_SUBJECT))
	}

	fn as_str(&self) -> Option<&str> {
		Some(PROFILE_CONFIG_SUBJECT)
	}
}
