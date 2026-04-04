mod common;

use common::utils::{
	get_local, read_changelog_entries, read_legacy_value, read_v2_committed_value,
	write_legacy_value,
};
use epoxy::ops::propose::{
	self, Command, CommandError, CommandKind, Proposal, ProposalResult, SetCommand,
};
use test_snapshot::SnapshotTestCtx;

const TEST_KEYS: &[(&[u8], &[u8])] = &[
	(b"actor:abc123", b"running"),
	(b"actor:def456", b"stopped"),
	(b"config:version", b"42"),
];

/// Propose a key scoped to a single replica.
async fn propose_local(
	ctx: &gas::prelude::TestCtx,
	replica_id: epoxy_protocol::protocol::ReplicaId,
	key: &[u8],
	value: &[u8],
	mutable: bool,
) -> anyhow::Result<ProposalResult> {
	ctx.op(propose::Input {
		proposal: Proposal {
			commands: vec![Command {
				kind: CommandKind::SetCommand(SetCommand {
					key: key.to_vec(),
					value: Some(value.to_vec()),
				}),
			}],
		},
		mutable,
		purge_cache: false,
		target_replicas: Some(vec![replica_id]),
	})
	.await
}

/// Load a v1 snapshot and verify dual reads, proposals, and backfill across
/// both replicas.
#[tokio::test(flavor = "multi_thread")]
async fn v1_snapshot_dual_read_mutate_and_backfill() {
	let mut test_ctx = SnapshotTestCtx::from_snapshot_with_coordinator("epoxy-v1")
		.await
		.unwrap();

	for &replica_id in &test_ctx.replica_ids() {
		let ctx = test_ctx.get_ctx(replica_id);

		// -- Phase 1: Verify v1 data exists and dual reads work --

		for &(key, expected_value) in TEST_KEYS {
			// Legacy subspace should have the v1 data.
			let legacy_val = read_legacy_value(ctx, replica_id, key).await.unwrap();
			assert!(
				legacy_val.is_some(),
				"replica {replica_id}: legacy value for {:?} should exist",
				String::from_utf8_lossy(key),
			);

			// V2 subspace should be empty before backfill.
			let v2_val = read_v2_committed_value(ctx, replica_id, key).await.unwrap();
			assert!(
				v2_val.is_none(),
				"replica {replica_id}: v2 value for {:?} should not exist before backfill",
				String::from_utf8_lossy(key),
			);

			// Dual-read (get_local) should see the v1 data through fallback.
			let dual_val = get_local(ctx, replica_id, key).await.unwrap();
			assert_eq!(
				dual_val.as_deref(),
				Some(expected_value),
				"replica {replica_id}: dual read for {:?} should return v1 value",
				String::from_utf8_lossy(key),
			);
		}

		// -- Phase 2: Immutable re-proposal on v1 keys --

		// Re-proposing the same value is idempotent (no-op, no changelog write).
		let result = propose_local(ctx, replica_id, b"actor:abc123", b"running", false)
			.await
			.unwrap();
		assert!(
			matches!(result, ProposalResult::Committed),
			"replica {replica_id}: idempotent re-proposal should succeed: {result:?}",
		);

		// Proposing a different value for an immutable key returns CommandError.
		let result = propose_local(ctx, replica_id, b"actor:abc123", b"different", false)
			.await
			.unwrap();
		assert!(
			matches!(
				result,
				ProposalResult::CommandError(CommandError::ExpectedValueDoesNotMatch { .. })
			),
			"replica {replica_id}: conflicting immutable proposal should return CommandError: {result:?}",
		);

		// -- Phase 3: Propose a new mutable key alongside v1 data --

		let result = propose_local(ctx, replica_id, b"new-key", b"new-value", true)
			.await
			.unwrap();
		assert!(
			matches!(result, ProposalResult::Committed),
			"replica {replica_id}: new key proposal should commit: {result:?}",
		);

		let new_val = get_local(ctx, replica_id, b"new-key").await.unwrap();
		assert_eq!(
			new_val.as_deref(),
			Some(b"new-value".as_slice()),
			"replica {replica_id}: new key should be readable via dual-read",
		);

		// Mutate it.
		let result = propose_local(ctx, replica_id, b"new-key", b"updated-value", true)
			.await
			.unwrap();
		assert!(
			matches!(result, ProposalResult::Committed),
			"replica {replica_id}: mutation should commit: {result:?}",
		);
		let updated = get_local(ctx, replica_id, b"new-key").await.unwrap();
		assert_eq!(
			updated.as_deref(),
			Some(b"updated-value".as_slice()),
			"replica {replica_id}: mutated key should reflect new value",
		);

		// -- Phase 4: Run backfill and verify migration --

		let workflow_id = ctx
			.workflow(epoxy::workflows::backfill::Input { chunk_size: Some(10) })
			.tag("replica", replica_id)
			.dispatch()
			.await
			.unwrap();
		let migrated_keys = ctx
			.workflow::<epoxy::workflows::backfill::Input>(workflow_id)
			.output()
			.await
			.unwrap();

		assert_eq!(
			migrated_keys, 3,
			"replica {replica_id}: backfill should migrate all 3 legacy keys",
		);

		// All original v1 keys should now have v2 committed values.
		for &(key, expected_value) in TEST_KEYS {
			let committed = read_v2_committed_value(ctx, replica_id, key)
				.await
				.unwrap()
				.unwrap_or_else(|| {
					panic!(
						"replica {replica_id}: v2 value for {:?} should exist after backfill",
						String::from_utf8_lossy(key),
					)
				});
			assert_eq!(committed.value, expected_value);
			assert_eq!(committed.version, 0);
			assert!(!committed.mutable);
		}

		// Verify changelog has entries for all 3 backfilled keys. Additional
		// entries from phase-3 proposals and commit propagation may also exist.
		let changelog = read_changelog_entries(ctx, replica_id).await.unwrap();
		assert!(
			changelog.len() >= 5,
			"replica {replica_id}: expected at least 5 changelog entries (3 backfill + 2 proposals), got {}",
			changelog.len(),
		);
		for &(key, expected_value) in TEST_KEYS {
			assert!(
				changelog
					.iter()
					.any(|e| e.key == key && e.value == expected_value),
				"replica {replica_id}: changelog should contain backfill entry for {:?}",
				String::from_utf8_lossy(key),
			);
		}

		// -- Phase 5: Dual read prefers v2 after backfill --

		// Overwrite the legacy value with stale data. Dual read should still
		// return the v2 value since v2 is checked first in the read order.
		write_legacy_value(ctx, replica_id, b"actor:abc123", b"stale-legacy")
			.await
			.unwrap();

		let dual_val = get_local(ctx, replica_id, b"actor:abc123").await.unwrap();
		assert_eq!(
			dual_val.as_deref(),
			Some(b"running".as_slice()),
			"replica {replica_id}: dual read should prefer v2 over stale legacy",
		);

		// -- Phase 6: Propose on a backfilled key --

		// The backfilled keys are immutable (version 0, mutable=false). A same-
		// value re-proposal should be idempotent. A new-value proposal should
		// be rejected since the v2 committed value now exists.
		let result =
			propose_local(ctx, replica_id, b"actor:def456", b"stopped", false)
				.await
				.unwrap();
		assert!(
			matches!(result, ProposalResult::Committed),
			"replica {replica_id}: idempotent proposal on backfilled key should succeed: {result:?}",
		);

		let result =
			propose_local(ctx, replica_id, b"actor:def456", b"changed", false)
				.await
				.unwrap();
		assert!(
			matches!(
				result,
				ProposalResult::CommandError(CommandError::ExpectedValueDoesNotMatch { .. })
			),
			"replica {replica_id}: conflicting proposal on backfilled key should fail: {result:?}",
		);
	}

	test_ctx.shutdown().await.unwrap();
}
