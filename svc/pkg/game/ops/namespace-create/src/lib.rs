use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "game-namespace-create")]
async fn handle(
	ctx: OperationContext<game::namespace_create::Request>,
) -> GlobalResult<game::namespace_create::Response> {
	let req_game_id = unwrap_ref!(ctx.game_id);
	let game_id = req_game_id.as_uuid();
	let version_id = unwrap_ref!(ctx.version_id).as_uuid();

	// Validate namespace
	let validation_res = op!([ctx] game_namespace_validate {
		game_id: Some(*req_game_id),
		name_id: ctx.name_id.to_owned(),
		display_name: ctx.display_name.to_owned(),
	})
	.await?;
	if !validation_res.errors.is_empty() {
		tracing::warn!(errors = ?validation_res.errors, "validation errors");

		let readable_errors = validation_res
			.errors
			.iter()
			.map(|err| err.path.join("."))
			.collect::<Vec<_>>()
			.join(", ");
		bail_with!(VALIDATION_ERROR, error = readable_errors);
	}

	let namespace_id = Uuid::new_v4();

	sqlx::query(indoc!(
		"
		INSERT INTO db_game.game_namespaces (namespace_id, game_id, create_ts, display_name, version_id, name_id)
		VALUES ($1, $2, $3, $4, $5, $6)
		"
	))
	.bind(namespace_id)
	.bind(game_id)
	.bind(ctx.ts())
	.bind(&ctx.display_name)
	.bind(version_id)
	.bind(&ctx.name_id)
	.execute(&ctx.crdb().await?)
	.await?;

	msg!([ctx] cdn::msg::ns_config_update(namespace_id) {
		namespace_id: Some(namespace_id.into()),
	})
	.await?;

	// Publish updates
	msg!([ctx] game::msg::update(game_id) {
		game_id: Some(game_id.into()),
	})
	.await?;
	msg!([ctx] game::msg::ns_version_set_complete(namespace_id) {
		namespace_id: Some(namespace_id.into()),
		version_id: Some(version_id.into()),
	})
	.await?;

	Ok(game::namespace_create::Response {
		namespace_id: Some(namespace_id.into()),
	})
}
