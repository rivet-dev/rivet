use anyhow::*;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum SubCommand {
	Validate {
		#[clap(long)]
		json: bool,
	},
}

impl SubCommand {
	pub async fn execute(&self) -> Result<()> {
		match &self {
			SubCommand::Validate { json } => {
				let config = toolchain::config::Config::load(None).await?;
				if *json {
					println!("{}", serde_json::to_string_pretty(&config)?);
				} else {
					println!("Config is valid.");
				}

				Ok(())
			}
		}
	}
}
