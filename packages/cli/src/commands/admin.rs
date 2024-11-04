use anyhow::*;
use clap::Parser;

#[derive(Parser)]
pub enum SubCommand {
	/// Generates a login link for the given access token. Automatically turns the existing user into an
	/// admin (or creates a new admin if no user).
	Login {
		#[clap(long, short = 'u', default_value = "admin")]
		username: String,
	},
}

impl SubCommand {
	pub async fn execute(self, config: rivet_config::Config) -> Result<()> {
		match self {
			Self::Login { username } => {
				rivet_term::status::progress("Logging in as", &username);

				let url = crate::util::admin::login_create(&config, username).await?;

				eprintln!();
				rivet_term::status::success("Login with this url", "");
				eprintln!("{url}");

				Ok(())
			}
		}
	}
}
