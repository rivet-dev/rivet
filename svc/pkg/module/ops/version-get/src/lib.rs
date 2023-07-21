use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct Version {
	version_id: Uuid,
	module_id: Uuid,
	create_ts: i64,
	major: i64,
	minor: i64,
	patch: i64,
	image_docker_image_tag: Option<String>,
}

#[derive(sqlx::FromRow)]
struct Function {
	version_id: Uuid,
	name: String,
	request_schema: String,
	response_schema: String,
	callable: bool,
}

#[operation(name = "module-version-get")]
pub async fn handle(
	ctx: OperationContext<module::version_get::Request>,
) -> GlobalResult<module::version_get::Response> {
	let version_ids = ctx
		.version_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let versions = sqlx::query_as::<_, Version>(indoc!(
		"
		SELECT
			v.version_id,
			v.module_id,
			v.create_ts,
			v.major,
			v.minor,
			v.patch,
			vid.image_tag AS image_docker_image_tag
		FROM versions AS v
		LEFT JOIN versions_image_docker AS vid ON vid.version_id = v.version_id
		WHERE v.version_id = ANY($1)
		"
	))
	.bind(&version_ids)
	.fetch_all(&ctx.crdb("db-module").await?)
	.await?;

	let functions = sqlx::query_as::<_, Function>(indoc!(
		"
		SELECT
			f.version_id,
			f.name,
			f.request_schema,
			f.response_schema,
			fc.version_id IS NOT NULL AS callable
		FROM functions AS f
		LEFT JOIN functions_callable AS fc ON fc.version_id = f.version_id AND fc.name = f.name
		WHERE f.version_id = ANY($1)
		"
	))
	.bind(version_ids)
	.fetch_all(&ctx.crdb("db-module").await?)
	.await?;

	Ok(module::version_get::Response {
		versions: versions
			.into_iter()
			.map(|version| {
				let functions = functions
					.iter()
					.filter(|function| function.version_id == version.version_id)
					.map(|function| backend::module::Function {
						name: function.name.clone(),
						request_schema: function.request_schema.clone(),
						response_schema: function.response_schema.clone(),
						callable: if function.callable {
							Some(backend::module::function::Callable {})
						} else {
							None
						},
					})
					.collect::<Vec<_>>();

				backend::module::Version {
					version_id: Some(version.version_id.into()),
					module_id: Some(version.module_id.into()),
					create_ts: version.create_ts,
					major: version.major as u64,
					minor: version.minor as u64,
					patch: version.patch as u64,
					functions,
					image: version.image_docker_image_tag.map(|image_tag| {
						backend::module::version::Image::Docker(backend::module::version::Docker {
							image_tag,
						})
					}),
				}
			})
			.collect::<Vec<_>>(),
	})
}
