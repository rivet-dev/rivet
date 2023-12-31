use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use rivet_api::models;
use rivet_operation::prelude::*;

use crate::auth::Auth;

// MARK: GET /bootstrap
pub async fn get(
	_ctx: Ctx<Auth>,
	_watch_index_query: WatchIndexQuery,
) -> GlobalResult<models::CloudBootstrapResponse> {
	Ok(build_bootstrap_data().await?)
}

pub async fn build_bootstrap_data() -> GlobalResult<models::CloudBootstrapResponse> {
	Ok(models::CloudBootstrapResponse {
		cluster: models::CloudBootstrapCluster::Oss,
		domains: if let (Some(main), Some(cdn), Some(job)) = (
			util::env::domain_main(),
			util::env::domain_cdn(),
			util::env::domain_job(),
		) {
			Some(Box::new(models::CloudBootstrapDomains {
				main: main.into(),
				cdn: cdn.into(),
				job: job.into(),
			}))
		} else {
			None
		},
		origins: Box::new(models::CloudBootstrapOrigins {
			hub: util::env::origin_hub().into(),
		}),
		captcha: Box::new(models::CloudBootstrapCaptcha {
			turnstile: std::env::var("TURNSTILE_SITE_KEY_MAIN")
				.ok()
				.map(|site_key| Box::new(models::CloudBootstrapCaptchaTurnstile { site_key })),
		}),
	})
}
