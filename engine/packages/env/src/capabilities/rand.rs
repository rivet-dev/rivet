//! Replaces ambient `rand` draws and `uuid::Uuid::new_v4()` with labelled, seeded
//! streams backed by `rand_chacha::ChaCha8Rng`.

#![allow(clippy::disallowed_types)]
use crate::NodeId;
pub use ::rand::{Rng, RngCore};
use ::rand::{SeedableRng, distributions::uniform::SampleUniform};
use parking_lot::Mutex;
use rand_chacha::ChaCha8Rng;
use std::{ops::Range, sync::Arc};

/// Pure operational randomness for one transition, with no shared state or environment handle.
/// Never use this generator for secrets, clone it, or retain it in state machine state.
pub struct LocalRng(ChaCha8Rng);

impl LocalRng {
	/// Reconstruct the exact draw sequence from a recorded delivery seed.
	pub fn from_seed(seed: [u8; 32]) -> Self {
		Self(ChaCha8Rng::from_seed(seed))
	}

	/// Draw from a nonempty half-open range.
	pub fn gen_range<T: SampleUniform + PartialOrd>(&mut self, range: Range<T>) -> T {
		self.0.gen_range(range)
	}

	/// Draw an index below a nonempty length. Samples in `u64` so a seed replays the same
	/// draws on 32-bit and 64-bit targets; `gen_range` over `usize` does not.
	pub fn index(&mut self, len: usize) -> usize {
		let len = u64::try_from(len).unwrap_or(u64::MAX);
		let index = self.0.gen_range(0..len);
		usize::try_from(index).unwrap_or(usize::MAX)
	}

	/// Generate an operational identifier with UUID v4 version and variant bits.
	pub fn uuid_v4(&mut self) -> uuid::Uuid {
		let mut bytes = [0; 16];
		self.fill_bytes(&mut bytes);
		uuid::Builder::from_random_bytes(bytes).into_uuid()
	}
}

impl RngCore for LocalRng {
	fn next_u32(&mut self) -> u32 {
		self.0.next_u32()
	}

	fn next_u64(&mut self) -> u64 {
		self.0.next_u64()
	}

	fn fill_bytes(&mut self, dest: &mut [u8]) {
		self.0.fill_bytes(dest)
	}

	fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), ::rand::Error> {
		self.0.try_fill_bytes(dest)
	}
}

/// Handles for a label share one stream; reopening a label never repeats its draws.
/// This generator is for simulation and operational jitter, never secrets.
/// Use `backends::production::secure_random` at credential/key creation boundaries.
#[derive(Clone)]
pub struct RngStream(Arc<Mutex<LocalRng>>);

impl RngStream {
	/// Preserve the deterministic stream derived from the root seed, node, and label.
	pub fn seeded(seed: u64, node: &NodeId, label: &str) -> Self {
		let mut h = blake3::Hasher::new();
		h.update(&seed.to_le_bytes());
		h.update(&(node.0.len() as u64).to_le_bytes());
		h.update(&node.0);
		h.update(label.as_bytes());

		let generator = LocalRng::from_seed(*h.finalize().as_bytes());
		Self(Arc::new(Mutex::new(generator)))
	}

	#[cfg(not(target_arch = "wasm32"))]
	pub(crate) fn production_seed() -> u64 {
		::rand::rngs::OsRng.next_u64()
	}

	pub fn gen_range<T: SampleUniform + PartialOrd>(&mut self, range: Range<T>) -> T {
		self.0.lock().gen_range(range)
	}

	/// Draw an index below a nonempty length; see [`LocalRng::index`].
	pub fn index(&mut self, len: usize) -> usize {
		self.0.lock().index(len)
	}

	/// Generate an operational identifier from the shared labelled stream.
	pub fn uuid_v4(&mut self) -> uuid::Uuid {
		self.0.lock().uuid_v4()
	}

	/// Draw one independent delivery seed, advancing the shared stream by 32 bytes.
	pub fn fork_seed(&mut self) -> [u8; 32] {
		let mut seed = [0; 32];
		self.fill_bytes(&mut seed);
		seed
	}
}

impl RngCore for RngStream {
	fn next_u32(&mut self) -> u32 {
		self.0.lock().next_u32()
	}

	fn next_u64(&mut self) -> u64 {
		self.0.lock().next_u64()
	}

	fn fill_bytes(&mut self, dest: &mut [u8]) {
		self.0.lock().fill_bytes(dest)
	}

	fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), ::rand::Error> {
		self.0.lock().try_fill_bytes(dest)
	}
}
