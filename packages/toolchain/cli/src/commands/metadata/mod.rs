use anyhow::*;
use clap::Subcommand;
use toolchain::paths;

#[derive(Subcommand)]
pub enum SubCommand {
	Path,
	ApiEndpoint,
	AccessToken,
	ProjectNameId,
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
				let is_logged_in = crate::util::login::is_logged_in().await?;
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
