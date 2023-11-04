use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "mm-config-namespace-config-set")]
async fn handle(
	ctx: OperationContext<mm_config::namespace_config_set::Request>,
) -> GlobalResult<mm_config::namespace_config_set::Response> {
	let namespace_id = unwrap_ref!(ctx.namespace_id).as_uuid();

	// Validate game
	let validation_res = op!([ctx] mm_config_namespace_config_validate {
		namespace_id: ctx.namespace_id,
		lobby_count_max: ctx.lobby_count_max,
		max_players_per_client: ctx.max_players_per_client,
		max_players_per_client_vpn: ctx.max_players_per_client_vpn,
		max_players_per_client_proxy: ctx.max_players_per_client_proxy,
		max_players_per_client_tor: ctx.max_players_per_client_tor,
		max_players_per_client_hosting: ctx.max_players_per_client_hosting,
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

	sql_execute!(
		[ctx]
		"
		UPDATE db_mm_config.game_namespaces
		SET 
			lobby_count_max = $2,
			max_players_per_client = $3,
			max_players_per_client_vpn = $4,
			max_players_per_client_proxy = $5,
			max_players_per_client_tor = $6,
			max_players_per_client_hosting = $7
		WHERE namespace_id = $1
		",
		namespace_id,
		ctx.lobby_count_max as i64,
		ctx.max_players_per_client as i64,
		ctx.max_players_per_client_vpn as i64,
		ctx.max_players_per_client_proxy as i64,
		ctx.max_players_per_client_tor as i64,
		ctx.max_players_per_client_hosting as i64,
	)
	.await?;

	Ok(mm_config::namespace_config_set::Response {})
}
