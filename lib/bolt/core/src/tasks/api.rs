use anyhow::*;
use serde_json::json;
use uuid::Uuid;

use crate::context::ProjectContext;

/// Converts a team into a developer team via the Rivet API.
pub async fn convert_team(project_ctx: &ProjectContext, team_id: String) -> Result<()> {
	if let Err(err) = Uuid::parse_str(&team_id) {
		bail!("failed to parse uuid: {}", err);
	}

	eprintln!();
	rivet_term::status::progress("Converting team", &team_id);

	let api_admin_token = project_ctx
		.read_secret(&["rivet", "api_admin", "token"])
		.await?;
	let response = reqwest::Client::new()
		.post(format!(
			"https://admin.api.{}/v1/groups/{}/developer",
			project_ctx.domain_main(),
			team_id,
		))
		.header(
			reqwest::header::AUTHORIZATION,
			reqwest::header::HeaderValue::from_str(&format!("Bearer {api_admin_token}"))?,
		)
		.json(&json!({}))
		.send()
		.await?;

	if !response.status().is_success() {
		bail!(
			"failed to convert team ({}):\n{:#?}",
			response.status().as_u16(),
			response.json().await?
		);
	}

	eprintln!();
	rivet_term::status::success("Converted", "");

	Ok(())
}
