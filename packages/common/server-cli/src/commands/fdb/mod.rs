use std::{
	path::Path,
	result::Result::{Err, Ok},
};

use anyhow::*;
use clap::Parser;
use cli::CommandResult;
use rivet_pools::FdbPool;
use rustyline::{error::ReadlineError, DefaultEditor};

use crate::util::fdb::SimpleTuple;

mod cli;

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
		let pool = pools.fdb()?;

		if let Some(query) = &self.query {
			let mut current_tuple = SimpleTuple::new();

			run_commands(&pool, &mut current_tuple, query).await;
		} else {
			let mut rl = DefaultEditor::new()?;
			let history_location = Path::new("/tmp/rivet-server-fdb-history");
			if history_location.exists() {
				rl.load_history(&history_location)?;
			}

			println!("FDB Viewer\n");

			let mut current_tuple = SimpleTuple::new();

			loop {
				match rl.readline(&format!("{current_tuple}> ")) {
					Ok(line) => {
						rl.add_history_entry(line.as_str())?;

						if let CommandResult::Exit =
							run_commands(&pool, &mut current_tuple, &line).await
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
	pool: &FdbPool,
	current_tuple: &mut SimpleTuple,
	query: &str,
) -> CommandResult {
	let mut escaped = false;
	let mut start = 0;

	for (i, c) in query.chars().enumerate() {
		match c {
			'&' if !escaped && query.chars().nth(i + 1) == Some('&') => {
				let command = query[start..i].trim();

				// Parse the command string
				match cli::SubCommand::try_parse_from(
					std::iter::once("").chain(command.split_whitespace()),
				) {
					Ok(cmd) => match cmd.execute(pool, current_tuple).await {
						CommandResult::Ok => {}
						x => return x,
					},
					Err(err) => {
						if command.trim().is_empty() {
							println!("expected command");
						} else {
							println!("{err}");
						}

						return CommandResult::Error;
					}
				}

				start = i + 2;
			}
			'\\' => escaped = !escaped,
			_ => escaped = false,
		}
	}

	// Run final command
	let command = query[start..].trim();
	if !command.is_empty() {
		// Parse the command string
		match cli::SubCommand::try_parse_from(std::iter::once("").chain(command.split_whitespace()))
		{
			Ok(cmd) => return cmd.execute(pool, current_tuple).await,
			Err(err) => {
				if command.trim().is_empty() {
					println!("expected command");
				} else {
					println!("{err}");
				}

				return CommandResult::Error;
			}
		}
	}

	CommandResult::Ok
}
