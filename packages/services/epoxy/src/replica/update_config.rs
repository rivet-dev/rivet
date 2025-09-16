use anyhow::Result;
use epoxy_protocol::protocol::{self, ReplicaId};
use universaldb::Transaction;
use universaldb::utils::FormalKey;

use crate::keys;

pub fn update_config(
	tx: &Transaction,
	replica_id: ReplicaId,
	update_config_req: protocol::UpdateConfigRequest,
) -> Result<()> {
	tracing::debug!("updating config");

	// Store config in UDB
	let config_key = keys::replica::ConfigKey;
	let subspace = keys::subspace(replica_id);
	let packed_key = subspace.pack(&config_key);
	let value = config_key.serialize(update_config_req.config)?;

	tx.set(&packed_key, &value);

	Ok(())
}
