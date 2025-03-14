use anyhow::*;
use clap::Subcommand;

mod list;
mod select;

/// Commands for managing environments
#[derive(Subcommand)]
pub enum SubCommand {
	/// Select and set the default environment
	#[clap(alias = "s")]
	Select(select::Opts),
	/// Open the environment dashboard in a browser
	#[clap(alias = "v")]
	View {
		/// Specify the environment to view (will prompt if not specified)
		#[clap(long, alias = "env", short = 'e')]
		environment: Option<String>,
	},
	/// List all available environments
	#[clap(alias = "ls")]
	List(list::Opts),
}

impl SubCommand {
	pub async fn execute(&self) -> Result<()> {
		match &self {
			SubCommand::List(opts) => opts.execute().await,
			SubCommand::Select(opts) => opts.execute().await,
			SubCommand::View { environment } => {
				let ctx = crate::util::login::load_or_login().await?;
				let env = crate::util::env::get_or_select(&ctx, environment.as_ref()).await?;

				let url = format!(
					"{hub}/projects/{proj}/environments/{env}",
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
