use anyhow::Result;
use epoxy_protocol::protocol::{self, ReplicaId};
use universaldb::Transaction;
use universaldb::utils::FormalKey;

use crate::{keys, metrics, utils};

pub fn update_config(
	tx: &Transaction,
	replica_id: ReplicaId,
	update_config_req: protocol::UpdateConfigRequest,
) -> Result<()> {
	tracing::debug!("updating config");

	metrics::REPLICAS_TOTAL.reset();
	for replica in &update_config_req.config.replicas {
		metrics::REPLICAS_TOTAL
			.with_label_values(&[match replica.status {
				protocol::ReplicaStatus::Active => "active",
				protocol::ReplicaStatus::Learning => "learning",
				protocol::ReplicaStatus::Joining => "joining",
			}])
			.inc();
	}

	let quorum_members = utils::get_quorum_members(&update_config_req.config);
	let quorum_member_count = quorum_members.len();

	metrics::QUORUM_SIZE
		.with_label_values(&["fast"])
		.set(utils::calculate_quorum(quorum_member_count, utils::QuorumType::Fast) as i64);
	metrics::QUORUM_SIZE
		.with_label_values(&["slow"])
		.set(utils::calculate_quorum(quorum_member_count, utils::QuorumType::Slow) as i64);
	metrics::QUORUM_SIZE
		.with_label_values(&["all"])
		.set(utils::calculate_quorum(quorum_member_count, utils::QuorumType::All) as i64);
	metrics::QUORUM_SIZE
		.with_label_values(&["any"])
		.set(utils::calculate_quorum(quorum_member_count, utils::QuorumType::Any) as i64);

	// Store config in UDB
	let config_key = keys::replica::ConfigKey;
	let subspace = keys::subspace(replica_id);
	let packed_key = subspace.pack(&config_key);
	let value = config_key.serialize(update_config_req.config)?;

	tx.set(&packed_key, &value);

	Ok(())
}
