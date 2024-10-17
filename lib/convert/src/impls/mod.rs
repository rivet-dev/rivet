use proto::common;
use rivet_api::models as new_models;
use rivet_group_server::models;
use rivet_operation::prelude::*;

use crate::ApiFrom;

pub mod admin;
pub mod api;
pub mod cloud;
pub mod group;
pub mod identity;
pub mod kv;
pub mod portal;

impl ApiFrom<common::ValidationError> for new_models::ValidationError {
	fn api_from(value: common::ValidationError) -> new_models::ValidationError {
		new_models::ValidationError { path: value.path }
	}
}

impl ApiFrom<common::ValidationError> for models::ValidationError {
	fn api_from(value: common::ValidationError) -> models::ValidationError {
		models::ValidationError { path: value.path }
	}
}

// Reimplement conversions for ease of use in this module
mod num {
	use crate::ApiTryFrom;

	impl ApiTryFrom<i32> for u32 {
		type Error = std::num::TryFromIntError;

		fn api_try_from(v: i32) -> Result<Self, Self::Error> {
			std::convert::TryInto::try_into(v)
		}
	}

	impl ApiTryFrom<u32> for i32 {
		type Error = std::num::TryFromIntError;

		fn api_try_from(v: u32) -> Result<Self, Self::Error> {
			std::convert::TryInto::try_into(v)
		}
	}

	impl ApiTryFrom<u64> for i64 {
		type Error = std::num::TryFromIntError;

		fn api_try_from(v: u64) -> Result<Self, Self::Error> {
			std::convert::TryInto::try_into(v)
		}
	}

	impl ApiTryFrom<i64> for u64 {
		type Error = std::num::TryFromIntError;

		fn api_try_from(v: i64) -> Result<Self, Self::Error> {
			std::convert::TryInto::try_into(v)
		}
	}

	impl ApiTryFrom<u64> for i32 {
		type Error = std::num::TryFromIntError;

		fn api_try_from(v: u64) -> Result<Self, Self::Error> {
			std::convert::TryInto::try_into(v)
		}
	}

	impl ApiTryFrom<i32> for u64 {
		type Error = std::num::TryFromIntError;

		fn api_try_from(v: i32) -> Result<Self, Self::Error> {
			std::convert::TryInto::try_into(v)
		}
	}
}
