use std::{collections::HashMap, time::Duration};

use anyhow::{Result, bail};
use chrono::{SecondsFormat, Utc};
use clap::{Parser, builder::PossibleValuesParser};

use crate::{
	cloud::{
		CloudClient,
		v2::{self, LogLine, LogQuery},
	},
	target::{CloudArgs, ClusterTarget, Target},
	util::color_enabled,
};

const POLL_INTERVAL: Duration = Duration::from_secs(2);
/// How late a line may reach the logs store after it was written. Collectors
/// batch and retry, so lines newer than this are read again on every poll.
const LATE_ARRIVAL: chrono::Duration = chrono::Duration::seconds(60);
const MAX_PAGE: u32 = 1000;

#[derive(Parser)]
pub struct Opts {
	#[command(flatten)]
	cloud: CloudArgs,
	/// Only the engine's logs or only the runners'.
	#[arg(long, value_parser = PossibleValuesParser::new(["engine", "runner"]))]
	source: Option<String>,
	/// Only this runner pool's logs.
	#[arg(long)]
	pool: Option<String>,
	/// Only lines at this severity or above.
	#[arg(long, value_parser = PossibleValuesParser::new(["trace", "debug", "info", "warn", "error", "fatal"]))]
	severity: Option<String>,
	/// Only lines containing this text (case-insensitive).
	#[arg(long)]
	contains: Option<String>,
	/// Follow the log output (live tail).
	#[arg(long, short = 'f')]
	follow: bool,
	/// Number of most recent lines to show first.
	#[arg(long, short = 'n', default_value_t = 100)]
	limit: u32,
	/// Only show lines before this line id, from `--json` output.
	#[arg(long)]
	before: Option<String>,
	/// Emit raw JSON lines instead of formatted output.
	#[arg(long)]
	json: bool,
}

impl Opts {
	pub async fn execute(self) -> Result<()> {
		let (cloud, _, target) = self.cloud.connect().await?;
		let cluster = match target {
			Target::Cluster(ClusterTarget { cluster, .. }) => cluster,
			Target::Project(_) => {
				bail!("no cluster selected; pass --cluster or run `rivet link --cluster <name>`")
			}
		};
		self.run(&cloud, &cluster).await
	}

	fn query(&self) -> LogQuery<'_> {
		LogQuery {
			source: self.source.as_deref(),
			runner_pool: self.pool.as_deref(),
			contains: self.contains.as_deref(),
			min_severity: self.severity.as_deref(),
			..LogQuery::default()
		}
	}

	async fn run(&self, cloud: &CloudClient, cluster: &str) -> Result<()> {
		let color = color_enabled();
		let history = v2::list_logs(
			cloud,
			cluster,
			&LogQuery {
				before: self.before.as_deref(),
				limit: Some(self.limit),
				..self.query()
			},
		)
		.await?;

		let mut follower = Follower::default();
		for line in follower.accept(history.items) {
			print_line(&line, self.json, color)?;
		}
		if !self.follow {
			return Ok(());
		}

		loop {
			tokio::time::sleep(POLL_INTERVAL).await;
			follower.settle(&timestamp(Utc::now() - LATE_ARRIVAL));
			// Without a cursor yet, the newest page is read instead.
			let mut after = follower.cursor().map(str::to_owned);
			loop {
				let page = v2::list_logs(
					cloud,
					cluster,
					&LogQuery {
						after: after.as_deref(),
						limit: Some(MAX_PAGE),
						..self.query()
					},
				)
				.await?;
				let next = page.items.last().map(|line| line.id.clone());
				for line in follower.accept(page.items) {
					print_line(&line, self.json, color)?;
				}
				match (after.is_some() && page.has_more, next) {
					(true, Some(next)) => after = Some(next),
					(true, None) | (false, _) => break,
				}
			}
		}
	}
}

fn timestamp(at: chrono::DateTime<Utc>) -> String {
	at.to_rfc3339_opts(SecondsFormat::Micros, true)
}

/// Tracks which lines a follower printed. Lines can reach the store late and
/// out of order, so it reads again from the newest line old enough to be
/// settled and prints only lines it has not printed.
#[derive(Default)]
pub struct Follower {
	/// Printed line ids newer than the cursor, with their timestamps.
	seen: HashMap<String, String>,
	/// The newest settled line: every line before it was printed.
	cursor: Option<(String, String)>,
}

impl Follower {
	pub fn cursor(&self) -> Option<&str> {
		self.cursor.as_ref().map(|(id, _)| id.as_str())
	}

	/// Returns the lines not printed yet, oldest first.
	pub fn accept(&mut self, mut lines: Vec<LogLine>) -> Vec<LogLine> {
		lines.retain(|line| !self.seen.contains_key(&line.id));
		lines.sort_by(|a, b| (&a.timestamp, &a.id).cmp(&(&b.timestamp, &b.id)));
		lines.dedup_by(|a, b| a.id == b.id);
		for line in &lines {
			self.seen.insert(line.id.clone(), line.timestamp.clone());
		}
		lines
	}

	/// Moves the cursor to the newest printed line written before `settled`,
	/// an ISO 8601 timestamp, and forgets the lines before it. Until a line
	/// settles, the cursor is the oldest printed line, so a follower never
	/// rereads history it was not asked for.
	pub fn settle(&mut self, settled: &str) {
		let newest = self
			.seen
			.iter()
			.filter(|(_, timestamp)| timestamp.as_str() < settled)
			.max_by(|a, b| (a.1, a.0).cmp(&(b.1, b.0)))
			.map(|(id, timestamp)| (id.clone(), timestamp.clone()));
		let Some(newest) = newest else {
			if self.cursor.is_none() {
				self.cursor = self
					.seen
					.iter()
					.min_by(|a, b| (a.1, a.0).cmp(&(b.1, b.0)))
					.map(|(id, timestamp)| (id.clone(), timestamp.clone()));
			}
			return;
		};
		self.seen.retain(|id, timestamp| {
			(timestamp.as_str(), id.as_str()) > (newest.1.as_str(), newest.0.as_str())
		});
		self.cursor = Some(newest);
	}
}

fn print_line(line: &LogLine, json: bool, color: bool) -> Result<()> {
	if json {
		println!("{}", serde_json::to_string(line)?);
		return Ok(());
	}

	let severity = line.severity.as_deref().unwrap_or("-");
	let severity = if color {
		format!("\x1b[{}m{severity:<5}\x1b[0m", severity_color(severity))
	} else {
		format!("{severity:<5}")
	};
	let source = match &line.runner_pool {
		Some(pool) => format!("{}/{pool}", line.source),
		None => line.source.clone(),
	};
	println!("{} {severity} {source} {}", line.timestamp, line.message);
	Ok(())
}

/// Maps a log level to an ANSI SGR color code.
fn severity_color(severity: &str) -> &'static str {
	match severity {
		"trace" | "debug" => "90",
		"info" => "32",
		"warn" => "33",
		"error" => "31",
		"fatal" => "1;31",
		_ => "37",
	}
}


