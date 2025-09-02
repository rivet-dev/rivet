use anyhow::*;
use chrono::{TimeZone, Utc};
use gas::db::debug::{Event, EventData, HistoryData, WorkflowState};
use gas::history::event::SleepState;
use rivet_term::console::{Style, style};

use crate::util::format::{chunk_string, colored_json, indent_string};

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

pub async fn print_workflows(
	workflows: Vec<gas::db::debug::WorkflowData>,
	pretty: bool,
) -> Result<()> {
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
			let date = datetime.format("%Y-%m-%d %H:%M:%S%.3f");

			println!("  {} {}", style("created at").bold(), style(date).magenta());

			println!(
				"  {} {}",
				style("state").bold(),
				display_state(&workflow.state)
			);

			println!(
				"  {} {}",
				style("tags").bold(),
				&indent_string(&colored_json(&workflow.tags)?, "    ", true)
			);

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

			println!(
				"  {} (state) {}",
				style("data").bold(),
				indent_string(&colored_json(&workflow.data)?, "    ", true)
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
	print_ts: u8,
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
			println!(
				"{} state {}",
				style("|").dim(),
				indent_string(
					&colored_json(&history.wf.data)?,
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
		let c = event_style.apply_to("|").dim();

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

		if print_ts != 0 {
			// Indent
			print!("{}{c} ", "  ".repeat(indent));

			let datetime = Utc
				.timestamp_millis_opt(event.create_ts)
				.single()
				.context("invalid ts")?;
			let date = if print_ts > 1 {
				datetime.format("%Y-%m-%d %H:%M:%S%.3f")
			} else {
				datetime.format("%Y-%m-%d %H:%M:%S")
			};

			println!("created {}", style(date).magenta());
		}

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
						let date = if print_ts > 1 {
							datetime.format("%Y-%m-%d %H:%M:%S%.3f")
						} else {
							datetime.format("%Y-%m-%d %H:%M:%S")
						};

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

				if let Some(workflow_id) = data.workflow_id {
					// Indent
					print!("{}{c} ", "  ".repeat(indent));
					println!("to workflow id {}", style(workflow_id).green());
				}

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
				let date = if print_ts > 1 {
					datetime.format("%Y-%m-%d %H:%M:%S%.3f")
				} else {
					datetime.format("%Y-%m-%d %H:%M:%S")
				};

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
	} else if let WorkflowState::Sleeping = history.wf.state {
		println!();

		println!("{}", style("Workflow sleeping").yellow().bold());

		if let Some(error) = history.wf.error {
			println!(
				"{} reason {}",
				style("|").yellow().dim(),
				style(error).green(),
			);
		}
	} else if let WorkflowState::Silenced = history.wf.state {
		println!();

		println!("{}", style("Workflow silenced").magenta().bold());

		if let Some(error) = history.wf.error {
			println!(
				"{} error {}",
				style("|").magenta().dim(),
				style(error).green(),
			);
		}
	} else {
		println!();

		println!("{}", style("Workflow dead").red().bold());

		if let Some(error) = history.wf.error {
			println!("{} error {}", style("|").red().dim(), style(error).green(),);
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
	if event.forgotten {
		print!("{}", style("forgotten ").red().dim())
	}

	let style = event_style(event);
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
			print!(
				"{}",
				style
					.apply_to(format!("removed {}", removed.event_type))
					.bold(),
			);

			if let Some(name) = &removed.name {
				print!(" {}", style.apply_to(name))
			}
		}
		EventData::VersionCheck => print!("{}", style.apply_to("version check").bold()),
		EventData::Branch => print!("{}", style.apply_to("branch").bold()),
		EventData::Empty => print!("{}", style.apply_to("empty").bold()),
	}
}

fn display_state(state: &WorkflowState) -> String {
	match state {
		WorkflowState::Complete => style("complete").bright().blue().to_string(),
		WorkflowState::Running => style("running").green().to_string(),
		WorkflowState::Sleeping => style("sleeping").yellow().to_string(),
		WorkflowState::Dead => style("dead").red().to_string(),
		WorkflowState::Silenced => style("silenced").bright().magenta().to_string(),
	}
}

mod table {
	use anyhow::*;
	use gas::db::debug::{WorkflowData, WorkflowState};
	use rivet_util::Id;
	use tabled::Tabled;

	use super::display_state;
	use crate::util::format::colored_json_ugly;

	#[derive(Tabled)]
	struct WorkflowTableRow {
		pub workflow_id: Id,
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
}
