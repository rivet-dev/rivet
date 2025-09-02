use epoxy_protocol::protocol;
use udb_util::FormalKey;
use universaldb::{FdbBindingError, Transaction};

use crate::{keys, replica::ballot};

pub async fn prepare(
	tx: &Transaction,
	replica_id: protocol::ReplicaId,
	prepare_req: protocol::PrepareRequest,
) -> Result<protocol::PrepareResponse, FdbBindingError> {
	tracing::info!(?replica_id, "handling prepare message");

	let protocol::PrepareRequest { ballot, instance } = prepare_req;

	tracing::debug!(?ballot, ?instance, "processing prepare request");

	// Look up the current log entry for this instance
	let subspace = keys::subspace(replica_id);
	let log_key = keys::replica::LogEntryKey::new(instance.replica_id, instance.slot_id);

	let current_entry = match tx.get(&subspace.pack(&log_key), false).await? {
		Some(bytes) => {
			// Deserialize the existing log entry
			Some(
				log_key
					.deserialize(&bytes)
					.map_err(|e| FdbBindingError::CustomError(e.into()))?,
			)
		}
		None => None,
	};

	// EPaxos Step 38: Validate ballot for this instance
	let is_valid =
		ballot::validate_and_update_ballot_for_instance(tx, replica_id, &ballot, &instance).await?;

	let response = if is_valid {
		// EPaxos Step 39: Reply PrepareOK with current log entry data
		match current_entry {
			Some(entry) => protocol::PrepareResponse::PrepareOk(protocol::PrepareOk {
				data: Some(protocol::PrepareResponseData {
					commands: entry.commands,
					seq: entry.seq,
					deps: entry.deps,
					state: entry.state,
					ballot: entry.ballot.clone(),
				}),
				previous_ballot: entry.ballot,
				instance,
			}),
			None => {
				// No existing entry
				let default_ballot = protocol::Ballot {
					epoch: 0,
					ballot: 0,
					replica_id: instance.replica_id,
				};

				protocol::PrepareResponse::PrepareOk(protocol::PrepareOk {
					data: None,
					previous_ballot: default_ballot,
					instance,
				})
			}
		}
	} else {
		// EPaxos Step 40: Ballot validation failed
		// EPaxos Step 41: Reply NACK with highest ballot stored for this instance
		let instance_ballot_key =
			keys::replica::InstanceBallotKey::new(instance.replica_id, instance.slot_id);
		let subspace = keys::subspace(replica_id);
		let packed_key = subspace.pack(&instance_ballot_key);

		let highest_ballot = match tx.get(&packed_key, false).await? {
			Some(bytes) => instance_ballot_key
				.deserialize(&bytes)
				.map_err(|e| FdbBindingError::CustomError(e.into()))?,
			None => {
				// Default ballot for the original replica
				protocol::Ballot {
					epoch: 0,
					ballot: 0,
					replica_id: instance.replica_id,
				}
			}
		};

		protocol::PrepareResponse::PrepareNack(protocol::PrepareNack { highest_ballot })
	};

	tracing::debug!(?response, "prepare response generated");
	Ok(response)
}
