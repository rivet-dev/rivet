pub mod actor;
pub mod build;
pub mod deno;
pub mod deploy;
pub mod environment;
pub mod init;
pub mod login;
pub mod logout;
pub mod manager;
pub mod metadata;
pub mod region;

use anyhow::*;
use clap::Parser;

#[derive(Parser)]
pub enum SubCommand {
	Init(init::Opts),
	#[clap(alias = "signin")]
	Login(login::Opts),
	#[clap(alias = "signout")]
	Logout(logout::Opts),
	#[clap(alias = "d")]
	Deploy(deploy::Opts),
	#[clap(alias = "e", alias = "env")]
	Environment {
		#[clap(subcommand)]
		subcommand: environment::SubCommand,
	},
	#[clap(alias = "a")]
	Actor {
		#[clap(subcommand)]
		subcommand: actor::SubCommand,
	},
	#[clap(alias = "b")]
	Build {
		#[clap(subcommand)]
		subcommand: build::SubCommand,
	},
	Region {
		#[clap(subcommand)]
		subcommand: region::SubCommand,
	},
	Manager {
		#[clap(subcommand)]
		subcommand: manager::SubCommand,
	},
	#[clap(alias = "meta")]
	Metadata {
		#[clap(subcommand)]
		subcommand: metadata::SubCommand,
	},
	Deno(deno::Opts),
}

impl SubCommand {
	pub async fn execute(&self) -> Result<()> {
		match self {
			SubCommand::Init(opts) => opts.execute().await,
			SubCommand::Login(opts) => opts.execute().await,
			SubCommand::Logout(opts) => opts.execute().await,
			SubCommand::Deploy(opts) => opts.execute().await,
			SubCommand::Environment { subcommand } => subcommand.execute().await,
			SubCommand::Actor { subcommand } => subcommand.execute().await,
			SubCommand::Build { subcommand } => subcommand.execute().await,
			SubCommand::Region { subcommand } => subcommand.execute().await,
			SubCommand::Manager { subcommand } => subcommand.execute().await,
			SubCommand::Metadata { subcommand } => subcommand.execute().await,
			SubCommand::Deno(opts) => opts.execute().await,
		}
	}
}
