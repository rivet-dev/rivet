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
	ctx: OperationContext<cf::turnstile_verify::Request>,
) -> GlobalResult<cf::turnstile_verify::Response> {
	let client = reqwest::Client::new();

	let res = client
		.post("https://challenges.cloudflare.com/turnstile/v0/siteverify")
		.header("content-type", "application/x-www-form-urlencoded")
		.body(format!(
			"response={client_response}&secret={secret}&remoteip={remote_address}",
			client_response = ctx.client_response,
			secret = ctx.secret_key,
			remote_address = ctx.remote_address,
		))
		.send()
		.await?
		.json::<VerifyResponse>()
		.await?;
	tracing::info!(?res, "captcha response");

	Ok(cf::turnstile_verify::Response {
		success: res.success,
	})
}
