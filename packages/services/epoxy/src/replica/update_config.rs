use epoxy_protocol::protocol::{self, ReplicaId};
use udb_util::FormalKey;
use universaldb::{FdbBindingError, Transaction};

use crate::keys;

pub fn update_config(
	tx: &Transaction,
	replica_id: ReplicaId,
	update_config_req: protocol::UpdateConfigRequest,
) -> Result<(), FdbBindingError> {
	tracing::debug!("updating config");

	// Store config in UDB
	let config_key = keys::replica::ConfigKey;
	let subspace = keys::subspace(replica_id);
	let packed_key = subspace.pack(&config_key);
	let value = config_key
		.serialize(update_config_req.config)
		.map_err(|e| FdbBindingError::CustomError(e.into()))?;

	tx.set(&packed_key, &value);

	Ok(())
}
