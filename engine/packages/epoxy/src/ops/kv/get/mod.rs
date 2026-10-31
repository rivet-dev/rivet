//! Reads expose explicit consistency guarantees, including unfinished owner writes.

use crate::{client::Client, ops::EpoxyOperation};
use anyhow::Result;
use epoxy_protocol::protocol::{CommittedValue, ReplicaId};

// Every key's voting scope is this datacenter's replica, so every mode reads that replica's
// record; the modes keep the guarantees a multi-region deployment will need to honor.

#[derive(Debug, Clone)]
pub enum ReadMode {
	/// Committed state at one replica. `pending_write` prevents an owner read from hiding
	/// an unfinished proposal. This mode alone makes no freshness guarantee.
	LocalCommitted { replica_id: ReplicaId },
	/// Newest committed value among reachable replicas, possibly stale.
	LatestReachable,
	/// The key's newest committed value.
	Linearizable,
	/// Only for values that never change after creation.
	OptimisticImmutable,
}

pub struct Input {
	pub key: Vec<u8>,
	pub mode: ReadMode,
}

impl std::fmt::Debug for Input {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Input")
			.field("key_bytes", &self.key.len())
			.field("mode", &self.mode)
			.finish()
	}
}

pub struct Output {
	pub value: Option<CommittedValue>,
	pub pending_write: bool,
}

impl std::fmt::Debug for Output {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Output")
			.field("version", &self.value.as_ref().map(|v| v.version))
			.field("pending_write", &self.pending_write)
			.finish_non_exhaustive()
	}
}

impl EpoxyOperation for Input {
	type Output = Output;

	async fn execute(&self, client: &Client) -> Result<Output> {
		epoxy_kv_get(client, self).await
	}
}

#[tracing::instrument(skip_all)]
pub async fn epoxy_kv_get(client: &Client, input: &Input) -> Result<Output> {
	rivet_env::timeout(
		client.env().clock().as_ref(),
		std::time::Duration::from_secs(10),
		async {
			let consensus = client.consensus();
			let value = match &input.mode {
				ReadMode::LocalCommitted { replica_id } => {
					let state = consensus.get_owner(&input.key, *replica_id).await?;
					return Ok(Output {
						value: state.committed,
						pending_write: state.accepted.is_some(),
					});
				}
				ReadMode::LatestReachable
				| ReadMode::Linearizable
				| ReadMode::OptimisticImmutable => consensus.get(&input.key).await?,
			};
			Ok(Output {
				value,
				pending_write: false,
			})
		},
	)
	.await?
}
