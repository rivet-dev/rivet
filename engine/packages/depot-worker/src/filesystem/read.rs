//! Executes typed read-session operations against the grant-owned filesystem store.

use super::{
	session::{Error, Result},
	store::FilesystemStore,
};
use depot_fs_core::store::{Row, RowKey, RowRange};
use futures_util::{StreamExt, TryStreamExt};
use rivet_data::generated::{shared_depot_fs_read_v1::*, shared_depot_types_v1::VolumeLogPosition};
use uuid::Uuid;

impl FilesystemStore {
	/// Reads `keys` of the KV file `file` under the volume's root at `applied`, the position an
	/// open answers its read-back at. Returns one value per key, `None` where the key or the
	/// file is absent.
	pub async fn read_keys(
		&self,
		key: &depot_types::VolumeLogKey,
		record: depot_fs_core::FsVolumeRecord,
		applied: VolumeLogPosition,
		file: &[u8],
		keys: &[vbare::Bytes],
	) -> Result<Vec<Option<vbare::Bytes>>> {
		// This read is not ordered against the volume's store writes, so the session
		// checks the stored record under the volume's stripe.
		if record.volume_incarnation != key.0.volume_incarnation {
			return Err(Error::NotFound);
		}
		let session = self
			.open_session(key, &record, applied.clone(), Some(applied.volume_seq))
			.await?;
		let _close = SessionClose {
			store: self,
			id: session.id,
		};
		read_kv_values(&session, file, keys).await
	}

	/// The record of `key` a read opens its session against, and whether the core ordered the
	/// read after every store write of the volume. A record from the core is the loaded
	/// volume's current one for exactly that reason; only an unloaded volume needs the stored
	/// record.
	async fn read_record(
		&self,
		key: &depot_types::VolumeLogKey,
		record: Option<depot_fs_core::FsVolumeRecord>,
	) -> Result<(depot_fs_core::FsVolumeRecord, bool)> {
		let ordered = record.is_some();
		let record = match record {
			Some(record) => record,
			None => self
				.incarnation_row(&key.0.volume_key, key.0.volume_incarnation)
				.await?
				.ok_or(Error::NotFound)?,
		};
		if record.volume_incarnation != key.0.volume_incarnation {
			return Err(Error::NotFound);
		}
		Ok((record, ordered))
	}

	/// Registers a session of `key` reading at `as_of`, ordered or against the stored record
	/// as [`Self::read_record`] decided.
	async fn register_read(
		&self,
		key: &depot_types::VolumeLogKey,
		record: &depot_fs_core::FsVolumeRecord,
		ordered: bool,
		applied: VolumeLogPosition,
		as_of: u64,
	) -> Result<std::sync::Arc<super::session::ReadSession>> {
		if ordered {
			self.open_ordered_session(key, record, applied, Some(as_of))
				.await
		} else {
			self.open_session(key, record, applied, Some(as_of)).await
		}
	}

	/// Opens a session at `position`, or at the latest position the reader may use when it is
	/// `None`, and returns it with the position it serves.
	async fn open_pinned(
		&self,
		key: &depot_types::VolumeLogKey,
		client_epoch: Option<u64>,
		position: Option<VolumeLogPosition>,
		applied: Option<VolumeLogPosition>,
		settled: Option<VolumeLogPosition>,
		record: Option<depot_fs_core::FsVolumeRecord>,
	) -> Result<(
		std::sync::Arc<super::session::ReadSession>,
		VolumeLogPosition,
	)> {
		let (record, ordered) = self.read_record(key, record).await?;
		let applied = applied.unwrap_or_else(|| record.cold.clone());
		// A latest open by a writer serves the highest applied position with no filesystem
		// transaction half applied, so the writer can replay the log from just above it. A
		// stateless reader replays nothing, so it reads the whole applied position. Cold can
		// pass the settled position when a flush lands while a transaction is open, and a
		// session cannot read below cold, so the served position is never below it.
		let position = match position {
			Some(position) => position,
			None if client_epoch.is_none() => applied.clone(),
			None => match settled {
				Some(settled) if settled.volume_seq > record.cold.volume_seq => settled,
				Some(_) | None => record.cold.clone(),
			},
		};
		if client_epoch.is_some_and(|epoch| applied.client_epoch > epoch)
			|| (position.volume_seq == record.cold.volume_seq && position != record.cold)
			|| (position.volume_seq == applied.volume_seq && position != applied)
		{
			return Err(Error::Fenced);
		}
		let session = self
			.register_read(key, &record, ordered, applied, position.volume_seq)
			.await?;
		Ok((session, position))
	}

	/// Opens the one-request session of a claim read of `key`. It reads at the newest applied
	/// position from `floor` through `through`, which the core only dispatches once it has
	/// applied `floor`. A claim read comes only from the volume's exclusive client, so it must
	/// carry the client epoch, and evidence of a later writer fences it as it fences an open.
	///
	/// The core runs claim reads beside the volume's other store work, so `record` is the one
	/// it captured at dispatch rather than one ordered after every write; `QueryOrder::Claim`
	/// and `FilesystemStore::open_claim_session` say why that is safe without a lock.
	async fn open_claim_read(
		&self,
		key: &depot_types::VolumeLogKey,
		client_epoch: Option<u64>,
		read: &FsClaimRead,
		applied: Option<VolumeLogPosition>,
		record: Option<depot_fs_core::FsVolumeRecord>,
	) -> Result<std::sync::Arc<super::session::ReadSession>> {
		let Some(client_epoch) = client_epoch else {
			return Err(Error::Corrupt);
		};
		if read.through < read.floor.volume_seq {
			return Err(Error::Corrupt);
		}
		let (record, _) = self.read_record(key, record).await?;
		let applied = applied.unwrap_or_else(|| record.cold.clone());
		let floor = &read.floor;
		if applied.client_epoch > client_epoch
			|| (floor.volume_seq == record.cold.volume_seq && *floor != record.cold)
			|| (floor.volume_seq == applied.volume_seq && *floor != applied)
		{
			return Err(Error::Fenced);
		}
		if floor.volume_seq > applied.volume_seq {
			return Err(Error::Lagging);
		}
		let as_of = applied.volume_seq.min(read.through);
		self.open_claim_session(key, &record, applied, as_of).await
	}

	pub async fn read_request(
		&self,
		request: FsReadRequest,
		applied: Option<VolumeLogPosition>,
		settled: Option<VolumeLogPosition>,
		record: Option<depot_fs_core::FsVolumeRecord>,
	) -> std::result::Result<FsReadResponse, String> {
		match self.read_operation(request, applied, settled, record).await {
			Ok(response) => Ok(response),
			Err(Error::Storage(error)) => Err(error.to_string()),
			Err(error) => Ok(FsReadResponse::FsReadError(match error {
				Error::TooOld => FsReadError::TooOld,
				Error::Lagging => FsReadError::Lagging,
				Error::Fenced => FsReadError::Fenced,
				Error::TooLarge => FsReadError::TooLarge,
				Error::Overloaded => FsReadError::Overloaded,
				Error::NotFound => FsReadError::NotFound,
				Error::Corrupt => FsReadError::Corrupt,
				Error::BaseUnavailable => FsReadError::BaseUnavailable,
				Error::Storage(_) => unreachable!(),
			})),
		}
	}
	async fn read_operation(
		&self,
		request: FsReadRequest,
		applied: Option<VolumeLogPosition>,
		settled: Option<VolumeLogPosition>,
		record: Option<depot_fs_core::FsVolumeRecord>,
	) -> Result<FsReadResponse> {
		if depot_fs_core::wire::source_link(&request.operation).is_some() {
			if !depot_fs_core::wire::valid_source_target(&request) {
				return Err(Error::Corrupt);
			}
			return match request.operation {
				FsReadOperation::FsBaseChunksRequest(v) => self
					.read_base_chunks(&v.link, v.ino, &v.indices)
					.await
					.map(|chunks| {
						FsReadResponse::FsBaseChunksResponse(FsBaseChunksResponse {
							chunks: chunks.into_iter().map(Into::into).collect(),
						})
					}),
				FsReadOperation::FsForkMetadataRequest(v) => self
					.fork_metadata_page(v)
					.await
					.map(FsReadResponse::FsForkMetadataPage),
				FsReadOperation::FsBaseKeysRequest(v) => self
					.base_chunk_keys(v)
					.await
					.map(FsReadResponse::FsBaseKeysPage),
				_ => unreachable!("source operation classified above"),
			};
		}
		let key = match &request.target {
			FsReadTarget::VolumeLogKey(key) => depot_types::VolumeLogKey(key.clone()),
			FsReadTarget::VolumeKey(key) => {
				let latest = match &request.operation {
					FsReadOperation::FsReadOpen(FsReadOpen { position, .. })
					| FsReadOperation::FsReadKeys(FsReadKeys { position, .. })
					| FsReadOperation::FsReadRaw(FsReadRaw { position, .. }) => position.is_none(),
					FsReadOperation::FsReadGet(_)
					| FsReadOperation::FsReadScan(_)
					| FsReadOperation::FsReadChunks(_)
					| FsReadOperation::FsReadClose(_)
					| FsReadOperation::FsBaseChunksRequest(_)
					| FsReadOperation::FsForkMetadataRequest(_)
					| FsReadOperation::FsBaseKeysRequest(_) => false,
				};
				if request.client_epoch.is_some() || !latest {
					return Err(Error::Corrupt);
				}
				// The core resolves a loaded volume's latest open to an exact key, so a
				// logical target here names a volume the maintainer does not hold.
				let current = match &record {
					Some(record) => record.clone(),
					None => self.record(key).await?.ok_or(Error::NotFound)?,
				};
				depot_types::VolumeLogKey(
					rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
						volume_key: key.clone(),
						volume_incarnation: current.volume_incarnation,
					},
				)
			}
		};
		if let FsReadOperation::FsReadClose(close) = request.operation {
			self.close_session_for(Uuid::from_bytes(close.session), &key)?;
			return Ok(FsReadResponse::FsReadClosed);
		}
		if let FsReadOperation::FsReadKeys(point) = request.operation {
			if point.keys.len() > self.config.max_rows {
				return Err(Error::TooLarge);
			}
			let session = self
				.open_pinned(
					&key,
					request.client_epoch,
					point.position,
					applied,
					settled.clone(),
					record,
				)
				.await?
				.0;
			let values = {
				let _close = SessionClose {
					store: self,
					id: session.id,
				};
				read_kv_values(&session, &point.file, &point.keys).await?
			};
			let bytes = values
				.iter()
				.map(|value| value.as_ref().map_or(0, |value| value.len()))
				.fold(0usize, usize::saturating_add);
			if bytes > self.config.max_read_bytes {
				return Err(Error::TooLarge);
			}
			return Ok(FsReadResponse::FsReadValues(FsReadValues {
				volume_log_key: key.into(),
				applied: session.applied.clone(),
				as_of: session.as_of,
				values,
			}));
		}
		if let FsReadOperation::FsReadRaw(raw) = request.operation {
			let session = self
				.open_pinned(
					&key,
					request.client_epoch,
					raw.position,
					applied,
					settled,
					record,
				)
				.await?
				.0;
			let row = RowKey::from(raw.key);
			let versions = {
				let _close = SessionClose {
					store: self,
					id: session.id,
				};
				session.raw_versions(&row, raw.versions as usize).await?
			};
			return Ok(FsReadResponse::FsReadRawVersions(FsReadRawVersions {
				volume_log_key: key.into(),
				applied: session.applied.clone(),
				as_of: session.as_of,
				versions: versions
					.into_iter()
					.map(|(seq, raw, decoded)| {
						let (deleted, decoded, decode_error) = match decoded {
							Ok(Some(value)) => (false, Some(value.into()), None),
							Ok(None) => (true, None, None),
							Err(error) => (false, None, Some(error)),
						};
						FsRawVersion {
							seq,
							raw: raw.into(),
							deleted,
							decoded,
							decode_error: decode_error.map(Into::into),
						}
					})
					.collect(),
			}));
		}
		if let FsReadOperation::FsReadOpen(open) = request.operation {
			let latest = open.position.is_none();
			let (session, position) = self
				.open_pinned(
					&key,
					request.client_epoch,
					open.position,
					applied,
					settled,
					record,
				)
				.await?;
			let now = self.clock.observe().map_err(|_| Error::Fenced)?.boot_ms;
			// A claim holder reads with claim reads afterwards, so its open keeps the session
			// only for the prefetch and releases it with the reply.
			let claim = request.client_epoch.is_some();
			let _close = match claim {
				true => Some(SessionClose {
					store: self,
					id: session.id,
				}),
				false => None,
			};
			// A stateless reader never reopens a latest session at its position, and an HTTP
			// directory cursor resumes it between pages, so it keeps the longer idle timeout.
			if latest && !claim {
				session.idle_as_latest(now);
			}
			let rows = match &open.prefetch {
				Some(prefetch) => {
					let rows = self.prefetch_rows(&session, prefetch).await?;
					record_served(&session, rows.iter().map(|(_, row, _)| row));
					Some(
						rows.into_iter()
							.map(|(key, value, change_seq)| FsReadRow {
								change_seq,
								key: key.into(),
								value: value.into(),
							})
							.collect(),
					)
				}
				None => None,
			};
			return Ok(FsReadResponse::FsReadOpened(FsReadOpened {
				volume_log_key: key.into(),
				session: match claim {
					true => None,
					false => Some(FsOpenedSession {
						id: *session.id.as_bytes(),
						lifetime_ms: session.expires_boot_ms.saturating_sub(now),
					}),
				},
				// A latest open reports the position it serves as the applied position, which
				// is what the reader moves its snapshot to.
				applied: if latest {
					position.clone()
				} else {
					session.applied.clone()
				},
				as_of: session.as_of,
				rows,
			}));
		}
		let at = match &request.operation {
			FsReadOperation::FsReadGet(v) => &v.at,
			FsReadOperation::FsReadScan(v) => &v.at,
			FsReadOperation::FsReadChunks(v) => &v.at,
			FsReadOperation::FsReadClose(_)
			| FsReadOperation::FsReadOpen(_)
			| FsReadOperation::FsBaseChunksRequest(_)
			| FsReadOperation::FsForkMetadataRequest(_)
			| FsReadOperation::FsBaseKeysRequest(_)
			| FsReadOperation::FsReadKeys(_)
			| FsReadOperation::FsReadRaw(_) => unreachable!("handled before row reads"),
		};
		// A session read uses the session its open registered; a claim read registers one for
		// this request only and releases it when the read finishes or is cancelled.
		let (session, reply_session, _close) = match at {
			FsReadAt::FsSessionRead(read) => {
				let session = self.session(Uuid::from_bytes(read.session), &key)?;
				if request
					.client_epoch
					.is_some_and(|epoch| session.applied.client_epoch > epoch)
				{
					return Err(Error::Fenced);
				}
				(session, Some(read.session), None)
			}
			FsReadAt::FsClaimRead(read) => {
				let session = self
					.open_claim_read(&key, request.client_epoch, read, applied, record)
					.await?;
				let close = SessionClose {
					store: self,
					id: session.id,
				};
				(session, None, Some(close))
			}
		};
		let mut change_seq = None;
		let rows: Vec<(RowKey, Row)> = match request.operation {
			FsReadOperation::FsReadGet(v) => {
				let key = RowKey::from(v.key);
				if matches!(key, RowKey::Inode(_)) {
					change_seq = session.version(&key).await?.map(|(seq, _)| seq);
				}
				session
					.get(&key)
					.await?
					.map(|row| (key, row))
					.into_iter()
					.collect()
			}
			FsReadOperation::FsReadScan(v) => {
				let after = v.after.map(RowKey::from);
				session
					.scan(&RowRange::from(v.range), after.as_ref(), v.limit as usize)
					.await?
			}
			FsReadOperation::FsReadChunks(v) => {
				let mut rows = Vec::new();
				let mut bytes = 0usize;
				read_chunk_ranges(
					&session,
					&self.config,
					v.ino,
					&v.ranges,
					&mut rows,
					&mut bytes,
				)
				.await?;
				rows
			}
			FsReadOperation::FsReadClose(_)
			| FsReadOperation::FsReadOpen(_)
			| FsReadOperation::FsBaseChunksRequest(_)
			| FsReadOperation::FsForkMetadataRequest(_)
			| FsReadOperation::FsBaseKeysRequest(_)
			| FsReadOperation::FsReadKeys(_)
			| FsReadOperation::FsReadRaw(_) => unreachable!("handled before row reads"),
		};
		record_served(&session, rows.iter().map(|(_, row)| row));
		Ok(FsReadResponse::FsReadRows(FsReadRows {
			session: reply_session,
			applied: session.applied.clone(),
			as_of: session.as_of,
			rows: rows
				.into_iter()
				.map(|(key, value)| FsReadRow {
					change_seq,
					key: key.into(),
					value: value.into(),
				})
				.collect(),
		}))
	}
}

impl FilesystemStore {
	/// Reads what an open asked for beside its session, at the session's position: the root
	/// inode, the format row, the first page of orphan rows, the path's dentries and inodes as
	/// far as it resolves, the final inode's xattrs, and for each KV prefetch its path the same
	/// way and the first page of that inode's rows in its range. Bounded like one read before
	/// anything is read: a prefetch that could return more than `max_rows` rows fails the open
	/// as too large, so a caller cannot make an ordered open do unbounded work.
	async fn prefetch_rows(
		&self,
		session: &super::session::ReadSession,
		prefetch: &FsOpenPrefetch,
	) -> Result<Vec<(RowKey, Row, Option<u64>)>> {
		let mut most_rows = prefetch
			.path
			.len()
			.saturating_mul(2)
			.saturating_add(prefetch.xattrs.len())
			.saturating_add(2)
			.saturating_add(PREFETCH_ORPHAN_ROWS);
		for kv in &prefetch.kv {
			most_rows = most_rows
				.saturating_add(kv.path.len().saturating_mul(2))
				.saturating_add(depot_fs_core::wire::PREFETCH_KV_ROWS);
		}
		if most_rows > self.config.max_rows {
			return Err(Error::TooLarge);
		}
		// All branches share the same fixed snapshot. Start independent metadata and
		// paths together so the block cache can coalesce their common SST reads.
		let metadata = async {
			let (root, format, orphans) = futures_util::try_join!(
				session.version(&RowKey::Inode(depot_fs_core::ROOT_INO)),
				session.get(&RowKey::Format),
				session.scan(&RowRange::Orphans, None, PREFETCH_ORPHAN_ROWS),
			)?;
			let mut rows = Vec::new();
			if let Some((change_seq, Some(row))) = root {
				rows.push((
					RowKey::Inode(depot_fs_core::ROOT_INO),
					row,
					Some(change_seq),
				));
			}
			if let Some(row) = format {
				rows.push((RowKey::Format, row, None));
			}
			rows.extend(orphans.into_iter().map(|(key, row)| (key, row, None)));
			Ok::<_, Error>(rows)
		};
		let path = async {
			let mut rows = Vec::new();
			if let Some(ino) = resolve_prefetch_path(session, &prefetch.path, &mut rows).await? {
				let mut reads = Vec::with_capacity(prefetch.xattrs.len());
				for name in &prefetch.xattrs {
					reads.push(async move {
						let key = RowKey::Xattr(ino, name.to_vec());
						Ok::<_, Error>(session.get(&key).await?.map(|row| (key, row, None)))
					});
				}
				let xattrs = futures_util::stream::iter(reads)
					.buffered(OPEN_READS_IN_FLIGHT)
					.try_collect::<Vec<_>>()
					.await?;
				rows.extend(xattrs.into_iter().flatten());
			}
			Ok::<_, Error>(rows)
		};
		let mut reads = Vec::with_capacity(prefetch.kv.len());
		for kv in &prefetch.kv {
			reads.push(async move {
				let mut rows = Vec::new();
				if let Some(ino) = resolve_prefetch_path(session, &kv.path, &mut rows).await? {
					let range = RowRange::Kv {
						ino,
						start: kv.start.to_vec(),
						end: kv.end.clone().map(Into::into),
					};
					let values = session
						.scan(&range, None, depot_fs_core::wire::PREFETCH_KV_ROWS)
						.await?;
					rows.extend(values.into_iter().map(|(key, row)| (key, row, None)));
				}
				Ok::<_, Error>(rows)
			});
		}
		let kv = futures_util::stream::iter(reads)
			.buffered(OPEN_READS_IN_FLIGHT)
			.try_collect::<Vec<_>>();
		let (mut rows, path, kv) = futures_util::try_join!(metadata, path, kv)?;
		// Preserve the wire order even when storage reads finish out of order.
		rows.extend(path);
		rows.extend(kv.into_iter().flatten());
		Ok(rows)
	}
}

/// Independent paths or attributes read concurrently within one bounded open.
const OPEN_READS_IN_FLIGHT: usize = 16;

/// Resolves `path` from the root at the session's position, pushing each step's dentry and
/// the child's inode onto `rows`, and returns the final inode, or `None` at the first missing
/// step. An empty path resolves to the root.
async fn resolve_prefetch_path(
	session: &super::session::ReadSession,
	path: &[vbare::Bytes],
	rows: &mut Vec<(RowKey, Row, Option<u64>)>,
) -> Result<Option<u64>> {
	let mut parent = depot_fs_core::ROOT_INO;
	for name in path {
		let key = RowKey::Dentry(parent, name.to_vec());
		let Some(row) = session.get(&key).await? else {
			return Ok(None);
		};
		let Row::Dentry(dentry) = &row else {
			return Err(Error::Corrupt);
		};
		let child = dentry.child;
		rows.push((key, row, None));
		let Some((change_seq, Some(inode))) = session.version(&RowKey::Inode(child)).await? else {
			return Ok(None);
		};
		rows.push((RowKey::Inode(child), inode, Some(change_seq)));
		parent = child;
	}
	Ok(Some(parent))
}

/// Orphan rows an open's prefetch returns: one page, as the opening scans them.
const PREFETCH_ORPHAN_ROWS: usize = 256;

/// Chunk ranges of one read that are read from the store at once. A client used to send
/// each range as its own request with this many in flight, which is what spread a large
/// opening read's block fetches across the store; one request keeps that spread.
const CHUNK_RANGES_IN_FLIGHT: usize = 16;

/// Reads the chunk rows of `ino` in `ranges` into `rows`, counting their bytes in `bytes`.
/// The ranges together are bounded as one read, by `max_rows` chunks and `max_read_bytes`,
/// before anything is read. Ranges are read concurrently and returned in range order.
async fn read_chunk_ranges(
	session: &super::session::ReadSession,
	config: &super::session::SessionConfig,
	ino: u64,
	ranges: &[FsChunkRange],
	rows: &mut Vec<(RowKey, Row)>,
	bytes: &mut usize,
) -> Result<()> {
	let mut chunks = 0usize;
	for range in ranges {
		chunks = chunks.saturating_add(range.count as usize);
		if chunks > config.max_rows || range.first.checked_add(u64::from(range.count)).is_none() {
			return Err(Error::TooLarge);
		}
	}
	let mut reads = Vec::with_capacity(ranges.len());
	for range in ranges {
		let first = range.first;
		let count = range.count as usize;
		reads.push(async move {
			let values = session.read_chunks(ino, first, count).await?;
			Ok::<_, Error>((first, values))
		});
	}
	let reads = futures_util::stream::iter(reads)
		.buffered(CHUNK_RANGES_IN_FLIGHT)
		.try_collect::<Vec<_>>()
		.await?;
	for (first, values) in reads {
		for (offset, value) in values.into_iter().enumerate() {
			if let Some(data) = value {
				*bytes = bytes.saturating_add(data.len()).saturating_add(32);
				if *bytes > config.max_read_bytes {
					return Err(Error::TooLarge);
				}
				rows.push((
					RowKey::Chunk(ino, first + offset as u64),
					Row::Bytes(data.into()),
				));
			}
		}
	}
	Ok(())
}

/// Releases a one-shot read's session when the read finishes or its task is cancelled, so an
/// aborted read does not hold a session slot until the idle expiry.
struct SessionClose<'a> {
	store: &'a FilesystemStore,
	id: Uuid,
}

impl Drop for SessionClose<'_> {
	fn drop(&mut self) {
		self.store.close_session(self.id);
	}
}

/// Reads `keys` of the KV file `file` under the volume root from one session. Returns one
/// value per key, `None` where the key or the file is absent.
async fn read_kv_values(
	session: &super::session::ReadSession,
	file: &[u8],
	keys: &[vbare::Bytes],
) -> Result<Vec<Option<vbare::Bytes>>> {
	let ino = match session
		.get(&RowKey::Dentry(depot_fs_core::ROOT_INO, file.to_vec()))
		.await?
	{
		Some(Row::Dentry(dentry)) if dentry.kind == depot_fs_core::FsKind::Kv => dentry.child,
		Some(_) | None => return Ok(vec![None; keys.len()]),
	};
	let mut values = Vec::with_capacity(keys.len());
	for wanted in keys {
		let value = match session.get(&RowKey::Kv(ino, wanted.to_vec())).await? {
			Some(Row::Bytes(bytes)) => Some(vbare::Bytes::from(bytes)),
			Some(_) => return Err(Error::Corrupt),
			None => None,
		};
		values.push(value);
	}
	Ok(values)
}

/// Counts a read's rows once per request, by whether its snapshot reached past cold.
fn record_served<'a>(
	session: &super::session::ReadSession,
	rows: impl ExactSizeIterator<Item = &'a Row>,
) {
	if rows.len() == 0 {
		return;
	}
	let snapshot = if session.as_of <= session.cold_volume_seq {
		"cold"
	} else {
		"includes_applied"
	};
	let count = rows.len() as u64;
	let bytes: u64 = rows
		.map(|row| match row {
			Row::Bytes(bytes) => bytes.len() as u64,
			Row::Inode(inode) => inode.inline.as_ref().map_or(0, |inline| inline.len()) as u64,
			Row::Format(_) | Row::Dentry(_) | Row::Unit => 0,
		})
		.sum();
	crate::metrics::READ_SESSION_ROWS
		.with_label_values(&[snapshot])
		.inc_by(count);
	crate::metrics::READ_SESSION_BYTES
		.with_label_values(&[snapshot])
		.inc_by(bytes);
}
