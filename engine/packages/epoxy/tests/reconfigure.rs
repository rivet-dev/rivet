use anyhow::{Result, anyhow};
use common::{
	TestCtx,
	utils::{execute_command, read_changelog_entries, read_v2_value},
};
use epoxy::ops::propose::{CommandKind, ProposalResult, SetCommand};
use epoxy_protocol::protocol::{ChangelogEntry, ReplicaId};
use gas::prelude::*;
use std::collections::HashSet;

mod common;

static TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn generate_set_commands(keys: &[(Vec<u8>, Vec<u8>)]) -> Vec<CommandKind> {
	keys.iter()
		.map(|(key, value)| {
			CommandKind::SetCommand(SetCommand {
				key: key.clone(),
				value: Some(value.clone()),
			})
		})
		.collect()
}

#[tokio::test]
async fn reconfigure_catches_up_mixed_commands() {
	let _guard = TEST_LOCK.lock().await;
	let expected_keys = vec![
		(b"set-key-1".to_vec(), b"set-value-1".to_vec()),
		(b"set-key-2".to_vec(), b"set-value-2".to_vec()),
		(b"cas-key-1".to_vec(), b"cas-value-1".to_vec()),
		(b"cas-key-2".to_vec(), b"cas-value-2".to_vec()),
	];

	let mut commands = generate_set_commands(&expected_keys[..2]);
	for (key, value) in &expected_keys[2..] {
		commands.push(CommandKind::CheckAndSetCommand(
			epoxy::ops::propose::CheckAndSetCommand {
				key: key.clone(),
				expect_one_of: vec![None],
				new_value: Some(value.clone()),
			},
		));
	}

	test_inner(TestConfig {
		expected_keys,
		commands,
		init_replica_count: 3,
		new_replica_count: 2,
	})
	.await;
}

struct TestConfig {
	expected_keys: Vec<(Vec<u8>, Vec<u8>)>,
	commands: Vec<CommandKind>,
	init_replica_count: ReplicaId,
	new_replica_count: ReplicaId,
}

async fn test_inner(config: TestConfig) {
	// The workflow cutover starts fresh coordinator state, so the rebuilt cluster config begins at
	// epoch 0 instead of inheriting historical epoch counts from old workflow state.
	let mut epoch = 0;
	let init_replica_ids = (1..=config.init_replica_count).collect::<Vec<ReplicaId>>();
	let mut test_ctx = TestCtx::new_with(&init_replica_ids).await.unwrap();
	let leader_replica_id = test_ctx.leader_id;

	verify_configuration_propagated(&test_ctx, epoch)
		.await
		.unwrap();

	let leader_ctx = test_ctx.get_ctx(leader_replica_id);
	for command in &config.commands {
		let result = execute_command(leader_ctx, command.clone(), false)
			.await
			.unwrap();
		assert!(
			matches!(result, ProposalResult::Committed),
			"proposal failed during changelog catch-up setup: {result:?}"
		);
	}

	test_ctx
		.stop_replica(leader_replica_id, true)
		.await
		.unwrap();

	let new_replica_ids = ((config.init_replica_count + 1)
		..(config.init_replica_count + config.new_replica_count + 1))
		.collect::<Vec<ReplicaId>>();
	let all_replica_ids = init_replica_ids
		.iter()
		.chain(new_replica_ids.iter())
		.copied()
		.collect::<HashSet<_>>();

	for new_replica_id in &new_replica_ids {
		test_ctx.add_replica(*new_replica_id).await.unwrap();
		test_ctx.start_replica(*new_replica_id).await.unwrap();
	}

	test_ctx.start_replica(leader_replica_id).await.unwrap();

	let leader_ctx = test_ctx.get_ctx(leader_replica_id);
	let mut config_sub = leader_ctx
		.subscribe::<epoxy::workflows::coordinator::ConfigChangeMessage>((
			"replica",
			leader_replica_id,
		))
		.await
		.unwrap();

	leader_ctx
		.signal(epoxy::workflows::coordinator::Reconfigure {})
		.to_workflow_id(test_ctx.coordinator_workflow_id)
		.send()
		.await
		.unwrap();

	loop {
		let config_msg = config_sub.next().await.unwrap();
		epoch += 1;
		assert_eq!(config_msg.config.epoch, epoch, "epoch should increment");

		let config_replica_ids = config_msg
			.config
			.replicas
			.iter()
			.map(|replica| replica.replica_id)
			.collect::<HashSet<_>>();
		assert_eq!(all_replica_ids, config_replica_ids);

		if config_msg
			.config
			.replicas
			.iter()
			.all(|replica| replica.status == epoxy::types::ReplicaStatus::Active)
		{
			break;
		}
	}

	verify_configuration_propagated(&test_ctx, epoch)
		.await
		.unwrap();
	for new_replica_id in &new_replica_ids {
		verify_changelog_catch_up(&test_ctx, *new_replica_id, &config.expected_keys)
			.await
			.unwrap();
		verify_kv_replication(&test_ctx, *new_replica_id, &config.expected_keys)
			.await
			.unwrap();
	}

	test_ctx.shutdown().await.unwrap();
}

async fn verify_configuration_propagated(test_ctx: &TestCtx, expected_epoch: u64) -> Result<()> {
	for replica_id in test_ctx.replica_ids() {
		let ctx = test_ctx.get_ctx(replica_id);
		let result = ctx.op(epoxy::ops::read_cluster_config::Input {}).await?;

		if result.config.epoch != expected_epoch {
			return Err(anyhow!(
				"replica {} has epoch {} but expected {}",
				replica_id,
				result.config.epoch,
				expected_epoch
			));
		}

		if result.config.replicas.len() != test_ctx.replica_ids().len() {
			return Err(anyhow!(
				"replica {} has {} replicas in config but expected {}",
				replica_id,
				result.config.replicas.len(),
				test_ctx.replica_ids().len()
			));
		}
	}

	Ok(())
}

async fn verify_changelog_catch_up(
	test_ctx: &TestCtx,
	replica_id: ReplicaId,
	expected_keys: &[(Vec<u8>, Vec<u8>)],
) -> Result<()> {
	let changelog_entries =
		read_changelog_entries(test_ctx.get_ctx(replica_id), replica_id).await?;
	assert_eq!(
		changelog_entries.len(),
		expected_keys.len(),
		"replica {} should rebuild changelog entries during catch-up",
		replica_id
	);

	for (key, value) in expected_keys {
		assert!(
			changelog_entries.contains(&ChangelogEntry {
				key: key.clone(),
				value: Some(value.clone()),
				version: 1,
				mutable: false,
			}),
			"replica {} changelog is missing key {}",
			replica_id,
			String::from_utf8_lossy(key)
		);
	}
	Ok(())
}

async fn verify_kv_replication(
	test_ctx: &TestCtx,
	replica_id: ReplicaId,
	expected_keys: &[(Vec<u8>, Vec<u8>)],
) -> Result<()> {
	let ctx = test_ctx.get_ctx(replica_id);

	for (key, expected_value) in expected_keys {
		assert_eq!(
			read_v2_value(ctx, replica_id, key).await?,
			Some(expected_value.clone()),
			"replica {} is missing committed v2 value for key {}",
			replica_id,
			String::from_utf8_lossy(key)
		);
	}

	Ok(())
}
