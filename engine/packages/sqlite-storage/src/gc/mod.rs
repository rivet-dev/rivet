use anyhow::{Context, Result};
use futures_util::TryStreamExt;
use universaldb::{
	RangeOption,
	options::StreamingMode,
	utils::IsolationLevel::{Serializable, Snapshot},
};

use crate::pump::{
	keys,
	types::{ActorBranchId, decode_actor_branch_record},
};

pub const VERSIONSTAMP_INFINITY: [u8; 16] = [0xff; 16];
pub const VERSIONSTAMP_ZERO: [u8; 16] = [0; 16];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BranchGcPin {
	pub branch_id: ActorBranchId,
	pub refcount: i64,
	pub root_pin: [u8; 16],
	pub desc_pin: [u8; 16],
	pub bk_pin: [u8; 16],
	pub gc_pin: [u8; 16],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BranchHotGcOutcome {
	pub gc_pin: [u8; 16],
	pub txid_floor: Option<u64>,
	pub commits_deleted: usize,
	pub vtx_deleted: usize,
	pub delta_chunks_deleted: usize,
}

pub async fn estimate_branch_gc_pin(
	db: &universaldb::Database,
	branch_id: ActorBranchId,
) -> Result<Option<BranchGcPin>> {
	db.run(move |tx| async move { read_branch_gc_pin_tx(&tx, branch_id).await })
		.await
}

pub async fn sweep_branch_hot_history(
	db: &universaldb::Database,
	branch_id: ActorBranchId,
) -> Result<Option<BranchHotGcOutcome>> {
	db.run(move |tx| async move { sweep_branch_hot_history_tx(&tx, branch_id).await })
		.await
}

pub(crate) async fn read_branch_gc_pin_tx(
	tx: &universaldb::Transaction,
	branch_id: ActorBranchId,
) -> Result<Option<BranchGcPin>> {
	let Some(record_bytes) = tx
		.informal()
		.get(&keys::branches_list_key(branch_id), Serializable)
		.await?
	else {
		return Ok(None);
	};
	let record =
		decode_actor_branch_record(&record_bytes).context("decode sqlite branch record for GC")?;
	let refcount = read_i64_le(tx, &keys::branches_refcount_key(branch_id)).await?;
	let root_pin = if refcount > 0 {
		record.root_versionstamp
	} else {
		VERSIONSTAMP_INFINITY
	};
	let desc_pin = read_versionstamp_pin(tx, &keys::branches_desc_pin_key(branch_id)).await?;
	let bk_pin = read_versionstamp_pin(tx, &keys::branches_bk_pin_key(branch_id)).await?;
	let gc_pin = root_pin.min(desc_pin).min(bk_pin);

	Ok(Some(BranchGcPin {
		branch_id,
		refcount,
		root_pin,
		desc_pin,
		bk_pin,
		gc_pin,
	}))
}

pub(crate) async fn sweep_branch_hot_history_tx(
	tx: &universaldb::Transaction,
	branch_id: ActorBranchId,
) -> Result<Option<BranchHotGcOutcome>> {
	let Some(pin) = read_branch_gc_pin_tx(tx, branch_id).await? else {
		return Ok(None);
	};
	let Some(txid_floor) = gc_pin_txid_floor(tx, branch_id, pin.gc_pin).await? else {
		return Ok(Some(BranchHotGcOutcome {
			gc_pin: pin.gc_pin,
			txid_floor: None,
			commits_deleted: 0,
			vtx_deleted: 0,
			delta_chunks_deleted: 0,
		}));
	};

	let mut commits_deleted = 0;
	for (key, _value) in scan_prefix(tx, &keys::branch_commit_prefix(branch_id), Snapshot).await? {
		let txid = decode_suffix_u64(&keys::branch_commit_prefix(branch_id), &key)
			.context("decode sqlite branch commit txid during GC")?;
		if txid < txid_floor {
			tx.informal().clear(&key);
			commits_deleted += 1;
		}
	}

	let mut vtx_deleted = 0;
	for (key, _value) in scan_prefix(tx, &keys::branch_vtx_prefix(branch_id), Snapshot).await? {
		let versionstamp = decode_suffix_versionstamp(&keys::branch_vtx_prefix(branch_id), &key)
			.context("decode sqlite branch VTX versionstamp during GC")?;
		if versionstamp < pin.gc_pin {
			tx.informal().clear(&key);
			vtx_deleted += 1;
		}
	}

	let mut delta_chunks_deleted = 0;
	for (key, _value) in scan_prefix(tx, &keys::branch_delta_prefix(branch_id), Snapshot).await? {
		let txid = keys::decode_branch_delta_chunk_txid(branch_id, &key)?;
		if txid < txid_floor {
			tx.informal().clear(&key);
			delta_chunks_deleted += 1;
		}
	}

	Ok(Some(BranchHotGcOutcome {
		gc_pin: pin.gc_pin,
		txid_floor: Some(txid_floor),
		commits_deleted,
		vtx_deleted,
		delta_chunks_deleted,
	}))
}

async fn gc_pin_txid_floor(
	tx: &universaldb::Transaction,
	branch_id: ActorBranchId,
	gc_pin: [u8; 16],
) -> Result<Option<u64>> {
	if gc_pin == VERSIONSTAMP_ZERO {
		return Ok(None);
	}
	if gc_pin == VERSIONSTAMP_INFINITY {
		return Ok(Some(u64::MAX));
	}

	read_u64_be(tx, &keys::branch_vtx_key(branch_id, gc_pin)).await
}

async fn read_i64_le(tx: &universaldb::Transaction, key: &[u8]) -> Result<i64> {
	let Some(bytes) = tx.informal().get(key, Serializable).await? else {
		return Ok(0);
	};
	let bytes: [u8; std::mem::size_of::<i64>()] = Vec::<u8>::from(bytes)
		.as_slice()
		.try_into()
		.context("sqlite branch refcount should be exactly 8 bytes")?;

	Ok(i64::from_le_bytes(bytes))
}

async fn read_versionstamp_pin(
	tx: &universaldb::Transaction,
	key: &[u8],
) -> Result<[u8; 16]> {
	let Some(bytes) = tx.informal().get(key, Serializable).await? else {
		return Ok(VERSIONSTAMP_INFINITY);
	};
	let pin: [u8; 16] = Vec::<u8>::from(bytes)
		.as_slice()
		.try_into()
		.context("sqlite branch pin should be exactly 16 bytes")?;
	if pin == VERSIONSTAMP_ZERO {
		return Ok(VERSIONSTAMP_INFINITY);
	}

	Ok(pin)
}

async fn read_u64_be(tx: &universaldb::Transaction, key: &[u8]) -> Result<Option<u64>> {
	let Some(bytes) = tx.informal().get(key, Serializable).await? else {
		return Ok(None);
	};
	let bytes: [u8; std::mem::size_of::<u64>()] = Vec::<u8>::from(bytes)
		.as_slice()
		.try_into()
		.context("sqlite VTX entry should be exactly 8 bytes")?;

	Ok(Some(u64::from_be_bytes(bytes)))
}

async fn scan_prefix(
	tx: &universaldb::Transaction,
	prefix: &[u8],
	isolation_level: universaldb::utils::IsolationLevel,
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
	let informal = tx.informal();
	let prefix_subspace =
		universaldb::Subspace::from(universaldb::tuple::Subspace::from_bytes(prefix.to_vec()));
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
			mode: StreamingMode::WantAll,
			..RangeOption::from(&prefix_subspace)
		},
		isolation_level,
	);
	let mut rows = Vec::new();

	while let Some(entry) = stream.try_next().await? {
		rows.push((entry.key().to_vec(), entry.value().to_vec()));
	}

	Ok(rows)
}

fn decode_suffix_u64(prefix: &[u8], key: &[u8]) -> Result<u64> {
	let suffix = key
		.strip_prefix(prefix)
		.context("key did not start with expected prefix")?;
	let bytes: [u8; std::mem::size_of::<u64>()] = suffix
		.try_into()
		.context("key suffix had invalid u64 length")?;

	Ok(u64::from_be_bytes(bytes))
}

fn decode_suffix_versionstamp(prefix: &[u8], key: &[u8]) -> Result<[u8; 16]> {
	let suffix = key
		.strip_prefix(prefix)
		.context("key did not start with expected prefix")?;
	let bytes: [u8; 16] = suffix
		.try_into()
		.context("key suffix had invalid versionstamp length")?;

	Ok(bytes)
}
