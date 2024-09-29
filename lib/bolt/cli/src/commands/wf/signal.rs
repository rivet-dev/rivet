use anyhow::*;
use bolt_core::{
	context::ProjectContext,
	tasks::{
		self,
		wf::{signal::SignalState, KvPair},
	},
};
use clap::Parser;
use uuid::Uuid;

#[derive(Parser)]
pub enum SubCommand {
	/// Prints the given signal.
	Get {
		#[clap(index = 1)]
		signal_id: Uuid,
	},
	/// Finds signals that match the given tags.
	List {
		tags: Vec<KvPair>,
		#[clap(long, short = 'w')]
		workflow_id: Option<Uuid>,
		/// Signal name.
		#[clap(long, short = 'n')]
		name: Option<String>,
		#[clap(long, short = 's')]
		state: Option<SignalState>,
		/// Prints paragraphs instead of a table.
		#[clap(long, short = 'p')]
		pretty: bool,
	},
	/// Silences a signal from showing up as dead or running again.
	Ack {
		#[clap(index = 1)]
		signal_id: Uuid,
	},
}

impl SubCommand {
	pub async fn execute(self, ctx: ProjectContext) -> Result<()> {
		match self {
			Self::Get { signal_id } => {
				let signal = tasks::wf::signal::get_signal(&ctx, signal_id).await?;
				tasks::wf::signal::print_signals(signal.into_iter().collect(), true).await
			}
			Self::List {
				tags,
				workflow_id,
				name,
				state,
				pretty,
			} => {
				let signals =
					tasks::wf::signal::find_signals(&ctx, tags, workflow_id, name, state).await?;
				tasks::wf::signal::print_signals(signals, pretty).await
			}
			Self::Ack { signal_id } => tasks::wf::signal::silence_signal(&ctx, signal_id).await,
		}
	}
}
