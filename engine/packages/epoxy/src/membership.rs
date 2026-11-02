//! The Epoxy replicas this process can reach.
//!
//! Membership is fixed to this datacenter's replica (see `depot::replica::local_membership`), so
//! nothing installs or stores it. The configured topology only names peer addresses for
//! protocol version discovery.

use anyhow::Result;
use rivet_data::generated::epoxy_control_state_v1;

/// Global cross-region Epoxy replicas, independent of Depot workers and tablet placement.
pub fn epoxy_topology(
	config: &rivet_config::Config,
) -> Result<epoxy_control_state_v1::EpoxyTopology> {
	let mut replicas: Vec<_> = config
		.topology()
		.datacenters
		.iter()
		.map(|dc| epoxy_control_state_v1::TopologyReplica {
			replica_id: dc.datacenter_label as u64,
			api_peer_url: dc.peer_url.to_string().into(),
			guard_url: dc.public_url.to_string().into(),
		})
		.collect();
	replicas.sort_by_key(|r| r.replica_id);
	Ok(epoxy_control_state_v1::EpoxyTopology {
		initial_coordinator_replica_id: config.topology().leader_dc()?.datacenter_label as u64,
		replicas,
	})
}
