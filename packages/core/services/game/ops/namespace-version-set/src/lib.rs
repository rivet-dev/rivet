use proto::backend::pkg::*;
use rivet_operation::prelude::*;
use serde_json::json;
use tracing::Instrument;

#[operation(name = "game-namespace-version-set")]
async fn handle(
	ctx: OperationContext<game::namespace_version_set::Request>,
) -> GlobalResult<game::namespace_version_set::Response> {
	let namespace_id = unwrap_ref!(ctx.namespace_id).as_uuid();
	let version_id = unwrap_ref!(ctx.version_id).as_uuid();

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

	{
		let mut tx = ctx.crdb().await?.begin().await?;

		let update_query = sql_execute!(
			[ctx, @tx &mut tx]
			"
			UPDATE db_game.game_namespaces
			SET version_id = $2
			WHERE namespace_id = $1
			",
			namespace_id,
			version_id,
		)
		.await?;
		ensure_eq!(1, update_query.rows_affected(), "invalid namespace id");

		ctx.cache().purge("namespace", [namespace_id]).await?;

		sql_execute!(
			[ctx, @tx &mut tx]
			"
			INSERT INTO db_game.game_namespace_version_history (
				namespace_id, version_id, deploy_ts
			)
			VALUES ($1, $2, $3)
			",
			namespace_id,
			version_id,
			ctx.ts(),
		)
		.await?;

		tx.commit().await?;

		ctx.cache().purge("game", [game_id]).await?;
	}

	// Update idle lobbies in all regions in the background
	// TODO: Write tests for this
	{
		let ctx = ctx.base();
		tokio::task::Builder::new()
			.name("game::namespace_version_set::update_idle_lobbies_for_all_regions")
			.spawn(
				async move {
					// List regions
					let region_res = op!([ctx] region_list_for_game {
						game_ids: vec![game_id.into()],
					})
					.await;
					let region_ids = match region_res {
						Ok(res) => res.region_ids,
						Err(err) => {
							tracing::error!(?err, "failed to fetch region list");
							return;
						}
					};

					// Update lobbies in each region
					for region_id in region_ids {
						let ctx = ctx.base();
						let spawn_res = tokio::task::Builder::new()
							.name("game::namespace_version_set::update_idle_lobbies")
							.spawn(
								async move {
									let res = op!([ctx] mm_lobby_idle_update {
										namespace_id: Some(namespace_id.into()),
										region_id: Some(region_id),
									})
									.await;
									match res {
										Ok(_) => {
											tracing::info!(
												?namespace_id,
												"lobby idle updated successfully"
											);
										}
										Err(err) => {
											tracing::error!(?err, "failed to update idle lobbies");
										}
									}
								}
								.instrument(tracing::info_span!("lobby_idle_update_inner")),
							);
						if let Err(err) = spawn_res {
							tracing::error!(?err, "failed to spawn update_idle_lobbies task");
						}
					}
				}
				.instrument(tracing::info_span!("lobby_idle_update")),
			)?;
	}

	// TODO: Update this to use ns_version_set event
	msg!([ctx] cdn::msg::ns_config_update(namespace_id) {
		namespace_id: Some(namespace_id.into()),
	})
	.await?;

	// Publish updates
	msg!([ctx] game::msg::update(game_id) {
		game_id: game.game_id,
	})
	.await?;
	msg!([ctx] game::msg::ns_version_set_complete(namespace_id) {
		namespace_id: Some(namespace_id.into()),
		version_id: Some(version_id.into()),
	})
	.await?;

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				event_id: Some(Uuid::new_v4().into()),
				name: "game.namespace.version_set".into(),
				user_id: ctx.creator_user_id,
				properties_json: Some(serde_json::to_string(&json!({
					"developer_team_id": developer_team_id,
					"game_id": game_id,
					"namespace_id": namespace_id,
					"version_id": version_id,
				}))?),
				..Default::default()
			}
		],
	})
	.await?;

	Ok(game::namespace_version_set::Response {})
}
