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
	/// Manages databases
	#[clap(alias = "db")]
	Database {
		#[clap(subcommand)]
		command: db::SubCommand,
	},
}

#[tokio::main]
async fn main() -> Result<()> {
	let cli = Cli::parse();

	match cli.command {
		SubCommand::Server(opts) => opts.execute().await,
		SubCommand::Database { command } => command.execute().await,
	}
}
