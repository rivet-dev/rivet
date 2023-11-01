use proto::backend::pkg::*;
use rivet_operation::prelude::*;
use serde_json::json;

#[operation(name = "cloud-namespace-create")]
async fn handle(
	ctx: OperationContext<cloud::namespace_create::Request>,
) -> GlobalResult<cloud::namespace_create::Response> {
	let namespace_id = unwrap_ref!(ctx.namespace_id).as_uuid();

	let ns_res = op!([ctx] game_namespace_get {
		namespace_ids: vec![namespace_id.into()],
	})
	.await?;
	let ns = unwrap!(
		ns_res.namespaces.first(),
		"game namespace not found for cloud namespace"
	);
	let game_id = unwrap_ref!(ns.game_id).as_uuid();

	let game_res = op!([ctx] game_get {
		game_ids: vec![game_id.into()],
	})
	.await?;
	let game = unwrap!(game_res.games.first());
	let developer_team_id = unwrap_ref!(game.developer_team_id).as_uuid();

	tokio::try_join!(
		op!([ctx] cdn_namespace_create {
			namespace_id: Some(namespace_id.into()),
		}),
		op!([ctx] mm_config_namespace_create {
			namespace_id: Some(namespace_id.into()),
		}),
		op!([ctx] kv_config_namespace_create {
			namespace_id: Some(namespace_id.into()),
		}),
		op!([ctx] identity_config_namespace_create {
			namespace_id: Some(namespace_id.into()),
		}),
	)?;

	sql_query!(
		[ctx]
		"
		INSERT INTO db_cloud.game_namespaces (namespace_id)
		VALUES ($1)
		",
		namespace_id,
	)
	.await?;

	// Send game update
	msg!([ctx] game::msg::update(game_id) {
		game_id: Some(game_id.into()),
	})
	.await?;

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				name: "game.namespace.create".into(),
				user_id: ctx.creator_user_id,
				properties_json: Some(serde_json::to_string(&json!({
					"developer_team_id": developer_team_id,
					"game_id": game_id,
					"namespace_id": namespace_id,
					"display_name": ns.display_name,
					"name_id": ns.name_id,
				}))?),
				..Default::default()
			}
		],
	})
	.await?;

	Ok(cloud::namespace_create::Response {})
}
