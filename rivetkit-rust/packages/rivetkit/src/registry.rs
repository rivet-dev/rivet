use std::{future::Future, sync::Arc};

use anyhow::Result;
use rivetkit_core::{
	ActorConfig, ActorFactory as CoreActorFactory, ActorStart, CoreRegistry, ServeConfig,
};

use crate::{
	actor::Actor,
	start::{wrap_start, Start},
};

pub struct Registry {
	inner: CoreRegistry,
}

impl Registry {
	pub fn new() -> Self {
		Self {
			inner: CoreRegistry::new(),
		}
	}

	pub fn register<A, F, Fut>(&mut self, name: &str, entry: F) -> &mut Self
	where
		A: Actor,
		F: Fn(Start<A>) -> Fut + Send + Sync + 'static,
		Fut: Future<Output = Result<()>> + Send + 'static,
	{
		self.register_with::<A, F, Fut>(name, ActorConfig::default(), entry)
	}

	pub fn register_with<A, F, Fut>(
		&mut self,
		name: &str,
		config: ActorConfig,
		entry: F,
	) -> &mut Self
	where
		A: Actor,
		F: Fn(Start<A>) -> Fut + Send + Sync + 'static,
		Fut: Future<Output = Result<()>> + Send + 'static,
	{
		self.inner
			.register(name, build_factory::<A, F, Fut>(config, entry));
		self
	}

	pub async fn serve(self) -> Result<()> {
		self.inner.serve().await
	}

	pub async fn serve_with_config(self, config: ServeConfig) -> Result<()> {
		self.inner.serve_with_config(config).await
	}
}

impl Default for Registry {
	fn default() -> Self {
		Self::new()
	}
}

fn build_factory<A, F, Fut>(config: ActorConfig, entry: F) -> CoreActorFactory
where
	A: Actor,
	F: Fn(Start<A>) -> Fut + Send + Sync + 'static,
	Fut: Future<Output = Result<()>> + Send + 'static,
{
	let entry = Arc::new(entry);
	CoreActorFactory::new(config, move |core_start: ActorStart| {
		let entry = Arc::clone(&entry);
		Box::pin(async move {
			let start = wrap_start::<A>(core_start)?;
			entry(start).await
		})
	})
}

#[cfg(test)]
mod tests {
	use tokio::sync::mpsc;

	use super::*;
	use crate::action;

	struct EmptyActor;

	impl Actor for EmptyActor {
		type Input = ();
		type ConnParams = ();
		type ConnState = ();
		type Action = action::Raw;
	}

	async fn drain_events(mut start: Start<EmptyActor>) -> Result<()> {
		while start.events.recv().await.is_some() {}
		Ok(())
	}

	#[tokio::test]
	async fn registry_bridge_wraps_core_start_and_runs_typed_entry() {
		let mut registry = Registry::new();
		registry
			.register::<EmptyActor, _, _>("empty-default", drain_events)
			.register_with::<EmptyActor, _, _>(
				"empty-custom",
				ActorConfig::default(),
				drain_events,
			);

		let factory = build_factory::<EmptyActor, _, _>(ActorConfig::default(), drain_events);
		let (event_tx, event_rx) = mpsc::channel(1);
		drop(event_tx);

		let result = factory
			.start(ActorStart {
				ctx: rivetkit_core::ActorContext::new("actor-id", "empty", Vec::new(), "local"),
				input: None,
				snapshot: None,
				hibernated: Vec::new(),
				events: event_rx.into(),
			})
			.await;

		assert!(result.is_ok());
	}
}
