use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use rivet_api::models;
use rivet_operation::prelude::*;

use crate::auth::Auth;

// MARK: GET /bootstrap
pub async fn get(
	_ctx: Ctx<Auth>,
	_watch_index_query: WatchIndexQuery,
) -> GlobalResult<models::CloudBootstrapResponse> {
	Ok(models::CloudBootstrapResponse {
		captcha: Box::new(models::CloudBootstrapCaptcha {
			turnstile: Box::new(models::CloudBootstrapCaptchaTurnstile {
				// TODO: Find a better way of cleanly disabling Turnstile
				site_key: std::env::var("TURNSTILE_SITE_KEY_MAIN").unwrap_or_default(),
			}),
		}),
	})
}
