use anyhow::*;
use clap::Subcommand;

mod get;
mod list;
mod patch_tags;
pub mod publish;

/// Commands for managing builds
#[derive(Subcommand)]
pub enum SubCommand {
	Publish(publish::Opts),
	Get(get::Opts),
	List(list::Opts),
	PatchTags(patch_tags::Opts),
}

impl SubCommand {
	pub async fn execute(&self) -> Result<()> {
		match &self {
			SubCommand::Publish(opts) => opts.execute().await,
			SubCommand::Get(opts) => opts.execute().await,
			SubCommand::List(opts) => opts.execute().await,
			SubCommand::PatchTags(opts) => opts.execute().await,
		}
	}
}
