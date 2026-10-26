//! Conditional object-store access using the typed, versioned manifest key.

use super::state::alarms::AlarmManifestState;
use super::types::*;
use depot_types::keys::s3::ManifestKey;
use object_store::{GetOptions, ObjectStore, ObjectStoreExt, PutMode, UpdateVersion};
use rivet_keys::keys::{KeyAddress, codec::FormalKey};

const MAX_MANIFEST_BYTES: usize = 16 * 1024 * 1024;

/// The stored envelope of a manifest written without alarm state: every tablet's alarm
/// index is legacy-empty and no split holds one.
pub(crate) fn legacy_envelope(
	manifest: rivet_data::generated::depot_tablet_manifest_v1::TabletManifest,
) -> rivet_data::generated::depot_tablet_manifest_v1::AlarmTabletManifest {
	use rivet_data::generated::depot_tablet_manifest_v1::{
		AlarmIndexInitialization, AlarmIndexPhase, AlarmTabletManifest,
	};

	let initialization = manifest
		.tablets
		.iter()
		.map(|tablet| {
			let tablet_id = uuid::Uuid::from_bytes(tablet.id).to_string();
			let legacy = AlarmIndexInitialization {
				phase: AlarmIndexPhase::LegacyEmpty,
				source: None,
			};
			(tablet_id.into(), legacy)
		})
		.collect();
	AlarmTabletManifest {
		manifest,
		initialization,
		holds: Vec::new(),
	}
}

/// Every accepted mutation leaves room to publish the already-admitted children. `alarms` is
/// the alarm state the mutation would store beside `manifest`, since it decides which
/// envelope the write encodes.
pub(crate) fn has_completion_headroom(
	manifest: &rivet_data::generated::depot_tablet_manifest_v1::TabletManifest,
	alarms: &AlarmManifestState,
) -> bool {
	has_completion_headroom_limit(manifest, alarms, MAX_MANIFEST_BYTES)
}

fn has_completion_headroom_limit(
	manifest: &rivet_data::generated::depot_tablet_manifest_v1::TabletManifest,
	alarms: &AlarmManifestState,
	limit: usize,
) -> bool {
	let Some(completed) = completed_envelope(manifest, alarms) else {
		return false;
	};
	ManifestKey::new()
		.serialize(completed)
		.is_ok_and(|bytes| bytes.len() <= limit)
		&& ManifestKey::new()
			.serialize(stored_envelope(manifest.clone(), alarms))
			.is_ok_and(|bytes| bytes.len() <= limit)
}

/// The envelope the coordinator stores for `manifest`: the alarm state's own when alarms are
/// enabled, else the legacy one that [`write`] encodes.
pub(crate) fn stored_envelope(
	manifest: rivet_data::generated::depot_tablet_manifest_v1::TabletManifest,
	alarms: &AlarmManifestState,
) -> rivet_data::generated::depot_tablet_manifest_v1::AlarmTabletManifest {
	if alarms.enabled {
		alarms.envelope(manifest)
	} else {
		legacy_envelope(manifest)
	}
}

/// The envelope stored once every split admitted in `manifest` has published its children,
/// with the children's clone sources and the parents' holds that publication adds, sized at
/// the largest encoding of the version and epochs publication assigns. `None` when a split
/// cannot complete against `manifest`.
pub(crate) fn completed_envelope(
	manifest: &rivet_data::generated::depot_tablet_manifest_v1::TabletManifest,
	alarms: &AlarmManifestState,
) -> Option<rivet_data::generated::depot_tablet_manifest_v1::AlarmTabletManifest> {
	let mut completed = manifest.clone();
	let mut completed_alarms = alarms.clone();
	let parents: Vec<_> = manifest
		.tablets
		.iter()
		.filter(|tablet| tablet.split.is_some())
		.cloned()
		.collect();
	for parent in &parents {
		let split = parent.split.as_ref()?;
		let report = rivet_data::generated::depot_tablet_coordinator_protocol_v1::SplitPrepared {
			tablet_id: parent.id,
			tablet_grant_epoch: parent.tablet_grant_epoch,
			checkpoint_id: split.checkpoint_id,
		};
		super::state::split::prepared(&mut completed, &report, i64::MAX).ok()?;
		if completed_alarms.enabled {
			completed_alarms
				.split_prepared(parent, &completed, u64::MAX)
				.ok()?;
		}
	}
	for child in &mut completed.tablets {
		if child
			.parent_tablet_id
			.is_some_and(|parent| parents.iter().any(|t| t.id == parent))
		{
			child.tablet_grant_epoch = u64::MAX - 1;
		}
	}
	// Publication increments the version; reserve its largest encoding too.
	completed.tablet_manifest_version = u64::MAX;
	if completed_alarms.enabled {
		completed_alarms.fill_native_empty(&completed);
	}
	Some(stored_envelope(completed, &completed_alarms))
}

/// A missing object is distinct from a failed read; only a confirmed ETag permits an update.
#[tracing::instrument(skip_all)]
pub async fn read(store: &dyn ObjectStore) -> ReadResult {
	read_inner(store, false).await
}
pub(crate) async fn read_alarm(store: &dyn ObjectStore) -> ReadResult {
	read_inner(store, true).await
}
async fn read_inner(store: &dyn ObjectStore, alarms: bool) -> ReadResult {
	match store.get(&ManifestKey::new().pack()).await {
		Ok(result) => read_body(result, alarms).await,
		Err(error) => read_error(error),
	}
}

/// Reads the manifest only if its ETag differs from `etag`, which a previous read returned.
/// An unchanged object costs one billed GET and transfers no body. A store that ignores the
/// condition answers with the whole object, which reads exactly like an unconditional read.
pub(crate) async fn read_alarm_if_none_match(
	store: &dyn ObjectStore,
	etag: &str,
) -> ConditionalRead {
	let options = GetOptions {
		if_none_match: Some(etag.to_owned()),
		..GetOptions::default()
	};
	match store.get_opts(&ManifestKey::new().pack(), options).await {
		Ok(result) => ConditionalRead::Read(read_body(result, true).await),
		Err(object_store::Error::NotModified { .. }) => ConditionalRead::Unchanged,
		Err(error) => ConditionalRead::Read(read_error(error)),
	}
}

/// Maps a failed GET: a missing object is `Missing`, and every other failure is `Unknown`,
/// including a store that reports an unconditional read as not modified, since that answers
/// a condition it was not sent.
pub(crate) fn read_error(error: object_store::Error) -> ReadResult {
	match error {
		object_store::Error::NotFound { .. } => ReadResult::Missing,
		error => ReadResult::Unknown(error.to_string()),
	}
}

/// Decodes a manifest the store returned. An empty ETag, or the wildcard `*` that would match
/// any object in a later `If-None-Match` or `If-Match`, counts as no ETag at all.
async fn read_body(result: object_store::GetResult, alarms: bool) -> ReadResult {
	let Some(etag) = result
		.meta
		.e_tag
		.clone()
		.filter(|etag| !etag.is_empty() && etag != "*")
	else {
		return ReadResult::Invalid("manifest has no ETag".into());
	};
	if result.meta.size > MAX_MANIFEST_BYTES as u64 {
		return ReadResult::Invalid("manifest exceeds byte limit".into());
	}
	let bytes = match result.bytes().await {
		Ok(bytes) => bytes,
		Err(error) => return ReadResult::Unknown(error.to_string()),
	};
	if bytes.len() > MAX_MANIFEST_BYTES {
		return ReadResult::Invalid("manifest exceeds byte limit".into());
	}
	use vbare::Versioned;
	match rivet_data::versioned::DepotAlarmTabletManifestData::deserialize_with_embedded_version(
		&bytes,
	) {
		Ok(envelope) if alarms => ReadResult::FoundAlarm { envelope, etag },
		Ok(envelope) => ReadResult::Found(StoredManifest {
			manifest: envelope.manifest,
			etag,
		}),
		Err(error) => ReadResult::Invalid(error.to_string()),
	}
}

/// Does not retry or rebase. Unknown outcomes must retain this exact precondition.
#[tracing::instrument(skip_all)]
pub async fn write(store: &dyn ObjectStore, write: ManifestWrite) -> WriteResult {
	let bytes = ManifestKey::new().serialize(legacy_envelope(write.manifest.clone()));
	write_bytes(store, write, bytes).await
}
pub(crate) async fn write_alarm(
	store: &dyn ObjectStore,
	write: ManifestWrite,
	envelope: rivet_data::generated::depot_tablet_manifest_v1::AlarmTabletManifest,
) -> WriteResult {
	use vbare::Versioned;
	if envelope.manifest != write.manifest || super::state::alarms::validate(&envelope).is_err() {
		return WriteResult::Unknown("invalid alarm manifest".into());
	}
	let bytes =
		rivet_data::versioned::DepotAlarmTabletManifestData::serialize_with_embedded_version(
			&envelope,
			rivet_data::versioned::DepotAlarmTabletManifestData::LATEST,
		);
	write_bytes(store, write, bytes.map(Into::into)).await
}
async fn write_bytes(
	store: &dyn ObjectStore,
	write: ManifestWrite,
	bytes: anyhow::Result<bytes::Bytes>,
) -> WriteResult {
	let bytes = match bytes {
		Ok(bytes) if bytes.len() <= MAX_MANIFEST_BYTES => bytes,
		Ok(_) => return WriteResult::Invalid("manifest exceeds byte limit".into()),
		Err(error) => return WriteResult::Invalid(error.to_string()),
	};
	let mode = match write.expected_etag {
		Some(etag) => PutMode::Update(UpdateVersion {
			e_tag: Some(etag),
			version: None,
		}),
		None => PutMode::Create,
	};
	match store
		.put_opts(&ManifestKey::new().pack(), bytes.into(), mode.into())
		.await
	{
		Ok(result) => match result.e_tag.filter(|etag| !etag.is_empty()) {
			Some(etag) => WriteResult::Confirmed { etag },
			None => WriteResult::Unknown("manifest write returned no ETag".into()),
		},
		Err(
			object_store::Error::Precondition { .. } | object_store::Error::AlreadyExists { .. },
		) => WriteResult::Precondition,
		Err(error) => WriteResult::Unknown(error.to_string()),
	}
}


