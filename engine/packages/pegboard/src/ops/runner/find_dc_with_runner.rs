use std::time::Duration;

use anyhow::Result;
use futures_util::{FutureExt, StreamExt, TryFutureExt, stream::FuturesUnordered};
use gas::prelude::*;
use rivet_api_types::{runner_configs::list as runner_configs_list, runners::list as runners_list};
use rivet_api_util::{Method, request_remote_datacenter};
use rivet_types::runner_configs::RunnerConfigKind;
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
	// Check if this DC has any non-draining runners
	let res = ctx
		.op(super::list_for_ns::Input {
			namespace_id: input.namespace_id,
			name: Some(input.runner_name.clone()),
			include_stopped: false,
			created_before: None,
			limit: 16,
		})
		.await?;
	if res
		.runners
		.iter()
		.filter(|runner| runner.drain_ts.is_none())
		.count()
		!= 0
	{
		return Ok(Some(ctx.config().dc_label()));
	}

	// Check if a serverless runner config with a max runners > 0 exists
	let res = ctx
		.op(crate::ops::runner_config::get::Input {
			runners: vec![(input.namespace_id, input.runner_name.clone())],
			bypass_cache: false,
		})
		.await?;
	if let Some(runner) = res.first() {
		match &runner.config.kind {
			RunnerConfigKind::Serverless { max_runners, .. } => {
				if *max_runners != 0 {
					return Ok(Some(ctx.config().dc_label()));
				}
			}
			_ => {}
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

	// Fanout two requests to all datacenters: runner list, and runner config list (with specific name)
	let runners_fut =
		race_request_to_datacenters::<runners_list::ListQuery, runners_list::ListResponse, _>(
			ctx,
			"/runners",
			runners_list::ListQuery {
				namespace: namespace.name.clone(),
				name: Some(input.runner_name.clone()),
				runner_ids: None,
				runner_id: Vec::new(),
				include_stopped: Some(false),
				limit: Some(16),
				cursor: None,
			},
			// Check for non draining runners
			|res| {
				res.runners
					.iter()
					.filter(|runner| runner.drain_ts.is_none())
					.count() != 0
			},
		)
		.map(|res| res.map(|x| x.map(|x| x.0)))
		.boxed();

	let runner_configs_fut = race_request_to_datacenters::<
		runner_configs_list::ListQuery,
		runner_configs_list::ListResponse,
		_,
	>(
		ctx,
		"/runner-configs",
		runner_configs_list::ListQuery {
			namespace: namespace.name.clone(),
			variant: None,
			runner_names: None,
			runner_name: vec![input.runner_name.clone()],
			limit: None,
			cursor: None,
		},
		// Check for configs with a max runners > 0
		|res| {
			res.runner_configs
				.iter()
				.filter(|(_, rc)| match rc.config.kind {
					RunnerConfigKind::Serverless { max_runners, .. } => max_runners != 0,
					_ => false,
				})
				.count() != 0
		},
	)
	.map(|res| res.map(|x| x.map(|x| x.0)))
	.boxed();

	let mut futs = [runners_fut, runner_configs_fut]
		.into_iter()
		.collect::<FuturesUnordered<_>>();

	Ok(futs.next().await.transpose()?.flatten())
}

const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// Helper fn that will send a request to all datacenters and return the response of the first
/// datacenter that matches `filter`.
pub async fn race_request_to_datacenters<Q, R, F>(
	ctx: &OperationCtx,
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
	let mut responses = dcs
		.iter()
		.filter(|dc| dc.datacenter_label != ctx.config().dc_label())
		.map(|dc| {
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
						Some(&query),
						Option::<&()>::None,
					)
					.map_ok(|x| (dc.datacenter_label, x)),
				)
				.await
			}
		})
		.collect::<FuturesUnordered<_>>();

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
