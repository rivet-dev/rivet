use std::net::IpAddr;

use async_trait::async_trait;
use global_error::GlobalResult;
use rivet_cache::{Cache, RateLimitConfig};

/// Api Authentication Trait used for API services so that they can have their own auth
/// flow with the `api_helper::Ctx`.
#[async_trait]
pub trait ApiAuth: Sized {
	async fn new(
		api_token: Option<String>,
		rate_limit_ctx: AuthRateLimitCtx<'_>,
	) -> GlobalResult<Self>;

	async fn rate_limit(rate_limit_ctx: AuthRateLimitCtx<'_>) -> GlobalResult<()>;
}

pub struct AuthRateLimitCtx<'a> {
	pub cache: &'a Cache,
	pub remote_address: Option<&'a IpAddr>,
	pub rate_limit_config: RateLimitConfig,
	pub bypass_token: Option<String>,
}
