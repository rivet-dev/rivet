use rand::Rng;
pub use rivet_util_env as env;
pub use rivet_util_macros as macros;
use tokio::time::{Duration, Instant};

pub mod billing;
pub mod check;
pub mod debug;
pub mod duration;
pub mod faker;
pub mod feature;
pub mod file_size;
pub mod format;
pub mod future;
pub mod geo;
pub mod glob;
pub mod math;
pub mod route;
pub mod sort;
pub mod timestamp;
pub mod uuid;

#[cfg(feature = "serde")]
pub mod serde {
	use aws_smithy_types::Document;
	use serde_json::Value;

	#[derive(thiserror::Error, Debug)]
	#[error("Number could not be decoded by serde_json")]
	pub struct NumberDecodeError;

	pub fn as_serde(value: &Document) -> Result<Value, NumberDecodeError> {
		let val = match value {
			Document::Object(map) => Value::Object(
				map.iter()
					.map(|(k, v)| Ok((k.clone(), as_serde(v)?)))
					.collect::<Result<_, _>>()?,
			),
			Document::Array(arr) => {
				Value::Array(arr.iter().map(as_serde).collect::<Result<_, _>>()?)
			}
			Document::Number(n) => match n {
				aws_smithy_types::Number::PosInt(n) => Value::Number(Into::into(*n)),
				aws_smithy_types::Number::NegInt(n) => Value::Number(Into::into(*n)),
				aws_smithy_types::Number::Float(n) => {
					Value::Number(serde_json::Number::from_f64(*n).ok_or(NumberDecodeError)?)
				}
			},
			Document::String(s) => Value::String(s.clone()),
			Document::Bool(b) => Value::Bool(*b),
			Document::Null => Value::Null,
		};

		Ok(val)
	}

	pub fn as_smithy(value: Value) -> Document {
		match value {
			Value::Object(map) => {
				Document::Object(map.into_iter().map(|(k, v)| (k, as_smithy(v))).collect())
			}
			Value::Array(arr) => Document::Array(arr.into_iter().map(as_smithy).collect()),
			Value::Number(n) => {
				if let Some(n) = n.as_i64() {
					Document::Number(aws_smithy_types::Number::NegInt(n))
				} else if let Some(n) = n.as_u64() {
					Document::Number(aws_smithy_types::Number::PosInt(n))
				} else if let Some(n) = n.as_f64() {
					Document::Number(aws_smithy_types::Number::Float(n))
				} else {
					unreachable!()
				}
			}
			Value::String(s) => Document::String(s),
			Value::Bool(b) => Document::Bool(b),
			Value::Null => Document::Null,
		}
	}
}

#[cfg(feature = "macros")]
#[macro_export]
macro_rules! err_path {
	($($topics:expr),+ $(,)?) => {
		vec![
			$($topics.to_string(),)+
		]
	};
}

// TODO: Clean this up, pass flags to Bolt to configure this
#[macro_export]
macro_rules! inject_latency {
	() => {
		// if !$crate::env::is_production_namespace() {
		// 	tokio::time::sleep(::std::time::Duration::from_secs(1)).await;
		// }
	};
	($perf:expr) => {
		// if !$crate::env::is_production_namespace() {
		// 	let span = $perf.start("inject-latency").await;
		// 	tokio::time::sleep(::std::time::Duration::from_secs(1)).await;
		// 	span.end();
		// }
	};
}

pub struct Backoff {
	/// Maximum exponent for the backoff.
	max_exponent: usize,

	/// Maximum amount of retries.
	max_retries: Option<usize>,

	/// Base wait time in ms.
	wait: usize,

	/// Maximum randomness.
	randomness: usize,

	/// Iteration of the backoff.
	i: usize,

	/// Timestamp to sleep until in ms.
	sleep_until: Instant,
}

impl Backoff {
	pub fn new(
		max_exponent: usize,
		max_retries: Option<usize>,
		wait: usize,
		randomness: usize,
	) -> Backoff {
		Backoff {
			max_exponent,
			max_retries,
			wait,
			randomness,
			i: 0,
			sleep_until: Instant::now(),
		}
	}

	pub fn tick_index(&self) -> usize {
		self.i
	}

	/// Waits for the next backoff tick.
	///
	/// Returns true if the index is greater than `max_retries`.
	pub async fn tick(&mut self) -> bool {
		if self.max_retries.map_or(false, |x| self.i > x) {
			return true;
		}

		tokio::time::sleep_until(self.sleep_until.into()).await;

		let next_wait = self.wait * 2usize.pow(self.i.min(self.max_exponent) as u32)
			+ rand::thread_rng().gen_range(0..self.randomness);
		self.sleep_until = self.sleep_until + Duration::from_millis(next_wait as u64);

		self.i += 1;

		false
	}

	pub fn default_infinite() -> Backoff {
		Backoff::new(8, None, 1_000, 1_000)
	}
}

impl Default for Backoff {
	fn default() -> Backoff {
		Backoff::new(5, Some(16), 1_000, 1_000)
	}
}

#[cfg(test)]
mod tests {
	use std::time::Instant;

	#[tokio::test]
	#[ignore]
	async fn test_backoff() {
		// Manually validate with `--nocapture` that the ticks are powers of 2

		let mut backoff = super::Backoff::new(5, Some(8), 100, 100);
		let mut last_tick = Instant::now();
		loop {
			let now = Instant::now();
			let dt = now.duration_since(last_tick);
			last_tick = now;
			println!("tick: {}", dt.as_secs_f64());

			if backoff.tick().await {
				println!("cancelling");
				break;
			}
		}
	}
}

/// Used to statically assert a clean exit. See
/// https://stackoverflow.com/a/62408044
pub struct CleanExit;
