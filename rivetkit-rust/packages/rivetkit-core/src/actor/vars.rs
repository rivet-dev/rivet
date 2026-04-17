use std::sync::Arc;
use std::sync::RwLock;

#[derive(Clone, Default)]
pub struct ActorVars(Arc<RwLock<Vec<u8>>>);

impl ActorVars {
	pub fn vars(&self) -> Vec<u8> {
		self.0.read().expect("actor vars lock poisoned").clone()
	}

	pub fn set_vars(&self, vars: Vec<u8>) {
		*self.0.write().expect("actor vars lock poisoned") = vars;
	}
}

impl std::fmt::Debug for ActorVars {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ActorVars")
			.field("len", &self.vars().len())
			.finish()
	}
}
