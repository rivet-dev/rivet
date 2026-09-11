use std::fmt;

pub const HEADER_RIVET_RAY_ID: &str = "x-rivet-ray-id";
pub const RAY_BAGGAGE_KEY: &str = "rivet.ray.id";

const MAX_LEN: usize = 128;

/// A bounded correlation token carried between RivetKit requests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RayId(String);

impl RayId {
	pub fn parse(value: impl Into<String>) -> Result<Self, InvalidRayId> {
		let value = value.into();
		if value.is_empty()
			|| value.len() > MAX_LEN
			|| !value
				.bytes()
				.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
		{
			return Err(InvalidRayId);
		}
		Ok(Self(value))
	}

	pub fn as_str(&self) -> &str {
		&self.0
	}

	pub fn into_string(self) -> String {
		self.0
	}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidRayId;

impl fmt::Display for InvalidRayId {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter.write_str("ray ID must be 1 to 128 characters of [A-Za-z0-9_-]")
	}
}

impl std::error::Error for InvalidRayId {}
