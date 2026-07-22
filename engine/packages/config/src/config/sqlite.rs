use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Sqlite {
	/// UNSTABLE: disables the SQLite v2 commit dirty-page size cap.
	#[serde(default)]
	pub unstable_disable_commit_size_cap: Option<bool>,
	/// UNSTABLE: disables SQLite hot compaction.
	#[serde(default)]
	pub unstable_disable_compaction: Option<bool>,
}

impl Sqlite {
	pub fn unstable_disable_commit_size_cap(&self) -> bool {
		self.unstable_disable_commit_size_cap.unwrap_or_default()
	}

	pub fn unstable_disable_compaction(&self) -> bool {
		// TODO: Re-enable compaction by default after thorough testing is complete.
		self.unstable_disable_compaction.unwrap_or(true)
	}
}
