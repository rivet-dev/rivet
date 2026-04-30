use std::{
	collections::BTreeSet,
	time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use futures_util::TryStreamExt;
use universaldb::{
	RangeOption,
	options::MutationType,
	options::StreamingMode,
	utils::IsolationLevel::{self, Serializable},
};

use super::{
	constants::{MAX_FORK_DEPTH, MAX_NAMESPACE_DEPTH},
	error::SqliteStorageError,
	keys, udb,
	types::{
		ActorBranchId, ActorBranchRecord, ActorPointer, BookmarkRef, BranchState, DBHead,
		NamespaceBranchId, NamespaceBranchRecord, NamespaceId, NamespacePointer,
		ResolvedVersionstamp,
		decode_actor_branch_record, decode_actor_pointer, decode_commit_row,
		decode_namespace_branch_record, decode_namespace_pointer, encode_actor_branch_record,
		encode_actor_pointer, encode_db_head, encode_namespace_branch_record,
		encode_namespace_pointer,
	},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NamespaceBranchResolution {
	pub branch_id: NamespaceBranchId,
	pub initialized: bool,
}

pub async fn resolve_or_allocate_root_namespace_branch(
	tx: &universaldb::Transaction,
	namespace_id: NamespaceId,
) -> Result<NamespaceBranchResolution> {
	if let Some(branch_id) =
		resolve_namespace_branch(tx, namespace_id, IsolationLevel::Serializable).await?
	{
		return Ok(NamespaceBranchResolution {
			branch_id,
			initialized: false,
		});
	}

	Ok(NamespaceBranchResolution {
		branch_id: NamespaceBranchId::new_v4(),
		initialized: true,
	})
}

pub async fn resolve_namespace_branch(
	tx: &universaldb::Transaction,
	namespace_id: NamespaceId,
	isolation_level: IsolationLevel,
) -> Result<Option<NamespaceBranchId>> {
	let Some(pointer_bytes) = tx
		.informal()
		.get(&keys::namespace_pointer_cur_key(namespace_id), isolation_level)
		.await?
	else {
		return Ok(None);
	};

	let pointer = decode_namespace_pointer(&pointer_bytes).context("decode sqlite namespace pointer")?;
	Ok(Some(pointer.current_branch))
}

pub fn write_root_namespace_metadata(
	tx: &universaldb::Transaction,
	namespace_id: NamespaceId,
	branch_id: NamespaceBranchId,
	now_ms: i64,
	root_versionstamp: &[u8; 16],
) -> Result<()> {
	let record = NamespaceBranchRecord {
		branch_id,
		parent: None,
		parent_versionstamp: None,
		root_versionstamp: *root_versionstamp,
		fork_depth: 0,
		created_at_ms: now_ms,
		created_from_bookmark: None,
		state: BranchState::Live,
	};
	let encoded_record =
		encode_namespace_branch_record(record).context("encode sqlite root namespace branch record")?;
	let versionstamped_record = udb::append_versionstamp_offset(encoded_record, root_versionstamp)
		.context("prepare versionstamped sqlite root namespace branch record")?;
	tx.informal().atomic_op(
		&keys::namespace_branches_list_key(branch_id),
		&versionstamped_record,
		MutationType::SetVersionstampedValue,
	);
	tx.informal().atomic_op(
		&keys::namespace_branches_refcount_key(branch_id),
		&1_i64.to_le_bytes(),
		MutationType::Add,
	);

	let pointer = NamespacePointer {
		current_branch: branch_id,
		last_swapped_at_ms: now_ms,
	};
	let encoded_pointer =
		encode_namespace_pointer(pointer).context("encode sqlite namespace pointer")?;
	tx.informal()
		.set(&keys::namespace_pointer_cur_key(namespace_id), &encoded_pointer);

	Ok(())
}

pub async fn resolve_actor_branch(
	tx: &universaldb::Transaction,
	namespace_id: NamespaceId,
	actor_id: &str,
	isolation_level: IsolationLevel,
) -> Result<Option<ActorBranchId>> {
	let Some(namespace_branch_id) =
		resolve_namespace_branch(tx, namespace_id, isolation_level).await?
	else {
		return resolve_actor_branch_in_namespace(
			tx,
			NamespaceBranchId::nil(),
			actor_id,
			isolation_level,
		)
		.await;
	};

	if let Some(branch_id) =
		resolve_actor_branch_in_namespace(tx, namespace_branch_id, actor_id, isolation_level).await?
	{
		return Ok(Some(branch_id));
	}

	resolve_actor_branch_in_namespace(tx, NamespaceBranchId::nil(), actor_id, isolation_level).await
}

pub async fn resolve_actor_branch_in_namespace(
	tx: &universaldb::Transaction,
	namespace_branch_id: NamespaceBranchId,
	actor_id: &str,
	isolation_level: IsolationLevel,
) -> Result<Option<ActorBranchId>> {
	Ok(resolve_actor_pointer(tx, namespace_branch_id, actor_id, isolation_level)
		.await?
		.map(|pointer| pointer.current_branch))
}

pub async fn resolve_actor_pointer(
	tx: &universaldb::Transaction,
	namespace_branch_id: NamespaceBranchId,
	actor_id: &str,
	isolation_level: IsolationLevel,
) -> Result<Option<ActorPointer>> {
	let mut current_branch_id = namespace_branch_id;

	for _ in 0..=MAX_NAMESPACE_DEPTH {
		if let Some(pointer_bytes) = tx
			.informal()
			.get(
				&keys::actor_pointer_cur_key(current_branch_id, actor_id),
				isolation_level,
			)
			.await?
		{
			let pointer = decode_actor_pointer(&pointer_bytes).context("decode sqlite actor pointer")?;
			return Ok(Some(pointer));
		}

		if current_branch_id == NamespaceBranchId::nil() {
			return Ok(None);
		}

		if tx
			.informal()
			.get(
				&keys::namespace_branches_actor_tombstone_key(current_branch_id, actor_id),
				isolation_level,
			)
			.await?
			.is_some()
		{
			return Err(SqliteStorageError::ActorNotFound.into());
		}

		let Some(record_bytes) = tx
			.informal()
			.get(&keys::namespace_branches_list_key(current_branch_id), isolation_level)
			.await?
		else {
			return Ok(None);
		};
		let record = decode_namespace_branch_record(&record_bytes)
			.context("decode sqlite namespace branch record")?;
		let Some(parent) = record.parent else {
			return Ok(None);
		};
		current_branch_id = parent;
	}

	Err(SqliteStorageError::NamespaceForkChainTooDeep.into())
}

pub async fn fork_actor(
	udb: &universaldb::Database,
	source_namespace: NamespaceId,
	source_actor_id: String,
	at: ResolvedVersionstamp,
	target_namespace: NamespaceId,
) -> Result<String> {
	let new_actor_id = format!("fork-{}", uuid::Uuid::new_v4().simple());
	let new_actor_branch_id = ActorBranchId::new_v4();

	udb.run({
		let new_actor_id = new_actor_id.clone();
		move |tx| {
			let source_actor_id = source_actor_id.clone();
			let new_actor_id = new_actor_id.clone();
			let at = at.clone();

			async move {
				let source_namespace_branch =
					resolve_namespace_branch(&tx, source_namespace, Serializable)
						.await?
						.ok_or(SqliteStorageError::ActorNotFound)?;
				let source_actor_branch = resolve_actor_branch_in_namespace(
					&tx,
					source_namespace_branch,
					&source_actor_id,
					Serializable,
				)
				.await?
				.ok_or(SqliteStorageError::ActorNotFound)?;
				let target_namespace_branch =
					resolve_namespace_branch(&tx, target_namespace, Serializable)
						.await?
						.ok_or(SqliteStorageError::ActorNotFound)?;

				derive_branch_at(
					&tx,
					source_actor_branch,
					at.versionstamp,
					new_actor_branch_id,
					target_namespace_branch,
					at.bookmark,
				)
				.await?;

				let pointer = super::types::ActorPointer {
					current_branch: new_actor_branch_id,
					last_swapped_at_ms: now_ms()?,
				};
				let encoded_pointer =
					encode_actor_pointer(pointer).context("encode sqlite fork actor pointer")?;
				tx.informal().set(
					&keys::actor_pointer_cur_key(target_namespace_branch, &new_actor_id),
					&encoded_pointer,
				);
				write_namespace_catalog_marker(&tx, target_namespace_branch, new_actor_branch_id)?;

				Ok(())
			}
		}
	})
	.await?;

	Ok(new_actor_id)
}

pub async fn list_databases(
	udb: &universaldb::Database,
	namespace: NamespaceId,
) -> Result<Vec<ActorBranchId>> {
	udb.run(move |tx| async move {
		let Some(namespace_branch_id) =
			resolve_namespace_branch(&tx, namespace, Serializable).await?
		else {
			return Ok(Vec::new());
		};

		list_databases_in_namespace_branch(&tx, namespace_branch_id).await
	})
	.await
}

pub async fn delete_database(
	udb: &universaldb::Database,
	namespace: NamespaceId,
	database_id: ActorBranchId,
) -> Result<()> {
	udb.run(move |tx| async move {
		let namespace_branch_id = resolve_namespace_branch(&tx, namespace, Serializable)
			.await?
			.ok_or(SqliteStorageError::ActorNotFound)?;

		let visible =
			is_database_visible_in_namespace_branch(&tx, namespace_branch_id, database_id).await?;
		if !visible {
			return Err(SqliteStorageError::ActorNotFound.into());
		}

		tx.informal().set(
			&keys::namespace_branches_database_tombstone_key(namespace_branch_id, database_id),
			&[],
		);
		tx.informal().atomic_op(
			&keys::branches_refcount_key(database_id),
			&(-1_i64).to_le_bytes(),
			MutationType::Add,
		);

		Ok(())
	})
	.await
}

pub async fn fork_namespace(
	udb: &universaldb::Database,
	source_namespace: NamespaceId,
	at: ResolvedVersionstamp,
) -> Result<NamespaceId> {
	let new_namespace_id = NamespaceId::new_v4();
	let new_namespace_branch_id = NamespaceBranchId::new_v4();

	udb.run({
		let at = at.clone();

		move |tx| {
			let at = at.clone();

			async move {
				let source_namespace_branch =
					resolve_namespace_branch(&tx, source_namespace, Serializable)
						.await?
						.ok_or(SqliteStorageError::ActorNotFound)?;

				derive_namespace_branch_at(
					&tx,
					source_namespace_branch,
					at.versionstamp,
					new_namespace_branch_id,
					at.bookmark,
				)
				.await?;

				let pointer = NamespacePointer {
					current_branch: new_namespace_branch_id,
					last_swapped_at_ms: now_ms()?,
				};
				let encoded_pointer =
					encode_namespace_pointer(pointer).context("encode sqlite fork namespace pointer")?;
				tx.informal()
					.set(&keys::namespace_pointer_cur_key(new_namespace_id), &encoded_pointer);

				Ok(())
			}
		}
	})
	.await?;

	Ok(new_namespace_id)
}

pub async fn rollback_namespace(
	udb: &universaldb::Database,
	namespace: NamespaceId,
	at: ResolvedVersionstamp,
) -> Result<NamespaceBranchId> {
	let rolled_branch_id = NamespaceBranchId::new_v4();

	udb.run({
		let at = at.clone();

		move |tx| {
			let at = at.clone();

			async move {
				let cur_ptr_bytes = tx
					.informal()
					.get(&keys::namespace_pointer_cur_key(namespace), Serializable)
					.await?
					.ok_or(SqliteStorageError::ActorNotFound)?;
				let cur_ptr = decode_namespace_pointer(&cur_ptr_bytes)
					.context("decode sqlite namespace pointer for rollback")?;
				let cur_record = read_namespace_branch_record(&tx, cur_ptr.current_branch).await?;

				derive_namespace_branch_at(
					&tx,
					cur_ptr.current_branch,
					at.versionstamp,
					rolled_branch_id,
					at.bookmark,
				)
				.await?;
				freeze_namespace_branch(&tx, cur_record).await?;

				let now_ms = now_ms()?;
				let nonce = uuid::Uuid::new_v4().as_u128() as u32;
				let encoded_history_pointer = encode_namespace_pointer(cur_ptr.clone())
					.context("encode sqlite rollback namespace pointer history")?;
				tx.informal().set(
					&keys::namespace_pointer_history_key(namespace, now_ms, nonce),
					&encoded_history_pointer,
				);
				tx.informal().atomic_op(
					&keys::namespace_branches_refcount_key(cur_ptr.current_branch),
					&(-1_i64).to_le_bytes(),
					MutationType::Add,
				);

				let new_ptr = NamespacePointer {
					current_branch: rolled_branch_id,
					last_swapped_at_ms: now_ms,
				};
				let encoded_pointer =
					encode_namespace_pointer(new_ptr).context("encode sqlite rollback namespace pointer")?;
				tx.informal()
					.set(&keys::namespace_pointer_cur_key(namespace), &encoded_pointer);

				Ok(())
			}
		}
	})
	.await?;

	Ok(rolled_branch_id)
}

pub async fn rollback_actor(
	udb: &universaldb::Database,
	namespace: NamespaceId,
	actor_id: String,
	at: ResolvedVersionstamp,
) -> Result<ActorBranchId> {
	let rolled_branch_id = ActorBranchId::new_v4();

	udb.run({
		let actor_id = actor_id.clone();
		let at = at.clone();

		move |tx| {
			let actor_id = actor_id.clone();
			let at = at.clone();

			async move {
				let namespace_branch =
					resolve_namespace_branch(&tx, namespace, Serializable)
						.await?
						.ok_or(SqliteStorageError::ActorNotFound)?;
				let cur_ptr = resolve_actor_pointer(
					&tx,
					namespace_branch,
					&actor_id,
					Serializable,
				)
				.await?
				.ok_or(SqliteStorageError::ActorNotFound)?;
				let cur_record = read_actor_branch_record(&tx, cur_ptr.current_branch).await?;

				derive_branch_at(
					&tx,
					cur_ptr.current_branch,
					at.versionstamp,
					rolled_branch_id,
					cur_record.namespace_branch,
					at.bookmark,
				)
				.await?;
				freeze_actor_branch(&tx, cur_record).await?;

				let now_ms = now_ms()?;
				let nonce = uuid::Uuid::new_v4().as_u128() as u32;
				let encoded_history_pointer = encode_actor_pointer(cur_ptr.clone())
					.context("encode sqlite rollback actor pointer history")?;
				tx.informal().set(
					&keys::actor_pointer_history_key(namespace_branch, &actor_id, now_ms, nonce),
					&encoded_history_pointer,
				);
				tx.informal().atomic_op(
					&keys::branches_refcount_key(cur_ptr.current_branch),
					&(-1_i64).to_le_bytes(),
					MutationType::Add,
				);

				let new_ptr = ActorPointer {
					current_branch: rolled_branch_id,
					last_swapped_at_ms: now_ms,
				};
				let encoded_pointer =
					encode_actor_pointer(new_ptr).context("encode sqlite rollback actor pointer")?;
				tx.informal().set(
					&keys::actor_pointer_cur_key(namespace_branch, &actor_id),
					&encoded_pointer,
				);

				Ok(())
			}
		}
	})
	.await?;

	Ok(rolled_branch_id)
}

pub async fn derive_branch_at(
	tx: &universaldb::Transaction,
	source_branch_id: ActorBranchId,
	at_versionstamp: [u8; 16],
	new_branch_id: ActorBranchId,
	namespace_branch: NamespaceBranchId,
	bookmark_ref: Option<BookmarkRef>,
) -> Result<()> {
	let source = read_actor_branch_record(tx, source_branch_id).await?;
	if source.fork_depth >= MAX_FORK_DEPTH {
		return Err(SqliteStorageError::ForkChainTooDeep.into());
	}

	let bk_pin = read_versionstamp_pin(tx, &keys::branches_bk_pin_key(source_branch_id)).await?;
	if bk_pin > at_versionstamp {
		return Err(SqliteStorageError::ForkOutOfRetention.into());
	}

	let txid_at_versionstamp = lookup_txid_at_versionstamp(tx, source_branch_id, at_versionstamp)
		.await
		.with_context(|| {
			format!(
				"lookup sqlite VTX entry for actor branch {}",
				source_branch_id.as_uuid()
			)
		})?;
	let commit_at_versionstamp = read_commit_row(tx, source_branch_id, txid_at_versionstamp)
		.await
		.with_context(|| {
			format!(
				"read sqlite commit row {txid_at_versionstamp} for actor branch {}",
				source_branch_id.as_uuid()
			)
		})?;
	let head_at_fork = DBHead {
		head_txid: txid_at_versionstamp,
		db_size_pages: commit_at_versionstamp.db_size_pages,
		post_apply_checksum: commit_at_versionstamp.post_apply_checksum,
		branch_id: new_branch_id,
		#[cfg(debug_assertions)]
		generation: 0,
	};
	let encoded_head_at_fork =
		encode_db_head(head_at_fork).context("encode sqlite fork head snapshot")?;
	tx.informal().set(
		&keys::branch_meta_head_at_fork_key(new_branch_id),
		&encoded_head_at_fork,
	);

	let new_record = ActorBranchRecord {
		branch_id: new_branch_id,
		namespace_branch,
		parent: Some(source_branch_id),
		parent_versionstamp: Some(at_versionstamp),
		root_versionstamp: at_versionstamp,
		fork_depth: source.fork_depth + 1,
		created_at_ms: now_ms()?,
		created_from_bookmark: bookmark_ref,
		state: BranchState::Live,
	};
	let encoded_record =
		encode_actor_branch_record(new_record).context("encode sqlite derived actor branch record")?;
	tx.informal()
		.set(&keys::branches_list_key(new_branch_id), &encoded_record);
	tx.informal().atomic_op(
		&keys::branches_refcount_key(source_branch_id),
		&1_i64.to_le_bytes(),
		MutationType::Add,
	);
	tx.informal().atomic_op(
		&keys::branches_refcount_key(new_branch_id),
		&1_i64.to_le_bytes(),
		MutationType::Add,
	);
	tx.informal().atomic_op(
		&keys::branches_desc_pin_key(source_branch_id),
		&at_versionstamp,
		MutationType::ByteMin,
	);

	Ok(())
}

async fn freeze_actor_branch(
	tx: &universaldb::Transaction,
	mut record: ActorBranchRecord,
) -> Result<()> {
	record.state = BranchState::Frozen;
	let branch_id = record.branch_id;
	let encoded_record =
		encode_actor_branch_record(record).context("encode frozen sqlite actor branch record")?;
	tx.informal()
		.set(&keys::branches_list_key(branch_id), &encoded_record);

	Ok(())
}

async fn freeze_namespace_branch(
	tx: &universaldb::Transaction,
	mut record: NamespaceBranchRecord,
) -> Result<()> {
	record.state = BranchState::Frozen;
	let branch_id = record.branch_id;
	let encoded_record =
		encode_namespace_branch_record(record).context("encode frozen sqlite namespace branch record")?;
	tx.informal()
		.set(&keys::namespace_branches_list_key(branch_id), &encoded_record);

	Ok(())
}

pub async fn derive_namespace_branch_at(
	tx: &universaldb::Transaction,
	source_branch_id: NamespaceBranchId,
	at_versionstamp: [u8; 16],
	new_branch_id: NamespaceBranchId,
	bookmark_ref: Option<BookmarkRef>,
) -> Result<()> {
	let source = read_namespace_branch_record(tx, source_branch_id).await?;
	if source.fork_depth >= MAX_NAMESPACE_DEPTH {
		return Err(SqliteStorageError::NamespaceForkChainTooDeep.into());
	}

	let bk_pin =
		read_versionstamp_pin(tx, &keys::namespace_branches_bk_pin_key(source_branch_id)).await?;
	if bk_pin > at_versionstamp {
		return Err(SqliteStorageError::ForkOutOfRetention.into());
	}

	let new_record = NamespaceBranchRecord {
		branch_id: new_branch_id,
		parent: Some(source_branch_id),
		parent_versionstamp: Some(at_versionstamp),
		root_versionstamp: at_versionstamp,
		fork_depth: source.fork_depth + 1,
		created_at_ms: now_ms()?,
		created_from_bookmark: bookmark_ref,
		state: BranchState::Live,
	};
	let encoded_record = encode_namespace_branch_record(new_record)
		.context("encode sqlite derived namespace branch record")?;
	tx.informal()
		.set(&keys::namespace_branches_list_key(new_branch_id), &encoded_record);
	tx.informal().atomic_op(
		&keys::namespace_branches_refcount_key(source_branch_id),
		&1_i64.to_le_bytes(),
		MutationType::Add,
	);
	tx.informal().atomic_op(
		&keys::namespace_branches_refcount_key(new_branch_id),
		&1_i64.to_le_bytes(),
		MutationType::Add,
	);
	tx.informal().atomic_op(
		&keys::namespace_branches_desc_pin_key(source_branch_id),
		&at_versionstamp,
		MutationType::ByteMin,
	);

	Ok(())
}

async fn list_databases_in_namespace_branch(
	tx: &universaldb::Transaction,
	namespace_branch_id: NamespaceBranchId,
) -> Result<Vec<ActorBranchId>> {
	let mut result = BTreeSet::new();
	let mut tombstones = BTreeSet::new();
	let mut current_branch_id = namespace_branch_id;
	let mut versionstamp_cap = [0xff; 16];

	for depth in 0..=MAX_NAMESPACE_DEPTH {
		for database_id in scan_database_tombstones(tx, current_branch_id).await? {
			tombstones.insert(database_id);
			result.remove(&database_id);
		}

		for (database_id, catalog_versionstamp) in
			scan_namespace_catalog(tx, current_branch_id).await?
		{
			if catalog_versionstamp <= versionstamp_cap && !tombstones.contains(&database_id) {
				result.insert(database_id);
			}
		}

		let record = read_namespace_branch_record(tx, current_branch_id).await?;
		let Some(parent_branch_id) = record.parent else {
			return Ok(result.into_iter().collect());
		};
		if depth == MAX_NAMESPACE_DEPTH {
			return Err(SqliteStorageError::NamespaceForkChainTooDeep.into());
		}

		versionstamp_cap = record
			.parent_versionstamp
			.context("sqlite namespace branch parent versionstamp is missing")?;
		current_branch_id = parent_branch_id;
	}

	Err(SqliteStorageError::NamespaceForkChainTooDeep.into())
}

async fn is_database_visible_in_namespace_branch(
	tx: &universaldb::Transaction,
	namespace_branch_id: NamespaceBranchId,
	database_id: ActorBranchId,
) -> Result<bool> {
	Ok(list_databases_in_namespace_branch(tx, namespace_branch_id)
		.await?
		.contains(&database_id))
}

fn write_namespace_catalog_marker(
	tx: &universaldb::Transaction,
	namespace_branch_id: NamespaceBranchId,
	database_id: ActorBranchId,
) -> Result<()> {
	let value = udb::append_versionstamp_offset(
		udb::INCOMPLETE_VERSIONSTAMP.to_vec(),
		&udb::INCOMPLETE_VERSIONSTAMP,
	)
	.context("prepare versionstamped sqlite namespace catalog marker")?;
	tx.informal().atomic_op(
		&keys::namespace_catalog_key(namespace_branch_id, database_id),
		&value,
		MutationType::SetVersionstampedValue,
	);

	Ok(())
}

async fn scan_namespace_catalog(
	tx: &universaldb::Transaction,
	namespace_branch_id: NamespaceBranchId,
) -> Result<Vec<(ActorBranchId, [u8; 16])>> {
	let rows = tx_scan_prefix_values(tx, &keys::namespace_catalog_prefix(namespace_branch_id)).await?;
	rows.into_iter()
		.map(|(key, value)| {
			let database_id = keys::decode_namespace_catalog_database_id(namespace_branch_id, &key)?;
			let versionstamp = decode_versionstamp_value(&value)
				.context("decode sqlite namespace catalog versionstamp")?;

			Ok((database_id, versionstamp))
		})
		.collect()
}

async fn scan_database_tombstones(
	tx: &universaldb::Transaction,
	namespace_branch_id: NamespaceBranchId,
) -> Result<Vec<ActorBranchId>> {
	let rows = tx_scan_prefix_values(
		tx,
		&keys::namespace_branches_database_tombstone_prefix(namespace_branch_id),
	)
	.await?;
	rows.into_iter()
		.map(|(key, _)| {
			keys::decode_namespace_branches_database_tombstone_id(namespace_branch_id, &key)
		})
		.collect()
}

fn decode_versionstamp_value(bytes: &[u8]) -> Result<[u8; 16]> {
	bytes
		.try_into()
		.context("sqlite versionstamp value should be exactly 16 bytes")
}

async fn tx_scan_prefix_values(
	tx: &universaldb::Transaction,
	prefix: &[u8],
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
	let prefix_subspace =
		universaldb::Subspace::from(universaldb::tuple::Subspace::from_bytes(prefix.to_vec()));
	let informal = tx.informal();
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
			mode: StreamingMode::WantAll,
			..RangeOption::from(&prefix_subspace)
		},
		Serializable,
	);
	let mut rows = Vec::new();

	while let Some(entry) = stream.try_next().await? {
		rows.push((entry.key().to_vec(), entry.value().to_vec()));
	}

	Ok(rows)
}

pub(super) async fn read_actor_branch_record(
	tx: &universaldb::Transaction,
	branch_id: ActorBranchId,
) -> Result<ActorBranchRecord> {
	let bytes = tx
		.informal()
		.get(&keys::branches_list_key(branch_id), Serializable)
		.await?
		.context("sqlite actor branch record is missing")?;

	decode_actor_branch_record(&bytes).context("decode sqlite actor branch record")
}

async fn read_namespace_branch_record(
	tx: &universaldb::Transaction,
	branch_id: NamespaceBranchId,
) -> Result<NamespaceBranchRecord> {
	let bytes = tx
		.informal()
		.get(&keys::namespace_branches_list_key(branch_id), Serializable)
		.await?
		.context("sqlite namespace branch record is missing")?;

	decode_namespace_branch_record(&bytes).context("decode sqlite namespace branch record")
}

async fn read_versionstamp_pin(
	tx: &universaldb::Transaction,
	key: &[u8],
) -> Result<[u8; 16]> {
	let Some(bytes) = tx.informal().get(key, Serializable).await? else {
		return Ok([0; 16]);
	};
	let bytes = Vec::<u8>::from(bytes);
	bytes
		.as_slice()
		.try_into()
		.context("sqlite branch pin should be exactly 16 bytes")
}

async fn lookup_txid_at_versionstamp(
	tx: &universaldb::Transaction,
	branch_id: ActorBranchId,
	versionstamp: [u8; 16],
) -> Result<u64> {
	let bytes = tx
		.informal()
		.get(&keys::branch_vtx_key(branch_id, versionstamp), Serializable)
		.await?
		.ok_or(SqliteStorageError::BookmarkExpired)?;
	let bytes = Vec::<u8>::from(bytes);
	let bytes: [u8; 8] = bytes
		.as_slice()
		.try_into()
		.context("sqlite VTX entry should be exactly 8 bytes")?;

	Ok(u64::from_be_bytes(bytes))
}

async fn read_commit_row(
	tx: &universaldb::Transaction,
	branch_id: ActorBranchId,
	txid: u64,
) -> Result<super::types::CommitRow> {
	let bytes = tx
		.informal()
		.get(&keys::branch_commit_key(branch_id, txid), Serializable)
		.await?
		.ok_or(SqliteStorageError::BookmarkExpired)?;

	decode_commit_row(&bytes).context("decode sqlite commit row")
}

fn now_ms() -> Result<i64> {
	let millis = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.context("system clock is before unix epoch")?
		.as_millis();
	i64::try_from(millis).context("current timestamp exceeded i64 milliseconds")
}
