use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(Clone, sqlx::FromRow)]
struct GameVersion {
	version_id: Uuid,
	captcha_config: Option<Vec<u8>>,
}

#[derive(Clone, sqlx::FromRow)]
struct LobbyGroup {
	lobby_group_id: Uuid,
	version_id: Uuid,

	name_id: String,

	max_players_normal: i64,
	max_players_direct: i64,
	max_players_party: i64,
	listable: bool,
	taggable: bool,
	allow_dynamic_max_players: bool,

	runtime: Vec<u8>,
	runtime_meta: Vec<u8>,
	find_config: Option<Vec<u8>>,
	join_config: Option<Vec<u8>>,
	create_config: Option<Vec<u8>>,
}

#[derive(Clone, sqlx::FromRow)]
struct LobbyGroupRegion {
	lobby_group_id: Uuid,
	region_id: Uuid,
	tier_name_id: Option<String>,
}

#[derive(Clone, sqlx::FromRow)]
struct LobbyGroupIdleLobbies {
	lobby_group_id: Uuid,
	region_id: Uuid,
	min_idle_lobbies: i64,
	max_idle_lobbies: i64,
}

#[operation(name = "mm-config-version-get")]
async fn handle(
	ctx: OperationContext<mm_config::version_get::Request>,
) -> GlobalResult<mm_config::version_get::Response> {
	let req_version_ids = ctx
		.version_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	// TODO: There's a bug with this that returns the lobby groups for the wrong
	// version, can't figure this out
	// let versions = ctx
	// 	.cache()
	// 	.immutable()
	// 	.fetch_all_proto("versions", req_version_ids, |mut cache, req_version_ids| {
	// 		let ctx = ctx.base();
	//
	// 		async move {
	// 			fetch_versions(&ctx, req_version_ids)
	// 				.await?
	// 				.into_iter()
	// 				.for_each(|(version_id, version)| {
	// 					cache.resolve_with_topic(
	// 						&version_id,
	// 						version,
	// 						("game_mm_versions", &version_id),
	// 					)
	// 				});
	// 			Ok(cache)
	// 		}
	// 	})
	// 	.await?;

	// HACK: Because fetch all doesn't work, we'll use fetch one
	// let mut versions = Vec::new();
	// for version_id in req_version_ids {
	// 	let version = ctx
	// 		.cache()
	// 		.immutable()
	// 		.fetch_one_proto("versions2", version_id, |mut cache, req_version_id| {
	// 			let ctx = ctx.base();
	//
	// 			async move {
	// 				let versions = fetch_versions(&ctx.base(), vec![req_version_id]).await?;
	// 				ensure!(versions.len() <= 1, "too many versions");
	// 				if let Some((_, version)) = versions.into_iter().next() {
	// 					cache.resolve(&version_id, version);
	// 				}
	//
	// 				Ok(cache)
	// 			}
	// 		})
	// 		.await?;
	// 	if let Some(version) = version {
	// 		versions.push(version);
	// 	}
	// }

	let versions = fetch_versions(&ctx.base(), req_version_ids)
		.await?
		.into_iter()
		.map(|x| x.1)
		.collect::<Vec<_>>();

	Ok(mm_config::version_get::Response { versions })
}

async fn fetch_versions(
	ctx: &OperationContext<()>,
	req_version_ids: Vec<Uuid>,
) -> GlobalResult<Vec<(Uuid, mm_config::version_get::response::Version)>> {
	let (versions, lobby_groups) = tokio::try_join!(
		sql_fetch_all!(
			[ctx, GameVersion]
			"
			SELECT version_id, captcha_config
			FROM db_mm_config.game_versions
			WHERE version_id = ANY($1)
			",
			&req_version_ids,
		),
		sql_fetch_all!(
			[ctx, LobbyGroup]
			"
			SELECT 
				lobby_group_id, version_id,
				name_id,
				max_players_normal, max_players_direct, max_players_party,
				listable, taggable, allow_dynamic_max_players,
				runtime, runtime_meta,
				find_config, join_config, create_config
			FROM db_mm_config.lobby_groups
			WHERE version_id = ANY($1)
			",
			&req_version_ids,
		),
	)?;

	let all_lobby_group_ids = lobby_groups
		.iter()
		.map(|lg| lg.lobby_group_id)
		.collect::<Vec<_>>();
	let (lobby_group_regions, lobby_group_idle_lobbies) = tokio::try_join!(
		sql_fetch_all!(
			[ctx, LobbyGroupRegion]
			"
			SELECT lobby_group_id, region_id, tier_name_id
			FROM db_mm_config.lobby_group_regions
			WHERE lobby_group_id = ANY($1)
			",
			&all_lobby_group_ids,
		),
		sql_fetch_all!(
			[ctx, LobbyGroupIdleLobbies]
			"
			SELECT lobby_group_id, region_id, min_idle_lobbies, max_idle_lobbies
			FROM db_mm_config.lobby_group_idle_lobbies
			WHERE lobby_group_id = ANY($1)
			",
			&all_lobby_group_ids,
		),
	)?;

	let res_versions = versions
		.iter()
		.map(|v| {
			let mut version_lobby_groups = lobby_groups
				.iter()
				.filter(|lg| lg.version_id == v.version_id)
				.cloned()
				.collect::<Vec<_>>();
			version_lobby_groups.sort_by_cached_key(|x| x.name_id.to_owned());

			let captcha_config = v
				.captcha_config
				.clone()
				.map(|captcha_config| {
					backend::captcha::CaptchaConfig::decode(captcha_config.as_slice())
				})
				.transpose()?;

			let version = mm_config::version_get::response::Version {
				version_id: Some(v.version_id.into()),
				config: Some(backend::matchmaker::VersionConfig {
					lobby_groups: version_lobby_groups
						.iter()
						.map(|lg| -> GlobalResult<backend::matchmaker::LobbyGroup> {
							let lobby_group_id = lg.lobby_group_id;
							let lobby_regions = lobby_group_regions
								.iter()
								.filter(|lgr| lgr.lobby_group_id == lobby_group_id);

							let runtime =
								backend::matchmaker::LobbyRuntime::decode(lg.runtime.as_ref())?;
							let (find_config, join_config, create_config) = match (
								lg.find_config
									.as_ref()
									.map(|fc| backend::matchmaker::FindConfig::decode(fc.as_ref()))
									.transpose(),
								lg.join_config
									.as_ref()
									.map(|jc| backend::matchmaker::JoinConfig::decode(jc.as_ref()))
									.transpose(),
								lg.create_config
									.as_ref()
									.map(|jc| {
										backend::matchmaker::CreateConfig::decode(jc.as_ref())
									})
									.transpose(),
							) {
								(Ok(a), Ok(b), Ok(c)) => (a, b, c),
								(Err(err), _, _) | (_, Err(err), _) | (_, _, Err(err)) => {
									tracing::warn!(
										?err,
										"failed to decode actions, this is a bad proto migration"
									);
									(None, None, None)
								}
							};

							Ok(backend::matchmaker::LobbyGroup {
								name_id: lg.name_id.clone(),

								regions: lobby_regions
									.cloned()
									.map(|lgr| backend::matchmaker::lobby_group::Region {
										region_id: Some(lgr.region_id.into()),
										tier_name_id: lgr.tier_name_id.clone().unwrap_or_else(
											|| util_mm::defaults::TIER_NAME_ID.to_owned(),
										),
										idle_lobbies: lobby_group_idle_lobbies
											.iter()
											.find(|lgil| {
												lgil.lobby_group_id == lobby_group_id
													&& lgil.region_id == lgr.region_id
											})
											.map(|lgil| {
												backend::matchmaker::lobby_group::IdleLobbies {
													min_idle_lobbies: lgil.min_idle_lobbies as u32,
													max_idle_lobbies: lgil.max_idle_lobbies as u32,
												}
											}),
									})
									.collect(),
								max_players_normal: lg.max_players_normal as u32,
								max_players_direct: lg.max_players_direct as u32,
								max_players_party: lg.max_players_party as u32,
								listable: lg.listable,
								taggable: lg.taggable,
								allow_dynamic_max_players: lg.allow_dynamic_max_players,

								runtime: Some(runtime),

								actions: (find_config.is_some()
									|| join_config.is_some() || create_config.is_some())
								.then_some(backend::matchmaker::lobby_group::Actions {
									find: find_config,
									join: join_config,
									create: create_config,
								}),
							})
						})
						.collect::<GlobalResult<Vec<_>>>()?,
					captcha: captcha_config,
				}),
				config_meta: Some(backend::matchmaker::VersionConfigMeta {
					lobby_groups: version_lobby_groups
						.iter()
						.map(|lg| -> GlobalResult<backend::matchmaker::LobbyGroupMeta> {
							let lobby_group_id = lg.lobby_group_id;
							let runtime_meta = backend::matchmaker::LobbyRuntimeMeta::decode(
								lg.runtime_meta.as_ref(),
							)?;

							Ok(backend::matchmaker::LobbyGroupMeta {
								lobby_group_id: Some(lobby_group_id.into()),
								runtime: Some(runtime_meta),
							})
						})
						.collect::<GlobalResult<Vec<_>>>()?,
				}),
			};

			GlobalResult::Ok((v.version_id, version))
		})
		.filter_map(|res| match res {
			Ok(x) => Some(x),
			Err(err) => {
				tracing::error!(?err, "failed to build matchmaker version");
				None
			}
		})
		.collect::<Vec<_>>();
	Ok(res_versions)
}
