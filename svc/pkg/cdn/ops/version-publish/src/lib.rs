use itertools::{Either, Itertools};
use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;
use unzip_n::unzip_n;

unzip_n!(4);

struct FlattenedHeader<'a> {
	route: &'a backend::cdn::Route,
	custom_header: &'a backend::cdn::CustomHeadersMiddleware,
}

#[operation(name = "cdn-version-publish")]
async fn handle(
	ctx: OperationContext<cdn::version_publish::Request>,
) -> GlobalResult<cdn::version_publish::Response> {
	let version_id = unwrap_ref!(ctx.version_id);
	let config = unwrap_ref!(ctx.config);
	let _config_ctx = unwrap_ref!(ctx.config_ctx);

	let site_id = unwrap_ref!(config.site_id);

	let crdb = ctx.crdb().await?;
	sqlx::query("INSERT INTO db_cdn.game_versions (version_id, site_id) VALUES ($1, $2)")
		.bind(**version_id)
		.bind(**site_id)
		.execute(&crdb)
		.await?;

	// Funky batch insert. Cannot be joined with the above future because it relies on it to create foreign
	// row first.
	{
		// First, partition out the custom header middlewares from the rest
		let (custom_headers, _other) = config
			.routes
			.iter()
			.flat_map(|route| {
				// Keep a reference of the route next to each middleware for later use
				route
					.middlewares
					.iter()
					.map(move |middleware| Ok((route, unwrap_ref!(middleware.kind))))
			})
			.collect::<GlobalResult<Vec<_>>>()?
			.into_iter()
			.partition_map::<Vec<_>, Vec<_>, _, _, _>(|(route, middleware)| match middleware {
				backend::cdn::middleware::Kind::CustomHeaders(custom_header) => {
					// Remove `Kind` wrapper from middleware so we can partition
					Either::Left(FlattenedHeader {
						route,
						custom_header,
					})
				}
				_ => Either::Right(()),
			});

		let version_ids = vec![**version_id; custom_headers.len()];
		// Unzip each header into a 5 values forming a single row each
		let (globs, priorities, header_names, header_values) = custom_headers
			.iter()
			.flat_map(|flattened_header| {
				flattened_header
					.custom_header
					.headers
					.iter()
					.map(move |header| {
						let glob = unwrap_ref!(flattened_header.route.glob);
						let mut glob_buf = Vec::with_capacity(glob.encoded_len());
						glob.encode(&mut glob_buf)?;

						Ok((
							glob_buf,
							flattened_header.route.priority,
							header.name.as_str(),
							header.value.as_str(),
						))
					})
			})
			.collect::<GlobalResult<Vec<_>>>()?
			.into_iter()
			.unzip_n_vec();

		sqlx::query(indoc!(
			"
			INSERT INTO db_cdn.game_version_custom_headers (
				version_id, glob, priority, header_name, header_value
			)
			SELECT * FROM UNNEST($1, $2, $3, $4, $5)
			"
		))
		.bind(version_ids)
		.bind(globs)
		.bind(priorities)
		.bind(header_names)
		.bind(header_values)
		.execute(&crdb)
		.await?;
	}

	Ok(cdn::version_publish::Response {})
}
