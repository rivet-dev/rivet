use epoxy_protocol::protocol;
use universaldb::{FdbBindingError, Transaction};

use crate::replica::{ballot, messages};

pub async fn accept(
	tx: &Transaction,
	replica_id: protocol::ReplicaId,
	accept_req: protocol::AcceptRequest,
) -> Result<protocol::AcceptResponse, FdbBindingError> {
	let protocol::Payload {
		proposal,
		seq,
		deps,
		instance,
	} = accept_req.payload;

	tracing::info!(?replica_id, ?instance, "handling accept message");

	// Validate ballot
	let current_ballot = ballot::get_ballot(tx, replica_id).await?;
	let is_valid =
		ballot::validate_and_update_ballot_for_instance(tx, replica_id, &current_ballot, &instance)
			.await?;
	if !is_valid {
		return Err(FdbBindingError::CustomError(
			anyhow::anyhow!("ballot validation failed for pre_accept").into(),
		));
	}

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
