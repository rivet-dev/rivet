use chirp_workflow::prelude::*;
use rivet_operation::prelude::proto::backend::{self, pkg::*};
use util::dev_defaults;

#[tracing::instrument(skip_all)]
pub async fn start(config: rivet_config::Config, pools: rivet_pools::Pools) -> GlobalResult<()> {
	let client =
		chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("cloud-default-create");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = StandaloneCtx::new(
		chirp_workflow::compat::db_from_pools(&pools).await?,
		config,
		rivet_connection::Connection::new(client, pools, cache),
		"cloud-default-create",
	)
	.await?;

	ensure!(
		ctx.config().server()?.rivet.auth.access_kind
			== rivet_config::config::rivet::AccessKind::Development,
		"access kind must be development"
	);

	// Create user
	let user_resolve_res = ctx
		.op(::user::ops::resolve_display_name::Input {
			display_name: dev_defaults::USER_NAME.into(),
		})
		.await?;
	let user_id = if let Some(user_id) = user_resolve_res.user_id {
		user_id
	} else {
		let user_id = Uuid::new_v4();
		msg!([ctx] user::msg::create(user_id) -> user::msg::create_complete {
			user_id: Some(user_id.into()),
			namespace_id: None,
			display_name: Some(dev_defaults::USER_NAME.into()),
		})
		.await?;

		user_id
	};

	// Create team
	let team_resolve_res = op!([ctx] team_resolve_display_name {
		display_names: vec![dev_defaults::TEAM_NAME.into()],
	})
	.await?;
	let team_id = if let Some(team) = team_resolve_res.teams.first() {
		unwrap!(team.team_id).as_uuid()
	} else {
		let team_id = Uuid::new_v4();
		msg!([ctx] team::msg::create(team_id) -> team::msg::create_complete {
			team_id: Some(team_id.into()),
			display_name: dev_defaults::TEAM_NAME.to_string(),
			owner_user_id: Some(user_id.into())
		})
		.await?;

		team_id
	};

	// Create game
	let game_resolve_res = op!([ctx] game_resolve_name_id {
		name_ids: vec![dev_defaults::PROJECT_SLUG.into()],
	})
	.await?;
	let game_id = if let Some(existing) = game_resolve_res.games.first() {
		tracing::debug!("default game already exists");
		unwrap!(existing.game_id).as_uuid()
	} else {
		let create_game_res = op!([ctx] game_create {
			name_id: dev_defaults::PROJECT_SLUG.into(),
			display_name: dev_defaults::PROJECT_NAME.into(),
			developer_team_id: Some(team_id.into()),
			creator_user_id: Some(user_id.into()),
		})
		.await?;

		op!([ctx] cloud_game_config_create {
			game_id: create_game_res.game_id,
		})
		.await?;

		unwrap_ref!(create_game_res.game_id).as_uuid()
	};

	// Create namespace
	let ns_resolve_res = op!([ctx] game_namespace_resolve_name_id {
		game_id: Some(game_id.into()),
		name_ids: vec![dev_defaults::ENVIRONMENT_SLUG.into()],
	})
	.await?;
	if ns_resolve_res.namespaces.is_empty() {
		// Publish default version
		let publish_res = op!([ctx] cloud_version_publish {
			game_id: Some(game_id.into()),
			display_name: "0.0.1".into(),
			config: Some(backend::cloud::VersionConfig {
				cdn: None,
				matchmaker: None,
			}),
			creator_user_id: Some(user_id.into()),
		})
		.await?;
		let version_id = unwrap_ref!(publish_res.version_id).as_uuid();

		// Create namespace
		let create_ns_res = op!([ctx] game_namespace_create {
			game_id: Some(game_id.into()),
			display_name: dev_defaults::ENVIRONMENT_NAME.into(),
			version_id: Some(version_id.into()),
			name_id: dev_defaults::ENVIRONMENT_SLUG.into(),
		})
		.await?;
		let namespace_id = unwrap_ref!(create_ns_res.namespace_id).as_uuid();

		op!([ctx] cloud_namespace_create {
			namespace_id: Some(namespace_id.into()),
		})
		.await?;
	}

	Ok(())
}
