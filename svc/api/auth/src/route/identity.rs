use std::collections::HashMap;
use std::str::FromStr;

use api_helper::ctx::Ctx;
use email_verification::complete::response::Status as StatusProto;
use http::response::Builder;
use proto::backend::{self, pkg::*};
use rivet_api::models;
use rivet_auth_server::models as models_old;
use rivet_convert::ApiTryInto;
use rivet_operation::prelude::*;

use crate::{auth::Auth, utils::refresh_token_header};

// MARK: POST /identity/email/start-verification
pub async fn start(
	ctx: Ctx<Auth>,
	body: models::AuthStartEmailVerificationRequest,
) -> GlobalResult<models::AuthStartEmailVerificationResponse> {
	let user_ent = ctx.auth().user(ctx.op_ctx()).await?;

	// Verify captcha
	let secret_key_opt = util::env::read_secret_opt(&["turnstile", "main", "secret_key"]).await?;

	// If no Turnstile key defined, skip captcha
	if let Some(secret_key) = secret_key_opt {
		if let Some(captcha) = body.captcha {
			if captcha.turnstile.is_some() {
				op!([ctx] captcha_verify {
					topic: HashMap::<String, String>::from([
						("kind".into(), "auth:verification-start".into()),
					]),
					remote_address: unwrap_ref!(ctx.remote_address()).to_string(),
					origin_host: Some("rivet.gg".to_string()),
					captcha_config: Some(backend::captcha::CaptchaConfig {
						requests_before_reverify: 0,
						verification_ttl: 0,
						turnstile: Some(backend::captcha::captcha_config::Turnstile {
							domains: vec![
								backend::captcha::captcha_config::turnstile::Domain {
									domain: "rivet.gg".to_string(),
									secret_key: secret_key
								},
							],
						}),
						..Default::default()
					}),
					client_response: Some((*captcha).try_into()?),
					user_id: Some(user_ent.user_id.into()),
				})
				.await?;
			} else {
				bail_with!(CAPTCHA_CAPTCHA_INVALID)
			}
		} else {
			bail_with!(CAPTCHA_CAPTCHA_INVALID)
		}
	}

	let res = op!([ctx] email_verification_create {
		email: body.email.clone(),
		game_id: body.game_id.map(|x| x.into()),
	})
	.await?;

	let verification_id = unwrap_ref!(res.verification_id).as_uuid();

	Ok(models::AuthStartEmailVerificationResponse {
		verification_id: verification_id,
	})
}

// MARK: POST /identity/email/complete-verification
pub async fn complete(
	ctx: Ctx<Auth>,
	response: &mut Builder,
	body: models_old::CompleteEmailVerificationRequest,
) -> GlobalResult<models_old::CompleteEmailVerificationResponse> {
	let user_ent = ctx.auth().user(ctx.op_ctx()).await?;

	let origin = unwrap!(ctx.origin());

	let res = op!([ctx] email_verification_complete {
		verification_id: Some(Into::into(
			Uuid::from_str(body.verification_id.as_str())?
		)),
		code: body.code.clone()
	})
	.await?;

	let status = unwrap!(StatusProto::from_i32(res.status));

	// Handle error statuses
	let err = match status {
		StatusProto::Correct => None,
		StatusProto::AlreadyComplete => Some(models_old::CompleteStatus::AlreadyComplete),
		StatusProto::Expired => Some(models_old::CompleteStatus::Expired),
		StatusProto::TooManyAttempts => Some(models_old::CompleteStatus::TooManyAttempts),
		StatusProto::Incorrect => Some(models_old::CompleteStatus::Incorrect),
	};

	if let Some(status) = err {
		return Ok(models_old::CompleteEmailVerificationResponse { status });
	}

	let email_res = op!([ctx] user_resolve_email {
		emails: vec![res.email.clone()]
	})
	.await?;

	tracing::info!(email = %res.email, "resolved email");

	// Switch to new user
	if let Some(new_user) = email_res.users.first() {
		let new_user_id = unwrap_ref!(new_user.user_id).as_uuid();

		tracing::info!(old_user_id = %user_ent.user_id, %new_user_id, "identity found, switching user");

		let token_res = op!([ctx] user_token_create {
			user_id: Some(new_user_id.into()),
			client: Some(ctx.client_info()),
		})
		.await?;

		// Set refresh token
		{
			let (k, v) = refresh_token_header(origin, token_res.refresh_token)?;
			unwrap!(response.headers_mut()).insert(k, v);
		}

		Ok(models_old::CompleteEmailVerificationResponse {
			status: models_old::CompleteStatus::SwitchIdentity,
		})
	}
	// Associate identity with existing user
	else {
		tracing::info!(user_id = %user_ent.user_id, "creating new identity for guest");

		op!([ctx] user_identity_create {
			user_id: Some(Into::into(user_ent.user_id)),
			identity: Some(backend::user_identity::Identity {
				kind: Some(backend::user_identity::identity::Kind::Email(
					backend::user_identity::identity::Email {
						email: res.email.clone(),
					}
				))
			})
		})
		.await?;

		// Send user update to api-portal
		msg!([ctx] user::msg::update(user_ent.user_id) {
			user_id: Some(user_ent.user_id.into()),
		})
		.await?;

		Ok(models_old::CompleteEmailVerificationResponse {
			status: models_old::CompleteStatus::LinkedAccountAdded,
		})
	}
}
