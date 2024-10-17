use anyhow::*;
use clap::Parser;
use commands::*;

mod commands;

#[derive(Parser)]
// This will use the version from the Cargo.toml file during compilation
#[clap(version = env!("CARGO_PKG_VERSION"))]
struct Opts {
	#[clap(subcommand)]
	command: SubCommand,
}

#[derive(Parser)]
enum SubCommand {
	/// Generates configs & creates infrastructure for a namespace.
	///
	/// Can we ran multiple times.
	Init(init::InitOpts),
	/// Manages Rivet cluster configuration files
	Config {
		#[clap(subcommand)]
		command: config::SubCommand,
	},
	/// Manages infrastructure required to run the Rivet cluster
	Infra {
		#[clap(subcommand)]
		command: infra::SubCommand,
	},
	/// Builds and deploys all Rivet services.
	Up(up::UpOpts),
	/// Tails the logs of a Rivet service.
	Logs(logs::LogsOpts),
	/// Deploys and tests Rivet services.
	Test(test::TestOpts),
	/// Generates files required for the Rivet project. Seldom used.
	#[clap(hide(true), alias = "gen")]
	Generate {
		#[clap(subcommand)]
		command: generate::SubCommand,
	},
	/// Manages Rivet secrets.
	#[clap(alias = "secrets")]
	Secret {
		#[clap(subcommand)]
		command: secret::SubCommand,
	},
	/// Outputs information from the Rivet project.
	#[clap(hide(true))]
	Output {
		#[clap(subcommand)]
		command: output::SubCommand,
	},
	/// Manages Terraform plans.
	#[clap(alias = "tf")]
	Terraform {
		#[clap(subcommand)]
		command: terraform::SubCommand,
	},
	/// Cluster related operations
	Cluster {
		#[clap(subcommand)]
		command: cluster::SubCommand,
	},
	/// Provides admin functionality.
	Admin {
		#[clap(subcommand)]
		command: admin::SubCommand,
	},
}

#[tokio::main]
async fn main() -> Result<std::process::ExitCode> {
	let args = Opts::parse();

	// Match commands that need to be ran before ProjectContext is created
	match &args.command {
		SubCommand::Init(command) => {
			command.execute().await?;
			return Ok(std::process::ExitCode::SUCCESS);
		}
		SubCommand::Config { command } => {
			command.execute().await?;
			return Ok(std::process::ExitCode::SUCCESS);
		}
		_ => {}
	}

	let ctx =
		bolt_core::context::ProjectContextData::new(std::env::var("BOLT_NAMESPACE").ok()).await;

	// Prompt confirmation if deploying to prod
	if ctx.ns().bolt.confirm_commands {
		prompt_prod(&ctx).await?;
	}

	match args.command {
		SubCommand::Init(_) | SubCommand::Config { .. } => {
			unreachable!("should be evaluated before creating project context")
		}
		SubCommand::Infra { command } => command.execute(ctx).await?,
		SubCommand::Up(command) => command.execute(ctx).await?,
		SubCommand::Logs(command) => command.execute(ctx).await?,
		SubCommand::Test(command) => command.execute(ctx).await?,
		SubCommand::Generate { command } => command.execute(ctx).await?,
		SubCommand::Secret { command } => command.execute(ctx).await?,
		SubCommand::Output { command } => command.execute(ctx).await?,
		SubCommand::Terraform { command } => command.execute(ctx).await?,
		SubCommand::Cluster { command } => command.execute(ctx).await?,
		SubCommand::Admin { command } => command.execute(ctx).await?,
	}

	Ok(std::process::ExitCode::SUCCESS)
}

async fn prompt_prod(ctx: &bolt_core::context::ProjectContextData) -> Result<()> {
	if std::env::var("BOLT_HEADLESS").ok() == Some("1".to_string())
		|| !atty::is(atty::Stream::Stdout)
	{
		return Ok(());
	}

	let term = rivet_term::terminal();
	let response = rivet_term::prompt::PromptBuilder::default()
		.message(format!(
			"Are you sure you want to run this command in {}?",
			ctx.ns_id()
		))
		.build()?
		.bool(&term)
		.await?;

	if response {
		Ok(())
	} else {
		bail!("Bailing");
	}
}
