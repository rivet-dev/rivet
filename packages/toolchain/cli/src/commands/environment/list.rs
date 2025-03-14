use anyhow::Result;
use clap::Parser;

/// List all available environments
#[derive(Parser)]
pub struct Opts {
	#[clap(long, short)]
	/// Output in JSON format
	json: bool,
}

impl Opts {
	pub async fn execute(&self) -> Result<()> {
		let ctx = crate::util::login::load_or_login().await?;
		match self.json {
			true => {
				let json = serde_json::to_string(&ctx.project.namespaces)?;
				println!("{}", json);
				Ok(())
			}
			false => {
				for ns in &ctx.project.namespaces {
					println!("{} ({})", ns.display_name, ns.name_id);
				}
				Ok(())
			}
		}
	}
}
