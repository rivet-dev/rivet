use std::io::IsTerminal;

use anyhow::{Context, Result, bail};
use clap::Parser;
use reqwest::Method;

use crate::{
	DEFAULT_CLOUD_API,
	auth::resolve_auth,
	cloud::{CloudClient, TokenInspectResponse},
	target::{Link, ProjectTarget, pick_target, remove_link, write_link},
};

#[derive(Parser)]
pub struct LinkOpts {
	/// Rivet Cloud API token. Overrides `rivet login`.
	#[arg(long)]
	token: Option<String>,
	/// Organization slug to link. Prompts when omitted.
	#[arg(long)]
	org: Option<String>,
	/// Project name to link. Prompts when omitted.
	#[arg(long)]
	project: Option<String>,
	/// Cluster name to link instead of a project. `--org` names the
	/// organization it belongs to.
	#[arg(long, conflicts_with = "project")]
	cluster: Option<String>,
	/// Cloud API endpoint.
	#[arg(long, default_value = DEFAULT_CLOUD_API)]
	cloud_api: String,
}

impl LinkOpts {
	pub async fn execute(self) -> Result<()> {
		if let Some(cluster) = self.cluster {
			let path = write_link(&Link::Cluster {
				cluster: cluster.clone(),
				org: self.org.clone(),
			})?;
			eprintln!("linked cluster {cluster} in {}", path.display());
			return Ok(());
		}

		let target = match (self.org, self.project) {
			(Some(org), Some(project)) => ProjectTarget { org, project },
			(org, project) => {
				let auth = resolve_auth(self.token.as_deref(), &self.cloud_api).await?;
				let cloud = CloudClient::new(&self.cloud_api, auth.token.clone())?
					.with_login_refresh(&auth, &self.cloud_api);
				if auth.is_login() {
					if !std::io::stdin().is_terminal() {
						bail!("pass --org and --project to link non-interactively");
					}
					pick_target(&cloud, org.as_deref(), project.as_deref()).await?
				} else {
					let inspect: TokenInspectResponse = cloud
						.request(Method::GET, "/tokens/api/inspect", None)
						.await?
						.context("token inspect returned no body")?;
					ProjectTarget {
						org: org.unwrap_or(inspect.organization),
						project: project.unwrap_or(inspect.project),
					}
				}
			}
		};

		let path = write_link(&Link::Project {
			org: target.org.clone(),
			project: target.project.clone(),
		})?;
		eprintln!(
			"linked {}/{} in {}",
			target.org,
			target.project,
			path.display()
		);
		Ok(())
	}
}

#[derive(Parser)]
pub struct UnlinkOpts {}

impl UnlinkOpts {
	pub async fn execute(self) -> Result<()> {
		match remove_link()? {
			Some(path) => eprintln!("removed {}", path.display()),
			None => eprintln!("this directory is not linked"),
		}
		Ok(())
	}
}
