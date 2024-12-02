use anyhow::*;
use clap::Subcommand;
use toolchain::paths;

#[derive(Subcommand)]
pub enum SubCommand {
	Path,
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
		}
	}
}
