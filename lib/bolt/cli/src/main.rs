use anyhow::*;
use clap::Parser;
use commands::*;

mod commands;

#[derive(Parser)]
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
	/// Checks if a Rivet service is valid without deploying it.
	Check(check::CheckOpts),
	/// Tails the logs of a Rivet service.
	Logs(logs::LogsOpts),
	/// Deploys and tests Rivet services.
	Test(test::TestOpts),
	/// Creates a new service.
	Create(create::CreateOpts),
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
	/// Manages SaltStack configs.
	Salt {
		#[clap(subcommand)]
		command: salt::SubCommand,
	},
	/// Provides SSH access to provisioned servers.
	Ssh {
		#[clap(subcommand)]
		command: ssh::SubCommand,
	},
	/// Manages databases for services.
	#[clap(alias = "db")]
	Database {
		#[clap(subcommand)]
		command: db::SubCommand,
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

	// Prompt confirmation if delpoying to prod
	if ctx.ns_id() == "prod" {
		tokio::task::block_in_place(|| prompt_prod())?;
	}

	match args.command {
		SubCommand::Init(_) | SubCommand::Config { .. } => {
			unreachable!("should be evaluated before creating project context")
		}
		SubCommand::Infra { command } => command.execute(ctx).await?,
		SubCommand::Up(command) => command.execute(ctx).await?,
		SubCommand::Check(command) => command.execute(ctx).await?,
		SubCommand::Logs(command) => command.execute(ctx).await?,
		SubCommand::Test(command) => command.execute(ctx).await?,
		SubCommand::Create(command) => command.execute(ctx).await?,
		SubCommand::Generate { command } => command.execute(ctx).await?,
		SubCommand::Secret { command } => command.execute(ctx).await?,
		SubCommand::Output { command } => command.execute(ctx).await?,
		SubCommand::Terraform { command } => command.execute(ctx).await?,
		SubCommand::Salt { command } => command.execute(ctx).await?,
		SubCommand::Ssh { command } => command.execute(ctx).await?,
		SubCommand::Database { command } => command.execute(ctx).await?,
		SubCommand::Admin { command } => command.execute(ctx).await?,
	}

	Ok(std::process::ExitCode::SUCCESS)
}

fn prompt_prod() -> Result<()> {
	use std::io::Write;

	let mut input = String::new();

	print!("Are you sure you want to run this command in prod? (yes) ");
	std::io::stdout().flush()?;

	std::io::stdin().read_line(&mut input)?;

	if input.trim().eq_ignore_ascii_case("yes") {
		return Ok(());
	} else {
		bail!("Bailing");
	}
}
