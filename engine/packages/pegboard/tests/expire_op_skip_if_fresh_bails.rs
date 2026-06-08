mod common;

use anyhow::Result;
use rivet_util::timestamp;

#[tokio::test]
async fn skip_if_fresh_bails_without_deleting_keys() -> Result<()> {
	let test_deps = common::setup_deps().await?;
	let fixture = common::write_envoy(&test_deps, timestamp::now(), Some(8)).await?;

	let output = common::expire(&test_deps, &fixture, true).await?;
	assert!(!output.did_expire);

	let state = common::read_key_state(&test_deps, &fixture, 8).await?;
	common::assert_registration_keys_present(&state, 8);
	assert!(!state.expired_ts, "ExpiredTsKey should not be written");
	assert!(state.virtual_nodes, "VirtualNodesKey should remain");

	Ok(())
}
