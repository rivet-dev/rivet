use anyhow::Result;
use gas::prelude::*;
use pegboard::{envoy_expire_scheduler, metrics};

use crate::common;

#[tokio::test]
async fn stale_envoy_seen_by_hash_read_path_is_expired_by_scheduler() -> Result<()> {
	let test_deps = common::setup_deps().await?;
	let namespace_id = Id::new_v1(test_deps.config().dc_label());
	let namespace_id_label = namespace_id.to_string();
	let pool_name = common::unique_pool_name("envoy-read-path-expire");
	let virtual_nodes = 1;
	let stale_envoy_key = "stale-envoy";

	common::write_hash_envoy(
		&test_deps,
		namespace_id,
		&pool_name,
		stale_envoy_key,
		common::stale_ping_ts(),
		vec![common::hash_pos(0x10)],
		Some(0),
	)
	.await?;
	common::write_hash_envoy(
		&test_deps,
		namespace_id,
		&pool_name,
		"fresh-envoy",
		common::fresh_ping_ts(),
		vec![common::hash_pos(0x20)],
		Some(0),
	)
	.await?;

	let before = metrics::ENVOY_EXPIRE_SCHEDULER_COMPLETED_TOTAL
		.with_label_values(&[namespace_id_label.as_str(), "expired"])
		.get();
	let (allocation, _) = common::allocate_hash(
		&test_deps,
		namespace_id,
		&pool_name,
		1,
		8,
		vec![common::hash_pos(0x10)],
		0,
	)
	.await?;
	assert_eq!(allocation.as_deref(), Some("fresh-envoy"));

	envoy_expire_scheduler::get(test_deps.pools())
		.wait_pending_empty()
		.await;
	let after = metrics::ENVOY_EXPIRE_SCHEDULER_COMPLETED_TOTAL
		.with_label_values(&[namespace_id_label.as_str(), "expired"])
		.get();
	assert_eq!(after - before, 1);

	let fixture = common::EnvoyFixture {
		namespace_id,
		envoy_key: stale_envoy_key.to_string(),
		pool_name,
		version: common::VERSION,
		create_ts: common::HASH_NOW,
		last_ping_ts: common::stale_ping_ts(),
		virtual_nodes: Some(virtual_nodes),
	};
	let state = common::read_key_state(&test_deps, &fixture, virtual_nodes).await?;
	assert_eq!(state.hash_entries, 0);
	assert!(!state.virtual_nodes, "VirtualNodesKey should be deleted");
	assert!(
		!state.load_balancer_idx,
		"EnvoyLoadBalancerIdxKey should be deleted"
	);
	assert!(!state.active_envoy, "ActiveEnvoyKey should be deleted");
	assert!(
		!state.active_envoy_by_name,
		"ActiveEnvoyByNameKey should be deleted"
	);
	assert!(state.expired_ts, "ExpiredTsKey should be written");

	Ok(())
}
