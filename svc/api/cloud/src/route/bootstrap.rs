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
		cluster: models::CloudBootstrapCluster::Oss,
		captcha: Box::new(models::CloudBootstrapCaptcha {
			turnstile: std::env::var("TURNSTILE_SITE_KEY_MAIN")
				.ok()
				.map(|site_key| Box::new(models::CloudBootstrapCaptchaTurnstile { site_key })),
		}),
	})
}
