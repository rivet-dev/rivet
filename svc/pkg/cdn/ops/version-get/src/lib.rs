use std::collections::HashMap;

use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct GameVersion {
	version_id: Uuid,
	site_id: Uuid,
}

#[derive(Debug, sqlx::FromRow)]
struct GameVersionCustomHeaders {
	version_id: Uuid,
	glob: Vec<u8>,
	priority: i64,
	header_name: String,
	header_value: String,
}

#[derive(Debug)]
struct CustomHeader {
	priority: i32,
	headers: Vec<(String, String)>,
}

#[operation(name = "cdn-version-get")]
async fn handle(
	ctx: OperationContext<cdn::version_get::Request>,
) -> GlobalResult<cdn::version_get::Response> {
	let version_ids = ctx.version_ids.iter().map(|id| **id).collect::<Vec<_>>();

	let crdb = ctx.crdb().await?;
	let (versions, custom_headers) = tokio::try_join!(
		sql_fetch_all!(
			[ctx, GameVersion, &crdb]
			"
				SELECT version_id, site_id
				FROM db_cdn.game_versions
				WHERE version_id = ANY($1)
			",
			&version_ids,
		),
		sql_fetch_all!(
			[ctx, GameVersionCustomHeaders, &crdb]
			"
				SELECT version_id, glob, priority, header_name, header_value
				FROM db_cdn.game_version_custom_headers
				WHERE version_id = ANY($1)
			",
			&version_ids,
		)
	)?;

	let versions = versions
		.into_iter()
		.map(|version| {
			let mut headers_map: HashMap<&[u8], CustomHeader> = HashMap::new();

			// Collect rows into hashmap
			for custom_header in custom_headers
				.iter()
				.filter(|row| row.version_id == version.version_id)
			{
				let entry = headers_map
					.entry(custom_header.glob.as_slice())
					.or_insert_with(|| CustomHeader {
						priority: custom_header.priority as i32,
						headers: Vec::new(),
					});

				entry.headers.push((
					custom_header.header_name.clone(),
					custom_header.header_value.clone(),
				));
			}

			Ok(cdn::version_get::response::Version {
				version_id: Some(version.version_id.into()),
				config: Some(backend::cdn::VersionConfig {
					site_id: Some(version.site_id.into()),
					routes: headers_map
						.into_iter()
						.map(|(glob, custom_header)| {
							Ok(backend::cdn::Route {
								glob: Some(common::Glob::decode(glob)?),
								priority: custom_header.priority,
								middlewares: vec![backend::cdn::Middleware {
									kind: Some(backend::cdn::middleware::Kind::CustomHeaders(
										backend::cdn::CustomHeadersMiddleware {
											headers: custom_header
												.headers
												.into_iter()
												.map(|(name, value)| {
													backend::cdn::custom_headers_middleware::Header {
															name,
															value,
														}
												})
												.collect::<Vec<_>>(),
										},
									)),
								}],
							})
						})
						.collect::<GlobalResult<Vec<_>>>()?,
				}),
				config_meta: Some(backend::cdn::VersionConfigMeta {}),
			})
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok(cdn::version_get::Response { versions })
}
