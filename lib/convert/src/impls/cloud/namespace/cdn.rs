use proto::backend;
use rivet_api::models;
use rivet_operation::prelude::*;

use crate::ApiFrom;

impl ApiFrom<backend::cdn::namespace_config::AuthType> for models::CloudCdnAuthType {
	fn api_from(value: backend::cdn::namespace_config::AuthType) -> Self {
		match value {
			backend::cdn::namespace_config::AuthType::None => models::CloudCdnAuthType::None,
			backend::cdn::namespace_config::AuthType::Basic => models::CloudCdnAuthType::Basic,
		}
	}
}

impl ApiFrom<models::CloudCdnAuthType> for backend::cdn::namespace_config::AuthType {
	fn api_from(value: models::CloudCdnAuthType) -> Self {
		match value {
			models::CloudCdnAuthType::None => backend::cdn::namespace_config::AuthType::None,
			models::CloudCdnAuthType::Basic => backend::cdn::namespace_config::AuthType::Basic,
		}
	}
}

impl ApiFrom<backend::cdn::namespace_config::AuthUser> for models::CloudCdnNamespaceAuthUser {
	fn api_from(value: backend::cdn::namespace_config::AuthUser) -> Self {
		models::CloudCdnNamespaceAuthUser { user: value.user }
	}
}
