#![cfg(debug_assertions)]

use std::{collections::BTreeSet, sync::Arc};

use anyhow::{Context, Result, anyhow};
use futures_util::TryStreamExt;
use rivet_pools::NodeId;
use universaldb::{
	Database, RangeOption,
	options::StreamingMode,
	utils::IsolationLevel::Snapshot,
};

use crate::{
	compactor::metrics,
	pump::{
		keys,
		types::{decode_db_head, DBHead},
	},
};

const PIDX_PGNO_BYTES: usize = std::mem::size_of::<u32>();
const PIDX_TXID_BYTES: usize = std::mem::size_of::<u64>();
const SHARD_ID_BYTES: usize = std::mem::size_of::<u32>();
const UNKNOWN_NODE_ID: &str = "unknown";

pub async fn reconcile(udb: &Database, actor_id: &str) -> Result<()> {
	reconcile_inner(udb, actor_id, None).await
}

pub(crate) async fn reconcile_with_node_id(
	udb: &Database,
	actor_id: &str,
	node_id: NodeId,
) -> Result<()> {
	reconcile_inner(udb, actor_id, Some(node_id)).await
}

pub(crate) fn reconcile_blocking(udb: Arc<Database>, actor_id: String, node_id: NodeId) {
	let result = std::thread::Builder::new()
		.name("sqlite-takeover-reconcile".to_string())
		.spawn(move || -> Result<()> {
			let runtime = tokio::runtime::Builder::new_current_thread()
				.enable_all()
				.build()
				.context("build sqlite takeover reconciliation runtime")?;

			runtime.block_on(reconcile_with_node_id(&udb, &actor_id, node_id))
		})
		.expect("spawn sqlite takeover reconciliation thread")
		.join()
		.expect("sqlite takeover reconciliation thread panicked");

	if let Err(err) = result {
		panic!("sqlite takeover reconciliation failed: {err:#}");
	}
}

async fn reconcile_inner(
	udb: &Database,
	actor_id: &str,
	node_id: Option<NodeId>,
) -> Result<()> {
	let actor_id = actor_id.to_string();
	let actor_id_for_tx = actor_id.clone();
	let scan = udb
		.run(move |tx| {
			let actor_id = actor_id_for_tx.clone();

			async move {
				let head = tx
					.informal()
					.get(&keys::meta_head_key(&actor_id), Snapshot)
					.await?
					.map(|bytes| decode_db_head(bytes.as_ref()))
					.transpose()
					.context("decode sqlite db head for takeover reconciliation")?
					.unwrap_or_else(empty_head);

				let delta_rows = tx_scan_prefix_values(&tx, &keys::delta_prefix(&actor_id)).await?;
				let pidx_rows = tx_scan_prefix_values(&tx, &keys::pidx_delta_prefix(&actor_id)).await?;
				let shard_rows = tx_scan_prefix_values(&tx, &keys::shard_prefix(&actor_id)).await?;

				classify_rows(&actor_id, &head, delta_rows, pidx_rows, shard_rows)
			}
		})
		.await?;

	if let Some(violation) = scan.violation {
		return Err(report_violation(actor_id.as_str(), node_id, violation));
	}

	Ok(())
}

fn classify_rows(
	actor_id: &str,
	head: &DBHead,
	delta_rows: Vec<(Vec<u8>, Vec<u8>)>,
	pidx_rows: Vec<(Vec<u8>, Vec<u8>)>,
	shard_rows: Vec<(Vec<u8>, Vec<u8>)>,
) -> Result<ReconcileScan> {
	let mut delta_txids = BTreeSet::new();

	for (key, _value) in &delta_rows {
		let txid = keys::decode_delta_chunk_txid(actor_id, key)?;
		if txid > head.head_txid {
			return Ok(ReconcileScan::violated(
				TakeoverViolationKind::AboveHeadTxid,
				key,
			));
		}
		delta_txids.insert(txid);
	}

	for (key, value) in &pidx_rows {
		let pgno = decode_pidx_pgno(actor_id, key)?;
		let txid = decode_pidx_txid(value)?;

		if pgno == 0 || pgno > head.db_size_pages {
			return Ok(ReconcileScan::violated(
				TakeoverViolationKind::AboveEof,
				key,
			));
		}
		if txid > head.head_txid {
			return Ok(ReconcileScan::violated(
				TakeoverViolationKind::AboveHeadTxid,
				key,
			));
		}
		if !delta_txids.contains(&txid) {
			return Ok(ReconcileScan::violated(
				TakeoverViolationKind::DanglingPidxRef,
				key,
			));
		}
	}

	for (key, _value) in &shard_rows {
		let shard_id = decode_shard_id(actor_id, key)?;
		if shard_id.saturating_mul(keys::SHARD_SIZE) > head.db_size_pages {
			return Ok(ReconcileScan::violated(
				TakeoverViolationKind::AboveEof,
				key,
			));
		}
	}

	Ok(ReconcileScan { violation: None })
}

fn report_violation(
	actor_id: &str,
	node_id: Option<NodeId>,
	violation: TakeoverViolation,
) -> anyhow::Error {
	let node_id = node_id
		.map(|node_id| node_id.to_string())
		.unwrap_or_else(|| UNKNOWN_NODE_ID.to_string());
	let kind = violation.kind.as_str();
	let key_snippet = violation.key_snippet;

	metrics::SQLITE_TAKEOVER_INVARIANT_VIOLATION_TOTAL
		.with_label_values(&[node_id.as_str(), kind])
		.inc();
	tracing::error!(
		actor_id = %actor_id,
		kind,
		key_snippet = ?key_snippet,
		"sqlite takeover invariant violation"
	);

	anyhow!(
		"sqlite takeover invariant violation for actor {actor_id}: {kind} at key {:?}",
		key_snippet
	)
}

async fn tx_scan_prefix_values(
	tx: &universaldb::Transaction,
	prefix: &[u8],
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
	let informal = tx.informal();
	let prefix_subspace =
		universaldb::Subspace::from(universaldb::tuple::Subspace::from_bytes(prefix.to_vec()));
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
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

fn decode_pidx_pgno(actor_id: &str, key: &[u8]) -> Result<u32> {
	let prefix = keys::pidx_delta_prefix(actor_id);
	let suffix = key
		.strip_prefix(prefix.as_slice())
		.context("pidx key did not start with expected prefix")?;
	let bytes: [u8; PIDX_PGNO_BYTES] = suffix
		.try_into()
		.map_err(|_| anyhow!("pidx key suffix had invalid length"))?;

	Ok(u32::from_be_bytes(bytes))
}

fn decode_pidx_txid(value: &[u8]) -> Result<u64> {
	let bytes: [u8; PIDX_TXID_BYTES] = value
		.try_into()
		.map_err(|_| anyhow!("pidx txid had invalid length"))?;

	Ok(u64::from_be_bytes(bytes))
}

fn decode_shard_id(actor_id: &str, key: &[u8]) -> Result<u32> {
	let prefix = keys::shard_prefix(actor_id);
	let suffix = key
		.strip_prefix(prefix.as_slice())
		.context("shard key did not start with expected prefix")?;
	let bytes: [u8; SHARD_ID_BYTES] = suffix
		.try_into()
		.map_err(|_| anyhow!("shard key suffix had invalid length"))?;

	Ok(u32::from_be_bytes(bytes))
}

fn empty_head() -> DBHead {
	DBHead {
		head_txid: 0,
		db_size_pages: 0,
		#[cfg(debug_assertions)]
		generation: 0,
	}
}

#[derive(Debug)]
struct ReconcileScan {
	violation: Option<TakeoverViolation>,
}

impl ReconcileScan {
	fn violated(kind: TakeoverViolationKind, key: &[u8]) -> Self {
		Self {
			violation: Some(TakeoverViolation {
				kind,
				key_snippet: key.iter().copied().take(64).collect(),
			}),
		}
	}
}

#[derive(Debug)]
struct TakeoverViolation {
	kind: TakeoverViolationKind,
	key_snippet: Vec<u8>,
}

#[derive(Debug, Clone, Copy)]
enum TakeoverViolationKind {
	AboveEof,
	AboveHeadTxid,
	DanglingPidxRef,
}

impl TakeoverViolationKind {
	fn as_str(self) -> &'static str {
		match self {
			Self::AboveEof => "above_eof",
			Self::AboveHeadTxid => "above_head_txid",
			Self::DanglingPidxRef => "dangling_pidx_ref",
		}
	}
}
