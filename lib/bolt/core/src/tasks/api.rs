use anyhow::*;
use rivet_api::{apis::admin_clusters_api, models};
use serde_json::json;

use crate::context::ProjectContext;

/// Creates a login link for the hub.
pub async fn access_token_login(project_ctx: &ProjectContext, name: String) -> Result<()> {
	rivet_term::status::progress("Logging in as", &name);

	let api_admin_token = project_ctx
		.read_secret(&["rivet", "api_admin", "token"])
		.await?;
	let response = reqwest::Client::new()
		.post(format!("{}/admin/login", project_ctx.origin_api(),))
		.bearer_auth(api_admin_token)
		.json(&json!({
			"name": name,
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
	let url = body
		.get("url")
		.expect("url in login body")
		.as_str()
		.unwrap();

	eprintln!();
	rivet_term::status::success("Login with this url", "");
	eprintln!("{url}");

	Ok(())
}

pub async fn get_cluster_server_ips(
	project_ctx: &ProjectContext,
	server_id: Option<&str>,
	pools: Option<&str>,
) -> Result<Vec<String>> {
	let server_ips = admin_clusters_api::admin_clusters_get_server_ips(
		&project_ctx.openapi_config_cloud().await?,
		server_id,
		pools
			.map(|p| match p {
				"job" => Ok(models::AdminPoolType::Job),
				"gg" => Ok(models::AdminPoolType::Gg),
				"ats" => Ok(models::AdminPoolType::Ats),
				_ => Err(anyhow!("invalid pool type")),
			})
			.transpose()?,
	)
	.await?;

	Ok(server_ips.ips)
}
