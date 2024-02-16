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

#[operation(name = "captcha-hcaptcha-verify")]
async fn handle(
	ctx: OperationContext<captcha::hcaptcha_verify::Request>,
) -> GlobalResult<captcha::hcaptcha_verify::Response> {
	let client = reqwest::Client::new();

	let secret_key = if let Some(secret_key) = &ctx.secret_key {
		secret_key.clone()
	} else {
		util::env::read_secret(&["hcaptcha", "secret"]).await?
	};

	let res = client
		.post("https://hcaptcha.com/siteverify")
		.header("content-type", "application/x-www-form-urlencoded")
		.body(format!(
			"response={client_response}&secret={secret}&sitekey={site_key}&remoteip={remote_address}",
			client_response = ctx.client_response,
			secret = secret_key,
			site_key = ctx.site_key,
			remote_address = ctx.remote_address,
		))
		.send()
		.await?
		.json::<VerifyResponse>()
		.await?;
	tracing::info!(?res, "captcha response");

	Ok(captcha::hcaptcha_verify::Response {
		success: res.success,
	})
}
