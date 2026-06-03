use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use universalpubsub::Subject;

pub const PROFILE_CONFIG_SUBJECT: &str = "rivet.debug.profile.config";

#[derive(Serialize, Deserialize)]
pub struct SetProfileConfigMessage {
	pub enabled: bool,
	/// Overrides the configured sampling frequency (Hz) for this run. Falls back to the
	/// `pyroscope.sample_rate` config when absent.
	#[serde(default)]
	pub sample_rate: Option<u32>,
}

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
