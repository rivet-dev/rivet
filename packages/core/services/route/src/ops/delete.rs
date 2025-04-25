use chirp_workflow::prelude::*;
use formatted_error::code::ROUTE_NOT_FOUND;

#[derive(Debug)]
pub struct Input {
	pub namespace_id: Uuid,
	pub name_id: String,
}

#[derive(Debug)]
pub struct Output {}

#[operation]
pub async fn delete(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	// Get a database pool client
	let pool = ctx.crdb().await?;

	// Find the route ID by namespace_id and name_id
	let route_id = sql_fetch_optional!(
	[ctx, (Uuid,)]
		"
			SELECT route_id FROM db_route.routes 
			WHERE namespace_id = $1 AND name_id = $2 AND delete_ts IS NULL
			",
		input.namespace_id,
		&input.name_id
	)
	.await?
	.map(|(id,)| id)
	.ok_or_else(|| err_code!(ROUTE_NOT_FOUND))?;

	// Perform a soft delete by setting delete_ts instead of actually deleting the record
	let now = ctx.ts();
	sql_execute!(
		[ctx]
		"
			UPDATE db_route.routes
			SET delete_ts = $1
			WHERE route_id = $2
			AND delete_ts IS NULL
			",
		now,
		route_id
	)
	.await?;

	Ok(Output {})
}
