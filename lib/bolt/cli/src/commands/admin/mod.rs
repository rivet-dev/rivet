use anyhow::*;
use bolt_core::{
	context::ProjectContext,
	tasks::{self},
	utils,
};
use clap::Parser;

mod cluster;

#[derive(Parser)]
pub enum SubCommand {
	/// Generates a login link for the given access token. Automatically turns the existing user into an
	/// admin (or creates a new admin if no user).
	Login {
		#[clap(default_value = "root")]
		name: String,
	},
	/// Cluster related operations
	Cluster {
		#[clap(subcommand)]
		command: cluster::SubCommand,
	},
}

impl SubCommand {
	pub async fn execute(self, ctx: ProjectContext) -> Result<()> {
		match self {
			Self::Login { name } => {
				assert!(
					ctx.ns().rivet.login.enable_admin,
					"admin login is not enabled in the namespace config (rivet.admin_login)"
				);

				tasks::api::access_token_login(&ctx, name).await?;

				utils::ringadingding();

				Ok(())
			}
			Self::Cluster { command } => command.execute(ctx).await,
		}
	}
}
