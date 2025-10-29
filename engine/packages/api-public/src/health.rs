use anyhow::{Result, bail};
use axum::{Json, extract::Extension, response::IntoResponse};
use futures_util::StreamExt;
use rivet_api_builder::ApiError;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use utoipa::ToSchema;

use crate::ctx::ApiCtx;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(as = HealthFanoutResponse)]
pub struct FanoutResponse {
	pub datacenters: Vec<DatacenterHealth>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DatacenterHealth {
	pub datacenter_label: u16,
	pub datacenter_name: String,
	pub status: HealthStatus,
	pub rtt_ms: Option<f64>,
	pub response: Option<HealthResponse>,
	pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
	Ok,
	Error,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
	pub runtime: String,
	pub status: String,
	pub version: String,
}

#[utoipa::path(
	get,
	operation_id = "health_fanout",
	path = "/health/fanout",
	responses(
		(status = 200, body = FanoutResponse),
	),
	security(("bearer_auth" = [])),
)]
#[tracing::instrument(skip_all)]
pub async fn fanout(Extension(ctx): Extension<ApiCtx>) -> impl IntoResponse {
	match fanout_inner(ctx).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

async fn fanout_inner(ctx: ApiCtx) -> Result<FanoutResponse> {
	// Require datacenter read permissions to access health status
	ctx.auth().await?;

	let dcs = &ctx.config().topology().datacenters;

	tracing::debug!(datacenters = dcs.len(), "starting health fanout");

	let results = futures_util::stream::iter(dcs.clone().into_iter().map(|dc| {
		let ctx = ctx.clone();

		async move {
			let start = Instant::now();

			if dc.datacenter_label == ctx.config().dc_label() {
				// Local datacenter - check directly
				let response = HealthResponse {
					runtime: "engine".to_string(),
					status: "ok".to_string(),
					version: env!("CARGO_PKG_VERSION").to_string(),
				};

				DatacenterHealth {
					datacenter_label: dc.datacenter_label,
					datacenter_name: dc.name.clone(),
					status: HealthStatus::Ok,
					rtt_ms: Some(start.elapsed().as_secs_f64() * 1000.0),
					response: Some(response),
					error: None,
				}
			} else {
				// Remote datacenter - HTTP request
				match send_health_checks(&dc).await {
					Ok(response) => DatacenterHealth {
						datacenter_label: dc.datacenter_label,
						datacenter_name: dc.name.clone(),
						status: HealthStatus::Ok,
						rtt_ms: Some(start.elapsed().as_secs_f64() * 1000.0),
						response: Some(response),
						error: None,
					},
					Err(err) => {
						tracing::warn!(
							?dc.datacenter_label,
							?err,
							"health check failed for datacenter"
						);

						DatacenterHealth {
							datacenter_label: dc.datacenter_label,
							datacenter_name: dc.name.clone(),
							status: HealthStatus::Error,
							rtt_ms: Some(start.elapsed().as_secs_f64() * 1000.0),
							response: None,
							error: Some(err.to_string()),
						}
					}
				}
			}
		}
	}))
	.buffer_unordered(16)
	.collect::<Vec<_>>()
	.await;

	tracing::debug!(results = results.len(), "health fanout completed");

	Ok(FanoutResponse {
		datacenters: results,
	})
}

#[tracing::instrument(skip_all)]
async fn send_health_checks(
	dc: &rivet_config::config::topology::Datacenter,
) -> Result<HealthResponse> {
	let client = rivet_pools::reqwest::client().await?;
	let peer_url = dc.peer_url.join("/health")?;
	let proxy_url = dc.proxy_url().join("/health")?;

	tracing::debug!(
		?dc.datacenter_label,
		?peer_url,
		?proxy_url,
		"sending health checks to remote datacenter"
	);

	let (peer_res, proxy_res) = tokio::try_join!(
		client
			.get(peer_url)
			.timeout(std::time::Duration::from_secs(5))
			.send(),
		client
			.get(proxy_url)
			.timeout(std::time::Duration::from_secs(5))
			.send()
	)?;

	if !peer_res.status().is_success() {
		bail!("Peer health check returned status: {}", peer_res.status())
	}

	if proxy_res.status().is_success() {
		let response = proxy_res.json::<HealthResponse>().await?;
		Ok(response)
	} else {
		bail!("Proxy health check returned status: {}", proxy_res.status())
	}
}
