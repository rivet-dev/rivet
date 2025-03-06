use std::sync::Arc;

use anyhow::*;
use chirp_workflow::db::{
	self,
	debug::{DatabaseDebug, WorkflowState as DebugWorkflowState},
	Database,
};
use clap::{Parser, ValueEnum};
use uuid::Uuid;

use crate::util::{self, wf::KvPair};

mod signal;

#[derive(Parser)]
pub enum SubCommand {
	/// Prints the given workflow(s).
	Get { workflow_ids: Vec<Uuid> },
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
	Ack { workflow_ids: Vec<Uuid> },
	/// Sets the wake immediate property of a workflow to true.
	Wake { workflow_ids: Vec<Uuid> },
	/// Lists the entire event history of a workflow.
	History {
		#[clap(index = 1)]
		workflow_id: Uuid,
		/// Excludes all JSON in history graph.
		#[clap(short = 'j', long)]
		exclude_json: bool,
		/// Includes forgotten events in graph, shown in red.
		#[clap(short = 'f', long)]
		include_forgotten: bool,
		/// Includes location numbers for events in graph.
		#[clap(short = 'l', long)]
		print_location: bool,
		/// Includes create timestamps for events in graph.
		#[clap(short = 't', long)]
		print_ts: bool,
	},
	Signal {
		#[clap(subcommand)]
		command: signal::SubCommand,
	},
}

impl SubCommand {
	pub async fn execute(self, config: rivet_config::Config) -> Result<()> {
		let pools = rivet_pools::Pools::new(config.clone()).await?;
		// Choose db driver based on edge config
		let db = if config
			.server
			.as_ref()
			.context("missing server")?
			.rivet
			.edge
			.is_none()
		{
			db::DatabaseCrdbNats::from_pools(pools)? as Arc<dyn DatabaseDebug>
		} else {
			db::DatabaseFdbSqliteNats::from_pools(pools)? as Arc<dyn DatabaseDebug>
		};

		match self {
			Self::Get { workflow_ids } => {
				let workflows = db.get_workflows(workflow_ids).await?;
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
			Self::Ack { workflow_ids } => db.silence_workflows(workflow_ids).await,
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
}

impl From<WorkflowState> for DebugWorkflowState {
	fn from(state: WorkflowState) -> Self {
		match state {
			WorkflowState::Complete => DebugWorkflowState::Complete,
			WorkflowState::Running => DebugWorkflowState::Running,
			WorkflowState::Sleeping => DebugWorkflowState::Sleeping,
			WorkflowState::Dead => DebugWorkflowState::Dead,
		}
	}
}
