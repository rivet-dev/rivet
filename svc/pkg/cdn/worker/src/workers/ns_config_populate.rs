use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use redis::AsyncCommands;

#[worker(name = "cdn-ns-config-populate")]
async fn worker(ctx: &OperationContext<cdn::msg::ns_config_update::Message>) -> GlobalResult<()> {
	let namespace_id = unwrap!(ctx.namespace_id).as_uuid();

	if let Some((game, ns, cdn_ns_config, cdn_version)) = get_cdn_version(ctx, namespace_id).await?
	{
		let cdn_version_config = unwrap_ref!(cdn_version.config);
		let site_id = unwrap!(cdn_version_config.site_id);
		let cdn_site_res = op!([ctx] cdn_site_get {
			site_ids: vec![site_id],
		})
		.await?;
		let site = unwrap!(cdn_site_res.sites.first());
		let upload_id = unwrap!(site.upload_id);

		let upload_res = op!([ctx] upload_get {
			upload_ids: vec![upload_id],
		})
		.await?;
		let upload = if let Some(upload) = upload_res.uploads.first() {
			upload
		} else {
			tracing::error!(?site_id, ?upload_id, "failed to find upload for site");
			return Ok(());
		};

		let routes = cdn_version_config.routes.clone();

		// Encode config
		let config = cdn::redis_cdn::NamespaceCdnConfig {
			namespace_id: Some(namespace_id.into()),
			game_name_id: game.name_id.clone(),
			namespace_name_id: ns.name_id.clone(),
			domains: cdn_ns_config.domains.clone(),
			upload_id: upload.upload_id,
			auth_type: cdn_ns_config.auth_type,
			auth_user_list: cdn_ns_config.auth_user_list.clone(),
			routes,
		};
		tracing::info!(?config, "cdn namespace config");

		let mut buf = Vec::with_capacity(config.encoded_len());
		config.encode(&mut buf)?;

		// Write to database
		ctx.redis_cdn()
			.await?
			.hset(
				util_cdn::key::ns_cdn_configs(),
				namespace_id.to_string(),
				buf,
			)
			.await?;
	} else {
		tracing::info!("removing cdn config");
		// Remove CDN config
		ctx.redis_cdn()
			.await?
			.hdel(util_cdn::key::ns_cdn_configs(), namespace_id.to_string())
			.await?;
	};

	Ok(())
}

async fn get_cdn_version(
	ctx: &OperationContext<cdn::msg::ns_config_update::Message>,
	namespace_id: Uuid,
) -> GlobalResult<
	Option<(
		backend::game::Game,
		backend::game::Namespace,
		backend::cdn::NamespaceConfig,
		cdn::version_get::response::Version,
	)>,
> {
	let ns_res = op!([ctx] game_namespace_get {
		namespace_ids: vec![namespace_id.into()],
	})
	.await?;
	let ns = unwrap!(ns_res.namespaces.first());
	let game_id = unwrap_ref!(ns.game_id);
	let version_id = unwrap_ref!(ns.version_id);

	let game_res = op!([ctx] game_get {
		game_ids: vec![*game_id],
	})
	.await?;
	let game = unwrap!(game_res.games.first());

	let cdn_ns_res = op!([ctx] cdn_namespace_get {
		namespace_ids: vec![namespace_id.into()],
	})
	.await?;
	let cdn_ns = if let Some(x) = cdn_ns_res.namespaces.first() {
		x
	} else {
		tracing::info!("missing cdn namespace");
		return Ok(None);
	};
	let cdn_ns_config = unwrap_ref!(cdn_ns.config);

	let cdn_version_res = op!([ctx] cdn_version_get {
		version_ids: vec![*version_id],
	})
	.await?;

	if let Some(version) = cdn_version_res.versions.first() {
		Ok(Some((
			game.clone(),
			ns.clone(),
			cdn_ns_config.clone(),
			version.clone(),
		)))
	} else {
		tracing::info!("missing cdn version");
		Ok(None)
	}
}
