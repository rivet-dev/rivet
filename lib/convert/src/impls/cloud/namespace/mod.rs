use proto::backend;
use rivet_api::models;
use rivet_operation::prelude::*;

use crate::ApiTryFrom;

pub mod cdn;
pub mod identity;
pub mod kv;
pub mod matchmaker;

impl ApiTryFrom<backend::game::Namespace> for models::CloudNamespaceSummary {
	type Error = GlobalError;

	fn api_try_from(value: backend::game::Namespace) -> GlobalResult<Self> {
		Ok(models::CloudNamespaceSummary {
			namespace_id: unwrap!(value.namespace_id).as_uuid(),
			create_ts: util::timestamp::to_string(value.create_ts)?,
			display_name: value.display_name,
			version_id: unwrap!(value.version_id).as_uuid(),
			name_id: value.name_id,
		})
	}
}
