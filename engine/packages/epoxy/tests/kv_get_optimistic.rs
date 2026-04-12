mod common;

use common::{
	DEFAULT_REPLICA_IDS, THREE_REPLICAS, TestCtx,
	utils::{
		get_local, read_cache_committed_value, read_cache_value, read_v2_value, set_if_absent,
		set_mutable, write_cache_committed_value, write_legacy_value, write_v2_committed_value,
		write_v2_value,
	},
};
use epoxy::ops::propose::ProposalResult;
use epoxy_protocol::protocol::{CachingBehavior, ReplicaId};

static TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn optimistic_get(
	ctx: &gas::prelude::TestCtx,
	replica_id: ReplicaId,
	key: &[u8],
) -> Option<Vec<u8>> {
	get_with_behavior(ctx, replica_id, key, CachingBehavior::Optimistic).await
}

async fn get_with_behavior(
	ctx: &gas::prelude::TestCtx,
	replica_id: ReplicaId,
	key: &[u8],
	caching_behavior: CachingBehavior,
) -> Option<Vec<u8>> {
	ctx.op(epoxy::ops::kv::get_optimistic::Input {
		replica_id,
		key: key.to_vec(),
		caching_behavior,
		target_replicas: None,
		save_empty: false,
	})
	.await
	.unwrap()
	.value
}

#[tokio::test(flavor = "multi_thread")]
async fn test_kv_get_optimistic_paths() {
	let _guard = TEST_LOCK.lock().await;
	{
		let mut test_ctx = TestCtx::new_with(THREE_REPLICAS).await.unwrap();
		let replica_id = test_ctx.leader_id;
		let ctx = test_ctx.get_ctx(replica_id);
		let key = b"test-optimistic-local";
		let value = b"local-value";

		let result = set_if_absent(ctx, key, value).await.unwrap();
		assert!(matches!(result, ProposalResult::Committed));
		assert_eq!(
			optimistic_get(ctx, replica_id, key).await,
			Some(value.to_vec())
		);

		test_ctx.shutdown().await.unwrap();
	}

	{
		let mut test_ctx = TestCtx::new().await.unwrap();
		let writer_replica_id = DEFAULT_REPLICA_IDS[0];
		let reader_replica_id = DEFAULT_REPLICA_IDS[1];

		assert_eq!(
			optimistic_get(
				test_ctx.get_ctx(reader_replica_id),
				reader_replica_id,
				b"nonexistent-key",
			)
			.await,
			None,
		);

		let remote_key = b"test-optimistic-remote";
		let remote_value = b"remote-value";
		write_v2_value(
			test_ctx.get_ctx(writer_replica_id),
			writer_replica_id,
			remote_key,
			remote_value,
		)
		.await
		.unwrap();
		assert_eq!(
			read_v2_value(
				test_ctx.get_ctx(reader_replica_id),
				reader_replica_id,
				remote_key,
			)
			.await
			.unwrap(),
			None,
		);

		assert_eq!(
			optimistic_get(
				test_ctx.get_ctx(reader_replica_id),
				reader_replica_id,
				remote_key,
			)
			.await,
			Some(remote_value.to_vec()),
		);
		assert_eq!(
			read_cache_value(
				test_ctx.get_ctx(reader_replica_id),
				reader_replica_id,
				remote_key,
			)
			.await
			.unwrap(),
			Some(remote_value.to_vec()),
		);

		test_ctx
			.stop_replica(writer_replica_id, false)
			.await
			.unwrap();
		assert_eq!(
			optimistic_get(
				test_ctx.get_ctx(reader_replica_id),
				reader_replica_id,
				remote_key,
			)
			.await,
			Some(remote_value.to_vec()),
		);

		let fallback_key = b"test-optimistic-v1-fallback";
		let fallback_value = b"legacy-value";
		write_legacy_value(
			test_ctx.get_ctx(reader_replica_id),
			reader_replica_id,
			fallback_key,
			fallback_value,
		)
		.await
		.unwrap();
		assert_eq!(
			read_v2_value(
				test_ctx.get_ctx(reader_replica_id),
				reader_replica_id,
				fallback_key,
			)
			.await
			.unwrap(),
			None,
		);
		assert_eq!(
			get_local(
				test_ctx.get_ctx(reader_replica_id),
				reader_replica_id,
				fallback_key,
			)
			.await
			.unwrap(),
			Some(fallback_value.to_vec()),
		);
		assert_eq!(
			optimistic_get(
				test_ctx.get_ctx(reader_replica_id),
				reader_replica_id,
				fallback_key,
			)
			.await,
			Some(fallback_value.to_vec()),
		);

		test_ctx.shutdown().await.unwrap();
	}

	{
		let mut test_ctx = TestCtx::new().await.unwrap();
		let writer_replica_id = DEFAULT_REPLICA_IDS[0];
		let reader_replica_id = DEFAULT_REPLICA_IDS[1];
		let key = b"skip-cache-key";

		write_v2_committed_value(
			test_ctx.get_ctx(writer_replica_id),
			writer_replica_id,
			key,
			epoxy::keys::CommittedValue {
				value: b"remote-value".to_vec(),
				version: 2,
				mutable: true,
			},
		)
		.await
		.unwrap();
		write_cache_committed_value(
			test_ctx.get_ctx(reader_replica_id),
			reader_replica_id,
			key,
			epoxy::keys::CommittedValue {
				value: b"stale-cache".to_vec(),
				version: 1,
				mutable: true,
			},
		)
		.await
		.unwrap();

		assert_eq!(
			get_with_behavior(
				test_ctx.get_ctx(reader_replica_id),
				reader_replica_id,
				key,
				CachingBehavior::SkipCache,
			)
			.await,
			Some(b"remote-value".to_vec()),
		);
		assert_eq!(
			read_cache_committed_value(
				test_ctx.get_ctx(reader_replica_id),
				reader_replica_id,
				key,
			)
			.await
			.unwrap(),
			Some(epoxy::keys::CommittedValue {
				value: b"stale-cache".to_vec(),
				version: 1,
				mutable: true,
			}),
		);

		test_ctx.shutdown().await.unwrap();
	}

	{
		let mut test_ctx = TestCtx::new().await.unwrap();
		let leader_replica_id = DEFAULT_REPLICA_IDS[0];
		let follower_replica_id = DEFAULT_REPLICA_IDS[1];
		let leader_ctx = test_ctx.get_ctx(leader_replica_id);
		let follower_ctx = test_ctx.get_ctx(follower_replica_id);
		let key = b"mutable-cache-purge";

		assert!(matches!(
			set_mutable(leader_ctx, key, b"value1").await.unwrap(),
			ProposalResult::Committed
		));
		write_cache_committed_value(
			follower_ctx,
			follower_replica_id,
			key,
			epoxy::keys::CommittedValue {
				value: b"value1".to_vec(),
				version: 1,
				mutable: true,
			},
		)
		.await
		.unwrap();

		assert!(matches!(
			set_mutable(leader_ctx, key, b"value2").await.unwrap(),
			ProposalResult::Committed
		));

		for _ in 0..20 {
			if read_cache_value(follower_ctx, follower_replica_id, key)
				.await
				.unwrap()
				.is_none() && read_v2_value(follower_ctx, follower_replica_id, key)
				.await
				.unwrap() == Some(b"value2".to_vec())
			{
				test_ctx.shutdown().await.unwrap();
				return;
			}

			tokio::time::sleep(std::time::Duration::from_millis(50)).await;
		}

		panic!("mutable commit did not clear follower cache and replicate the new value");
	}
}
