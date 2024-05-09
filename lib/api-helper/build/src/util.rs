use std::{str::FromStr, time::Duration};

use global_error::prelude::*;
use headers::{
	AccessControlAllowHeaders, AccessControlAllowMethods, AccessControlExposeHeaders,
	AccessControlMaxAge, HeaderMap, HeaderMapExt,
};
use hyper::{
	header::{self, HeaderName},
	Body, Request,
};
use regex::Regex;
use rivet_claims::ClaimsDecode;

/// Origins that the hub may be requesting from.
pub fn hub_origin_regex() -> Regex {
	// TODO: Make this lazy static to prevent reparsing regex for every request
	let regex_str =
		std::env::var("RIVET_API_HUB_ORIGIN_REGEX").expect("RIVET_API_HUB_ORIGIN_REGEX");
	Regex::new(&regex_str).expect("invalid regex")
}

#[derive(Default)]
pub struct CorsConfigBuilder {
	pub origins: Vec<String>,
	pub origin_regex: Option<Regex>,
	pub any_origin: bool,
	pub methods: Vec<String>,
	pub headers: Vec<String>,
	pub credentials: bool,
}

impl CorsConfigBuilder {
	/// CORS config for services intended to be exposed publicly.
	pub fn public() -> Self {
		Self::default()
			.any_origin()
			.methods(&["GET", "POST", "PUT", "DELETE"])
			.headers(&["Content-Type", "Authorization"])
			.credentials()
	}

	/// CORS config for services intended to be exposed to the Hub.
	pub fn hub() -> Self {
		Self::default()
			.origin_regex(hub_origin_regex())
			.methods(&["GET", "POST", "PUT", "DELETE"])
			.headers(&["Content-Type", "Authorization"])
			.credentials()
	}
}

impl CorsConfigBuilder {
	pub fn origin(mut self, origin: impl ToString) -> Self {
		self.origins.push(origin.to_string());
		self
	}

	pub fn origin_regex(mut self, regex: Regex) -> Self {
		self.origin_regex = Some(regex);
		self
	}

	pub fn any_origin(mut self) -> Self {
		self.any_origin = true;
		self
	}

	pub fn methods<'a, Item>(mut self, methods: impl IntoIterator<Item = &'a Item>) -> Self
	where
		Item: ToString + 'a,
	{
		self.methods
			.extend(methods.into_iter().map(ToString::to_string));
		self
	}

	pub fn header(mut self, header: impl ToString) -> Self {
		self.headers.push(header.to_string());
		self
	}

	pub fn headers<'a, Item>(mut self, headers: impl IntoIterator<Item = &'a Item>) -> Self
	where
		Item: ToString + 'a,
	{
		self.headers
			.extend(headers.into_iter().map(ToString::to_string));
		self
	}

	pub fn credentials(mut self) -> Self {
		self.credentials = true;
		self
	}

	pub fn build(self) -> CorsConfig {
		let allowed_methods = self
			.methods
			.iter()
			.map(|m| http::Method::from_str(&m))
			.collect::<Result<Vec<_>, _>>()
			.expect("invalid method");
		let allowed_headers = self
			.headers
			.iter()
			.map(|h| HeaderName::from_str(&h))
			.collect::<Result<Vec<_>, _>>()
			.expect("invalid header");

		let text_headers = self
			.headers
			.into_iter()
			.map(|h| h.to_lowercase())
			.collect::<Vec<_>>();

		CorsConfig {
			allowed_origins: self.origins,
			allowed_origins_regex: self.origin_regex,
			allowed_any_origin: self.any_origin,
			allowed_methods,
			allowed_headers,
			text_headers,
			credentials: self.credentials,
		}
	}
}

#[derive(Debug)]
pub struct CorsConfig {
	allowed_origins: Vec<String>,
	allowed_origins_regex: Option<Regex>,
	allowed_any_origin: bool,
	allowed_methods: Vec<http::Method>,
	allowed_headers: Vec<HeaderName>,
	text_headers: Vec<String>,

	pub credentials: bool,
}

impl CorsConfig {
	fn matches_origin(&self, origin: &str) -> bool {
		if self.allowed_any_origin {
			true
		} else if !self.allowed_origins.is_empty()
			&& self.allowed_origins.iter().any(|x| x == origin)
		{
			true
		} else if let Some(regex) = &self.allowed_origins_regex {
			regex.is_match(origin)
		} else {
			false
		}
	}
}

#[derive(Debug)]
pub enum CorsResponse {
	Preflight(HeaderMap),
	Regular(HeaderMap),
	NoCors,
}

pub fn verify_cors(request: &Request<Body>, config: &CorsConfig) -> GlobalResult<CorsResponse> {
	let headers = request.headers();

	// Build generic CORS headers
	let mut cors_headers = HeaderMap::new();
	if config.credentials {
		cors_headers.insert(
			header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
			header::HeaderValue::from_static("true"),
		);
	}
	cors_headers.typed_insert(
		vec![HeaderName::from_static("rvt-ray-id")]
			.into_iter()
			.collect::<AccessControlExposeHeaders>(),
	);

	match (headers.get(header::ORIGIN), request.method()) {
		// Verify preflight request
		(Some(origin), &http::Method::OPTIONS) => {
			// Verify origin
			if !config.matches_origin(origin.to_str()?) {
				return Err(err_code!(API_CORS_ORIGIN_NOT_ALLOWED));
			}

			let allowed_methods_str = config
				.allowed_methods
				.iter()
				.map(|m| m.as_str())
				.collect::<Vec<_>>()
				.join(", ");

			// Verify method
			if let Some(req_method) = headers.get(header::ACCESS_CONTROL_REQUEST_METHOD) {
				if let Ok(method) = http::Method::from_bytes(req_method.as_bytes()) {
					if !config.allowed_methods.contains(&method) {
						return Err(err_code!(
							API_CORS_METHOD_NOT_ALLOWED,
							allowed_methods = allowed_methods_str
						));
					}
				} else {
					tracing::warn!("failed to decode method header");
					return Err(err_code!(
						API_CORS_METHOD_NOT_ALLOWED,
						allowed_methods = allowed_methods_str
					));
				}
			} else {
				tracing::warn!("preflight request missing access-control-request-method header");
				return Err(err_code!(
					API_CORS_METHOD_NOT_ALLOWED,
					allowed_methods = allowed_methods_str
				));
			}

			// Verify headers
			if let Some(acr_headers) = headers.get(header::ACCESS_CONTROL_REQUEST_HEADERS) {
				let acr_headers = acr_headers
					.to_str()
					.map_err(|_| err_code!(API_BAD_HEADERS))?;

				for header in acr_headers.split(',') {
					let header = header.trim().to_lowercase();

					if !config.text_headers.contains(&header) {
						return Err(err_code!(API_CORS_HEADER_NOT_ALLOWED, header = header));
					}
				}
			}

			// Add CORS headers
			let max_age = Duration::from_secs(60 * 60);
			cors_headers.typed_insert(
				config
					.allowed_headers
					.iter()
					.cloned()
					.collect::<AccessControlAllowHeaders>(),
			);
			cors_headers.typed_insert(
				config
					.allowed_methods
					.iter()
					.cloned()
					.collect::<AccessControlAllowMethods>(),
			);
			cors_headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin.clone());
			cors_headers.typed_insert(AccessControlMaxAge::from(max_age));

			Ok(CorsResponse::Preflight(cors_headers))
		}
		// Verify any other request
		(Some(origin), _) => {
			// Verify origin
			if !config.matches_origin(origin.to_str()?) {
				return Err(err_code!(API_CORS_ORIGIN_NOT_ALLOWED));
			}

			// Modify headers
			cors_headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin.clone());

			Ok(CorsResponse::Regular(cors_headers))
		}
		// No origin header
		(None, _) => Ok(CorsResponse::NoCors),
	}
}

/// Converts a TOKEN_EXPIRED error into a CLAIMS_ENTITLEMENT_EXPIRED error.
pub fn as_auth_expired<T>(res: GlobalResult<T>) -> GlobalResult<T> {
	match res {
		Err(err) if err.is(formatted_error::code::TOKEN_EXPIRED) => {
			Err(err_code!(CLAIMS_ENTITLEMENT_EXPIRED))
		}
		_ => res,
	}
}

pub async fn basic_rate_limit(
	rate_limit_ctx: crate::auth::AuthRateLimitCtx<'_>,
) -> GlobalResult<()> {
	if let Some(remote_address) = rate_limit_ctx.remote_address {
		let config = rate_limit_ctx.rate_limit_config;

		// Trigger rate limit
		let rate_limit_key = config.key.to_owned();
		let rate_limit_results = rate_limit_ctx
			.cache
			.rate_limit(
				&("req", config.key.to_owned()),
				&remote_address.to_string(),
				config,
			)
			.await;

		// Decode bypass token
		if let Some(bypass_token) = rate_limit_ctx.bypass_token {
			as_auth_expired(rivet_claims::decode(&bypass_token)?)?.as_bypass()?;
		}
		// Handle rate limiting
		else if let Some(rate_limit_result) = rate_limit_results.iter().find(|res| !res.is_valid)
		{
			tracing::info!(
				?remote_address,
				?rate_limit_key,
				result=%rate_limit_result,
				"too many requests"
			);

			return Err(err_code!(API_RATE_LIMIT {
				metadata: rate_limit_result.retry_after_ts(),
			}));
		}
	}

	return Ok(());
}
