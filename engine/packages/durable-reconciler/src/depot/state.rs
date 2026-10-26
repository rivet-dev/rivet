use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct State<D, A> {
	/// Adapter-owned concurrency token; preserved by schema conversions.
	pub revision: u64,
	pub desired: D,
	pub actual: A,
}
