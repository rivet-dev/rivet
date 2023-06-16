use proto::backend;
use rivet_api::models;
use rivet_operation::prelude::*;

use crate::ApiTryFrom;

impl ApiTryFrom<models::CaptchaConfig> for backend::captcha::CaptchaClientResponse {
	type Error = GlobalError;

	fn try_from(
		value: models::CaptchaConfig,
	) -> GlobalResult<backend::captcha::CaptchaClientResponse> {
		let kind = if let Some(hcaptcha) = value.hcaptcha {
			backend::captcha::captcha_client_response::Kind::Hcaptcha(
				backend::captcha::captcha_client_response::Hcaptcha {
					client_response: hcaptcha.client_response,
				},
			)
		} else if let Some(turnstile) = value.turnstile {
			backend::captcha::captcha_client_response::Kind::Turnstile(
				backend::captcha::captcha_client_response::Turnstile {
					client_response: turnstile.client_response,
				},
			)
		} else {
			internal_panic!("unknown captcha kind")
		};

		Ok(backend::captcha::CaptchaClientResponse { kind: Some(kind) })
	}
}
