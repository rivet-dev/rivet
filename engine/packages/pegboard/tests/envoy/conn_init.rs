use anyhow::Result;

use crate::common;

#[tokio::test]
async fn envoy_conn_init_writes_hash_ring_entries_with_existing_init_keys() -> Result<()> {
	let test_deps = common::setup_deps().await?;
	let virtual_nodes = 6;
	let fixture = common::write_conn_init_registration(&test_deps, virtual_nodes).await?;

	let state = common::read_key_state(&test_deps, &fixture, virtual_nodes).await?;
	common::assert_registration_keys_present(&state, virtual_nodes as usize);
	assert!(state.virtual_nodes, "VirtualNodesKey should exist");
	assert_eq!(
		common::read_virtual_nodes(&test_deps, &fixture).await?,
		Some(virtual_nodes)
	);

	Ok(())
}
