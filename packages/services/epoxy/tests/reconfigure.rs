use anyhow::*;
use common::{TestCtx, utils::execute_command};
use epoxy::ops::propose::ProposalResult;
use epoxy::types;
use epoxy_protocol::protocol::{self, ReplicaId};
use futures_util::TryStreamExt;
use gas::prelude::*;
use serde_json::json;
use std::collections::HashSet;
use udb_util::prelude::*;
use universaldb::{FdbBindingError, KeySelector, RangeOption, options::StreamingMode};

mod common;

/// Helper function to generate SetCommand operations from key-value pairs
fn generate_set_commands(keys: &[(Vec<u8>, Vec<u8>)]) -> Vec<protocol::CommandKind> {
	keys.iter()
		.map(|(key, value)| {
			protocol::CommandKind::SetCommand(protocol::SetCommand {
				key: key.clone(),
				value: Some(value.clone()),
			})
		})
		.collect()
}

/// Tests simple process of adding 1 ndoe
#[tokio::test]
async fn reconfig_1_to_2() {
	let keys = vec![
		(b"test_key_1".to_vec(), b"test_value_1".to_vec()),
		(b"test_key_2".to_vec(), b"test_value_2".to_vec()),
		(b"test_key_3".to_vec(), b"test_value_3".to_vec()),
	];
	let commands = generate_set_commands(&keys);

	test_inner(TestConfig {
		expected_keys: keys,
		commands,
		init_replica_count: 1,
		new_replica_count: 1,
	})
	.await
}

/// Tests downloading data from multiple nodes
#[tokio::test]
async fn reconfig_3_to_4() {
	let keys = vec![
		(b"test_key_1".to_vec(), b"test_value_1".to_vec()),
		(b"test_key_2".to_vec(), b"test_value_2".to_vec()),
		(b"test_key_3".to_vec(), b"test_value_3".to_vec()),
	];
	let commands = generate_set_commands(&keys);

	test_inner(TestConfig {
		expected_keys: keys,
		commands,
		init_replica_count: 3,
		new_replica_count: 1,
	})
	.await
}

/// Tests adding multiple nodes at once
#[tokio::test]
async fn reconfig_3_to_5() {
	let keys = vec![
		(b"test_key_1".to_vec(), b"test_value_1".to_vec()),
		(b"test_key_2".to_vec(), b"test_value_2".to_vec()),
		(b"test_key_3".to_vec(), b"test_value_3".to_vec()),
	];
	let commands = generate_set_commands(&keys);

	test_inner(TestConfig {
		expected_keys: keys,
		commands,
		init_replica_count: 3,
		new_replica_count: 2,
	})
	.await
}

/// Tests dependent operations with check and set
#[tokio::test]
async fn reconfig_check_and_set() {
	let test_key = b"counter".to_vec();

	// Create commands that perform dependent check-and-set operations
	let commands = vec![
		// Initialize the counter
		protocol::CommandKind::SetCommand(protocol::SetCommand {
			key: test_key.clone(),
			value: Some(b"0".to_vec()),
		}),
		// Update if value is 0
		protocol::CommandKind::CheckAndSetCommand(protocol::CheckAndSetCommand {
			key: test_key.clone(),
			expect_one_of: vec![Some(b"0".to_vec())],
			new_value: Some(b"1".to_vec()),
		}),
		// Update if value is 1
		protocol::CommandKind::CheckAndSetCommand(protocol::CheckAndSetCommand {
			key: test_key.clone(),
			expect_one_of: vec![Some(b"1".to_vec())],
			new_value: Some(b"2".to_vec()),
		}),
		// Update if value is 2
		protocol::CommandKind::CheckAndSetCommand(protocol::CheckAndSetCommand {
			key: test_key.clone(),
			expect_one_of: vec![Some(b"2".to_vec())],
			new_value: Some(b"3".to_vec()),
		}),
		// Add other keys for verification
		protocol::CommandKind::SetCommand(protocol::SetCommand {
			key: b"test_key_1".to_vec(),
			value: Some(b"test_value_1".to_vec()),
		}),
		protocol::CommandKind::SetCommand(protocol::SetCommand {
			key: b"test_key_2".to_vec(),
			value: Some(b"test_value_2".to_vec()),
		}),
	];

	let expected_keys = vec![
		(test_key.clone(), b"3".to_vec()),
		(b"test_key_1".to_vec(), b"test_value_1".to_vec()),
		(b"test_key_2".to_vec(), b"test_value_2".to_vec()),
	];

	test_inner(TestConfig {
		expected_keys,
		commands,
		init_replica_count: 3,
		new_replica_count: 1,
	})
	.await
}

/// Tests changing the same value multiple times with set
#[tokio::test]
async fn reconfig_repeated_sets() {
	let test_key = b"version".to_vec();

	// Create commands that repeatedly update the same key
	let commands = vec![
		// Initial value
		protocol::CommandKind::SetCommand(protocol::SetCommand {
			key: test_key.clone(),
			value: Some(b"v1.0.0".to_vec()),
		}),
		// Update to v1.1.0
		protocol::CommandKind::SetCommand(protocol::SetCommand {
			key: test_key.clone(),
			value: Some(b"v1.1.0".to_vec()),
		}),
		// Update to v1.2.0
		protocol::CommandKind::SetCommand(protocol::SetCommand {
			key: test_key.clone(),
			value: Some(b"v1.2.0".to_vec()),
		}),
		// Update to v2.0.0
		protocol::CommandKind::SetCommand(protocol::SetCommand {
			key: test_key.clone(),
			value: Some(b"v2.0.0".to_vec()),
		}),
		// Final update to v2.1.0
		protocol::CommandKind::SetCommand(protocol::SetCommand {
			key: test_key.clone(),
			value: Some(b"v2.1.0".to_vec()),
		}),
		// Add other keys for more comprehensive test
		protocol::CommandKind::SetCommand(protocol::SetCommand {
			key: b"config_name".to_vec(),
			value: Some(b"production".to_vec()),
		}),
		protocol::CommandKind::SetCommand(protocol::SetCommand {
			key: b"status".to_vec(),
			value: Some(b"active".to_vec()),
		}),
	];

	let expected_keys = vec![
		(test_key.clone(), b"v2.1.0".to_vec()),
		(b"config_name".to_vec(), b"production".to_vec()),
		(b"status".to_vec(), b"active".to_vec()),
	];

	test_inner(TestConfig {
		expected_keys,
		commands,
		init_replica_count: 3,
		new_replica_count: 1,
	})
	.await
}

/// Tests recovering key chunks
#[tokio::test]
async fn reconfig_recover_chunking() {
	// Generate keys larger than the
	let keys = (0..epoxy::consts::RECOVER_KEY_CHUNK_SIZE * 2 + 10)
		.map(|i| {
			(
				format!("test_key_{i}").into_bytes(),
				format!("test_value_{i}").into_bytes(),
			)
		})
		.collect::<Vec<_>>();

	let commands = generate_set_commands(&keys);

	test_inner(TestConfig {
		expected_keys: keys,
		commands,
		init_replica_count: 1,
		new_replica_count: 1,
	})
	.await
}

/// Tests loading multiple chunks
///
/// (This test takes a long time)
#[tokio::test]
#[ignore]
async fn reconfig_download_chunking() {
	// Generate keys larger than the
	let keys = (0..epoxy::consts::DOWNLOAD_INSTANCE_COUNT + 10)
		.map(|i| {
			(
				format!("test_key_{i}").into_bytes(),
				format!("test_value_{i}").into_bytes(),
			)
		})
		.collect::<Vec<_>>();

	let commands = generate_set_commands(&keys);

	test_inner(TestConfig {
		expected_keys: keys,
		commands,
		init_replica_count: 1,
		new_replica_count: 1,
	})
	.await
}

struct TestConfig {
	expected_keys: Vec<(Vec<u8>, Vec<u8>)>,
	commands: Vec<protocol::CommandKind>,
	init_replica_count: ReplicaId,
	new_replica_count: ReplicaId,
}

async fn test_inner(config: TestConfig) {
	tracing::info!("Starting test_reconfigure");

	// Setup initial replicas
	let mut epoch = config.init_replica_count as u64;
	let init_replica_ids = (1..=(config.init_replica_count)).collect::<Vec<ReplicaId>>();
	let mut test_ctx = TestCtx::new_with(&init_replica_ids).await.unwrap();
	let leader_replica_id = test_ctx.leader_id;
	tracing::info!(?init_replica_ids, "started init replicas");

	verify_configuration_propagated(&test_ctx, epoch)
		.await
		.unwrap();

	// Execute commands on replica
	tracing::info!("executing commands on leader replica");
	let leader_ctx = test_ctx.get_ctx(leader_replica_id);
	for (i, command) in config.commands.iter().enumerate() {
		let result = execute_command(leader_ctx, command.clone(), true)
			.await
			.unwrap();
		assert!(matches!(result, ProposalResult::Committed));
		tracing::info!(
			progress = format!("{}/{}", i + 1, config.commands.len()),
			"executed command"
		);
	}

	// Stop the leader
	tracing::info!(leader_replica_id, "stopping leader replica");
	test_ctx
		.stop_replica(leader_replica_id, true)
		.await
		.unwrap();

	// Add a new replicas to the cluster
	let new_replica_ids = ((config.init_replica_count + 1)
		..(config.init_replica_count + config.new_replica_count + 1))
		.collect::<Vec<ReplicaId>>();
	tracing::info!(?new_replica_ids, "starting new replicas");
	let all_replica_ids = init_replica_ids
		.iter()
		.chain(new_replica_ids.iter())
		.cloned()
		.collect::<HashSet<ReplicaId>>();
	for new_replica_id in &new_replica_ids {
		tracing::info!(new_replica_id, "adding new replica to cluster");
		test_ctx.add_replica(*new_replica_id).await.unwrap();
		test_ctx.start_replica(*new_replica_id).await.unwrap();
	}

	// Start the leader. This will start with the new replica configuration.
	tracing::info!(
		leader_replica_id,
		"restarting leader replica with new config including new replica"
	);
	test_ctx.start_replica(leader_replica_id).await.unwrap();

	// Trigger reconfiguration
	let leader_ctx = test_ctx.get_ctx(leader_replica_id);
	tracing::info!("manually triggering reconfiguration");
	let mut config_sub = leader_ctx
		.subscribe::<epoxy::workflows::coordinator::ConfigChangeMessage>(
			json!({ "replica": leader_replica_id }),
		)
		.await
		.unwrap();
	leader_ctx
		.signal(epoxy::workflows::coordinator::ReconfigureSignal {})
		.to_workflow_id(test_ctx.coordinator_workflow_id)
		.send()
		.await
		.unwrap();
	tracing::info!("sent reconfigure signal, waiting for completion");

	// Wait for all reconfigurations to finish
	loop {
		let config_msg = config_sub.next().await.unwrap();

		epoch += 1;
		assert_eq!(config_msg.config.epoch, epoch, "epoch should increment");

		let config_replica_ids = config_msg
			.config
			.replicas
			.iter()
			.map(|x| x.replica_id)
			.collect::<HashSet<ReplicaId>>();
		assert_eq!(
			all_replica_ids, config_replica_ids,
			"should have all replicas in config"
		);

		let active_replicas = config_msg
			.config
			.replicas
			.iter()
			.filter(|r| r.status == types::ReplicaStatus::Active)
			.map(|r| r.replica_id)
			.collect::<Vec<_>>();
		tracing::info!(
			progress = format!("{}/{}", active_replicas.len(), all_replica_ids.len()),
			?active_replicas,
			"replica join progress"
		);
		if active_replicas.len() == all_replica_ids.len() {
			break;
		}
	}

	for new_replica_id in &new_replica_ids {
		// Verify that the configuration was successfully propagated to all replicas (including new one)
		verify_configuration_propagated(&test_ctx, epoch)
			.await
			.unwrap();

		// Verify log entries match between replicas
		tracing::info!("verifying log entries match between replicas");
		verify_log_entries_match(&test_ctx, leader_replica_id, *new_replica_id)
			.await
			.unwrap();

		// Verify that KV data and log entries have been replicated to replica 2
		tracing::info!("verifying KV data replication to replica 2");
		verify_kv_replication(&test_ctx, *new_replica_id, &config.expected_keys)
			.await
			.unwrap();
	}
}

/// Verify that all replicas have the expected configuration with the given epoch.
async fn verify_configuration_propagated(test_ctx: &TestCtx, expected_epoch: u64) -> Result<()> {
	tracing::info!(expected_epoch, "verifying configuration propagation");

	let replica_ids = test_ctx.replica_ids();
	for &replica_id in &replica_ids {
		let ctx = test_ctx.get_ctx(replica_id);

		let result = ctx
			.op(epoxy::ops::read_cluster_config::Input { replica_id })
			.await?;

		tracing::info!(
			replica_id,
			actual_epoch = result.config.epoch,
			expected_epoch,
			replica_count = result.config.replicas.len(),
			"replica config verification"
		);

		// Verify the epoch matches expectation
		if result.config.epoch != expected_epoch {
			return Err(anyhow!(
				"Replica {} has epoch {} but expected {}",
				replica_id,
				result.config.epoch,
				expected_epoch
			));
		}

		// Verify that the configuration includes all expected replicas
		let replica_count = result.config.replicas.len();
		if replica_count != replica_ids.len() {
			return Err(anyhow!(
				"Replica {} has {} replicas in config but expected {}",
				replica_id,
				replica_count,
				replica_ids.len()
			));
		}

		tracing::info!(
			replica_id,
			epoch = result.config.epoch,
			"replica config verified"
		);
	}

	tracing::info!(
		expected_epoch,
		"configuration propagation verified successfully"
	);
	Ok(())
}

/// Verify that log entries match between two replicas
async fn verify_log_entries_match(
	test_ctx: &TestCtx,
	replica_1_id: epoxy_protocol::protocol::ReplicaId,
	replica_2_id: epoxy_protocol::protocol::ReplicaId,
) -> Result<()> {
	tracing::info!(
		replica_1_id,
		replica_2_id,
		"verifying log entries match between replicas"
	);

	let ctx_1 = test_ctx.get_ctx(replica_1_id);
	let ctx_2 = test_ctx.get_ctx(replica_2_id);

	// Read log entries from replica 1
	let log_entries_1 = ctx_1
		.udb()?
		.run(move |tx, _| async move {
			let subspace = epoxy::keys::subspace(replica_1_id);

			// Range scan to get all log entries for this replica
			let range = RangeOption {
				begin: KeySelector::first_greater_or_equal(subspace.pack(&(LOG,))),
				end: KeySelector::first_greater_than(subspace.pack(&(LOG + 1,))),
				mode: StreamingMode::WantAll,
				..Default::default()
			};

			let mut stream = tx.get_ranges_keyvalues(range, SERIALIZABLE);
			let mut log_entries = Vec::new();

			while let Some(kv) = stream.try_next().await? {
				let key_bytes = kv.key();
				let value_bytes = kv.value();

				// Parse the key to get replica_id and slot_id
				let key = subspace
					.unpack::<epoxy::keys::replica::LogEntryKey>(key_bytes)
					.map_err(|x| FdbBindingError::CustomError(x.into()))?;

				// Deserialize the log entry
				let log_entry = epoxy::keys::replica::LogEntryKey::new(
					key.instance_replica_id,
					key.instance_slot_id,
				)
				.deserialize(value_bytes)
				.map_err(|e| FdbBindingError::CustomError(e.into()))?;

				log_entries.push((key.instance_replica_id, key.instance_slot_id, log_entry));
			}

			Result::<_, FdbBindingError>::Ok(log_entries)
		})
		.await?;

	// Read log entries from replica 2
	let log_entries_2 = ctx_2
		.udb()?
		.run(move |tx, _| async move {
			let subspace = epoxy::keys::subspace(replica_2_id);

			// Range scan to get all log entries for this replica
			let range = RangeOption {
				begin: KeySelector::first_greater_or_equal(subspace.pack(&(LOG,))),
				end: KeySelector::first_greater_than(subspace.pack(&(LOG + 1,))),
				mode: StreamingMode::WantAll,
				..Default::default()
			};

			let mut stream = tx.get_ranges_keyvalues(range, SERIALIZABLE);
			let mut log_entries = Vec::new();

			while let Some(kv) = stream.try_next().await? {
				let key_bytes = kv.key();
				let value_bytes = kv.value();

				// Parse the key to get replica_id and slot_id
				let key = subspace
					.unpack::<epoxy::keys::replica::LogEntryKey>(key_bytes)
					.map_err(|x| FdbBindingError::CustomError(x.into()))?;

				// Deserialize the log entry
				let log_entry = epoxy::keys::replica::LogEntryKey::new(
					key.instance_replica_id,
					key.instance_slot_id,
				)
				.deserialize(value_bytes)
				.map_err(|e| FdbBindingError::CustomError(e.into()))?;

				log_entries.push((key.instance_replica_id, key.instance_slot_id, log_entry));
			}

			Result::<_, FdbBindingError>::Ok(log_entries)
		})
		.await?;

	tracing::info!(
		replica_1_count = log_entries_1.len(),
		replica_2_count = log_entries_2.len(),
		"log entry counts"
	);

	// Both replicas should have the same number of log entries
	assert_eq!(
		log_entries_1.len(),
		log_entries_2.len(),
		"Log entry count mismatch between replicas"
	);

	// Verify each log entry matches
	for (entry_1, entry_2) in log_entries_1.iter().zip(log_entries_2.iter()) {
		let (replica_1, slot_1, log_1) = entry_1;
		let (replica_2, slot_2, log_2) = entry_2;

		// The instance IDs should match
		assert_eq!(replica_1, replica_2, "Instance replica ID mismatch");
		assert_eq!(slot_1, slot_2, "Instance slot ID mismatch");

		// Log entries should be equivalent
		// Note: We're comparing the high-level structure, you may want to add more detailed comparison
		assert_eq!(
			log_1.state, log_2.state,
			"Log entry state mismatch for instance ({}, {})",
			replica_1, slot_1
		);

		tracing::info!(
			?replica_1,
			?slot_1,
			state = ?log_1.state,
			"log entry verified"
		);
	}

	tracing::info!("log entries match between replicas successfully");
	Ok(())
}

/// Verify that KV data has been replicated to the specified replica
async fn verify_kv_replication(
	test_ctx: &TestCtx,
	replica_id: epoxy_protocol::protocol::ReplicaId,
	expected_keys: &[(Vec<u8>, Vec<u8>)],
) -> Result<()> {
	tracing::info!(replica_id, "verifying KV data replication");

	let ctx = test_ctx.get_ctx(replica_id);

	// Access the UDB directly to verify the data
	let udb = ctx.udb()?;

	for (i, (key, expected_value)) in expected_keys.iter().enumerate() {
		// Read the KV value from the replica's UDB
		let actual_value = udb
			.run(move |tx, _| {
				let key_clone = key.clone();
				async move {
					let subspace = epoxy::keys::subspace(replica_id);
					let kv_key = epoxy::keys::keys::KvValueKey::new(key_clone);
					let result = tx.get(&subspace.pack(&kv_key), SERIALIZABLE).await?;

					// KvValueKey stores Vec<u8> directly, so we can return it as is
					Result::<_, FdbBindingError>::Ok(result)
				}
			})
			.await?;

		tracing::info!(
			progress = format!("{}/{}", i + 1, expected_keys.len()),
			"checked KV pair"
		);

		// Verify the value matches
		assert_eq!(
			actual_value,
			Some(expected_value.clone()),
			"KV value mismatch for key {} on replica {}",
			String::from_utf8_lossy(&key),
			replica_id
		);
	}

	tracing::info!(replica_id, "KV data replication verified successfully");
	Ok(())
}
