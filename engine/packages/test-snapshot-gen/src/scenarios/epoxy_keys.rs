use anyhow::{Result, ensure};
use async_trait::async_trait;
use epoxy::ops::propose::{self, ProposalResult};
use epoxy_protocol::protocol::{self, Command, CommandKind, SetCommand};

use crate::test_cluster::TestCluster;

use super::Scenario;

/// Scenario that writes epoxy keys through the normal v1 consensus path.
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
			let result = leader_ctx
				.op(propose::Input {
					proposal: protocol::Proposal {
						commands: vec![Command {
							kind: CommandKind::SetCommand(SetCommand {
								key: key.to_vec(),
								value: Some(value.to_vec()),
							}),
						}],
					},
					purge_cache: false,
					target_replicas: None,
				})
				.await?;

			ensure!(
				matches!(result, ProposalResult::Committed),
				"proposal failed: {result:?}"
			);
		}

		tracing::info!(count = test_keys.len(), "wrote keys via consensus");

		Ok(())
	}
}
