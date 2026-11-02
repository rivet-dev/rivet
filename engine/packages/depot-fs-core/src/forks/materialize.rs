//! Ordered local operations for a background materialization, scoped to one incarnation.
use super::Materialization;
use crate::{FsChunkPosition, FsMaterializeCursor, FsMaterializedChunk, FsVolumeRecord};
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Action {
	Inspect,
	/// A source confirmed this layer now contains the complete image. Restart
	/// before its first key: recently copied sequence-zero keys may sort earlier.
	Restart {
		expected: FsMaterializeCursor,
		layer: u32,
	},
	Copy {
		expected: FsMaterializeCursor,
		chunks: Vec<FsMaterializedChunk>,
		complete: bool,
	},
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Progress {
	Idle,
	Ready(Materialization),
	Complete(FsVolumeRecord),
}
pub fn position_key(p: &FsChunkPosition) -> (u64, u64) {
	(p.ino, p.index)
}
