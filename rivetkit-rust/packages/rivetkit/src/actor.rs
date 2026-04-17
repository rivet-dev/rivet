use std::sync::Arc;

use anyhow::{Result, bail};
use async_trait::async_trait;
use http::StatusCode;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::context::{ConnCtx, Ctx};
use rivetkit_core::{ActorConfig, Request, Response, WebSocket};

#[async_trait]
pub trait Actor: Send + Sync + Sized + 'static {
	type State: Serialize + DeserializeOwned + Send + Sync + Clone + 'static;
	type ConnParams: DeserializeOwned + Send + Sync + 'static;
	type ConnState: Serialize + DeserializeOwned + Send + Sync + 'static;
	type Input: DeserializeOwned + Send + Sync + 'static;
	type Vars: Send + Sync + 'static;

	async fn create_state(
		ctx: &Ctx<Self>,
		input: &Self::Input,
	) -> Result<Self::State>;

	async fn create_vars(_ctx: &Ctx<Self>) -> Result<Self::Vars> {
		bail!("Actor::create_vars must be implemented when Vars is not ()")
	}

	async fn create_conn_state(
		self: &Arc<Self>,
		ctx: &Ctx<Self>,
		params: &Self::ConnParams,
	) -> Result<Self::ConnState>;

	async fn on_create(ctx: &Ctx<Self>, input: &Self::Input) -> Result<Self>;

	async fn on_wake(self: &Arc<Self>, ctx: &Ctx<Self>) -> Result<()> {
		let _ = ctx;
		Ok(())
	}

	async fn on_sleep(self: &Arc<Self>, ctx: &Ctx<Self>) -> Result<()> {
		let _ = ctx;
		Ok(())
	}

	async fn on_destroy(self: &Arc<Self>, ctx: &Ctx<Self>) -> Result<()> {
		let _ = ctx;
		Ok(())
	}

	async fn on_state_change(self: &Arc<Self>, ctx: &Ctx<Self>) -> Result<()> {
		let _ = ctx;
		Ok(())
	}

	async fn on_request(
		self: &Arc<Self>,
		ctx: &Ctx<Self>,
		request: Request,
	) -> Result<Response> {
		let _ = (ctx, request);
		let mut response = Response::new(Vec::new());
		*response.status_mut() = StatusCode::NOT_FOUND;
		Ok(response)
	}

	async fn on_websocket(
		self: &Arc<Self>,
		ctx: &Ctx<Self>,
		ws: WebSocket,
	) -> Result<()> {
		let _ = (ctx, ws);
		Ok(())
	}

	async fn on_before_connect(
		self: &Arc<Self>,
		ctx: &Ctx<Self>,
		params: &Self::ConnParams,
	) -> Result<()> {
		let _ = (ctx, params);
		Ok(())
	}

	async fn on_connect(
		self: &Arc<Self>,
		ctx: &Ctx<Self>,
		conn: ConnCtx<Self>,
	) -> Result<()> {
		let _ = (ctx, conn);
		Ok(())
	}

	async fn on_disconnect(
		self: &Arc<Self>,
		ctx: &Ctx<Self>,
		conn: ConnCtx<Self>,
	) -> Result<()> {
		let _ = (ctx, conn);
		Ok(())
	}

	async fn run(self: &Arc<Self>, ctx: &Ctx<Self>) -> Result<()> {
		let _ = ctx;
		Ok(())
	}

	fn config() -> ActorConfig {
		ActorConfig::default()
	}
}
