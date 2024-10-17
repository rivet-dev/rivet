use std::cmp::Ordering;

use anyhow::*;
use chrono::{Local, TimeZone};
use clap::ValueEnum;
use indoc::indoc;
use rivet_term::console::style;
use sqlx::PgPool;
use uuid::Uuid;

use crate::util::{
	self,
	format::{colored_json, indent_string},
};

pub mod signal;

#[derive(Debug, Clone)]
pub struct KvPair {
	pub key: String,
	pub value: String,
}

#[derive(thiserror::Error, Debug)]
pub enum ParseKvPairError {
	#[error("kv pairs must use `=` with no spaces (ex: foo=bar)")]
	NoEquals,
}

impl std::str::FromStr for KvPair {
	type Err = ParseKvPairError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let Some((key, value)) = s.split_once('=') else {
			return Result::Err(ParseKvPairError::NoEquals);
		};

		let key = key.trim().to_string();
		let value = value.trim().to_string();

		Result::Ok(KvPair { key, value })
	}
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[clap(rename_all = "kebab_case")]
pub enum WorkflowState {
	Complete,
	Running,
	Sleeping,
	Dead,
}

#[derive(strum::FromRepr)]
enum EventType {
	Activity = 0,
	Signal = 1,
	SubWorkflow = 2,
	SignalSend = 3,
	MessageSend = 4,
	Loop = 5,
}

impl EventType {
	fn name(&self) -> &str {
		match self {
			EventType::Activity => "activity",
			EventType::Signal => "signal receive",
			EventType::SubWorkflow => "sub workflow",
			EventType::SignalSend => "signal send",
			EventType::MessageSend => "message send",
			EventType::Loop => "loop",
		}
	}
}

#[derive(Debug, sqlx::FromRow)]
pub struct WorkflowRow {
	workflow_id: Uuid,
	workflow_name: String,
	tags: Option<serde_json::Value>,
	create_ts: i64,
	input: serde_json::Value,
	output: Option<serde_json::Value>,
	error: Option<String>,

	is_active: bool,
	has_wake_condition: bool,
}

#[derive(sqlx::FromRow)]
struct HistoryEvent {
	location: Vec<i64>,
	t: i64,
	name: Option<String>,
	input: Option<serde_json::Value>,
	output: Option<serde_json::Value>,
	forgotten: bool,
}

pub async fn get_workflow(workflow_id: Uuid) -> Result<Option<WorkflowRow>> {
	let pool = build_pool().await?;
	let mut conn = pool.acquire().await?;

	let workflow = sqlx::query_as::<_, WorkflowRow>(indoc!(
		"
		SELECT
			workflow_id,
			workflow_name,
			tags,
			create_ts,
			input,
			output,
			error,
			worker_instance_id IS NOT NULL AS is_active,
			(
				wake_immediate OR
				wake_deadline_ts IS NOT NULL OR
				cardinality(wake_signals) > 0 OR
				wake_sub_workflow_id IS NOT NULL
			) AS has_wake_condition
		FROM db_workflow.workflows
		WHERE
			workflow_id = $1
		"
	))
	.bind(workflow_id)
	.fetch_optional(&mut *conn)
	.await?;

	Ok(workflow)
}

pub async fn find_workflows(
	tags: Vec<KvPair>,
	name: Option<String>,
	state: Option<WorkflowState>,
) -> Result<Vec<WorkflowRow>> {
	let pool = build_pool().await?;
	let mut conn = pool.acquire().await?;

	let mut query_str = indoc!(
		"
		SELECT
			workflow_id,
			workflow_name,
			tags,
			create_ts,
			input,
			output,
			error,
			worker_instance_id IS NOT NULL AS is_active,
			(
				wake_immediate OR
				wake_deadline_ts IS NOT NULL OR
				cardinality(wake_signals) > 0 OR
				wake_sub_workflow_id IS NOT NULL
			) AS has_wake_condition
		FROM db_workflow.workflows
		WHERE
			($1 IS NULL OR workflow_name = $1) AND
			silence_ts IS NULL AND
			-- Complete
			(NOT $2 OR output IS NOT NULL) AND
			-- Running
			(NOT $3 OR (
				output IS NULL AND
				worker_instance_id IS NOT NULL
			)) AND
			-- Sleeping
			(NOT $4 OR (
				output IS NULL AND
				worker_instance_id IS NULL AND
				(
					wake_immediate OR
					wake_deadline_ts IS NOT NULL OR
					cardinality(wake_signals) > 0 OR
					wake_sub_workflow_id IS NOT NULL
				)
			)) AND
			-- Dead
			(NOT $5 OR (
				output IS NULL AND
				worker_instance_id IS NULL AND
				wake_immediate = FALSE AND
				wake_deadline_ts IS NULL AND
				cardinality(wake_signals) = 0 AND
				wake_sub_workflow_id IS NULL
			))
		"
	)
	.to_string();

	// Procedurally add tags. We don't combine the tags into an object because we are comparing
	// strings with `->>` whereas with @> and `serde_json::Map` we would have to know the type of the input
	// given.
	for i in 0..tags.len() {
		let idx = i * 2 + 6;
		let idx2 = idx + 1;

		query_str.push_str(&format!(" AND tags->>${idx} = ${idx2}"));
	}

	query_str.push_str("LIMIT 100");

	let mut query = sqlx::query_as::<_, WorkflowRow>(&query_str)
		.bind(name)
		.bind(matches!(state, Some(WorkflowState::Complete)))
		.bind(matches!(state, Some(WorkflowState::Running)))
		.bind(matches!(state, Some(WorkflowState::Sleeping)))
		.bind(matches!(state, Some(WorkflowState::Dead)));

	for tag in tags {
		query = query.bind(tag.key);
		query = query.bind(tag.value);
	}

	let workflows = query.fetch_all(&mut *conn).await?;

	Ok(workflows)
}

pub async fn print_workflows(workflows: Vec<WorkflowRow>, pretty: bool) -> Result<()> {
	if workflows.is_empty() {
		rivet_term::status::success("No workflows found", "");
		return Ok(());
	}

	rivet_term::status::success("Workflows", workflows.len());

	if pretty {
		for workflow in workflows {
			println!("");

			println!("{}", style(workflow.workflow_name).bold());

			println!("  {} {}", style("id").bold(), workflow.workflow_id);

			let datetime = Local
				.timestamp_millis_opt(workflow.create_ts)
				.single()
				.context("invalid ts")?;
			let date = datetime.format("%Y-%m-%d %H:%M:%S");

			println!("  {} {}", style("created at").bold(), style(date).magenta());

			print!("  {} ", style("state").bold());
			if workflow.output.is_some() {
				println!("{}", style("complete").bright().blue());
			} else if workflow.is_active {
				println!("{}", style("running").green());
			} else if workflow.has_wake_condition {
				println!("{}", style("sleeping").yellow());
			} else {
				println!("{}", style("dead").red());
			}

			if let Some(error) = workflow.error {
				println!(
					"  {} {}",
					style("error").bold(),
					style(&indent_string(&chunk_string(&error, 200).join("\n"), "    ")[4..])
						.green()
				);
			}

			println!(
				"  {} {}",
				style("input").bold(),
				&indent_string(&colored_json(&workflow.input)?, "    ")[4..]
			);

			print!("  {} ", style("output").bold());
			if let Some(output) = workflow.output {
				println!("{}", &indent_string(&colored_json(&output)?, "    ")[4..]);
			} else {
				println!("{}", style("<none>").dim());
			}
		}
	} else {
		render::workflows(workflows)?;
	}

	Ok(())
}

pub async fn silence_workflow(workflow_id: Uuid) -> Result<()> {
	let pool = build_pool().await?;
	let mut conn = pool.acquire().await?;

	sqlx::query(indoc!(
		"
		UPDATE db_workflow.workflows
		SET silence_ts = $2
		WHERE workflow_id = $1
		"
	))
	.bind(workflow_id)
	.bind(util::now())
	.execute(&mut *conn)
	.await?;

	Ok(())
}

pub async fn wake_workflow(workflow_id: Uuid) -> Result<()> {
	let pool = build_pool().await?;
	let mut conn = pool.acquire().await?;

	sqlx::query(indoc!(
		"
		UPDATE db_workflow.workflows
		SET wake_immediate = TRUE
		WHERE workflow_id = $1
		"
	))
	.bind(workflow_id)
	.execute(&mut *conn)
	.await?;

	Ok(())
}

pub async fn print_history(
	workflow_id: Uuid,
	include_forgotten: bool,
	print_location: bool,
) -> Result<()> {
	let pool = build_pool().await?;
	let mut conn = pool.acquire().await?;
	let mut conn2 = pool.acquire().await?;

	let (wf_row, events) = tokio::try_join!(
		async move {
			sqlx::query_as::<_, (String, serde_json::Value, Option<serde_json::Value>)>(indoc!(
				"
				SELECT workflow_name, input, output
				FROM db_workflow.workflows
				WHERE workflow_id = $1
				"
			))
			.bind(workflow_id)
			.fetch_optional(&mut *conn)
			.await
			.map_err(Into::into)
		},
		async move {
			sqlx::query_as::<_, HistoryEvent>(indoc!(
				"
			WITH workflow_events AS (
				SELECT $1 AS workflow_id
			)
			SELECT location, 0 AS t, activity_name AS name, input, output, forgotten
			FROM db_workflow.workflow_activity_events, workflow_events
			WHERE
				workflow_activity_events.workflow_id = workflow_events.workflow_id AND ($2 OR NOT forgotten)
			UNION ALL
			SELECT location, 1 AS t, signal_name AS name, body as input, null as output, forgotten
			FROM db_workflow.workflow_signal_events, workflow_events
			WHERE
				workflow_signal_events.workflow_id = workflow_events.workflow_id AND ($2 OR NOT forgotten)
			UNION ALL
			SELECT location, 2 AS t, w.workflow_name AS name, w.input, w.output, forgotten
			FROM workflow_events, db_workflow.workflow_sub_workflow_events AS sw
			JOIN db_workflow.workflows AS w
			ON sw.sub_workflow_id = w.workflow_id
			WHERE
				sw.workflow_id = workflow_events.workflow_id AND ($2 OR NOT forgotten)
			UNION ALL
			SELECT location, 3 AS t, signal_name AS name, body as input, null as output, forgotten
			FROM db_workflow.workflow_signal_send_events, workflow_events
			WHERE
				workflow_signal_send_events.workflow_id = workflow_events.workflow_id AND ($2 OR NOT forgotten)
			UNION ALL
			SELECT location, 4 AS t, message_name AS name, body as input, null as output, forgotten
			FROM db_workflow.workflow_message_send_events, workflow_events
			WHERE
				workflow_message_send_events.workflow_id = workflow_events.workflow_id AND ($2 OR NOT forgotten)
			UNION ALL
			SELECT location, 5 AS t, NULL AS name, null as input, null as output, forgotten
			FROM db_workflow.workflow_loop_events, workflow_events
			WHERE
				workflow_loop_events.workflow_id = workflow_events.workflow_id AND ($2 OR NOT forgotten)
			ORDER BY location ASC;
			"
			))
			.bind(workflow_id)
			.bind(include_forgotten)
			.fetch_all(&mut *conn2)
			.await
			.map_err(Into::into)
		}
	)?;

	let Some((workflow_name, input, output)) = wf_row else {
		rivet_term::status::success("No workflow found", "");

		return Ok(());
	};

	// Print header
	{
		println!("");

		println!(
			"{} {}",
			style(workflow_name).yellow().bold(),
			style(workflow_id).yellow()
		);

		print!(
			"  {} {}",
			style("╰").yellow().dim(),
			style("input").yellow()
		);

		let input = serde_json::to_string(&input)?;
		let input_trim = input.chars().take(50).collect::<String>();
		print!(" {}", style(input_trim).yellow().dim());
		if input.len() > 50 {
			print!(" {}", style("...").yellow().dim());
		}

		println!("\n");
	}

	for i in 0..events.len() {
		let event = events.get(i).unwrap();

		let t = EventType::from_repr(event.t.try_into()?).context("invalid event type")?;

		// Indentation
		print!("{}", "  ".repeat(event.location.len().saturating_sub(1)));

		if print_location {
			print!("{} ", style(event.location.last().unwrap()).dim());
		}

		if event.forgotten {
			print!("{}", style(t.name()).red().dim().bold());
		} else {
			print!("{}", style(t.name()).bold());
		}

		if let Some(name) = &event.name {
			print!(" {}", style(name));
		}

		println!("");

		if let Some(input) = &event.input {
			print!("{}", "  ".repeat(event.location.len()));

			let c = if event.output.is_none() { "╰" } else { "├" };
			let c = if event.forgotten {
				style(c).red().dim()
			} else {
				style(c).dim()
			};
			print!("{} ", c);

			print!("input");

			let input = serde_json::to_string(&input)?;
			let input_trim = input.chars().take(50).collect::<String>();
			print!(" {}", style(input_trim).dim());
			if input.len() > 50 {
				print!(" {}", style("...").dim());
			}

			println!("");
		}

		if let Some(output) = &event.output {
			print!("{}", "  ".repeat(event.location.len()));

			if event.forgotten {
				print!("{} ", style("╰").red().dim());
			} else {
				print!("{} ", style("╰").dim());
			}

			print!("output");

			let output = serde_json::to_string(&output)?;
			let output_trim = output.chars().take(50).collect::<String>();
			print!(" {}", style(output_trim).dim());
			if output.len() > 50 {
				print!(" {}", style("...").dim());
			}

			println!("");
		}

		if !matches!(t, EventType::Loop) {
			println!("");
		}

		let next_event = events.get(i + 1);
		if let Some(ne) = next_event {
			match event.location.len().cmp(&ne.location.len()) {
				Ordering::Equal => {}
				Ordering::Less => {
					let start = if matches!(t, EventType::Loop) { 1 } else { 0 };

					for i in start..ne.location.len() - event.location.len() {
						print!(
							"{}",
							"  ".repeat(event.location.len().saturating_sub(1) + i)
						);

						if print_location {
							print!(
								"{} ",
								style(ne.location.get(ne.location.len() - 2).unwrap()).dim()
							);
						}

						if ne.forgotten {
							print!("{}", style("branch").red().dim().bold());
						} else {
							print!("{}", style("branch").bold());
						}

						println!();
					}
				}
				Ordering::Greater => {
					for (j, (a, b)) in event.location.iter().zip(ne.location.iter()).enumerate() {
						if a != b {
							if j + 1 != ne.location.len() {
								// Indentation
								print!("{}", "  ".repeat(j));

								if print_location {
									print!("{} ", style(b).dim());
								}

								if ne.forgotten {
									print!("{}", style("branch").red().dim().bold());
								} else {
									print!("{}", style("branch").bold());
								}

								println!();
							}

							break;
						}
					}
				}
			}
		}
	}

	// Print footer
	if let Some(output) = output {
		println!("{}", style("Workflow complete").yellow().bold());

		print!(
			"  {} {}",
			style("╰").yellow().dim(),
			style("output").yellow()
		);

		let output = serde_json::to_string(&output)?;
		let output_trim = output.chars().take(50).collect::<String>();
		print!(" {}", style(output_trim).yellow().dim());
		if output.len() > 50 {
			print!(" {}", style("...").yellow().dim());
		}

		println!("");
	}

	Ok(())
}

async fn build_pool() -> Result<PgPool> {
	let pool = rivet_pools::crdb_from_env("rivet-workflow".into())
		.await?
		.context("missing crdb pool")?;
	Ok(pool)
}

fn chunk_string(s: &str, size: usize) -> Vec<String> {
	s.as_bytes()
		.chunks(size)
		.map(|chunk| String::from_utf8_lossy(chunk).to_string())
		.collect()
}

mod render {
	use anyhow::*;
	use rivet_term::console::style;
	use tabled::Tabled;
	use uuid::Uuid;

	use super::{WorkflowRow, WorkflowState};
	use crate::util::format::colored_json_ugly;

	#[derive(Tabled)]
	struct WorkflowTableRow {
		pub workflow_name: String,
		pub workflow_id: Uuid,
		#[tabled(display_with = "display_state")]
		pub state: WorkflowState,
		#[tabled(display_with = "display_option")]
		pub tags: Option<String>,
	}

	pub fn workflows(workflows: Vec<WorkflowRow>) -> Result<()> {
		let mut rows = workflows
			.iter()
			.map(|w| {
				Ok(WorkflowTableRow {
					workflow_name: w.workflow_name.clone(),
					workflow_id: w.workflow_id,
					state: if w.output.is_some() {
						WorkflowState::Complete
					} else if w.is_active {
						WorkflowState::Running
					} else if w.has_wake_condition {
						WorkflowState::Sleeping
					} else {
						WorkflowState::Dead
					},
					tags: w.tags.as_ref().map(colored_json_ugly).transpose()?,
				})
			})
			.collect::<Result<Vec<_>>>()?;

		rows.sort_by_key(|w| w.state);

		rivet_term::format::table(rows);

		Ok(())
	}

	fn display_state(state: &WorkflowState) -> String {
		match state {
			WorkflowState::Complete => style("complete").bright().blue().to_string(),
			WorkflowState::Running => style("running").green().to_string(),
			WorkflowState::Sleeping => style("sleeping").yellow().to_string(),
			WorkflowState::Dead => style("dead").red().to_string(),
		}
	}

	pub(crate) fn display_option<T: std::fmt::Display>(item: &Option<T>) -> String {
		match item {
			Some(s) => s.to_string(),
			None => String::new(),
		}
	}
}
