use api_helper::auth::{ApiAuth, AuthRateLimitCtx};
use proto::claims::Claims;
use rivet_operation::prelude::*;

/// Information derived from the authentication middleware.
pub struct Auth {
	_claims: Option<Claims>,
}

#[async_trait]
impl ApiAuth for Auth {
	async fn new(
		config: rivet_config::Config,
		_api_token: Option<String>,
		rate_limit_ctx: AuthRateLimitCtx<'_>,
	) -> GlobalResult<Auth> {
		Self::rate_limit(&config, rate_limit_ctx).await?;

		Ok(Auth { _claims: None })
	}

	async fn rate_limit(
		config: &rivet_config::Config,
		_rate_limit_ctx: AuthRateLimitCtx<'_>,
	) -> GlobalResult<()> {
		Ok(())
	}
}
