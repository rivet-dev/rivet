pub mod actor;
pub mod build;
pub mod config;
pub mod deno;
pub mod deploy;
pub mod environment;
pub mod login;
pub mod logout;
pub mod metadata;
pub mod project;
pub mod region;
pub mod shell;

use anyhow::*;
use clap::Parser;

/// Main Rivet CLI commands
#[derive(Parser)]
pub enum SubCommand {
	/// Login to a project
	#[clap(alias = "signin")]
	Login(login::Opts),
	/// Logout from a project
	#[clap(alias = "signout")]
	Logout(logout::Opts),
	/// Deploy a build to a specific environment
	#[clap(alias = "d")]
	Deploy(deploy::Opts),
	/// Publish a new build from local files or a Docker image
	#[clap(alias = "p")]
	Publish(build::publish::Opts),
	/// Commands for managing environments
	#[clap(alias = "e", alias = "env")]
	Environment {
		#[clap(subcommand)]
		subcommand: environment::SubCommand,
	},
	/// Commands for managing projects
	#[clap(alias = "proj")]
	Project {
		#[clap(subcommand)]
		subcommand: project::SubCommand,
	},
	/// Commands for managing actors
	#[clap(alias = "a")]
	Actor {
		#[clap(subcommand)]
		subcommand: actor::SubCommand,
	},
	/// Commands for managing builds
	#[clap(alias = "b")]
	Build {
		#[clap(subcommand)]
		subcommand: build::SubCommand,
	},
	/// Commands for managing regions
	Region {
		#[clap(subcommand)]
		subcommand: region::SubCommand,
	},
	/// Commands for managing Rivet configuration
	Config {
		#[clap(subcommand)]
		subcommand: config::SubCommand,
	},
	/// Commands for retrieving metadata about Rivet configuration
	#[clap(alias = "meta")]
	Metadata {
		#[clap(subcommand)]
		subcommand: metadata::SubCommand,
	},
	/// Launch an interactive shell with Rivet environment variables
	Shell(shell::Opts),
	/// Execute Deno commands with Rivet environment variables
	#[clap(hide = true)]
	Deno(deno::Opts),
	/// Open the environment dashboard in a browser (alias of `environment view`)
	#[clap(alias = "v")]
	View {
		/// Specify the environment to view (will prompt if not specified)
		#[clap(long, alias = "env", short = 'e')]
		environment: Option<String>,
	},
}

impl SubCommand {
	pub async fn execute(&self) -> Result<()> {
		match self {
			SubCommand::Login(opts) => opts.execute().await,
			SubCommand::Logout(opts) => opts.execute().await,
			SubCommand::Deploy(opts) => opts.execute().await,
			SubCommand::Publish(opts) => opts.execute().await,
			SubCommand::Environment { subcommand } => subcommand.execute().await,
			SubCommand::Project { subcommand } => subcommand.execute().await,
			SubCommand::Actor { subcommand } => subcommand.execute().await,
			SubCommand::Build { subcommand } => subcommand.execute().await,
			SubCommand::Region { subcommand } => subcommand.execute().await,
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
