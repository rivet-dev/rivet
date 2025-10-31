use anyhow::{Result, ensure};
use epoxy_protocol::protocol;
use universaldb::Transaction;

use crate::replica::ballot;

#[tracing::instrument(skip_all)]
pub async fn accept(
	tx: &Transaction,
	replica_id: protocol::ReplicaId,
	accept_req: protocol::AcceptRequest,
) -> Result<protocol::AcceptResponse> {
	let protocol::Payload {
		proposal,
		seq,
		deps,
		instance,
	} = accept_req.payload;

	tracing::info!(?replica_id, ?instance, "handling accept message");

	// Validate ballot
	let current_ballot = ballot::get_ballot(tx, replica_id).await?;
	let validation =
		ballot::validate_and_update_ballot_for_instance(tx, replica_id, &current_ballot, &instance)
			.await?;
	ensure!(
		validation.is_valid,
		"ballot validation failed for accept: incoming ballot {:?} is not greater than stored ballot {:?} for instance {:?} (comparison: {:?})",
		validation.incoming_ballot,
		validation.stored_ballot,
		instance,
		validation.comparison
	);

	// EPaxos Step 18
	let log_entry = protocol::LogEntry {
		commands: proposal.commands.clone(),
		seq,
		deps,
		state: protocol::State::Accepted,
		ballot: current_ballot,
	};
	crate::replica::update_log(tx, replica_id, log_entry, &instance).await?;

	// EPaxos Step 19
	Ok(protocol::AcceptResponse {
		payload: protocol::AcceptOKPayload { proposal, instance },
	})
}
