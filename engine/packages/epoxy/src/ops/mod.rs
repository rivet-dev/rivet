//! Stable operation inputs and results for native callers.

use crate::client::Client;
use anyhow::Result;
use std::future::Future;
pub mod kv;
pub mod propose;
pub mod read_cluster_config;

pub trait EpoxyOperation: Sync {
	type Output: Send;

	fn execute(&self, client: &Client) -> impl Future<Output = Result<Self::Output>> + Send;
}
