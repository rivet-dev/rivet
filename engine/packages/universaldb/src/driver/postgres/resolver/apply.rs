use anyhow::{Context, Result};
use deadpool_postgres::Transaction;

use crate::{
	atomic::apply_atomic_op, options::MutationType, tuple::Versionstamp, tx_ops::Operation,
	versionstamp::substitute_raw_versionstamp,
};

/// Apply a winning transaction's operations to `kv` inside the leader's batch txn.
///
/// `commit_version` is the Postgres-resolved version assigned to this commit (`nextval`). It is
/// substituted into the 8-byte committed-version slot of any versionstamped key/value so
/// versionstamps are globally monotonic with commit order across all follower processes.
pub async fn apply(
	txn: &Transaction<'_>,
	operations: Vec<Operation>,
	commit_version: u64,
) -> Result<()> {
	// Distinguishes multiple versionstamped operations within a single commit so their 10-byte
	// stamps stay unique (8-byte version shared, 2-byte counter incremented).
	let mut versionstamp_counter: u16 = 0;

	for op in operations {
		match op {
			Operation::SetValue { key, value } => {
				upsert(txn, &key, &value).await?;
			}
			Operation::Clear { key } => {
				txn.execute("DELETE FROM kv WHERE key = $1", &[&key])
					.await
					.context("failed to clear key")?;
			}
			Operation::ClearRange { begin, end } => {
				txn.execute(
					"DELETE FROM kv WHERE key >= $1 AND key < $2",
					&[&begin, &end],
				)
				.await
				.context("failed to clear range")?;
			}
			Operation::AtomicOp {
				key,
				param,
				op_type,
			} => {
				apply_atomic(
					txn,
					key,
					param,
					op_type,
					commit_version,
					&mut versionstamp_counter,
				)
				.await?;
			}
		}
	}

	Ok(())
}

async fn apply_atomic(
	txn: &Transaction<'_>,
	key: Vec<u8>,
	param: Vec<u8>,
	op_type: MutationType,
	commit_version: u64,
	versionstamp_counter: &mut u16,
) -> Result<()> {
	match op_type {
		MutationType::SetVersionstampedKey => {
			let versionstamp = build_versionstamp(commit_version, versionstamp_counter);
			let key = substitute_raw_versionstamp(key, &versionstamp)
				.map_err(anyhow::Error::msg)
				.context("failed substituting versionstamped key")?;
			upsert(txn, &key, &param).await?;
		}
		MutationType::SetVersionstampedValue => {
			let versionstamp = build_versionstamp(commit_version, versionstamp_counter);
			let value = substitute_raw_versionstamp(param, &versionstamp)
				.map_err(anyhow::Error::msg)
				.context("failed substituting versionstamped value")?;
			upsert(txn, &key, &value).await?;
		}
		// Read-modify-write atomics: the leader is the single writer, so reading the live value
		// inside the apply txn and writing the result is serializable with no lost update.
		MutationType::Add
		| MutationType::And
		| MutationType::BitAnd
		| MutationType::Or
		| MutationType::BitOr
		| MutationType::Xor
		| MutationType::BitXor
		| MutationType::AppendIfFits
		| MutationType::Max
		| MutationType::Min
		| MutationType::ByteMin
		| MutationType::ByteMax
		| MutationType::CompareAndClear => {
			let current = txn
				.query_opt("SELECT value FROM kv WHERE key = $1", &[&key])
				.await
				.context("failed to read current value for atomic op")?
				.map(|row| row.get::<_, Vec<u8>>(0));

			let new_value = apply_atomic_op(current.as_deref(), &param, op_type);

			if let Some(new_value) = new_value {
				upsert(txn, &key, &new_value).await?;
			} else {
				txn.execute("DELETE FROM kv WHERE key = $1", &[&key])
					.await
					.context("failed to clear key after atomic op")?;
			}
		}
	}

	Ok(())
}

async fn upsert(txn: &Transaction<'_>, key: &[u8], value: &[u8]) -> Result<()> {
	txn.execute(
		"INSERT INTO kv (key, value) VALUES ($1, $2) ON CONFLICT (key) DO UPDATE SET value = $2",
		&[&key, &value],
	)
	.await
	.context("failed to upsert kv")?;
	Ok(())
}

/// Build a 10-byte versionstamp (plus the 2 user-version bytes the substitution helper ignores)
/// from the Postgres-resolved commit version and a per-commit counter.
fn build_versionstamp(commit_version: u64, counter: &mut u16) -> Versionstamp {
	let mut bytes = [0u8; 12];
	bytes[0..8].copy_from_slice(&commit_version.to_be_bytes());
	bytes[8..10].copy_from_slice(&counter.to_be_bytes());
	*counter = counter.wrapping_add(1);
	Versionstamp::from(bytes)
}
