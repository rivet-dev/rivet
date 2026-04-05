mod common;

use common::{
	THREE_REPLICAS, TestCtx,
	utils::{get_local, read_ballot, set_if_absent, set_mutable, write_ballot},
};
use epoxy::{metrics, ops::propose::ProposalResult};
use epoxy_protocol::protocol;

static TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[tokio::test(flavor = "multi_thread")]
async fn proposal_uses_fast_and_contention_paths() {
	let _guard = TEST_LOCK.lock().await;
	let mut test_ctx = TestCtx::new_with(THREE_REPLICAS).await.unwrap();
	let replica_id = test_ctx.leader_id;
	let ctx = test_ctx.get_ctx(replica_id);

	let fast_path_before = metrics::FAST_PATH_TOTAL.get();
	let slow_path_before = metrics::SLOW_PATH_TOTAL.get();
	let prepare_before = metrics::PREPARE_TOTAL.get();
	let ballot_bumps_before = metrics::BALLOT_BUMP_TOTAL.get();
	let committed_before = metrics::PROPOSAL_TOTAL
		.with_label_values(&["committed"])
		.get();
	let slow_result_before = metrics::PROPOSAL_TOTAL
		.with_label_values(&["slow_path"])
		.get();

	let fast_key = b"proposal-fast-path";
	let fast_value = b"fast-value";
	let fast_result = set_if_absent(ctx, fast_key, fast_value).await.unwrap();
	assert!(matches!(fast_result, ProposalResult::Committed));
	assert_eq!(
		get_local(ctx, replica_id, fast_key).await.unwrap(),
		Some(fast_value.to_vec()),
	);
	assert_eq!(
		read_ballot(ctx, replica_id, fast_key).await.unwrap(),
		Some(protocol::Ballot {
			counter: 1,
			replica_id,
		}),
	);
	assert_eq!(metrics::FAST_PATH_TOTAL.get() - fast_path_before, 1);
	assert_eq!(metrics::SLOW_PATH_TOTAL.get() - slow_path_before, 0);
	assert_eq!(metrics::PREPARE_TOTAL.get() - prepare_before, 0);
	assert_eq!(metrics::BALLOT_BUMP_TOTAL.get() - ballot_bumps_before, 0);
	assert_eq!(
		metrics::PROPOSAL_TOTAL
			.with_label_values(&["committed"])
			.get() - committed_before,
		1,
	);
	assert_eq!(
		metrics::PROPOSAL_TOTAL
			.with_label_values(&["slow_path"])
			.get() - slow_result_before,
		0,
	);

	let slow_key = b"proposal-contention-path";
	let slow_value = b"slow-value";
	write_ballot(
		ctx,
		replica_id,
		slow_key,
		protocol::Ballot {
			counter: 7,
			replica_id: THREE_REPLICAS[1],
		},
	)
	.await
	.unwrap();

	let fast_path_before = metrics::FAST_PATH_TOTAL.get();
	let slow_path_before = metrics::SLOW_PATH_TOTAL.get();
	let prepare_before = metrics::PREPARE_TOTAL.get();
	let ballot_bumps_before = metrics::BALLOT_BUMP_TOTAL.get();
	let slow_result_before = metrics::PROPOSAL_TOTAL
		.with_label_values(&["slow_path"])
		.get();

	let slow_result = set_if_absent(ctx, slow_key, slow_value).await.unwrap();
	assert!(matches!(slow_result, ProposalResult::Committed));
	assert_eq!(
		get_local(ctx, replica_id, slow_key).await.unwrap(),
		Some(slow_value.to_vec()),
	);
	assert_eq!(
		read_ballot(ctx, replica_id, slow_key).await.unwrap(),
		Some(protocol::Ballot {
			counter: 8,
			replica_id,
		}),
	);
	assert_eq!(metrics::FAST_PATH_TOTAL.get() - fast_path_before, 0);
	assert_eq!(metrics::SLOW_PATH_TOTAL.get() - slow_path_before, 1);
	assert_eq!(metrics::PREPARE_TOTAL.get() - prepare_before, 1);
	assert_eq!(metrics::BALLOT_BUMP_TOTAL.get() - ballot_bumps_before, 1);
	assert_eq!(
		metrics::PROPOSAL_TOTAL
			.with_label_values(&["slow_path"])
			.get() - slow_result_before,
		1,
	);
	let key = b"mutable-fast-path";

	let first_result = set_mutable(ctx, key, b"value1").await.unwrap();
	assert!(matches!(first_result, ProposalResult::Committed));
	assert_eq!(read_ballot(ctx, replica_id, key).await.unwrap(), None);

	let fast_path_before = metrics::FAST_PATH_TOTAL.get();
	let slow_path_before = metrics::SLOW_PATH_TOTAL.get();
	let prepare_before = metrics::PREPARE_TOTAL.get();

	let second_result = set_mutable(ctx, key, b"value2").await.unwrap();
	assert!(matches!(second_result, ProposalResult::Committed));
	assert_eq!(read_ballot(ctx, replica_id, key).await.unwrap(), None);
	assert_eq!(metrics::FAST_PATH_TOTAL.get() - fast_path_before, 1);
	assert_eq!(metrics::SLOW_PATH_TOTAL.get() - slow_path_before, 0);
	assert_eq!(metrics::PREPARE_TOTAL.get() - prepare_before, 0);

	for _ in 0..20 {
		let mut replicated = true;
		for replica_id in THREE_REPLICAS {
			if get_local(test_ctx.get_ctx(*replica_id), *replica_id, key)
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
