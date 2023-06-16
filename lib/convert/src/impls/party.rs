use proto::backend;
use rivet_operation::prelude::*;
use rivet_party_server::models;

use crate::ApiTryFrom;

impl ApiTryFrom<models::CaptchaConfig> for backend::captcha::CaptchaClientResponse {
	type Error = GlobalError;

	fn try_from(
		value: models::CaptchaConfig,
	) -> GlobalResult<backend::captcha::CaptchaClientResponse> {
		let kind = match value {
			models::CaptchaConfig::Hcaptcha(hcaptcha) => {
				backend::captcha::captcha_client_response::Kind::Hcaptcha(
					backend::captcha::captcha_client_response::Hcaptcha {
						client_response: hcaptcha.client_response,
					},
				)
			}
			models::CaptchaConfig::Turnstile(turnstile) => {
				backend::captcha::captcha_client_response::Kind::Turnstile(
					backend::captcha::captcha_client_response::Turnstile {
						client_response: turnstile.client_response,
					},
				)
			}
		};

		Ok(backend::captcha::CaptchaClientResponse { kind: Some(kind) })
	}
}
