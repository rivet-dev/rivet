use chirp_workflow::prelude::*;
use rivet_operation::prelude::proto::backend::{self, pkg::*};
use util::dev_defaults;

#[tracing::instrument(skip_all)]
pub async fn start(config: rivet_config::Config, pools: rivet_pools::Pools) -> GlobalResult<()> {
	let client =
		chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("cloud-default-create");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = StandaloneCtx::new(
		db::DatabaseCrdbNats::from_pools(pools.clone())?,
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

	wait_for_consumers(&ctx).await?;

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

		let game_id = unwrap_ref!(create_game_res.game_id).as_uuid();

		ctx.op(pegboard::ops::game_config::upsert::Input {
			game_id,
			host_networking_enabled: Some(true),
			root_user_enabled: Some(true),
			..Default::default()
		})
		.await?;

		game_id
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

/// Keys that must have consumers before proceeding.
const REQUIRED_CONSUMER_KEYS: &[&str] = &[
	"{topic:msg-user-create}:topic",
	"{topic:msg-team-create}:topic",
];

/// HACK: Wait until there has been a consumer created before publishing messages. This is
/// required because `chirp-worker` must create the consumers before they can start accepting
/// messages.
async fn wait_for_consumers(ctx: &StandaloneCtx) -> GlobalResult<()> {
	let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));
	'poll: loop {
		interval.tick().await;

		for key in REQUIRED_CONSUMER_KEYS {
			// Check if the stream key exists
			let exists: bool = redis::cmd("EXISTS")
				.arg(&key)
				.query_async(&mut ctx.redis_chirp().await?)
				.await?;

			if !exists {
				tracing::debug!(?key, "key does not exist");
				continue 'poll;
			}

			// Check if there are consumers for the stream
			let groups: Vec<redis::Value> = redis::cmd("XINFO")
				.arg("GROUPS")
				.arg(&key)
				.query_async(&mut ctx.redis_chirp().await?)
				.await?;

			if groups.is_empty() {
				tracing::debug!(?key, "missing consumers");
				continue 'poll;
			}
		}

		tracing::debug!("all consumers found");
		break;
	}

	Ok(())
}
