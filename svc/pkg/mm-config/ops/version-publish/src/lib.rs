use prost::Message;
use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[operation(name = "mm-config-version-publish")]
async fn handle(
	ctx: OperationContext<mm_config::version_publish::Request>,
) -> GlobalResult<mm_config::version_publish::Response> {
	let version_id = unwrap_ref!(ctx.version_id).as_uuid();
	let config = unwrap_ref!(ctx.config);
	let config_ctx = unwrap_ref!(ctx.config_ctx);

	ensure_eq!(
		config.lobby_groups.len(),
		config_ctx.lobby_groups.len(),
		"incorrect lobby group ctx count"
	);

	let tx = ctx.crdb().await?.begin().await?;

	// Encode captcha data
	let captcha_buf = config
		.captcha
		.clone()
		.map(|captcha| {
			let mut captcha_buf = Vec::with_capacity(captcha.encoded_len());
			captcha.encode(&mut captcha_buf)?;

			GlobalResult::Ok(captcha_buf)
		})
		.transpose()?;

	// Save version
	sql_execute!(
		[ctx]
		"INSERT INTO db_mm_config.game_versions (
			version_id,
			captcha_config,
			migrations
		)
		VALUES ($1, $2, $3)",
		version_id,
		captcha_buf,
		util_mm::version_migrations::all(),
	)
	.await?;

	// Save lobby groups
	ensure_eq!(config.lobby_groups.len(), config_ctx.lobby_groups.len());
	for (lobby_group, lobby_group_ctx) in config
		.lobby_groups
		.iter()
		.zip(config_ctx.lobby_groups.iter())
	{
		let lobby_group_id = Uuid::new_v4();

		// Build runtime meta
		let runtime = unwrap_ref!(lobby_group.runtime);
		let runtime_ctx = unwrap_ref!(lobby_group_ctx.runtime);
		let (runtime, runtime_meta) = publish_runtime(
			unwrap_ref!(runtime.runtime),
			unwrap_ref!(runtime_ctx.runtime),
		)?;

		// Encode runtime data
		let (runtime_buf, runtime_meta_buf) = {
			let mut runtime_buf = Vec::with_capacity(runtime.encoded_len());
			runtime.encode(&mut runtime_buf)?;

			let mut runtime_meta_buf = Vec::with_capacity(runtime_meta.encoded_len());
			runtime_meta.encode(&mut runtime_meta_buf)?;

			(runtime_buf, runtime_meta_buf)
		};

		// Encode config data
		let find_config_buf = lobby_group
			.actions
			.as_ref()
			.and_then(|a| a.find.as_ref())
			.map(|config| {
				let mut buf = Vec::with_capacity(config.encoded_len());
				config.encode(&mut buf)?;

				GlobalResult::Ok(buf)
			})
			.transpose()?;
		let join_config_buf = lobby_group
			.actions
			.as_ref()
			.and_then(|a| a.join.as_ref())
			.map(|config| {
				let mut buf = Vec::with_capacity(config.encoded_len());
				config.encode(&mut buf)?;

				GlobalResult::Ok(buf)
			})
			.transpose()?;
		let create_config_buf = lobby_group
			.actions
			.as_ref()
			.and_then(|a| a.create.as_ref())
			.map(|config| {
				let mut buf = Vec::with_capacity(config.encoded_len());
				config.encode(&mut buf)?;

				GlobalResult::Ok(buf)
			})
			.transpose()?;

		sql_execute!(
			[ctx]
			"
			INSERT INTO db_mm_config.lobby_groups (
				lobby_group_id, 
				version_id,

				name_id,

				max_players_normal,
				max_players_direct,
				max_players_party,
				listable,

				runtime,
				runtime_meta,
				find_config,
				join_config,
				create_config,
				allow_dynamic_max_players,
				taggable
			)
			VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
			",
			lobby_group_id,
			version_id,
			&lobby_group.name_id,
			lobby_group.max_players_normal as i64,
			lobby_group.max_players_direct as i64,
			lobby_group.max_players_party as i64,
			lobby_group.listable,
			&runtime_buf,
			&runtime_meta_buf,
			&find_config_buf,
			&join_config_buf,
			&create_config_buf,
			lobby_group.allow_dynamic_max_players,
			lobby_group.taggable,
		)
		.await?;

		for region in &lobby_group.regions {
			let region_id = unwrap_ref!(region.region_id).as_uuid();
			sql_execute!(
				[ctx]
				"
				INSERT INTO db_mm_config.lobby_group_regions (
					lobby_group_id, region_id, tier_name_id
				)
				VALUES ($1, $2, $3)
				",
				lobby_group_id,
				region_id,
				&region.tier_name_id,
			)
			.await?;

			if let Some(idle_lobbies) = &region.idle_lobbies {
				sql_execute!(
					[ctx]
					"
				INSERT INTO db_mm_config.lobby_group_idle_lobbies (
					lobby_group_id, region_id, min_idle_lobbies, max_idle_lobbies
				)
				VALUES ($1, $2, $3, $4)
				",
					lobby_group_id,
					region_id,
					idle_lobbies.min_idle_lobbies as i64,
					idle_lobbies.max_idle_lobbies as i64,
				)
				.await?;
			}
		}
	}

	let commit_perf = ctx.perf().start("tx-commit").await;
	tx.commit().await?;
	commit_perf.end();

	Ok(mm_config::version_publish::Response {})
}

/// Takes the given runtime and runtime ctx configs and outputs a new runtime config and runtime
/// meta. We re-create the root config here because this gives an opportunity to resolve certain
/// values to values we can use in production. It's not common we need to modify the core config,
/// though.
///
/// For example: a docker image with an input of `nginx` would have the tag resolved against the
/// registry to `nginx:1.21.1` in order to pin the version.
fn publish_runtime(
	runtime: &backend::matchmaker::lobby_runtime::Runtime,
	runtime_ctx: &backend::matchmaker::lobby_runtime_ctx::Runtime,
) -> GlobalResult<(
	backend::matchmaker::LobbyRuntime,
	backend::matchmaker::LobbyRuntimeMeta,
)> {
	let (runtime, runtime_meta): (
		backend::matchmaker::LobbyRuntime,
		backend::matchmaker::LobbyRuntimeMeta,
	) = match (runtime, runtime_ctx) {
		(
			backend::matchmaker::lobby_runtime::Runtime::Docker(runtime),
			backend::matchmaker::lobby_runtime_ctx::Runtime::Docker(runtime_ctx),
		) => (
			backend::matchmaker::lobby_runtime::Docker { ..runtime.clone() }.into(),
			backend::matchmaker::lobby_runtime_meta::Docker {
				job_template_id: runtime_ctx.job_template_id,
			}
			.into(),
		),
	};

	Ok((runtime, runtime_meta))
}
