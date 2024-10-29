use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "captcha-turnstile-config-get")]
async fn handle(
	ctx: OperationContext<captcha::turnstile_config_get::Request>,
) -> GlobalResult<captcha::turnstile_config_get::Response> {
	let config = unwrap_ref!((*ctx).config);

	// Check for "rivet.game" host
	let site_key = if let Some(origin_host) = &ctx.origin_host {
		if ctx
			.config()
			.server()?
			.rivet
			.dns
			.as_ref()
			.and_then(|x| x.domain_cdn.as_ref())
			.map_or(false, |domain_cdn| {
				domain_cdn == origin_host || origin_host.ends_with(&format!(".{domain_cdn}"))
			}) {
			Some(
				unwrap!(
					ctx.config()
						.server()?
						.turnstile
						.as_ref()
						.and_then(|x| x.cdn_site_key.as_ref()),
					"missing turnstile cdn site key"
				)
				.clone(),
			)
		} else {
			None
		}
	} else {
		None
	};

	// Default to host from captcha config
	let site_key = site_key.unwrap_or_else(|| config.site_key.clone());

	Ok(captcha::turnstile_config_get::Response { site_key })
}
