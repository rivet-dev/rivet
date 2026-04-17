use napi::bindgen_prelude::{Buffer, Promise};
use napi_derive::napi;
use rivetkit_core::{ActorContext as CoreActorContext, SaveStateOpts};

use crate::connection::ConnHandle;
use crate::kv::Kv;
use crate::queue::Queue;
use crate::schedule::Schedule;
use crate::sqlite_db::SqliteDb;
use crate::napi_error;

/// N-API wrapper around `rivetkit-core::ActorContext`.
#[napi]
pub struct ActorContext {
	inner: CoreActorContext,
}

impl ActorContext {
	pub(crate) fn new(inner: CoreActorContext) -> Self {
		Self { inner }
	}

	#[allow(dead_code)]
	pub(crate) fn inner(&self) -> &CoreActorContext {
		&self.inner
	}
}

#[napi]
impl ActorContext {
	#[napi(constructor)]
	pub fn constructor(actor_id: String, name: String, region: String) -> Self {
		Self::new(CoreActorContext::new(actor_id, name, Vec::new(), region))
	}

	#[napi]
	pub fn state(&self) -> Buffer {
		Buffer::from(self.inner.state())
	}

	#[napi]
	pub fn set_state(&self, state: Buffer) {
		self.inner.set_state(state.to_vec());
	}

	#[napi]
	pub fn kv(&self) -> Kv {
		Kv::new(self.inner.kv().clone())
	}

	#[napi]
	pub fn sql(&self) -> SqliteDb {
		SqliteDb::new(self.inner.clone())
	}

	#[napi]
	pub fn schedule(&self) -> Schedule {
		Schedule::new(self.inner.schedule().clone())
	}

	#[napi]
	pub fn queue(&self) -> Queue {
		Queue::new(self.inner.queue().clone())
	}

	#[napi]
	pub async fn save_state(&self, immediate: bool) -> napi::Result<()> {
		self.inner
			.save_state(SaveStateOpts { immediate })
			.await
			.map_err(napi_error)
	}

	#[napi]
	pub fn actor_id(&self) -> String {
		self.inner.actor_id().to_owned()
	}

	#[napi]
	pub fn name(&self) -> String {
		self.inner.name().to_owned()
	}

	#[napi]
	pub fn region(&self) -> String {
		self.inner.region().to_owned()
	}

	#[napi]
	pub fn sleep(&self) {
		self.inner.sleep();
	}

	#[napi]
	pub fn destroy(&self) {
		self.inner.destroy();
	}

	#[napi]
	pub fn set_prevent_sleep(&self, prevent_sleep: bool) {
		self.inner.set_prevent_sleep(prevent_sleep);
	}

	#[napi]
	pub fn prevent_sleep(&self) -> bool {
		self.inner.prevent_sleep()
	}

	#[napi]
	pub fn aborted(&self) -> bool {
		self.inner.aborted()
	}

	#[napi]
	pub fn conns(&self) -> Vec<ConnHandle> {
		self
			.inner
			.conns()
			.into_iter()
			.map(ConnHandle::new)
			.collect()
	}

	pub async fn wait_until(
		&self,
		promise: Promise<serde_json::Value>,
	) -> napi::Result<()> {
		self.inner.wait_until(async move {
			if let Err(error) = promise.await {
				tracing::warn!(?error, "actor wait_until promise rejected");
			}
		});
		Ok(())
	}
}
