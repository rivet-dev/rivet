use std::{convert::Infallible, net::SocketAddr, sync::Arc};

use hyper::{
	server::Server,
	service::{make_service_fn, service_fn},
	Body, Request, Response,
};

#[derive(Clone, Default)]
pub struct Config {
	pub pools: Option<rivet_pools::Pools>,
}

#[derive(Debug, thiserror::Error)]
pub enum HealthCheckError {
	#[error("missing required config value")]
	MissingConfigValue,
}

#[tracing::instrument(skip_all)]
pub async fn run_standalone(config: Config) {
	let config = Arc::new(config);
	let port: u16 = std::env::var("HEALTH_PORT")
		.ok()
		.and_then(|v| v.parse::<u16>().ok())
		.unwrap();
	let addr = SocketAddr::from(([0, 0, 0, 0], port));
	let make_service = make_service_fn(|_conn| {
		let config = config.clone();
		async move {
			let config = config.clone();
			Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
				let config = config.clone();
				async move { handle_infallible(&*config, req).await }
			}))
		}
	});
	let server = Server::bind(&addr).serve(make_service);
	if let Err(e) = server.await {
		eprintln!("server error: {}", e);
	}
}

#[tracing::instrument(skip_all)]
async fn handle_infallible(
	config: &Config,
	req: Request<Body>,
) -> Result<Response<Body>, Infallible> {
	match handle(config, req).await {
		Ok(res) => Ok(res),
		Err(_) => Ok(Response::builder()
			.status(404)
			.body(Body::empty())
			.expect("build request")),
	}
}

#[tracing::instrument(skip_all)]
pub async fn handle(config: &Config, req: Request<Body>) -> Result<Response<Body>, Request<Body>> {
	let res = if req.uri().path() == "/health/liveness" {
		status::liveness::route().await
	} else if let (Some(pools), "/health/essential") = (config.pools.clone(), req.uri().path()) {
		status::essential::route(pools).await
	} else if let Some(crdb_pool) = req
		.uri()
		.path()
		.strip_prefix("/health/crdb")
		.and_then(|_| config.pools.as_ref().and_then(|p| p.crdb().ok()))
	{
		status::crdb::route(crdb_pool).await
	} else if let Some(redis_pool) = req
		.uri()
		.path()
		.strip_prefix("/health/redis/")
		.and_then(|x| config.pools.as_ref().and_then(|p| p.redis(x).ok()))
	{
		status::redis::route(redis_pool).await
	} else if let Some(nats) = req
		.uri()
		.path()
		.strip_prefix("/health/nats")
		.and_then(|_| config.pools.as_ref().and_then(|p| p.nats().ok()))
	{
		status::nats::route(nats).await
	} else {
		return Err(req);
	};

	Ok(res)
}

mod status {
	use hyper::{Body, Response};
	use serde::Serialize;

	pub type StatusResponse = Response<Body>;

	#[derive(Serialize)]
	#[serde(tag = "type")]
	enum Status<T: Serialize> {
		Ok(T),
		Err {
			ty: String,
			message: String,
			debug: String,
		},
	}

	impl<T> Status<T>
	where
		T: Serialize,
	{
		fn ok(data: T) -> StatusResponse
		where
			T: Serialize,
		{
			Response::builder()
				.status(200)
				.body(Body::from(
					serde_json::to_vec(&Status::Ok(data)).expect("serialize status"),
				))
				.expect("build request")
		}

		fn err<E>(err: E) -> StatusResponse
		where
			E: std::fmt::Display + std::fmt::Debug,
		{
			Response::builder()
				.status(500)
				.body(Body::from(
					serde_json::to_vec(&Status::<T>::Err {
						ty: std::any::type_name::<T>().to_owned(),
						message: format!("{}", err),
						debug: format!("{:?}", err),
					})
					.expect("serialize status"),
				))
				.expect("build request")
		}
	}

	pub mod liveness {
		use super::{Status, StatusResponse};

		#[derive(serde::Serialize)]
		struct LivenessStatus {}

		#[tracing::instrument]
		pub async fn route() -> StatusResponse {
			Status::ok(LivenessStatus {})
		}
	}

	pub mod essential {
		use super::{Status, StatusResponse};

		#[derive(serde::Serialize)]
		struct EssentialStatus {
			crdb: Option<super::crdb::CrdbStatus>,
			redis: Option<super::redis::RedisStatus>,
			nats: Option<super::nats::NatsStatus>,
		}

		#[tracing::instrument(skip_all)]
		pub async fn route(pools: rivet_pools::Pools) -> StatusResponse {
			let (crdb, redis, nats) = tokio::join!(
				async {
					if let Ok(pool) = pools.crdb() {
						Some(super::crdb::check_status(pool).await)
					} else {
						None
					}
				},
				async {
					if let Ok(pool) = pools.redis("persistent") {
						Some(super::redis::check_status(pool).await)
					} else {
						None
					}
				},
				async {
					if let Ok(pool) = pools.nats() {
						Some(super::nats::check_status(pool).await)
					} else {
						None
					}
				}
			);

			let crdb = match crdb {
				Some(Ok(status)) => Some(status),
				Some(Err(err)) => return Status::<EssentialStatus>::err(err),
				None => None,
			};
			let redis = match redis {
				Some(Ok(status)) => Some(status),
				Some(Err(err)) => return Status::<EssentialStatus>::err(err),
				None => None,
			};
			let nats = match nats {
				Some(Ok(status)) => Some(status),
				Some(Err(err)) => return Status::<EssentialStatus>::err(err),
				None => None,
			};

			Status::ok(EssentialStatus { crdb, redis, nats })
		}
	}

	pub mod crdb {
		use std::time::Instant;

		use rivet_pools::prelude::*;

		use super::{Status, StatusResponse};

		#[derive(serde::Serialize)]
		pub struct CrdbStatus {
			rtt_ns: u128,
			rtt_s: f64,
		}

		#[tracing::instrument(skip_all)]
		pub async fn check_status(pool: CrdbPool) -> Result<CrdbStatus, sqlx::Error> {
			let start = Instant::now();
			sqlx::query("SELECT 1").execute(&pool).await?;
			let rtt = Instant::now().duration_since(start);
			Ok(CrdbStatus {
				rtt_ns: rtt.as_nanos(),
				rtt_s: rtt.as_secs_f64(),
			})
		}

		#[tracing::instrument(skip_all)]
		pub async fn route(crdb_pool: CrdbPool) -> StatusResponse {
			match check_status(crdb_pool).await {
				Ok(res) => Status::ok(res),
				Err(err) => Status::<CrdbStatus>::err(err),
			}
		}
	}

	pub mod redis {
		use std::time::Instant;

		use rivet_pools::prelude::*;

		use super::{Status, StatusResponse};

		#[derive(serde::Serialize)]
		pub struct RedisStatus {
			rtt_ns: u128,
			rtt_s: f64,
		}

		#[tracing::instrument(skip_all)]
		pub async fn check_status(mut pool: RedisPool) -> Result<RedisStatus, redis::RedisError> {
			let start = Instant::now();
			redis::cmd("PING").query_async::<_, ()>(&mut pool).await?;
			let rtt = Instant::now().duration_since(start);

			Ok(RedisStatus {
				rtt_ns: rtt.as_nanos(),
				rtt_s: rtt.as_secs_f64(),
			})
		}

		#[tracing::instrument(skip_all)]
		pub async fn route(pool: RedisPool) -> StatusResponse {
			match check_status(pool).await {
				Ok(status) => Status::ok(status),
				Err(err) => Status::<RedisStatus>::err(err),
			}
		}
	}

	pub mod nats {
		use std::time::{Duration, Instant};

		use rivet_pools::prelude::*;

		use super::{Status, StatusResponse};

		#[derive(Debug)]
		pub enum StatusError {
			Timeout(tokio::time::error::Elapsed),
			Nats(nats::Error),
		}

		impl std::fmt::Display for StatusError {
			fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
				match self {
					StatusError::Timeout(e) => write!(f, "timeout: {}", e),
					StatusError::Nats(e) => write!(f, "nats: {}", e),
				}
			}
		}

		#[derive(serde::Serialize)]
		pub struct NatsStatus {
			rtt_ns: u128,
			rtt_s: f64,
		}

		#[tracing::instrument(skip_all)]
		pub async fn check_status(pool: NatsPool) -> Result<NatsStatus, StatusError> {
			let start = Instant::now();
			tokio::time::timeout(Duration::from_secs(10), pool.flush())
				.await
				.map_err(StatusError::Timeout)?
				.map_err(|err| StatusError::Nats(err.into()))?;
			let rtt = Instant::now().duration_since(start);
			Ok(NatsStatus {
				rtt_ns: rtt.as_nanos(),
				rtt_s: rtt.as_secs_f64(),
			})
		}

		#[tracing::instrument(skip_all)]
		pub async fn route(pool: NatsPool) -> StatusResponse {
			match check_status(pool).await {
				Ok(res) => Status::ok(res),
				Err(err) => Status::<NatsStatus>::err(err),
			}
		}
	}
}
