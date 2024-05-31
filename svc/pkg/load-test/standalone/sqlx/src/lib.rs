use rivet_operation::prelude::*;

#[tracing::instrument(skip_all)]
pub async fn run_from_env(_ts: i64) -> GlobalResult<()> {
	let pools = rivet_pools::from_env("load-test-sqlx").await?;
	let client = chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("load-test-sqlx");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = OperationContext::new(
		"load-test-sqlx".into(),
		std::time::Duration::from_secs(60),
		rivet_connection::Connection::new(client, pools, cache),
		Uuid::new_v4(),
		Uuid::new_v4(),
		util::timestamp::now(),
		util::timestamp::now(),
		(),
		Vec::new(),
	);

	let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));
	loop {
		interval.tick().await;

		let ctx = ctx.clone();
		tokio::spawn(async move {
			match exec(ctx).await {
				Ok(_) => {}
				Err(err) => {
					tracing::error!(?err, "error");
				}
			}
		});
	}
}

async fn exec(ctx: OperationContext<()>) -> GlobalResult<()> {
	// Users
	let user_ids = sql_fetch_all!(
		[ctx, (Uuid,)]
		"
		SELECT user_id
		FROM db_user.users
		ORDER BY random()
		LIMIT 10
		",
	)
	.await?
	.into_iter()
	.map(|x| x.0)
	.collect::<Vec<Uuid>>();

	// Namespaces
	let namespace_ids = sql_fetch_all!(
		[ctx, (Uuid,)]
		"
		SELECT namespace_id
		FROM db_game.game_namespaces
		ORDER BY random()
		LIMIT 10
		",
	)
	.await?
	.into_iter()
	.map(|x| x.0)
	.collect::<Vec<Uuid>>();
	let namespace_ids_proto = namespace_ids
		.iter()
		.cloned()
		.map(Into::<common::Uuid>::into)
		.collect::<Vec<_>>();

	// Versions
	let version_ids = sql_fetch_all!(
		[ctx, (Uuid,)]
		"
		SELECT version_id
		FROM db_game.game_versions
		ORDER BY random()
		LIMIT 10
		",
	)
	.await?
	.into_iter()
	.map(|x| x.0)
	.collect::<Vec<Uuid>>();
	let version_ids_proto = version_ids
		.iter()
		.cloned()
		.map(Into::<common::Uuid>::into)
		.collect::<Vec<_>>();

	let _ = tokio::try_join!(
		sql_fetch_one!(
			[ctx, (String,)]
			"SELECT display_name FROM db_user.users WHERE user_id = ANY($1)",
			user_ids,
		),
		sql_fetch_all!(
			[ctx, (Uuid, String, i64)]
			"
			SELECT namespace_id, domain, create_ts
			FROM db_cdn.game_namespace_domains
			WHERE namespace_id = ANY($1)
			",
			&namespace_ids,
		),
		sql_fetch_all!(
			[ctx, LobbyGroup]
			"
			SELECT 
				lobby_group_id, version_id,
				name_id,
				max_players_normal, max_players_direct, max_players_party,
				listable, taggable,
				runtime, runtime_meta,
				find_config, join_config, create_config
			FROM db_mm_config.lobby_groups
			WHERE version_id = ANY($1)
			",
			&version_ids,
		),
		sql_fetch_all!(
			[ctx, (Uuid, Vec<u8>, i64, String, String)]
			"
				SELECT version_id, glob, priority, header_name, header_value
				FROM db_cdn.game_version_custom_headers
				WHERE version_id = ANY($1)
			",
			&version_ids,
		),
		op!([ctx] cdn_namespace_get { namespace_ids: namespace_ids_proto.clone() }),
		op!([ctx] mm_config_version_get { version_ids: version_ids_proto.clone() }),
		op!([ctx] cdn_version_get { version_ids: version_ids_proto.clone() }),
	)?;

	Ok(())
}

#[derive(Clone, sqlx::FromRow)]
#[allow(dead_code)]
struct LobbyGroup {
	lobby_group_id: Uuid,
	version_id: Uuid,

	name_id: String,

	max_players_normal: i64,
	max_players_direct: i64,
	max_players_party: i64,
	listable: bool,
	taggable: bool,

	runtime: Vec<u8>,
	runtime_meta: Vec<u8>,
	find_config: Option<Vec<u8>>,
	join_config: Option<Vec<u8>>,
	create_config: Option<Vec<u8>>,
}
