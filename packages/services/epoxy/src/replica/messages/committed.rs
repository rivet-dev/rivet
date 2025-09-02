use epoxy_protocol::protocol;
use universaldb::{FdbBindingError, Transaction};

use crate::replica::ballot;

// EPaxos Steps 21-22
pub async fn committed(
	tx: &Transaction,
	replica_id: protocol::ReplicaId,
	payload: &protocol::Payload,
) -> Result<Option<crate::ops::propose::CommandError>, FdbBindingError> {
	let protocol::Payload {
		proposal,
		seq,
		deps,
		instance,
	} = payload;

	tracing::info!(?replica_id, ?instance, "handling committed message");

	// EPaxos Step 21: Create committed log entry
	let current_ballot = ballot::get_ballot(tx, replica_id).await?;
	let log_entry = protocol::LogEntry {
		commands: proposal.commands.clone(),
		seq: *seq,
		deps: deps.clone(),
		state: protocol::State::Committed,
		ballot: current_ballot,
	};
	crate::replica::update_log(tx, replica_id, log_entry, &instance).await?;

	// Commit commands
	let cmd_err =
		crate::replica::commit_kv::commit_kv(&*tx, replica_id, &payload.proposal.commands).await?;

	tracing::debug!(?replica_id, ?instance, ?cmd_err, "committed");

	Ok(cmd_err)
}
