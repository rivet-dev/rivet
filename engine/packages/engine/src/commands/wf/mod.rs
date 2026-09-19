use std::sync::Arc;

use anyhow::{Result, ensure};
use clap::{Parser, ValueEnum};
use gas::{
	db::{
		self, Database,
		debug::{
			DatabaseDebug, RepairVariant as DebugRepairVariant, WorkflowState as DebugWorkflowState,
		},
	},
	history::location::Location,
};
use rivet_util::Id;

use crate::util::{self, wf::KvPair};

mod dead;
mod repair;
mod signal;

#[derive(Parser)]
pub enum SubCommand {
	/// Prints the given workflow(s).
	Get { workflow_ids: Vec<Id> },
	/// Finds workflows with the given tags, name and state.
	List {
		tags: Vec<KvPair>,
		/// Workflow name.
		#[clap(long, short = 'n')]
		name: Option<String>,
		#[clap(long, short = 's')]
		state: Option<WorkflowState>,
		/// Prints paragraphs instead of a table.
		#[clap(long, short = 'p')]
		pretty: bool,
	},
	/// Silences a workflow from showing up as dead or running again.
	Silence { workflow_ids: Vec<Id> },
	/// Sets the wake immediate property of a workflow to true.
	Wake { workflow_ids: Vec<Id> },
	/// Wakes dead workflows that match the name and error queries.
	Revive {
		#[clap(short = 'n', long)]
		name: Vec<String>,
		/// Matches via substring (i.e. error = "database" will match workflows that died with error "database transaction failed").
		#[clap(short = 'e', long)]
		error: Vec<String>,
		#[clap(short = 'd', long)]
		dry_run: bool,
	},
	/// Lists dead workflows that match the name and error queries, with the repairs that apply to
	/// each.
	Dead {
		#[clap(short = 'n', long)]
		name: Vec<String>,
		/// Matches via substring (i.e. error = "database" will match workflows that died with error "database transaction failed").
		#[clap(short = 'e', long)]
		error: Vec<String>,
		/// How many workflows to inspect at once.
		#[clap(short = 'p', long)]
		parallelization: Option<u16>,
	},
	/// Deletes the history for completed workflows that match the name and before filter.
	PruneHistory {
		#[clap(short = 'n', long)]
		name: Vec<String>,
		#[clap(short = 'b', long)]
		before: chrono::DateTime<chrono::Utc>,
		#[clap(short = 'd', long)]
		dry_run: bool,
		#[clap(short = 'p', long)]
		parallelization: Option<u16>,
	},
	/// Lists the entire event history of a workflow.
	History {
		#[clap(index = 1)]
		workflow_id: Id,
		/// Excludes all JSON in history graph.
		#[clap(short = 'j', long)]
		exclude_json: bool,
		/// Includes forgotten events in graph, shown in red.
		#[clap(short = 'f', long)]
		include_forgotten: bool,
		/// Includes location numbers for events in graph.
		#[clap(short = 'l', long)]
		print_location: bool,
		/// Includes create timestamps for events in graph. Two of this flag enables millisecond display.
		#[clap(short = 't', action = clap::ArgAction::Count, long)]
		print_ts: u8,
	},
	/// Repairs a dead workflow that has a known history defect.
	///
	/// Determines which repair applies by validating the workflow's raw history, prints every check
	/// it ran, then applies the repair, wakes the workflow, and verifies that it replayed. Repair
	/// one workflow at a time and confirm it is healthy before moving to the next.
	Repair {
		#[clap(index = 1)]
		workflow_id: Id,
		/// Only inspects and applies this repair instead of every known repair. Use this when a
		/// workflow matches a detect-only symptom that would otherwise block an automatic repair.
		#[clap(long, short = 'v')]
		variant: Option<RepairVariant>,
		/// Exact history location to repair. Only needed when a repair reports more than one
		/// candidate location.
		#[clap(long, short = 'l')]
		location: Option<Location>,
		/// Skips the confirmation prompt.
		#[clap(long, short = 'y')]
		yes: bool,
		/// Only inspects and prints the diagnosis, never changes anything.
		#[clap(long, short = 'd')]
		dry_run: bool,
	},
	Signal {
		#[clap(subcommand)]
		command: signal::SubCommand,
	},
	/// Prints the current workflow registry
	Registry {},
}

impl SubCommand {
	pub async fn execute(self, config: rivet_config::Config) -> Result<()> {
		let pools = rivet_pools::Pools::new(config.clone()).await?;
		let db = db::DatabaseKv::new(config.clone(), pools).await? as Arc<dyn DatabaseDebug>;

		match self {
			Self::Get { workflow_ids } => {
				let workflows = DatabaseDebug::get_workflows(&*db, workflow_ids).await?;
				util::wf::print_workflows(workflows, true).await
			}
			Self::List {
				tags,
				name,
				state,
				pretty,
			} => {
				let workflows = DatabaseDebug::find_workflows(
					&*db,
					&tags
						.into_iter()
						.map(|kv| (kv.key, kv.value))
						.collect::<Vec<_>>(),
					name.as_deref(),
					state.map(Into::into),
				)
				.await?;
				util::wf::print_workflows(workflows, pretty).await
			}
			Self::Silence { workflow_ids } => db.silence_workflows(workflow_ids).await,
			Self::Wake { workflow_ids } => db.wake_workflows(workflow_ids).await,
			Self::Revive {
				name,
				error,
				dry_run,
			} => {
				ensure!(!name.is_empty(), "must provide at least one name");

				let total = db
					.revive_workflows(
						&name.iter().map(|x| x.as_str()).collect::<Vec<_>>(),
						&error.iter().map(|x| x.as_str()).collect::<Vec<_>>(),
						dry_run,
					)
					.await?;

				if dry_run {
					rivet_term::status::success("Workflows Matched", total);
				} else {
					rivet_term::status::success("Workflows Revived", total);
				}

				Ok(())
			}
			Self::Dead {
				name,
				error,
				parallelization,
			} => {
				ensure!(!name.is_empty(), "must provide at least one name");

				dead::execute(
					&*db,
					&name.iter().map(|x| x.as_str()).collect::<Vec<_>>(),
					&error.iter().map(|x| x.as_str()).collect::<Vec<_>>(),
					usize::from(parallelization.unwrap_or(8)),
				)
				.await
			}
			Self::PruneHistory {
				name,
				before,
				dry_run,
				parallelization,
			} => {
				let total = db
					.prune_complete_workflow_history(
						&name.iter().map(|x| x.as_str()).collect::<Vec<_>>(),
						before.timestamp_millis(),
						dry_run,
						parallelization.unwrap_or(1),
					)
					.await?;

				if dry_run {
					rivet_term::status::success("Workflows Matched", total);
				} else {
					rivet_term::status::success("Workflows Pruned", total);
				}

				Ok(())
			}
			Self::History {
				workflow_id,
				exclude_json,
				include_forgotten,
				print_location,
				print_ts,
			} => {
				let history = db
					.get_workflow_history(workflow_id, include_forgotten)
					.await?;
				util::wf::print_history(history, exclude_json, print_location, print_ts).await
			}
			Self::Repair {
				workflow_id,
				variant,
				location,
				yes,
				dry_run,
			} => {
				repair::execute(
					&*db,
					workflow_id,
					variant.map(Into::into),
					location,
					yes,
					dry_run,
				)
				.await
			}
			Self::Signal { command } => command.execute(db).await,
			Self::Registry {} => {
				let reg = rivet_workflow_worker::registry(&config)?;
				let mut names = reg.names();
				names.sort();

				rivet_term::status::success("Workflows", names.len());
				println!();

				for name in names {
					println!("{name}");
				}

				Ok(())
			}
		}
	}
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
#[clap(rename_all = "kebab_case")]
pub enum RepairVariant {
	DeallocateSetError,
	OrphanedSleepState,
	SleepStateMismatch,
	DuplicateIterationHistory,
	LoopIterationMismatch,
	IterationTimestampInversion,
	ProposeConsensusFailed,
	MissingInitState,
}

impl From<RepairVariant> for DebugRepairVariant {
	fn from(variant: RepairVariant) -> Self {
		match variant {
			RepairVariant::DeallocateSetError => DebugRepairVariant::DeallocateSetError,
			RepairVariant::OrphanedSleepState => DebugRepairVariant::OrphanedSleepState,
			RepairVariant::SleepStateMismatch => DebugRepairVariant::SleepStateMismatch,
			RepairVariant::DuplicateIterationHistory => {
				DebugRepairVariant::DuplicateIterationHistory
			}
			RepairVariant::LoopIterationMismatch => DebugRepairVariant::LoopIterationMismatch,
			RepairVariant::IterationTimestampInversion => {
				DebugRepairVariant::IterationTimestampInversion
			}
			RepairVariant::ProposeConsensusFailed => DebugRepairVariant::ProposeConsensusFailed,
			RepairVariant::MissingInitState => DebugRepairVariant::MissingInitState,
		}
	}
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[clap(rename_all = "kebab_case")]
pub enum WorkflowState {
	Complete,
	Running,
	Sleeping,
	Dead,
	Silenced,
}

impl From<WorkflowState> for DebugWorkflowState {
	fn from(state: WorkflowState) -> Self {
		match state {
			WorkflowState::Complete => DebugWorkflowState::Complete,
			WorkflowState::Running => DebugWorkflowState::Running,
			WorkflowState::Sleeping => DebugWorkflowState::Sleeping,
			WorkflowState::Dead => DebugWorkflowState::Dead,
			WorkflowState::Silenced => DebugWorkflowState::Silenced,
		}
	}
}
