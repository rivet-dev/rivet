use anyhow::*;
use clap::Subcommand;

mod get;
mod list;
mod patch_tags;

#[derive(Subcommand)]
pub enum SubCommand {
	Get(get::Opts),
	List(list::Opts),
	PatchTags(patch_tags::Opts),
}

impl SubCommand {
	pub async fn execute(&self) -> Result<()> {
		match &self {
			SubCommand::Get(opts) => opts.execute().await,
			SubCommand::List(opts) => opts.execute().await,
			SubCommand::PatchTags(opts) => opts.execute().await,
		}
	}
}
