use std::{
	fs,
	io::{BufRead, BufReader},
};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};

use crate::util::udb::SimpleTuple;

#[derive(Parser)]
pub struct Opts {
	#[command(subcommand)]
	command: SubCommand,
}

#[derive(Subcommand)]
pub enum SubCommand {
	/// Decode a key from a byte array
	Decode {
		/// JSON array of bytes to decode (e.g. "[20, 21, 1, 21, 2]")
		#[arg(long)]
		array: String,
	},
	/// Parse and decode transaction conflicts from a logfmt log file
	ParseConflictLogs {
		/// Path to the logfmt log file
		#[arg(long)]
		file: String,
	},
}

impl Opts {
	pub fn execute(&self) -> Result<()> {
		match &self.command {
			SubCommand::Decode { array } => {
				decode_array(array)?;
				Ok(())
			}
			SubCommand::ParseConflictLogs { file } => {
				parse_conflicts(file)?;
				Ok(())
			}
		}
	}
}

fn decode_array(array: &str) -> Result<()> {
	// Parse the JSON array
	let bytes: Vec<u8> = serde_json::from_str(array)
		.with_context(|| format!("Failed to parse array as JSON: {}", array))?;

	// Decode the tuple using foundationdb tuple unpacking
	match universaldb::tuple::unpack::<SimpleTuple>(&bytes) {
		Ok(tuple) => {
			println!("{}", tuple);
		}
		Err(err) => {
			bail!("Failed to decode key: {:#}", err);
		}
	}

	Ok(())
}

fn parse_conflicts(file_path: &str) -> Result<()> {
	let file =
		fs::File::open(file_path).with_context(|| format!("Failed to open file: {}", file_path))?;
	let reader = BufReader::new(file);

	let mut conflict_count = 0;

	for line in reader.lines() {
		let line = line?;

		// Check if this is a transaction conflict log
		if !line.contains("transaction conflict detected") {
			continue;
		}

		conflict_count += 1;

		// Parse logfmt fields
		let mut fields = std::collections::HashMap::new();
		let mut in_quotes = false;
		let mut current_key = String::new();
		let mut current_value = String::new();
		let mut in_key = true;

		for c in line.chars() {
			match c {
				'"' => in_quotes = !in_quotes,
				'=' if !in_quotes && in_key => {
					in_key = false;
				}
				' ' if !in_quotes => {
					if !current_key.is_empty() {
						fields.insert(current_key.clone(), current_value.clone());
						current_key.clear();
						current_value.clear();
						in_key = true;
					}
				}
				_ => {
					if in_key {
						current_key.push(c);
					} else {
						current_value.push(c);
					}
				}
			}
		}

		// Don't forget the last field
		if !current_key.is_empty() {
			fields.insert(current_key, current_value);
		}

		// Extract and decode keys
		println!("\n═══════════════════════════════════════════════════════════");
		println!("Conflict #{}", conflict_count);
		println!("═══════════════════════════════════════════════════════════");

		if let Some(ts) = fields.get("ts") {
			println!("Timestamp: {}", ts);
		}

		if let (Some(cr1_type), Some(cr2_type)) = (fields.get("cr1_type"), fields.get("cr2_type")) {
			println!("CR1 Type: {}, CR2 Type: {}", cr1_type, cr2_type);
		}

		if let (Some(start_v), Some(commit_v)) = (
			fields.get("txn1_start_version"),
			fields.get("txn1_commit_version"),
		) {
			println!("TXN1: start={}, commit={}", start_v, commit_v);
		}

		if let (Some(start_v), Some(commit_v)) = (
			fields.get("txn2_start_version"),
			fields.get("txn2_commit_version"),
		) {
			println!("TXN2: start={}, commit={}", start_v, commit_v);
		}

		println!("\nCR1 Range:");
		if let Some(cr1_start) = fields.get("cr1_start") {
			print!("  Start: ");
			if let Err(e) = decode_from_logfmt(cr1_start) {
				println!("Error: {:#}", e);
			}
		}
		if let Some(cr1_end) = fields.get("cr1_end") {
			print!("  End:   ");
			if let Err(e) = decode_from_logfmt(cr1_end) {
				println!("Error: {:#}", e);
			}
		}

		println!("\nCR2 Range:");
		if let Some(cr2_start) = fields.get("cr2_start") {
			print!("  Start: ");
			if let Err(e) = decode_from_logfmt(cr2_start) {
				println!("Error: {:#}", e);
			}
		}
		if let Some(cr2_end) = fields.get("cr2_end") {
			print!("  End:   ");
			if let Err(e) = decode_from_logfmt(cr2_end) {
				println!("Error: {:#}", e);
			}
		}
	}

	if conflict_count == 0 {
		println!("No transaction conflicts found in the log file.");
	} else {
		println!("\n═══════════════════════════════════════════════════════════");
		println!("Total conflicts found: {}", conflict_count);
	}

	Ok(())
}

fn decode_from_logfmt(value: &str) -> Result<()> {
	// Remove surrounding quotes if present
	let value = value.trim_matches('"');

	// Parse the JSON array
	let bytes: Vec<u8> = serde_json::from_str(value)
		.with_context(|| format!("Failed to parse array as JSON: {}", value))?;

	// Decode the tuple
	let tuple = universaldb::tuple::unpack::<SimpleTuple>(&bytes)
		.with_context(|| "Failed to decode key")?;

	println!("{}", tuple);

	Ok(())
}
