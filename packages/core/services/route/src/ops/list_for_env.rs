use chirp_workflow::prelude::*;

#[derive(Debug)]
pub struct Input {
	pub namespace_id: Uuid,
}

#[derive(Debug)]
pub struct Output {
	pub route_ids: Vec<Uuid>,
}

#[operation]
pub async fn list_for_env(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	let route_ids = sql_fetch_all!(
		[ctx, (Uuid,)]
		"
        SELECT
            route_id
        FROM
            db_route.routes
        WHERE
            namespace_id = $1
        AND
            delete_ts IS NULL
        ORDER BY
            create_ts DESC
        ",
		input.namespace_id
	)
	.await?
	.into_iter()
	.map(|(id,)| id)
	.collect::<Vec<_>>();

	Ok(Output { route_ids })
}
