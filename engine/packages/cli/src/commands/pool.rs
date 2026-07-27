use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use reqwest::Method;

use crate::{
	DEFAULT_CLOUD_API, DEFAULT_NAMESPACE,
	cloud::{CloudClient, TokenInspectResponse, get_namespace, list_pools},
	credentials::resolve_token,
};

#[derive(Parser)]
pub struct Opts {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	/// List the compute pools in a namespace.
	List(ListOpts),
}

#[derive(Parser)]
pub struct ListOpts {
	/// Rivet Cloud API token.
	#[arg(long)]
	token: Option<String>,
	/// Cloud namespace to list pools from.
	#[arg(long, default_value = DEFAULT_NAMESPACE)]
	namespace: String,
	/// Override project from /tokens/api/inspect.
	#[arg(long)]
	project: Option<String>,
	/// Override organization from /tokens/api/inspect.
	#[arg(long)]
	org: Option<String>,
	/// Emit raw JSON instead of formatted output.
	#[arg(long)]
	json: bool,
	/// Cloud API endpoint.
	#[arg(long, default_value = DEFAULT_CLOUD_API)]
	cloud_api: String,
}

impl Opts {
	pub async fn execute(self) -> Result<()> {
		match self.command {
			Commands::List(opts) => opts.execute().await,
		}
	}
}

impl ListOpts {
	pub async fn execute(self) -> Result<()> {
		let token = resolve_token(self.token.as_deref())?;
		let cloud = CloudClient::new(&self.cloud_api, token)?;

		let inspect: TokenInspectResponse = cloud
			.request(Method::GET, "/tokens/api/inspect", None)
			.await?
			.context("token inspect returned no body")?;
		let project = self.project.clone().unwrap_or(inspect.project);
		let org = self.org.clone().unwrap_or(inspect.organization);
		let namespace = get_namespace(&cloud, &project, &org, &self.namespace).await?;

		let pools = list_pools(&cloud, &project, &org, &namespace.name).await?;

		if self.json {
			println!("{}", serde_json::to_string(&pools)?);
		} else {
			for pool in &pools {
				let status = pool.status.as_deref().unwrap_or("unknown");
				println!("{}\t{}", pool.name, status);
			}
		}
		Ok(())
	}
}
