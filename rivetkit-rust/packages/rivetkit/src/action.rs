use serde::Deserialize;
use serde::de::{self, Deserializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Raw;

impl<'de> Deserialize<'de> for Raw {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let _ = de::IgnoredAny::deserialize(deserializer)?;
		Err(de::Error::custom(
			"rivetkit::action::Raw cannot be deserialized; use Action::raw_args() or Action::decode_as(...) instead",
		))
	}
}

#[cfg(test)]
mod tests {
	use serde::Deserialize;
	use serde::de::value::{Error as ValueError, UnitDeserializer};

	use super::Raw;

	#[test]
	fn raw_deserialize_fails_with_guidance() {
		let err = Raw::deserialize(UnitDeserializer::<ValueError>::new())
			.expect_err("Raw should refuse serde decoding");

		let message = err.to_string();
		assert!(message.contains("Action::raw_args()"));
		assert!(message.contains("Action::decode_as"));
	}
}
