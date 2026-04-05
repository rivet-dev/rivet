mod common;

use epoxy::ops::propose::{CommandError, ProposalResult};

use common::{
	THREE_REPLICAS, TestCtx,
	utils::{
		check_and_set_absent, get_local, read_ballot, read_changelog_entries,
		read_v2_committed_value, read_v2_value, set_if_absent, set_mutable,
	},
};

static TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[tokio::test(flavor = "multi_thread")]
async fn test_kv_operations() {
	let _guard = TEST_LOCK.lock().await;
	let mut test_ctx = TestCtx::new_with(THREE_REPLICAS).await.unwrap();
	let replica_id = THREE_REPLICAS[0];
	let ctx = test_ctx.get_ctx(replica_id);

	let set_key = b"immutable-set-key";

	let first_result = set_if_absent(ctx, set_key, b"value1").await.unwrap();
	assert!(matches!(first_result, ProposalResult::Committed));
	assert_eq!(
		read_v2_value(ctx, replica_id, set_key).await.unwrap(),
		Some(b"value1".to_vec()),
	);

	let same_value_result = set_if_absent(ctx, set_key, b"value1").await.unwrap();
	assert!(matches!(same_value_result, ProposalResult::Committed));
	assert_eq!(
		get_local(ctx, replica_id, set_key).await.unwrap(),
		Some(b"value1".to_vec()),
	);

	let different_value_result = set_if_absent(ctx, set_key, b"value2").await.unwrap();
	assert!(matches!(
		different_value_result,
		ProposalResult::CommandError(CommandError::ExpectedValueDoesNotMatch {
			current_value: Some(value),
		}) if value == b"value1".to_vec()
	));
	assert_eq!(
		read_v2_value(ctx, replica_id, set_key).await.unwrap(),
		Some(b"value1".to_vec()),
	);

	let cas_key = b"immutable-cas-key";

	let first_result = check_and_set_absent(ctx, cas_key, b"created")
		.await
		.unwrap();
	assert!(matches!(first_result, ProposalResult::Committed));
	assert_eq!(
		get_local(ctx, replica_id, cas_key).await.unwrap(),
		Some(b"created".to_vec()),
	);

	let same_value_result = check_and_set_absent(ctx, cas_key, b"created")
		.await
		.unwrap();
	assert!(matches!(same_value_result, ProposalResult::Committed));

	let different_value_result = check_and_set_absent(ctx, cas_key, b"other").await.unwrap();
	assert!(matches!(
		different_value_result,
		ProposalResult::CommandError(CommandError::ExpectedValueDoesNotMatch {
			current_value: Some(value),
		}) if value == b"created".to_vec()
	));
	assert_eq!(
		read_v2_value(ctx, replica_id, cas_key).await.unwrap(),
		Some(b"created".to_vec()),
	);
	let key = b"mutable-key";

	let first_result = set_mutable(ctx, key, b"value1").await.unwrap();
	assert!(matches!(first_result, ProposalResult::Committed));
	assert_eq!(
		read_v2_committed_value(ctx, replica_id, key).await.unwrap(),
		Some(epoxy::keys::CommittedValue {
			value: b"value1".to_vec(),
			version: 1,
			mutable: true,
		}),
	);
	assert_eq!(read_ballot(ctx, replica_id, key).await.unwrap(), None);

	let second_result = set_mutable(ctx, key, b"value2").await.unwrap();
	assert!(matches!(second_result, ProposalResult::Committed));
	assert_eq!(
		read_v2_committed_value(ctx, replica_id, key).await.unwrap(),
		Some(epoxy::keys::CommittedValue {
			value: b"value2".to_vec(),
			version: 2,
			mutable: true,
		}),
	);
	assert_eq!(read_ballot(ctx, replica_id, key).await.unwrap(), None);

	let immutable_result = set_if_absent(ctx, key, b"value3").await.unwrap();
	assert!(matches!(
		immutable_result,
		ProposalResult::CommandError(CommandError::ExpectedValueDoesNotMatch {
			current_value: Some(value),
		}) if value == b"value2".to_vec()
	));

	let changelog_entries = read_changelog_entries(ctx, replica_id).await.unwrap();
	assert!(
		changelog_entries.contains(&epoxy::protocol::ChangelogEntry {
			key: key.to_vec(),
			value: b"value1".to_vec(),
			version: 1,
			mutable: true,
		})
	);
	assert!(
		changelog_entries.contains(&epoxy::protocol::ChangelogEntry {
			key: key.to_vec(),
			value: b"value2".to_vec(),
			version: 2,
			mutable: true,
		})
	);

	for _ in 0..20 {
		let mut replicated = true;
		for replica_id in THREE_REPLICAS {
			if read_v2_value(test_ctx.get_ctx(*replica_id), *replica_id, key)
				.await
				.unwrap() != Some(b"value2".to_vec())
			{
				replicated = false;
				break;
			}
		}
		if replicated {
			break;
		}

		tokio::time::sleep(std::time::Duration::from_millis(50)).await;
	}

	test_ctx.shutdown().await.unwrap();
}
