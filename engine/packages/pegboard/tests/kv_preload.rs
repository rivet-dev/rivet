use anyhow::Result;
use gas::prelude::*;
use pegboard::actor_kv as kv;
use rivet_config::config::pegboard::Pegboard;
use rivet_data::converted::ActorNameKeyData;
use serde_json::json;

async fn setup_recipient(
	test_name: &str,
) -> Result<(rivet_test_deps::TestDeps, kv::Recipient, Id)> {
	let test_id = Uuid::new_v4();
	let dc_label = 1;
	let datacenters = [(
		"test-dc".to_string(),
		rivet_config::config::topology::Datacenter {
			name: "test-dc".to_string(),
			datacenter_label: dc_label,
			is_leader: true,
			peer_url: url::Url::parse("http://127.0.0.1:8080")?,
			public_url: url::Url::parse("http://127.0.0.1:8081")?,
			proxy_url: None,
			valid_hosts: None,
		},
	)]
	.into_iter()
	.collect();

	let api_peer_port = portpicker::pick_unused_port().expect("failed to pick api peer port");
	let guard_port = portpicker::pick_unused_port().expect("failed to pick guard port");
	let test_deps = rivet_test_deps::setup_single_datacenter(
		test_id,
		dc_label,
		datacenters,
		api_peer_port,
		guard_port,
	)
	.await?;

	let actor_id = Id::new_v1(dc_label);
	let namespace_id = Id::new_v1(dc_label);
	let recipient = kv::Recipient {
		actor_id,
		namespace_id,
		name: test_name.to_string(),
	};

	Ok((test_deps, recipient, namespace_id))
}

#[tokio::test]
async fn preload_oversized_exact_key_is_not_marked_requested() -> Result<()> {
	let (test_deps, recipient, namespace_id) =
		setup_recipient("preload_oversized_exact_key").await?;
	let db = &test_deps.pools.udb()?;
	let actor_name = "preload-oversized-exact-key";
	let key = b"large-exact-key".to_vec();
	let value = vec![42; 256];

	kv::put(db, &recipient, vec![key.clone()], vec![value]).await?;
	db.txn("test_pegboardkv_preload", |tx| {
		let actor_name = actor_name.to_string();
		let key = key.clone();
		async move {
			let tx = tx.with_subspace(pegboard::keys::subspace());
			tx.write(
				&pegboard::keys::ns::ActorNameKey::new(namespace_id, actor_name),
				ActorNameKeyData {
					metadata: serde_json::Map::from_iter([(
						"preload".to_string(),
						json!({
							"keys": [key],
							"prefixes": [],
						}),
					)]),
				},
			)?;
			Ok(())
		}
	})
	.await?;

	let preloaded = kv::preload::fetch_preloaded_kv(
		db,
		&Pegboard {
			preload_max_total_bytes: Some(1),
			..Default::default()
		},
		recipient.actor_id,
		namespace_id,
		actor_name,
	)
	.await?
	.expect("preload should be enabled by actor metadata");

	assert!(
		preloaded.entries.is_empty(),
		"oversized exact key should not be included"
	);
	assert!(
		preloaded.requested_get_keys.is_empty(),
		"oversized present key must not be marked requested so runtimes live-fetch it"
	);

	Ok(())
}

#[tokio::test]
async fn preload_missing_exact_key_is_marked_requested() -> Result<()> {
	let (test_deps, recipient, namespace_id) = setup_recipient("preload_missing_exact_key").await?;
	let db = &test_deps.pools.udb()?;
	let actor_name = "preload-missing-exact-key";
	let key = b"missing-exact-key".to_vec();

	db.txn("test_pegboardkv_preload", |tx| {
		let actor_name = actor_name.to_string();
		let key = key.clone();
		async move {
			let tx = tx.with_subspace(pegboard::keys::subspace());
			tx.write(
				&pegboard::keys::ns::ActorNameKey::new(namespace_id, actor_name),
				ActorNameKeyData {
					metadata: serde_json::Map::from_iter([(
						"preload".to_string(),
						json!({
							"keys": [key],
							"prefixes": [],
						}),
					)]),
				},
			)?;
			Ok(())
		}
	})
	.await?;

	let preloaded = kv::preload::fetch_preloaded_kv(
		db,
		&Pegboard {
			preload_max_total_bytes: Some(1_024),
			..Default::default()
		},
		recipient.actor_id,
		namespace_id,
		actor_name,
	)
	.await?
	.expect("preload should be enabled by actor metadata");

	assert!(preloaded.entries.is_empty());
	assert_eq!(preloaded.requested_get_keys, vec![key]);

	Ok(())
}
