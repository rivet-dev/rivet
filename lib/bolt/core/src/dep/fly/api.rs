use anyhow::{ensure, Context, Result};
use serde::Deserialize;

use crate::context::ProjectContext;

pub fn app_name_belongs_to_namespace(ctx: &ProjectContext, app_name: &str) -> bool {
	app_name.starts_with(&format!("{}-", ctx.ns_id()))
}

async fn auth_token(ctx: &ProjectContext) -> Result<String> {
	ctx.read_secret(&["fly", "auth_token"]).await
}

#[derive(Deserialize, Debug)]
pub struct App {
	pub name: String,
}

pub async fn list_apps(ctx: &ProjectContext) -> Result<Vec<App>> {
	let fly = ctx.ns().fly.as_ref().context("missing fly")?;

	#[derive(Deserialize, Debug)]
	pub struct Response {
		apps: Vec<App>,
	}

	let res = reqwest::Client::new()
		.get(format!(
			"https://api.machines.dev/v1/apps?org_slug={}",
			"rivet-gg"
		))
		.bearer_auth(auth_token(ctx).await?)
		.send()
		.await?
		.error_for_status()?
		.json::<Response>()
		.await?;

	let apps = res
		.apps
		.into_iter()
		.filter(|x| app_name_belongs_to_namespace(ctx, &x.name))
		.collect::<Vec<_>>();

	Ok(apps)
}

pub async fn delete_app(ctx: &ProjectContext, app: &str) -> Result<()> {
	let _fly = ctx.ns().fly.as_ref().context("missing fly")?;

	ensure!(
		app_name_belongs_to_namespace(ctx, app),
		"app does not belong to namespace"
	);

	reqwest::Client::new()
		.delete(format!("https://api.machines.dev/v1/apps/{app}"))
		.bearer_auth(auth_token(ctx).await?)
		.send()
		.await?
		.error_for_status()?;

	Ok(())
}
