use std::fmt::Debug;

use futures_util::StreamExt;
use gas::prelude::*;
use rivet_api_client::{HeaderMap, Method, request_remote_datacenter};

#[derive(Clone, Debug, Default)]
pub struct Input {}

#[operation]
pub async fn bump_serverless_autoscaler_global(ctx: &OperationCtx, input: &Input) -> Result<()> {
	let dcs = &ctx.config().topology().datacenters;

	let results = futures_util::stream::iter(dcs.clone().into_iter().map(|dc| {
		let ctx = ctx.clone();

		async move {
			if dc.datacenter_label == ctx.config().dc_label() {
				// Local datacenter
				ctx.msg(rivet_types::msgs::pegboard::BumpServerlessAutoscaler {})
					.send()
					.await
			} else {
				// Remote datacenter - HTTP request
				request_remote_datacenter(
					ctx.config(),
					dc.datacenter_label,
					"/bump-serverless-autoscaler",
					Method::POST,
					HeaderMap::new(),
					Option::<&()>::None,
					Option::<&()>::None,
				)
				.await
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
