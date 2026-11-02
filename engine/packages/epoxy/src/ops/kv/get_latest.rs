use crate::{client::Client, ops::EpoxyOperation};
use anyhow::Result;
use epoxy_protocol::protocol::CommittedValue;

#[derive(Debug)]
pub struct Input {
	pub key: Vec<u8>,
}

#[derive(Debug)]
pub struct Output {
	pub value: Option<CommittedValue>,
}

impl EpoxyOperation for Input {
	type Output = Output;

	async fn execute(&self, client: &Client) -> Result<Output> {
		epoxy_kv_get_latest(client, self).await
	}
}

#[tracing::instrument(skip_all)]
pub async fn epoxy_kv_get_latest(client: &Client, input: &Input) -> Result<Output> {
	Ok(Output {
		value: client.consensus().get(&input.key).await?,
	})
}
