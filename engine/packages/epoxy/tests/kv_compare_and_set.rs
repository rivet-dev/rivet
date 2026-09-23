mod common;

use common::{
	THREE_REPLICAS, TestCtx,
	utils::{check_and_set_absent, check_and_set_mutable, get_local, set_mutable, write_ballot},
};
use epoxy::ops::propose::{ConsensusFailedReason, ProposalResult};
use epoxy_protocol::protocol;
use tokio::time::{Duration, Instant};

fn assert_committed(result: ProposalResult) {
	assert!(matches!(&result, ProposalResult::Committed), "{result:?}");
}

fn assert_mismatch(result: ProposalResult, expected_current: Option<&[u8]>) {
	match result {
		ProposalResult::ConsensusFailed {
			reason: ConsensusFailedReason::ExpectedValueDoesNotMatch { current_value },
		} => assert_eq!(current_value.as_deref(), expected_current),
		other => panic!("expected value mismatch, got {other:?}"),
	}
}

fn assert_lost_concurrent_race(result: ProposalResult) {
	assert!(
		matches!(result, ProposalResult::ConsensusFailed { .. }),
		"expected concurrent CAS loser to retry, got {result:?}"
	);
}

async fn wait_for_replication(test_ctx: &TestCtx, key: &[u8], expected: &[u8]) {
	let deadline = Instant::now() + Duration::from_secs(5);
	loop {
		let mut replicated = true;
		for &replica_id in THREE_REPLICAS {
			if get_local(test_ctx.get_ctx(replica_id), replica_id, key)
				.await
				.unwrap()
				.as_deref() != Some(expected)
			{
				replicated = false;
				break;
			}
		}
		if replicated {
			return;
		}
		assert!(
			Instant::now() < deadline,
			"timed out waiting for baseline value to replicate"
		);
		tokio::time::sleep(Duration::from_millis(10)).await;
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn mutable_check_and_set_semantics() {
	let mut test_ctx = TestCtx::new_with(THREE_REPLICAS).await.unwrap();

	matching_value_commits(&test_ctx).await;
	stale_expectation_conflicts(&test_ctx).await;
	concurrent_writers_commit_one_successor(&test_ctx).await;
	prepare_retry_preserves_expectation(&test_ctx).await;
	immutable_none_remains_idempotent(&test_ctx).await;

	test_ctx.shutdown().await.unwrap();
}

async fn matching_value_commits(test_ctx: &TestCtx) {
	let replica_id = test_ctx.leader_id;
	let ctx = test_ctx.get_ctx(replica_id);
	let key = b"mutable-cas-success";

	assert_committed(set_mutable(ctx, key, b"generation-1").await.unwrap());
	wait_for_replication(test_ctx, key, b"generation-1").await;
	assert_committed(
		check_and_set_mutable(
			ctx,
			key,
			vec![
				Some(b"another-generation".to_vec()),
				Some(b"generation-1".to_vec()),
			],
			Some(b"generation-2".to_vec()),
		)
		.await
		.unwrap(),
	);
	assert_eq!(
		get_local(ctx, replica_id, key).await.unwrap().as_deref(),
		Some(b"generation-2".as_slice())
	);
}

async fn stale_expectation_conflicts(test_ctx: &TestCtx) {
	let ctx = test_ctx.get_ctx(test_ctx.leader_id);
	let key = b"mutable-cas-stale";

	assert_committed(set_mutable(ctx, key, b"generation-2").await.unwrap());
	wait_for_replication(test_ctx, key, b"generation-2").await;
	assert_mismatch(
		check_and_set_mutable(
			ctx,
			key,
			vec![Some(b"generation-1".to_vec())],
			Some(b"generation-3".to_vec()),
		)
		.await
		.unwrap(),
		Some(b"generation-2"),
	);
}

async fn concurrent_writers_commit_one_successor(test_ctx: &TestCtx) {
	let replica_id = test_ctx.leader_id;
	let ctx = test_ctx.get_ctx(replica_id);
	let key = b"mutable-cas-concurrent";

	assert_committed(set_mutable(ctx, key, b"generation-1").await.unwrap());
	wait_for_replication(test_ctx, key, b"generation-1").await;
	let (left, right) = tokio::join!(
		check_and_set_mutable(
			ctx,
			key,
			vec![Some(b"generation-1".to_vec())],
			Some(b"left".to_vec()),
		),
		check_and_set_mutable(
			ctx,
			key,
			vec![Some(b"generation-1".to_vec())],
			Some(b"right".to_vec()),
		),
	);
	let left = left.unwrap();
	let right = right.unwrap();
	let committed = get_local(ctx, replica_id, key).await.unwrap().unwrap();
	match (left, right, committed.as_slice()) {
		(ProposalResult::Committed, loser, b"left") => assert_lost_concurrent_race(loser),
		(loser, ProposalResult::Committed, b"right") => assert_lost_concurrent_race(loser),
		results => panic!("expected one committed writer and one conflict, got {results:?}"),
	}
}

async fn prepare_retry_preserves_expectation(test_ctx: &TestCtx) {
	let replica_id = test_ctx.leader_id;
	let ctx = test_ctx.get_ctx(replica_id);
	let key = b"mutable-cas-retry";

	assert_committed(set_mutable(ctx, key, b"generation-1").await.unwrap());
	wait_for_replication(test_ctx, key, b"generation-1").await;
	for &remote_replica_id in &THREE_REPLICAS[1..] {
		write_ballot(
			test_ctx.get_ctx(remote_replica_id),
			remote_replica_id,
			key,
			protocol::Ballot {
				counter: 20,
				replica_id: remote_replica_id,
			},
		)
		.await
		.unwrap();
	}

	assert_committed(
		check_and_set_mutable(
			ctx,
			key,
			vec![Some(b"generation-1".to_vec())],
			Some(b"generation-2".to_vec()),
		)
		.await
		.unwrap(),
	);
	assert_eq!(
		get_local(ctx, replica_id, key).await.unwrap().as_deref(),
		Some(b"generation-2".as_slice())
	);
}

async fn immutable_none_remains_idempotent(test_ctx: &TestCtx) {
	let ctx = test_ctx.get_ctx(test_ctx.leader_id);
	let key = b"immutable-create-if-absent";

	assert_committed(check_and_set_absent(ctx, key, b"created").await.unwrap());
	assert_committed(check_and_set_absent(ctx, key, b"created").await.unwrap());
	assert_mismatch(
		check_and_set_absent(ctx, key, b"different").await.unwrap(),
		Some(b"created"),
	);
}
