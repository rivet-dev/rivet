use anyhow::*;
use epoxy_protocol::protocol::{self, ReplicaId};
use udb_util::prelude::*;
use universaldb::{FdbBindingError, Transaction};

use crate::{keys, ops::propose::CommandError, replica::utils};

/// Commits a proposal to KV store.
pub async fn commit_kv(
	tx: &Transaction,
	replica_id: ReplicaId,
	commands: &[protocol::Command],
) -> Result<Option<CommandError>, FdbBindingError> {
	let subspace = keys::subspace(replica_id);

	for command in commands.iter() {
		// Validate command logic
		match &command.kind {
			protocol::CommandKind::SetCommand(_) => {
				// Always succeeds
			}
			protocol::CommandKind::CheckAndSetCommand(cmd) => {
				// Read current value
				let kv_key = keys::keys::KvValueKey::new(cmd.key.clone());
				let packed_key = subspace.pack(&kv_key);
				let current_value = if let Some(bytes) = tx.get(&packed_key, false).await? {
					Some(
						kv_key
							.deserialize(&bytes)
							.map_err(|x| FdbBindingError::CustomError(x.into()))?,
					)
				} else {
					None
				};

				// Validate CAS state
				if !cmd.expect_one_of.iter().any(|x| *x == current_value) {
					return Result::Ok(Some(CommandError::ExpectedValueDoesNotMatch {
						current_value,
					}));
				}
			}
			protocol::CommandKind::NoopCommand => {
				// No-op command does nothing
			}
		};
	}

	// Apply commands
	for command in commands.iter() {
		let Some(key) = utils::extract_key_from_command(&command) else {
			continue;
		};

		// Build key
		let kv_key = keys::keys::KvValueKey::new(key.clone());
		let packed_key = subspace.pack(&kv_key);

		// Read value
		let new_value = match &command.kind {
			protocol::CommandKind::SetCommand(cmd) => &cmd.value,
			protocol::CommandKind::CheckAndSetCommand(cmd) => &cmd.new_value,
			protocol::CommandKind::NoopCommand => {
				continue;
			}
		};

		// Update the value
		if let Some(value) = new_value {
			let serialized = kv_key
				.serialize(value.clone())
				.map_err(|x| FdbBindingError::CustomError(x.into()))?;
			tx.set(&packed_key, &serialized);
		} else {
			tx.clear(&packed_key);
		}

		// Clear cached key, since we have the committed value for sure
		let cache_key = keys::keys::KvOptimisticCacheKey::new(key.clone());
		let cache_packed_key = subspace.pack(&cache_key);
		tx.clear(&cache_packed_key);
	}

	Result::Ok(None)
}
