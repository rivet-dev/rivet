use std::fmt::Debug;

use futures_util::StreamExt;
use gas::prelude::*;
use rivet_api_util::{HeaderMap, Method, request_remote_datacenter};
use rivet_cache::RawCacheKey;
use serde::Serialize;

#[derive(Clone, Debug, Default)]
pub struct Input {
	pub base_key: String,
	pub keys: Vec<RawCacheKey>,
}

#[operation]
pub async fn cache_purge_global(ctx: &OperationCtx, input: &Input) -> Result<()> {
	let dcs = &ctx.config().topology().datacenters;

	let results = futures_util::stream::iter(dcs.clone().into_iter().map(|dc| {
		let ctx = ctx.clone();
		let input = input.clone();

		async move {
			if dc.datacenter_label == ctx.config().dc_label() {
				// Local datacenter
				ctx.cache()
					.clone()
					.request()
					.purge(input.base_key, input.keys)
					.await
			} else {
				// Remote datacenter - HTTP request
				request_remote_datacenter::<CachePurgeResponse>(
					ctx.config(),
					dc.datacenter_label,
					"/cache/purge",
					Method::POST,
					HeaderMap::new(),
					Option::<&()>::None,
					Some(&CachePurgeRequest {
						base_key: input.base_key,
						keys: input.keys,
					}),
				)
				.await
				.map(|_| ())
			}
		}
	}))
	.buffer_unordered(16)
	.collect::<Vec<_>>()
	.await;

	// Aggregate results
	let result_count = results.len();
	let mut errors = Vec::new();
	for res in results {
		if let Err(err) = res {
			tracing::error!(?err, "failed to request edge dc");
			errors.push(err);
		}
	}

	// Error only if all requests failed
	if result_count == errors.len() {
		if let Some(res) = errors.into_iter().next() {
			return Err(res).context("all datacenter requests failed");
		}
	}

	Ok(())
}

// TODO: This is cloned from api-peer because of a cyclical dependency
#[derive(Serialize)]
pub struct CachePurgeRequest {
	pub base_key: String,
	pub keys: Vec<rivet_cache::RawCacheKey>,
}

#[derive(Deserialize)]
pub struct CachePurgeResponse {}
