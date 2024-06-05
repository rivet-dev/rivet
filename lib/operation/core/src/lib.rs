use std::{
	fmt::{self, Debug},
	time::{Duration, Instant},
};

use async_trait::async_trait;
use chirp_client::prelude::*;
use chirp_metrics as metrics;
use global_error::{GlobalError, GlobalResult};
use rivet_connection::Connection;
use rivet_pools::prelude::*;
use rivet_util as util;
use tracing::Instrument;

pub mod prelude;

#[async_trait]
pub trait Operation: Clone + Send + Sync + 'static {
	type Request: prost::Message + Default + Clone;
	type Response: prost::Message + Default + Clone;

	const NAME: &'static str;
	const TIMEOUT: std::time::Duration;

	async fn handle(ctx: OperationContext<Self::Request>) -> GlobalResult<Self::Response>;
}

/// Contains the context that will be passed to the worker.
#[derive(Clone)]
pub struct OperationContext<B>
where
	B: Debug + Clone,
{
	name: String,
	timeout: Duration,

	conn: Connection,
	req_id: Uuid,
	ray_id: Uuid,
	ts: i64,
	req_ts: i64,
	body: B,

	// Denotes whether this is from a workflow. Disables the ability to create workflows itself when true
	// (workflows must be created at the workflow level context)
	pub from_workflow: bool,
}

impl<B> OperationContext<B>
where
	B: Debug + Clone,
{
	pub fn new(
		name: String,
		timeout: Duration,
		conn: Connection,
		req_id: Uuid,
		ray_id: Uuid,
		ts: i64,
		req_ts: i64,
		body: B,
	) -> Self {
		OperationContext {
			name,
			timeout,
			conn,
			req_id,
			ray_id,
			ts,
			req_ts,
			body,
			from_workflow: false,
		}
	}

	/// Calls the given operation. Use the `op!` macro instead of calling this directly.
	#[tracing::instrument(err, skip_all, fields(operation = O::NAME))]
	pub async fn call<O: Operation>(&self, body: O::Request) -> GlobalResult<O::Response> {
		tracing::info!(?body, "operation call");

		// Record metrics
		metrics::CHIRP_REQUEST_PENDING
			.with_label_values(&[O::NAME])
			.inc();
		metrics::CHIRP_REQUEST_TOTAL
			.with_label_values(&[O::NAME])
			.inc();

		let start_instant = Instant::now();

		// TODO: Throw dedicated "timed out" error here
		// Process the request
		let req_op_ctx = self.wrap::<O>(body);
		let timeout_fut = tokio::time::timeout(O::TIMEOUT, O::handle(req_op_ctx).in_current_span());
		let res = tokio::task::Builder::new()
			.name("operation::handle")
			.spawn(timeout_fut)?
			.await??;

		tracing::info!(?res, "operation response");

		// Record metrics
		{
			let error_code_str = match &res {
				Err(GlobalError::Internal { ty, .. }) => {
					let err_code_str = "__UNKNOWN__".to_string();
					metrics::CHIRP_REQUEST_ERRORS
						.with_label_values(&[O::NAME, &err_code_str, &ty])
						.inc();

					err_code_str
				}
				Err(GlobalError::BadRequest { code, .. }) => {
					metrics::CHIRP_REQUEST_ERRORS
						.with_label_values(&[O::NAME, &code, "bad_request"])
						.inc();

					code.clone()
				}
				_ => String::new(),
			};

			// Other request metrics
			let dt = start_instant.elapsed().as_secs_f64();
			metrics::CHIRP_REQUEST_PENDING
				.with_label_values(&[O::NAME])
				.dec();
			metrics::CHIRP_REQUEST_DURATION
				.with_label_values(&[O::NAME, error_code_str.as_str()])
				.observe(dt);
		}

		res
	}

	/// Adds trace and correctly wraps `Connection` (and subsequently `chirp_client::Client`).
	fn wrap<O: Operation>(&self, body: O::Request) -> OperationContext<O::Request> {
		let now = util::timestamp::now();
		let req_id = Uuid::new_v4();
		let trace_entry = chirp_client::TraceEntry {
			context_name: O::NAME.to_string(),
			req_id: Some(req_id.into()),
			ts: now,
			run_context: match rivet_util::env::run_context() {
				rivet_util::env::RunContext::Service => chirp_client::RunContext::Service,
				rivet_util::env::RunContext::Test => chirp_client::RunContext::Test,
			} as i32,
		};

		OperationContext {
			name: O::NAME.to_string(),
			timeout: O::TIMEOUT,
			conn: self.conn.wrap(req_id, self.ray_id, trace_entry),
			req_id,
			ray_id: self.ray_id,
			ts: now,
			req_ts: self.req_ts,
			body,
			from_workflow: self.from_workflow,
		}
	}

	/// Clones everything but the body. This should always be used over `.clone()` unless you need to
	/// clone the body.
	pub fn base(&self) -> OperationContext<()> {
		OperationContext {
			name: self.name.clone(),
			timeout: self.timeout,
			conn: self.conn.clone(),
			req_id: self.req_id,
			ray_id: self.ray_id,
			ts: self.ts,
			req_ts: self.req_ts,
			body: (),
			from_workflow: self.from_workflow,
		}
	}

	pub fn name(&self) -> &str {
		&self.name
	}

	pub fn timeout(&self) -> Duration {
		self.timeout
	}

	pub fn req_id(&self) -> Uuid {
		self.req_id
	}

	pub fn ray_id(&self) -> Uuid {
		self.ray_id
	}

	/// Timestamp at which the request started.
	pub fn ts(&self) -> i64 {
		self.ts
	}

	/// Timestamp at which the request was published.
	pub fn req_ts(&self) -> i64 {
		self.req_ts
	}

	/// Time between when the timestamp was processed and when it was published.
	pub fn req_dt(&self) -> i64 {
		self.ts.saturating_sub(self.req_ts)
	}

	pub fn body(&self) -> &B {
		&self.body
	}

	pub fn from_workflow(&self) -> bool {
		self.from_workflow
	}

	pub fn trace(&self) -> &[chirp_client::TraceEntry] {
		self.conn.trace()
	}

	pub fn test(&self) -> bool {
		self.trace()
			.iter()
			.any(|x| x.run_context == chirp_client::RunContext::Test as i32)
	}

	pub fn op_ctx(&self) -> &OperationContext<B> {
		&self
	}

	pub fn conn(&self) -> &Connection {
		&self.conn
	}

	pub fn chirp(&self) -> &chirp_client::Client {
		self.conn.chirp()
	}

	pub fn cache(&self) -> rivet_cache::RequestConfig {
		self.conn.cache()
	}

	pub fn cache_handle(&self) -> rivet_cache::Cache {
		self.conn.cache_handle()
	}

	pub async fn crdb(&self) -> Result<CrdbPool, rivet_pools::Error> {
		self.conn.crdb().await
	}

	pub async fn redis_cache(&self) -> Result<RedisPool, rivet_pools::Error> {
		self.conn.redis_cache().await
	}

	pub async fn redis_cdn(&self) -> Result<RedisPool, rivet_pools::Error> {
		self.conn.redis_cdn().await
	}

	pub async fn redis_job(&self) -> Result<RedisPool, rivet_pools::Error> {
		self.conn.redis_job().await
	}

	pub async fn redis_mm(&self) -> Result<RedisPool, rivet_pools::Error> {
		self.conn.redis_mm().await
	}

	pub async fn redis_user_presence(&self) -> Result<RedisPool, rivet_pools::Error> {
		self.conn.redis_user_presence().await
	}

	pub fn perf(&self) -> &chirp_perf::PerfCtx {
		self.conn.perf()
	}

	pub async fn clickhouse(&self) -> GlobalResult<ClickHousePool> {
		self.conn.clickhouse().await
	}
}

impl<B> std::ops::Deref for OperationContext<B>
where
	B: Debug + Clone,
{
	type Target = B;

	fn deref(&self) -> &Self::Target {
		&self.body
	}
}

impl<B> Debug for OperationContext<B>
where
	B: Debug + Clone,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("OperationContext")
			.field("req_id", &self.req_id)
			.field("ray_id", &self.ray_id)
			.field("ts", &self.ts)
			.finish()
	}
}
