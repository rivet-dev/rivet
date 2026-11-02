use crate::{client::Client, ops::EpoxyOperation};
use anyhow::Result;
use epoxy_protocol::protocol;

#[derive(Debug)]
pub struct Input {}

#[derive(Debug)]
pub struct Output {
	pub config: protocol::ClusterConfig,
}

impl EpoxyOperation for Input {
	type Output = Output;

	async fn execute(&self, client: &Client) -> Result<Output> {
		epoxy_read_cluster_config(client, self).await
	}
}

#[tracing::instrument(skip_all)]
pub async fn epoxy_read_cluster_config(client: &Client, _: &Input) -> Result<Output> {
	Ok(Output {
		config: client.cluster_config().await?,
	})
}
