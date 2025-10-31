use anyhow::{Result, ensure};
use epoxy_protocol::protocol;
use std::cmp;
use universaldb::Transaction;

use crate::replica::{ballot, utils};

#[tracing::instrument(skip_all)]
pub async fn pre_accept(
	tx: &Transaction,
	replica_id: protocol::ReplicaId,
	pre_accept_req: protocol::PreAcceptRequest,
) -> Result<protocol::PreAcceptResponse> {
	tracing::info!(?replica_id, "handling pre-accept message");

	let protocol::Payload {
		proposal,
		seq,
		mut deps,
		instance,
	} = pre_accept_req.payload;

	tracing::debug!("processing PreAccept");

	// Validate ballot
	let current_ballot = ballot::get_ballot(tx, replica_id).await?;
	let validation =
		ballot::validate_and_update_ballot_for_instance(tx, replica_id, &current_ballot, &instance)
			.await?;
	ensure!(
		validation.is_valid,
		"ballot validation failed for pre_accept: incoming ballot {:?} is not greater than stored ballot {:?} for instance {:?} (comparison: {:?})",
		validation.incoming_ballot,
		validation.stored_ballot,
		instance,
		validation.comparison
	);

	// Find interference for this key
	let interf = utils::find_interference(tx, replica_id, &proposal.commands).await?;

	// EPaxos Step 6
	let seq = cmp::max(seq, 1 + utils::find_max_seq(tx, replica_id, &interf).await?);

	// EPaxos Step 7
	if interf != deps {
		deps = utils::union_deps(deps, interf);
	}

	// EPaxos Step 8
	let log_entry = protocol::LogEntry {
		commands: proposal.commands.clone(),
		seq,
		deps: deps.clone(),
		state: protocol::State::PreAccepted,
		ballot: current_ballot,
	};
	crate::replica::update_log(tx, replica_id, log_entry, &instance).await?;

	// EPaxos Step 9
	Ok(protocol::PreAcceptResponse {
		payload: protocol::Payload {
			proposal,
			seq,
			deps,
			instance,
		},
	})
}
