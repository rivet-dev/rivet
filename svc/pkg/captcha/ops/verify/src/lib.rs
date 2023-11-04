use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;
use serde_json::json;

#[operation(name = "captcha-verify")]
async fn handle(
	ctx: OperationContext<captcha::verify::Request>,
) -> GlobalResult<captcha::verify::Response> {
	let crdb = ctx.crdb().await?;

	let captcha_config = unwrap_ref!(ctx.captcha_config);
	let client_response = unwrap_ref!(ctx.client_response);
	let client_response_kind = unwrap_ref!(client_response.kind);

	let topic_value = serde_json::to_value(&ctx.topic)?;
	let topic_str = util_captcha::serialize_topic_str(&ctx.topic)?;

	let user_id = ctx.user_id.as_ref().map(common::Uuid::as_uuid);
	let namespace_id = ctx.namespace_id.as_ref().map(common::Uuid::as_uuid);

	let (success, response_kind_str) = match (&captcha_config, &client_response_kind) {
		(
			backend::captcha::CaptchaConfig {
				hcaptcha: Some(hcaptcha),
				..
			},
			backend::captcha::captcha_client_response::Kind::Hcaptcha(hcaptcha_client_res),
		) => {
			let config_res = op!([ctx] captcha_hcaptcha_config_get {
				config: Some(hcaptcha.clone()),
			})
			.await?;

			let res = op!([ctx] captcha_hcaptcha_verify {
				client_response: hcaptcha_client_res.client_response.clone(),
				site_key: config_res.site_key.clone(),
				remote_address: ctx.remote_address.to_owned(),
			})
			.await?;

			// Insert verification
			sql_execute!(
				[ctx]
				"
				INSERT INTO db_captcha.captcha_verifications (
					verification_id, topic, topic_str, remote_address,
					complete_ts, expire_ts, provider, success, user_id,
					namespace_id
				)
				VALUES ($1, $2, $3, $4, $5, to_timestamp($6::float / 1000), $7, $8, $9, $10)
				",
				Uuid::new_v4(),
				&topic_value,
				&topic_str,
				ctx.remote_address.as_str(),
				ctx.ts(),
				ctx.ts() + captcha_config.verification_ttl,
				backend::captcha::CaptchaProvider::Hcaptcha as i64,
				res.success,
				user_id,
				namespace_id,
			)
			.await?;

			(res.success, "hcaptcha")
		}
		(
			backend::captcha::CaptchaConfig {
				turnstile: Some(turnstile),
				..
			},
			backend::captcha::captcha_client_response::Kind::Turnstile(turnstile_client_res),
		) => {
			let origin_host = unwrap_ref!(ctx.origin_host, "no origin");

			// Check for "rivet.game" host
			let secret_key = if util::env::domain_cdn().map_or(false, |domain_cdn| {
				domain_cdn == origin_host || origin_host.ends_with(&format!(".{domain_cdn}"))
			}) {
				Some(util::env::read_secret(&["turnstile", "cdn", "secret_key"]).await?)
			}
			// Check for host from captcha config
			else {
				turnstile.domains.iter().find_map(|domain| {
					(&domain.domain == origin_host
						|| origin_host.ends_with(&format!(".{}", domain.domain)))
					.then(|| domain.secret_key.clone())
				})
			};
			let secret_key = unwrap_with!(secret_key, CAPTCHA_CAPTCHA_ORIGIN_NOT_ALLOWED);

			let res = op!([ctx] cf_turnstile_verify {
				client_response: turnstile_client_res.client_response.clone(),
				remote_address: ctx.remote_address.to_owned(),
				secret_key: secret_key,
			})
			.await?;

			// Insert verification
			sql_execute!(
				[ctx]
				"
				INSERT INTO db_captcha.captcha_verifications (
					verification_id, topic, topic_str, remote_address, complete_ts, expire_ts, provider, success, user_id, namespace_id
				)
				VALUES ($1, $2, $3, $4, $5, to_timestamp($6::float / 1000), $7, $8, $9, $10)
				",
				Uuid::new_v4(),
				&topic_value,
				&topic_str,
				ctx.remote_address.as_str(),
				ctx.ts(),
				ctx.ts() + captcha_config.verification_ttl,
				backend::captcha::CaptchaProvider::Turnstile as i64,
				res.success,
				user_id,
				namespace_id,
			)
			.await?;

			(res.success, "turnstile")
		}
		_ => bail!("invalid request"),
	};

	ensure_with!(success, CAPTCHA_CAPTCHA_FAILED);

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				name: if success { "captcha.success" } else { "captcha.fail" }.into(),
				properties_json: Some(serde_json::to_string(&json!({
					"user_id": user_id,
					"namespace_id": namespace_id,
					"topic": ctx.topic,
					"requests_before_reverify": captcha_config.requests_before_reverify,
					"verification_ttl": captcha_config.verification_ttl,
					"has_hcaptcha": captcha_config.hcaptcha.is_some(),
					"has_turnstile": captcha_config.turnstile.is_some(),
					"client_response_kind": response_kind_str,
				}))?),
				..Default::default()
			}
		],
	})
	.await?;

	Ok(captcha::verify::Response {})
}
