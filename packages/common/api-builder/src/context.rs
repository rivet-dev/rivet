use std::{fmt, ops::Deref};

use anyhow::*;
use axum::extract::{FromRequest, Request};
use gas::prelude::*;

use crate::{ApiError, GlobalApiCtx, RequestIds};

/// Request-specific API context
#[derive(Clone)]
pub struct ApiCtx {
	ray_id: Id,
	req_id: Id,
	standalone_ctx: StandaloneCtx,
}

impl Deref for ApiCtx {
	type Target = StandaloneCtx;

	fn deref(&self) -> &Self::Target {
		&self.standalone_ctx
	}
}

impl fmt::Debug for ApiCtx {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("ApiCtx")
			.field("global", &"GlobalApiCtx { ... }")
			.finish()
	}
}

impl ApiCtx {
	pub fn new(global: GlobalApiCtx, ray_id: Id, req_id: Id) -> Result<Self> {
		// Create StandaloneCtx synchronously by using a blocking call
		// This is necessary because we need Clone support and async Clone is not possible
		let standalone_ctx = StandaloneCtx::new(
			global.db.clone(),
			global.config.clone(),
			global.pools.clone(),
			global.cache.clone(),
			global.name,
			ray_id,
			req_id,
		)?;

		Ok(Self {
			ray_id,
			req_id,
			standalone_ctx,
		})
	}

	pub fn ray_id(&self) -> Id {
		self.ray_id
	}

	pub fn req_id(&self) -> Id {
		self.req_id
	}
}

impl FromRequest<GlobalApiCtx> for ApiCtx {
	type Rejection = ApiError;

	async fn from_request(req: Request, state: &GlobalApiCtx) -> Result<Self, Self::Rejection> {
		let global = state.clone();

		let dc_label = state.config.dc_label();

		// Try to get request IDs from request extensions (set by logging middleware)
		let request_ids = req
			.extensions()
			.get::<RequestIds>()
			.copied()
			.unwrap_or_else(|| RequestIds::new(dc_label));

		ApiCtx::new(global, request_ids.ray_id, request_ids.req_id).map_err(ApiError::from)
	}
}
