use anyhow::*;
use clap::Subcommand;

mod select;

#[derive(Subcommand)]
pub enum SubCommand {
	#[clap(alias = "s")]
	Select(select::Opts),
	#[clap(alias = "v")]
	View {
		#[clap(long, alias = "env", short = 'e')]
		environment: Option<String>,
	},
}

impl SubCommand {
	pub async fn execute(&self) -> Result<()> {
		match &self {
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
