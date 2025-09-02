use anyhow::Result;
use axum::{body::Body, response::Response};
use rivet_api_builder::{ErrorResponse, RawErrorResponse};
use serde::de::DeserializeOwned;

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
