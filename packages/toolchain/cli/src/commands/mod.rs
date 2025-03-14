pub mod actor;
pub mod build;
pub mod config;
pub mod deno;
pub mod deploy;
pub mod environment;
pub mod init;
pub mod login;
pub mod logout;
pub mod manager;
pub mod metadata;
pub mod project;
pub mod region;
pub mod shell;

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
	#[clap(alias = "p")]
	Publish(build::publish::Opts),
	#[clap(alias = "e", alias = "env")]
	Environment {
		#[clap(subcommand)]
		subcommand: environment::SubCommand,
	},
	#[clap(alias = "proj")]
	Project {
		#[clap(subcommand)]
		subcommand: project::SubCommand,
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
	Config {
		#[clap(subcommand)]
		subcommand: config::SubCommand,
	},
	#[clap(alias = "meta")]
	Metadata {
		#[clap(subcommand)]
		subcommand: metadata::SubCommand,
	},
	Deno(deno::Opts),
	#[clap(hide = true)]
	Shell(shell::Opts),
	#[clap(alias = "v")]
	// Alias of `environment view`
	View {
		#[clap(long, alias = "env", short = 'e')]
		environment: Option<String>,
	},
}

impl SubCommand {
	pub async fn execute(&self) -> Result<()> {
		match self {
			SubCommand::Init(opts) => opts.execute().await,
			SubCommand::Login(opts) => opts.execute().await,
			SubCommand::Logout(opts) => opts.execute().await,
			SubCommand::Deploy(opts) => opts.execute().await,
			SubCommand::Publish(opts) => opts.execute().await,
			SubCommand::Environment { subcommand } => subcommand.execute().await,
			SubCommand::Project { subcommand } => subcommand.execute().await,
			SubCommand::Actor { subcommand } => subcommand.execute().await,
			SubCommand::Build { subcommand } => subcommand.execute().await,
			SubCommand::Region { subcommand } => subcommand.execute().await,
			SubCommand::Manager { subcommand } => subcommand.execute().await,
			SubCommand::Config { subcommand } => subcommand.execute().await,
			SubCommand::Metadata { subcommand } => subcommand.execute().await,
			SubCommand::Deno(opts) => opts.execute().await,
			SubCommand::Shell(opts) => opts.execute().await,
			SubCommand::View { environment } => {
				environment::SubCommand::View {
					environment: environment.clone(),
				}
				.execute()
				.await
			}
		}
	}
}
