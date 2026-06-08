mod common;

use anyhow::Result;
use rivet_util::timestamp;

#[tokio::test]
async fn skip_if_fresh_expires_stale_envoy_and_deletes_per_envoy_indexes() -> Result<()> {
	let test_deps = common::setup_deps().await?;
	let stale_ping_ts = timestamp::now() - 60_000;
	let fixture = common::write_envoy(&test_deps, stale_ping_ts, Some(8)).await?;

	let output = common::expire(&test_deps, &fixture, true).await?;
	assert!(output.did_expire);

	let state = common::read_key_state(&test_deps, &fixture, 8).await?;
	assert!(
		state.pool_name,
		"PoolNameKey should remain as envoy metadata"
	);
	assert!(state.version, "VersionKey should remain as envoy metadata");
	assert!(
		state.create_ts,
		"CreateTsKey should remain as envoy metadata"
	);
	assert!(
		state.last_ping_ts,
		"LastPingTsKey should remain as envoy metadata"
	);
	assert!(state.expired_ts, "ExpiredTsKey should be written");
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
	assert_eq!(state.hash_entries, 0);

	Ok(())
}
