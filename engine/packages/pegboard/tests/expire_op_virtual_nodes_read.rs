mod common;

use anyhow::Result;
use rivet_config::config::{
	Root,
	pegboard::{EnvoyLoadBalancer, Pegboard},
};
use rivet_util::timestamp;

#[tokio::test]
async fn expire_deletes_hash_positions_from_persisted_virtual_node_count() -> Result<()> {
	let test_deps = common::setup_deps().await?;
	let stale_ping_ts = timestamp::now() - 60_000;
	let fixture = common::write_envoy(&test_deps, stale_ping_ts, Some(16)).await?;

	let mut root = Root::default();
	root.pegboard = Some(Pegboard {
		envoy_load_balancer: Some(EnvoyLoadBalancer::Hash {
			virtual_nodes: 4,
			samples: 2,
			max_scan: 16,
			slot_jitter: 0,
			use_snapshot_read: true,
		}),
		..Default::default()
	});
	let config = rivet_config::Config::from_root(root);

	let output = pegboard::ops::envoy::expire::expire_with_pools(
		&config,
		test_deps.pools(),
		&pegboard::ops::envoy::expire::Input {
			namespace_id: fixture.namespace_id,
			envoy_key: fixture.envoy_key.clone(),
			skip_if_fresh: true,
		},
	)
	.await?;
	assert!(output.did_expire);

	let state = common::read_key_state(&test_deps, &fixture, 16).await?;
	assert_eq!(state.hash_entries, 0);
	assert!(!state.virtual_nodes, "VirtualNodesKey should be deleted");

	Ok(())
}
