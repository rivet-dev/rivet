use anyhow::*;
use chirp_workflow::history::{event::SleepState, location::Location};
use chrono::{TimeZone, Utc};
use clap::ValueEnum;
use indoc::indoc;
use rivet_pools::CrdbPool;
use rivet_term::console::style;
use uuid::Uuid;

use crate::util::{
	self,
	format::{chunk_string, colored_json, indent_string},
};

mod history;
use history::EventData;
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

#[derive(Debug, sqlx::FromRow)]
pub struct HistoryWorkflowRow {
	workflow_name: String,
	input: serde_json::Value,
	output: Option<serde_json::Value>,
	error: Option<String>,
	is_active: bool,
	has_wake_condition: bool,
}

#[derive(sqlx::FromRow)]
struct ActivityErrorRow {
	location: Location,
	error: String,
	count: i64,
	latest_ts: i64,
}

pub async fn get_workflows(pool: CrdbPool, workflow_ids: Vec<Uuid>) -> Result<Vec<WorkflowRow>> {
	let mut conn = pool.acquire().await?;

	let workflows = sqlx::query_as::<_, WorkflowRow>(indoc!(
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
			workflow_id = ANY($1)
		"
	))
	.bind(workflow_ids)
	.fetch_all(&mut *conn)
	.await?;

	Ok(workflows)
}

pub async fn find_workflows(
	pool: CrdbPool,
	tags: Vec<KvPair>,
	name: Option<String>,
	state: Option<WorkflowState>,
) -> Result<Vec<WorkflowRow>> {
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
			println!();

			println!("{}", style(workflow.workflow_name).bold());

			println!("  {} {}", style("id").bold(), workflow.workflow_id);

			let datetime = Utc
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
					style(indent_string(
						&chunk_string(&error, 200).join("\n"),
						"    ",
						true
					))
					.green()
				);
			}

			println!(
				"  {} {}",
				style("input").bold(),
				indent_string(&colored_json(&workflow.input)?, "    ", true)
			);

			print!("  {} ", style("output").bold());
			if let Some(output) = workflow.output {
				println!("{}", indent_string(&colored_json(&output)?, "    ", true));
			} else {
				println!("{}", style("<none>").dim());
			}
		}
	} else {
		table::workflows(workflows)?;
	}

	Ok(())
}

pub async fn silence_workflows(pool: CrdbPool, workflow_ids: Vec<Uuid>) -> Result<()> {
	let mut conn = pool.acquire().await?;

	sqlx::query(indoc!(
		"
		UPDATE db_workflow.workflows
		SET silence_ts = $2
		WHERE workflow_id = ANY($1)
		"
	))
	.bind(workflow_ids)
	.bind(util::now())
	.execute(&mut *conn)
	.await?;

	Ok(())
}

pub async fn wake_workflows(pool: CrdbPool, workflow_ids: Vec<Uuid>) -> Result<()> {
	let mut conn = pool.acquire().await?;

	sqlx::query(indoc!(
		"
		UPDATE db_workflow.workflows
		SET wake_immediate = TRUE
		WHERE workflow_id = ANY($1)
		"
	))
	.bind(workflow_ids)
	.execute(&mut *conn)
	.await?;

	Ok(())
}

pub async fn print_history(
	pool: CrdbPool,
	workflow_id: Uuid,
	exclude_json: bool,
	include_forgotten: bool,
	print_location: bool,
) -> Result<()> {
	let mut conn = pool.acquire().await?;
	let mut conn2 = pool.acquire().await?;
	let mut conn3 = pool.acquire().await?;

	let (wf_row, events, error_rows) = tokio::try_join!(
		async move {
			sqlx::query_as::<_, HistoryWorkflowRow>(indoc!(
				"
				SELECT
					workflow_name,
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
				WHERE workflow_id = $1
				"
			))
			.bind(workflow_id)
			.fetch_optional(&mut *conn)
			.await
			.map_err(Into::into)
		},
		async move {
			sqlx::query_as::<_, history::AmalgamEventRow>(indoc!(
				"
				-- Activity events
				SELECT
					location,
					location2,
					NULL AS tags,
					0 AS event_type,
					version,
					activity_name AS name,
					NULL AS auxiliary_id,
					input,
					output,
					NULL AS iteration,
					NULL AS deadline_ts,
					NULL AS state,
					NULL AS inner_event_type,
					forgotten
				FROM db_workflow.workflow_activity_events
				WHERE
					workflow_activity_events.workflow_id = $1 AND ($2 OR NOT forgotten)
				UNION ALL
				-- Signal listen events
				SELECT
					location,
					location2,
					NULL AS tags,
					1 AS event_type,
					version,
					signal_name AS name,
					signal_id::UUID AS auxiliary_id,
					NULL AS input,
					body AS output,
					NULL AS iteration,
					NULL AS deadline_ts,
					NULL AS state,
					NULL AS inner_event_type,
					forgotten
				FROM db_workflow.workflow_signal_events
				WHERE
					workflow_signal_events.workflow_id = $1 AND ($2 OR NOT forgotten)
				UNION ALL
				-- Signal send events
				SELECT
					location,
					location2,
					s.tags,
					2 AS event_type,
					version,
					se.signal_name AS name,
					se.signal_id AS auxiliary_id,
					se.body AS input,
					NULL AS output,
					NULL AS iteration,
					NULL AS deadline_ts,
					NULL AS state,
					NULL AS inner_event_type,
					forgotten
				FROM db_workflow.workflow_signal_send_events AS se
				LEFT JOIN db_workflow.tagged_signals AS s
				ON se.signal_id = s.signal_id
				WHERE
					se.workflow_id = $1 AND ($2 OR NOT forgotten)
				UNION ALL
				-- Message send events
				SELECT
					location,
					location2,
					tags,
					3 AS event_type,
					version,
					message_name AS name,
					NULL AS auxiliary_id,
					body AS input,
					NULL AS output,
					NULL AS iteration,
					NULL AS deadline_ts,
					NULL AS state,
					NULL AS inner_event_type,
					forgotten				
				FROM db_workflow.workflow_message_send_events
				WHERE
					workflow_message_send_events.workflow_id = $1 AND ($2 OR NOT forgotten)
				UNION ALL
				-- Sub workflow events
				SELECT
					location,
					location2,
					w.tags,
					4 AS event_type,
					version,
					w.workflow_name AS name,
					sw.sub_workflow_id AS auxiliary_id,
					w.input,
					w.output,
					NULL AS iteration,
					NULL AS deadline_ts,
					NULL AS state,
					NULL AS inner_event_type,
					forgotten
				FROM db_workflow.workflow_sub_workflow_events AS sw
				JOIN db_workflow.workflows AS w
				ON sw.sub_workflow_id = w.workflow_id
				WHERE
					sw.workflow_id = $1 AND ($2 OR NOT forgotten)
				UNION ALL
				-- Loop events
				SELECT
					location,
					location2,
					NULL AS tags,
					5 AS event_type,
					version,
					NULL AS name,
					NULL AS auxiliary_id,
					NULL AS input,
					NULL AS output,
					iteration,
					NULL AS deadline_ts,
					NULL AS state,
					NULL AS inner_event_type,
					forgotten
				FROM db_workflow.workflow_loop_events
				WHERE
					workflow_loop_events.workflow_id = $1 AND ($2 OR NOT forgotten)
				UNION ALL
				SELECT
					location,
					location2,
					NULL AS tags,
					6 AS event_type,
					version,
					NULL AS name,
					NULL AS auxiliary_id,
					NULL AS input,
					NULL AS output,
					NULL AS iteration,
					deadline_ts,
					state,
					NULL AS inner_event_type,
					forgotten
				FROM db_workflow.workflow_sleep_events
				WHERE
					workflow_sleep_events.workflow_id = $1 AND ($2 OR NOT forgotten)
				UNION ALL
				SELECT
					ARRAY[] AS location,
					location AS location2,
					NULL AS tags,
					7 AS event_type,
					version,
					NULL AS name,
					NULL AS auxiliary_id,
					NULL AS input,
					NULL AS output,
					NULL AS iteration,
					NULL AS deadline_ts,
					NULL AS state,
					NULL AS inner_event_type,
					forgotten
				FROM db_workflow.workflow_branch_events
				WHERE
					workflow_branch_events.workflow_id = $1 AND ($2 OR NOT forgotten)
				UNION ALL
				SELECT
					ARRAY[] AS location,
					location AS location2,
					NULL AS tags,
					8 AS event_type,
					1 AS version,
					NULL AS name,
					NULL AS auxiliary_id,
					NULL AS input,
					NULL AS output,
					NULL AS iteration,
					NULL AS deadline_ts,
					NULL AS state,
					event_type AS inner_event_type,
					forgotten
				FROM db_workflow.workflow_removed_events
				WHERE
					workflow_removed_events.workflow_id = $1 AND ($2 OR NOT forgotten)
				UNION ALL
				SELECT
					ARRAY[] AS location,
					location AS location2,
					NULL AS tags,
					9 AS event_type,
					version,
					NULL AS name,
					NULL AS auxiliary_id,
					NULL AS input,
					NULL AS output,
					NULL AS iteration,
					NULL AS deadline_ts,
					NULL AS state,
					NULL AS inner_event_type,
					forgotten
				FROM db_workflow.workflow_version_check_events
				WHERE
					workflow_version_check_events.workflow_id = $1 AND ($2 OR NOT forgotten)
				ORDER BY location ASC, location2 ASC
				"
			))
			.bind(workflow_id)
			.bind(include_forgotten)
			.fetch_all(&mut *conn2)
			.await
			.map_err(Into::into)
		},
		async move {
			sqlx::query_as::<_, ActivityErrorRow>(indoc!(
				"
				SELECT location2 AS location, error, COUNT(error), MAX(ts) AS latest_ts
				FROM db_workflow.workflow_activity_errors
				WHERE workflow_id = $1
				GROUP BY location2, error
				ORDER BY latest_ts
				"
			))
			.bind(workflow_id)
			.fetch_all(&mut *conn3)
			.await
			.map_err(Into::into)
		},
	)?;

	let Some(workflow) = wf_row else {
		rivet_term::status::success("No workflow found", "");

		return Ok(());
	};

	let history = history::build(events)?;

	// Print header
	{
		println!();

		println!(
			"{} {}",
			style(workflow.workflow_name).bold(),
			style(workflow_id)
		);

		if !exclude_json {
			println!(
				"{} input {}",
				style("|").dim(),
				indent_string(
					&colored_json(&workflow.input)?,
					style("  | ").dim().to_string(),
					true
				)
			);
		}

		println!();
	}

	for i in 0..history.len() {
		let event = history.get(i).unwrap();
		let indent = event.location.len();

		// Indentation
		print!(
			"{}{} ",
			"  ".repeat(indent.saturating_sub(1)),
			event.style().apply_to("-"),
		);

		// Structure char
		let c = if event.forgotten {
			style("|").red().dim()
		} else {
			event.style().apply_to("|").dim()
		};

		event.print_name();

		if print_location {
			print!(
				" {}",
				event
					.style()
					.apply_to(format!("v{} @ {}", event.version, event.location))
					.dim()
			);
		}

		println!();

		// TODO: Color code each (make header white instead of yellow)
		match &event.data {
			EventData::Activity(data) => {
				let errors = error_rows
					.iter()
					.filter(|row| row.location == event.location)
					.collect::<Vec<_>>();

				if !exclude_json {
					// Indent
					print!("{}{c} ", "  ".repeat(indent));

					println!(
						"input {}",
						indent_string(
							&colored_json(&data.input)?,
							format!("{}{c} ", "  ".repeat(indent)),
							true
						)
					);

					if let Some(output) = &data.output {
						// Indent
						print!("{}{c} ", "  ".repeat(indent));

						println!(
							"output {}",
							indent_string(
								&colored_json(&output)?,
								format!("{}{c} ", "  ".repeat(indent)),
								true
							)
						);
					}
				}

				if !errors.is_empty() {
					// Indent
					print!("{}{c} ", "  ".repeat(indent));

					println!("errors");

					for error in errors {
						print!("{}{c}   - ", "  ".repeat(indent));

						let datetime = Utc
							.timestamp_millis_opt(error.latest_ts)
							.single()
							.context("invalid ts")?;
						let date = datetime.format("%Y-%m-%d %H:%M:%S");

						println!(
							"{} {} {}",
							style(format!("x{}", error.count)).yellow().bold(),
							style(format!("(last {})", date)).magenta(),
							style(error.error.replace('\n', " ")).green(),
						);
					}
				}
			}
			EventData::Signal(data) => {
				// Indent
				print!("{}{c} ", "  ".repeat(indent));

				println!("id {}", style(data.signal_id).green());

				if !exclude_json {
					// Indent
					print!("{}{c} ", "  ".repeat(indent));

					println!(
						"body {}",
						indent_string(
							&colored_json(&data.body)?,
							format!("{}{c} ", "  ".repeat(indent)),
							true
						)
					);
				}
			}
			EventData::SignalSend(data) => {
				// Indent
				print!("{}{c} ", "  ".repeat(indent));

				println!("id {}", style(data.signal_id).green());

				if !exclude_json {
					if let Some(tags) = &data.tags {
						if tags.as_object().map(|x| !x.is_empty()).unwrap_or_default() {
							// Indent
							print!("{}{c} ", "  ".repeat(indent));

							println!(
								"tags {}",
								indent_string(
									&colored_json(&tags)?,
									format!("{}{c} ", "  ".repeat(indent)),
									true
								)
							);
						}
					}

					// Indent
					print!("{}{c} ", "  ".repeat(indent));

					println!(
						"body {}",
						indent_string(
							&colored_json(&data.body)?,
							format!("{}{c} ", "  ".repeat(indent)),
							true
						)
					);
				}
			}
			EventData::MessageSend(data) => {
				if !exclude_json {
					if data
						.tags
						.as_object()
						.map(|x| !x.is_empty())
						.unwrap_or_default()
					{
						// Indent
						print!("{}{c} ", "  ".repeat(indent));

						println!(
							"tags {}",
							indent_string(
								&colored_json(&data.tags)?,
								format!("{}{c} ", "  ".repeat(indent)),
								true
							)
						);
					}

					// Indent
					print!("{}{c} ", "  ".repeat(indent));

					println!(
						"{} {}",
						"body",
						indent_string(
							&colored_json(&data.body)?,
							format!("{}{c} ", "  ".repeat(indent)),
							true
						)
					);
				}
			}
			EventData::SubWorkflow(data) => {
				// Indent
				print!("{}{c} ", "  ".repeat(indent));

				println!("id {}", style(data.sub_workflow_id).green());

				if !exclude_json {
					if data
						.tags
						.as_object()
						.map(|x| !x.is_empty())
						.unwrap_or_default()
					{
						// Indent
						print!("{}{c} ", "  ".repeat(indent));

						println!(
							"tags {}",
							indent_string(
								&colored_json(&data.tags)?,
								format!("{}{c} ", "  ".repeat(indent)),
								true
							)
						);
					}

					// Indent
					print!("{}{c} ", "  ".repeat(indent));

					println!(
						"{} {}",
						"input",
						indent_string(
							&colored_json(&data.input)?,
							format!("{}{c} ", "  ".repeat(indent)),
							true
						)
					);
				}
			}
			EventData::Loop(data) => {
				// Indent
				print!("{}{c} ", "  ".repeat(indent));

				println!("iteration {}", style(data.iteration).yellow());

				if let Some(output) = &data.output {
					// Indent
					print!("{}{c} ", "  ".repeat(indent));

					println!(
						"output {}",
						indent_string(
							&colored_json(&output)?,
							format!("{}{c} ", "  ".repeat(indent)),
							true
						)
					);
				}
			}
			EventData::Sleep(data) => {
				// Indent
				print!("{}{c} ", "  ".repeat(indent));

				let datetime = Utc
					.timestamp_millis_opt(data.deadline_ts)
					.single()
					.context("invalid ts")?;
				let date = datetime.format("%Y-%m-%d %H:%M:%S");

				println!("deadline {}", style(date).magenta());

				match data.state {
					SleepState::Normal => {}
					_ => {
						// Indent
						print!("{}{c} ", "  ".repeat(indent));

						println!("state {}", style(data.state).blue());
					}
				}
			}
			_ => {}
		}
	}

	// Print footer
	if let Some(output) = workflow.output {
		println!();

		println!("{}", style("Workflow complete").bright().blue().bold());

		if !exclude_json {
			println!(
				"  {} output {}",
				style("|").blue(),
				indent_string(
					&colored_json(&output)?,
					style("  | ").blue().to_string(),
					true
				)
			);
		}
	} else if workflow.is_active {
		println!();

		println!("{}", style("Workflow running").green().bold());
	} else {
		println!();

		if workflow.has_wake_condition {
			println!("{}", style("Workflow sleeping").yellow().bold());

			if let Some(error) = workflow.error {
				println!(
					"{} reason {}",
					style("|").yellow().dim(),
					style(error).green(),
				);
			}
		} else {
			println!("{}", style("Workflow dead").red().bold());

			if let Some(error) = workflow.error {
				println!("{} error {}", style("|").red().dim(), style(error).green(),);
			}
		}
	}

	println!();

	Ok(())
}

mod table {
	use anyhow::*;
	use rivet_term::console::style;
	use tabled::Tabled;
	use uuid::Uuid;

	use super::{WorkflowRow, WorkflowState};
	use crate::util::format::colored_json_ugly;

	#[derive(Tabled)]
	struct WorkflowTableRow {
		pub workflow_id: Uuid,
		pub workflow_name: String,
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
