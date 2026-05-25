use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use rand::Rng;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, JsValue};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;

/// Convert an ID (byte slice) to a hex string.
pub fn id_to_str(id: &[u8]) -> String {
	hex::encode(id)
}

pub fn display_id(id: &[u8]) -> DisplayId<'_> {
	DisplayId(id)
}

#[derive(Clone, Copy)]
pub struct DisplayId<'a>(&'a [u8]);

impl std::fmt::Display for DisplayId<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		for byte in self.0 {
			write!(f, "{byte:02x}")?;
		}
		Ok(())
	}
}

/// Stringify an error for logging.
pub fn stringify_error(error: &anyhow::Error) -> String {
	format!("{error:#}")
}

/// Error returned when the envoy is shutting down.
#[derive(Debug)]
pub struct EnvoyShutdownError;

impl std::fmt::Display for EnvoyShutdownError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "envoy shut down")
	}
}

impl std::error::Error for EnvoyShutdownError {}

/// Error returned when a sent remote SQLite request may have completed but the
/// WebSocket closed before the response arrived.
#[derive(Debug)]
pub struct RemoteSqliteIndeterminateResultError {
	pub operation: &'static str,
}

impl std::fmt::Display for RemoteSqliteIndeterminateResultError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"remote sqlite {} result is indeterminate after envoy disconnect",
			self.operation
		)
	}
}

impl std::error::Error for RemoteSqliteIndeterminateResultError {}

/// Inject artificial latency for testing.
pub async fn inject_latency(ms: Option<u64>) {
	if let Some(ms) = ms {
		if ms > 0 {
			sleep(Duration::from_millis(ms)).await;
		}
	}
}

#[cfg(not(target_arch = "wasm32"))]
pub type SleepFuture = Pin<Box<dyn Future<Output = ()> + Send>>;
#[cfg(target_arch = "wasm32")]
pub type SleepFuture = Pin<Box<dyn Future<Output = ()>>>;

pub fn boxed_sleep(duration: Duration) -> SleepFuture {
	Box::pin(sleep(duration))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn sleep(duration: Duration) {
	tokio::time::sleep(duration).await;
}

#[cfg(target_arch = "wasm32")]
pub async fn sleep(duration: Duration) {
	let delay_ms = duration.as_millis().min(u32::MAX as u128) as f64;
	let promise = js_sys::Promise::new(&mut |resolve, _reject| {
		let global = js_sys::global();
		let set_timeout = js_sys::Reflect::get(&global, &JsValue::from_str("setTimeout"))
			.ok()
			.and_then(|value| value.dyn_into::<js_sys::Function>().ok());

		if let Some(set_timeout) = set_timeout {
			let _ = set_timeout.call2(&global, &resolve, &JsValue::from_f64(delay_ms));
		} else {
			let _ = resolve.call0(&JsValue::UNDEFINED);
		}
	});

	let _ = JsFuture::from(promise).await;
}

#[cfg(not(target_arch = "wasm32"))]
pub fn spawn_detached<F>(future: F)
where
	F: Future<Output = ()> + Send + 'static,
{
	tokio::spawn(future);
}

#[cfg(target_arch = "wasm32")]
pub fn spawn_detached<F>(future: F)
where
	F: Future<Output = ()> + 'static,
{
	tokio::task::spawn_local(future);
}

pub struct BackoffOptions {
	pub initial_delay: u64,
	pub max_delay: u64,
	pub multiplier: f64,
	pub jitter: bool,
}

impl Default for BackoffOptions {
	fn default() -> Self {
		Self {
			initial_delay: 1000,
			max_delay: 30000,
			multiplier: 2.0,
			jitter: true,
		}
	}
}

pub fn calculate_backoff(attempt: u32, options: &BackoffOptions) -> Duration {
	let delay = (options.initial_delay as f64 * options.multiplier.powi(attempt as i32))
		.min(options.max_delay as f64);

	let delay = if options.jitter {
		let jitter = rand::thread_rng().gen_range(0.0..0.25);
		delay * (1.0 + jitter)
	} else {
		delay
	};

	Duration::from_millis(delay as u64)
}

pub struct ParsedCloseReason {
	pub group: String,
	pub error: String,
	pub ray_id: Option<String>,
}

pub fn parse_ws_close_reason(reason: &str) -> Option<ParsedCloseReason> {
	let (main_part, ray_id) = match reason.split_once('#') {
		Some((main, ray)) => (main, Some(ray.to_string())),
		None => (reason, None),
	};

	let (group, error) = main_part.split_once('.')?;

	if group.is_empty() || error.is_empty() {
		tracing::warn!(%reason, "failed to parse close reason");
		return None;
	}

	Some(ParsedCloseReason {
		group: group.to_string(),
		error: error.to_string(),
		ray_id,
	})
}

const U16_MAX: u32 = 65535;

pub fn wrapping_add_u16(a: u16, b: u16) -> u16 {
	a.wrapping_add(b)
}

pub fn wrapping_sub_u16(a: u16, b: u16) -> u16 {
	a.wrapping_sub(b)
}

pub fn wrapping_gt_u16(a: u16, b: u16) -> bool {
	a != b && (a.wrapping_sub(b) as u32) < U16_MAX / 2
}

pub fn wrapping_lt_u16(a: u16, b: u16) -> bool {
	a != b && (b.wrapping_sub(a) as u32) < U16_MAX / 2
}

pub fn wrapping_gte_u16(a: u16, b: u16) -> bool {
	a == b || wrapping_gt_u16(a, b)
}

pub fn wrapping_lte_u16(a: u16, b: u16) -> bool {
	a == b || wrapping_lt_u16(a, b)
}

pub type BufferMapKey = [u8; 8];

pub fn tunnel_request_key(gateway_id: &[u8; 4], request_id: &[u8; 4]) -> BufferMapKey {
	let mut key = [0u8; 8];
	key[..4].copy_from_slice(gateway_id);
	key[4..].copy_from_slice(request_id);
	key
}

/// Hash-map keyed by fixed tunnel request keys.
pub struct BufferMap<T> {
	inner: HashMap<BufferMapKey, T>,
}

impl<T> BufferMap<T> {
	pub fn new() -> Self {
		Self {
			inner: HashMap::new(),
		}
	}

	pub fn get(&self, key: BufferMapKey) -> Option<&T> {
		self.inner.get(&key)
	}

	pub fn get_mut(&mut self, key: BufferMapKey) -> Option<&mut T> {
		self.inner.get_mut(&key)
	}

	pub fn insert(&mut self, key: BufferMapKey, value: T) {
		self.inner.insert(key, value);
	}

	pub fn remove(&mut self, key: BufferMapKey) -> Option<T> {
		self.inner.remove(&key)
	}

	pub fn contains_key(&self, key: BufferMapKey) -> bool {
		self.inner.contains_key(&key)
	}
}

impl<T> Default for BufferMap<T> {
	fn default() -> Self {
		Self::new()
	}
}
