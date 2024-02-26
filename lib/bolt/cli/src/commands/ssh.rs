use anyhow::*;
use clap::{Parser, ValueEnum};

use bolt_core::{context::ProjectContext, tasks::ssh::TempSshKey};

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
	Name {
		#[clap(index = 1)]
		name: String,
		#[clap(index = 2)]
		command: Option<String>,
	},
	Pool {
		#[clap(index = 1)]
		pool: String,
		#[clap(long, short = 'r')]
		region: Option<String>,
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
					command.as_ref().map(String::as_str),
				)
				.await?;
			}
			Self::Name { name, command } => {
				bolt_core::tasks::ssh::name(&ctx, &name, command.as_ref().map(String::as_str))
					.await?;
			}
			Self::Pool {
				pool,
				region,
				command,
				all,
			} => {
				if all {
					let command = command.context("must provide command with --all")?;
					bolt_core::tasks::ssh::pool_all(
						&ctx,
						&pool,
						region.as_ref().map(String::as_str),
						&command,
					)
					.await?;
				} else {
					bolt_core::tasks::ssh::pool(
						&ctx,
						&pool,
						region.as_ref().map(String::as_str),
						command.as_ref().map(String::as_str),
					)
					.await?;
				}
			}
		}

		Ok(())
	}
}
