use serde::{de::DeserializeOwned, Serialize};

pub trait Actor: Send + 'static {
	type Input: DeserializeOwned + Send + 'static;
	type ConnParams: DeserializeOwned + Send + Sync + 'static;
	type ConnState: Serialize + DeserializeOwned + Send + Sync + Clone + 'static;
	type Action: DeserializeOwned + Send + 'static;
}

#[cfg(test)]
mod tests {
	use super::Actor;
	use crate::action;

	struct EmptyActor;

	impl Actor for EmptyActor {
		type Input = ();
		type ConnParams = ();
		type ConnState = ();
		type Action = action::Raw;
	}

	fn assert_actor<A: Actor>() {}

	#[test]
	fn empty_actor_impl_compiles() {
		assert_actor::<EmptyActor>();
	}
}
