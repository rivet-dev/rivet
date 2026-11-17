//! `GET /migration/v1/actors/{actor_id}/export`: the actor's whole KV and SQLite as one stream of
//! length-prefixed BARE frames, read without waking the actor.
//!
//! The order is a header, KV in key order, SQLite pages `1..=dbSizePages`, then an end frame with
//! a digest. Each internal transaction reads at most `TX_MAX_BYTES` or for at most `TX_MAX_TIME`
//! and re-reads the fence at its own read version: the generation, the sleep timestamp, the
//! allocation keys, `DestroyTsKey` and the SQLite head. KV and SQLite only change while the actor
//! runs, and running needs an allocation that moves the generation, so an unchanged fence means
//! the chunks join into one consistent copy. Any change ends the stream with `FenceMoved`.
//!
//! An `ExportFence` frame follows every transaction. Its cursor resumes a broken stream from that
//! point, within the same fence.

use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use axum::{
	body::{Body, Bytes},
	http::header::CONTENT_TYPE,
	response::{IntoResponse, Response},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use depot::{
	conveyer::{Db, error::SqliteStorageError},
	types::{DepotReadMode, GetPagesOptions, SQLITE_PAGE_SIZE},
};
use futures_util::TryStreamExt;
use pegboard::keys;
use rivet_envoy_protocol as ep;
use rivet_util::Id;
use universaldb::{
	options::StreamingMode,
	utils::{FormalKey, IsolationLevel::Snapshot},
};
use v2_migration_protocol as proto;

use super::{ExportQuery, authorize, ensure_local, ensure_namespace, facts};
use crate::{ctx::ApiCtx, errors};

/// Bytes one internal transaction reads before it ends, well inside UniversalDB's limits.
const TX_MAX_BYTES: usize = 8 * 1024 * 1024;
/// Time one internal transaction reads before it ends, inside UniversalDB's 5 s limit.
const TX_MAX_TIME: Duration = Duration::from_secs(1);
/// Bytes per `KvBatch` frame.
const BATCH_MAX_BYTES: usize = 1024 * 1024;
/// Pages per `PageBatch` frame (1 MiB of 4 KiB pages).
const BATCH_MAX_PAGES: u32 = 256;
/// Pages read per depot call, one fenced chunk.
const PAGES_PER_READ: u32 = (TX_MAX_BYTES / SQLITE_PAGE_SIZE as usize) as u32;

/// The fence an export holds for its whole lifetime, including resumes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Fence {
	generation: u32,
	sleep_ts: i64,
	head_txid: Option<u64>,
}

enum FenceCheck {
	Held,
	Moved,
	Destroyed,
}

pub async fn export(ctx: ApiCtx, actor_id: Id, query: ExportQuery) -> Result<Response> {
	ensure_local(&ctx, actor_id)?;
	let namespace_id = authorize(&ctx, &query.namespace).await?;

	let resume = match &query.resume {
		Some(resume) => {
			let raw = URL_SAFE_NO_PAD
				.decode(resume)
				.map_err(|_| invalid("resume is not base64"))?;
			Some(
				proto::decode::<proto::ExportCursor>(&raw)
					.map_err(|_| invalid("resume is not a cursor"))?,
			)
		}
		None => None,
	};

	// Read the facts, the SQLite head and the KV size estimate at one read version.
	let hibernating_since_ts = facts::hibernating_since_ts(
		ctx.config()
			.pegboard()
			.hibernating_request_eligible_threshold(),
	);
	let udb = std::sync::Arc::new((*ctx.pools().udb()?).clone());
	let (actor, head, sqlite_v1, kv_bytes_estimate) = udb
		.txn("migration_export_header", |tx| async move {
			let actor = facts::read(&tx, actor_id, hibernating_since_ts, Snapshot).await?;
			ensure_namespace(actor.as_ref(), namespace_id)?;
			let actor = actor.context("actor checked above")?;

			let (head, sqlite_v1, kv_bytes_estimate) = tokio::try_join!(
				facts::read_sqlite_head(&tx, namespace_id, actor_id, Snapshot),
				facts::sqlite_v1(&tx, namespace_id, actor_id, Snapshot),
				pegboard::actor_kv::estimate_kv_size(&tx, actor_id),
			)?;

			Ok((actor, head, sqlite_v1, kv_bytes_estimate))
		})
		.await?;

	let abort = if actor.destroy_ts.is_some() {
		Some(proto::AbortReason::Destroyed)
	} else if actor.version == 1 || sqlite_v1 {
		Some(proto::AbortReason::Ineligible)
	} else if actor.sleep_ts.is_none() || actor.allocated {
		Some(proto::AbortReason::FenceMoved)
	} else {
		None
	};
	if let Some(reason) = abort {
		return Ok(stream_response(Body::from(proto::encode_frame(
			&proto::Frame::ExportAbort(proto::ExportAbort { reason }),
		)?)));
	}

	let fence = Fence {
		generation: actor.generation,
		sleep_ts: actor.sleep_ts.context("checked sleeping above")?,
		head_txid: head.as_ref().map(|head| head.head_txid),
	};

	let cursor = match resume {
		Some(cursor) => {
			let resumed = Fence {
				generation: cursor.generation,
				sleep_ts: cursor.sleep_ts,
				head_txid: cursor.head_txid,
			};
			if resumed != fence {
				return Ok(stream_response(Body::from(proto::encode_frame(
					&proto::Frame::ExportAbort(proto::ExportAbort {
						reason: proto::AbortReason::FenceMoved,
					}),
				)?)));
			}
			cursor
		}
		None => proto::ExportCursor {
			generation: fence.generation,
			sleep_ts: fence.sleep_ts,
			head_txid: fence.head_txid,
			after_kv_key: None,
			kv_done: false,
			next_pgno: 1,
			digest: vec![0; 16],
			kv_entries: 0,
			kv_bytes: 0,
			pages: 0,
		},
	};

	let header = proto::ExportHeader {
		generation: fence.generation,
		sleep_ts: fence.sleep_ts,
		wake_deadline_ts: actor.wake_deadline_ts,
		name: actor.name.clone(),
		key: actor.key.clone(),
		pool_name: actor.pool_name.clone(),
		create_ts: actor.create_ts,
		kv_bytes_estimate: u64::try_from(kv_bytes_estimate).unwrap_or_default(),
		sqlite: head.as_ref().map(|head| proto::SqliteHead {
			head_txid: head.head_txid,
			db_size_pages: head.db_size_pages,
			page_size: SQLITE_PAGE_SIZE,
		}),
	};

	let depot_db = Db::new(
		udb.clone(),
		namespace_id,
		actor_id.to_string(),
		rivet_pools::NodeId::new(),
	)
	.with_config(ctx.config().clone());

	let exporter = Exporter {
		udb,
		depot_db,
		actor_id,
		namespace_id,
		fence,
		hibernating_since_ts,
		db_size_pages: head.map_or(0, |head| head.db_size_pages),
		cursor,
	};

	let stream = async_stream::stream! {
		yield proto::encode_frame(&proto::Frame::ExportHeader(header)).map(Bytes::from);

		let mut exporter = exporter;
		loop {
			match exporter.next_chunk().await {
				Ok(Chunk::Frames(frames)) => {
					for frame in frames {
						yield Ok(Bytes::from(frame));
					}
				}
				Ok(Chunk::Last(frame)) => {
					yield Ok(Bytes::from(frame));
					break;
				}
				Err(err) => {
					tracing::warn!(?err, actor_id = %exporter.actor_id, "migration export failed mid-stream; the migrator resumes from the last fence");
					yield Err(err);
					break;
				}
			}
		}
	};

	Ok(stream_response(Body::from_stream(
		stream.map_err(|err| std::io::Error::other(format!("{err:#}"))),
	)))
}

fn stream_response(body: Body) -> Response {
	([(CONTENT_TYPE, proto::EXPORT_CONTENT_TYPE)], body).into_response()
}

fn invalid(message: &str) -> anyhow::Error {
	errors::Migration::InvalidRequest {
		message: message.to_string(),
	}
	.build()
}

enum Chunk {
	/// Data frames followed by an `ExportFence`.
	Frames(Vec<Vec<u8>>),
	/// The final `ExportEnd` or `ExportAbort` frame.
	Last(Vec<u8>),
}

struct Exporter {
	udb: std::sync::Arc<universaldb::Database>,
	depot_db: Db,
	actor_id: Id,
	namespace_id: Id,
	fence: Fence,
	/// Fixed at the request's start, so every fence check agrees on which requests are live.
	hibernating_since_ts: i64,
	db_size_pages: u32,
	cursor: proto::ExportCursor,
}

impl Exporter {
	/// Reads one fenced chunk and returns its encoded frames.
	async fn next_chunk(&mut self) -> Result<Chunk> {
		if !self.cursor.kv_done {
			return self.next_kv_chunk().await;
		}

		if self.cursor.next_pgno <= self.db_size_pages {
			return self.next_page_chunk().await;
		}

		let end = proto::Frame::ExportEnd(proto::ExportEnd {
			kv_entries: self.cursor.kv_entries,
			kv_bytes: self.cursor.kv_bytes,
			pages: self.cursor.pages,
			digest: self.cursor.digest.clone(),
		});
		Ok(Chunk::Last(proto::encode_frame(&end)?))
	}

	async fn next_kv_chunk(&mut self) -> Result<Chunk> {
		let actor_id = self.actor_id;
		let namespace_id = self.namespace_id;
		let fence = self.fence;
		let hibernating_since_ts = self.hibernating_since_ts;
		let after_key = self.cursor.after_kv_key.clone();

		let (check, entries, done) = self
			.udb
			.txn("migration_export_kv", |tx| {
				let after_key = after_key.clone();
				async move {
					let check =
						check_fence(&tx, actor_id, namespace_id, fence, hibernating_since_ts)
							.await?;
					if !matches!(check, FenceCheck::Held) {
						return Ok((check, Vec::new(), false));
					}

					let (entries, done) = read_kv_chunk(&tx, actor_id, after_key).await?;
					Ok((FenceCheck::Held, entries, done))
				}
			})
			.await?;

		if let Some(last) = abort_for(check)? {
			return Ok(Chunk::Last(last));
		}

		let mut frames = Vec::new();
		let mut batch = Vec::new();
		let mut batch_bytes = 0;
		for entry in entries {
			let entry_bytes = entry.key.len() + entry.value.len();
			if !batch.is_empty() && batch_bytes + entry_bytes > BATCH_MAX_BYTES {
				frames.push(self.data_frame(proto::Frame::KvBatch(proto::KvBatch {
					entries: std::mem::take(&mut batch),
				}))?);
				batch_bytes = 0;
			}

			self.cursor.kv_entries += 1;
			self.cursor.kv_bytes += entry_bytes as u64;
			self.cursor.after_kv_key = Some(entry.key.clone());
			batch_bytes += entry_bytes;
			batch.push(entry);
		}
		if !batch.is_empty() {
			frames.push(self.data_frame(proto::Frame::KvBatch(proto::KvBatch { entries: batch }))?);
		}

		self.cursor.kv_done = done;
		frames.push(self.fence_frame()?);
		Ok(Chunk::Frames(frames))
	}

	async fn next_page_chunk(&mut self) -> Result<Chunk> {
		let first = self.cursor.next_pgno;
		let last = first
			.saturating_add(PAGES_PER_READ - 1)
			.min(self.db_size_pages);
		let pgnos = (first..=last).collect::<Vec<_>>();

		// The head fence makes depot refuse the read if the database moved.
		let read = self
			.depot_db
			.get_pages_with_options(
				pgnos,
				GetPagesOptions {
					expected_head_txid: self.fence.head_txid,
					mode: DepotReadMode::DiagnosticNoSideEffects,
					..Default::default()
				},
			)
			.await;
		let mut pages = match read {
			Ok(result) => result.pages,
			Err(err) if is_head_fence_mismatch(&err) => {
				return Ok(Chunk::Last(proto::encode_frame(
					&proto::Frame::ExportAbort(proto::ExportAbort {
						reason: proto::AbortReason::FenceMoved,
					}),
				)?));
			}
			Err(err) => return Err(err.context("read sqlite pages for migration export")),
		};
		pages.sort_by_key(|page| page.pgno);

		// Re-check the pegboard fence after the read, so a wake that raced it ends the stream.
		let actor_id = self.actor_id;
		let namespace_id = self.namespace_id;
		let fence = self.fence;
		let hibernating_since_ts = self.hibernating_since_ts;
		let check = self
			.udb
			.txn("migration_export_pages_fence", |tx| async move {
				check_fence(&tx, actor_id, namespace_id, fence, hibernating_since_ts).await
			})
			.await?;
		if let Some(last) = abort_for(check)? {
			return Ok(Chunk::Last(last));
		}

		let mut by_pgno = pages
			.into_iter()
			.map(|page| (page.pgno, page.bytes))
			.collect::<std::collections::BTreeMap<_, _>>();

		let mut frames = Vec::new();
		let mut batch_first = first;
		while batch_first <= last {
			let batch_last = batch_first.saturating_add(BATCH_MAX_PAGES - 1).min(last);
			let batch_pages = (batch_first..=batch_last)
				.map(|pgno| {
					by_pgno
						.remove(&pgno)
						.flatten()
						.unwrap_or_else(|| vec![0; SQLITE_PAGE_SIZE as usize])
				})
				.collect::<Vec<_>>();

			self.cursor.pages += batch_last - batch_first + 1;
			frames.push(self.data_frame(proto::Frame::PageBatch(proto::PageBatch {
				first_pgno: batch_first,
				pages: batch_pages,
			}))?);
			batch_first = batch_last + 1;
		}

		self.cursor.next_pgno = last + 1;
		frames.push(self.fence_frame()?);
		Ok(Chunk::Frames(frames))
	}

	/// Encodes a data frame and chains it into the digest.
	fn data_frame(&mut self, frame: proto::Frame) -> Result<Vec<u8>> {
		let encoded = proto::encode_frame(&frame)?;
		self.cursor.digest = chain_digest(&self.cursor.digest, &encoded[4..]);
		Ok(encoded)
	}

	fn fence_frame(&self) -> Result<Vec<u8>> {
		proto::encode_frame(&proto::Frame::ExportFence(proto::ExportFence {
			cursor: proto::encode(&self.cursor)?,
		}))
	}
}

/// Chains one frame body into the running digest: `xxh3_128(previous || frame)`.
pub fn chain_digest(previous: &[u8], frame_body: &[u8]) -> Vec<u8> {
	let mut hasher = xxhash_rust::xxh3::Xxh3::new();
	hasher.update(previous);
	hasher.update(frame_body);
	hasher.digest128().to_be_bytes().to_vec()
}

fn abort_for(check: FenceCheck) -> Result<Option<Vec<u8>>> {
	let reason = match check {
		FenceCheck::Held => return Ok(None),
		FenceCheck::Moved => proto::AbortReason::FenceMoved,
		FenceCheck::Destroyed => proto::AbortReason::Destroyed,
	};

	Ok(Some(proto::encode_frame(&proto::Frame::ExportAbort(
		proto::ExportAbort { reason },
	))?))
}

fn is_head_fence_mismatch(err: &anyhow::Error) -> bool {
	err.chain().any(|source| {
		matches!(
			source.downcast_ref::<SqliteStorageError>(),
			Some(SqliteStorageError::HeadFenceMismatch { .. })
		)
	})
}

async fn check_fence(
	tx: &universaldb::Transaction,
	actor_id: Id,
	namespace_id: Id,
	fence: Fence,
	hibernating_since_ts: i64,
) -> Result<FenceCheck> {
	let (actor, head) = tokio::try_join!(
		facts::read(tx, actor_id, hibernating_since_ts, Snapshot),
		facts::read_sqlite_head(tx, namespace_id, actor_id, Snapshot),
	)?;

	let Some(actor) = actor else {
		return Ok(FenceCheck::Destroyed);
	};
	if actor.destroy_ts.is_some() {
		return Ok(FenceCheck::Destroyed);
	}

	let held = actor.generation == fence.generation
		&& actor.sleep_ts == Some(fence.sleep_ts)
		&& !actor.allocated
		&& head.map(|head| head.head_txid) == fence.head_txid;

	Ok(if held {
		FenceCheck::Held
	} else {
		FenceCheck::Moved
	})
}

/// Reads whole KV entries after `after_key`, in key order, until the chunk budget is spent.
///
/// Returns the entries and whether the end of the actor's KV was reached. An entry is never split
/// across chunks.
async fn read_kv_chunk(
	tx: &universaldb::Transaction,
	actor_id: Id,
	after_key: Option<Vec<u8>>,
) -> Result<(Vec<proto::KvEntry>, bool)> {
	let subspace = keys::actor_kv::subspace(actor_id);
	let (range_start, range_end) = subspace.range();

	// Every sub-key of an entry starts with its packed key wrapper, so the byte after it is past
	// all of them.
	let begin = match after_key {
		Some(after_key) => {
			let mut begin = subspace.pack(&keys::actor_kv::KeyWrapper(after_key));
			begin.push(0xFF);
			begin
		}
		None => range_start,
	};

	let kv_tx = tx.with_subspace(subspace);
	let mut stream = kv_tx.get_ranges_keyvalues(
		universaldb::RangeOption {
			mode: StreamingMode::Iterator,
			..(begin, range_end).into()
		},
		Snapshot,
	);

	let started_at = Instant::now();
	let mut entries = Vec::new();
	let mut read_bytes = 0;
	let mut current: Option<EntryBuilder> = None;

	loop {
		let Some(row) = stream.try_next().await? else {
			if let Some(builder) = current.take() {
				entries.push(builder.build()?);
			}
			return Ok((entries, true));
		};

		let key = kv_tx.unpack::<keys::actor_kv::EntryBaseKey>(row.key())?.key;

		// A new entry starts. Stop at this boundary once the chunk's budget is spent.
		if current.as_ref().is_none_or(|builder| builder.key != key.0) {
			if let Some(builder) = current.take() {
				entries.push(builder.build()?);

				if read_bytes >= TX_MAX_BYTES || started_at.elapsed() >= TX_MAX_TIME {
					return Ok((entries, false));
				}
			}
			current = Some(EntryBuilder::new(key.0.clone()));
		}

		read_bytes += row.key().len() + row.value().len();
		let builder = current.as_mut().context("entry builder set above")?;

		if let Ok(chunk_key) = kv_tx.unpack::<keys::actor_kv::EntryValueChunkKey>(row.key()) {
			builder.append_chunk(chunk_key.chunk, row.value());
		} else if let Ok(metadata_key) = kv_tx.unpack::<keys::actor_kv::EntryMetadataKey>(row.key())
		{
			builder.metadata = Some(metadata_key.deserialize(row.value())?);
		} else {
			anyhow::bail!("unexpected actor kv sub key");
		}
	}
}

/// Reassembles one KV entry from its value chunks and metadata row, as `actor_kv::get` does.
struct EntryBuilder {
	key: Vec<u8>,
	value: Vec<u8>,
	next_chunk: usize,
	metadata: Option<ep::KvMetadata>,
}

impl EntryBuilder {
	fn new(key: Vec<u8>) -> Self {
		EntryBuilder {
			key,
			value: Vec::new(),
			next_chunk: 0,
			metadata: None,
		}
	}

	fn append_chunk(&mut self, chunk: usize, bytes: &[u8]) {
		if chunk >= self.next_chunk {
			self.value.extend_from_slice(bytes);
			self.next_chunk = chunk + 1;
		}
	}

	fn build(self) -> Result<proto::KvEntry> {
		let metadata = self
			.metadata
			.context("actor kv entry has no metadata row")?;

		Ok(proto::KvEntry {
			key: self.key,
			value: self.value,
			version: metadata.version,
			update_ts: metadata.update_ts,
		})
	}
}
