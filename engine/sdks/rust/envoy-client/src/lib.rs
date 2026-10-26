pub mod actor;
pub mod async_counter;
pub mod callbacks;
pub mod commands;
pub mod config;
pub mod connection;
pub mod context;
pub mod deadlines;
pub mod envoy;
pub mod events;
pub mod handle;
pub mod http;
pub mod kv;
pub mod latency_channel;
pub mod metrics;
pub mod sqlite;
pub mod stringify;
pub(crate) mod time {
	// Tokio's clock is the monotonic clock in production and follows a paused runtime's
	// simulated time, so request deadlines expire on the same clock their timers sleep on.
	// Reconnect jitter and ping health still read real randomness and wall time.
	#[cfg(not(target_arch = "wasm32"))]
	pub use tokio::time::Instant;
	#[cfg(target_arch = "wasm32")]
	pub use web_time::Instant;

	pub fn now_millis() -> i64 {
		#[cfg(not(target_arch = "wasm32"))]
		{
			std::time::SystemTime::now()
				.duration_since(std::time::UNIX_EPOCH)
				.expect("system clock should be after UNIX epoch")
				.as_millis() as i64
		}

		#[cfg(target_arch = "wasm32")]
		{
			js_sys::Date::now() as i64
		}
	}
}
pub mod tunnel;
pub mod utils;
pub mod websocket;

pub use rivet_envoy_protocol as protocol;
