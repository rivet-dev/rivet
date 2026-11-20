//! Writes the backup of one V2 actor's export to the `migrate_from_v2.backup` store while the
//! import copies it, so the actor can be restored after its V2 data is deleted.
//!
//! The backup keeps V2's export stream verbatim, split into one segment per V2 fence: a segment
//! holds the length-prefixed frames since the previous fence, ending with the fence frame, and is
//! uploaded when that fence arrives. A broken stream resumes from its last fence, which is where
//! the last uploaded segment ends, so the segments of a resumed export still join into one stream.
//! The header frame is kept only once, in the first segment, and the end frame closes the last
//! one. The manifest is written after every segment; see `pegboard.v2_migration_backup.v1.bare`
//! for the layout.

use std::sync::Arc;

use anyhow::{Context, Result};
use migrate_from_v2_protocol as proto;
use object_store::{ObjectStore, ObjectStoreExt, PutPayload, path::Path};
use rivet_data::generated::pegboard_v2_migration_backup_v1 as wire;
use rivet_data::versioned::BackupManifest;
use rivet_util::Id;
use vbare::Versioned;

/// The backup being written for one copy attempt.
pub struct Backup {
	store: Arc<dyn ObjectStore>,
	v2_namespace: String,
	v2_actor_id: Id,
	v3_actor_id: Id,
	generation: u32,
	sleep_ts: i64,
	/// The frames since the last uploaded segment, each with its length prefix.
	pending: Vec<u8>,
	/// Segments uploaded so far, which is the index of the next one.
	segments: u32,
}

impl Backup {
	/// A backup of the export at the V2 fence `generation` and `sleep_ts`, in the directory of
	/// [`directory`]. A copy restarted from scratch writes the same directory again; only its
	/// manifest names how many segments count.
	pub fn new(
		store: Arc<dyn ObjectStore>,
		v2_namespace: &str,
		v2_actor_id: Id,
		v3_actor_id: Id,
		generation: u32,
		sleep_ts: i64,
	) -> Self {
		Self {
			store,
			v2_namespace: v2_namespace.to_owned(),
			v2_actor_id,
			v3_actor_id,
			generation,
			sleep_ts,
			pending: Vec::new(),
			segments: 0,
		}
	}

	/// The segments uploaded so far. A resume passes it back to [`Backup::rewind`].
	pub fn segments(&self) -> u32 {
		self.segments
	}

	/// Drops the frames received since the last uploaded segment, for a stream that broke and
	/// resumes from the fence that segment ends with.
	pub fn rewind(&mut self, segments: u32) {
		self.pending.clear();
		self.segments = segments;
	}

	/// Keeps one frame, given its BARE body, with the same length prefix V2 sent it with.
	pub fn push_frame(&mut self, frame_body: &[u8]) -> Result<()> {
		let len = u32::try_from(frame_body.len()).context("export frame too large to back up")?;
		self.pending.extend_from_slice(&len.to_le_bytes());
		self.pending.extend_from_slice(frame_body);
		Ok(())
	}

	/// Uploads the frames kept since the last segment as the next segment.
	pub async fn write_segment(&mut self) -> Result<()> {
		let path = self.path(&segment_name(self.segments));
		let bytes = std::mem::take(&mut self.pending);
		self.store
			.put(&path, PutPayload::from(bytes))
			.await
			.with_context(|| format!("write backup segment {path}"))?;
		self.segments += 1;
		Ok(())
	}

	/// Writes the manifest of a finished export, after its last segment.
	pub async fn write_manifest(&self, end: &proto::ExportEnd, written_at: i64) -> Result<()> {
		let manifest = wire::BackupManifest {
			v2_namespace: self.v2_namespace.clone().into(),
			v2_actor_id: self.v2_actor_id.as_bytes().into(),
			v3_actor_id: self.v3_actor_id.as_bytes().into(),
			generation: self.generation,
			sleep_ts: self.sleep_ts,
			written_at,
			segments: self.segments,
			kv_entries: end.kv_entries,
			kv_bytes: end.kv_bytes,
			pages: end.pages,
			digest: end.digest.to_vec().into(),
		};
		let bytes =
			BackupManifest::serialize_with_embedded_version(&manifest, BackupManifest::LATEST)?;
		let path = self.path(MANIFEST_NAME);
		self.store
			.put(&path, PutPayload::from(bytes))
			.await
			.with_context(|| format!("write backup manifest {path}"))?;
		Ok(())
	}

	fn path(&self, name: &str) -> Path {
		Path::from(format!(
			"{}/{name}",
			directory(&self.v2_namespace, self.v2_actor_id, self.generation)
		))
	}
}

/// The directory of the backup of `v2_actor_id`'s export at `generation`. A sleeping actor's data
/// cannot change and a wake moves its generation, so the generation names one export.
pub fn directory(v2_namespace: &str, v2_actor_id: Id, generation: u32) -> String {
	format!("{v2_namespace}/{v2_actor_id}/{generation}")
}

/// Whether the backup of `v2_actor_id`'s export at `generation` is complete: its manifest, which
/// is written after every segment, exists.
pub async fn is_complete(
	store: &dyn ObjectStore,
	v2_namespace: &str,
	v2_actor_id: Id,
	generation: u32,
) -> Result<bool> {
	let path = Path::from(format!(
		"{}/{MANIFEST_NAME}",
		directory(v2_namespace, v2_actor_id, generation)
	));
	match store.head(&path).await {
		Ok(_) => Ok(true),
		Err(object_store::Error::NotFound { .. }) => Ok(false),
		Err(error) => Err(error).with_context(|| format!("read backup manifest {path}")),
	}
}

/// The manifest object's name inside a backup directory.
pub const MANIFEST_NAME: &str = "manifest";

/// The name of segment `index`, zero-padded so the segments list in order.
pub fn segment_name(index: u32) -> String {
	format!("segment-{index:06}")
}
