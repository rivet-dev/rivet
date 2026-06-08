use anyhow::Result;

use crate::common;

#[tokio::test]
async fn envoy_read_path_expire_and_graceful_expire_are_idempotent_when_racing() -> Result<()> {
	let test_deps = common::setup_deps().await?;
	let virtual_nodes = 4;
	let fixture =
		common::write_envoy(&test_deps, common::stale_ping_ts(), Some(virtual_nodes)).await?;

	let graceful = common::expire(&test_deps, &fixture, false);
	let read_path = common::expire(&test_deps, &fixture, true);
	let (graceful, read_path) = tokio::try_join!(graceful, read_path)?;

	assert_eq!(
		usize::from(graceful.did_expire) + usize::from(read_path.did_expire),
		1
	);

	let state = common::read_key_state(&test_deps, &fixture, virtual_nodes).await?;
	assert!(
		state.expired_ts,
		"ExpiredTsKey should be visible after both observers finish"
	);
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

	Ok(())
}
