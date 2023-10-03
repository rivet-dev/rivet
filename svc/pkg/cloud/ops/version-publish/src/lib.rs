use proto::backend::pkg::*;
use rivet_operation::prelude::*;
use serde_json::json;

#[operation(name = "cloud-version-publish")]
async fn handle(
	ctx: OperationContext<cloud::version_publish::Request>,
) -> GlobalResult<cloud::version_publish::Response> {
	let req_game_id = internal_unwrap!(ctx.game_id);
	let game_id = req_game_id.as_uuid();
	let config = internal_unwrap!(ctx.config);

	let game_res = op!([ctx] game_get {
		game_ids: vec![game_id.into()],
	})
	.await?;
	let game = internal_unwrap_owned!(game_res.games.first());
	let developer_team_id = internal_unwrap!(game.developer_team_id).as_uuid();

	// Validate version
	let validation_res = op!([ctx] game_version_validate {
		game_id: Some(*req_game_id),
		display_name: ctx.display_name.to_owned(),
		config: Some(config.clone())
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
		panic_with!(VALIDATION_ERROR, error = readable_errors);
	}

	// Prepare the version
	let cdn_config_ctx = if let Some(cdn_config) = &config.cdn {
		let prepare_res = op!([ctx] cdn_version_prepare {
			game_id: Some(game_id.into()),
			config: Some(cdn_config.clone()),
		})
		.await?;

		Some(internal_unwrap!(prepare_res.config_ctx).clone())
	} else {
		None
	};
	let mm_config_ctx = if let Some(mm_config) = &config.matchmaker {
		let prepare_res = op!([ctx] mm_config_version_prepare {
			game_id: Some(game_id.into()),
			config: Some(mm_config.clone()),
		})
		.await?;

		Some(internal_unwrap!(prepare_res.config_ctx).clone())
	} else {
		None
	};
	let kv_config_ctx = if let Some(kv_config) = &config.kv {
		let prepare_res = op!([ctx] kv_config_version_prepare {
			game_id: Some(game_id.into()),
			config: Some(kv_config.clone()),
		})
		.await?;

		Some(internal_unwrap!(prepare_res.config_ctx).clone())
	} else {
		None
	};
	let identity_config_ctx = if let Some(identity_config) = &config.identity {
		let prepare_res = op!([ctx] identity_config_version_prepare {
			game_id: Some(game_id.into()),
			config: Some(identity_config.clone()),
		})
		.await?;

		Some(internal_unwrap!(prepare_res.config_ctx).clone())
	} else {
		None
	};
	let module_config_ctx = if let Some(module_config) = &config.module {
		let prepare_res = op!([ctx] module_game_version_prepare {
			game_id: Some(game_id.into()),
			config: Some(module_config.clone()),
		})
		.await?;

		Some(internal_unwrap!(prepare_res.config_ctx).clone())
	} else {
		None
	};

	// Create the game version
	let version_create_res = op!([ctx] game_version_create {
		game_id: Some(game_id.into()),
		display_name: ctx.display_name.clone(),
	})
	.await?;
	let version_id = internal_unwrap!(version_create_res.version_id).as_uuid();

	// Create the cloud version
	sqlx::query(indoc!(
		"
		INSERT INTO db_cloud.game_versions (version_id)
		VALUES ($1)
	"
	))
	.bind(version_id)
	.execute(&ctx.crdb().await?)
	.await?;

	// Create the cloud versions
	if let (Some(cdn_config), Some(cdn_config_ctx)) = (&config.cdn, &cdn_config_ctx) {
		op!([ctx] cdn_version_publish {
			version_id: Some(version_id.into()),
			config: Some(cdn_config.clone()),
			config_ctx: Some((*cdn_config_ctx).clone()),
		})
		.await?;
	}
	if let (Some(mm_config), Some(mm_config_ctx)) = (&config.matchmaker, &mm_config_ctx) {
		op!([ctx] mm_config_version_publish {
			version_id: Some(version_id.into()),
			config: Some(mm_config.clone()),
			config_ctx: Some((*mm_config_ctx).clone()),
		})
		.await?;
	}
	if let (Some(kv_config), Some(kv_config_ctx)) = (&config.kv, &kv_config_ctx) {
		op!([ctx] kv_config_version_publish {
			version_id: Some(version_id.into()),
			config: Some(kv_config.clone()),
			config_ctx: Some((*kv_config_ctx).clone()),
		})
		.await?;
	}
	if let (Some(identity_config), Some(identity_config_ctx)) =
		(&config.identity, &identity_config_ctx)
	{
		op!([ctx] identity_config_version_publish {
			version_id: Some(version_id.into()),
			config: Some(identity_config.clone()),
			config_ctx: Some((*identity_config_ctx).clone()),
		})
		.await?;
	}
	if let (Some(module_config), Some(module_config_ctx)) = (&config.module, &module_config_ctx) {
		op!([ctx] module_game_version_publish {
			version_id: Some(version_id.into()),
			config: Some(module_config.clone()),
			config_ctx: Some((*module_config_ctx).clone()),
		})
		.await?;
	}

	// Send game update
	{
		msg!([ctx] game::msg::update(game_id) {
			game_id: ctx.game_id,
		})
		.await?;
	}

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				name: "game.version.publish".into(),
				user_id: ctx.creator_user_id,
				properties_json: Some(serde_json::to_string(&json!({
					"developer_team_id": developer_team_id,
					"game_id": game_id,
					"version_id": version_id,
					"game_id": game_id,
					"developer_team_id": developer_team_id,
					"display_name": ctx.display_name,
				}))?),
				..Default::default()
			}
		],
	})
	.await?;

	Ok(cloud::version_publish::Response {
		version_id: Some(version_id.into()),
	})
}
