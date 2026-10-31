use anyhow::Result;
use clap::{Parser, Subcommand};

mod auth;
mod cloud;
mod commands;
mod credentials;
mod engine_runner;
mod target;
mod templates;
mod util;

pub(crate) const DEFAULT_CLOUD_API: &str = "https://cloud-api.rivet.dev";
pub(crate) const DEFAULT_ENGINE_ENDPOINT: &str = "http://127.0.0.1:6420";
pub(crate) const DEFAULT_NAMESPACE: &str = "production";
pub(crate) const LOCAL_NAMESPACE: &str = "default";
pub(crate) const POOL_NAME: &str = "default";
pub(crate) const SUPABASE_FN_DEFAULT: &str = "rivet";
/// Supabase runs the edge runtime in a container, so the handler reaches the
/// engine on the host through Docker's host alias instead of loopback.
/// `supabase functions serve` supplies the `host-gateway` mapping on Linux;
/// Docker Desktop provides the alias on macOS and Windows.
pub(crate) const SUPABASE_ENGINE_ENDPOINT: &str = "http://host.docker.internal:6420";

#[derive(Parser)]
#[command(name = "rivet", version, about = "Rivet CLI")]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	/// Sign in to Rivet Cloud in the browser.
	Login(commands::account::LoginOpts),
	/// Sign out of Rivet Cloud.
	Logout(commands::account::LogoutOpts),
	/// Show the signed-in account and linked project.
	Whoami(commands::account::WhoamiOpts),
	/// Link the current directory to a Rivet Cloud project.
	Link(commands::link::LinkOpts),
	/// Remove the current directory's project link.
	Unlink(commands::link::UnlinkOpts),
	/// Make an authenticated request to the Rivet Cloud API.
	Api(commands::api::Opts),
	/// Run a local Rivet engine and the dev server for your handler.
	Dev(commands::dev::Opts),
	/// Run the bundled rivet-engine binary directly (proxies all arguments).
	Engine(commands::engine::Opts),
	/// Manage Rivet Cloud clusters.
	Cluster(commands::cluster::Opts),
	/// Build and deploy the current project to Rivet Cloud.
	Deploy(commands::deploy::Opts),
	/// View logs for your Rivet Compute instances.
	Logs(commands::logs::Opts),
	/// Manage compute pools.
	Pool(commands::pool::Opts),
	/// Install the GitHub Actions workflow that deploys to Rivet Cloud.
	SetupCi(commands::setup_ci::Opts),
	/// Mint a namespace-scoped token.
	Token(commands::token::Opts),
}

#[tokio::main]
async fn main() -> Result<()> {
	init_tracing();

	let cli = Cli::parse();
	match cli.command {
		Commands::Login(opts) => opts.execute().await,
		Commands::Logout(opts) => opts.execute().await,
		Commands::Whoami(opts) => opts.execute().await,
		Commands::Link(opts) => opts.execute().await,
		Commands::Unlink(opts) => opts.execute().await,
		Commands::Api(opts) => opts.execute().await,
		Commands::Dev(opts) => opts.execute().await,
		Commands::Engine(opts) => opts.execute().await,
		Commands::Cluster(opts) => opts.execute().await,
		Commands::Deploy(opts) => opts.execute().await,
		Commands::Logs(opts) => opts.execute().await,
		Commands::Pool(opts) => opts.execute().await,
		Commands::SetupCi(opts) => opts.execute().await,
		Commands::Token(opts) => opts.execute().await,
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
