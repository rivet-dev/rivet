use std::sync::Arc;

use anyhow::*;
use clap::{Parser, ValueEnum};
use gas::db::{
	self, Database,
	debug::{DatabaseDebug, WorkflowState as DebugWorkflowState},
};
use rivet_util::Id;

use crate::util::{self, wf::KvPair};

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
	Signal {
		#[clap(subcommand)]
		command: signal::SubCommand,
	},
}

impl SubCommand {
	pub async fn execute(self, config: rivet_config::Config) -> Result<()> {
		let pools = rivet_pools::Pools::new(config.clone()).await?;
		let db = db::DatabaseKv::from_pools(pools).await? as Arc<dyn DatabaseDebug>;

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
				let workflows = db
					.find_workflows(
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
			Self::Signal { command } => command.execute(db).await,
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
