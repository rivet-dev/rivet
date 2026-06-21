//! Rust-native actor wrapper around `agent-os-client`.
//!
//! Exposes a single `build_core_factory(config) -> CoreActorFactory`
//! consumed by the NAPI binding (`rivetkit-typescript/packages/rivetkit-napi`).
//! The factory's entry function drives the actor's event loop, brings up
//! an Agent OS VM lazily on first action, and tears it down on Sleep /
//! Destroy.

pub mod actions;
pub mod actor;
pub mod config;
pub mod persistence;
pub mod run;

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use futures::future::BoxFuture;
use rivet_error::RivetError;
use rivetkit::start::wrap_start;
use rivetkit_core::{ActorConfig, ActorFactory as CoreActorFactory, ActorStart};

pub use actor::AgentOsActor;
pub use config::AgentOsActorConfig;

/// Build a [`CoreActorFactory`] that runs the agent-os actor with the
/// given config. The factory's entry function captures the config via
/// `Arc` so multiple actor instances share the same builder.
pub fn build_core_factory(config: AgentOsActorConfig) -> CoreActorFactory {
	let config = Arc::new(config);
	let actor_config = ActorConfig {
		has_database: true,
		// Match the legacy TS actor's timeouts so long-running prompts
		// and slow shutdowns don't get cut off prematurely.
		sleep_grace_period: Duration::from_millis(900_000),
		sleep_grace_period_overridden: true,
		action_timeout: Duration::from_millis(900_000),
		..ActorConfig::default()
	};
	CoreActorFactory::new_with_manual_startup_ready(actor_config, move |core_start: ActorStart| {
		let config = config.clone();
		Box::pin(async move {
			let mut core_start = core_start;
			let startup_ready = core_start.startup_ready.take();
			match wrap_start::<AgentOsActor>(core_start) {
				Ok(start) => {
					if let Some(reply) = startup_ready {
						let _ = reply.send(Ok(()));
					}
					run::run(config, start).await
				}
				Err(error) => {
					if let Some(reply) = startup_ready {
						let startup_error = anyhow::Error::new(RivetError::extract(&error));
						let _ = reply.send(Err(startup_error));
					}
					Err(error)
				}
			}
		}) as BoxFuture<'static, Result<()>>
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn core_factory_enables_actor_database() {
		let factory = build_core_factory(AgentOsActorConfig::default());
		assert!(factory.config().has_database);
	}
}
