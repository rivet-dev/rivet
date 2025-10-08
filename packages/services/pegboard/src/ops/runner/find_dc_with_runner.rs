use std::time::Duration;

use anyhow::Result;
use futures_util::{StreamExt, TryFutureExt, stream::FuturesUnordered};
use gas::prelude::*;
use rivet_api_types::runners::list as runners_list;
use rivet_api_util::{HeaderMap, Method, request_remote_datacenter};
use rivet_types::namespaces::RunnerConfig;
use serde::de::DeserializeOwned;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
	pub namespace_id: Id,
	pub runner_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
	pub dc_label: Option<u16>,
}

/// Finds a datacenter that contains a given runner name in a namespace.
///
/// This is core to determining which datacenter actors should run in, since actors can only run in
/// datacenters with supported runners.
#[operation]
pub async fn find_dc_with_runner(ctx: &OperationCtx, input: &Input) -> Result<Output> {
	// TODO: We should figure out how to pre-emptively validate this cache so we don't have
	// "stutters" where every 15s we have a high request duration
	ctx.cache()
		.clone()
		.request()
		.ttl(15_000)
		.fetch_one_json(
			"runner.find_dc_with_runner",
			(input.namespace_id, input.runner_name.clone()),
			{
				move |mut cache, key| async move {
					let dc_id = find_dc_with_runner_inner(ctx, input).await?;

					if let Some(dc_id) = dc_id {
						cache.resolve(&key, dc_id);
					}

					Ok(cache)
				}
			},
		)
		.await
		.map(|dc_label| Output { dc_label })
}

async fn find_dc_with_runner_inner(ctx: &OperationCtx, input: &Input) -> Result<Option<u16>> {
	// Check if this DC has any runners
	let res = ctx
		.op(super::list_for_ns::Input {
			namespace_id: input.namespace_id,
			name: Some(input.runner_name.clone()),
			include_stopped: false,
			created_before: None,
			limit: 1,
		})
		.await?;
	if !res.runners.is_empty() {
		return Ok(Some(ctx.config().dc_label()));
	}

	// Check if serverless runner config exists
	let res = ctx
		.op(namespace::ops::runner_config::get_global::Input {
			runners: vec![(input.namespace_id, input.runner_name.clone())],
		})
		.await?;
	if let Some(runner) = res.first() {
		match &runner.config {
			RunnerConfig::Serverless { max_runners, .. } => {
				// Check if runner config does not have a max runner count of 0
				if *max_runners != 0 {
					return Ok(Some(ctx.config().dc_label()));
				}
			}
		}
	}

	// Get namespace
	let namespace = ctx
		.op(namespace::ops::get_global::Input {
			namespace_ids: vec![input.namespace_id],
		})
		.await?
		.into_iter()
		.next()
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	// Fanout to all datacenters
	let runners =
		race_request_to_datacenters::<runners_list::ListQuery, runners_list::ListResponse, _>(
			ctx,
			Default::default(),
			"/runners",
			runners_list::ListQuery {
				namespace: namespace.name,
				name: Some(input.runner_name.clone()),
				runner_ids: None,
				include_stopped: Some(false),
				limit: Some(1),
				cursor: None,
			},
			|res| !res.runners.is_empty(),
		)
		.await?;

	Ok(runners.map(|x| x.0))
}

const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// Helper fn that will send a request to all datacenters and return the response of the first
/// datacenter that matches `filter`.
pub async fn race_request_to_datacenters<Q, R, F>(
	ctx: &OperationCtx,
	headers: HeaderMap,
	endpoint: &str,
	query: Q,
	filter: F,
) -> Result<Option<(u16, R)>>
where
	R: DeserializeOwned + Send + 'static,
	Q: Serialize + Clone + Send + 'static,
	F: Fn(&R) -> bool,
{
	// Create futures for all dcs except the current
	let dcs = &ctx.config().topology().datacenters;
	let mut responses = futures_util::stream::iter(
		dcs.iter()
			.filter(|dc| dc.datacenter_label != ctx.config().dc_label())
			.map(|dc| {
				let headers = headers.clone();
				let query = query.clone();
				async move {
					tokio::time::timeout(
						REQUEST_TIMEOUT,
						// Remote datacenter
						request_remote_datacenter::<R>(
							ctx.config(),
							dc.datacenter_label,
							&endpoint,
							Method::GET,
							headers,
							Some(&query),
							Option::<&()>::None,
						)
						.map_ok(|x| (dc.datacenter_label, x)),
					)
					.await
				}
			}),
	)
	.collect::<FuturesUnordered<_>>()
	.await;

	// Collect responses until we reach quorum or all futures complete
	while let Some(out) = responses.next().await {
		match out {
			std::result::Result::Ok(result) => match result {
				std::result::Result::Ok((dc_label, response)) => {
					if filter(&response) {
						return Ok(Some((dc_label, response)));
					}
				}
				std::result::Result::Err(err) => {
					tracing::warn!(?err, "received error from replica");
				}
			},
			std::result::Result::Err(err) => {
				tracing::warn!(?err, "received timeout from replica");
			}
		}
	}

	Ok(None)
}
