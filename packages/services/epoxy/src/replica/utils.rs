use epoxy_protocol::protocol::{self, ReplicaId};
use futures_util::TryStreamExt;
use std::{cmp::Ordering, collections::HashSet};
use udb_util::prelude::*;
use universaldb::{FdbBindingError, KeySelector, RangeOption, Transaction, options::StreamingMode};

use crate::keys;

// Helper function to find interference for a key
pub async fn find_interference(
	tx: &Transaction,
	replica_id: ReplicaId,
	commands: &Vec<protocol::Command>,
) -> Result<Vec<protocol::Instance>, FdbBindingError> {
	let mut interf = Vec::new();

	// Get deduplicated keys
	//
	// Commands with no keys cause no interference
	let keys = commands
		.iter()
		.filter_map(|x| extract_key_from_command(x))
		.collect::<HashSet<_>>();

	for key in keys {
		// TODO: is there a better way to do this range
		// Query all instances for this key from UDB
		let subspace = keys::subspace(replica_id);
		let prefix = subspace.pack(&keys::replica::KeyInstanceKey::subspace(key.to_owned()));
		let range = RangeOption {
			begin: KeySelector::first_greater_or_equal(prefix.clone()),
			end: KeySelector::first_greater_than(prefix),
			mode: StreamingMode::WantAll,
			..Default::default()
		};

		let mut stream = tx.get_ranges_keyvalues(range, SERIALIZABLE);

		while let Some(kv) = stream.try_next().await? {
			// Parse the key to extract replica_id and slot_id
			let key_bytes = kv.key();
			let key = subspace
				.unpack::<keys::replica::KeyInstanceKey>(key_bytes)
				.map_err(|x| FdbBindingError::CustomError(x.into()))?;

			interf.push(protocol::Instance {
				replica_id: key.instance_replica_id,
				slot_id: key.instance_slot_id,
			});
		}
	}

	interf.sort_by(sort_instances);
	Ok(interf)
}

// Helper function to find max sequence number from interference set
pub async fn find_max_seq(
	tx: &Transaction,
	replica_id: protocol::ReplicaId,
	interf: &Vec<protocol::Instance>,
) -> Result<u64, FdbBindingError> {
	let mut seq = 0;

	for instance in interf {
		// Get the log entry for this instance
		let key = keys::replica::LogEntryKey::new(instance.replica_id, instance.slot_id);
		let subspace = keys::subspace(replica_id);

		let value = tx.get(&subspace.pack(&key), false).await?;
		if let Some(ref bytes) = value {
			let log_entry: protocol::LogEntry = key
				.deserialize(bytes)
				.map_err(|e| FdbBindingError::CustomError(e.into()))?;
			if log_entry.seq > seq {
				seq = log_entry.seq;
			}
		}
	}

	Ok(seq)
}

// Utility function to extract key/value from proposal
pub fn extract_key_from_command(command: &protocol::Command) -> Option<Vec<u8>> {
	match &command.kind {
		protocol::CommandKind::SetCommand(cmd) => Some(cmd.key.clone()),
		protocol::CommandKind::CheckAndSetCommand(cmd) => Some(cmd.key.clone()),
		protocol::CommandKind::NoopCommand => {
			// Noop command has no effects on the KV and does not cause interference
			None
		}
	}
}

pub fn union_deps(
	mut deps1: Vec<protocol::Instance>,
	mut deps2: Vec<protocol::Instance>,
) -> Vec<protocol::Instance> {
	deps1.append(&mut deps2);
	deps1.sort_by(sort_instances);
	deps1.dedup();
	deps1
}

// Sort instances by replica_id then slot_id
fn sort_instances(inst1: &protocol::Instance, inst2: &protocol::Instance) -> Ordering {
	match inst1.replica_id.cmp(&inst2.replica_id) {
		Ordering::Equal => inst1.slot_id.cmp(&inst2.slot_id),
		other => other,
	}
}
