//! Store facts carried through operator repair state machine effects.

use bytes::Bytes;
use depot_fs_core::FsVolumeRecord;
use rivet_data::generated::depot_client_protocol_v1::RepairAuditRow;

/// What the store holds for one repair call before it runs: the call's audit row, for a
/// record rebuild whether the incarnation's format row and root inode exist at the cold
/// position the plan would record, and for an append the stored value of each row it expects.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct RepairPrepared {
	pub audit: Option<RepairAuditRow>,
	pub probe: Option<RepairProbe>,
	/// The position the rows were read at, with one entry per requested row: its newest
	/// stored value bytes there, or `None` when the row is absent or deleted there.
	pub rows: Option<(
		rivet_data::generated::shared_depot_types_v1::VolumeLogPosition,
		Vec<Option<Bytes>>,
	)>,
}

/// The store facts a repair call's plan needs beside its audit row.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum PrepareReads {
	None,
	/// A rebuild's probe of the incarnation's rows at the record's cold position.
	Probe(FsVolumeRecord),
	/// An append's preimages: rows of the registered volume read at `applied`.
	Rows {
		record: FsVolumeRecord,
		applied: rivet_data::generated::shared_depot_types_v1::VolumeLogPosition,
		keys: Vec<rivet_data::generated::shared_depot_fs_read_v1::FsRowKey>,
	},
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct RepairProbe {
	/// The record the probe read against.
	pub record: FsVolumeRecord,
	pub format_row_present: bool,
	pub root_inode_present: bool,
}
