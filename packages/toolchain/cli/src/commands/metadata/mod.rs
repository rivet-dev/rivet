use anyhow::*;
use clap::Subcommand;
use toolchain::paths;

/// Commands for retrieving metadata about Rivet configuration
#[derive(Subcommand)]
pub enum SubCommand {
	/// Get the path to the project data directory
	Path,
	/// Get the current API endpoint
	ApiEndpoint,
	/// Get the current access token
	AccessToken,
	/// Get the current project name ID
	ProjectNameId,
	/// Check if the user is currently logged in
	AuthStatus,
}

impl SubCommand {
	pub async fn execute(&self) -> Result<()> {
		match self {
			SubCommand::Path => {
				println!(
					"{}",
					paths::project_data_dir(&paths::data_dir().expect("data_dir"))
						.expect("project_data_dir")
						.display()
				);
				Ok(())
			}
			SubCommand::ApiEndpoint => {
				let ctx = crate::util::login::load_or_login().await?;
				println!("{}", ctx.api_endpoint);
				Ok(())
			}
			SubCommand::AccessToken => {
				let ctx = crate::util::login::load_or_login().await?;
				println!("{}", ctx.access_token);
				Ok(())
			}
			SubCommand::AuthStatus => {
				let is_logged_in = toolchain::toolchain_ctx::has_cloud_config().await?;
				println!("{}", is_logged_in);
				Ok(())
			}
			SubCommand::ProjectNameId => {
				let ctx = crate::util::login::load_or_login().await?;
				println!("{}", ctx.project.name_id);
				Ok(())
			}
		}
	}
}
