mod common;

use anyhow::Result;
use rivet_util::timestamp;

#[tokio::test]
async fn expire_legacy_envoy_without_virtual_nodes_key_skips_hash_deletes() -> Result<()> {
	let test_deps = common::setup_deps().await?;
	let stale_ping_ts = timestamp::now() - 60_000;
	let fixture = common::write_envoy(&test_deps, stale_ping_ts, None).await?;

	let output = common::expire(&test_deps, &fixture, true).await?;
	assert!(output.did_expire);

	let state = common::read_key_state(&test_deps, &fixture, 8).await?;
	assert!(state.expired_ts, "ExpiredTsKey should be written");
	assert!(
		!state.virtual_nodes,
		"legacy envoy should not have VirtualNodesKey"
	);
	assert!(
		!state.load_balancer_idx,
		"EnvoyLoadBalancerIdxKey should be deleted"
	);
	assert!(!state.active_envoy, "ActiveEnvoyKey should be deleted");
	assert!(
		!state.active_envoy_by_name,
		"ActiveEnvoyByNameKey should be deleted"
	);
	assert_eq!(state.hash_entries, 0);

	Ok(())
}
