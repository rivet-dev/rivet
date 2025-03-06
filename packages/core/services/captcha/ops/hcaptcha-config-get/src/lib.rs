use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "captcha-hcaptcha-config-get")]
async fn handle(
	ctx: OperationContext<captcha::hcaptcha_config_get::Request>,
) -> GlobalResult<captcha::hcaptcha_config_get::Response> {
	let config = unwrap_ref!((*ctx).config);

	let site_key = if let Some(site_key) = &config.site_key {
		site_key.clone()
	} else {
		unwrap_ref!(
			ctx.config().server()?.hcaptcha()?.site_key_fallback,
			"missing hcaptcha site key fallback"
		)
		.read()
		.to_string()
	};

	Ok(captcha::hcaptcha_config_get::Response { site_key })
}
