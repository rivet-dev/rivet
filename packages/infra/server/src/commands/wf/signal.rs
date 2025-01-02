use anyhow::*;
use clap::Parser;
use uuid::Uuid;

use crate::util::{
	self,
	wf::{signal::SignalState, KvPair},
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
	pub async fn execute(self, config: rivet_config::Config) -> Result<()> {
		match self {
			Self::Get { signal_ids } => {
				let pool = rivet_pools::db::crdb::setup(config.clone()).await?;
				let signals = util::wf::signal::get_signals(pool, signal_ids).await?;
				util::wf::signal::print_signals(signals, true).await
			}
			Self::List {
				tags,
				workflow_id,
				name,
				state,
				pretty,
			} => {
				let pool = rivet_pools::db::crdb::setup(config.clone()).await?;
				let signals =
					util::wf::signal::find_signals(pool, tags, workflow_id, name, state).await?;
				util::wf::signal::print_signals(signals, pretty).await
			}
			Self::Ack { signal_ids } => {
				let pool = rivet_pools::db::crdb::setup(config.clone()).await?;
				util::wf::signal::silence_signals(pool, signal_ids).await
			}
		}
	}
}
