use anyhow::*;
use bolt_core::context::ProjectContext;
use clap::{Parser, ValueEnum};
use serde_json::json;

#[derive(ValueEnum, Clone)]
pub enum Format {
	Json,
}

#[derive(Parser)]
pub enum SubCommand {
	Get {
		#[clap(index = 1)]
		path: String,
		#[clap(long)]
		optional: bool,
		#[clap(long, value_parser)]
		format: Option<Format>,
	},
}

impl SubCommand {
	pub async fn execute(self, ctx: ProjectContext) -> Result<()> {
		match self {
			Self::Get {
				path,
				optional,
				format,
			} => {
				let path = path.split("/").collect::<Vec<_>>();

				// Fetch value
				let value = if optional {
					ctx.read_secret_opt(&path).await?
				} else {
					Some(ctx.read_secret(&path).await?)
				};

				// Log value
				match format {
					None => {
						if let Some(value) = value {
							println!("{value}")
						} else {
							println!("")
						}
					}
					Some(Format::Json) => println!("{}", json!({ "value": value })),
				}
			}
		}

		Ok(())
	}
}
