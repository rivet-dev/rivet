mod common;

use anyhow::Result;

#[tokio::test]
async fn config_rejects_oversize_hash_knobs() -> Result<()> {
	let err = common::load_hash_config("samples: 9")
		.await
		.err()
		.expect("samples=9 should fail config loading");
	assert!(err.to_string().contains("samples must be in 1..=8"));

	let err = common::load_hash_config("virtual_nodes: 65")
		.await
		.err()
		.expect("virtual_nodes=65 should fail config loading");
	assert!(err.to_string().contains("virtual_nodes must be in 1..=64"));

	let err = common::load_hash_config("max_scan: 257")
		.await
		.err()
		.expect("max_scan=257 should fail config loading");
	assert!(err.to_string().contains("max_scan must be in 1..=256"));

	Ok(())
}
