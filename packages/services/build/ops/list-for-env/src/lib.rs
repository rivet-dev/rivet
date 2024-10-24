use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "build-list-for-env")]
async fn handle(
	ctx: OperationContext<build::list_for_env::Request>,
) -> GlobalResult<build::list_for_env::Response> {
	let env_id = unwrap_ref!(ctx.env_id).as_uuid();

	let build_ids = sql_fetch_all!(
		[ctx, (Uuid,)]
		"
		SELECT
			build_id
		FROM
			db_build.builds
		WHERE
			env_id = $1
		AND
			tags @> $2
		",
		env_id,
		serde_json::to_value(&ctx.tags)?
	)
	.await?
	.into_iter()
	.map(|(id,)| common::Uuid::from(id))
	.collect::<Vec<_>>();

	Ok(build::list_for_env::Response { build_ids })
}
