//! Grant-routed filesystem reads of the claim holder, which registers no read sessions.

use super::{
	Error, Result,
	transport::{ColdSnapshot, ColdStore, Journal, OpenPrefetch, OpenedSnapshot, PrefetchedRow},
};
use depot_fs_core::store::{Row, RowKey, RowRange};
use depot_types::VolumeLogKey;
use futures_util::future::BoxFuture;
use rivet_data::generated::{
	depot_client_protocol_v1::{ClientRequest, ClientResponse, VolumeRejectResponse},
	shared_depot_fs_read_v1::*,
	shared_depot_types_v1::VolumeLogPosition,
};
use rivet_env::Clock;
use std::sync::{
	Arc,
	atomic::{AtomicU64, Ordering},
};

/// The process owns transport tasks. This adapter starts none and sends each read once,
/// within the caller's deadline.
pub trait ReadTransport: Send + Sync {
	/// Sends one read with the transport's own timeout.
	fn read(&self, request: FsReadRequest) -> BoxFuture<'_, Result<FsReadResponse>>;

	/// Sends one read whose deadline, also sent to the maintainer, is the earlier of
	/// `deadline_boot_ms` and the transport's own timeout.
	fn read_at(
		&self,
		request: FsReadRequest,
		deadline_boot_ms: u64,
	) -> BoxFuture<'_, Result<FsReadResponse>>;

	/// Sends one open or claim read whose deadline, also sent to the maintainer, is exactly
	/// `deadline_boot_ms`. Either can wait on the maintainer until it has applied the position
	/// the read names, so it carries its caller's deadline instead of the transport's shorter
	/// timeout. Callers never pass a deadline later than their own caller's.
	fn open_at(
		&self,
		request: FsReadRequest,
		deadline_boot_ms: u64,
	) -> BoxFuture<'_, Result<FsReadResponse>> {
		self.read_at(request, deadline_boot_ms)
	}
}

impl ReadTransport for crate::volume_requests::VolumeRequests {
	fn read(&self, request: FsReadRequest) -> BoxFuture<'_, Result<FsReadResponse>> {
		Box::pin(
			async move { read_response(self.request(ClientRequest::FsReadRequest(request)).await) },
		)
	}

	fn read_at(
		&self,
		request: FsReadRequest,
		deadline_boot_ms: u64,
	) -> BoxFuture<'_, Result<FsReadResponse>> {
		Box::pin(async move {
			let deadline = self
				.deadline()
				.map_err(|_| Error::Unavailable)?
				.min(deadline_boot_ms);
			read_response(
				self.request_at(ClientRequest::FsReadRequest(request), deadline)
					.await,
			)
		})
	}

	fn open_at(
		&self,
		request: FsReadRequest,
		deadline_boot_ms: u64,
	) -> BoxFuture<'_, Result<FsReadResponse>> {
		Box::pin(async move {
			read_response(
				self.request_at(ClientRequest::FsReadRequest(request), deadline_boot_ms)
					.await,
			)
		})
	}
}

fn read_response(response: anyhow::Result<ClientResponse>) -> Result<FsReadResponse> {
	match response.map_err(|error| {
		let kind = error.downcast_ref::<rivet_error::RivetError>();
		match kind.map(|kind| (kind.group(), kind.code())) {
			Some(("ups", "overloaded")) => Error::Overloaded,
			Some(("depot", "routing_unavailable"))
			| Some((
				"ups",
				"wrong_worker"
				| "unknown_worker"
				| "endpoint_unresolved"
				| "no_responders"
				| "consumer_closed"
				| "transport"
				| "timeout",
			)) => Error::ColdReadRefused,
			// Revoked authority, invalid messages and unclassified failures are terminal.
			_ => Error::Unavailable,
		}
	})? {
		ClientResponse::FsReadResponse(response) => Ok(response),
		ClientResponse::VolumeRejectResponse(
			VolumeRejectResponse::NotMaintainer(_) | VolumeRejectResponse::TabletSplitting,
		) => Err(Error::ColdReadRefused),
		ClientResponse::VolumeRejectResponse(
			VolumeRejectResponse::VolumeNotFound | VolumeRejectResponse::VolumeDeleted,
		) => Err(depot_fs_core::Error::NotFound.into()),
		ClientResponse::VolumeRejectResponse(VolumeRejectResponse::Overloaded(_)) => {
			crate::metrics::FS_OVERLOADED
				.with_label_values(&["remote_rejected"])
				.inc();
			Err(Error::Overloaded)
		}
		ClientResponse::VolumeRejectResponse(VolumeRejectResponse::VolumeExists) => {
			Err(Error::Unavailable)
		}
		ClientResponse::VolumeRejectResponse(VolumeRejectResponse::VolumeRecordMissing(reject)) => {
			Err(Error::RecordMissing {
				incarnation: reject.volume_incarnation,
			})
		}
		_ => Err(corrupt!()),
	}
}

/// The claim holder's cold store. It registers no read session: an open reads its position
/// and prefetch in one request, and every later miss is one claim read.
pub struct RemoteColdStore {
	transport: Arc<dyn ReadTransport>,
	journal: Arc<dyn Journal>,
	clock: Arc<dyn Clock>,
	/// The newest applied sequence any reply reported, shared with this store's snapshots.
	applied: Arc<AtomicU64>,
}
impl RemoteColdStore {
	pub fn new(
		transport: Arc<dyn ReadTransport>,
		journal: Arc<dyn Journal>,
		clock: Arc<dyn Clock>,
	) -> Self {
		Self {
			transport,
			journal,
			clock,
			applied: Arc::new(AtomicU64::new(0)),
		}
	}
}
impl ColdStore for RemoteColdStore {
	fn open_prefetching(
		&self,
		volume: VolumeLogKey,
		position: VolumeLogPosition,
		prefetch: OpenPrefetch,
		deadline_boot_ms: u64,
	) -> BoxFuture<'_, Result<OpenedSnapshot>> {
		Box::pin(async move {
			self.open_snapshot(volume, Opening::Exact(position), prefetch, deadline_boot_ms)
				.await
		})
	}

	fn open_latest_prefetching(
		&self,
		volume: VolumeLogKey,
		floor: VolumeLogPosition,
		prefetch: OpenPrefetch,
		deadline_boot_ms: u64,
	) -> BoxFuture<'_, Result<OpenedSnapshot>> {
		Box::pin(async move {
			self.open_snapshot(
				volume,
				Opening::Latest { floor },
				prefetch,
				deadline_boot_ms,
			)
			.await
		})
	}

	fn claim_snapshot(
		&self,
		volume: VolumeLogKey,
		floor: VolumeLogPosition,
	) -> Arc<dyn ColdSnapshot> {
		Arc::new(ClaimSnapshot {
			transport: self.transport.clone(),
			journal: self.journal.clone(),
			clock: self.clock.clone(),
			applied: self.applied.clone(),
			volume,
			floor,
		})
	}

	fn observed_applied(&self) -> u64 {
		self.applied.load(Ordering::Acquire)
	}
}

impl RemoteColdStore {
	async fn open_snapshot(
		&self,
		volume: VolumeLogKey,
		opening: Opening,
		prefetch: OpenPrefetch,
		deadline_boot_ms: u64,
	) -> Result<OpenedSnapshot> {
		let (position, applied, rows) = open_position(
			self.transport.as_ref(),
			self.journal.as_ref(),
			&volume,
			&opening,
			&prefetch,
			deadline_boot_ms,
			self.clock.as_ref(),
		)
		.await?;
		self.applied.fetch_max(applied, Ordering::AcqRel);
		// Only rows the maintainer says it read count; an open that got no prefetch back
		// learns nothing about absent rows.
		let prefetched = match rows {
			Some(rows) => Some(decode_prefetched(rows, &position)?),
			None => None,
		};
		Ok(OpenedSnapshot {
			snapshot: self.claim_snapshot(volume, position),
			prefetched,
		})
	}
}

/// Decodes the rows an open's prefetch returned. An inode row must carry a change sequence
/// no newer than the snapshot, as a single inode read must.
fn decode_prefetched(
	rows: Vec<FsReadRow>,
	position: &VolumeLogPosition,
) -> Result<Vec<PrefetchedRow>> {
	rows.into_iter()
		.map(|row| {
			let change_seq = row.change_seq;
			let (key, value) = depot_fs_core::wire::decode_row(row)?;
			if matches!(key, RowKey::Inode(_)) {
				let change_seq = change_seq.ok_or_else(|| corrupt!())?;
				if change_seq > position.volume_seq {
					return Err(corrupt!());
				}
			}
			Ok(PrefetchedRow {
				key,
				row: value,
				change_seq,
			})
		})
		.collect()
}

/// Converts the opening's prefetch to its wire form.
fn wire_prefetch(prefetch: &OpenPrefetch) -> FsOpenPrefetch {
	FsOpenPrefetch {
		path: prefetch
			.path
			.iter()
			.map(|name| name.clone().into())
			.collect(),
		xattrs: prefetch
			.xattrs
			.iter()
			.map(|name| name.clone().into())
			.collect(),
		kv: prefetch
			.kv
			.iter()
			.map(|kv| FsKvPrefetch {
				path: kv.path.iter().map(|name| name.clone().into()).collect(),
				start: kv.start.clone().into(),
				end: kv.end.clone().map(Into::into),
			})
			.collect(),
	}
}

/// Which position an open asks the maintainer for.
enum Opening {
	/// Exactly this position, or an error.
	Exact(VolumeLogPosition),
	/// The maintainer's latest applied position, which must not be below `floor`.
	Latest { floor: VolumeLogPosition },
}

/// Opens `volume` for the opening's current client epoch at the position `opening` asks for,
/// and returns that position, the applied sequence the maintainer reported, and the rows
/// `prefetch` found there. The maintainer releases the open's snapshot with its reply, so
/// nothing stays registered for this opening.
async fn open_position(
	transport: &dyn ReadTransport,
	journal: &dyn Journal,
	volume: &VolumeLogKey,
	opening: &Opening,
	prefetch: &OpenPrefetch,
	deadline_boot_ms: u64,
	clock: &dyn Clock,
) -> Result<(VolumeLogPosition, u64, Option<Vec<FsReadRow>>)> {
	let status = journal.status()?;
	if status.key != *volume {
		crate::fences::count(crate::fences::FenceReason::KeyMismatch);
		journal.fence();
		return Err(Error::Fenced);
	}
	let request = FsReadRequest {
		target: FsReadTarget::VolumeLogKey(volume.clone().into()),
		client_epoch: Some(status.client_epoch),
		operation: FsReadOperation::FsReadOpen(FsReadOpen {
			position: match opening {
				Opening::Exact(position) => Some(position.clone()),
				Opening::Latest { .. } => None,
			},
			prefetch: Some(wire_prefetch(prefetch)),
		}),
	};
	let response = routed_read(transport, clock, request, deadline_boot_ms)
		.await
		.inspect_err(|error| {
			if *error == Error::Fenced {
				crate::fences::count(crate::fences::FenceReason::RemoteReadFenced);
				journal.fence();
			}
		})?;
	let FsReadResponse::FsReadOpened(opened) = response else {
		return Err(corrupt!());
	};
	// A maintainer that registered a session for a claim holder's open would hold it until it
	// expired, because nothing here reads or closes it.
	if opened.volume_log_key != volume.0 || opened.session.is_some() {
		return Err(corrupt!());
	}
	let position = match opening {
		Opening::Exact(position) => {
			validate_position(journal, volume, position, opened.as_of, &opened.applied)?;
			position.clone()
		}
		Opening::Latest { floor } => {
			validate_latest(journal, volume, floor, opened.as_of, &opened.applied)?;
			opened.applied.clone()
		}
	};
	Ok((position, opened.applied.volume_seq, opened.rows))
}

/// Checks that a latest open served the maintainer's applied position under the opening's
/// current epoch, at or above `floor`. A maintainer that restarted can answer below the
/// caller's own snapshot; the caller decides whether that position is useful.
fn validate_latest(
	journal: &dyn Journal,
	volume: &VolumeLogKey,
	floor: &VolumeLogPosition,
	as_of: u64,
	applied: &VolumeLogPosition,
) -> Result<()> {
	current_epoch(journal, volume)?;
	if as_of != applied.volume_seq {
		return Err(corrupt!());
	}
	let status = journal.status()?;
	if applied.client_epoch > status.client_epoch
		|| (applied.volume_seq == floor.volume_seq && applied != floor)
	{
		crate::fences::count(crate::fences::FenceReason::RemoteReadStale);
		journal.fence();
		return Err(Error::Fenced);
	}
	if applied.volume_seq < floor.volume_seq {
		return Err(Error::Lagging);
	}
	Ok(())
}

/// Checks that a reply served exactly `position` under the opening's current epoch.
fn validate_position(
	journal: &dyn Journal,
	volume: &VolumeLogKey,
	position: &VolumeLogPosition,
	as_of: u64,
	applied: &VolumeLogPosition,
) -> Result<()> {
	let epoch = current_epoch(journal, volume)?;
	if as_of != position.volume_seq || applied.volume_seq < as_of {
		return Err(corrupt!());
	}
	if applied.client_epoch > epoch || (applied.volume_seq == as_of && applied != position) {
		crate::fences::count(crate::fences::FenceReason::RemoteReadStale);
		journal.fence();
		return Err(Error::Fenced);
	}
	Ok(())
}

/// Checks that a claim read served a position from `floor` through `through` under the
/// opening's current epoch.
fn validate_claim_read(
	journal: &dyn Journal,
	volume: &VolumeLogKey,
	floor: &VolumeLogPosition,
	through: u64,
	as_of: u64,
	applied: &VolumeLogPosition,
) -> Result<()> {
	let epoch = current_epoch(journal, volume)?;
	if as_of < floor.volume_seq || as_of > through || applied.volume_seq < as_of {
		return Err(corrupt!());
	}
	if applied.client_epoch > epoch || (applied.volume_seq == floor.volume_seq && applied != floor)
	{
		crate::fences::count(crate::fences::FenceReason::RemoteReadStale);
		journal.fence();
		return Err(Error::Fenced);
	}
	Ok(())
}

/// The opening's current client epoch, fencing its journal when the journal moved to
/// another volume.
fn current_epoch(journal: &dyn Journal, volume: &VolumeLogKey) -> Result<u64> {
	let status = journal.status()?;
	if status.key != *volume {
		crate::fences::count(crate::fences::FenceReason::KeyMismatch);
		journal.fence();
		return Err(Error::Fenced);
	}
	Ok(status.client_epoch)
}

/// The claim holder's snapshot of one volume at `floor`, read without a session.
///
/// The claim holder is the volume's only writer, and it holds a layer in memory for every
/// entry above `floor` that a view includes. For any row no such layer writes, every applied
/// position from `floor` through the view's sequence has the row `floor` has, so the latest
/// applied snapshot plus the holder's own layers is always a consistent view and nothing needs
/// pinning across requests. Each miss is one claim read: the maintainer answers at the newest
/// position it has applied, waiting within the read's deadline until it has applied `floor`,
/// so a row the holder dropped from memory is never read from a lagging snapshot. The read
/// carries the client epoch, so a newer claim fails it as fenced.
struct ClaimSnapshot {
	transport: Arc<dyn ReadTransport>,
	journal: Arc<dyn Journal>,
	clock: Arc<dyn Clock>,
	/// The store's newest applied sequence, raised by every validated reply.
	applied: Arc<AtomicU64>,
	volume: VolumeLogKey,
	floor: VolumeLogPosition,
}
impl ClaimSnapshot {
	/// Sends one claim read through `through` and returns its validated rows.
	async fn wire_rows(
		&self,
		through: u64,
		deadline_boot_ms: u64,
		operation: impl FnOnce(FsReadAt) -> FsReadOperation,
	) -> Result<Vec<FsReadRow>> {
		let epoch = current_epoch(self.journal.as_ref(), &self.volume)?;
		let at = FsReadAt::FsClaimRead(FsClaimRead {
			floor: self.floor.clone(),
			through,
		});
		let request = FsReadRequest {
			target: FsReadTarget::VolumeLogKey(self.volume.clone().into()),
			client_epoch: Some(epoch),
			operation: operation(at),
		};
		let response = match routed_read(
			self.transport.as_ref(),
			self.clock.as_ref(),
			request,
			deadline_boot_ms,
		)
		.await
		{
			Err(Error::Fenced) => {
				crate::fences::count(crate::fences::FenceReason::RemoteReadFenced);
				self.journal.fence();
				return Err(Error::Fenced);
			}
			other => other?,
		};
		let FsReadResponse::FsReadRows(rows) = response else {
			return Err(corrupt!());
		};
		if rows.session.is_some() {
			return Err(corrupt!());
		}
		validate_claim_read(
			self.journal.as_ref(),
			&self.volume,
			&self.floor,
			through,
			rows.as_of,
			&rows.applied,
		)?;
		self.applied
			.fetch_max(rows.applied.volume_seq, Ordering::AcqRel);
		Ok(rows.rows)
	}

	async fn rows(
		&self,
		through: u64,
		deadline_boot_ms: u64,
		operation: impl FnOnce(FsReadAt) -> FsReadOperation,
	) -> Result<Vec<(RowKey, Row)>> {
		self.wire_rows(through, deadline_boot_ms, operation)
			.await?
			.into_iter()
			.map(|row| depot_fs_core::wire::decode_row(row).map_err(Error::from))
			.collect()
	}
}
impl ColdSnapshot for ClaimSnapshot {
	fn position(&self) -> &VolumeLogPosition {
		&self.floor
	}

	fn get_inode(
		&self,
		ino: u64,
		through: u64,
		deadline_boot_ms: u64,
	) -> BoxFuture<'_, Result<Option<super::Inode>>> {
		Box::pin(async move {
			let rows = self
				.wire_rows(through, deadline_boot_ms, |at| {
					FsReadOperation::FsReadGet(FsReadGet {
						at,
						key: RowKey::Inode(ino).into(),
					})
				})
				.await?;
			if rows.len() > 1 {
				return Err(corrupt!());
			}
			rows.into_iter()
				.next()
				.map(|row| {
					let change_seq = row.change_seq.ok_or_else(|| corrupt!())?;
					if change_seq > through {
						return Err(corrupt!());
					}
					let (key, value) = depot_fs_core::wire::decode_row(row)?;
					match (key, value) {
						(RowKey::Inode(found), Row::Inode(inode)) if found == ino => {
							Ok(super::Inode { inode, change_seq })
						}
						_ => Err(corrupt!()),
					}
				})
				.transpose()
		})
	}

	fn get(
		&self,
		key: RowKey,
		through: u64,
		deadline_boot_ms: u64,
	) -> BoxFuture<'_, Result<Option<Row>>> {
		Box::pin(async move {
			let mut rows = self
				.rows(through, deadline_boot_ms, |at| {
					FsReadOperation::FsReadGet(FsReadGet {
						at,
						key: key.clone().into(),
					})
				})
				.await?;
			if rows.len() > 1 || rows.first().is_some_and(|(found, _)| found != &key) {
				return Err(corrupt!());
			}
			Ok(rows.pop().map(|(_, row)| row))
		})
	}

	fn chunks(
		&self,
		ino: u64,
		ranges: Vec<(u64, u32)>,
		through: u64,
		deadline_boot_ms: u64,
	) -> BoxFuture<'_, Result<Vec<(RowKey, Row)>>> {
		Box::pin(async move {
			let wire_ranges: Vec<FsChunkRange> = ranges
				.iter()
				.map(|&(first, count)| FsChunkRange { first, count })
				.collect();
			let rows = self
				.rows(through, deadline_boot_ms, |at| {
					FsReadOperation::FsReadChunks(FsReadChunks {
						at,
						ino,
						ranges: wire_ranges,
					})
				})
				.await?;
			// Rows come back in range order, ascending within each range, and inside the
			// ranges asked for.
			let mut remaining = ranges.iter().copied();
			let mut current = remaining.next();
			let mut previous = None;
			for (key, _) in &rows {
				let RowKey::Chunk(found, index) = key else {
					return Err(corrupt!());
				};
				if *found != ino {
					return Err(corrupt!());
				}
				loop {
					let Some((first, count)) = current else {
						return Err(corrupt!());
					};
					let end = first.saturating_add(u64::from(count));
					if (first..end).contains(index) {
						if previous.is_some_and(|previous| previous >= *index) {
							return Err(corrupt!());
						}
						previous = Some(*index);
						break;
					}
					current = remaining.next();
					previous = None;
				}
			}
			Ok(rows)
		})
	}

	fn scan(
		&self,
		range: RowRange,
		after: Option<RowKey>,
		limit: usize,
		through: u64,
		deadline_boot_ms: u64,
	) -> BoxFuture<'_, Result<Vec<(RowKey, Row)>>> {
		Box::pin(async move {
			let wire_limit: u32 = limit.try_into().map_err(|_| Error::TooLarge)?;
			let rows = self
				.rows(through, deadline_boot_ms, |at| {
					FsReadOperation::FsReadScan(FsReadScan {
						at,
						range: range.clone().into(),
						after: after.clone().map(Into::into),
						limit: wire_limit,
					})
				})
				.await?;
			let mut previous = after.as_ref();
			if rows.len() > limit {
				return Err(corrupt!());
			}
			for (key, _) in &rows {
				if !range.contains(key) || previous.is_some_and(|old| old >= key) {
					return Err(corrupt!());
				}
				previous = Some(key);
			}
			Ok(rows)
		})
	}
}
/// Sends one read with its caller's deadline and returns its answer. A refused cold read
/// returns as [`Error::ColdReadRefused`] instead of being resent here: the edge owns retries of
/// a user request, and the refusal has already refreshed the manifest so the edge's next
/// attempt follows placement. Every read the claim holder sends can wait on the maintainer
/// until it has applied the position the read names, so each carries its caller's deadline
/// instead of the transport's shorter timeout.
async fn routed_read(
	transport: &dyn ReadTransport,
	clock: &dyn Clock,
	request: FsReadRequest,
	deadline: u64,
) -> Result<FsReadResponse> {
	let now = clock.observe().map_err(|_| Error::Unavailable)?.boot_ms;
	if now >= deadline {
		return Err(Error::DeadlineExceeded);
	}
	let read = transport.open_at(request, deadline);
	rivet_env::timeout_at(clock, deadline, read)
		.await
		.map_err(|_| Error::DeadlineExceeded)?
		.and_then(checked)
}

fn checked(response: FsReadResponse) -> Result<FsReadResponse> {
	match response {
		FsReadResponse::FsReadError(error) => Err(match error {
			FsReadError::TooOld => Error::TooOld,
			FsReadError::Lagging => Error::Lagging,
			FsReadError::Fenced => Error::Fenced,
			FsReadError::TooLarge => Error::TooLarge,
			FsReadError::Overloaded => {
				crate::metrics::FS_OVERLOADED
					.with_label_values(&["maintainer_read"])
					.inc();
				Error::Overloaded
			}
			FsReadError::NotFound => depot_fs_core::Error::NotFound.into(),
			FsReadError::Corrupt => corrupt!(),
			FsReadError::Unavailable => Error::Unavailable,
			FsReadError::BaseUnavailable => Error::BaseUnavailable,
			FsReadError::NotServing => Error::ColdReadRefused,
		}),
		response => Ok(response),
	}
}


