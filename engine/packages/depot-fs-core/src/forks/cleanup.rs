//! Durable cleanup identities discovered independently of client request lifetimes.
use crate::{FsForkPresence, FsForkSpec, keys::slatedb::Scope};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Kind {
	Check,
	Release,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Work {
	pub scope: Scope,
	pub kind: Kind,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Action {
	/// Re-read durable identity before deciding whether any remote work is needed.
	Inspect,
	/// Apply a definitive destination result to the exact source hold.
	Checked {
		spec: FsForkSpec,
		presence: FsForkPresence,
	},
	/// The source has confirmed its permanent release receipt.
	Released(FsForkSpec),
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Progress {
	/// Already complete or currently pinned; a later bounded scan may revisit it.
	Idle,
	Check {
		spec: FsForkSpec,
		created_at: i64,
	},
	Release(FsForkSpec),
}

impl Work {
	pub fn matches(&self, spec: &FsForkSpec, region: u16) -> bool {
		let (key, expected_region) = match self.kind {
			Kind::Check => (&spec.parent_volume_key, spec.parent_region),
			Kind::Release => (&spec.destination_volume_key, spec.destination_region),
		};
		region == expected_region
			&& crate::forks::valid_spec(spec, spec.destination_region)
			&& self.scope.hash == depot_types::partition_key_hash(key)
			&& self.scope.name.as_deref() == key.name.as_deref()
			&& *self.scope.incarnation.as_bytes() == spec.operation_id
	}
}
