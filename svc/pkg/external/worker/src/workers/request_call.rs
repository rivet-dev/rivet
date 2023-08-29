use std::str::FromStr;

use chirp_worker::prelude::*;
use proto::backend::{net::HttpMethod, pkg::*};
use reqwest::{
	header::{HeaderMap, HeaderName, HeaderValue},
	StatusCode,
};

#[tracing::instrument]
async fn fail(
	ctx: &OperationContext<external::msg::request_call::Message>,
	request_id: Uuid,
	error_code: external::msg::request_call_fail::ErrorCode,
) -> GlobalResult<()> {
	msg!([ctx] external::msg::request_call_fail(request_id) {
		request_id: Some(request_id.into()),
		error_code: error_code as i32,
	})
	.await?;

	Ok(())
}

#[worker(name = "external-request-call")]
async fn worker(ctx: &OperationContext<external::msg::request_call::Message>) -> GlobalResult<()> {
	let request_id = internal_unwrap!(ctx.request_id).as_uuid();
	let config = internal_unwrap!(ctx.config);

	let req = reqwest::Client::new();

	// Add method
	let method = internal_unwrap_owned!(HttpMethod::from_i32(config.method), "invalid http method");
	let req = match method {
		HttpMethod::Get => req.get(&config.url),
		HttpMethod::Post => req.post(&config.url),
		HttpMethod::Put => req.put(&config.url),
		HttpMethod::Delete => req.delete(&config.url),
	};

	// Add body
	// TODO: Figure out a way to not clone this
	let req = if let Some(body) = ctx.body.clone() {
		req.body(body)
	} else {
		req
	};

	// Add headers (NOTE: If using a config that has been validated by `external-request-validate`, the
	// creation of this header map should be infallible)
	let headers = config
		.headers
		.iter()
		.map(|(k, v)| Ok((HeaderName::from_str(k)?, HeaderValue::from_str(v)?)))
		.collect::<GlobalResult<HeaderMap>>()?;
	let req = req.headers(headers);

	// Add timeout
	let req = if ctx.timeout != 0 {
		let timeout = std::time::Duration::from_millis(ctx.timeout);
		req.timeout(timeout)
	} else {
		req
	};

	// Execute
	let (status, body) = match req.send().await {
		Ok(res) => (
			res.status().as_u16(),
			if ctx.read_response_body {
				Some(res.bytes().await?.to_vec())
			} else {
				None
			},
		),
		Err(err) => {
			// Coerce reqwest timeout into http request timeout
			if err.is_timeout() {
				(StatusCode::REQUEST_TIMEOUT.as_u16(), None)
			} else {
				tracing::info!(?err, "external request failed");

				fail(
					ctx,
					request_id,
					external::msg::request_call_fail::ErrorCode::RequestFailed,
				)
				.await?;

				return Ok(());
			}
		}
	};

	msg!([ctx] external::msg::request_call_complete(request_id) {
		request_id: ctx.request_id,
		status_code: status as u32,
		body: body,
	})
	.await?;

	Ok(())
}
