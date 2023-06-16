use proto::backend;
use rivet_api::models;
use rivet_operation::prelude::*;

use crate::{ApiTryFrom, ApiTryInto};

impl ApiTryFrom<models::CloudVersionIdentityConfig> for backend::identity::VersionConfig {
	type Error = GlobalError;

	fn try_from(value: models::CloudVersionIdentityConfig) -> GlobalResult<Self> {
		Ok(backend::identity::VersionConfig {
			custom_display_names: value
				.custom_display_names
				.into_iter()
				.flat_map(|x| x.into_iter().map(ApiTryInto::try_into))
				.chain(value.display_names.into_iter().flat_map(|x| {
					x.into_iter().map(|display_name| {
						GlobalResult::Ok(backend::identity::CustomDisplayName { display_name })
					})
				}))
				.collect::<GlobalResult<Vec<_>>>()?,
			custom_avatars: value
				.custom_avatars
				.into_iter()
				.flat_map(|x| x.into_iter().map(ApiTryInto::try_into))
				.chain(value.avatars.into_iter().flat_map(|x| {
					x.into_iter().map(|upload_id| {
						GlobalResult::Ok(backend::identity::CustomAvatar {
							upload_id: Some(upload_id.into()),
						})
					})
				}))
				.collect::<GlobalResult<Vec<_>>>()?,
		})
	}
}

impl ApiTryFrom<models::CloudVersionIdentityCustomDisplayName>
	for backend::identity::CustomDisplayName
{
	type Error = GlobalError;

	fn try_from(value: models::CloudVersionIdentityCustomDisplayName) -> GlobalResult<Self> {
		Ok(backend::identity::CustomDisplayName {
			display_name: value.display_name,
		})
	}
}

impl ApiTryFrom<models::CloudVersionIdentityCustomAvatar> for backend::identity::CustomAvatar {
	type Error = GlobalError;

	fn try_from(value: models::CloudVersionIdentityCustomAvatar) -> GlobalResult<Self> {
		Ok(backend::identity::CustomAvatar {
			upload_id: Some(value.upload_id.into()),
		})
	}
}

impl ApiTryFrom<backend::identity::VersionConfig> for models::CloudVersionIdentityConfig {
	type Error = GlobalError;

	fn try_from(value: backend::identity::VersionConfig) -> GlobalResult<Self> {
		Ok(models::CloudVersionIdentityConfig {
			display_names: Some(
				value
					.custom_display_names
					.iter()
					.cloned()
					.map(|x| x.display_name)
					.collect(),
			),
			avatars: Some(
				value
					.custom_avatars
					.iter()
					.cloned()
					.flat_map(|x| x.upload_id)
					.map(|x| x.as_uuid())
					.collect(),
			),
			custom_display_names: Some(
				value
					.custom_display_names
					.into_iter()
					.map(ApiTryInto::try_into)
					.collect::<Result<Vec<_>, _>>()?,
			),
			custom_avatars: Some(
				value
					.custom_avatars
					.into_iter()
					.map(ApiTryInto::try_into)
					.collect::<Result<Vec<_>, _>>()?,
			),
		})
	}
}

impl ApiTryFrom<backend::identity::CustomDisplayName>
	for models::CloudVersionIdentityCustomDisplayName
{
	type Error = GlobalError;

	fn try_from(value: backend::identity::CustomDisplayName) -> GlobalResult<Self> {
		Ok(models::CloudVersionIdentityCustomDisplayName {
			display_name: value.display_name,
		})
	}
}

impl ApiTryFrom<backend::identity::CustomAvatar> for models::CloudVersionIdentityCustomAvatar {
	type Error = GlobalError;

	fn try_from(value: backend::identity::CustomAvatar) -> GlobalResult<Self> {
		Ok(models::CloudVersionIdentityCustomAvatar {
			upload_id: internal_unwrap_owned!(value.upload_id).as_uuid(),
		})
	}
}
