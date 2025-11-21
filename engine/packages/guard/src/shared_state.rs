use anyhow::*;
use std::{ops::Deref, sync::Arc};
use universalpubsub::PubSub;

#[derive(Clone)]
pub struct SharedState(Arc<SharedStateInner>);

impl SharedState {
	pub fn new(config: &rivet_config::Config, pubsub: PubSub) -> SharedState {
		SharedState(Arc::new(SharedStateInner {
			pegboard_gateway: pegboard_gateway::shared_state::SharedState::new(config, pubsub),
		}))
	}

	pub async fn start(&self) -> Result<()> {
		self.pegboard_gateway.start().await?;
		Ok(())
	}
}

impl Deref for SharedState {
	type Target = SharedStateInner;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

pub struct SharedStateInner {
	pub pegboard_gateway: pegboard_gateway::shared_state::SharedState,
}
