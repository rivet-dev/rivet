use std::fmt;
use std::marker::PhantomData;

use rivetkit_core::{ActorContext, ConnHandle};

use crate::actor::Actor;

#[derive(Clone)]
pub struct Ctx<A: Actor> {
	inner: ActorContext,
	_phantom: PhantomData<fn() -> A>,
}

impl<A: Actor> Ctx<A> {
	pub fn new(inner: ActorContext) -> Self {
		Self {
			inner,
			_phantom: PhantomData,
		}
	}

	pub fn inner(&self) -> &ActorContext {
		&self.inner
	}

	pub fn into_inner(self) -> ActorContext {
		self.inner
	}
}

impl<A: Actor> From<ActorContext> for Ctx<A> {
	fn from(value: ActorContext) -> Self {
		Self::new(value)
	}
}

impl<A: Actor> fmt::Debug for Ctx<A> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Ctx").field("inner", &self.inner).finish()
	}
}

#[derive(Clone)]
pub struct ConnCtx<A: Actor> {
	inner: ConnHandle,
	_phantom: PhantomData<fn() -> A>,
}

impl<A: Actor> ConnCtx<A> {
	pub fn new(inner: ConnHandle) -> Self {
		Self {
			inner,
			_phantom: PhantomData,
		}
	}

	pub fn inner(&self) -> &ConnHandle {
		&self.inner
	}

	pub fn into_inner(self) -> ConnHandle {
		self.inner
	}
}

impl<A: Actor> From<ConnHandle> for ConnCtx<A> {
	fn from(value: ConnHandle) -> Self {
		Self::new(value)
	}
}

impl<A: Actor> fmt::Debug for ConnCtx<A> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("ConnCtx").field("inner", &self.inner).finish()
	}
}
