use anyhow::*;
use clap::{Parser, ValueEnum};

#[derive(Parser)]
pub enum SubCommand {
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
	#[clap(alias = "ch")]
	Clickhouse,
	#[clap(alias = "wfd")]
	WorkflowData,
	#[clap(alias = "wfi")]
	WorkflowInternal,
}

impl SubCommand {
	pub async fn execute(self, config: rivet_config::Config) -> Result<()> {
		match self {
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
					DatabaseType::Clickhouse => {
						crate::util::db::clickhouse_shell(config, shell_ctx).await?
					}
					DatabaseType::WorkflowData => {
						crate::util::db::wf_sqlite_shell(config, shell_ctx, false).await?
					}
					DatabaseType::WorkflowInternal => {
						crate::util::db::wf_sqlite_shell(config, shell_ctx, true).await?
					}
				}

				Ok(())
			}
		}
	}
}
