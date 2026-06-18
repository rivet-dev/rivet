use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetPagesOptions {
	pub expected_head_txid: Option<u64>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitStageBeginResult {
	pub stage_id: Uuid,
	pub max_pages_per_batch: u32,
	pub max_batch_bytes: u32,
	pub observed_head_txid: u64,
	pub staged_txid: u64,
}
