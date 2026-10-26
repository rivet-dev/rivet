//! Object-store latency canary. It issues the same small requests a worker heartbeat does,
//! through the engine's object-store adapter, and measures their raw tail: each request gets
//! one attempt and a long deadline, so a store stall shows up as its real length instead of
//! being retried away or cut at a short timeout.
//!
//! Each of two lanes rewrites its own 1 KiB object once a second with a conditional PUT on the
//! last ETag, as a heartbeat renewal does, and reads the other lane's object once a second
//! with a plain GET. Writes and reads run in separate loops so neither waits on the other. A
//! loop issues its next request only after the previous one ends, so a stall holds one
//! outstanding request per loop instead of piling up requests behind it.

pub mod metrics;
pub mod stalls;

use anyhow::{Context, Result, ensure};
use bytes::Bytes;
use futures_util::{FutureExt, future::BoxFuture};
use object_store::{
	ObjectStore, PutMode, PutPayload, PutResult, UpdateVersion, path::Path as ObjectPath,
};
use rivet_config::config::ObjectStoreBackend;
use rivet_env::{Clock, Env, TimeoutError, capabilities::shutdown::Shutdown};
use std::{future::Future, pin::pin, sync::Arc, time::Duration, time::Instant};

use self::stalls::StallTracker;

/// How long one canary request may take before it is recorded as a timeout. Far beyond any
/// lease, so stalls are measured rather than cut off.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

/// Each loop starts one request per period.
pub const PROBE_PERIOD_MS: u64 = 1_000;

/// Two lanes give 2 writes and 2 reads a second.
pub const LANES: usize = 2;

/// Size of each probe object, about a heartbeat's.
pub const OBJECT_BYTES: usize = 1024;

#[derive(Clone, Debug, clap::Args)]
pub struct Args {
	/// Region label for this canary's metrics, such as `westus2`. Run one canary per store and
	/// region.
	#[arg(long)]
	pub region: String,

	/// Store label for this canary's metrics. Defaults to the configured backend (`azure`,
	/// `s3`, `gcs` or `memory`); set it to tell storage classes apart, such as `s3-express`.
	#[arg(long)]
	pub store: Option<String>,

	/// Key prefix of the probe objects inside the configured object storage.
	#[arg(long, default_value = "canary")]
	pub prefix: String,
}

/// The fixed `store` and `region` labels of this canary process.
#[derive(Clone, Debug)]
pub struct Labels {
	pub store: String,
	pub region: String,
}

impl Labels {
	/// Validates operator-supplied labels so they stay short, fixed tokens.
	pub fn new(store: String, region: String) -> Result<Self> {
		for (name, value) in [("store", &store), ("region", &region)] {
			ensure!(
				is_label_token(value),
				"canary {name} label {value:?} must be 1 to 63 lowercase letters, digits or hyphens"
			);
		}
		Ok(Self { store, region })
	}
}

fn is_label_token(value: &str) -> bool {
	let valid_chars = value
		.chars()
		.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
	!value.is_empty() && value.len() <= 63 && valid_chars
}

/// The default store label of a configured backend.
pub fn backend_label(backend: &ObjectStoreBackend) -> &'static str {
	match backend {
		ObjectStoreBackend::InMemory => "memory",
		ObjectStoreBackend::FileSystem(_) => "file-system",
		ObjectStoreBackend::S3(_) => "s3",
		ObjectStoreBackend::Azure(_) => "azure",
		ObjectStoreBackend::Gcs(_) => "gcs",
	}
}

/// Probes `store` until shutdown. Returns an error only when the clock fails.
pub async fn run(
	env: Arc<dyn Env>,
	store: Arc<dyn ObjectStore>,
	labels: Labels,
	prefix: &str,
	mut shutdown: Shutdown,
) -> Result<()> {
	let clock = env.clock();
	let keys = (0..LANES)
		.map(|lane| ObjectPath::from(format!("{prefix}/{}/lane-{lane}", labels.region)))
		.collect::<Vec<_>>();

	let mut loops: Vec<BoxFuture<'_, Result<()>>> = Vec::new();
	for lane in 0..LANES {
		let read_key = &keys[(lane + 1) % LANES];
		loops.push(write_loop(&clock, store.as_ref(), &labels, &keys[lane]).boxed());
		loops.push(read_loop(&clock, store.as_ref(), &labels, read_key).boxed());
	}
	tracing::info!(
		store = %labels.store,
		region = %labels.region,
		lanes = LANES,
		"object-store canary started"
	);

	rivet_env::select_biased!("shutdown stops probing even while requests are outstanding",
		event = shutdown.recv() => {
			event?;
			tracing::info!("object-store canary stopping");
			Ok(())
		},
		result = futures_util::future::try_join_all(loops) => result.map(drop),
	)
}

/// Rewrites `key` once a period. A conditional PUT on the last ETag renews it; after a failed
/// write the ETag is unknown, so the next write is a plain PUT that learns a fresh one.
async fn write_loop(
	clock: &Arc<dyn Clock>,
	store: &dyn ObjectStore,
	labels: &Labels,
	key: &ObjectPath,
) -> Result<()> {
	let mut ticks = rivet_env::interval(clock.clone(), PROBE_PERIOD_MS);
	let mut version: Option<UpdateVersion> = None;
	let mut sequence = 0u64;
	loop {
		ticks.tick().await?;
		sequence += 1;

		let (op, mode) = match version.take() {
			Some(version) => ("put_conditional", PutMode::Update(version)),
			None => ("put", PutMode::Overwrite),
		};
		let request = store.put_opts(key, payload(sequence), mode.into());
		let written: Option<PutResult> = probe(clock.as_ref(), labels, op, request).await?;
		version = written.map(|written| UpdateVersion {
			e_tag: written.e_tag,
			version: written.version,
		});
	}
}

/// Reads `key` once a period, including its body.
async fn read_loop(
	clock: &Arc<dyn Clock>,
	store: &dyn ObjectStore,
	labels: &Labels,
	key: &ObjectPath,
) -> Result<()> {
	let mut ticks = rivet_env::interval(clock.clone(), PROBE_PERIOD_MS);
	loop {
		ticks.tick().await?;
		let request = async { store.get_opts(key, Default::default()).await?.bytes().await };
		probe(clock.as_ref(), labels, "get", request).await?;
	}
}

/// A 1 KiB body that differs on every write, so stores that derive ETags from content still
/// see a change.
fn payload(sequence: u64) -> PutPayload {
	let mut body = vec![b'c'; OBJECT_BYTES];
	body[..8].copy_from_slice(&sequence.to_be_bytes());
	PutPayload::from(Bytes::from(body))
}

/// Runs one canary request with the canary's deadline and records its latency and result.
/// Each stall threshold is counted when it passes, so a stall shows up while the request is
/// still outstanding. Returns the request's value when it succeeded, and an error only when
/// the clock fails.
pub async fn probe<T>(
	clock: &dyn Clock,
	labels: &Labels,
	op: &'static str,
	request: impl Future<Output = object_store::Result<T>>,
) -> Result<Option<T>> {
	let started = telemetry_now();
	let started_ms = clock.boot_ms()?;
	let deadline_ms = started_ms
		.checked_add(duration_ms(REQUEST_TIMEOUT))
		.context("canary deadline overflows the clock")?;
	let mut stalls = StallTracker::default();
	let mut request = pin!(rivet_env::timeout_at(clock, deadline_ms, request));

	let answer = loop {
		let Some(threshold) = stalls.next() else {
			break request.as_mut().await;
		};
		let wake_ms = started_ms.saturating_add(duration_ms(threshold.after));
		rivet_env::select_biased!("a request that answered is recorded before a threshold it beat",
			answer = request.as_mut() => break answer,
			slept = clock.sleep_until(wake_ms) => {
				slept?;
				count_stalls(labels, op, stalls.pass(threshold.after));
			},
		);
	};

	// Catch up on thresholds no timer counted, such as when the deadline fired first. The
	// telemetry clock has sub-millisecond resolution; the Env clock drives the deadline.
	let latency = started.elapsed();
	let clock_elapsed = Duration::from_millis(clock.boot_ms()?.saturating_sub(started_ms));
	count_stalls(labels, op, stalls.pass(latency.max(clock_elapsed)));

	let (result, value) = match answer {
		Ok(Ok(value)) => ("ok", Some(value)),
		Ok(Err(error)) => {
			tracing::debug!(op, ?error, "object-store canary request failed");
			(
				depot_worker::object_store_metrics::outcome::<T>(&Err(error)),
				None,
			)
		}
		Err(TimeoutError::Elapsed) => ("timeout", None),
		Err(TimeoutError::Clock(error)) => return Err(error.into()),
	};
	metrics::REQUEST_DURATION
		.with_label_values(&[&labels.store, &labels.region, op, result])
		.observe(latency.as_secs_f64());
	Ok(value)
}

fn count_stalls(labels: &Labels, op: &str, passed: &[stalls::StallThreshold]) {
	for threshold in passed {
		tracing::warn!(
			op,
			threshold = threshold.label,
			"object-store canary request is stalled, likely a store-side stall"
		);
		metrics::STALLS
			.with_label_values(&[&labels.store, &labels.region, op, threshold.label])
			.inc();
	}
}

fn duration_ms(duration: Duration) -> u64 {
	u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

/// A telemetry-only clock reading for request latency.
fn telemetry_now() -> Instant {
	#[allow(
		clippy::disallowed_methods,
		reason = "canary latency is a metric and uses real time"
	)]
	Instant::now()
}


