use api_helper::auth::{ApiAuth, AuthRateLimitCtx};
use proto::claims::Claims;
use rivet_operation::prelude::*;

pub struct Auth {
	_claims: Option<Claims>,
}

#[async_trait]
impl ApiAuth for Auth {
	async fn new(
		api_token: Option<String>,
		rate_limit_ctx: AuthRateLimitCtx<'_>,
	) -> GlobalResult<Auth> {
		Self::rate_limit(rate_limit_ctx).await?;

		// TODO: Don't hardcode this
		// TODO: Use JWT
		if let Some(api_token) = api_token {
			assert_eq_with!(
				api_token,
				util::env::read_secret(&["rivet", "api_status", "token"]).await?,
				API_FORBIDDEN,
				reason = "Invalid auth"
			);
		}

		Ok(Auth { _claims: None })
	}

	async fn rate_limit(_rate_limit_ctx: AuthRateLimitCtx<'_>) -> GlobalResult<()> {
		Ok(())
	}
}
