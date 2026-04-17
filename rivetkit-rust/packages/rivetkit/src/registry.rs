use std::marker::PhantomData;

use anyhow::Result;
use serde::Serialize;
use serde::de::DeserializeOwned;
use rivetkit_core::CoreRegistry;

use crate::actor::Actor;
use crate::bridge::{self, TypedActionMap};
use crate::context::Ctx;

#[derive(Debug, Default)]
pub struct Registry {
	inner: CoreRegistry,
}

impl Registry {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn register<A: Actor>(&mut self, name: &str) -> ActorRegistration<'_, A> {
		ActorRegistration::new(self, name)
	}

	pub async fn serve(self) -> Result<()> {
		self.inner.serve().await
	}
}

pub struct ActorRegistration<'a, A: Actor> {
	registry: &'a mut Registry,
	name: String,
	actions: TypedActionMap<A>,
	_phantom: PhantomData<A>,
}

impl<'a, A: Actor> ActorRegistration<'a, A> {
	fn new(registry: &'a mut Registry, name: &str) -> Self {
		Self {
			registry,
			name: name.to_owned(),
			actions: TypedActionMap::new(),
			_phantom: PhantomData,
		}
	}

	pub fn action<Args, Ret, F, Fut>(
		&mut self,
		name: &str,
		handler: F,
	) -> &mut Self
	where
		Args: DeserializeOwned + Send + 'static,
		Ret: Serialize + Send + 'static,
		F: Fn(std::sync::Arc<A>, Ctx<A>, Args) -> Fut + Send + Sync + 'static,
		Fut: std::future::Future<Output = Result<Ret>> + Send + 'static,
	{
		self
			.actions
			.insert(name.to_owned(), bridge::build_action(handler));
		self
	}

	pub fn done(&mut self) -> &mut Registry {
		let factory = bridge::build_factory(std::mem::take(&mut self.actions));
		self.registry.inner.register(&self.name, factory);
		self.registry
	}
}
