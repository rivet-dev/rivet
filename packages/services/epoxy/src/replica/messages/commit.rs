use epoxy_protocol::protocol;
use universaldb::{FdbBindingError, Transaction};

use crate::replica::ballot;

// EPaxos Step 24
pub async fn commit(
	tx: &Transaction,
	replica_id: protocol::ReplicaId,
	commit_req: protocol::CommitRequest,
	commit_to_kv: bool,
) -> Result<(), FdbBindingError> {
	let protocol::Payload {
		proposal,
		seq,
		deps,
		instance,
	} = commit_req.payload;

	tracing::info!(?replica_id, ?instance, "handling commit message");

	// EPaxos Step 24
	let current_ballot = ballot::get_ballot(tx, replica_id).await?;
	let log_entry = protocol::LogEntry {
		commands: proposal.commands.clone(),
		seq,
		deps,
		state: protocol::State::Committed,
		ballot: current_ballot,
	};
	crate::replica::update_log(tx, replica_id, log_entry, &instance).await?;

	// Commit commands if requested
	let cmd_err = if commit_to_kv {
		crate::replica::commit_kv::commit_kv(&*tx, replica_id, &proposal.commands).await?
	} else {
		tracing::debug!(?replica_id, ?instance, "skipping kv commit");
		None
	};

	tracing::debug!(?replica_id, ?instance, ?cmd_err, "committed");

	Ok(())
}
