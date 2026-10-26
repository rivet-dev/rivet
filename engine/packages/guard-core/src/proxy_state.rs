use rivet_engine_context::EngineContext;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Clone)]
pub struct ProxyState(Arc<ProxyStateInner>);

impl ProxyState {
	pub fn new(ctx: &EngineContext) -> Self {
		ProxyState(Arc::new(ProxyStateInner::new(ctx)))
	}
}

impl Deref for ProxyState {
	type Target = ProxyStateInner;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

pub struct ProxyStateInner {}

impl ProxyStateInner {
	pub fn new(_ctx: &EngineContext) -> Self {
		ProxyStateInner {}
	}
}
