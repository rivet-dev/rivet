use epoxy_protocol::protocol::{self, ReplicaId};
use udb_util::FormalKey;
use universaldb::{FdbBindingError, Transaction};

use crate::{keys, replica::utils};

/// Returns the numeric order of a state for comparison.
/// NONE < PREACCEPTED < ACCEPTED < COMMITTED
pub fn state_order(state: &protocol::State) -> u8 {
	match state {
		protocol::State::PreAccepted => 1,
		protocol::State::Accepted => 2,
		protocol::State::Committed => 3,
	}
}

pub async fn update_log(
	tx: &Transaction,
	replica_id: ReplicaId,
	log_entry: protocol::LogEntry,
	instance: &protocol::Instance,
) -> Result<(), FdbBindingError> {
	tracing::debug!(?replica_id, ?instance, ?log_entry.state, "updating log");

	let subspace = keys::subspace(replica_id);
	let log_key = keys::replica::LogEntryKey::new(instance.replica_id, instance.slot_id);
	let packed_key = subspace.pack(&log_key);

	// Read existing log entry to validate state progression
	let current_entry = match tx.get(&packed_key, false).await? {
		Some(bytes) => Some(
			log_key
				.deserialize(&bytes)
				.map_err(|e| FdbBindingError::CustomError(e.into()))?,
		),
		None => None,
	};

	// Validate that new state is higher than current state
	if let Some(current) = &current_entry {
		let current_order = state_order(&current.state);
		let new_order = state_order(&log_entry.state);

		if new_order <= current_order {
			return Err(FdbBindingError::CustomError(
				anyhow::anyhow!(
					"invalid state transition: cannot transition from {:?} to {:?} (order {} to {})",
					current.state,
					log_entry.state,
					current_order,
					new_order
				)
				.into(),
			));
		}

		tracing::debug!(
			?current.state,
			?log_entry.state,
			current_order,
			new_order,
			"validated state progression"
		);
	} else {
		tracing::debug!(?log_entry.state, "creating new log entry");
	}

	// Store log entry in UDB
	let value = log_key
		.serialize(log_entry.clone())
		.map_err(|e| FdbBindingError::CustomError(e.into()))?;
	tx.set(&packed_key, &value);

	// Store in keys for interference
	let mut written_keys = 0;
	for command in log_entry.commands {
		let Some(key) = utils::extract_key_from_command(&command) else {
			continue;
		};

		written_keys += 1;

		let instance_key =
			keys::replica::KeyInstanceKey::new(key.clone(), instance.replica_id, instance.slot_id);
		let instance_packed_key = subspace.pack(&instance_key);
		tx.set(&instance_packed_key, &[]);
	}

	tracing::debug!(?replica_id, ?instance, ?log_entry.state, ?written_keys, "wrote log entry");

	Ok(())
}
