use proto::backend;
use rivet_api::models;
use rivet_operation::prelude::*;

use crate::{ApiTryFrom, ApiTryInto};

impl ApiTryFrom<models::CloudVersionCdnConfig> for backend::cdn::VersionConfig {
	type Error = GlobalError;

	fn try_from(value: models::CloudVersionCdnConfig) -> GlobalResult<Self> {
		Ok(backend::cdn::VersionConfig {
			site_id: value.site_id.map(Into::into),
			routes: value
				.routes
				.unwrap_or_default()
				.into_iter()
				.map(ApiTryInto::try_into)
				.collect::<GlobalResult<Vec<_>>>()?,
		})
	}
}

impl ApiTryFrom<models::CloudVersionCdnRoute> for backend::cdn::Route {
	type Error = GlobalError;

	fn try_from(value: models::CloudVersionCdnRoute) -> GlobalResult<Self> {
		ensure!(value.priority >= 0);

		let glob = match util::glob::Glob::parse(&value.glob) {
			Ok(glob) => glob,
			Err(err) if err.is(formatted_error::code::GLOB_INVALID) => {
				// Replace invalid glob with an empty glob (also invalid) which will be caught
				// by game-version-validate
				tracing::warn!(?err, "invalid glob");
				util::glob::Glob::new(Vec::new())
			}
			Err(err) => return Err(err),
		};

		Ok(backend::cdn::Route {
			glob: Some(Into::into(glob)),
			priority: value.priority,
			middlewares: value
				.middlewares
				.into_iter()
				.map(ApiTryInto::try_into)
				.collect::<GlobalResult<Vec<_>>>()?,
		})
	}
}

impl ApiTryFrom<models::CloudVersionCdnMiddleware> for backend::cdn::Middleware {
	type Error = GlobalError;

	fn try_from(value: models::CloudVersionCdnMiddleware) -> GlobalResult<Self> {
		if let Some(custom_headers) = value.kind.custom_headers {
			Ok(backend::cdn::Middleware {
				kind: Some(backend::cdn::middleware::Kind::CustomHeaders(
					(*custom_headers).try_into()?,
				)),
			})
		} else {
			Ok(backend::cdn::Middleware { kind: None })
		}
	}
}

impl ApiTryFrom<models::CloudVersionCdnCustomHeadersMiddleware>
	for backend::cdn::CustomHeadersMiddleware
{
	type Error = GlobalError;

	fn try_from(value: models::CloudVersionCdnCustomHeadersMiddleware) -> GlobalResult<Self> {
		Ok(backend::cdn::CustomHeadersMiddleware {
			headers: value
				.headers
				.into_iter()
				.map(ApiTryInto::try_into)
				.collect::<GlobalResult<Vec<_>>>()?,
		})
	}
}

impl ApiTryFrom<models::CloudVersionCdnHeader> for backend::cdn::custom_headers_middleware::Header {
	type Error = GlobalError;

	fn try_from(value: models::CloudVersionCdnHeader) -> GlobalResult<Self> {
		Ok(backend::cdn::custom_headers_middleware::Header {
			name: value.name,
			value: value.value,
		})
	}
}

impl ApiTryFrom<backend::cdn::VersionConfig> for models::CloudVersionCdnConfig {
	type Error = GlobalError;

	fn try_from(value: backend::cdn::VersionConfig) -> GlobalResult<Self> {
		let site_id = unwrap_ref!(value.site_id).as_uuid();

		Ok(models::CloudVersionCdnConfig {
			build_command: None,
			build_output: None,
			site_id: Some(site_id),
			routes: Some(
				value
					.routes
					.into_iter()
					.map(ApiTryInto::try_into)
					.collect::<GlobalResult<Vec<_>>>()?,
			),
		})
	}
}

impl ApiTryFrom<backend::cdn::Route> for models::CloudVersionCdnRoute {
	type Error = GlobalError;

	fn try_from(value: backend::cdn::Route) -> GlobalResult<Self> {
		Ok(models::CloudVersionCdnRoute {
			glob: std::convert::TryInto::<util::glob::Glob>::try_into(unwrap!(
				value.glob.clone()
			))?
			.to_string(),
			priority: value.priority,
			middlewares: value
				.middlewares
				.into_iter()
				.map(ApiTryInto::try_into)
				.collect::<GlobalResult<Vec<_>>>()?,
		})
	}
}

impl ApiTryFrom<backend::cdn::Middleware> for models::CloudVersionCdnMiddleware {
	type Error = GlobalError;

	fn try_from(value: backend::cdn::Middleware) -> GlobalResult<Self> {
		let kind = unwrap_ref!(value.kind).clone();

		match kind {
			backend::cdn::middleware::Kind::CustomHeaders(custom_headers) => {
				Ok(models::CloudVersionCdnMiddleware {
					kind: Box::new(models::CloudVersionCdnMiddlewareKind {
						custom_headers: Some(Box::new(custom_headers.try_into()?)),
					}),
				})
			}
		}
	}
}

impl ApiTryFrom<backend::cdn::CustomHeadersMiddleware>
	for models::CloudVersionCdnCustomHeadersMiddleware
{
	type Error = GlobalError;

	fn try_from(value: backend::cdn::CustomHeadersMiddleware) -> GlobalResult<Self> {
		Ok(models::CloudVersionCdnCustomHeadersMiddleware {
			headers: value
				.headers
				.into_iter()
				.map(ApiTryInto::try_into)
				.collect::<GlobalResult<Vec<_>>>()?,
		})
	}
}

impl ApiTryFrom<backend::cdn::custom_headers_middleware::Header> for models::CloudVersionCdnHeader {
	type Error = GlobalError;

	fn try_from(value: backend::cdn::custom_headers_middleware::Header) -> GlobalResult<Self> {
		Ok(models::CloudVersionCdnHeader {
			name: value.name.clone(),
			value: value.value,
		})
	}
}
