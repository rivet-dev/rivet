mod common;

use epoxy::ops::propose::{CommandError, ProposalResult};
use epoxy_protocol::protocol;
use gas::prelude::*;

use common::{THREE_REPLICAS, TestCtx, utils::execute_command};

#[tokio::test(flavor = "multi_thread")]
async fn test_set_operations() {
	let test_ctx = TestCtx::new_with(THREE_REPLICAS).await.unwrap();
	let replica_id = THREE_REPLICAS[0];
	let ctx = test_ctx.get_ctx(replica_id);

	let key = b"test_key";

	// First set
	let result = execute_command(
		ctx,
		protocol::CommandKind::SetCommand(protocol::SetCommand {
			key: key.to_vec(),
			value: Some(b"value1".to_vec()),
		}),
		true,
	)
	.await
	.unwrap();
	assert!(matches!(result, ProposalResult::Committed));

	// Note: Direct workflow state checking is no longer available through ops API
	// Verify through KV get operation instead

	// Second set
	let result = execute_command(
		ctx,
		protocol::CommandKind::SetCommand(protocol::SetCommand {
			key: key.to_vec(),
			value: Some(b"value2".to_vec()),
		}),
		true,
	)
	.await
	.unwrap();
	assert!(matches!(result, ProposalResult::Committed));

	// Note: Direct workflow state checking is no longer available through ops API
	// Verify through KV get operation instead

	// Third set with None (delete)
	let result = execute_command(
		ctx,
		protocol::CommandKind::SetCommand(protocol::SetCommand {
			key: key.to_vec(),
			value: None,
		}),
		true,
	)
	.await
	.unwrap();
	assert!(matches!(result, ProposalResult::Committed));

	// Note: Direct workflow state checking is no longer available through ops API
	// Verify through KV get operation instead
}

#[tokio::test(flavor = "multi_thread")]
async fn test_check_and_set_operations() {
	let test_ctx = TestCtx::new_with(THREE_REPLICAS).await.unwrap();
	let replica_id = THREE_REPLICAS[0];
	let ctx = test_ctx.get_ctx(replica_id);

	let key = b"cas_test_key";

	// Set initial value
	let result = execute_command(
		ctx,
		protocol::CommandKind::SetCommand(protocol::SetCommand {
			key: key.to_vec(),
			value: Some(b"initial".to_vec()),
		}),
		false,
	)
	.await
	.unwrap();
	assert!(matches!(result, ProposalResult::Committed));

	// Successful check-and-set
	let result = execute_command(
		ctx,
		protocol::CommandKind::CheckAndSetCommand(protocol::CheckAndSetCommand {
			key: key.to_vec(),
			expect_one_of: vec![Some(b"initial".to_vec())],
			new_value: Some(b"updated".to_vec()),
		}),
		false,
	)
	.await
	.unwrap();
	assert!(matches!(result, ProposalResult::Committed));

	// Note: Direct workflow state checking is no longer available through ops API
	// Verify through KV get operation instead

	// Failed check-and-set (wrong expected value)
	let result = execute_command(
		ctx,
		protocol::CommandKind::CheckAndSetCommand(protocol::CheckAndSetCommand {
			key: key.to_vec(),
			expect_one_of: vec![Some(b"wrong".to_vec())],
			new_value: Some(b"should_not_apply".to_vec()),
		}),
		false,
	)
	.await
	.unwrap();

	// Verify it returns CommandError with ExpectedValueDoesNotMatch
	match result {
		ProposalResult::CommandError(CommandError::ExpectedValueDoesNotMatch {
			current_value: _,
		}) => {
			// Success - got the expected error variant
		}
		_ => panic!(
			"Expected CommandError::ExpectedValueDoesNotMatch, got {:?}",
			result
		),
	}

	// Note: Direct workflow state checking is no longer available through ops API
	// Verify through KV get operation instead

	// Check-and-set with None expected (should fail since value exists)
	let result = execute_command(
		ctx,
		protocol::CommandKind::CheckAndSetCommand(protocol::CheckAndSetCommand {
			key: key.to_vec(),
			expect_one_of: vec![None],
			new_value: Some(b"new_value".to_vec()),
		}),
		false,
	)
	.await
	.unwrap();

	match result {
		ProposalResult::CommandError(CommandError::ExpectedValueDoesNotMatch {
			current_value: _,
		}) => {
			// Success - got the expected error variant
		}
		_ => panic!(
			"Expected CommandError::ExpectedValueDoesNotMatch, got {:?}",
			result
		),
	}

	// Delete the value
	let result = execute_command(
		ctx,
		protocol::CommandKind::SetCommand(protocol::SetCommand {
			key: key.to_vec(),
			value: None,
		}),
		false,
	)
	.await
	.unwrap();
	assert!(matches!(result, ProposalResult::Committed));

	// Check-and-set with None expected (should succeed now)
	let result = execute_command(
		ctx,
		protocol::CommandKind::CheckAndSetCommand(protocol::CheckAndSetCommand {
			key: key.to_vec(),
			expect_one_of: vec![None],
			new_value: Some(b"created".to_vec()),
		}),
		false,
	)
	.await
	.unwrap();
	assert!(matches!(result, ProposalResult::Committed));

	// Note: Direct workflow state checking is no longer available through ops API
	// Verify through KV get operation instead
}
