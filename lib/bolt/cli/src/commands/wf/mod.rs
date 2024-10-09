use anyhow::*;
use bolt_core::{
	context::ProjectContext,
	tasks::{
		self,
		wf::{KvPair, WorkflowState},
	},
};
use clap::Parser;
use uuid::Uuid;

mod signal;

#[derive(Parser)]
pub enum SubCommand {
	/// Prints the given workflow.
	Get {
		#[clap(index = 1)]
		workflow_id: Uuid,
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
		#[clap(index = 1)]
		workflow_id: Uuid,
	},
	/// Sets the wake immediate property of a workflow to true.
	Wake {
		#[clap(index = 1)]
		workflow_id: Uuid,
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
	/// Starts a port forward to CockroachDB at port 26257.
	Forward {},
}

impl SubCommand {
	pub async fn execute(self, ctx: ProjectContext) -> Result<()> {
		match self {
			Self::Get { workflow_id } => {
				let workflow = tasks::wf::get_workflow(&ctx, workflow_id).await?;
				tasks::wf::print_workflows(workflow.into_iter().collect(), true).await
			}
			Self::List {
				tags,
				name,
				state,
				pretty,
			} => {
				let workflows = tasks::wf::find_workflows(&ctx, tags, name, state).await?;
				tasks::wf::print_workflows(workflows, pretty).await
			}
			Self::Ack { workflow_id } => tasks::wf::silence_workflow(&ctx, workflow_id).await,
			Self::Wake { workflow_id } => tasks::wf::wake_workflow(&ctx, workflow_id).await,
			Self::History {
				workflow_id,
				include_errors,
				include_forgotten,
				print_location,
			} => {
				tasks::wf::print_history(
					&ctx,
					workflow_id,
					include_errors,
					include_forgotten,
					print_location,
				)
				.await
			}
			Self::Signal { command } => command.execute(ctx).await,
			Self::Forward {} => tasks::wf::forward(&ctx).await,
		}
	}
}
