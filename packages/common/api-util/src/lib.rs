use anyhow::{Context, Result};
use axum::{body::Body, response::Response};
use futures_util::StreamExt;
use rivet_api_builder::{ApiCtx, ErrorResponse, RawErrorResponse};
use serde::{Serialize, de::DeserializeOwned};
use std::future::Future;

mod errors;

pub use axum::http::{HeaderMap, Method};

/// Generic function to make raw requests to remote datacenters by label (returns axum Response)
pub async fn request_remote_datacenter_raw(
	ctx: &ApiCtx,
	dc_label: u16,
	endpoint: &str,
	method: Method,
	headers: HeaderMap,
	query: Option<&impl Serialize>,
	body: Option<&impl Serialize>,
) -> Result<Response> {
	let dc = ctx
		.config()
		.dc_for_label(dc_label)
		.ok_or_else(|| errors::Datacenter::NotFound.build())?;

	let client = rivet_pools::reqwest::client().await?;
	let mut url = dc.api_peer_url.join(endpoint)?;

	// NOTE: We don't use reqwest's `.query` because it doesn't support list query parameters
	if let Some(q) = query {
		url.set_query(Some(&serde_html_form::to_string(q)?));
	}

	let mut request = client.request(method, url).headers(headers);

	if let Some(b) = body {
		request = request.json(b);
	}

	let res = request
		.send()
		.await
		.context("failed sending request to remote dc")?;
	reqwest_to_axum_response(res)
		.await
		.context("failed parsing response from remote dc")
}

/// Generic function to make requests to a specific datacenter
pub async fn request_remote_datacenter<T>(
	config: &rivet_config::Config,
	dc_label: u16,
	endpoint: &str,
	method: Method,
	headers: HeaderMap,
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
	let mut url = dc.api_peer_url.join(endpoint)?;

	// NOTE: We don't use reqwest's `.query` because it doesn't support list query parameters
	if let Some(q) = query {
		url.set_query(Some(&serde_html_form::to_string(q)?));
	}

	let mut request = client.request(method, url).headers(headers);

	if let Some(b) = body {
		request = request.json(b);
	}

	let res = request
		.send()
		.await
		.context("failed sending request to remote dc")?;
	parse_response::<T>(res)
		.await
		.context("failed parsing response from remote dc")
}

/// Generic function to fanout requests to all datacenters and aggregate results
/// Returns aggregated results and errors only if all requests fail
pub async fn fanout_to_datacenters<I, Q, F, Fut, A, R>(
	ctx: ApiCtx,
	headers: HeaderMap,
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
	A: Fn(I, &mut R),
	R: Default + Send + 'static,
{
	let dcs = &ctx.config().topology().datacenters;

	let results = futures_util::stream::iter(dcs.clone().into_iter().map(|dc| {
		let ctx = ctx.clone();
		let headers = headers.clone();
		let query = query.clone();
		let endpoint = endpoint.to_string();
		let local_handler = local_handler.clone();

		async move {
			if dc.datacenter_label == ctx.config().dc_label() {
				// Local datacenter - use direct API call
				local_handler(ctx, query).await
			} else {
				// Remote datacenter - HTTP request
				request_remote_datacenter::<I>(
					ctx.config(),
					dc.datacenter_label,
					&endpoint,
					Method::GET,
					headers,
					Some(&query),
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
	let mut aggregated = R::default();
	for res in results {
		match res {
			Ok(data) => aggregator(data, &mut aggregated),
			Err(err) => {
				tracing::error!(?err, "failed to request edge dc");
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

pub async fn reqwest_to_axum_response(reqwest_response: reqwest::Response) -> Result<Response> {
	let status = reqwest_response.status();
	let headers = reqwest_response.headers().clone();
	let body_bytes = reqwest_response.bytes().await?;

	let mut response = Response::builder()
		.status(status)
		.body(Body::from(body_bytes))?;

	*response.headers_mut() = headers;

	Ok(response)
}

pub async fn parse_response<T: DeserializeOwned>(reqwest_response: reqwest::Response) -> Result<T> {
	let status = reqwest_response.status();
	let response_text = reqwest_response.text().await?;

	if status.is_success() {
		serde_json::from_str::<T>(&response_text).map_err(Into::into)
	} else {
		Err(RawErrorResponse(
			status,
			serde_json::from_str::<ErrorResponse>(&response_text)?,
		)
		.into())
	}
}
