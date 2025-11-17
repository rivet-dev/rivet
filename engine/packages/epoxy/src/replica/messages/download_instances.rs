use anyhow::Result;
use epoxy_protocol::protocol::{self, ReplicaId};
use futures_util::TryStreamExt;
use universaldb::prelude::*;
use universaldb::{KeySelector, RangeOption, Transaction, options::StreamingMode};

use crate::keys;

#[tracing::instrument(skip_all)]
pub async fn download_instances(
	tx: &Transaction,
	replica_id: ReplicaId,
	req: protocol::DownloadInstancesRequest,
) -> Result<Vec<protocol::DownloadInstancesEntry>> {
	tracing::debug!(?replica_id, "handling download instances message");

	let mut entries = Vec::new();
	let subspace = keys::subspace(replica_id);

	// Build the range query for log entries
	// We need to scan the log/{replica}/{slot}/entry keyspace
	let begin_key = if let Some(after) = &req.after_instance {
		// Start after the specified instance
		let after_key = keys::replica::LogEntryKey::new(after.replica_id, after.slot_id);
		let packed = subspace.pack(&after_key);
		KeySelector::first_greater_than(packed)
	} else {
		// TODO: Use ::subspace()
		// Start from the beginning of the log
		let prefix = subspace.pack(&(universaldb::utils::keys::LOG,));
		KeySelector::first_greater_or_equal(prefix)
	};

	// TODO: Is there a cleaner way to do this
	// End key is after all log entries
	let end_prefix = subspace.pack(&(universaldb::utils::keys::LOG + 1,));
	let end_key = KeySelector::first_greater_or_equal(end_prefix);

	let range = RangeOption {
		begin: begin_key,
		end: end_key,
		mode: StreamingMode::Exact,
		limit: Some(req.count as usize),
		..Default::default()
	};

	// Query the range
	let mut stream = tx.get_ranges_keyvalues(range, Serializable);

	while let Some(kv) = stream.try_next().await? {
		// Parse the key to extract instance info
		let key_bytes = kv.key();
		let log_key = subspace.unpack::<keys::replica::LogEntryKey>(key_bytes)?;
		// Deserialize the log entry
		let log_entry = log_key.deserialize(kv.value())?;
		// Create the instance from the key
		let instance = protocol::Instance {
			replica_id: log_key.instance_replica_id,
			slot_id: log_key.instance_slot_id,
		};

		entries.push(protocol::DownloadInstancesEntry {
			instance,
			log_entry,
		});
	}

	Ok(entries)
}
