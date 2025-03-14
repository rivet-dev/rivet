use anyhow::*;
use clap::Subcommand;

/// Commands for managing Rivet configuration
#[derive(Subcommand)]
pub enum SubCommand {
	/// Validate the current configuration file
	Validate {
		/// Output the validated configuration as JSON
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
