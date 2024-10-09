use std::convert::TryInto;

use chirp_workflow::prelude::*;
use cluster::types::Provider;
use futures_util::{StreamExt, TryStreamExt};
use linode::util::{api, client};
use reqwest::header;
use serde_json::json;

#[tracing::instrument(skip_all)]
pub async fn run_from_env(pools: rivet_pools::Pools) -> GlobalResult<()> {
	let client = chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("linode-gc");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = StandaloneCtx::new(
		chirp_workflow::compat::db_from_pools(&pools).await?,
		rivet_connection::Connection::new(client, pools, cache),
		"linode-gc",
	)
	.await?;

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
		Provider::Linode as i64,
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
		let provider = unwrap!(Provider::from_repr(provider.try_into()?));

		match provider {
			Provider::Linode => run_for_linode_account(&ctx, &api_token, &headers).await?,
		}
	}

	Ok(())
}

async fn run_for_linode_account(
	ctx: &StandaloneCtx,
	api_token: &str,
	headers: &header::HeaderMap,
) -> GlobalResult<()> {
	// Build HTTP client
	let client =
		client::Client::new_with_headers(Some(api_token.to_string()), headers.clone()).await?;

	let complete_images = api::list_custom_images(&client).await?;

	if complete_images.len() == linode::util::api::CUSTOM_IMAGE_LIST_SIZE {
		// We don't need to paginate since we'll never have more than
		// `number of regions * number of pools * 2` images which is not more than 500 (x2 is for the old +
		// new images)
		tracing::warn!("page limit reached, new images may not be returned");
	}

	delete_expired_images(ctx, &complete_images).await?;

	// Get image ids
	let image_ids = complete_images
		.into_iter()
		.map(|x| x.id)
		.collect::<Vec<_>>();

	// Set images as complete
	let incomplete_images = sql_fetch_all!(
		[ctx, (String,)]
		"
		UPDATE db_linode.server_images
		SET complete_ts = $2
		WHERE
			image_id = ANY($1) AND
			complete_ts IS NULL
		RETURNING image_id
		",
		image_ids,
		util::timestamp::now(),
	)
	.await?;

	futures_util::stream::iter(incomplete_images.into_iter())
		.map(|(image_id,)| {
			let ctx = ctx.clone();

			async move {
				ctx.signal(linode::workflows::image::CreateComplete {
					image_id: image_id.clone(),
				})
				.tag("image_id", image_id)
				.send()
				.await
			}
		})
		.buffer_unordered(8)
		.try_collect::<Vec<_>>()
		.await?;

	Ok(())
}

async fn delete_expired_images(
	ctx: &StandaloneCtx,
	complete_images: &[api::CustomImage],
) -> GlobalResult<()> {
	// Prebake images have an expiration because of their server token. We add 2 days of padding here for
	// safety
	let expiration = chrono::Utc::now()
		- chrono::Duration::milliseconds(cluster::util::SERVER_TOKEN_TTL)
		+ chrono::Duration::days(2);

	let expired_images = complete_images
		.iter()
		.filter(|img| img.created < expiration);

	let expired_images_count = expired_images.clone().count();
	if expired_images_count != 0 {
		tracing::info!(count=?expired_images_count, "deleting expired images");
	}

	futures_util::stream::iter(expired_images)
		.map(|img| {
			let ctx = ctx.clone();

			async move {
				ctx.signal(linode::workflows::image::Destroy {})
					.tag("image_id", &img.id)
					.send()
					.await
			}
		})
		.buffer_unordered(8)
		.try_collect::<Vec<_>>()
		.await?;

	Ok(())
}
