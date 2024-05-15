use anyhow::*;
use bolt_core::{context::ProjectContext, tasks::ssh::TempSshKey};
use clap::{Parser, ValueEnum};

#[derive(ValueEnum, Clone)]
pub enum Format {
	Json,
}

#[derive(Parser)]
pub enum SubCommand {
	Ip {
		#[clap(index = 1)]
		ip: String,
		#[clap(index = 2)]
		command: Option<String>,
		#[clap(long)]
		ssh_key: Option<String>,
	},
	Id {
		#[clap(index = 1)]
		server_id: String,
		#[clap(index = 2)]
		command: Option<String>,
	},
	Pool {
		#[clap(index = 1)]
		pool: String,
		#[clap(index = 2)]
		command: Option<String>,
		#[clap(short = 'a', long)]
		all: bool,
	},
}

impl SubCommand {
	pub async fn execute(self, ctx: ProjectContext) -> Result<()> {
		match self {
			Self::Ip {
				ip,
				command,
				ssh_key,
			} => {
				let ssh_key =
					TempSshKey::new(&ctx, &ssh_key.map_or_else(|| "server".to_string(), |x| x))
						.await?;
				bolt_core::tasks::ssh::ip(
					&ctx,
					&ip,
					&ssh_key,
					command.as_deref(),
				)
				.await?;
			}
			Self::Id { server_id, command } => {
				bolt_core::tasks::ssh::id(&ctx, &server_id, command.as_deref())
					.await?;
			}
			Self::Pool { pool, command, all } => {
				if all {
					let command = command.context("must provide command with --all")?;
					bolt_core::tasks::ssh::pool_all(&ctx, &pool, &command).await?;
				} else {
					bolt_core::tasks::ssh::pool(&ctx, &pool, command.as_deref())
						.await?;
				}
			}
		}

		Ok(())
	}
}
