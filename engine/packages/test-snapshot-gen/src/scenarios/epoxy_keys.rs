use anyhow::Result;
use async_trait::async_trait;
use epoxy::ops::propose::{self, Command, CommandKind, Proposal, SetCommand};

use crate::test_cluster::TestCluster;

use super::Scenario;

/// Scenario that writes epoxy keys through the normal consensus path.
/// When generated from a v1 branch, these keys live in the v1 format.
/// Tests can load this snapshot to verify migration/backfill to v2.
pub struct EpoxyKeys;

#[async_trait(?Send)]
impl Scenario for EpoxyKeys {
	fn name(&self) -> &'static str {
		"epoxy-v1"
	}

	fn replica_count(&self) -> usize {
		2
	}

	async fn populate(&self, cluster: &TestCluster) -> Result<()> {
		let leader_ctx = cluster.get_ctx(cluster.leader_id());

		let test_keys: &[(&[u8], &[u8])] = &[
			(b"actor:abc123", b"running"),
			(b"actor:def456", b"stopped"),
			(b"config:version", b"42"),
		];

		for &(key, value) in test_keys {
			leader_ctx
				.op(propose::Input {
					proposal: Proposal {
						commands: vec![Command {
							kind: CommandKind::SetCommand(SetCommand {
								key: key.to_vec(),
								value: Some(value.to_vec()),
							}),
						}],
					},
					mutable: false,
					purge_cache: false,
					target_replicas: None,
				})
				.await?
				.resolve()?;
		}

		tracing::info!(count = test_keys.len(), "wrote keys via consensus");

		Ok(())
	}
}
