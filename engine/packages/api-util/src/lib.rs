use anyhow::{Context, Result};
use axum::{body::Body, response::Response};
use futures_util::StreamExt;
use rivet_api_builder::{ApiCtx, ErrorResponse, RawErrorResponse, X_RIVET_RAY_ID};
use serde::{Serialize, de::DeserializeOwned};
use std::future::Future;

pub mod errors;

pub use axum::http::{HeaderMap, Method};

/// Sends a request to a remote datacenter with proper error context.
async fn send_request(
	request: reqwest::RequestBuilder,
	dc_label: u16,
	url: &str,
) -> Result<reqwest::Response> {
	request.send().await.with_context(|| {
		format!("failed to send request to remote dc (dc: {dc_label}, url: {url})")
	})
}

/// Generic function to make raw requests to remote datacenters by label (returns axum Response)
#[tracing::instrument(skip(ctx, query, body))]
pub async fn request_remote_datacenter_raw(
	ctx: &ApiCtx,
	dc_label: u16,
	endpoint: &str,
	method: Method,
	query: Option<&impl Serialize>,
	body: Option<&impl Serialize>,
) -> Result<Response> {
	let dc = ctx
		.config()
		.dc_for_label(dc_label)
		.ok_or_else(|| errors::Datacenter::NotFound.build())?;

	let client = rivet_pools::reqwest::client().await?;
	let mut url = dc.peer_url.join(endpoint)?;

	// NOTE: We don't use reqwest's `.query` because it doesn't support list query parameters
	if let Some(q) = query {
		url.set_query(Some(&serde_html_form::to_string(q)?));
	}

	tracing::debug!(%method, %url, "sending raw request to remote datacenter");

	let url_string = url.to_string();
	let mut request = client.request(method, url);

	if let Some(b) = body {
		request = request.json(b);
	}

	let res = send_request(request, dc_label, &url_string).await?;
	reqwest_to_axum_response(res).await
}

/// Generic function to make requests to a specific datacenter
#[tracing::instrument(skip(config, query, body))]
pub async fn request_remote_datacenter<T>(
	config: &rivet_config::Config,
	dc_label: u16,
	endpoint: &str,
	method: Method,
	query: Option<&impl Serialize>,
	body: Option<&impl Serialize>,
) -> Result<T>
where
	T: DeserializeOwned,
{
	let dc = config
		.dc_for_label(dc_label)
		.ok_or_else(|| errors::Datacenter::NotFound.build())?;

	let client = rivet_pools::reqwest::client().await?;
	let mut url = dc.peer_url.join(endpoint)?;

	// NOTE: We don't use reqwest's `.query` because it doesn't support list query parameters
	if let Some(q) = query {
		url.set_query(Some(&serde_html_form::to_string(q)?));
	}

	tracing::debug!(%method, %url, "sending request to remote datacenter");

	let url_string = url.to_string();
	let mut request = client.request(method, url);

	if let Some(b) = body {
		request = request.json(b);
	}

	let res = send_request(request, dc_label, &url_string).await?;
	parse_response::<T>(res).await
}

/// Generic function to fanout requests to all datacenters and aggregate results
/// Returns aggregated results and errors only if all requests fail
#[tracing::instrument(skip(ctx, query, local_handler, aggregator))]
pub async fn fanout_to_datacenters<I, Q, F, Fut, A, R>(
	ctx: ApiCtx,
	endpoint: &str,
	query: Q,
	local_handler: F,
	aggregator: A,
) -> Result<R>
where
	I: DeserializeOwned + Send + 'static,
	Q: Serialize + Clone + Send + 'static,
	F: Fn(ApiCtx, Q) -> Fut + Clone + Send + 'static,
	Fut: Future<Output = Result<I>> + Send,
	A: Fn(u16, I, &mut R),
	R: Default + Send + 'static,
{
	let dcs = ctx.config().topology().datacenters.clone();

	let results = futures_util::stream::iter(dcs)
		.map(|dc| {
			let ctx = ctx.clone();
			let query = query.clone();
			let endpoint = endpoint.to_string();
			let local_handler = local_handler.clone();

			async move {
				if dc.datacenter_label == ctx.config().dc_label() {
					// Local datacenter - use direct API call
					(dc.datacenter_label, local_handler(ctx, query).await)
				} else {
					// Remote datacenter - HTTP request
					(
						dc.datacenter_label,
						request_remote_datacenter::<I>(
							ctx.config(),
							dc.datacenter_label,
							&endpoint,
							Method::GET,
							Some(&query),
							Option::<&()>::None,
						)
						.await,
					)
				}
			}
		})
		.buffer_unordered(16)
		.collect::<Vec<_>>()
		.await;

	// Aggregate results
	let result_count = results.len();
	let mut errors = Vec::new();
	let mut aggregated = R::default();
	for (dc_label, res) in results {
		match res {
			Ok(data) => aggregator(dc_label, data, &mut aggregated),
			Err(err) => {
				tracing::error!(?dc_label, ?err, "failed to request edge dc");
				errors.push(err);
			}
		}
	}

	// Error only if all requests failed
	if result_count == errors.len() {
		if let Some(res) = errors.into_iter().next() {
			return Err(res).context("all datacenter requests failed");
		}
	}

	Ok(aggregated)
}

#[tracing::instrument(skip_all)]
pub async fn reqwest_to_axum_response(reqwest_response: reqwest::Response) -> Result<Response> {
	let status = reqwest_response.status();
	let headers = reqwest_response.headers().clone();
	let ray_id = headers
		.get(X_RIVET_RAY_ID)
		.and_then(|v| v.to_str().ok())
		.map(|x| x.to_string());
	let body_bytes = reqwest_response.bytes().await?;

	if !status.is_success() {
		let body_text = String::from_utf8_lossy(&body_bytes);
		anyhow::bail!(
			"remote dc returned error (status: {status}, ray_id: {ray_id:?}, body: {body_text})"
		);
	}

	let mut response = Response::builder()
		.status(status)
		.body(Body::from(body_bytes))?;

	*response.headers_mut() = headers;

	Ok(response)
}

#[tracing::instrument(skip_all)]
pub async fn parse_response<T: DeserializeOwned>(reqwest_response: reqwest::Response) -> Result<T> {
	let status = reqwest_response.status();
	let headers = reqwest_response.headers();
	let ray_id = headers
		.get(X_RIVET_RAY_ID)
		.and_then(|v| v.to_str().ok())
		.map(|x| x.to_string());
	let response_text = reqwest_response.text().await?;

	if status.is_success() {
		serde_json::from_str::<T>(&response_text).with_context(|| {
			format!(
				"failed to parse response from remote dc (ray_id: {ray_id:?}, body: {response_text})"
			)
		})
	} else {
		let error_response = serde_json::from_str::<ErrorResponse>(&response_text)
			.with_context(|| {
				format!("failed to parse error response from remote dc (status: {status}, ray_id: {ray_id:?}, body: {response_text})")
			})?;
		let error = RawErrorResponse(status, error_response);

		if let Some(ray_id) = ray_id {
			Err(error).with_context(|| format!("remote request failed (ray_id: {ray_id})"))
		} else {
			Err(error.into())
		}
	}
}
