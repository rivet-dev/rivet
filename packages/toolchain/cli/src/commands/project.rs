use anyhow::*;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum SubCommand {
	#[clap(alias = "v")]
	View,
}

impl SubCommand {
	pub async fn execute(&self) -> Result<()> {
		match &self {
			SubCommand::View => {
				let ctx = toolchain::toolchain_ctx::load().await?;

				let url = format!(
					"{hub}/projects/{proj}",
					hub = ctx.bootstrap.origins.hub,
					proj = ctx.project.name_id,
				);

				webbrowser::open_browser_with_options(
					webbrowser::Browser::Default,
					&url,
					webbrowser::BrowserOptions::new().with_suppress_output(true),
				)?;

				Ok(())
			}
		}
	}
}
