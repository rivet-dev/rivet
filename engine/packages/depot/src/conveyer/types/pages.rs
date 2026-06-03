use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirtyPage {
	pub pgno: u32,
	pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FetchedPage {
	pub pgno: u32,
	pub bytes: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetPagesResult {
	pub pages: Vec<FetchedPage>,
	pub head_txid: u64,
	pub db_size_pages: u32,
	pub provenance: Vec<PageSourceProvenance>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DepotReadMode {
	#[default]
	Serving,
	DiagnosticNoSideEffects,
}

impl DepotReadMode {
	pub fn allows_side_effects(self) -> bool {
		match self {
			Self::Serving => true,
			Self::DiagnosticNoSideEffects => false,
		}
	}
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetPagesOptions {
	pub expected_head_txid: Option<u64>,
	pub mode: DepotReadMode,
	pub collect_provenance: bool,
	pub diagnostic_max_txid: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageSourceProvenance {
	pub pgno: u32,
	pub winner_kind: PageSourceKind,
	pub winner_txid: Option<u64>,
	pub winner_shard_id: Option<u32>,
	pub candidates: Vec<PageSourceCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageSourceCandidate {
	pub kind: PageSourceKind,
	pub txid: Option<u64>,
	pub shard_id: Option<u32>,
	pub result: PageSourceCandidateResult,
	pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PageSourceKind {
	PidxDelta,
	HistoricalDelta,
	MissingDelta,
	StaleDelta,
	HotShard,
	Cold,
	ZeroFill,
	OutOfRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PageSourceCandidateResult {
	Won,
	Lost,
	Selected,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitOptions {
	pub expected_head_txid: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitResult {
	pub head_txid: u64,
	pub db_size_pages: u32,
}
