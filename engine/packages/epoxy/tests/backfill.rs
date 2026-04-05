mod common;

use common::{
	TestCtx,
	utils::{
		read_changelog_entries, read_v2_committed_value, write_legacy_v2_value, write_legacy_value,
	},
};

#[tokio::test(flavor = "multi_thread")]
async fn backfill_migrates_legacy_values_into_v2_and_changelog() {
	let mut test_ctx = TestCtx::new_with(&[1_u64]).await.unwrap();
	let replica_id = test_ctx.leader_id;
	let ctx = test_ctx.get_ctx(replica_id);

	write_legacy_value(ctx, replica_id, b"legacy-key-a", b"legacy-value-a")
		.await
		.unwrap();
	write_legacy_v2_value(ctx, replica_id, b"legacy-key-b", b"legacy-value-b")
		.await
		.unwrap();

	let workflow_id = ctx
		.workflow(epoxy::workflows::backfill::Input {
			chunk_size: Some(1),
		})
		.tag("replica", replica_id)
		.dispatch()
		.await
		.unwrap();
	let migrated_keys = ctx
		.workflow::<epoxy::workflows::backfill::Input>(workflow_id)
		.output()
		.await
		.unwrap();

	assert_eq!(migrated_keys, 2);

	let committed_a = read_v2_committed_value(ctx, replica_id, b"legacy-key-a")
		.await
		.unwrap()
		.unwrap();
	assert_eq!(committed_a.value, b"legacy-value-a");
	assert_eq!(committed_a.version, 0);
	assert!(!committed_a.mutable);

	let committed_b = read_v2_committed_value(ctx, replica_id, b"legacy-key-b")
		.await
		.unwrap()
		.unwrap();
	assert_eq!(committed_b.value, b"legacy-value-b");
	assert_eq!(committed_b.version, 0);
	assert!(!committed_b.mutable);

	let changelog_entries = read_changelog_entries(ctx, replica_id).await.unwrap();
	assert_eq!(changelog_entries.len(), 2);
	assert!(changelog_entries.iter().any(|entry| {
		entry.key == b"legacy-key-a" && entry.value == b"legacy-value-a" && entry.version == 0
	}));
	assert!(changelog_entries.iter().any(|entry| {
		entry.key == b"legacy-key-b" && entry.value == b"legacy-value-b" && entry.version == 0
	}));

	test_ctx.shutdown().await.unwrap();
}
