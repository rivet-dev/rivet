use std::collections::HashMap;
use std::fmt;

use anyhow::Result;
use futures::future::BoxFuture;

use crate::actor::connection::ConnHandle;
use crate::actor::context::ActorContext;
use crate::websocket::WebSocket;

pub type Request = http::Request<Vec<u8>>;
pub type Response = http::Response<Vec<u8>>;

pub type LifecycleCallback<T> =
	Box<dyn Fn(T) -> BoxFuture<'static, Result<()>> + Send + Sync>;
pub type RequestCallback =
	Box<dyn Fn(OnRequestRequest) -> BoxFuture<'static, Result<Response>> + Send + Sync>;
pub type ActionHandler =
	Box<dyn Fn(ActionRequest) -> BoxFuture<'static, Result<Vec<u8>>> + Send + Sync>;
pub type BeforeActionResponseCallback = Box<
	dyn Fn(OnBeforeActionResponseRequest) -> BoxFuture<'static, Result<Vec<u8>>> + Send + Sync,
>;

#[derive(Clone, Debug)]
pub struct OnWakeRequest {
	pub ctx: ActorContext,
}

#[derive(Clone, Debug)]
pub struct OnSleepRequest {
	pub ctx: ActorContext,
}

#[derive(Clone, Debug)]
pub struct OnDestroyRequest {
	pub ctx: ActorContext,
}

#[derive(Clone, Debug)]
pub struct OnStateChangeRequest {
	pub ctx: ActorContext,
	pub new_state: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct OnRequestRequest {
	pub ctx: ActorContext,
	pub request: Request,
}

#[derive(Clone, Debug)]
pub struct OnWebSocketRequest {
	pub ctx: ActorContext,
	pub ws: WebSocket,
}

#[derive(Clone, Debug)]
pub struct OnBeforeConnectRequest {
	pub ctx: ActorContext,
	pub params: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct OnConnectRequest {
	pub ctx: ActorContext,
	pub conn: ConnHandle,
}

#[derive(Clone, Debug)]
pub struct OnDisconnectRequest {
	pub ctx: ActorContext,
	pub conn: ConnHandle,
}

#[derive(Clone, Debug)]
pub struct ActionRequest {
	pub ctx: ActorContext,
	pub conn: ConnHandle,
	pub name: String,
	pub args: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct OnBeforeActionResponseRequest {
	pub ctx: ActorContext,
	pub name: String,
	pub args: Vec<u8>,
	pub output: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct RunRequest {
	pub ctx: ActorContext,
}

#[derive(Default)]
pub struct ActorInstanceCallbacks {
	pub on_wake: Option<LifecycleCallback<OnWakeRequest>>,
	pub on_sleep: Option<LifecycleCallback<OnSleepRequest>>,
	pub on_destroy: Option<LifecycleCallback<OnDestroyRequest>>,
	pub on_state_change: Option<LifecycleCallback<OnStateChangeRequest>>,
	pub on_request: Option<RequestCallback>,
	pub on_websocket: Option<LifecycleCallback<OnWebSocketRequest>>,
	pub on_before_connect: Option<LifecycleCallback<OnBeforeConnectRequest>>,
	pub on_connect: Option<LifecycleCallback<OnConnectRequest>>,
	pub on_disconnect: Option<LifecycleCallback<OnDisconnectRequest>>,
	pub actions: HashMap<String, ActionHandler>,
	pub on_before_action_response: Option<BeforeActionResponseCallback>,
	pub run: Option<LifecycleCallback<RunRequest>>,
}

impl fmt::Debug for ActorInstanceCallbacks {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("ActorInstanceCallbacks")
			.field("on_wake", &self.on_wake.is_some())
			.field("on_sleep", &self.on_sleep.is_some())
			.field("on_destroy", &self.on_destroy.is_some())
			.field("on_state_change", &self.on_state_change.is_some())
			.field("on_request", &self.on_request.is_some())
			.field("on_websocket", &self.on_websocket.is_some())
			.field("on_before_connect", &self.on_before_connect.is_some())
			.field("on_connect", &self.on_connect.is_some())
			.field("on_disconnect", &self.on_disconnect.is_some())
			.field("actions", &self.actions.keys().collect::<Vec<_>>())
			.field(
				"on_before_action_response",
				&self.on_before_action_response.is_some(),
			)
			.field("run", &self.run.is_some())
			.finish()
	}
}
