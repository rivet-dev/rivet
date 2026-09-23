use anyhow::*;
use std::{ops::Deref, sync::Arc};
use universalpubsub::PubSub;

#[derive(Clone)]
pub struct SharedState(Arc<SharedStateInner>);

impl SharedState {
	pub fn new(ctx: &gas::prelude::StandaloneCtx, pubsub: PubSub) -> Result<SharedState> {
		let config = ctx.config();
		let jwt_key_ring_cache =
			config
				.auth
				.as_ref()
				.filter(|auth| auth.jwt.enabled())
				.map(|auth| {
					rivet_auth_jwt::key_ring_cache::KeyRingCache::new(
						ctx.clone(),
						if auth.jwt.issuance_enabled() {
							rivet_auth_jwt::key_ring_cache::KeyRingCacheMode::SignAndVerify
						} else {
							rivet_auth_jwt::key_ring_cache::KeyRingCacheMode::VerifyOnly
						},
					)
				});
		let jwt_key_ring_cache = jwt_key_ring_cache.transpose()?;
		Ok(SharedState(Arc::new(SharedStateInner {
			pegboard_gateway: pegboard_gateway::shared_state::SharedState::new(
				config,
				pubsub.clone(),
			),
			pegboard_gateway2: pegboard_gateway2::shared_state::SharedState::new(
				config,
				pubsub.clone(),
			),
			pegboard_gateway3: pegboard_gateway3::shared_state::SharedState::new(config, pubsub),
			jwt_key_ring_cache,
		})))
	}

	pub async fn start(&self) -> Result<()> {
		tokio::try_join!(
			self.pegboard_gateway.start(),
			self.pegboard_gateway2.start(),
			self.pegboard_gateway3.start(),
		)?;
		if let Some(jwt_key_ring_cache) = &self.jwt_key_ring_cache {
			jwt_key_ring_cache.start().await?;
		}

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
	pub pegboard_gateway2: pegboard_gateway2::shared_state::SharedState,
	pub pegboard_gateway3: pegboard_gateway3::shared_state::SharedState,
	pub jwt_key_ring_cache: Option<Arc<rivet_auth_jwt::key_ring_cache::KeyRingCache>>,
}
