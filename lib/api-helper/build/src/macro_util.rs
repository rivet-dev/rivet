use std::{net::IpAddr, str::FromStr};

use global_error::prelude::*;
use headers::{Cookie, HeaderMapExt};
use http::header::AsHeaderName;
use hyper::{
	body::{Bytes, HttpBody},
	header, Body, Request,
};
use rivet_operation::prelude::util;
use serde::de::DeserializeOwned;
use url::Url;
use uuid::Uuid;

use crate::{
	auth::{self, AuthRateLimitCtx},
	ctx::Ctx,
};

pub mod __metrics {
	pub use crate::metrics::*;
}

const MAX_ALLOWED_BODY_SIZE: u64 = rivet_util::file_size::gibibytes(10);
const BEARER: &str = "Bearer ";

// For code legibility
#[doc(hidden)]
pub struct __RouterConfig {
	pub route: url::Url,
	pub path_segments: Vec<String>,
	pub prefix: Option<&'static str>,
}

#[doc(hidden)]
impl __RouterConfig {
	pub fn new(uri: &hyper::Uri) -> GlobalResult<Self> {
		// This url doesn't actually represent the url of the request, it's just put here so that the
		// URI can be parsed by url::Url::parse
		let url = format!("{}{}", util::env::origin_api(), uri);
		let route = url::Url::parse(url.as_str())?;

		Ok(__RouterConfig {
			route: route.clone(),
			path_segments: route
				.path_segments()
				// Store it backwards for more efficient `pop`ing
				.map(|ps| ps.rev().map(ToString::to_string).collect::<Vec<_>>())
				.unwrap_or_default(),
			prefix: None,
		})
	}

	/// If the current prefix matches the first element in the path segments list, it is removed and
	/// returns true. Always true if no prefix is set.
	pub fn try_prefix(&mut self) -> bool {
		let Some(prefix) = &self.prefix else {
			return true;
		};

		match self.path_segments.last() {
			Some(segment) if segment == prefix => {
				self.path_segments.pop();
				true
			}
			_ => false,
		}
	}
}

// Allows for the use of async functions in `.or_else` (`.try_or_else` in this case)
#[doc(hidden)]
pub enum __AsyncOption<T> {
	Some(T),
	None,
}

#[doc(hidden)]
impl<T> __AsyncOption<T> {
	pub async fn try_or_else<F, Fut>(self, f: F) -> GlobalResult<__AsyncOption<T>>
	where
		F: FnOnce() -> Fut,
		Fut: std::future::Future<Output = GlobalResult<__AsyncOption<T>>>,
	{
		match self {
			__AsyncOption::Some(_) => Ok(self),
			__AsyncOption::None => f().await,
		}
	}

	pub fn ok_or_else<E, F: FnOnce() -> E>(self, err: F) -> Result<T, E> {
		match self {
			__AsyncOption::Some(v) => Ok(v),
			__AsyncOption::None => Err(err()),
		}
	}
}

#[doc(hidden)]
impl<T, E> __AsyncOption<Result<T, E>> {
	pub fn transpose(self) -> Result<__AsyncOption<T>, E> {
		match self {
			__AsyncOption::Some(Ok(x)) => Ok(__AsyncOption::Some(x)),
			__AsyncOption::Some(Err(e)) => Err(e),
			__AsyncOption::None => Ok(__AsyncOption::None),
		}
	}
}

#[doc(hidden)]
impl<T> From<__AsyncOption<T>> for Option<T> {
	fn from(value: __AsyncOption<T>) -> Option<T> {
		match value {
			__AsyncOption::Some(inner) => Some(inner),
			__AsyncOption::None => None,
		}
	}
}

#[doc(hidden)]
impl<T> From<Option<T>> for __AsyncOption<T> {
	fn from(value: Option<T>) -> __AsyncOption<T> {
		match value {
			Some(inner) => __AsyncOption::Some(inner),
			None => __AsyncOption::None,
		}
	}
}

#[doc(hidden)]
pub fn __validate_body(request: &mut Request<Body>) -> GlobalResult<()> {
	match request.body().size_hint().upper() {
		Some(request_content_length) => {
			if request_content_length < MAX_ALLOWED_BODY_SIZE {
				Ok(())
			} else {
				Err(err_code!(
					API_BODY_TOO_LARGE,
					content_length = request_content_length
				))
			}
		}
		None => Err(err_code!(API_BAD_BODY)),
	}
}

#[doc(hidden)]
pub async fn __read_body_bytes(request: &mut Request<Body>) -> GlobalResult<Bytes> {
	__validate_body(request)?;

	hyper::body::to_bytes(request.body_mut())
		.await
		.map_err(Into::into)
}

#[doc(hidden)]
pub async fn __deserialize_body<T: DeserializeOwned + Send>(
	request: &mut Request<Body>,
) -> GlobalResult<T> {
	__validate_body(request)?;

	// Read bytes
	let bytes_raw = hyper::body::to_bytes(request.body_mut()).await?;

	// Add default empty JSON body if no bytes provided
	let bytes = if bytes_raw.is_empty() {
		b"{}"
	} else {
		bytes_raw.as_ref()
	};

	// Deserialize bytes
	match serde_json::from_slice::<T>(bytes) {
		Ok(body) => Ok(body),
		Err(err) => {
			if bytes.len() < 1024 * 1024 {
				let body_raw = std::str::from_utf8(bytes);
				tracing::warn!(?body_raw, ?err, "failed to decode json body");
			} else {
				tracing::warn!(?err, "failed to decode large json body");
			}

			Err(err_code!(API_BAD_BODY, error = err))
		}
	}
}

#[doc(hidden)]
pub fn __deserialize_optional_header<T: FromStr + Send, U: AsHeaderName>(
	request: &Request<Body>,
	header_name: U,
) -> GlobalResult<Option<T>> {
	request
		.headers()
		.get(header_name.as_str())
		.map(|header_value| {
			T::from_str(
				header_value
					.to_str()
					.map_err(|_| err_code!(API_BAD_HEADER, header = header_name.as_str()))?,
			)
			.map_err(|_| err_code!(API_BAD_HEADER, header = header_name.as_str()))
		})
		.transpose()
}

#[doc(hidden)]
pub fn __deserialize_header<T: FromStr + Send, U: Clone + AsHeaderName>(
	request: &Request<Body>,
	header_name: U,
) -> GlobalResult<T> {
	__deserialize_optional_header::<T, U>(request, header_name.clone())?
		.ok_or_else(|| err_code!(API_MISSING_HEADER, header = header_name.as_str()))
}

#[doc(hidden)]
pub fn __deserialize_cookies(request: &Request<Body>) -> Option<Cookie> {
	request.headers().typed_get::<Cookie>()
}

#[doc(hidden)]
pub fn __deserialize_query<T: DeserializeOwned + Send>(route: &Url) -> GlobalResult<T> {
	let query_string = route.query().unwrap_or_else(|| "");

	serde_array_query::from_str::<T>(query_string)
		.map_err(|err| err_code!(API_BAD_QUERY, error = err))
}

#[doc(hidden)]
pub async fn __with_ctx<A: auth::ApiAuth + Send>(
	shared_client: chirp_client::SharedClientHandle,
	pools: rivet_pools::Pools,
	cache: rivet_cache::Cache,
	request: &Request<Body>,
	ray_id: Uuid,
	optional_auth: bool,
	not_using_cloudflare: bool,
	internal_endpoint: bool,
	rate_limit_config: rivet_cache::RateLimitConfig,
) -> GlobalResult<Ctx<A>> {
	let bearer_token = __deserialize_optional_header::<String, _>(&request, header::AUTHORIZATION)?
		.and_then(|h| h.strip_prefix(BEARER).map(|h| h.to_owned()));

	// Check auth exists
	if !optional_auth && bearer_token.is_none() {
		return Err(err_code!(
			API_UNAUTHORIZED,
			reason = "No bearer token provided."
		));
	}

	let user_agent = __deserialize_optional_header::<String, _>(&request, header::USER_AGENT)?;
	let origin = __deserialize_optional_header::<Url, _>(&request, header::ORIGIN)?;

	// Parse request asn and coords. This is generated by the custom Cloudflare
	// Workers script.
	let asn_str = __deserialize_optional_header::<String, _>(&request, "x-asn")?;
	let asn = asn_str.and_then(|x| match x.parse::<u32>() {
		Ok(x) => Some(x),
		Err(err) => {
			tracing::warn!(value = ?x, ?err, "failed to parse X-ASN header to u32");
			None
		}
	});

	let coords_str = __deserialize_optional_header::<String, _>(&request, "x-coords")?;
	let coords =
		if let Some((lat_str, long_str)) = coords_str.as_ref().and_then(|x| x.split_once(",")) {
			match (lat_str.parse::<f64>(), long_str.parse::<f64>()) {
				(Ok(lat), Ok(long)) => Some((lat, long)),
				(Err(err), _) | (_, Err(err)) => {
					tracing::error!(?err, lat_str, long_str, "failed to parse lat & long");
					None
				}
			}
		} else {
			None
		};

	// Read remote address
	let remote_address = if internal_endpoint {
		// We don't have a remote address if this is an internal service
		None
	} else {
		// Decode remote address.
		//
		// We use the x-forwarded-for generated by
		// Traefik if this is exposed directly to the internet or if Cloudflare is not configured.
		//
		// Otherwise, we use the cf-connecting-ip header since that's the recommended header to
		// use by Cloudflare.
		let remote_address_str =
			if rivet_util::env::dns_provider() == Some("cloudflare") && !not_using_cloudflare {
				__deserialize_header::<String, _>(&request, "cf-connecting-ip")?
			} else {
				// TODO: This should be a comma separated list of IPs
				// TODO: Test this is correct
				// Treafik will override any user-provided provided x-forwarded-for
				// header, so we can trust this
				__deserialize_header::<String, _>(&request, "x-forwarded-for")?
					.split(",")
					.next()
					.ok_or_else(|| err_code!(API_MISSING_HEADER, header = "x-forwarded-for"))?
					.to_string()
			};
		let remote_address =
			IpAddr::from_str(&remote_address_str).map_err(|_| err_code!(API_INVALID_IP))?;

		Some(remote_address)
	};

	// Create connections
	let req_id = Uuid::new_v4();
	let ts = rivet_util::timestamp::now();
	let svc_name = rivet_util::env::chirp_service_name().to_string();
	let client = shared_client.wrap(
		req_id,
		ray_id,
		vec![chirp_client::TraceEntry {
			context_name: svc_name.clone(),
			req_id: Some(req_id.into()),
			ts,
			run_context: match rivet_util::env::run_context() {
				rivet_util::env::RunContext::Service => chirp_client::RunContext::Service,
				rivet_util::env::RunContext::Test => chirp_client::RunContext::Test,
			} as i32,
		}],
	);
	let conn = rivet_connection::Connection::new(client, pools.clone(), cache.clone());
	let op_ctx = rivet_operation::OperationContext::new(
		svc_name,
		std::time::Duration::from_secs(60),
		conn,
		req_id,
		ray_id,
		ts,
		ts,
		(),
		Vec::new(),
	);

	// Create auth
	let rate_limit_ctx = AuthRateLimitCtx {
		cache: &cache,
		rate_limit_config,
		remote_address: remote_address.as_ref(),
		// Deserialize only if rate limit will be handled
		bypass_token: remote_address
			.map(|_| __deserialize_optional_header::<String, _>(&request, "x-bypass-token"))
			.transpose()?
			.flatten(),
	};
	let auth = A::new(bearer_token, rate_limit_ctx).await?;

	Ok(Ctx {
		auth,
		op_ctx,
		user_agent,
		origin,
		remote_address,
		coords,
		asn,
	})
}
