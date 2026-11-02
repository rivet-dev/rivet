//! Shared fingerprints for state machine logs and simulation replay.

use serde::Serialize;

/// Compare state machine runs without logging their entire state.
///
/// At debug level, the runner logs each delivered event, its resulting effects, and a hash of the
/// state after the transition. These are actual logs from the toy runner test,
/// wrapped for readability; the negative Unix time comes from its test clock:
///
/// ```text
/// DEBUG rivet_state_machine: state machine step sm="toy" seq=1
///   event={"now":{"boot_ms":30,"unix_ms":-30},"body":{"Timer":"A"}}
///   effects=[] state_hash=4658191084576012576 hasher="xxh3_64/seed0/bare"
/// DEBUG rivet_state_machine: state machine step sm="toy" seq=2
///   event={"now":{"boot_ms":30,"unix_ms":-30},"body":{"Timer":"B"}}
///   effects=[Execute(Work)] state_hash=3597066399425898285 hasher="xxh3_64/seed0/bare"
/// DEBUG rivet_state_machine: state machine step sm="toy" seq=3
///   event={"now":{"boot_ms":30,"unix_ms":-30},"body":{"Input":"Completed"}}
///   effects=[] state_hash=3523026795133155343 hasher="xxh3_64/seed0/bare"
/// ```
///
/// Here timer B requests work at step 2, and its completion returns through the
/// inbox at step 3. Effects are logged before dispatch, so `Execute(Work)` alone
/// does not mean the work succeeded. `state_hash` describes that step's state,
/// not the accumulated execution history.
///
/// To investigate a replay mismatch, compare corresponding runner instances and
/// find the first step with different events, effects, or state hashes. A changed
/// `event` exposes different inputs, delivery order, or clock readings. If events
/// match but state hashes differ, inspect the transition and preceding state.
/// Compare runs with the same code and serialization format; `seq` is local to
/// one runner, and `sm` names its type rather than identifying its instance.
///
/// Simulation can also feed successive records through `update` and compare
/// `finish` results as a checksum of the whole trace. A mismatch tells you to
/// inspect individual steps; the final checksum alone cannot locate the change.
///
/// Hashes depend on serialization and collection iteration order; this type does
/// not normalize states. Equal hashes do not prove equality, so compare values
/// when deduplicating visited states. BARE bytes are only hash input here, not a
/// persisted or wire format.
pub struct TraceHasher(xxhash_rust::xxh3::Xxh3);

impl Default for TraceHasher {
	fn default() -> Self {
		Self::new()
	}
}

impl TraceHasher {
	pub const NAME: &'static str = "xxh3_64/seed0/bare";

	pub fn new() -> Self {
		Self(xxhash_rust::xxh3::Xxh3::with_seed(0))
	}

	pub fn update<T: Serialize>(&mut self, item: &T) -> serde_bare::Result<()> {
		let bytes = serde_bare::to_vec(item)?;
		self.0.update(&bytes);
		Ok(())
	}

	pub fn finish(&self) -> u64 {
		self.0.digest()
	}

	pub fn hash<T: Serialize>(item: &T) -> serde_bare::Result<u64> {
		let mut h = Self::new();
		h.update(item)?;
		Ok(h.finish())
	}
}
