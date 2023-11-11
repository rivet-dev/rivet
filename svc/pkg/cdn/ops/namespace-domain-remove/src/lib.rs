use proto::backend::pkg::*;
use rivet_operation::prelude::*;
use serde_json::json;

#[operation(name = "cdn-namespace-domain-remove")]
async fn handle(
	ctx: OperationContext<cdn::namespace_domain_remove::Request>,
) -> GlobalResult<cdn::namespace_domain_remove::Response> {
	ensure!(util::feature::cf_custom_hostname());

	let namespace_id = unwrap_ref!(ctx.namespace_id).as_uuid();

	let game_res = op!([ctx] game_resolve_namespace_id {
		namespace_ids: vec![namespace_id.into()],
	})
	.await?;
	let game = unwrap!(game_res.games.first());
	let game_id = unwrap_ref!(game.game_id).as_uuid();

	let game_res = op!([ctx] game_get {
		game_ids: vec![game_id.into()],
	})
	.await?;
	let game = unwrap!(game_res.games.first());
	let developer_team_id = unwrap_ref!(game.developer_team_id).as_uuid();

	sql_execute!(
		[ctx]
		"DELETE FROM db_cdn.game_namespace_domains WHERE namespace_id = $1 AND domain = $2",
		namespace_id,
		&ctx.domain,
	)
	.await?;

	// Remove cloudflare hostname
	msg!([ctx] cf_custom_hostname::msg::delete(namespace_id, &ctx.domain) -> cf_custom_hostname::msg::delete_complete {
		namespace_id: ctx.namespace_id,
		hostname: ctx.domain.clone(),
	}).await?;

	msg!([ctx] cdn::msg::ns_config_update(namespace_id) {
		namespace_id: ctx.namespace_id,
	})
	.await?;

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				name: "cdn.domain.remove".into(),
				properties_json: Some(serde_json::to_string(&json!({
					"developer_team_id": developer_team_id,
					"game_id": game_id,
					"namespace_id": namespace_id,
					"domain": ctx.domain,
				}))?),
				..Default::default()
			}
		],
	})
	.await?;

	Ok(cdn::namespace_domain_remove::Response {})
}
