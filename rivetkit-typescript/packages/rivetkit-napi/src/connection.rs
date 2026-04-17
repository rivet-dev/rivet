use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use rivetkit_core::ConnHandle as CoreConnHandle;

use crate::napi_anyhow_error;

#[napi]
pub struct ConnHandle {
	inner: CoreConnHandle,
}

impl ConnHandle {
	pub(crate) fn new(inner: CoreConnHandle) -> Self {
		Self { inner }
	}
}

#[napi]
impl ConnHandle {
	#[napi]
	pub fn id(&self) -> String {
		self.inner.id().to_owned()
	}

	#[napi]
	pub fn params(&self) -> Buffer {
		Buffer::from(self.inner.params())
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
	pub fn is_hibernatable(&self) -> bool {
		self.inner.is_hibernatable()
	}

	#[napi]
	pub fn send(&self, name: String, args: Buffer) {
		self.inner.send(&name, args.as_ref());
	}

	#[napi]
	pub async fn disconnect(&self, reason: Option<String>) -> napi::Result<()> {
		self.inner
			.disconnect(reason.as_deref())
			.await
			.map_err(napi_anyhow_error)
	}
}
