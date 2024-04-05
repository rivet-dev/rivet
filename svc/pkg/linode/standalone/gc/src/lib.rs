use futures_util::{StreamExt, TryStreamExt};
use reqwest::header;
use rivet_operation::prelude::*;
use serde_json::json;
use util_linode::api;

#[derive(sqlx::FromRow)]
struct PrebakeServer {
	variant: String,
	ssh_key_id: i64,
	linode_id: Option<i64>,
	firewall_id: Option<i64>,
}

#[tracing::instrument(skip_all)]
pub async fn run_from_env(pools: rivet_pools::Pools) -> GlobalResult<()> {
	let client = chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("linode-gc");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = OperationContext::new(
		"linode-gc".into(),
		std::time::Duration::from_secs(60),
		rivet_connection::Connection::new(client, pools, cache),
		Uuid::new_v4(),
		Uuid::new_v4(),
		util::timestamp::now(),
		util::timestamp::now(),
		(),
		Vec::new(),
	);
	let crdb = ctx.crdb().await?;

	let filter = json!({
		"status": "available",
		"type": "manual"
	});
	let mut headers = header::HeaderMap::new();
	headers.insert(
		"X-Filter",
		header::HeaderValue::from_str(&serde_json::to_string(&filter)?)?,
	);

	// Build HTTP client
	let client = util_linode::Client::new_with_headers(headers).await?;

	let complete_images = api::list_custom_images(&client).await?;

	delete_expired_images(&client, &complete_images).await?;

	// Get image ids
	let image_ids = complete_images
		.into_iter()
		.map(|x| x.id.clone())
		.collect::<Vec<_>>();
	if image_ids.len() == 100 {
		tracing::warn!("page limit reached, new images may not be returned");
	}

	let prebake_servers = sql_fetch_all!(
		[ctx, PrebakeServer, &crdb]
		"
		SELECT variant, ssh_key_id, linode_id, firewall_id
		FROM db_cluster.server_images_linode_misc
		WHERE image_id = ANY($1)
		",
		image_ids,
	)
	.await?;

	if prebake_servers.is_empty() {
		return Ok(());
	}

	let variants = prebake_servers
		.iter()
		.map(|server| server.variant.clone())
		.collect::<Vec<_>>();

	// Update image id so it can now be used in provisioning
	sql_execute!(
		[ctx, &crdb]
		"
		UPDATE db_cluster.server_images AS i
		SET image_id = m.image_id
		FROM db_cluster.server_images_linode_misc AS m
		WHERE
			m.variant = ANY($1) AND
			i.variant = m.variant
		",
		&variants
	)
	.await?;

	// Remove records
	sql_execute!(
		[ctx, &crdb]
		"
		DELETE FROM db_cluster.server_images_linode_misc
		WHERE variant = ANY($1)
		",
		variants,
	)
	.await?;

	futures_util::stream::iter(prebake_servers.iter())
		.map(|server| {
			let client = client.clone();

			async move { destroy(&client, server).await }
		})
		.buffer_unordered(8)
		.try_collect::<Vec<_>>()
		.await?;

	Ok(())
}

async fn delete_expired_images(
	client: &util_linode::Client,
	complete_images: &[api::CustomImage],
) -> GlobalResult<()> {
	let expiration = chrono::Utc::now() - chrono::Duration::days(6 * 30);

	let expired_images = complete_images
		.iter()
		.filter(|img| img.created < expiration);

	let expired_images_count = expired_images.clone().count();
	if expired_images_count != 0 {
		tracing::info!(count=?expired_images_count, "deleting expired images");
	}

	futures_util::stream::iter(expired_images)
		.map(|img| {
			let client = client.clone();

			async move { api::delete_custom_image(&client, &img.id).await }
		})
		.buffer_unordered(8)
		.try_collect::<Vec<_>>()
		.await?;

	Ok(())
}

// NOTE: We do not use `cluster-server-destroy` here because this is a prebake server (only
// `cluster-server-install` works with both)
async fn destroy(client: &util_linode::Client, server: &PrebakeServer) -> GlobalResult<()> {
	if let Some(linode_id) = server.linode_id {
		api::delete_instance(client, linode_id).await?;
	}

	api::delete_ssh_key(client, server.ssh_key_id).await?;

	if let Some(firewall_id) = server.firewall_id {
		api::delete_firewall(client, firewall_id).await?;
	}

	Ok(())
}
