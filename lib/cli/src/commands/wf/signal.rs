use anyhow::*;
use clap::Parser;
use uuid::Uuid;

use crate::util::{
	self,
	wf::{signal::SignalState, KvPair},
};

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
	pub async fn execute(self) -> Result<()> {
		match self {
			Self::Get { signal_id } => {
				let signal = util::wf::signal::get_signal(signal_id).await?;
				util::wf::signal::print_signals(signal.into_iter().collect(), true).await
			}
			Self::List {
				tags,
				workflow_id,
				name,
				state,
				pretty,
			} => {
				let signals =
					util::wf::signal::find_signals(tags, workflow_id, name, state).await?;
				util::wf::signal::print_signals(signals, pretty).await
			}
			Self::Ack { signal_id } => util::wf::signal::silence_signal(signal_id).await,
		}
	}
}
