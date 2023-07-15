pub mod api;
pub mod auth;
pub mod chat;
pub mod cloud;
pub mod group;
pub mod identity;
pub mod kv;
pub mod party;
pub mod portal;
pub mod user;

// Reimplement conversions for ease of use in this module
mod num {
	use crate::ApiTryFrom;

	impl ApiTryFrom<i32> for u32 {
		type Error = std::num::TryFromIntError;

		fn try_from(v: i32) -> Result<Self, Self::Error> {
			std::convert::TryInto::try_into(v)
		}
	}

	impl ApiTryFrom<u32> for i32 {
		type Error = std::num::TryFromIntError;

		fn try_from(v: u32) -> Result<Self, Self::Error> {
			std::convert::TryInto::try_into(v)
		}
	}

	impl ApiTryFrom<u64> for i64 {
		type Error = std::num::TryFromIntError;

		fn try_from(v: u64) -> Result<Self, Self::Error> {
			std::convert::TryInto::try_into(v)
		}
	}

	impl ApiTryFrom<i64> for u64 {
		type Error = std::num::TryFromIntError;

		fn try_from(v: i64) -> Result<Self, Self::Error> {
			std::convert::TryInto::try_into(v)
		}
	}

	impl ApiTryFrom<u64> for i32 {
		type Error = std::num::TryFromIntError;

		fn try_from(v: u64) -> Result<Self, Self::Error> {
			std::convert::TryInto::try_into(v)
		}
	}

	impl ApiTryFrom<i32> for u64 {
		type Error = std::num::TryFromIntError;

		fn try_from(v: i32) -> Result<Self, Self::Error> {
			std::convert::TryInto::try_into(v)
		}
	}
}
