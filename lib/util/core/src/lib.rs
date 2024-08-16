use std::{
	collections::HashMap,
	fmt,
	hash::{Hash, Hasher},
	ops::Deref,
};

use indexmap::IndexMap;
use rand::Rng;
pub use rivet_util_env as env;
pub use rivet_util_macros as macros;
use serde::{Deserialize, Serialize};
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
pub mod net;
pub mod req;
pub mod route;
pub mod sort;
pub mod timestamp;
pub mod uuid;

pub mod watch {
	/// Represented in seconds.
	///
	/// See docs/infrastructure/TIMEOUTS.md for reasoning.
	pub const DEFAULT_TIMEOUT: u64 = 40 * 1000;
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

	pub fn new_at(
		max_exponent: usize,
		max_retries: Option<usize>,
		wait: usize,
		randomness: usize,
		i: usize,
	) -> Backoff {
		Backoff {
			max_exponent,
			max_retries,
			wait,
			randomness,
			i,
			sleep_until: Instant::now(),
		}
	}

	pub fn tick_index(&self) -> usize {
		self.i
	}

	/// Waits for the next backoff tick.
	///
	/// Returns false if the index is greater than `max_retries`.
	pub async fn tick(&mut self) -> bool {
		if self.max_retries.map_or(false, |x| self.i > x) {
			return false;
		}

		tokio::time::sleep_until(self.sleep_until.into()).await;

		let next_wait = self.wait * 2usize.pow(self.i.min(self.max_exponent) as u32)
			+ rand::thread_rng().gen_range(0..self.randomness);
		self.sleep_until = self.sleep_until + Duration::from_millis(next_wait as u64);

		self.i += 1;

		true
	}

	/// Returns the instant of the next backoff tick. Does not wait.
	///
	/// Returns None if the index is greater than `max_retries`.
	pub fn step(&mut self) -> Option<Instant> {
		if self.max_retries.map_or(false, |x| self.i > x) {
			return None;
		}

		let next_wait = self.wait * 2usize.pow(self.i.min(self.max_exponent) as u32)
			+ rand::thread_rng().gen_range(0..self.randomness);
		self.sleep_until += Duration::from_millis(next_wait as u64);

		self.i += 1;

		Some(self.sleep_until)
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

/// Used in workflow activity inputs/outputs. Using this over BTreeMap is preferred because this does not
/// reorder keys, providing faster insert and lookup.
#[derive(Serialize, Deserialize)]
pub struct HashableMap<K: Eq + Hash, V: Hash>(IndexMap<K, V>);

impl<K: Eq + Hash, V: Hash> Deref for HashableMap<K, V> {
	type Target = IndexMap<K, V>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<K: Eq + Ord + Hash, V: Hash> Hash for HashableMap<K, V> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		let mut kv = Vec::from_iter(&self.0);
		kv.sort_unstable_by(|a, b| a.0.cmp(b.0));
		kv.hash(state);
	}
}

impl<K: Eq + Hash + fmt::Debug, V: Hash + fmt::Debug> fmt::Debug for HashableMap<K, V> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_map().entries(self.iter()).finish()
	}
}

impl<K: Eq + Hash + Clone, V: Hash + Clone> Clone for HashableMap<K, V> {
	fn clone(&self) -> Self {
		HashableMap(self.0.clone())
	}

	fn clone_from(&mut self, other: &Self) {
		self.0.clone_from(&other.0);
	}
}

pub trait AsHashableExt<K: Eq + Hash, V: Hash> {
	/// Converts the iterable to a `HashableMap` via cloning.
	fn as_hashable(&self) -> HashableMap<K, V>;
}

impl<K: Eq + Clone + Hash, V: Clone + Hash> AsHashableExt<K, V> for HashMap<K, V> {
	fn as_hashable(&self) -> HashableMap<K, V> {
		HashableMap(self.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
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

			if !backoff.tick().await {
				println!("cancelling");
				break;
			}
		}
	}
}

/// Used to statically assert a clean exit. See
/// https://stackoverflow.com/a/62408044
pub struct CleanExit;
