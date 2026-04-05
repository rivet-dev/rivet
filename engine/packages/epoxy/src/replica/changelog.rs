use anyhow::{Context, Result, bail};
use epoxy_protocol::protocol;
use futures_util::TryStreamExt;
use universaldb::{
	KeySelector, RangeOption, Transaction,
	options::StreamingMode,
	tuple::Versionstamp,
	utils::{
		FormalKey, IsolationLevel::Serializable, keys::CHANGELOG,
	},
	versionstamp::{generate_versionstamp, substitute_versionstamp},
};

use crate::keys::{self, ChangelogKey, CommittedValue, KvAcceptedKey, KvBallotKey, KvOptimisticCacheKey, KvValueKey};
use crate::metrics;

#[tracing::instrument(skip_all, fields(%replica_id, key = ?key))]
pub fn append(
	replica_id: protocol::ReplicaId,
	tx: &Transaction,
	key: Vec<u8>,
	value: Vec<u8>,
	version: u64,
	mutable: bool,
) -> Result<()> {
	let changelog_key = ChangelogKey::new(Versionstamp::incomplete(0));
	let mut packed_key = keys::subspace(replica_id).pack_with_versionstamp(&changelog_key);
	let versionstamp = generate_versionstamp(0);

	substitute_versionstamp(&mut packed_key, versionstamp)
		.map_err(anyhow::Error::msg)
		.context("failed substituting changelog versionstamp")?;

	let serialized = changelog_key.serialize(protocol::ChangelogEntry {
		key,
		value,
		version,
		mutable,
	})?;
	tx.set(&packed_key, &serialized);
	metrics::record_changelog_append();

	Ok(())
}

#[tracing::instrument(skip_all, fields(%replica_id, count))]
pub async fn read(
	tx: &Transaction,
	replica_id: protocol::ReplicaId,
	req: protocol::ChangelogReadRequest,
) -> Result<protocol::ChangelogReadResponse> {
	let replica_subspace = keys::subspace(replica_id);
	let changelog_subspace = replica_subspace.subspace(&(CHANGELOG,));
	let mut range: RangeOption<'static> = (&changelog_subspace).into();
	range.limit = Some(
		usize::try_from(req.count).context("changelog read count does not fit in usize")?,
	);
	range.mode = StreamingMode::WantAll;

	let mut last_versionstamp = req.after_versionstamp.clone().unwrap_or_default();
	if let Some(after_versionstamp) = req.after_versionstamp {
		let after_key =
			replica_subspace.pack(&ChangelogKey::new(decode_versionstamp(&after_versionstamp)?));
		range.begin = KeySelector::first_greater_than(after_key);
	}

	let mut entries = Vec::new();
	let mut stream = tx.get_ranges_keyvalues(range, Serializable);
	while let Some(entry) = stream.try_next().await? {
		let changelog_key = replica_subspace
			.unpack::<ChangelogKey>(entry.key())
			.context("failed to unpack changelog key")?;
		let changelog_entry = changelog_key
			.deserialize(entry.value())
			.context("failed to deserialize changelog entry")?;

		last_versionstamp = changelog_key.versionstamp().as_bytes().to_vec();
		entries.push(changelog_entry);
	}

	Ok(protocol::ChangelogReadResponse {
		entries,
		last_versionstamp,
	})
}

#[tracing::instrument(skip_all, fields(%replica_id, key = ?entry.key))]
pub async fn apply_entry(
	tx: &Transaction,
	replica_id: protocol::ReplicaId,
	entry: protocol::ChangelogEntry,
) -> Result<()> {
	let tx = tx.with_subspace(keys::subspace(replica_id));
	let value_key = KvValueKey::new(entry.key.clone());
	let accepted_key = KvAcceptedKey::new(entry.key.clone());
	let ballot_key = KvBallotKey::new(entry.key.clone());
	let cache_key = KvOptimisticCacheKey::new(entry.key.clone());

	if let Some(existing_value) = tx.read_opt(&value_key, Serializable).await? {
		if !existing_value.mutable && existing_value.value == entry.value {
			return Ok(());
		}

		if !existing_value.mutable && existing_value.value != entry.value {
			bail!(
				"changelog catch-up saw conflicting committed value for immutable key {:?}",
				value_key.key()
			);
		}

		if entry.version <= existing_value.version {
			return Ok(());
		}
	}

	tx.write(
		&value_key,
		CommittedValue {
			value: entry.value.clone(),
			version: entry.version,
			mutable: entry.mutable,
		},
	)?;
	tx.delete(&accepted_key);
	if entry.mutable {
		tx.delete(&ballot_key);
		tx.delete(&cache_key);
	}
	append(
		replica_id,
		&tx,
		entry.key,
		entry.value,
		entry.version,
		entry.mutable,
	)?;

	Ok(())
}

fn decode_versionstamp(raw: &[u8]) -> Result<Versionstamp> {
	let bytes: [u8; 12] = raw
		.try_into()
		.context("expected 12-byte versionstamp cursor")?;
	Ok(Versionstamp::from(bytes))
}

