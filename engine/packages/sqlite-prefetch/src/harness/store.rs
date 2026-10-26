//! An in-memory database file and its two KV inodes, standing in for Depot, with counts of the
//! requests a real volume would serve. There is no latency model: the gates are counts.
use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use parking_lot::Mutex;

use crate::driver::Store;
use crate::overflow::commit::{KvOp, Sidecar};
use crate::page::Pgno;

pub const PAGE: usize = 4096;
/// Pages one object-store part holds: SlateDB's disk cache reads 1 MiB parts.
const PART_PAGES: u32 = 256;

/// Requests the store served.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct StoreCounts {
	/// Page reads, one per `read_ranges` call.
	pub page_reads: u64,
	pub pages_read: u64,
	pub kv_gets: u64,
	pub kv_scans: u64,
	/// Object-store requests for cold data: one per 1 MiB part of a run of cold pages, and
	/// one per cold KV read.
	pub object_store_requests: u64,
	pub commits: u64,
}

impl StoreCounts {
	pub fn depot_requests(&self) -> u64 {
		self.page_reads + self.kv_gets + self.kv_scans
	}
}

#[derive(Default)]
struct Inner {
	pages: Vec<Bytes>,
	recent: BTreeMap<Vec<u8>, Vec<u8>>,
	overflow: BTreeMap<Vec<u8>, Vec<u8>>,
	warm_pages: BTreeSet<Pgno>,
	warm_rows: bool,
	counts: StoreCounts,
}

#[derive(Default)]
pub struct MemStore {
	inner: Mutex<Inner>,
}

impl MemStore {
	pub fn size_pages(&self) -> u32 {
		self.inner.lock().pages.len() as u32
	}

	pub fn counts(&self) -> StoreCounts {
		self.inner.lock().counts
	}

	pub fn page(&self, pgno: Pgno) -> Option<Bytes> {
		self.inner.lock().pages.get(pgno as usize - 1).cloned()
	}

	/// Every page, for the oracle.
	pub fn file(&self) -> Vec<u8> {
		let inner = self.inner.lock();
		let mut bytes = Vec::with_capacity(inner.pages.len() * PAGE);
		for page in &inner.pages {
			bytes.extend_from_slice(page);
		}
		bytes
	}

	pub fn rows(&self, sidecar: Sidecar) -> BTreeMap<Vec<u8>, Vec<u8>> {
		let inner = self.inner.lock();
		match sidecar {
			Sidecar::Recent => inner.recent.clone(),
			Sidecar::Overflow => inner.overflow.clone(),
		}
	}

	/// Writes a row behind the library's back, as a corrupt or stale row would appear.
	pub fn inject_row(&self, sidecar: Sidecar, key: Vec<u8>, value: Vec<u8>) {
		let mut inner = self.inner.lock();
		match sidecar {
			Sidecar::Recent => inner.recent.insert(key, value),
			Sidecar::Overflow => inner.overflow.insert(key, value),
		};
	}

	/// Forgets every cache, as when the actor wakes on another engine with cold storage.
	pub fn chill(&self) {
		let mut inner = self.inner.lock();
		inner.warm_pages.clear();
		inner.warm_rows = false;
	}

	pub fn reset_counts(&self) {
		self.inner.lock().counts = StoreCounts::default();
	}

	/// Applies one commit's pages, size and KV ops atomically, as one entry.
	pub fn commit(&self, dirty: &BTreeMap<Pgno, Bytes>, size_pages: u32, ops: &[KvOp]) {
		let mut inner = self.inner.lock();
		inner.counts.commits += 1;
		inner
			.pages
			.resize(size_pages as usize, Bytes::from(vec![0; PAGE]));
		for (pgno, bytes) in dirty {
			if *pgno <= size_pages {
				inner.pages[*pgno as usize - 1] = bytes.clone();
				inner.warm_pages.insert(*pgno);
			}
		}
		for op in ops {
			let map = match op.sidecar() {
				Sidecar::Recent => &mut inner.recent,
				Sidecar::Overflow => &mut inner.overflow,
			};
			match op {
				KvOp::Put { key, value, .. } => {
					map.insert(key.clone(), value.clone());
				}
				KvOp::Delete { key, .. } => {
					map.remove(key);
				}
				KvOp::DeleteRange { start, end, .. } => {
					let doomed: Vec<Vec<u8>> = map
						.keys()
						.filter(|key| {
							key.as_slice() >= start.as_slice()
								&& end
									.as_ref()
									.is_none_or(|end| key.as_slice() < end.as_slice())
						})
						.cloned()
						.collect();
					for key in doomed {
						map.remove(&key);
					}
				}
			}
		}
	}

	fn kv_cold(inner: &mut Inner) {
		if !inner.warm_rows {
			inner.counts.object_store_requests += 1;
			inner.warm_rows = true;
		}
	}
}

#[async_trait]
impl Store for MemStore {
	async fn read_pages(&self, pgnos: &[Pgno]) -> Result<Vec<Option<Bytes>>> {
		let mut inner = self.inner.lock();
		inner.counts.page_reads += 1;
		inner.counts.pages_read += pgnos.len() as u64;

		// Runs of adjacent cold pages, each split at part boundaries.
		let mut cold: Vec<Pgno> = pgnos
			.iter()
			.copied()
			.filter(|pgno| *pgno as usize <= inner.pages.len() && !inner.warm_pages.contains(pgno))
			.collect();
		cold.sort_unstable();
		cold.dedup();
		let mut parts = BTreeSet::new();
		for pgno in &cold {
			parts.insert((pgno - 1) / PART_PAGES);
		}
		inner.counts.object_store_requests += parts.len() as u64;
		for pgno in cold {
			inner.warm_pages.insert(pgno);
		}

		Ok(pgnos
			.iter()
			.map(|pgno| inner.pages.get(*pgno as usize - 1).cloned())
			.collect())
	}

	async fn kv_get(&self, sidecar: Sidecar, key: &[u8]) -> Result<Option<Vec<u8>>> {
		let mut inner = self.inner.lock();
		inner.counts.kv_gets += 1;
		Self::kv_cold(&mut inner);
		let map = match sidecar {
			Sidecar::Recent => &inner.recent,
			Sidecar::Overflow => &inner.overflow,
		};
		Ok(map.get(key).cloned())
	}

	async fn kv_scan(
		&self,
		sidecar: Sidecar,
		start: &[u8],
		end: &[u8],
		limit: usize,
	) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
		let mut inner = self.inner.lock();
		inner.counts.kv_scans += 1;
		Self::kv_cold(&mut inner);
		let map = match sidecar {
			Sidecar::Recent => &inner.recent,
			Sidecar::Overflow => &inner.overflow,
		};
		Ok(map
			.range(start.to_vec()..=end.to_vec())
			.take(limit)
			.map(|(key, value)| (key.clone(), value.clone()))
			.collect())
	}
}
