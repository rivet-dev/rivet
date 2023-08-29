use proto::backend::pkg::*;
use rivet_operation::prelude::*;

lazy_static::lazy_static! {
	static ref REDIS_SCRIPT: redis::Script = redis::Script::new(include_str!("../redis-scripts/main.lua"));
}

#[derive(Debug)]
struct LobbyGroupConfig {
	lobby_group_id: Uuid,
	min_idle_lobbies: u32,
	max_idle_lobbies: u32,
}

#[operation(name = "mm-lobby-idle-update", timeout = 150)]
async fn handle(
	ctx: OperationContext<mm::lobby_idle_update::Request>,
) -> GlobalResult<mm::lobby_idle_update::Response> {
	let namespace_id = internal_unwrap!(ctx.namespace_id).as_uuid();
	let region_id = internal_unwrap!(ctx.region_id).as_uuid();

	let lobby_configs = fetch_lobby_configs(&ctx, namespace_id, region_id).await?;

	// Run the script
	let mut script = REDIS_SCRIPT.prepare_invoke();
	script
		.arg(namespace_id.to_string())
		.arg(lobby_configs.len())
		.key(util_mm::key::idle_lobby_lobby_group_ids(
			namespace_id,
			region_id,
		));
	for lg in &lobby_configs {
		script
			.arg(lg.lobby_group_id.to_string())
			.arg(lg.min_idle_lobbies)
			.arg(lg.max_idle_lobbies)
			.key(util_mm::key::idle_lobby_ids(
				namespace_id,
				region_id,
				lg.lobby_group_id,
			));
	}
	let (create_lobby_group_ids, overflow_lobby_ids, outdated_lobby_ids) = script
		.invoke_async::<_, (Vec<String>, Vec<String>, Vec<String>)>(&mut ctx.redis_mm().await?)
		.await?;
	tracing::info!(
		create_lobby_group_ids_len = %create_lobby_group_ids.len(),
		overflow_lobby_ids_len = %overflow_lobby_ids.len(),
		outdated_lobby_ids_len = %outdated_lobby_ids.len(),
		?create_lobby_group_ids,
		?overflow_lobby_ids,
		?outdated_lobby_ids,
		?lobby_configs,
		"check result"
	);

	// Create new lobbies
	let mut create_futs = Vec::new();
	for lobby_group_id in create_lobby_group_ids {
		let lobby_group_id = util::uuid::parse(&lobby_group_id)?;
		let lobby_config = internal_unwrap_owned!(
			lobby_configs
				.iter()
				.find(|x| x.lobby_group_id == lobby_group_id),
			"failed to find lobby config"
		);

		let lobby_id = Uuid::new_v4();
		create_futs.push(msg!([ctx] mm::msg::lobby_create(lobby_id) {
			lobby_id: Some(lobby_id.into()),
			namespace_id: Some(namespace_id.into()),
			lobby_group_id: Some(lobby_config.lobby_group_id.into()),
			region_id: Some(region_id.into()),
			create_ray_id: Some(ctx.ray_id().into()),
			preemptively_created: false,

			creator_user_id: None,
			is_custom: false,
			publicity: None,
			lobby_config_json: None,
		}));
	}
	futures_util::future::try_join_all(create_futs).await?;

	// Remove old lobbies
	let mut stop_futs = Vec::new();
	for lobby_id in overflow_lobby_ids
		.into_iter()
		.chain(outdated_lobby_ids.into_iter())
	{
		let lobby_id = util::uuid::parse(&lobby_id)?;
		stop_futs.push(msg!([ctx] mm::msg::lobby_stop(lobby_id) {
			lobby_id: Some(lobby_id.into()),
		}));
	}
	futures_util::future::try_join_all(stop_futs).await?;

	Ok(mm::lobby_idle_update::Response {})
}

/// Find all the idle lobby configs associated with active versions.
///
/// These indicate all of the lobby groups that we need to manage a pool of idle lobbies in.
#[tracing::instrument(skip(ctx))]
async fn fetch_lobby_configs(
	ctx: &OperationContext<mm::lobby_idle_update::Request>,
	namespace_id: Uuid,
	region_id: Uuid,
) -> GlobalResult<Vec<LobbyGroupConfig>> {
	let region_id_proto = common::Uuid::from(region_id);

	// Fetch all versions
	let ns_res = op!([ctx] @dont_log_body game_namespace_get {
		namespace_ids: vec![namespace_id.into()],
	})
	.await?;
	let ns = internal_unwrap_owned!(ns_res.namespaces.first());
	let version_id = internal_unwrap!(ns.version_id).as_uuid();

	let mm_versions_res = op!([ctx] @dont_log_body mm_config_version_get {
		version_ids: vec![version_id.into()],
	})
	.await?;
	let version = internal_unwrap_owned!(mm_versions_res.versions.first());
	let version_config = internal_unwrap!(version.config);
	let version_meta = internal_unwrap!(version.config_meta);

	// Iterate through each of the lobby groups looking for matching idle
	// lobby configs
	let mut idle_lobbies_configs = Vec::new();
	for (lobby_group_config, lobby_group_meta) in version_config
		.lobby_groups
		.iter()
		.zip(version_meta.lobby_groups.iter())
	{
		let region = if let Some(x) = lobby_group_config
			.regions
			.iter()
			.find(|r| r.region_id.as_ref() == Some(&region_id_proto))
		{
			x
		} else {
			// This lobby group is not enabled in this region
			continue;
		};

		idle_lobbies_configs.push(LobbyGroupConfig {
			lobby_group_id: internal_unwrap!(lobby_group_meta.lobby_group_id).as_uuid(),
			min_idle_lobbies: region
				.idle_lobbies
				.as_ref()
				.map_or(0, |x| x.min_idle_lobbies),
			max_idle_lobbies: region
				.idle_lobbies
				.as_ref()
				.map_or(0, |x| x.max_idle_lobbies),
		});
	}

	tracing::info!(
		count = idle_lobbies_configs.len(),
		"idle lobby config count"
	);
	tracing::debug!(?idle_lobbies_configs, "idle lobby configs");

	Ok(idle_lobbies_configs)
}
