// TODO: Get working again, env issue

use std::sync::Once;

use chirp_worker::prelude::*;
use http::{header, method::Method};
use hyper::{Body, Request, Response};
use uuid::Uuid;

use ::api_helper::{
	auth::ApiAuth,
	error::handle_rejection,
	macro_util, routes,
	util::{verify_cors, CorsConfigBuilder, CorsResponse},
};

// Simulated ctx auth
pub struct Auth {
	claims: Option<String>,
}

impl ApiAuth for Auth {
	fn new(api_token: &str) -> GlobalResult<Auth> {
		Ok(Auth {
			claims: Some(api_token.to_owned()),
		})
	}

	fn empty() -> Self {
		Auth { claims: None }
	}
}

static GLOBAL_INIT: Once = Once::new();

lazy_static::lazy_static! {
	static ref SERVICE_NAME: String = "api-helper-test".to_owned();
}

fn init_tracing() {
	GLOBAL_INIT.call_once(|| {
		tracing_subscriber::fmt()
			.pretty()
			.with_max_level(tracing::Level::INFO)
			.with_target(false)
			.without_time()
			.init();
	});
}

#[tokio::test(flavor = "multi_thread")]
pub async fn cors() {
	init_tracing();

	// No cors
	{
		let req = Request::builder()
			.method(Method::GET)
			.body(Body::from(""))
			.unwrap();

		assert!(
			matches!(
				verify_cors(
					&req,
					&(CorsConfigBuilder {
						origins: &["https://test.com"],
						methods: &["GET", "POST", "PUT", "DELETE"],
						headers: &["Content-Type", "Authorization"],
						credentials: true,
					})
					.build()
				)
				.expect("cors validation failed"),
				CorsResponse::NoCors
			),
			"wrong cors response"
		);
	}

	// Preflight
	{
		let req = Request::builder()
			.method(Method::OPTIONS)
			.header(header::ORIGIN, "https://test.com")
			.header(
				header::ACCESS_CONTROL_REQUEST_METHOD,
				Method::GET.to_string(),
			)
			.body(Body::from(""))
			.unwrap();

		assert!(
			matches!(
				verify_cors(
					&req,
					&(CorsConfigBuilder {
						origins: &["https://test.com"],
						methods: &["GET", "POST", "PUT", "DELETE"],
						headers: &["Content-Type", "Authorization"],
						credentials: true,
					})
					.build()
				)
				.expect("cors validation failed"),
				CorsResponse::Preflight(_)
			),
			"wrong cors response"
		);

		// Invalid method, origin, and headers
		let req = Request::builder()
			.method(Method::OPTIONS)
			.header(header::ORIGIN, "https://nottest.com")
			.header(
				header::ACCESS_CONTROL_REQUEST_METHOD,
				Method::HEAD.to_string(),
			)
			.header(header::ACCESS_CONTROL_REQUEST_HEADERS, "bad,headers")
			.body(Body::from(""))
			.unwrap();

		verify_cors(
			&req,
			&(CorsConfigBuilder {
				origins: &["https://test.com"],
				methods: &["GET", "POST", "PUT", "DELETE"],
				headers: &["Content-Type", "Authorization"],
				credentials: true,
			})
			.build(),
		)
		.expect_err("did not catch error");
	}

	// Regular
	{
		let req = Request::builder()
			.method(Method::GET)
			.header(header::ORIGIN, "https://test.com")
			.body(Body::from(""))
			.unwrap();

		assert!(
			matches!(
				verify_cors(
					&req,
					&(CorsConfigBuilder {
						origins: &["https://test.com"],
						methods: &["GET", "POST", "PUT", "DELETE"],
						headers: &["Content-Type", "Authorization"],
						credentials: true,
					})
					.build()
				)
				.expect("cors validation failed"),
				CorsResponse::Regular(_)
			),
			"wrong cors response"
		);

		// Invalid origin
		let req = Request::builder()
			.method(Method::GET)
			.header(header::ORIGIN, "https://nottest.com")
			.body(Body::from(""))
			.unwrap();

		verify_cors(
			&req,
			&(CorsConfigBuilder {
				origins: &["https://test.com"],
				methods: &["GET", "POST", "PUT", "DELETE"],
				headers: &["Content-Type", "Authorization"],
				credentials: true,
			})
			.build(),
		)
		.expect_err("did not catch error");
	}
}

#[tokio::test(flavor = "multi_thread")]
pub async fn auth() {
	init_tracing();
	set_env_vars().await;

	// Normal ctx
	{
		let req = Request::builder()
			.method(Method::GET)
			.header(header::ORIGIN, "https://test.com")
			.header(header::AUTHORIZATION, "Bearer token")
			.header("x-forwarded-for", "127.0.0.1")
			.body(Body::from(""))
			.unwrap();

		let pools = rivet_pools::from_env("api-helper-test").await.unwrap();
		let shared_client =
			chirp_client::SharedClient::from_env(pools.clone()).expect("create client");
		let cache = rivet_cache::CacheInner::from_env(pools.clone()).expect("create cache");

		if let Err(_) = macro_util::__with_ctx::<Auth>(
			shared_client,
			cache,
			&req,
			Uuid::new_v4(),
			false,
			false,
			false,
			"api-helper-test".to_owned(),
			32,
		)
		.await
		{
			panic!("failed to create ctx");
		}
	}

	// No auth or x-forwarded-for
	{
		let req = Request::builder()
			.method(Method::GET)
			.header(header::ORIGIN, "https://test.com")
			.body(Body::from(""))
			.unwrap();

		let pools = rivet_pools::from_env("api-helper-test").await.unwrap();
		let shared_client =
			chirp_client::SharedClient::from_env(pools.clone()).expect("create client");
		let cache = rivet_cache::CacheInner::from_env(pools.clone()).expect("create cache");

		if let Ok(_) = macro_util::__with_ctx::<Auth>(
			shared_client,
			cache,
			&req,
			Uuid::new_v4(),
			false,
			false,
			false,
			"api-helper-test".to_owned(),
			32,
		)
		.await
		{
			panic!("did not catch error");
		}
	}
}

#[tokio::test(flavor = "multi_thread")]
pub async fn responses() {
	init_tracing();
	set_env_vars().await;

	let pools = rivet_pools::from_env("api-helper-test").await.unwrap();
	let shared_client = chirp_client::SharedClient::from_env(pools.clone()).expect("create client");
	let cache = rivet_cache::CacheInner::from_env(pools.clone()).expect("create cache");

	// Test ray id header
	{
		let ray_id = Uuid::new_v4();
		let mut res = handle(
			shared_client.clone(),
			cache.clone(),
			ray_id,
			Request::builder()
				.method("GET")
				.uri("/empty")
				.body(Body::from(""))
				.unwrap(),
		)
		.await
		.unwrap();

		// Normally this code is run in the `api_helper::start` function
		res.headers_mut()
			.insert("rvt-ray-id", ray_id.to_string().parse().unwrap());

		assert_eq!(res.status(), http::StatusCode::OK, "request failed");
		res.headers().get("rvt-ray-id").expect("no ray id header");
	}

	// Invalid method
	{
		let res = handle(
			shared_client.clone(),
			cache.clone(),
			Uuid::new_v4(),
			Request::builder()
				.method("POST")
				.uri("/empty")
				.body(Body::from(""))
				.unwrap(),
		)
		.await
		.unwrap();

		assert_eq!(res.status(), http::StatusCode::METHOD_NOT_ALLOWED);
	}

	// Unauthorized
	{
		let res = handle(
			shared_client.clone(),
			cache.clone(),
			Uuid::new_v4(),
			Request::builder()
				.method("GET")
				.uri("/unauthorized")
				.body(Body::from(""))
				.unwrap(),
		)
		.await
		.unwrap();

		assert_eq!(res.status(), http::StatusCode::UNAUTHORIZED);
	}

	// Forbidden
	{
		let res = handle(
			shared_client.clone(),
			cache.clone(),
			Uuid::new_v4(),
			Request::builder()
				.method("GET")
				.uri("/forbidden")
				.body(Body::from(""))
				.unwrap(),
		)
		.await
		.unwrap();

		assert_eq!(res.status(), http::StatusCode::FORBIDDEN);
	}

	// Bad Request
	{
		let res = handle(
			shared_client.clone(),
			cache.clone(),
			Uuid::new_v4(),
			Request::builder()
				.method("GET")
				.uri("/bad_request")
				.body(Body::from(""))
				.unwrap(),
		)
		.await
		.unwrap();

		assert_eq!(res.status(), http::StatusCode::BAD_REQUEST);
	}

	// Not Found
	{
		let res = handle(
			shared_client.clone(),
			cache.clone(),
			Uuid::new_v4(),
			Request::builder()
				.method("GET")
				.uri("/invalid")
				.body(Body::from(""))
				.unwrap(),
		)
		.await
		.unwrap();

		assert_eq!(res.status(), http::StatusCode::NOT_FOUND);
	}
}

async fn handle(
	shared_client: chirp_client::SharedClientHandle,
	cache: rivet_cache::Cache,
	ray_id: uuid::Uuid,
	request: Request<Body>,
) -> Result<Response<Body>, http::Error> {
	// Handle route
	let mut response = match route(shared_client, cache, ray_id, request).await {
		Ok(response) => response,
		Err(err) => handle_rejection(err)?,
	};

	Ok(response)
}

async fn route(
	shared_client: chirp_client::SharedClientHandle,
	cache: rivet_cache::Cache,
	ray_id: uuid::Uuid,
	mut request: Request<Body>,
) -> GlobalResult<Response<Body>> {
	define_router! {
		routes: {
			"empty": {
				GET: test_endpoint::empty(
					opt_auth: true,
					not_using_cloudflare: true,
				),
			},
			"unauthorized": {
				GET: test_endpoint::unauthorized(
					opt_auth: true,
					not_using_cloudflare: true,
				),
			},
			"forbidden": {
				GET: test_endpoint::forbidden(
					opt_auth: true,
					not_using_cloudflare: true,
				),
			},
			"bad_request": {
				GET: test_endpoint::bad_request(
					opt_auth: true,
					not_using_cloudflare: true,
				),
			},
		},
	}
}

// Test endpoint module
mod test_endpoint {
	use super::Auth;
	use ::api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
	use chirp_worker::prelude::*;
	use serde::{Deserialize, Serialize};

	#[derive(Debug, Serialize, Deserialize)]
	pub struct TestResponse {}

	// MARK: GET /empty
	pub async fn empty(
		_ctx: Ctx<Auth>,
		_watch_index_query: WatchIndexQuery,
	) -> GlobalResult<TestResponse> {
		Ok(TestResponse {})
	}

	// MARK: GET /unauthorized
	pub async fn unauthorized(
		_ctx: Ctx<Auth>,
		_watch_index_query: WatchIndexQuery,
	) -> GlobalResult<TestResponse> {
		bail_with!(API_UNAUTHORIZED);
	}

	// MARK: GET /forbidden
	pub async fn forbidden(
		_ctx: Ctx<Auth>,
		_watch_index_query: WatchIndexQuery,
	) -> GlobalResult<TestResponse> {
		bail_with!(API_FORBIDDEN, reason = "forbidden");
	}

	// MARK: GET /bad_request
	pub async fn bad_request(
		_ctx: Ctx<Auth>,
		_watch_index_query: WatchIndexQuery,
	) -> GlobalResult<TestResponse> {
		bail_with!(API_BAD_QUERY, error = "something");
	}
}

// Set env vars
async fn set_env_vars() {
	std::env::set_var("RIVET_SOURCE_HASH", "00000000");
	std::env::set_var("RIVET_DOMAIN_MAIN", "127.0.0.1:8080");

	std::env::set_var("CHIRP_SERVICE_NAME", &*SERVICE_NAME);

	std::env::set_var("NATS_URL", todo!());
	std::env::set_var("NATS_USERNAME", "chirp");
	std::env::set_var("NATS_PASSWORD", "password");
}
