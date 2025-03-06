use std::collections::HashMap;

use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[derive(Debug, serde::Deserialize)]
struct VerifyResponse {
	success: bool,
	#[serde(rename = "challenge_ts")]
	challenge_ts: Option<String>,
	hostname: Option<String>,
	#[serde(rename = "error-codes")]
	error_codes: Option<Vec<String>>,
}

#[operation(name = "captcha-turnstile-verify")]
async fn handle(
	ctx: OperationContext<captcha::turnstile_verify::Request>,
) -> GlobalResult<captcha::turnstile_verify::Response> {
	let client = reqwest::Client::new();

	let mut params = HashMap::new();
	params.insert("response", &ctx.client_response);
	params.insert("secret", &ctx.secret_key);
	params.insert("remoteip", &ctx.remote_address);

	let res = client
		.post("https://challenges.cloudflare.com/turnstile/v0/siteverify")
		.form(&params)
		.send()
		.await?
		.json::<VerifyResponse>()
		.await?;
	tracing::info!(?res, "captcha response");

	Ok(captcha::turnstile_verify::Response {
		success: res.success,
	})
}
