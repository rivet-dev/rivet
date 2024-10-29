use anyhow::*;
use clap::Parser;
use uuid::Uuid;

use crate::util::{
	self,
	wf::{KvPair, WorkflowState},
};

mod signal;

#[derive(Parser)]
pub enum SubCommand {
	/// Prints the given workflow(s).
	Get {
		workflow_ids: Vec<Uuid>,
	},
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
	Ack {
		workflow_ids: Vec<Uuid>,
	},
	/// Sets the wake immediate property of a workflow to true.
	Wake {
		workflow_ids: Vec<Uuid>,
	},
	/// Lists the entire event history of a workflow.
	History {
		#[clap(index = 1)]
		workflow_id: Uuid,
		/// Includes activity errors in graph.
		#[clap(short = 'e', long)]
		include_errors: bool,
		/// Includes forgotten events in graph, shown in red.
		#[clap(short = 'f', long)]
		include_forgotten: bool,
		/// Includes location numbers for events in graph.
		#[clap(short = 'l', long)]
		print_location: bool,
	},
	Signal {
		#[clap(subcommand)]
		command: signal::SubCommand,
	},
}

impl SubCommand {
	pub async fn execute(self, config: rivet_config::Config) -> Result<()> {
		match self {
			Self::Get { workflow_ids } => {
				let pool = util::wf::build_pool(&config).await?;
				let workflows = util::wf::get_workflows(pool, workflow_ids).await?;
				util::wf::print_workflows(workflows, true).await
			}
			Self::List {
				tags,
				name,
				state,
				pretty,
			} => {
				let pool = util::wf::build_pool(&config).await?;
				let workflows = util::wf::find_workflows(pool, tags, name, state).await?;
				util::wf::print_workflows(workflows, pretty).await
			}
			Self::Ack { workflow_ids } => {
				let pool = util::wf::build_pool(&config).await?;
				util::wf::silence_workflows(pool, workflow_ids).await
			}
			Self::Wake { workflow_ids } => {
				let pool = util::wf::build_pool(&config).await?;
				util::wf::wake_workflows(pool, workflow_ids).await
			}
			Self::History {
				workflow_id,
				include_errors,
				include_forgotten,
				print_location,
			} => {
				let pool = util::wf::build_pool(&config).await?;
				util::wf::print_history(
					pool,
					workflow_id,
					include_errors,
					include_forgotten,
					print_location,
				)
				.await
			}
			Self::Signal { command } => command.execute(config).await,
		}
	}
}
