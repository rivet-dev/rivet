use anyhow::Result;
use clap::Parser;

use crate::{
	DEFAULT_CLOUD_API,
	auth::{self, resolve_auth},
	target::{Link, find_link},
};

#[derive(Parser)]
pub struct LoginOpts {
	/// Print the sign-in URL instead of opening a browser.
	#[arg(long)]
	no_browser: bool,
	/// Cloud API endpoint.
	#[arg(long, default_value = DEFAULT_CLOUD_API)]
	cloud_api: String,
}

impl LoginOpts {
	pub async fn execute(self) -> Result<()> {
		let session = auth::login(&self.cloud_api, !self.no_browser).await?;
		eprintln!("signed in as {}", session.user.email);
		Ok(())
	}
}

#[derive(Parser)]
pub struct LogoutOpts {
	/// Cloud API endpoint.
	#[arg(long, default_value = DEFAULT_CLOUD_API)]
	cloud_api: String,
}

impl LogoutOpts {
	pub async fn execute(self) -> Result<()> {
		if auth::logout(&self.cloud_api).await? {
			eprintln!("signed out");
		} else {
			eprintln!("not signed in");
		}
		Ok(())
	}
}

#[derive(Parser)]
pub struct WhoamiOpts {
	/// Rivet Cloud API token to describe instead of the active credentials.
	#[arg(long)]
	token: Option<String>,
	/// Cloud API endpoint.
	#[arg(long, default_value = DEFAULT_CLOUD_API)]
	cloud_api: String,
}

impl WhoamiOpts {
	pub async fn execute(self) -> Result<()> {
		let auth = resolve_auth(self.token.as_deref(), &self.cloud_api).await?;
		match &auth.user {
			Some(user) if user.name.is_empty() => println!("{}", user.email),
			Some(user) => println!("{} <{}>", user.name, user.email),
			None => println!("Cloud API token"),
		}
		eprintln!("credentials: {}", auth.source.describe());
		match find_link()? {
			Some((Link::Project { org, project }, path)) => {
				eprintln!("linked project: {org}/{project} ({})", path.display());
			}
			Some((Link::Cluster { cluster, .. }, path)) => {
				eprintln!("linked cluster: {cluster} ({})", path.display());
			}
			None => {}
		}
		Ok(())
	}
}
