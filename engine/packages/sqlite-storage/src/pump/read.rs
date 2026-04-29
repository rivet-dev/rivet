//! Page read path for the stateless sqlite-storage pump.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, ensure};
use futures_util::TryStreamExt;
use universaldb::{
	RangeOption,
	options::StreamingMode,
	utils::IsolationLevel::Snapshot,
};

use crate::pump::{
	ActorDb,
	error::SqliteStorageError,
	keys::{self, PAGE_SIZE, SHARD_SIZE},
	ltx::{DecodedLtx, decode_ltx_v3},
	metrics,
	page_index::DeltaPageIndex,
	types::{FetchedPage, decode_db_head},
};

const PIDX_PGNO_BYTES: usize = std::mem::size_of::<u32>();
const PIDX_TXID_BYTES: usize = std::mem::size_of::<u64>();

impl ActorDb {
	pub async fn get_pages(&self, pgnos: Vec<u32>) -> Result<Vec<FetchedPage>> {
		let node_id = self.node_id.to_string();
		let labels = &[node_id.as_str()];
		let _timer = metrics::SQLITE_PUMP_GET_PAGES_DURATION
			.with_label_values(labels)
			.start_timer();
		metrics::SQLITE_PUMP_GET_PAGES_PGNO_COUNT
			.with_label_values(labels)
			.observe(pgnos.len() as f64);

		for pgno in &pgnos {
			ensure!(*pgno > 0, "get_pages does not accept page 0");
		}

		let cached_pidx = {
			let cache = self.cache.lock();
			let cached_rows = cache.range(0, u32::MAX);
			if cached_rows.is_empty() {
				None
			} else {
				Some(
					pgnos
						.iter()
						.map(|pgno| (*pgno, cache.get(*pgno)))
						.collect::<BTreeMap<_, _>>(),
				)
			}
		};

		let actor_id = self.actor_id.clone();
		let pgnos_for_tx = pgnos.clone();
		let tx_result = self
			.udb
			.run(move |tx| {
				let actor_id = actor_id.clone();
				let pgnos = pgnos_for_tx.clone();
				let cached_pidx = cached_pidx.clone();

				async move {
					let head_bytes = tx_get_value(&tx, &keys::meta_head_key(&actor_id)).await?;
					let Some(head_bytes) = head_bytes else {
						return Err(SqliteStorageError::MetaMissing {
							operation: "get_pages",
						}
						.into());
					};
					let head = decode_db_head(&head_bytes)?;

					let pgnos_in_range = pgnos
						.into_iter()
						.filter(|pgno| *pgno <= head.db_size_pages)
						.collect::<Vec<_>>();
					if pgnos_in_range.is_empty() {
						return Ok(GetPagesTxResult {
							db_size_pages: head.db_size_pages,
							loaded_pidx_rows: None,
							page_sources: BTreeMap::new(),
							source_blobs: BTreeMap::new(),
							stale_pidx_pgnos: BTreeSet::new(),
						});
					}

					let mut pidx_by_pgno = BTreeMap::new();
					let mut loaded_pidx_rows = None;
					if let Some(cached_pidx) = cached_pidx.as_ref() {
						for (pgno, txid) in cached_pidx {
							if let Some(txid) = txid {
								pidx_by_pgno.insert(*pgno, *txid);
							}
						}
					} else {
						let rows = tx_scan_prefix_values(&tx, &keys::pidx_delta_prefix(&actor_id)).await?;
						let decoded_rows = rows
							.into_iter()
							.map(|(key, value)| {
								Ok((decode_pidx_pgno(&actor_id, &key)?, decode_pidx_txid(&value)?))
							})
							.collect::<Result<Vec<_>>>()?;
						for (pgno, txid) in &decoded_rows {
							pidx_by_pgno.insert(*pgno, *txid);
						}
						loaded_pidx_rows = Some(decoded_rows);
					}

					let mut page_sources = BTreeMap::new();
					let mut source_blobs = BTreeMap::new();
					let mut missing_delta_prefixes = BTreeSet::new();
					let mut stale_pidx_pgnos = BTreeSet::new();

					for pgno in &pgnos_in_range {
						let preferred_delta_prefix = pidx_by_pgno
							.get(pgno)
							.copied()
							.map(|txid| keys::delta_chunk_prefix(&actor_id, txid));

						let mut source_key = preferred_delta_prefix
							.clone()
							.unwrap_or_else(|| keys::shard_key(&actor_id, pgno / SHARD_SIZE));
						if preferred_delta_prefix
							.as_ref()
							.is_some_and(|prefix| missing_delta_prefixes.contains(prefix))
						{
							stale_pidx_pgnos.insert(*pgno);
							source_key = keys::shard_key(&actor_id, pgno / SHARD_SIZE);
						}

						if !source_blobs.contains_key(&source_key) {
							let mut blob = if source_key.starts_with(&keys::delta_prefix(&actor_id)) {
								tx_load_delta_blob(&tx, &source_key).await?
							} else {
								tx_get_value(&tx, &source_key).await?
							};

							if blob.is_none() {
								if let Some(delta_prefix) = preferred_delta_prefix.as_ref() {
									missing_delta_prefixes.insert(delta_prefix.clone());
									stale_pidx_pgnos.insert(*pgno);
									source_key = keys::shard_key(&actor_id, pgno / SHARD_SIZE);
									blob = match source_blobs.get(&source_key).cloned() {
										Some(existing) => Some(existing),
										None => tx_get_value(&tx, &source_key).await?,
									};
								}
							}

							if let Some(blob) = blob {
								source_blobs.insert(source_key.clone(), blob);
							} else {
								continue;
							}
						}

						page_sources.insert(*pgno, source_key);
					}

					Ok(GetPagesTxResult {
						db_size_pages: head.db_size_pages,
						loaded_pidx_rows,
						page_sources,
						source_blobs,
						stale_pidx_pgnos,
					})
				}
			})
			.await?;

		let mut stale_pidx_pgnos = tx_result.stale_pidx_pgnos;
		if let Some(loaded_pidx_rows) = tx_result.loaded_pidx_rows {
			metrics::SQLITE_PUMP_PIDX_COLD_SCAN_TOTAL
				.with_label_values(labels)
				.inc();

			let loaded_index = DeltaPageIndex::new();
			for (pgno, txid) in loaded_pidx_rows {
				if !stale_pidx_pgnos.contains(&pgno) {
					loaded_index.insert(pgno, txid);
				}
			}

			let cache = self.cache.lock();
			for (pgno, txid) in loaded_index.range(0, u32::MAX) {
				cache.insert(pgno, txid);
			}
		}

		let mut decoded_blobs = BTreeMap::new();
		let mut pages = Vec::with_capacity(pgnos.len());
		let mut returned_bytes = 0u64;

		for pgno in pgnos {
			if pgno > tx_result.db_size_pages {
				pages.push(FetchedPage { pgno, bytes: None });
				continue;
			}

			let bytes = if let Some(source_key) = tx_result.page_sources.get(&pgno) {
				let blob = tx_result
					.source_blobs
					.get(source_key)
					.with_context(|| format!("missing source blob for page {pgno}"))?;

				if !decoded_blobs.contains_key(source_key) {
					let decoded = decode_ltx_v3(blob)
						.with_context(|| format!("decode source blob for page {pgno}"))?;
					decoded_blobs.insert(source_key.clone(), decoded);
				}

				let mut bytes = decoded_blobs
					.get(source_key)
					.and_then(|decoded: &DecodedLtx| decoded.get_page(pgno))
					.map(ToOwned::to_owned);
				if bytes.is_none() && source_key.starts_with(&keys::delta_prefix(&self.actor_id)) {
					stale_pidx_pgnos.insert(pgno);
				}
				bytes.get_or_insert_with(|| vec![0; PAGE_SIZE as usize]).clone()
			} else {
				vec![0; PAGE_SIZE as usize]
			};

			returned_bytes += bytes.len() as u64;
			pages.push(FetchedPage {
				pgno,
				bytes: Some(bytes),
			});
		}

		if !stale_pidx_pgnos.is_empty() {
			let cache = self.cache.lock();
			for pgno in stale_pidx_pgnos {
				cache.remove(pgno);
			}
		}

		*self.read_bytes_since_rollup.lock() += returned_bytes;

		Ok(pages)
	}
}

struct GetPagesTxResult {
	db_size_pages: u32,
	loaded_pidx_rows: Option<Vec<(u32, u64)>>,
	page_sources: BTreeMap<u32, Vec<u8>>,
	source_blobs: BTreeMap<Vec<u8>, Vec<u8>>,
	stale_pidx_pgnos: BTreeSet<u32>,
}

async fn tx_get_value(
	tx: &universaldb::Transaction,
	key: &[u8],
) -> Result<Option<Vec<u8>>> {
	Ok(tx
		.informal()
		.get(key, Snapshot)
		.await?
		.map(Vec::<u8>::from))
}

async fn tx_scan_prefix_values(
	tx: &universaldb::Transaction,
	prefix: &[u8],
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
	let informal = tx.informal();
	let prefix_subspace =
		universaldb::Subspace::from(universaldb::tuple::Subspace::from_bytes(prefix.to_vec()));
	let mut stream = informal.get_ranges_keyvalues(
		universaldb::RangeOption {
			mode: StreamingMode::WantAll,
			..RangeOption::from(&prefix_subspace)
		},
		Snapshot,
	);
	let mut rows = Vec::new();

	while let Some(entry) = stream.try_next().await? {
		rows.push((entry.key().to_vec(), entry.value().to_vec()));
	}

	Ok(rows)
}

async fn tx_load_delta_blob(
	tx: &universaldb::Transaction,
	delta_prefix: &[u8],
) -> Result<Option<Vec<u8>>> {
	let delta_chunks = tx_scan_prefix_values(tx, delta_prefix).await?;
	if delta_chunks.is_empty() {
		return Ok(None);
	}

	let mut delta_blob = Vec::new();
	for (key, chunk) in delta_chunks {
		if key.strip_prefix(delta_prefix) == Some(b"META".as_slice()) {
			continue;
		}
		delta_blob.extend_from_slice(&chunk);
	}

	Ok((!delta_blob.is_empty()).then_some(delta_blob))
}

fn decode_pidx_pgno(actor_id: &str, key: &[u8]) -> Result<u32> {
	let prefix = keys::pidx_delta_prefix(actor_id);
	ensure!(
		key.starts_with(&prefix),
		"pidx key did not start with expected prefix"
	);

	let suffix = &key[prefix.len()..];
	ensure!(
		suffix.len() == PIDX_PGNO_BYTES,
		"pidx key suffix had {} bytes, expected {}",
		suffix.len(),
		PIDX_PGNO_BYTES
	);

	Ok(u32::from_be_bytes(
		suffix
			.try_into()
			.context("pidx key suffix should decode as u32")?,
	))
}

fn decode_pidx_txid(value: &[u8]) -> Result<u64> {
	ensure!(
		value.len() == PIDX_TXID_BYTES,
		"pidx value had {} bytes, expected {}",
		value.len(),
		PIDX_TXID_BYTES
	);

	Ok(u64::from_be_bytes(
		value
			.try_into()
			.context("pidx value should decode as u64")?,
	))
}
