use anyhow::Result;
use clap::{Parser, Subcommand};

mod cloud;
mod commands;
mod credentials;
mod engine_runner;
mod templates;
mod util;

pub(crate) const DEFAULT_CLOUD_API: &str = "https://cloud-api.rivet.dev";
pub(crate) const DEFAULT_ENGINE_ENDPOINT: &str = "http://127.0.0.1:6420";
pub(crate) const DEFAULT_NAMESPACE: &str = "production";
pub(crate) const LOCAL_NAMESPACE: &str = "default";
pub(crate) const POOL_NAME: &str = "default";
pub(crate) const SUPABASE_FN_DEFAULT: &str = "rivet";

#[derive(Parser)]
#[command(name = "rivet", version, about = "Rivet CLI")]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	/// Run a local Rivet engine and the dev server for your handler.
	Dev(commands::dev::Opts),
	/// Run the bundled rivet-engine binary directly (proxies all arguments).
	Engine(commands::engine::Opts),
	/// Build and deploy the current project to Rivet Cloud.
	Deploy(commands::deploy::Opts),
	/// View logs for your Rivet Compute instances.
	Logs(commands::logs::Opts),
	/// Install the GitHub Actions workflow that deploys to Rivet Cloud.
	SetupCi(commands::setup_ci::Opts),
}

#[tokio::main]
async fn main() -> Result<()> {
	init_tracing();

	let cli = Cli::parse();
	match cli.command {
		Commands::Dev(opts) => opts.execute().await,
		Commands::Engine(opts) => opts.execute().await,
		Commands::Deploy(opts) => opts.execute().await,
		Commands::Logs(opts) => opts.execute().await,
		Commands::SetupCi(opts) => opts.execute().await,
	}
}

fn init_tracing() {
	use tracing_subscriber::{EnvFilter, fmt};

	let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
	fmt()
		.with_env_filter(filter)
		.with_writer(std::io::stderr)
		.without_time()
		.with_target(false)
		.init();
}
