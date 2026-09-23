mod common;

use common::{
	TestCtx,
	utils::{
		get_local, read_legacy_value, read_v2_value, set_if_absent, write_legacy_v2_value,
		write_legacy_value,
	},
};
use epoxy::ops::kv::get::{Input, ReadMode};
use epoxy::ops::propose::{ConsensusFailedReason, ProposalResult};

#[tokio::test(flavor = "multi_thread")]
async fn dual_read_fallback_reads_legacy_subspaces_without_migrating() {
	let mut test_ctx = TestCtx::new_with(&[1_u64]).await.unwrap();
	let replica_id = test_ctx.leader_id;
	let ctx = test_ctx.get_ctx(replica_id);
	let blocked_key = b"legacy-committed-key";
	let blocked_value = b"legacy-committed-value";
	write_legacy_value(ctx, replica_id, blocked_key, blocked_value)
		.await
		.unwrap();
	assert_eq!(
		get_local(ctx, replica_id, blocked_key).await.unwrap(),
		Some(blocked_value.to_vec()),
	);
	assert_eq!(
		read_v2_value(ctx, replica_id, blocked_key).await.unwrap(),
		None
	);

	let blocked_result = set_if_absent(ctx, blocked_key, b"new-v2-value")
		.await
		.unwrap();
	assert!(matches!(
		blocked_result,
		ProposalResult::ConsensusFailed { reason: ConsensusFailedReason::ExpectedValueDoesNotMatch {
			current_value: Some(value),
		} } if value == blocked_value
	));
	assert_eq!(
		read_legacy_value(ctx, replica_id, blocked_key)
			.await
			.unwrap(),
		Some(blocked_value.to_vec()),
	);
	assert_eq!(
		read_v2_value(ctx, replica_id, blocked_key).await.unwrap(),
		None,
	);

	let migrated_key = b"legacy-value-key";
	let migrated_value = b"legacy-value";
	write_legacy_v2_value(ctx, replica_id, migrated_key, migrated_value)
		.await
		.unwrap();
	assert_eq!(
		get_local(ctx, replica_id, migrated_key).await.unwrap(),
		Some(migrated_value.to_vec()),
	);
	assert_eq!(
		read_v2_value(ctx, replica_id, migrated_key).await.unwrap(),
		None
	);
	for key in [blocked_key.as_slice(), migrated_key.as_slice()] {
		let local = ctx
			.op(Input {
				key: key.to_vec(),
				mode: ReadMode::LocalCommitted { replica_id },
			})
			.await
			.unwrap();
		assert_eq!(
			local.value.as_ref().and_then(|value| value.value.clone()),
			get_local(ctx, replica_id, key).await.unwrap()
		);
		assert_eq!(read_v2_value(ctx, replica_id, key).await.unwrap(), None);
		let agreed = ctx
			.op(Input {
				key: key.to_vec(),
				mode: ReadMode::Linearizable {
					target_replicas: None,
				},
			})
			.await
			.unwrap();
		assert_eq!(agreed.value, local.value);
	}

	let fresh_key = b"fresh-v2-key";
	let fresh_value = b"fresh-v2-value";
	let fresh_result = set_if_absent(ctx, fresh_key, fresh_value).await.unwrap();
	assert!(matches!(fresh_result, ProposalResult::Committed));
	assert_eq!(
		read_v2_value(ctx, replica_id, fresh_key).await.unwrap(),
		Some(fresh_value.to_vec()),
	);

	test_ctx.shutdown().await.unwrap();
}
