mod common;

use common::{THREE_REPLICAS, TestCtx, utils::write_v2_committed_value};
use epoxy_protocol::protocol::CommittedValue;

#[tokio::test(flavor = "multi_thread")]
async fn reads_the_newest_reachable_committed_value() {
	let mut test_ctx = TestCtx::new_with(THREE_REPLICAS).await.unwrap();
	let key = b"mutable-latest";
	write_v2_committed_value(
		test_ctx.get_ctx(THREE_REPLICAS[0]),
		THREE_REPLICAS[0],
		key,
		CommittedValue {
			value: Some(b"generation-1".to_vec()),
			version: 1,
			mutable: true,
		},
	)
	.await
	.unwrap();
	write_v2_committed_value(
		test_ctx.get_ctx(THREE_REPLICAS[1]),
		THREE_REPLICAS[1],
		key,
		CommittedValue {
			value: Some(b"generation-2".to_vec()),
			version: 2,
			mutable: true,
		},
	)
	.await
	.unwrap();

	let output = test_ctx
		.get_ctx(THREE_REPLICAS[2])
		.op(epoxy::ops::kv::get_latest::Input { key: key.to_vec() })
		.await
		.unwrap();
	assert_eq!(output.value.unwrap().version, 2);

	// Losing the configured leader does not prevent another replica from returning the current
	// committed value.
	test_ctx
		.stop_replica(THREE_REPLICAS[0], false)
		.await
		.unwrap();
	let output = test_ctx
		.get_ctx(THREE_REPLICAS[2])
		.op(epoxy::ops::kv::get_latest::Input { key: key.to_vec() })
		.await
		.unwrap();
	assert_eq!(
		output.value.unwrap().value.as_deref(),
		Some(b"generation-2".as_slice())
	);

	test_ctx.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn returns_a_stale_committed_value_when_newer_replica_is_unreachable() {
	let mut test_ctx = TestCtx::new_with(THREE_REPLICAS).await.unwrap();
	let key = b"mutable-revocation-lag";
	for (replica_id, version) in [(THREE_REPLICAS[0], 1), (THREE_REPLICAS[1], 2)] {
		write_v2_committed_value(
			test_ctx.get_ctx(replica_id),
			replica_id,
			key,
			CommittedValue {
				value: Some(format!("generation-{version}").into_bytes()),
				version,
				mutable: true,
			},
		)
		.await
		.unwrap();
	}

	let current = test_ctx
		.get_ctx(THREE_REPLICAS[0])
		.op(epoxy::ops::kv::get_latest::Input { key: key.to_vec() })
		.await
		.unwrap();
	assert_eq!(current.value.unwrap().version, 2);

	// The newer replica is isolated. Availability takes precedence over revocation freshness.
	test_ctx
		.stop_replica(THREE_REPLICAS[1], false)
		.await
		.unwrap();
	let stale = test_ctx
		.get_ctx(THREE_REPLICAS[0])
		.op(epoxy::ops::kv::get_latest::Input { key: key.to_vec() })
		.await
		.unwrap();
	assert_eq!(stale.value.unwrap().version, 1);

	let _ = test_ctx.shutdown().await;
}
