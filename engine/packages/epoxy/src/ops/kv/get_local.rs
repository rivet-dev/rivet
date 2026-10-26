use crate::{client::Client, ops::EpoxyOperation};
use anyhow::{Result, ensure};
use epoxy_protocol::protocol::{CommittedValue, ReplicaId};

#[derive(Debug)]
pub struct Input {
	pub replica_id: ReplicaId,
	pub key: Vec<u8>,
}

impl EpoxyOperation for Input {
	type Output = Option<CommittedValue>;

	async fn execute(&self, client: &Client) -> Result<Self::Output> {
		epoxy_kv_get_local(client, self).await
	}
}

#[tracing::instrument(skip_all)]
pub async fn epoxy_kv_get_local(client: &Client, input: &Input) -> Result<Option<CommittedValue>> {
	ensure!(
		input.replica_id == client.replica_id(),
		"local Epoxy read addressed to another replica"
	);
	client.replica.get(&input.key).await
}
