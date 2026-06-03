mod common;

use anyhow::Result;
use rivet_util::timestamp;

#[tokio::test]
async fn skip_if_fresh_bails_when_envoy_is_already_expired() -> Result<()> {
	let test_deps = common::setup_deps().await?;
	let stale_ping_ts = timestamp::now() - 60_000;
	let fixture = common::write_envoy(&test_deps, stale_ping_ts, Some(8)).await?;
	common::mark_expired(&test_deps, &fixture).await?;

	let output = common::expire(&test_deps, &fixture, true).await?;
	assert!(!output.did_expire);

	let state = common::read_key_state(&test_deps, &fixture, 8).await?;
	common::assert_registration_keys_present(&state, 8);
	assert!(state.expired_ts, "existing ExpiredTsKey should remain");
	assert!(
		state.virtual_nodes,
		"VirtualNodesKey should not be deleted twice"
	);

	Ok(())
}
