//! Streams one V2 actor's export into its V3 database volume.
//!
//! KV entries go to the `/actor/kv` inode with V2's version and update time kept verbatim, so V3
//! reads return exactly what V2 returned. SQLite pages go to the database under `/actor` through
//! the same page protocol the actor uses. The import holds the volume as its exclusive owner, which fences any
//! other opening; the actor cannot run in V3 meanwhile because it is held as `MigratingFromV2`.

use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result, bail, ensure};
use bytes::Bytes;
use depot_client::filesystem::Filesystem;
use futures_util::StreamExt;
use migrate_from_v2_protocol as proto;
use pegboard::keys::partition::ActorDatabaseVolume;
use pegboard_envoy::keys::kv::ActorKvKey;
use rivet_engine_context::EngineContext;
use rivet_env::Clock;
use rivet_envoy_protocol as ep;
use rivet_keys::keys::{KeyAddress, codec::GroupId};
use rivet_topology::versions::{VersionFamily, WriteVersions};
use rivet_util::Id;
use sqlite_embedded_filesystem::page_protocol::database::{self, Database};
use vbare::Versioned;

use crate::v2_client::V2Client;

/// Stream breaks the import resumes from the last fence before it gives up.
const MAX_RESUMES: usize = 3;
/// Refused writes the import resumes from the last fence after, apart from broken streams. Each
/// waits the refusal's hint first.
const MAX_REFUSAL_RESUMES: usize = 30;

/// How long V2 may take to answer an export request with its response headers.
const EXPORT_RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);
/// How long the export stream may stay silent. V2 sends a fence after every transaction, each
/// bounded to about a second, so a stream silent this long is dead and is resumed from the last
/// fence like a broken one.
const EXPORT_IDLE_TIMEOUT: Duration = Duration::from_secs(60);

const KV_NAME: &[u8] = b"kv";
/// How long one import write may keep being refused by the storage host before the copy gives up
/// on it.
const REFUSED_WRITE_BUDGET: Duration = Duration::from_secs(120);

/// How an export ended.
pub enum CopyOutcome {
	Copied(Copied),
	/// V2 ended the stream: the actor woke, was destroyed, or is not eligible.
	Aborted(proto::AbortReason),
	/// The export header no longer matches what the listing selected.
	FenceMoved,
}

pub struct Copied {
	pub kv_entries: u64,
	pub kv_bytes: u64,
	pub pages: u32,
}

/// Copies the export of `v2_actor_id` into the database volume of the held incarnation
/// `create_unix_ms` of `v3_actor_id`, at `group`, the placement the hold was begun at.
///
/// The volume is deleted first, so a copy restarted after a failover or a takeover of the hold
/// never mixes with an earlier one.
#[allow(
	clippy::too_many_arguments,
	reason = "one argument per part of the copy's identity and V2 fence"
)]
pub async fn copy(
	ctx: &EngineContext,
	v2: &V2Client,
	group: Option<GroupId>,
	v2_actor_id: Id,
	v3_actor_id: Id,
	create_unix_ms: i64,
	generation: u32,
	sleep_ts: i64,
) -> Result<CopyOutcome> {
	let key = ActorDatabaseVolume::in_namespace(group, v3_actor_id, create_unix_ms);
	let clock = ctx.env().clock();
	let depot = ctx.depot();
	resend_refused(clock.as_ref(), || depot.delete_current(key.clone()))
		.await
		.context("clear the V3 database volume before the import")?;
	let opened = resend_refused(clock.as_ref(), || {
		depot.open_or_create_filesystem(key.clone())
	})
	.await?;
	let fs = opened.filesystem().clone();

	let result = import(
		v2,
		&fs,
		ctx.env().clock(),
		ctx.topology(),
		v2_actor_id,
		generation,
		sleep_ts,
	)
	.await;
	let closed = opened.shutdown().await;
	let outcome = result?;
	closed.context("close the V3 database volume after the import")?;
	Ok(outcome)
}

struct Import {
	fs: Arc<Filesystem>,
	clock: Arc<dyn Clock>,
	/// Chooses the `pegboard.actor` version for imported KV values, as for every other KV writer.
	versions: Arc<dyn WriteVersions>,
	kv_ino: Option<u64>,
	database: Option<Database>,
	db_size_pages: u32,
	digest: [u8; 16],
	kv_entries: u64,
	kv_bytes: u64,
	pages: u32,
	/// The last fence's cursor and the digest and counts as of it, where a resume restarts.
	resume: Option<(Vec<u8>, [u8; 16], u64, u64, u32)>,
}

async fn import(
	v2: &V2Client,
	fs: &Arc<Filesystem>,
	clock: Arc<dyn Clock>,
	versions: Arc<dyn WriteVersions>,
	v2_actor_id: Id,
	generation: u32,
	sleep_ts: i64,
) -> Result<CopyOutcome> {
	let mut state = Import {
		fs: fs.clone(),
		clock,
		versions,
		kv_ino: None,
		database: None,
		db_size_pages: 0,
		digest: [0; 16],
		kv_entries: 0,
		kv_bytes: 0,
		pages: 0,
		resume: None,
	};

	let mut resumes = 0;
	let mut refusals = 0;
	loop {
		let resume_cursor = state.resume.as_ref().map(|(cursor, ..)| cursor.clone());
		match stream_once(
			v2,
			&mut state,
			v2_actor_id,
			generation,
			sleep_ts,
			resume_cursor,
		)
		.await
		{
			Ok(outcome) => return Ok(outcome),
			// A write the storage host refused did not run. Waiting its hint and resuming from the
			// last fence costs part of the copy instead of all of it, and does not count as a
			// broken stream.
			Err(error)
				if refusals < MAX_REFUSAL_RESUMES
					&& state.resume.is_some()
					&& depot_client::retry_after_ms(&error).is_some() =>
			{
				refusals += 1;
				let retry_after_ms = depot_client::retry_after_ms(&error).unwrap_or_default();
				tracing::debug!(?error, %v2_actor_id, refusals, "an import write was refused; resuming from the last fence");
				rivet_env::tokio::time::sleep(
					state.clock.as_ref(),
					Duration::from_millis(retry_after_ms),
				)
				.await?;
				if let Some((_, digest, kv_entries, kv_bytes, pages)) = &state.resume {
					state.digest = *digest;
					state.kv_entries = *kv_entries;
					state.kv_bytes = *kv_bytes;
					state.pages = *pages;
				}
			}
			Err(error) if resumes < MAX_RESUMES && state.resume.is_some() => {
				resumes += 1;
				tracing::warn!(?error, %v2_actor_id, resumes, "V2 export stream broke; resuming from the last fence");
				if let Some((_, digest, kv_entries, kv_bytes, pages)) = &state.resume {
					state.digest = *digest;
					state.kv_entries = *kv_entries;
					state.kv_bytes = *kv_bytes;
					state.pages = *pages;
				}
			}
			Err(error) => return Err(error),
		}
	}
}

async fn stream_once(
	v2: &V2Client,
	state: &mut Import,
	v2_actor_id: Id,
	generation: u32,
	sleep_ts: i64,
	resume: Option<Vec<u8>>,
) -> Result<CopyOutcome> {
	let mut body = rivet_env::tokio::time::timeout(
		state.clock.as_ref(),
		EXPORT_RESPONSE_TIMEOUT,
		v2.export(v2_actor_id, resume.as_deref()),
	)
	.await
	.context("V2 did not answer the export request")??;
	let mut buffer = Vec::<u8>::new();

	loop {
		while let Some((frame_body, used)) = proto::split_frame(&buffer)? {
			let frame_body = frame_body.to_vec();
			buffer.drain(..used);

			match proto::decode_frame(&frame_body)? {
				proto::Frame::ExportHeader(header) => {
					if header.generation != generation || header.sleep_ts != sleep_ts {
						return Ok(CopyOutcome::FenceMoved);
					}
					state.db_size_pages =
						header.sqlite.as_ref().map_or(0, |head| head.db_size_pages);
					if let Some(head) = &header.sqlite {
						ensure!(
							head.page_size as usize == crate::SQLITE_PAGE_SIZE,
							"V2 page size {} differs from V3's {}",
							head.page_size,
							crate::SQLITE_PAGE_SIZE
						);
					}
				}
				proto::Frame::KvBatch(batch) => {
					state.digest = proto::chain_digest(&state.digest, &frame_body);
					write_kv(state, batch).await?;
				}
				proto::Frame::PageBatch(batch) => {
					state.digest = proto::chain_digest(&state.digest, &frame_body);
					write_pages(state, batch).await?;
				}
				proto::Frame::ExportFence(fence) => {
					state.resume = Some((
						fence.cursor.to_vec(),
						state.digest,
						state.kv_entries,
						state.kv_bytes,
						state.pages,
					));
				}
				proto::Frame::ExportEnd(end) => {
					ensure!(
						end.digest[..] == state.digest[..],
						"V2 export digest does not match the imported frames"
					);
					ensure!(
						end.kv_entries == state.kv_entries && end.pages == state.pages,
						"V2 export counts do not match the imported frames"
					);
					return Ok(CopyOutcome::Copied(Copied {
						kv_entries: state.kv_entries,
						kv_bytes: state.kv_bytes,
						pages: state.pages,
					}));
				}
				proto::Frame::ExportAbort(abort) => return Ok(CopyOutcome::Aborted(abort.reason)),
			}
		}

		let next =
			rivet_env::tokio::time::timeout(state.clock.as_ref(), EXPORT_IDLE_TIMEOUT, body.next())
				.await
				.context("the V2 export stream went silent")?;
		match next {
			Some(chunk) => buffer.extend_from_slice(&chunk?),
			None => bail!("V2 export stream ended before its last frame"),
		}
	}
}

async fn kv_ino(state: &mut Import) -> Result<u64> {
	if let Some(ino) = state.kv_ino {
		return Ok(ino);
	}
	// The actor's KV lives under the volume's `/actor` directory, beside its database, as
	// pegboard-envoy lays it out.
	let started_ms = state.clock.observe()?.boot_ms;
	let actor = loop {
		match database::actor_directory(&state.fs).await {
			Ok(actor) => break actor,
			Err(error) => wait_out_refusal(state.clock.as_ref(), started_ms, error).await?,
		}
	};
	let ino = match state.fs.lookup(actor, KV_NAME).await {
		Ok(entry) => {
			ensure!(
				entry.kind == depot_fs_core::FsKind::Kv,
				"actor KV inode has the wrong kind"
			);
			entry.child
		}
		Err(depot_client::filesystem::Error::Filesystem(depot_fs_core::Error::NotFound)) => loop {
			let created = state
				.fs
				.create(
					actor,
					KV_NAME,
					depot_fs_core::rules::new_inode(depot_fs_core::FsKind::Kv, 0o600, 0, 0, 0),
				)
				.await;
			match created {
				Ok(ino) => break ino,
				Err(error) => {
					wait_out_refusal(state.clock.as_ref(), started_ms, error.into()).await?
				}
			}
		},
		Err(error) => return Err(error.into()),
	};
	state.kv_ino = Some(ino);
	Ok(ino)
}

/// Writes one batch in as few transactions as the volume's transaction limit allows.
async fn write_kv(state: &mut Import, batch: proto::KvBatch) -> Result<()> {
	let ino = kv_ino(state).await?;
	let max_bytes = state.fs.max_transaction_bytes();

	let version = kv_write_version(state.versions.as_ref())?;
	let mut puts = Vec::new();
	let mut tx_bytes = 0;
	for entry in batch.entries {
		let key = ActorKvKey::new(Bytes::copy_from_slice(&entry.key)).pack();
		let value = rivet_data::versioned::KvValue::serialize_with_embedded_version(
			&pegboard::wire::KvValue {
				value: entry.value.clone(),
				version: entry.version.clone(),
				updated_unix_ms: entry.update_ts,
			},
			version,
		)?;

		if tx_bytes > 0 && tx_bytes + key.len() + value.len() > max_bytes {
			commit_kv(state, ino, &puts).await?;
			puts.clear();
			tx_bytes = 0;
		}
		tx_bytes += key.len() + value.len();
		puts.push((key, value));

		state.kv_entries += 1;
		state.kv_bytes += (entry.key.len() + entry.value.len()) as u64;
	}
	if !puts.is_empty() {
		commit_kv(state, ino, &puts).await?;
	}
	Ok(())
}

/// The `pegboard.actor` version to write imported KV values at: the negotiated one every live
/// storage worker reads, never the compiled latest, because readers on older binaries share the
/// volume during a rolling deploy. Version 4 is written under the version 3 header by the
/// envelope's seal; see `rivet_data::versioned::legacy_v3`.
fn kv_write_version(versions: &dyn WriteVersions) -> Result<u16> {
	Ok(rivet_data::pegboard_actor::record_write_version(
		versions.write_version(VersionFamily::PEGBOARD_ACTOR)?,
	))
}

/// Commits one transaction of KV puts, sending it again while the storage host refuses it.
async fn commit_kv(state: &Import, ino: u64, puts: &[(Bytes, Vec<u8>)]) -> Result<()> {
	let started_ms = state.clock.observe()?.boot_ms;
	loop {
		let mut tx = state.fs.begin(ino).await?;
		for (key, value) in puts {
			tx.put(key, value)?;
		}
		match tx.commit().await {
			Ok(_) => return Ok(()),
			Err(error) => wait_out_refusal(state.clock.as_ref(), started_ms, error.into()).await?,
		}
	}
}

/// Sends `attempt` again after each refusal's hint until it is admitted, within
/// [`REFUSED_WRITE_BUDGET`]. For a step outside the export stream, such as holding the V3 actor or
/// opening its database volume, where there is no fence to resume from.
pub(crate) async fn resend_refused<T, E, F, Fut>(clock: &dyn Clock, mut attempt: F) -> Result<T>
where
	E: Into<anyhow::Error>,
	F: FnMut() -> Fut,
	Fut: Future<Output = std::result::Result<T, E>>,
{
	let started_ms = clock.observe()?.boot_ms;
	loop {
		match attempt().await {
			Ok(value) => return Ok(value),
			Err(error) => wait_out_refusal(clock, started_ms, error.into()).await?,
		}
	}
}

/// Waits out the hint of a write the storage host refused, so the caller sends the same write
/// again. A refusal means the write did not run, which makes resending it safe; shedding is
/// transient under load, and discarding a partly imported actor would redo the whole copy. Any
/// other error, or refusals past [`REFUSED_WRITE_BUDGET`], is returned and fails the copy.
async fn wait_out_refusal(clock: &dyn Clock, started_ms: u64, error: anyhow::Error) -> Result<()> {
	let Some(retry_after_ms) = depot_client::retry_after_ms(&error) else {
		return Err(error);
	};
	let now_ms = clock.observe()?.boot_ms;
	let budget_ms = REFUSED_WRITE_BUDGET.as_millis() as u64;
	if now_ms.saturating_add(retry_after_ms) > started_ms.saturating_add(budget_ms) {
		return Err(error.context("the storage host kept refusing an import write"));
	}
	tracing::debug!(
		retry_after_ms,
		"the storage host refused an import write; sending it again"
	);
	rivet_env::tokio::time::sleep(clock, Duration::from_millis(retry_after_ms)).await?;
	Ok(())
}

async fn write_pages(state: &mut Import, batch: proto::PageBatch) -> Result<()> {
	if state.database.is_none() {
		let started_ms = state.clock.observe()?.boot_ms;
		let database = loop {
			match Database::from_filesystem(state.fs.clone()).await {
				Ok(database) => break database,
				Err(error) => wait_out_refusal(state.clock.as_ref(), started_ms, error).await?,
			}
		};
		state.database = Some(database);
	}
	let max_pages = (sqlite_types::SHARD_SIZE * sqlite_types::COMMIT_SEGMENT_MAX_SHARDS) as usize;
	let max_pages = max_pages
		.min(state.fs.max_transaction_bytes() / crate::SQLITE_PAGE_SIZE)
		.max(1);
	let size_pages = state.db_size_pages;
	let database = state.database.as_mut().context("database opened above")?;

	let pages = batch
		.pages
		.into_iter()
		.enumerate()
		.map(|(i, bytes)| ep::SqliteDirtyPage {
			pgno: batch.first_pgno + i as u32,
			bytes: bytes.to_vec().into(),
		})
		.collect::<Vec<_>>();
	let count = pages.len() as u32;
	for chunk in pages.chunks(max_pages) {
		let started_ms = state.clock.observe()?.boot_ms;
		loop {
			match database.commit(chunk.to_vec(), size_pages).await {
				Ok(_) => break,
				Err(error) => wait_out_refusal(state.clock.as_ref(), started_ms, error).await?,
			}
		}
	}
	state.pages += count;
	Ok(())
}
