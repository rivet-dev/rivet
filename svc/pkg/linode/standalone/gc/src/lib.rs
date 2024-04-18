use futures_util::{StreamExt, TryStreamExt};
use proto::backend;
use reqwest::header;
use rivet_operation::prelude::*;
use serde_json::json;
use util_linode::api;

#[derive(sqlx::FromRow)]
struct LinodePrebakeServer {
	install_hash: String,
	datacenter_id: Uuid,
	pool_type: i64,

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

	let dc_rows = sql_fetch_all!(
		[ctx, (i64, String,)]
		"
		SELECT DISTINCT provider, provider_api_token
		FROM db_cluster.datacenters
		WHERE provider_api_token IS NOT NULL
		",
	)
	.await?
	.into_iter()
	.chain(std::iter::once((
		backend::cluster::Provider::Linode as i64,
		util::env::read_secret(&["linode", "token"]).await?,
	)));

	let filter = json!({
		"status": "available",
		"type": "manual"
	});
	let mut headers = header::HeaderMap::new();
	headers.insert(
		"X-Filter",
		header::HeaderValue::from_str(&serde_json::to_string(&filter)?)?,
	);

	for (provider, api_token) in dc_rows {
		let provider = unwrap!(backend::cluster::Provider::from_i32(provider as i32));

		match provider {
			backend::cluster::Provider::Linode => {
				run_for_linode_account(&ctx, &api_token, &headers).await?
			}
		}
	}

	Ok(())
}

async fn run_for_linode_account(
	ctx: &OperationContext<()>,
	api_token: &str,
	headers: &header::HeaderMap,
) -> GlobalResult<()> {
	// Build HTTP client
	let client =
		util_linode::Client::new_with_headers(Some(api_token.to_string()), headers.clone()).await?;

	let complete_images = api::list_custom_images(&client).await?;

	delete_expired_images(&client, &complete_images).await?;

	// Get image ids
	let image_ids = complete_images
		.into_iter()
		.map(|x| x.id)
		.collect::<Vec<_>>();
	if image_ids.len() == 100 {
		tracing::warn!("page limit reached, new images may not be returned");
	}

	let prebake_servers = rivet_pools::utils::crdb::tx(&ctx.crdb().await?, |tx| {
		let ctx = ctx.clone();
		let image_ids = image_ids.clone();

		Box::pin(async move {
			let prebake_servers = sql_fetch_all!(
				[ctx, LinodePrebakeServer, @tx tx]
				"
				SELECT
					install_hash, datacenter_id, pool_type,
					ssh_key_id, linode_id, firewall_id
				FROM db_cluster.server_images_linode
				WHERE image_id = ANY($1)
				",
				image_ids,
			)
			.await?;

			if prebake_servers.is_empty() {
				return Ok(Vec::new());
			}

			let primary_keys = prebake_servers
				.iter()
				.map(|server| {
					(
						&server.install_hash,
						&server.datacenter_id,
						server.pool_type,
					)
				})
				.collect::<Vec<_>>();
			let primary_keys = serde_json::to_string(&primary_keys)?;

			// Update image id so it can now be used in provisioning
			sql_execute!(
				[ctx, @tx tx]
				"
				UPDATE db_cluster.server_images AS i
				SET provider_image_id = m.image_id
				FROM (
					SELECT
						install_hash, datacenter_id, pool_type, image_id
					FROM db_cluster.server_images_linode AS s
					INNER JOIN jsonb_array_elements($1::JSONB) AS q
					ON
						s.install_hash = (q->>0)::TEXT AND
						s.datacenter_id = (q->>1)::UUID AND
						s.pool_type = (q->>2)::INT
				) AS m
				WHERE
					i.provider = $2 AND
					i.install_hash = m.install_hash AND
					i.datacenter_id = m.datacenter_id AND
					i.pool_type = m.pool_type
				",
				&primary_keys,
				backend::cluster::Provider::Linode as i64,
			)
			.await?;

			// Remove records
			sql_execute!(
				[ctx, @tx tx]
				"
				DELETE FROM db_cluster.server_images_linode AS s
				USING jsonb_array_elements($1::JSONB) AS q
				WHERE
					s.install_hash = (q->>0)::TEXT AND
					s.datacenter_id = (q->>1)::UUID AND
					s.pool_type = (q->>2)::INT
				",
				&primary_keys,
				backend::cluster::Provider::Linode as i64,
			)
			.await?;

			Ok(prebake_servers)
		})
	})
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
async fn destroy(client: &util_linode::Client, server: &LinodePrebakeServer) -> GlobalResult<()> {
	if let Some(linode_id) = server.linode_id {
		api::delete_instance(client, linode_id).await?;
	}

	api::delete_ssh_key(client, server.ssh_key_id).await?;

	if let Some(firewall_id) = server.firewall_id {
		api::delete_firewall(client, firewall_id).await?;
	}

	Ok(())
}
