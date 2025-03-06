use std::sync::Arc;

use anyhow::*;
use clap::{ValueEnum, Parser};
use uuid::Uuid;
use chirp_workflow::db::{self, Database, debug::{DatabaseDebug, SignalState as OtherSignalState}};

use crate::util::{
	self,
	wf::KvPair,
};

#[derive(Parser)]
pub enum SubCommand {
	/// Prints the given signal(s).
	Get { signal_ids: Vec<Uuid> },
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
	Ack { signal_ids: Vec<Uuid> },
}

impl SubCommand {
	pub async fn execute(self, db: Arc<dyn DatabaseDebug>) -> Result<()> {		
		match self {
			Self::Get { signal_ids } => {
				let signals = db.get_signals(signal_ids).await?;
				util::wf::signal::print_signals(signals, true).await
			}
			Self::List {
				tags,
				workflow_id,
				name,
				state,
				pretty,
			} => {
				let signals =
					db.find_signals(tags.into_iter().map(|kv| (kv.key, kv.value)).collect(), workflow_id, name, state.map(Into::into)).await?;
				util::wf::signal::print_signals(signals, pretty).await
			}
			Self::Ack { signal_ids } => {
				db.silence_signals(signal_ids).await
			}
		}
	}
}


#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[clap(rename_all = "kebab_case")]
pub enum SignalState {
	Acked,
	Pending,
}

impl From<SignalState> for OtherSignalState {
	fn from(state: SignalState) -> Self {
		match state {
			SignalState::Acked => OtherSignalState::Acked,
			SignalState::Pending => OtherSignalState::Pending,
		}
	}
}
