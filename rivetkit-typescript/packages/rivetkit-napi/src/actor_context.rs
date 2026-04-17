use anyhow::Error;
use napi::bindgen_prelude::{Buffer, Promise};
use napi_derive::napi;
use rivetkit_core::types::ActorKeySegment;
use rivetkit_core::{ActorContext as CoreActorContext, Request as CoreRequest, SaveStateOpts};

use crate::cancellation_token::CancellationToken;
use crate::connection::ConnHandle;
use crate::kv::Kv;
use crate::napi_anyhow_error;
use crate::queue::Queue;
use crate::schedule::Schedule;
use crate::sqlite_db::SqliteDb;

/// N-API wrapper around `rivetkit-core::ActorContext`.
#[napi]
pub struct ActorContext {
	inner: CoreActorContext,
}

#[napi(object)]
pub struct JsActorKeySegment {
	pub kind: String,
	pub string_value: Option<String>,
	pub number_value: Option<f64>,
}

#[napi(object)]
pub struct JsHttpRequest {
	pub method: String,
	pub uri: String,
	pub headers: Option<std::collections::HashMap<String, String>>,
	pub body: Option<Buffer>,
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
	pub fn vars(&self) -> Buffer {
		Buffer::from(self.inner.vars())
	}

	#[napi]
	pub fn set_state(&self, state: Buffer) {
		self.inner.set_state(state.to_vec());
	}

	#[napi]
	pub fn set_in_on_state_change_callback(&self, in_callback: bool) {
		self.inner.set_in_on_state_change_callback(in_callback);
	}

	#[napi]
	pub fn set_vars(&self, vars: Buffer) {
		self.inner.set_vars(vars.to_vec());
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
	pub fn set_alarm(&self, timestamp_ms: Option<i64>) -> napi::Result<()> {
		self.inner
			.set_alarm(timestamp_ms)
			.map_err(napi_anyhow_error)
	}

	#[napi]
	pub async fn save_state(&self, immediate: bool) -> napi::Result<()> {
		self.inner
			.save_state(SaveStateOpts { immediate })
			.await
			.map_err(napi_anyhow_error)
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
	pub fn key(&self) -> Vec<JsActorKeySegment> {
		self.inner
			.key()
			.iter()
			.map(|segment| match segment {
				ActorKeySegment::String(value) => JsActorKeySegment {
					kind: "string".to_owned(),
					string_value: Some(value.clone()),
					number_value: None,
				},
				ActorKeySegment::Number(value) => JsActorKeySegment {
					kind: "number".to_owned(),
					string_value: None,
					number_value: Some(*value),
				},
			})
			.collect()
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
	pub fn destroy_requested(&self) -> bool {
		self.inner.is_destroy_requested()
	}

	#[napi]
	pub async fn wait_for_destroy_completion(&self) {
		self.inner.wait_for_destroy_completion_public().await;
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
	pub fn run_handler_active(&self) -> bool {
		self.inner.run_handler_active()
	}

	#[napi]
	pub fn restart_run_handler(&self) -> napi::Result<()> {
		self.inner.restart_run_handler().map_err(napi_anyhow_error)
	}

	#[napi]
	pub fn begin_websocket_callback(&self) {
		self.inner.begin_websocket_callback();
	}

	#[napi]
	pub fn end_websocket_callback(&self) {
		self.inner.end_websocket_callback();
	}

	#[napi]
	pub fn abort_signal(&self) -> CancellationToken {
		CancellationToken::new(self.inner.abort_signal().clone())
	}

	#[napi]
	pub fn conns(&self) -> Vec<ConnHandle> {
		self.inner
			.conns()
			.into_iter()
			.map(ConnHandle::new)
			.collect()
	}

	#[napi]
	pub async fn connect_conn(
		&self,
		params: Buffer,
		request: Option<JsHttpRequest>,
	) -> napi::Result<ConnHandle> {
		let request = request.map(js_http_request_to_core_request).transpose()?;
		let conn = self
			.inner
			.connect_conn_with_request(params.to_vec(), request, async {
				Ok::<Vec<u8>, Error>(Vec::new())
			})
			.await
			.map_err(napi_anyhow_error)?;
		Ok(ConnHandle::new(conn))
	}

	#[napi]
	pub fn broadcast(&self, name: String, args: Buffer) {
		self.inner.broadcast(&name, args.as_ref());
	}

	#[napi]
	pub async fn wait_until(&self, promise: Promise<serde_json::Value>) -> napi::Result<()> {
		self.inner.wait_until(async move {
			if let Err(error) = promise.await {
				tracing::warn!(?error, "actor wait_until promise rejected");
			}
		});
		Ok(())
	}
}

fn js_http_request_to_core_request(request: JsHttpRequest) -> napi::Result<CoreRequest> {
	CoreRequest::from_parts(
		&request.method,
		&request.uri,
		request.headers.unwrap_or_default(),
		request.body.map(|body| body.to_vec()).unwrap_or_default(),
	)
	.map_err(napi_anyhow_error)
}
