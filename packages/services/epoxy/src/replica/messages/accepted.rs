use epoxy_protocol::protocol;
use universaldb::{FdbBindingError, Transaction};

use crate::replica::{ballot, messages, utils};

// EPaxos Step 16
pub async fn accepted(
	tx: &Transaction,
	replica_id: protocol::ReplicaId,
	payload: protocol::Payload,
) -> Result<(), FdbBindingError> {
	let protocol::Payload {
		proposal,
		seq,
		deps,
		instance,
	} = payload;

	tracing::info!(?replica_id, ?instance, "handling accepted message");

	// Create accepted log entry
	let current_ballot = ballot::get_ballot(tx, replica_id).await?;
	let log_entry = protocol::LogEntry {
		commands: proposal.commands.clone(),
		seq,
		deps,
		state: protocol::State::Accepted,
		ballot: current_ballot,
	};
	crate::replica::update_log(tx, replica_id, log_entry, &instance).await?;

	Ok(())
}
