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
			turnstile: if let Some(site_key) = std::env::var("TURNSTILE_SITE_KEY_MAIN").ok() {
				Some(Box::new(models::CloudBootstrapCaptchaTurnstile {
					site_key,
				}))
			} else {
				None
			},
		}),
	})
}
