use rivet_pools::prelude::*;
use std::{
	env,
	sync::{
		atomic::{AtomicBool, AtomicI64, Ordering},
		Arc,
	},
};
use std::{fmt::Debug, time::Instant};
use tokio::sync::RwLock;
use tracing::Instrument;
use types::rivet::perf;
use uuid::Uuid;

#[derive(thiserror::Error, Debug)]
pub enum PerfCtxInnerError {
	#[error("missing env var: {0}")]
	MissingEnvVar(String),
}

#[derive(Clone)]
pub struct PerfSpan {
	/// This label is used as a metric value in Prometheus, so we use a static
	/// string to ensure a low cardinality by requiring hardcoded labels.
	label: &'static str,
	base_ts: Instant,
	start_ts: i64,
	finish_ts: Arc<AtomicI64>,
	req_id: Option<Uuid>,
}

impl PerfSpan {
	#[tracing::instrument(skip_all)]
	fn new(base_ts: Instant, label: &'static str, req_id: Option<Uuid>) -> Self {
		tracing::trace!(%label, "perf span start");
		PerfSpan {
			base_ts,
			label,
			start_ts: base_ts.elapsed().as_nanos() as i64,
			finish_ts: Arc::new(AtomicI64::new(0)),
			req_id,
		}
	}

	#[tracing::instrument(skip_all)]
	pub fn end(self) {
		tracing::trace!(label = %self.label, dt = self.base_ts.elapsed().as_secs_f64(), "perf span finish");

		let dt = self.base_ts.elapsed().as_nanos() as i64;

		// Store finish
		self.finish_ts.store(dt, Ordering::SeqCst);

		// Record span
		let dt_secs = dt as f64 / 1000. / 1000. / 1000.;
		crate::metrics::CHIRP_PERF_DURATION
			.with_label_values(&[self.label])
			.observe(dt_secs);
	}
}

impl From<PerfSpan> for perf::Span {
	fn from(val: PerfSpan) -> Self {
		perf::Span {
			label: val.label.to_owned(),
			start_ts: val.start_ts,
			finish_ts: match val.finish_ts.load(Ordering::SeqCst) {
				0 => None,
				x => Some(x),
			},
			req_id: val.req_id.map(Into::into),
		}
	}
}

#[derive(Clone)]
pub struct PerfMark {
	label: String,
	ts: i64,
	ray_id: Option<Uuid>,
	req_id: Option<Uuid>,
}

impl PerfMark {
	fn new(base_ts: Instant, label: String, ray_id: Option<Uuid>, req_id: Option<Uuid>) -> Self {
		tracing::trace!(%label, "perf mark");
		PerfMark {
			label,
			ts: base_ts.elapsed().as_nanos() as i64,
			ray_id,
			req_id,
		}
	}
}

impl From<PerfMark> for perf::Mark {
	fn from(val: PerfMark) -> Self {
		perf::Mark {
			label: val.label.to_owned(),
			ts: val.ts,
			ray_id: val.ray_id.map(Into::into),
			req_id: val.req_id.map(Into::into),
		}
	}
}

pub type PerfCtx = Arc<PerfCtxInner>;

pub struct PerfCtxInner {
	#[allow(unused_imports, dead_code)]
	redis_conn: RedisPool,

	base_ts: Instant,
	perf_spans: Arc<RwLock<Vec<PerfSpan>>>,
	perf_marks: Arc<RwLock<Vec<PerfMark>>>,

	ts: i64,
	req_id: Uuid,
	ray_id: Uuid,
}

impl Debug for PerfCtxInner {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("PerfCtxInner")
			.field("ts", &self.ts)
			.field("req_id", &self.req_id)
			.field("ray_id", &self.ray_id)
			.finish_non_exhaustive()
	}
}

impl PerfCtxInner {
	pub fn new(redis_conn: RedisPool, ts: i64, req_id: Uuid, ray_id: Uuid) -> Self {
		PerfCtxInner {
			redis_conn,
			base_ts: Instant::now(),
			perf_spans: Arc::new(RwLock::new(Vec::new())),
			perf_marks: Arc::new(RwLock::new(Vec::new())),
			ts,
			req_id,
			ray_id,
		}
	}

	/// Export perf entries as a proto message.
	#[tracing::instrument]
	pub async fn perf_data(
		&self,
		ts: i64,
		req_id: Uuid,
	) -> Result<perf::SvcPerf, PerfCtxInnerError> {
		Ok(perf::SvcPerf {
			// TODO: This should reflect the worker name, not the service name
			context_name: env::var("CHIRP_SERVICE_NAME")
				.map_err(|_| PerfCtxInnerError::MissingEnvVar("CHIRP_SERVICE_NAME".to_owned()))?,
			ts,
			duration: self.base_ts.elapsed().as_nanos() as i64,
			req_id: Some(req_id.into()),
			spans: self
				.perf_spans
				.read()
				.await
				.clone()
				.into_iter()
				.map(Into::<perf::Span>::into)
				.collect::<Vec<_>>(),
			marks: self
				.perf_marks
				.read()
				.await
				.clone()
				.into_iter()
				.map(Into::into)
				.collect::<Vec<_>>(),
		})
	}

	/// Start a new performance measurement time span
	#[tracing::instrument]
	pub async fn start(&self, label: &'static str) -> PerfSpan {
		let span = PerfSpan::new(self.base_ts, label, None);

		self.perf_spans.write().await.push(span.clone());

		span
	}

	/// Mark a point in time with a given label
	pub fn mark(&self, label: impl Into<String>) {
		let base_ts = self.base_ts;
		let perf_marks = self.perf_marks.clone();
		let label = Into::<String>::into(label);

		tracing::trace!(%label, "perf mark");

		let spawn_res = tokio::task::Builder::new()
			.name("chirp_perf::perf_mark_async")
			.spawn(
				async move {
					perf_marks
						.write()
						.await
						.push(PerfMark::new(base_ts, label, None, None));
				}
				.instrument(tracing::info_span!("perf_mark_async")),
			);
		if let Err(err) = spawn_res {
			tracing::error!(?err, "failed to spawn perf_mark_async task");
		}
	}

	/// Start a new performance measurement time span for an rpc call
	#[tracing::instrument]
	pub async fn start_rpc(&self, context_name: &'static str, req_id: Uuid) -> PerfSpan {
		let span = PerfSpan::new(self.base_ts, context_name, Some(req_id));

		self.perf_spans.write().await.push(span.clone());

		span
	}

	/// Mark a point in time with a given label
	#[tracing::instrument(skip(label))]
	pub fn mark_rpc(&self, label: impl Into<String>, ray_id: Uuid, req_id: Uuid) {
		let base_ts = self.base_ts;
		let perf_marks = self.perf_marks.clone();
		let label = Into::<String>::into(label);

		let spawn_res = tokio::task::Builder::new()
			.name("chirp_perf::perf_mark_rpc_async")
			.spawn(
				async move {
					perf_marks.write().await.push(PerfMark::new(
						base_ts,
						label,
						Some(ray_id),
						Some(req_id),
					));
				}
				.instrument(tracing::info_span!("perf_mark_rpc_async")),
			);
		if let Err(err) = spawn_res {
			tracing::error!(?err, "failed to spawn perf_mark_rpc_async task");
		}
	}
}
