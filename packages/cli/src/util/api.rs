use anyhow::*;
use serde_json::json;

/// Creates a login link for the hub.
pub async fn admin_login_url(config: &rivet_config::Config, username: String) -> Result<String> {
	let server_config = config.server().map_err(|err| anyhow!("{err}"))?;
	let admin_token =
		server_config.rivet.token.admin.as_ref().context(
			"admin api not enabled. configure this by setting server.rivet.token.admin.",
		)?;
	let api_private_config = &server_config.rivet.api_private;

	let response = reqwest::Client::new()
		.post(format!(
			"http://{}:{}/admin/login",
			api_private_config.host(),
			api_private_config.port()
		))
		.bearer_auth(admin_token.read())
		.json(&json!({
			"name": username,
		}))
		.send()
		.await?;

	if !response.status().is_success() {
		bail!(
			"failed to login ({}):\n{:#?}",
			response.status().as_u16(),
			response.json::<serde_json::Value>().await?
		);
	}

	let body = response.json::<serde_json::Value>().await?;
	let url = body.get("url").expect("url in login body").to_string();

	Ok(url)
}
