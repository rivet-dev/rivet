mod common;

use epoxy::ops::propose::ProposalResult;
use epoxy_protocol::protocol;
use gas::prelude::*;

use common::{DEFAULT_REPLICA_IDS, THREE_REPLICAS, TestCtx, utils::execute_command};

#[tokio::test(flavor = "multi_thread")]
async fn test_kv_get_optimistic_local() {
	let test_ctx = TestCtx::new_with(THREE_REPLICAS).await.unwrap();
	let replica_id = test_ctx.leader_id;
	let ctx = test_ctx.get_ctx(replica_id);

	let key = b"test_optimistic_local";
	let value = b"local_value";

	// Set value locally
	let result = execute_command(
		ctx,
		protocol::CommandKind::SetCommand(protocol::SetCommand {
			key: key.to_vec(),
			value: Some(value.to_vec()),
		}),
		true, // Wait for propagation to ensure value is committed
	)
	.await
	.unwrap();
	assert!(matches!(result, ProposalResult::Committed));

	let output = ctx
		.op(epoxy::ops::kv::get_optimistic::Input {
			replica_id,
			key: key.to_vec(),
		})
		.await
		.unwrap();
	assert_eq!(output.value, Some(value.to_vec()));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_kv_get_optimistic_remote() {
	// Create test context with multiple replicas
	let test_ctx = TestCtx::new_with(THREE_REPLICAS).await.unwrap();

	// Use replica 1 to write data
	let writer_replica_id = THREE_REPLICAS[0];
	let writer_ctx = test_ctx.get_ctx(writer_replica_id);

	// Use replica 2 to read (simulating remote DC fetch)
	let reader_replica_id = THREE_REPLICAS[1];
	let reader_ctx = test_ctx.get_ctx(reader_replica_id);

	let key = b"test_optimistic_remote";
	let value = b"remote_value";

	// Set value on replica 1
	let result = execute_command(
		writer_ctx,
		protocol::CommandKind::SetCommand(protocol::SetCommand {
			key: key.to_vec(),
			value: Some(value.to_vec()),
		}),
		true, // Wait for propagation
	)
	.await
	.unwrap();
	assert!(matches!(result, ProposalResult::Committed));

	let output = reader_ctx
		.op(epoxy::ops::kv::get_optimistic::Input {
			replica_id: reader_replica_id,
			key: key.to_vec(),
		})
		.await
		.unwrap();
	assert_eq!(output.value, Some(value.to_vec()));

	// Second read from replica 2 - should now be cached
	let output2 = reader_ctx
		.op(epoxy::ops::kv::get_optimistic::Input {
			replica_id: reader_replica_id,
			key: key.to_vec(),
		})
		.await
		.unwrap();
	assert_eq!(output2.value, Some(value.to_vec()));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_kv_get_optimistic_not_found() {
	let test_ctx = TestCtx::new().await.unwrap();
	let replica_id = DEFAULT_REPLICA_IDS[0];
	let ctx = test_ctx.get_ctx(replica_id);

	let key = b"nonexistent_key";

	// Read a key that doesn't exist
	let output = ctx
		.op(epoxy::ops::kv::get_optimistic::Input {
			replica_id,
			key: key.to_vec(),
		})
		.await
		.unwrap();
	assert_eq!(output.value, None);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_kv_get_optimistic_cache_persistence() {
	// Create test context with multiple replicas
	let test_ctx = TestCtx::new().await.unwrap();

	let writer_replica_id = DEFAULT_REPLICA_IDS[0];
	let writer_ctx = test_ctx.get_ctx(writer_replica_id);

	let reader_replica_id = DEFAULT_REPLICA_IDS[1];
	let reader_ctx = test_ctx.get_ctx(reader_replica_id);

	let key = b"test_cache_persistence";
	let initial_value = b"initial";
	let updated_value = b"updated";

	// Set initial value on replica 1
	let result = execute_command(
		writer_ctx,
		protocol::CommandKind::SetCommand(protocol::SetCommand {
			key: key.to_vec(),
			value: Some(initial_value.to_vec()),
		}),
		true,
	)
	.await
	.unwrap();
	assert!(matches!(result, ProposalResult::Committed));

	// Wait for propagation
	tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

	// Read from replica 2 - this will cache the initial value
	let output = reader_ctx
		.op(epoxy::ops::kv::get_optimistic::Input {
			replica_id: reader_replica_id,
			key: key.to_vec(),
		})
		.await
		.unwrap();
	assert_eq!(output.value, Some(initial_value.to_vec()));

	// Update value on replica 1
	let result = execute_command(
		writer_ctx,
		protocol::CommandKind::SetCommand(protocol::SetCommand {
			key: key.to_vec(),
			value: Some(updated_value.to_vec()),
		}),
		true,
	)
	.await
	.unwrap();
	assert!(matches!(result, ProposalResult::Committed));

	// Wait for propagation
	tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

	// Read from replica 2 again
	// Since the value has been propagated to replica 2, it will now find it locally
	// and return the updated value (not from cache)
	let output2 = reader_ctx
		.op(epoxy::ops::kv::get_optimistic::Input {
			replica_id: reader_replica_id,
			key: key.to_vec(),
		})
		.await
		.unwrap();

	// The value should be updated since it's now available locally on replica 2
	assert_eq!(output2.value, Some(updated_value.to_vec()));
}
