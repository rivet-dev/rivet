use anyhow::*;
use chirp_workflow::history::{
	event::SleepState,
	location::{Coordinate, Location},
};
use chrono::{TimeZone, Utc};
use indoc::indoc;
use rivet_pools::CrdbPool;
use rivet_term::console::style;
use uuid::Uuid;
use rivet_term::console::Style;
use chirp_workflow::db::debug::{WorkflowData, Event, HistoryData, WorkflowState, EventData};

use crate::util::{
	self,
	format::{chunk_string, colored_json, indent_string},
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

pub async fn print_workflows(workflows: Vec<chirp_workflow::db::debug::WorkflowData>, pretty: bool) -> Result<()> {
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
			match workflow.state {
				WorkflowState::Complete => println!("{}", style("complete").bright().blue()),
				WorkflowState::Running => println!("{}", style("running").green()),
				WorkflowState::Sleeping => println!("{}", style("sleeping").yellow()),
				WorkflowState::Dead => println!("{}", style("dead").red()),
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

pub async fn print_history(
	history: Option<HistoryData>,
	exclude_json: bool,
	print_location: bool,
) -> Result<()> {
	let Some(history) = history else {
		rivet_term::status::success("No workflow found", "");

		return Ok(());
	};

	// Print header
	{
		println!();

		println!(
			"{} {}",
			style(history.wf.workflow_name).bold(),
			style(history.wf.workflow_id)
		);

		if !exclude_json {
			println!(
				"{} tags {}",
				style("|").dim(),
				indent_string(
					&colored_json(&history.wf.tags)?,
					style("| ").dim().to_string(),
					true
				)
			);
			println!(
				"{} input {}",
				style("|").dim(),
				indent_string(
					&colored_json(&history.wf.input)?,
					style("| ").dim().to_string(),
					true
				)
			);
		}

		println!();
	}

	for event in history.events {
		let event_style = event_style(&event);
		let indent = event.location.len();

		// Indentation
		print!(
			"{}{} ",
			"  ".repeat(indent.saturating_sub(1)),
			event_style.apply_to("-"),
		);

		// Structure char
		let c = if event.forgotten {
			style("|").red().dim()
		} else {
			event_style.apply_to("|").dim()
		};

		print_event_name(&event);

		if print_location {
			print!(
				" {}",
				event_style
					.apply_to(format!("v{} @ {}", event.version, event.location))
					.dim()
			);
		}

		println!();

		// TODO: Color code each (make header white instead of yellow)
		match &event.data {
			EventData::Activity(data) => {
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

				if !data.errors.is_empty() {
					// Indent
					print!("{}{c} ", "  ".repeat(indent));

					println!("errors");

					for error in &data.errors {
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

				if !exclude_json {
					// Indent
					print!("{}{c} ", "  ".repeat(indent));

					println!(
						"state {}",
						indent_string(
							&colored_json(&data.state)?,
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
	if let Some(output) = history.wf.output {
		println!();

		println!("{}", style("Workflow complete").bright().blue().bold());

		if !exclude_json {
			println!(
				"{} output {}",
				style("|").blue(),
				indent_string(
					&colored_json(&output)?,
					style("| ").blue().to_string(),
					true
				)
			);
		}
	} else if let WorkflowState::Running = history.wf.state {
		println!();

		println!("{}", style("Workflow running").green().bold());
	} else {
		println!();

		if let WorkflowState::Sleeping = history.wf.state {
			println!("{}", style("Workflow sleeping").yellow().bold());

			if let Some(error) = history.wf.error {
				println!(
					"{} reason {}",
					style("|").yellow().dim(),
					style(error).green(),
				);
			}
		} else {
			println!("{}", style("Workflow dead").red().bold());

			if let Some(error) = history.wf.error {
				println!("{} error {}", style("|").red().dim(), style(error).green(),);
			}
		}
	}

	println!();

	Ok(())
}

pub fn event_style(event: &Event) -> Style {
	match &event.data {
		EventData::Activity(_) => Style::new().yellow(),
		EventData::Signal(_) => Style::new().cyan(),
		EventData::SignalSend(_) => Style::new().bright().blue(),
		EventData::MessageSend(_) => Style::new().bright().blue(),
		EventData::SubWorkflow(_) => Style::new().green(),
		EventData::Loop(_) => Style::new().magenta(),
		EventData::Sleep(_) => Style::new().magenta(),
		EventData::Removed(_) => Style::new().red(),
		EventData::VersionCheck => Style::new().red(),
		EventData::Branch => Style::new(),
		EventData::Empty => Style::new(),
	}
}

pub fn print_event_name(event: &Event) {
	let style = if event.forgotten {
		Style::new().red().dim()
	} else {
		event_style(event)
	};

	match &event.data {
		EventData::Activity(activity) => print!(
			"{} {}",
			style.apply_to("activity").bold(),
			style.apply_to(&activity.name)
		),
		EventData::Signal(signal) => print!(
			"{} {}",
			style.apply_to("signal receive").bold(),
			style.apply_to(&signal.name)
		),
		EventData::SignalSend(signal_send) => print!(
			"{} {}",
			style.apply_to("signal send").bold(),
			style.apply_to(&signal_send.name)
		),
		EventData::MessageSend(message_send) => print!(
			"{} {}",
			style.apply_to("message send").bold(),
			style.apply_to(&message_send.name)
		),
		EventData::SubWorkflow(sub_workflow) => print!(
			"{} {}",
			style.apply_to("sub workflow").bold(),
			style.apply_to(&sub_workflow.name)
		),
		EventData::Loop(_) => print!("{}", style.apply_to("loop").bold()),
		EventData::Sleep(_) => print!("{}", style.apply_to("sleep").bold()),
		EventData::Removed(removed) => {
			if let Some(name) = &removed.name {
				print!(
					"{} {}",
					style
						.apply_to(format!("removed {}", removed.event_type))
						.bold(),
					style.apply_to(name)
				)
			} else {
				print!(
					"{}",
					style
						.apply_to(format!("removed {}", removed.event_type))
						.bold()
				)
			}
		}
		EventData::VersionCheck => print!("{}", style.apply_to("version check").bold()),
		EventData::Branch => print!("{}", style.apply_to("branch").bold()),
		EventData::Empty => print!("{}", style.apply_to("empty").bold()),
	}
}

mod table {
	use anyhow::*;
	use rivet_term::console::style;
	use tabled::Tabled;
	use uuid::Uuid;
	use chirp_workflow::db::debug::{WorkflowData, WorkflowState};

	use crate::util::format::colored_json_ugly;

	#[derive(Tabled)]
	struct WorkflowTableRow {
		pub workflow_id: Uuid,
		pub workflow_name: String,
		#[tabled(display_with = "display_state")]
		pub state: WorkflowState,
		pub tags: String,
	}

	pub fn workflows(workflows: Vec<WorkflowData>) -> Result<()> {
		let mut rows = workflows
			.iter()
			.map(|w| {
				Ok(WorkflowTableRow {
					workflow_name: w.workflow_name.clone(),
					workflow_id: w.workflow_id,
					state: w.state,
					tags: colored_json_ugly(&w.tags)?,
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
}
