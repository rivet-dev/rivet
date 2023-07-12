use std::str::FromStr;

use chirp_worker::prelude::*;
use proto::backend::{net::HttpMethod, pkg::*};
use reqwest::{
	header::{HeaderMap, HeaderName, HeaderValue},
	StatusCode,
};

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

	// Add headers
	let headers = config
		.headers
		.iter()
		.map(|(k, v)| Ok((HeaderName::from_str(k)?, HeaderValue::from_str(v)?)))
		.collect::<GlobalResult<HeaderMap>>()?;
	let req = req.headers(headers);

	// Add timeout
	let req = if config.timeout != 0 {
		let timeout = std::time::Duration::from_millis(config.timeout);
		req.timeout(timeout)
	} else {
		req
	};

	// Execute
	let (status, body) = match req.send().await {
		Ok(res) => (res.status().as_u16(), res.bytes().await?.to_vec()),
		Err(err) => {
			// Coerce reqwest timeout into http request timeout
			if err.is_timeout() {
				(StatusCode::REQUEST_TIMEOUT.as_u16(), Vec::new())
			} else {
				// Retry worker
				return Err(err.into());
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
