use anyhow::Result;
use gas::prelude::*;
use pegboard::keys;
use rivet_data::converted::ActorByKeyKeyData;

/// Must stay above `MAX_ACTOR_BY_KEY_SCAN_ENTRIES` in
/// `pegboard::ops::actor::list_for_ns`.
const OVER_CAP_ENTRIES: usize = 4100;
const UNDER_CAP_ENTRIES: usize = 64;
const WRITE_BATCH: usize = 256;

async fn build_ctx(test_name: &'static str) -> Result<StandaloneCtx> {
	let test_deps = rivet_test_deps::TestDeps::new().await?;
	let cache = rivet_cache::CacheInner::from_env(&test_deps.config, test_deps.pools.clone())?;

	Ok(StandaloneCtx::new(
		db::DatabaseKv::new(test_deps.config.clone(), test_deps.pools.clone()).await?,
		test_deps.config.clone(),
		test_deps.pools.clone(),
		cache,
		test_name,
		Id::new_v1(test_deps.config.dc_label()),
		Id::new_v1(test_deps.config.dc_label()),
	)?)
}

/// Writes `count` destroyed index entries under one key, oldest first.
async fn write_destroyed_entries(
	ctx: &StandaloneCtx,
	namespace_id: Id,
	name: &str,
	key: &str,
	count: usize,
) -> Result<()> {
	let dc_label = ctx.config().dc_label();

	for batch_start in (0..count).step_by(WRITE_BATCH) {
		let batch_end = (batch_start + WRITE_BATCH).min(count);

		ctx.udb()?
			.txn("test_actor_list_for_ns_seed", |tx| async move {
				let tx = tx.with_subspace(keys::subspace());

				for i in batch_start..batch_end {
					tx.write(
						&keys::ns::ActorByKeyKey::new(
							namespace_id,
							name.to_string(),
							key.to_string(),
							1_000_000 + i as i64,
							Id::new_v1(dc_label),
						),
						ActorByKeyKeyData {
							workflow_id: Id::new_v1(dc_label),
							is_destroyed: true,
						},
					)?;
				}

				Ok(())
			})
			.await?;
	}

	Ok(())
}

/// A key whose index has grown past the scan cap must surface an explicit error. Returning an empty
/// list instead would tell `get_or_create` that no actor exists, and the replacement it creates
/// writes yet another entry under the same key.
#[tokio::test]
async fn actor_list_for_ns_by_key_scan_cap_errors_over_cap() -> Result<()> {
	let ctx = build_ctx("test_actor_list_for_ns_by_key_scan_cap_errors_over_cap").await?;

	let namespace_id = Id::new_v1(ctx.config().dc_label());
	let name = "test-actor";
	let key = "over-cap";

	write_destroyed_entries(&ctx, namespace_id, name, key, OVER_CAP_ENTRIES).await?;

	let err = ctx
		.op(pegboard::ops::actor::list_for_ns::Input {
			namespace_id,
			name: name.to_string(),
			key: Some(key.to_string()),
			include_destroyed: false,
			created_before: None,
			limit: 1,
			fetch_error: false,
		})
		.await
		.expect_err("scan cap should surface an error rather than an empty list");

	let rivet_err = rivet_error::RivetError::extract(&err);
	assert_eq!(rivet_err.group(), "actor");
	assert_eq!(rivet_err.code(), "key_index_scan_limit_exceeded");

	Ok(())
}

/// A key with fewer entries than the cap must keep resolving normally.
#[tokio::test]
async fn actor_list_for_ns_by_key_scan_cap_allows_under_cap() -> Result<()> {
	let ctx = build_ctx("test_actor_list_for_ns_by_key_scan_cap_allows_under_cap").await?;

	let namespace_id = Id::new_v1(ctx.config().dc_label());
	let name = "test-actor";
	let key = "under-cap";

	write_destroyed_entries(&ctx, namespace_id, name, key, UNDER_CAP_ENTRIES).await?;

	let res = ctx
		.op(pegboard::ops::actor::list_for_ns::Input {
			namespace_id,
			name: name.to_string(),
			key: Some(key.to_string()),
			include_destroyed: false,
			created_before: None,
			limit: 1,
			fetch_error: false,
		})
		.await?;

	assert!(res.actors.is_empty());

	Ok(())
}
