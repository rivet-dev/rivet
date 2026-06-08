mod common;

use common::{THREE_REPLICAS, TestCtx, utils::set_mutable};
use epoxy::ops::propose::ProposalResult;
use futures_util::future::join_all;

static TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Reproduces a concurrent mutable upsert failure: `proposal failed`.
///
/// The scenario fans N parallel `epoxy_propose` calls with `mutable=true` on the same
/// by-name keys; the bytes differ between proposers because each one mints a fresh
/// `Id::new_v1()` for the same name. For idempotent semantics every caller
/// should observe `Committed` (last-writer-wins), but `propose::result_for_committed_value`
/// returns `ConsensusFailed{ExpectedValueDoesNotMatch}` whenever a concurrent proposer
/// committed a different value first.
///
/// This test isolates that behavior at the epoxy layer: N concurrent `set_mutable` calls on
/// the same key with distinct values. Expectation under idempotent mutable semantics: every
/// call resolves to `Committed`. Current behavior: at least one returns `ConsensusFailed`.
#[tokio::test(flavor = "multi_thread")]
async fn concurrent_mutable_proposals_same_key_different_values() {
	let _guard = TEST_LOCK.lock().await;
	let mut test_ctx = TestCtx::new_with(THREE_REPLICAS).await.unwrap();
	let replica_id = test_ctx.leader_id;
	let ctx = test_ctx.get_ctx(replica_id);

	let key = b"concurrent-mutable-repro-by-name-key";
	let n: usize = 10;

	tracing::info!(
		n,
		"firing concurrent mutable proposals on shared key with distinct values"
	);

	let results: Vec<(usize, anyhow::Result<ProposalResult>)> = join_all((0..n).map(|i| {
		let value = format!("value-from-proposer-{:02}", i).into_bytes();
		async move {
			let res = set_mutable(ctx, key, &value).await;
			(i, res)
		}
	}))
	.await;

	let mut committed = 0;
	let mut consensus_failed = 0;
	let mut errored = 0;
	for (i, res) in &results {
		match res {
			Ok(ProposalResult::Committed) => {
				committed += 1;
				tracing::info!(worker = i, "Committed");
			}
			Ok(ProposalResult::ConsensusFailed { reason }) => {
				consensus_failed += 1;
				tracing::warn!(worker = i, ?reason, "ConsensusFailed");
			}
			Err(err) => {
				errored += 1;
				tracing::error!(worker = i, ?err, "errored");
			}
		}
	}

	tracing::info!(committed, consensus_failed, errored, n, "round complete");

	test_ctx.shutdown().await.unwrap();

	if consensus_failed > 0 || errored > 0 {
		for (i, res) in &results {
			eprintln!("worker {i}: {res:?}");
		}
		panic!(
			"{consensus_failed} ConsensusFailed + {errored} errored out of {n} (idempotent mutable upsert expects 0)",
		);
	}
}
