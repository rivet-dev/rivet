use hyper::header::{self, HeaderName, HeaderValue};
use rivet_operation::prelude::*;
use url::Url;

use crate::route::tokens::{REFRESH_TOKEN_TTL, USER_REFRESH_TOKEN_COOKIE};

#[tracing::instrument(skip(refresh_token))]
fn build_cookie_header(origin: &Url, refresh_token: &str, max_age: i64) -> GlobalResult<String> {
	// Build base header
	//
	// Use `SameSite=None` because the hub is hosted on a separate subdomain than the API
	//
	// Localhost domains are considered secure, so we can leave `Secure` enabled even for local
	// development.
	let mut header = format!(
		"{USER_REFRESH_TOKEN_COOKIE}={refresh_token}; Max-Age={max_age}; HttpOnly; Path=/; SameSite=None; Secure",
	);
	if let Some(domain_api) = util::env::domain_main_api() {
		header.push_str(&format!("; Domain={domain_api}"));
	}

	tracing::info!(?header, "built cookie header");

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
