use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use rivet_api::models;
use rivet_config::config::rivet::AccessKind;
use rivet_operation::prelude::*;

use crate::auth::Auth;

// MARK: GET /bootstrap
pub async fn get(
	ctx: Ctx<Auth>,
	_watch_index_query: WatchIndexQuery,
) -> GlobalResult<models::CloudBootstrapResponse> {
	build_bootstrap_data(ctx.config()).await
}

pub async fn build_bootstrap_data(
	config: &rivet_config::Config,
) -> GlobalResult<models::CloudBootstrapResponse> {
	let server_config = config.server()?;

	Ok(models::CloudBootstrapResponse {
		cluster: models::CloudBootstrapCluster::Oss,
		access: match server_config.rivet.auth.access_kind {
			AccessKind::Public => models::CloudBootstrapAccess::Public,
			AccessKind::Private => models::CloudBootstrapAccess::Private,
			AccessKind::Development => models::CloudBootstrapAccess::Development,
		},
		domains: Box::new(models::CloudBootstrapDomains {
			cdn: server_config
				.rivet
				.dns
				.as_ref()
				.and_then(|x| x.domain_cdn.clone()),
			job: server_config
				.rivet
				.dns
				.as_ref()
				.and_then(|x| x.domain_job.clone()),
			opengb: None,
		}),
		origins: Box::new(models::CloudBootstrapOrigins {
			hub: config.server()?.rivet.ui.public_origin().to_string(),
		}),
		captcha: Box::new(models::CloudBootstrapCaptcha {
			turnstile: server_config
				.turnstile
				.as_ref()
				.and_then(|x| x.main_site_key.clone())
				.map(|site_key| Box::new(models::CloudBootstrapCaptchaTurnstile { site_key })),
		}),
		login_methods: Box::new(models::CloudBootstrapLoginMethods {
			email: server_config.sendgrid.is_some(),
		}),
		deploy_hash: rivet_env::source_hash().to_string(),
	})
}
