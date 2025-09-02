use epoxy_protocol::protocol;
use udb_util::FormalKey as _;
use universaldb::{FdbBindingError, Transaction};

use crate::keys;
use crate::replica::{ballot, messages, utils};

pub async fn lead_consensus(
	tx: &Transaction,
	replica_id: protocol::ReplicaId,
	proposal: protocol::Proposal,
) -> Result<protocol::Payload, FdbBindingError> {
	tracing::info!(?replica_id, "leading consensus");

	// EPaxos Step 1
	let instance_num_key = keys::replica::InstanceNumberKey;
	let packed_key = keys::subspace(replica_id).pack(&instance_num_key);

	let value = tx.get(&packed_key, false).await?;
	let current_slot = if let Some(ref bytes) = value {
		let current = instance_num_key
			.deserialize(bytes)
			.map_err(|e| FdbBindingError::CustomError(e.into()))?;
		current
	} else {
		0
	};

	// Increment and store the new instance number
	let slot_id = current_slot + 1;
	tx.set(
		&packed_key,
		&instance_num_key
			.serialize(slot_id)
			.map_err(|e| FdbBindingError::CustomError(e.into()))?,
	);

	// Find interference for this key
	let interf = utils::find_interference(tx, replica_id, &proposal.commands).await?;

	// EPaxos Step 2
	let seq = 1 + utils::find_max_seq(tx, replica_id, &interf).await?;

	// EPaxos Step 3: deps are set to interf below

	// EPaxos Step 4
	let current_ballot = ballot::increment_ballot(tx, replica_id).await?;
	let log_entry = protocol::LogEntry {
		commands: proposal.commands.clone(),
		seq,
		deps: interf.clone(),
		state: protocol::State::PreAccepted,
		ballot: current_ballot,
	};

	// Update log
	let instance = protocol::Instance {
		replica_id,
		slot_id,
	};
	crate::replica::update_log(tx, replica_id, log_entry, &instance).await?;

	// Return payload
	Ok(protocol::Payload {
		proposal,
		seq,
		deps: interf,
		instance,
	})
}
