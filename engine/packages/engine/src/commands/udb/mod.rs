use std::{
	path::Path,
	result::Result::{Err, Ok},
};

use anyhow::*;
use clap::Parser;
use cli::CommandResult;
use rivet_pools::UdbPool;
use rustyline::{DefaultEditor, error::ReadlineError};

use crate::util::udb::SimpleTuple;

mod cli;
pub mod key_parser;

#[derive(Parser)]
pub struct Opts {
	/// Immediately execute the given query without interactivity.
	#[arg(short = 'q', long)]
	query: Option<String>,
}

impl Opts {
	pub async fn execute(&self, config: rivet_config::Config) -> Result<()> {
		// Start server
		let pools = rivet_pools::Pools::new(config.clone()).await?;
		let pool = pools.udb()?;

		if let Some(query) = &self.query {
			let mut previous_tuple = SimpleTuple::new();
			let mut current_tuple = SimpleTuple::new();

			run_commands(&pool, &mut previous_tuple, &mut current_tuple, query).await;
		} else {
			let mut rl = DefaultEditor::new()?;
			let history_location = Path::new("/tmp/rivet-udb-cli-history");
			if history_location.exists() {
				rl.load_history(&history_location)?;
			}

			println!("UDB Editor\n");

			let mut previous_tuple = SimpleTuple::new();
			let mut current_tuple = SimpleTuple::new();

			loop {
				match rl.readline(&format!("{current_tuple}> ")) {
					Ok(line) => {
						rl.add_history_entry(line.as_str())?;

						if let CommandResult::Exit =
							run_commands(&pool, &mut previous_tuple, &mut current_tuple, &line)
								.await
						{
							break;
						}
					}
					// Ctrl + C
					Err(ReadlineError::Interrupted) => {}
					Err(ReadlineError::Eof) => break,
					Err(err) => return Err(err.into()),
				}
			}

			rl.save_history(&history_location)?;
		}

		Ok(())
	}
}

async fn run_commands(
	pool: &UdbPool,
	previous_tuple: &mut SimpleTuple,
	current_tuple: &mut SimpleTuple,
	query: &str,
) -> CommandResult {
	let command = query.trim();
	if command.is_empty() {
		return CommandResult::Ok;
	}

	match cli::SubCommand::try_parse_from(std::iter::once("").chain(command.split_whitespace())) {
		Ok(cmd) => cmd.execute(pool, previous_tuple, current_tuple).await,
		Err(err) => {
			println!("{err}");
			CommandResult::Error
		}
	}
}
