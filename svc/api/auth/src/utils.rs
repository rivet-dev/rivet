use hyper::header::{self, HeaderName, HeaderValue};
use rivet_operation::prelude::*;
use url::Url;

use crate::route::tokens::{REFRESH_TOKEN_TTL, USER_REFRESH_TOKEN_COOKIE};

#[tracing::instrument(skip(refresh_token))]
fn build_cookie_header(origin: &Url, refresh_token: &str, max_age: i64) -> GlobalResult<String> {
	// https://developer.mozilla.org/en-US/docs/Web/Security/Secure_Contexts#when_is_a_context_considered_secure

	// See https://developer.mozilla.org/en-US/docs/Web/Security/Secure_Contexts#when_is_a_context_considered_secure
	//
	// These local domains are considered secure, so we can leave `Secure`
	// enabled.
	let host = internal_unwrap_owned!(origin.host_str());
	let is_localhost = host == "127.0.0.1"
		|| host == "localhost"
		|| host.ends_with(".localhost")
		|| origin.scheme() == "file";
	let is_hub = api_helper::util::hub_origin_regex().is_match(&origin.to_string());

	// Disable `SameSite` to let localhost work.
	let same_site = if is_localhost || is_hub {
		"None"
	} else {
		"Strict"
	};

	// Build base header
	let mut header = format!(
		"{USER_REFRESH_TOKEN_COOKIE}={refresh_token}; Max-Age={max_age}; HttpOnly; Path=/; SameSite={same_site}; Secure",
	);
	if let Some(domain_api) = util::env::domain_main_api() {
		header.push_str(&format!("; Domain={domain_api}"));
	}

	tracing::info!(?host, ?is_hub, ?same_site, ?header, "built cookie header");

	Ok(header)
}

/// We rely on CORS to make sure the origin is valid.
#[tracing::instrument(skip(refresh_token))]
pub fn refresh_token_header(
	origin: &Url,
	refresh_token: String,
) -> GlobalResult<(HeaderName, HeaderValue)> {
	Ok((
		header::SET_COOKIE,
		header::HeaderValue::from_str(&build_cookie_header(
			origin,
			&refresh_token,
			REFRESH_TOKEN_TTL / 1000,
		)?)
		.map_err(Into::<GlobalError>::into)?,
	))
}

#[tracing::instrument]
pub fn delete_refresh_token_header(origin: &Url) -> GlobalResult<(HeaderName, HeaderValue)> {
	Ok((
		header::SET_COOKIE,
		header::HeaderValue::from_str(&build_cookie_header(origin, "", 0)?)
			.map_err(Into::<GlobalError>::into)?,
	))
}

pub fn touch_user_presence(ctx: OperationContext<()>, user_id: Uuid) {
	let spawn_res = tokio::task::Builder::new()
		.name("api_auth::user_presence_touch")
		.spawn(async move {
			let res = op!([ctx] user_presence_touch {
				user_id: Some(user_id.into()),
			})
			.await;
			match res {
				Ok(_) => {}
				Err(err) => tracing::error!(?err, "failed to touch user presence"),
			}
		});
	if let Err(err) = spawn_res {
		tracing::error!(?err, "failed to spawn user_presence_touch task");
	}
}
