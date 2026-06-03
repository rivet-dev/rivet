use anyhow::Result;

use crate::common;

#[tokio::test]
async fn expire_removes_hash_entries_and_per_envoy_indexes() -> Result<()> {
	let test_deps = common::setup_deps().await?;
	let virtual_nodes = 5;
	let fixture =
		common::write_envoy(&test_deps, common::stale_ping_ts(), Some(virtual_nodes)).await?;

	let output = common::expire(&test_deps, &fixture, false).await?;
	assert!(output.did_expire);

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
