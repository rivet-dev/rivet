use anyhow::*;
use rivet_api::{
	apis::{admin_api, configuration::Configuration},
	models,
};

pub fn private_api_config(config: &rivet_config::Config) -> Result<Configuration> {
	let server_config = config.server().map_err(|err| anyhow!("{err}"))?;
	let admin_token =
		server_config.rivet.token.admin.as_ref().context(
			"admin api not enabled. configure this by setting server.rivet.token.admin.",
		)?;
	let api_private_config = &server_config.rivet.api_private;

	Ok(Configuration {
		base_path: api_private_config.internal_origin().to_string().trim_end_matches("/").to_string(),
		bearer_access_token: Some(admin_token.read().clone()),
		..Default::default()
	})
}

/// Creates a login link for the hub.
pub async fn admin_login_url(config: &rivet_config::Config, username: String) -> Result<String> {
	let api_config = private_api_config(config)?;
	let url = admin_api::admin_login(&api_config, models::AdminLoginRequest { name: username })
		.await?
		.url;

	Ok(url)
}
