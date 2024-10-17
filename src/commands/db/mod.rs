use anyhow::*;
use clap::{Parser, ValueEnum};

mod migrate;

#[derive(Parser)]
pub enum SubCommand {
	Migrate {
		#[clap(subcommand)]
		command: migrate::SubCommand,
	},
	#[clap(alias = "sh")]
	Shell {
		#[clap(index = 1)]
		database_type: DatabaseType,
		#[clap(index = 2)]
		service: String,
		#[clap(short = 'q', long)]
		query: Option<String>,
	},
}

#[derive(ValueEnum, Clone, PartialEq)]
pub enum DatabaseType {
	#[clap(alias = "cockroach", alias = "crdb")]
	Cockroachdb,
	Redis,
	#[clap(alias = "ch")]
	Clickhouse,
}

impl SubCommand {
	pub async fn execute(self) -> Result<()> {
		match self {
			Self::Migrate { command } => command.execute().await,
			Self::Shell {
				database_type: db_type,
				service,
				query,
			} => {
				let shell_query = crate::util::db::ShellQuery {
					svc: service,
					query,
				};
				let shell_ctx = crate::util::db::ShellContext {
					queries: &[shell_query],
				};

				match db_type {
					DatabaseType::Cockroachdb => {
						crate::util::db::cockroachdb_shell(shell_ctx).await?
					}
					DatabaseType::Redis => crate::util::db::redis_shell(shell_ctx).await?,
					DatabaseType::Clickhouse => {
						crate::util::db::clickhouse_shell(shell_ctx).await?
					}
				}

				Ok(())
			}
		}
	}
}
