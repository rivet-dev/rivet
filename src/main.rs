use anyhow::*;
use clap::Parser;
use commands::*;

mod commands;

// Check that some services are enabled
#[cfg(not(any(
	feature = "api",
	feature = "api-internal",
	feature = "standalone",
	feature = "oneshot",
	feature = "cron"
)))]
compile_error!(
	"At least one feature must be enabled: api, api-internal, standalone, oneshot, or cron"
);

#[derive(Parser)]
#[command(name = "Rivet", version, about)]
struct Cli {
	#[command(subcommand)]
	command: SubCommand,
}

#[derive(Parser)]
enum SubCommand {
	/// Starts the Rivet server
	Server(server::Opts),
	/// Provisions all of the required resources to run Rivet.
	///
	/// If you need to provision specific parts, use the `rivet db migrate up` and `rivet storage
	/// provision` commands.
	Provision(provision::Opts),
	/// Manages databases
	#[clap(alias = "db")]
	Database {
		#[clap(subcommand)]
		command: db::SubCommand,
	},
	/// Manages buckets
	Storage {
		#[clap(subcommand)]
		command: storage::SubCommand,
	},
}

fn main() -> Result<()> {
	rivet_runtime::run(async { main_inner().await })??;
	Ok(())
}

async fn main_inner() -> Result<()> {
	// Run command
	let cli = Cli::parse();
	match cli.command {
		SubCommand::Server(opts) => opts.execute().await,
		SubCommand::Provision(opts) => opts.execute().await,
		SubCommand::Database { command } => command.execute().await,
		SubCommand::Storage { command } => command.execute().await,
	}
}
