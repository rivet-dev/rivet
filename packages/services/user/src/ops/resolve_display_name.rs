use chirp_workflow::prelude::*;

#[derive(Debug, Default)]
pub struct Input {
	pub display_name: String,
}

#[derive(Debug)]
pub struct Output {
	pub user_id: Option<Uuid>,
}

#[operation]
pub async fn resolve_display_name(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	let user_id = sql_fetch_optional!(
		[ctx, (Uuid,)]
		"
		SELECT user_id
		FROM users
		WHERE display_name = $1
		",
		&input.display_name,
	)
	.await?
	.map(|x| x.0);

	Ok(Output { user_id })
}
