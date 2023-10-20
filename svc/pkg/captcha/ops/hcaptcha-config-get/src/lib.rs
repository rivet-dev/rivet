use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[operation(name = "captcha-hcaptcha-config-get")]
async fn handle(
	ctx: OperationContext<captcha::hcaptcha_config_get::Request>,
) -> GlobalResult<captcha::hcaptcha_config_get::Response> {
	let config = unwrap_ref!(ctx.config);
	let level = unwrap!(backend::captcha::captcha_config::hcaptcha::Level::from_i32(
		config.level
	));

	let site_key = unwrap!(get_hcaptcha_site_key(level), "missing hcaptcha site key");

	Ok(captcha::hcaptcha_config_get::Response { site_key })
}

#[tracing::instrument]
fn get_hcaptcha_site_key(
	level: backend::captcha::captcha_config::hcaptcha::Level,
) -> Option<String> {
	match level {
		backend::captcha::captcha_config::hcaptcha::Level::Easy => {
			std::env::var("HCAPTCHA_SITE_KEY_EASY").ok()
		}
		backend::captcha::captcha_config::hcaptcha::Level::Moderate => {
			std::env::var("HCAPTCHA_SITE_KEY_MODERATE").ok()
		}
		backend::captcha::captcha_config::hcaptcha::Level::Difficult => {
			std::env::var("HCAPTCHA_SITE_KEY_DIFFICULT").ok()
		}
		backend::captcha::captcha_config::hcaptcha::Level::AlwaysOn => {
			std::env::var("HCAPTCHA_SITE_KEY_ALWAYS_ON").ok()
		}
	}
}
