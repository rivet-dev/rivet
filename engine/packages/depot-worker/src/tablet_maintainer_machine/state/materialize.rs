//! Bounded sparse copying is a nested maintainer activity. Peer reads never own
//! the ordered store slot. Pages and their cursor ride the tablet's ordinary passes, and
//! only a pass that covers the last page permits the release of the source's hold.
use super::{MaintainerPhase, MaintainerState, Operation, store::StoreWork};
use crate::tablet_maintainer_machine::{
	Effects,
	effect::MaintainerEffect,
	types::{MaintainerTimer, OpId},
};
use depot_fs_core::{
	CHUNK_BYTES, FsMaterializedChunk,
	forks::{ChunkLookup, Materialization, materialize::*},
	keys::slatedb::Scope,
};
use rivet_data::generated::{
	depot_client_protocol_v1::{ClientRequest, ClientResponse},
	shared_depot_fs_read_v1::*,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Materializations {
	jobs: BTreeMap<Scope, Phase>,
	/// Materializations this lifetime finished whose pages no pass has covered yet, with
	/// the dispatch count a covering pass must exceed. Their source releases wait.
	uncovered: BTreeMap<Scope, u64>,
	/// Volumes whose pending materialization discovery found while every job was taken.
	/// Bounded by `max_loaded_volumes`.
	skipped: super::pending_work::Skipped,
	/// Consecutive failed source reads per scope, which set the next retry's backoff. An
	/// entry stays until a successful read, an idle or complete result, or a one-page scan of
	/// its volume that finds no pending row removes it. A row gone before a multi-page or
	/// full scan leaves its entry, without a timer, until the maintainer fences or stops.
	retries: BTreeMap<Scope, u32>,
}

/// The longest wait before a failed source read is tried again.
const MAX_RETRY_MS: u64 = 60_000;
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum Phase {
	/// Waiting for the shared ordered store slot.
	Queued(Action),
	/// Its local effect is retained in Operations until storage acknowledges it.
	Storing,
	/// Earlier captured layers may have completed and released older sources.
	Probing { work: Materialization, layer: u32 },
	/// Enumerating one sparse source layer after the durable cursor.
	Keys(Materialization),
	/// Resolving one inode's candidate chunks nearest-first; completed groups
	/// remain in this bounded page until all chunks can be durably copied.
	Chunks(Resolution),
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct Resolution {
	work: Materialization,
	chunks: Vec<FsMaterializedChunk>,
	complete: bool,
	group_end: usize,
	layer: usize,
	/// Page slots still unresolved, with the tightest inherited-byte cap.
	pending: Vec<(usize, usize)>,
}

/// Whether `scope` finished materializing in this lifetime and no pass has covered its
/// pages yet, so its source must keep its hold.
pub(crate) fn uncovered(state: &MaintainerState, scope: &Scope) -> bool {
	state.materializations.uncovered.contains_key(scope)
}

/// A pass completed: the materializations it covers may release their sources now.
pub(crate) fn passes_completed(state: &mut MaintainerState) {
	let completed = state.maintainer_log.passes_completed;
	let covered = state
		.materializations
		.uncovered
		.iter()
		.filter(|(_, through)| completed > **through)
		.map(|(scope, _)| scope.clone())
		.collect::<Vec<_>>();
	for scope in covered {
		state.materializations.uncovered.remove(&scope);
		let work = depot_fs_core::forks::cleanup::Work {
			scope,
			kind: depot_fs_core::forks::cleanup::Kind::Release,
		};
		let _ = super::fork_cleanup::discover(state, work);
	}
}

/// Starts the materialization of `scope` found by discovery. One found while every job is
/// taken stays durable and its volume is scanned again when a job ends.
pub(crate) fn discover(state: &mut MaintainerState, scope: Scope) -> Result<(), ()> {
	if !state.tablet.hash_range.contains(scope.hash) {
		return Err(());
	}
	if state.phase != MaintainerPhase::Serving || state.materializations.jobs.contains_key(&scope) {
		return Ok(());
	}
	let volume = depot_fs_core::work::PendingVolume::of_scope(&scope);
	if !has_free_job(state) {
		let cap = state.config.max_loaded_volumes as usize;
		state.materializations.skipped.skip(volume, cap);
		return Ok(());
	}
	state.materializations.skipped.started(&volume);
	queue(state, scope, Action::Inspect);
	Ok(())
}

/// Whether a discovered materialization would get a job now.
pub(crate) fn has_free_job(state: &MaintainerState) -> bool {
	state.materializations.jobs.len() < state.config.max_concurrent_materializations as usize
}

pub(crate) fn skipped(state: &mut MaintainerState) -> &mut super::pending_work::Skipped {
	&mut state.materializations.skipped
}

/// A source read of `scope` failed or was refused and its job ended. Nothing else re-reads
/// its pending row while the tablet is quiet, so a deadline with exponential backoff hands
/// the volume back to discovery.
fn arm_retry(state: &mut MaintainerState, scope: &Scope, fx: &mut Effects) {
	let attempts = state
		.materializations
		.retries
		.entry(scope.clone())
		.or_insert(0);
	let delay_ms = state
		.config
		.tuning()
		.retry_ms
		.saturating_mul(1u64 << (*attempts).min(16))
		.min(MAX_RETRY_MS);
	*attempts = attempts.saturating_add(1);
	fx.arm_timer_after(MaintainerTimer::MaterializeRetry(scope.clone()), delay_ms);
}

/// `scope`'s retry deadline passed: its volume is scanned now when a job is free, else waits
/// with the work skipped for want of a job.
pub(crate) fn retry_due(state: &mut MaintainerState, scope: Scope, fx: &mut Effects) {
	if state.phase != MaintainerPhase::Serving || state.materializations.jobs.contains_key(&scope) {
		return;
	}
	let volume = depot_fs_core::work::PendingVolume::of_scope(&scope);
	if has_free_job(state) {
		super::pending_work::scan_volume(state, volume, "materialization_retry", fx);
	} else {
		let cap = state.config.max_loaded_volumes as usize;
		state.materializations.skipped.skip(volume, cap);
	}
}

/// Forgets `scope`'s failed reads and cancels its retry deadline.
fn retried(state: &mut MaintainerState, scope: &Scope, fx: &mut Effects) {
	if state.materializations.retries.remove(scope).is_some() {
		fx.cancel_timer(MaintainerTimer::MaterializeRetry(scope.clone()));
	}
}

/// A scan read all of `volume`'s pending rows and found the materializations in `found`.
/// A scope of that volume waiting to retry whose row is gone needs no retry.
pub(crate) fn volume_scanned(
	state: &mut MaintainerState,
	volume: &depot_fs_core::work::PendingVolume,
	found: &[Scope],
	fx: &mut Effects,
) {
	let first = Scope {
		hash: volume.hash,
		name: volume.name.clone(),
		incarnation: uuid::Uuid::nil(),
	};
	let last = Scope {
		incarnation: uuid::Uuid::max(),
		..first.clone()
	};
	let gone = state
		.materializations
		.retries
		.range(first..=last)
		.map(|(scope, _)| scope.clone())
		.filter(|scope| !found.contains(scope) && !state.materializations.jobs.contains_key(scope))
		.collect::<Vec<_>>();
	for scope in gone {
		retried(state, &scope, fx);
	}
}

/// The maintainer fenced: its materialization work and retry deadlines end with it.
pub(crate) fn fence(state: &mut MaintainerState, fx: &mut Effects) {
	let ended = std::mem::take(&mut state.materializations);
	for scope in ended.retries.into_keys() {
		fx.cancel_timer(MaintainerTimer::MaterializeRetry(scope));
	}
}

/// Hands one skipped volume back to discovery if `scope`'s job ended.
fn settle(state: &mut MaintainerState, scope: &Scope, fx: &mut Effects) {
	if state.materializations.jobs.contains_key(scope) {
		return;
	}
	super::pending_work::Skipped::job_ended(state, skipped, "materialization_job", fx);
}
fn queue(state: &mut MaintainerState, scope: Scope, action: Action) {
	state
		.materializations
		.jobs
		.insert(scope.clone(), Phase::Queued(action));
	state
		.store_queue
		.push_back(StoreWork::Materialize { scope });
}
pub(crate) fn dispatch(state: &mut MaintainerState, scope: Scope, fx: &mut Effects) {
	if state.phase != MaintainerPhase::Serving {
		state.materializations.jobs.remove(&scope);
		return;
	}
	let Some(Phase::Queued(action)) = state.materializations.jobs.get(&scope) else {
		return;
	};
	let action = action.clone();
	state
		.materializations
		.jobs
		.insert(scope.clone(), Phase::Storing);
	let op_id = state.begin(
		Operation::MaterializeStore {
			scope: scope.clone(),
		},
		fx,
	);
	fx.execute(MaintainerEffect::MaterializeRequest {
		op_id,
		scope,
		action,
	});
}
pub(crate) fn stored(
	state: &mut MaintainerState,
	id: OpId,
	result: Result<Progress, String>,
	fx: &mut Effects,
) {
	if !matches!(
		state.operations.get(&id),
		Some(Operation::MaterializeStore { .. })
	) {
		return;
	}
	let Some(Operation::MaterializeStore { scope }) = state.finish(id, fx) else {
		unreachable!()
	};
	stored_job(state, scope.clone(), result, fx);
	settle(state, &scope, fx);
}

fn stored_job(
	state: &mut MaintainerState,
	scope: Scope,
	result: Result<Progress, String>,
	fx: &mut Effects,
) {
	if state.materializations.jobs.remove(&scope) != Some(Phase::Storing) {
		super::fence(state, fx);
		return;
	}
	match result {
		Ok(Progress::Idle) => retried(state, &scope, fx),
		Ok(Progress::Complete(record)) => {
			if record.volume_incarnation != *scope.incarnation.as_bytes()
				|| Scope::new(&record.volume_key(), scope.incarnation) != scope
				|| !record.base_chain.is_empty()
			{
				super::fence(state, fx);
				return;
			}
			retried(state, &scope, fx);
			// The released chain rides the next pass with the copied pages; nothing is
			// flushed or trimmed for it.
			let key = depot_types::VolumeLogKey(
				rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
					volume_key: record.volume_key(),
					volume_incarnation: record.volume_incarnation,
				},
			);
			if let Some(loaded) = state.loaded_volumes.get_mut(&key) {
				if let Some(creation) = &mut loaded.creation {
					creation.row.base_chain.clear();
				}
				if let Some(open) = &mut loaded.unacknowledged_cold_open {
					open.base_chain.clear();
				}
			}
			super::filesystem::chain_released(state, &key, fx);
			let through = super::maintainer_log::wrote_uncovered(state, fx);
			state.materializations.uncovered.insert(scope, through);
		}
		Ok(Progress::Ready(work)) => {
			if Scope::new(
				&work.record.volume_key(),
				uuid::Uuid::from_bytes(work.record.volume_incarnation),
			) != scope || work.record.base_chain.len() > depot_fs_core::MAX_FORK_CHAIN_DEPTH
				|| work.cursor.layer as usize >= work.record.base_chain.len()
			{
				super::fence(state, fx);
				return;
			}
			probe(state, scope, work, 0, fx);
		}
		Err(_) => super::fence(state, fx),
	}
}
fn request(
	state: &mut MaintainerState,
	scope: Scope,
	link: depot_fs_core::FsBaseLink,
	operation: FsReadOperation,
	fx: &mut Effects,
) {
	if state.phase != MaintainerPhase::Serving {
		state.materializations.jobs.remove(&scope);
		return;
	}
	let op_id = state.begin(Operation::MaterializePeer { scope }, fx);
	fx.execute(MaintainerEffect::ForkPeerRequest {
		op_id,
		region: link.region.unwrap_or(state.config.local_region),
		request: ClientRequest::FsReadRequest(FsReadRequest {
			target: FsReadTarget::VolumeLogKey(
				rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
					volume_key: link.volume_key,
					volume_incarnation: link.incarnation,
				},
			),
			client_epoch: None,
			operation,
		}),
		deadline_boot_ms: fx
			.now_boot_ms()
			.saturating_add(state.config.tuning().operation_timeout_ms),
	});
}
fn probe(
	state: &mut MaintainerState,
	scope: Scope,
	work: Materialization,
	layer: u32,
	fx: &mut Effects,
) {
	let link = work.record.base_chain[layer as usize].clone();
	state
		.materializations
		.jobs
		.insert(scope.clone(), Phase::Probing { work, layer });
	request(
		state,
		scope,
		link.clone(),
		FsReadOperation::FsBaseKeysRequest(FsBaseKeysRequest {
			link,
			after: None,
			limit: 1,
		}),
		fx,
	);
}
fn limit(state: &MaintainerState) -> u32 {
	state
		.config
		.materialize_batch_chunks
		.min(state.config.tuning().max_rows_per_pass)
		.min(state.config.tuning().max_cold_read_bytes / (CHUNK_BYTES as u32 + 256))
		.max(1)
}
fn keys(state: &mut MaintainerState, scope: Scope, work: Materialization, fx: &mut Effects) {
	let link = work.record.base_chain[work.cursor.layer as usize].clone();
	let operation = FsReadOperation::FsBaseKeysRequest(FsBaseKeysRequest {
		link: link.clone(),
		after: work.cursor.after.clone(),
		limit: limit(state),
	});
	state
		.materializations
		.jobs
		.insert(scope.clone(), Phase::Keys(work));
	request(state, scope, link, operation, fx);
}
fn resolve_next(state: &mut MaintainerState, scope: Scope, mut page: Resolution, fx: &mut Effects) {
	if page.pending.is_empty() {
		if page.group_end == page.chunks.len() {
			queue(
				state,
				scope,
				Action::Copy {
					expected: page.work.cursor,
					chunks: page.chunks,
					complete: page.complete,
				},
			);
			return;
		}
		let start = page.group_end;
		let ino = page.chunks[start].position.ino;
		while page.group_end < page.chunks.len() && page.chunks[page.group_end].position.ino == ino
		{
			page.group_end += 1;
		}
		page.pending = (start..page.group_end)
			.map(|slot| (slot, CHUNK_BYTES))
			.collect();
		page.layer = 0;
	}
	let Some(link) = page.work.record.base_chain.get(page.layer).cloned() else {
		super::fence(state, fx);
		return;
	};
	let ino = page.chunks[page.pending[0].0].position.ino;
	let indices = page
		.pending
		.iter()
		.map(|(slot, _)| page.chunks[*slot].position.index)
		.collect();
	state
		.materializations
		.jobs
		.insert(scope.clone(), Phase::Chunks(page));
	request(
		state,
		scope,
		link.clone(),
		FsReadOperation::FsBaseChunksRequest(FsBaseChunksRequest { link, ino, indices }),
		fx,
	);
}
pub(crate) fn peer(
	state: &mut MaintainerState,
	id: OpId,
	result: Result<ClientResponse, String>,
	fx: &mut Effects,
) {
	if !matches!(
		state.operations.get(&id),
		Some(Operation::MaterializePeer { .. })
	) {
		return;
	}
	let Some(Operation::MaterializePeer { scope }) = state.finish(id, fx) else {
		unreachable!()
	};
	peer_job(state, scope.clone(), result, fx);
	settle(state, &scope, fx);
}

fn peer_job(
	state: &mut MaintainerState,
	scope: Scope,
	result: Result<ClientResponse, String>,
	fx: &mut Effects,
) {
	let Some(phase) = state.materializations.jobs.remove(&scope) else {
		super::fence(state, fx);
		return;
	};
	// Remote absence/unavailability is not data loss. The durable cursor and hold remain,
	// and a retry deadline hands the scope's volume back to discovery, which resumes it.
	let Ok(ClientResponse::FsReadResponse(response)) = result else {
		arm_retry(state, &scope, fx);
		return;
	};
	if matches!(response, FsReadResponse::FsReadError(_)) {
		arm_retry(state, &scope, fx);
		return;
	}
	retried(state, &scope, fx);
	match (phase, response) {
		(Phase::Probing { work, layer }, FsReadResponse::FsBaseKeysPage(page)) => {
			if page.positions.len() > 1 || page.positions.is_empty() && !page.complete {
				super::fence(state, fx);
				return;
			}
			if page.complete_image && (layer < work.cursor.layer || !work.cursor.complete_image) {
				queue(
					state,
					scope,
					Action::Restart {
						expected: work.cursor,
						layer,
					},
				);
			} else if layer < work.cursor.layer {
				probe(state, scope, work, layer + 1, fx);
			} else if work.cursor.complete_image && !page.complete_image {
				super::fence(state, fx);
			} else {
				keys(state, scope, work, fx);
			}
		}
		(Phase::Keys(work), FsReadResponse::FsBaseKeysPage(page)) => {
			if page.positions.len() > limit(state) as usize
				|| page.positions.is_empty() && !page.complete
				|| page
					.positions
					.windows(2)
					.any(|p| position_key(&p[0]) >= position_key(&p[1]))
				|| page
					.positions
					.first()
					.zip(work.cursor.after.as_ref())
					.is_some_and(|(first, after)| position_key(first) <= position_key(after))
			{
				super::fence(state, fx);
				return;
			}
			if page.complete_image && !work.cursor.complete_image {
				queue(
					state,
					scope,
					Action::Restart {
						layer: work.cursor.layer,
						expected: work.cursor,
					},
				);
				return;
			}
			if work.cursor.complete_image && !page.complete_image {
				super::fence(state, fx);
				return;
			}
			resolve_next(
				state,
				scope,
				Resolution {
					work,
					complete: page.complete,
					chunks: page
						.positions
						.into_iter()
						.map(|position| FsMaterializedChunk {
							position,
							bytes: None,
						})
						.collect(),
					group_end: 0,
					layer: 0,
					pending: Vec::new(),
				},
				fx,
			);
		}
		(Phase::Chunks(mut page), FsReadResponse::FsBaseChunksResponse(response)) => {
			if response.chunks.len() != page.pending.len() {
				super::fence(state, fx);
				return;
			}
			let mut pending = Vec::new();
			for ((slot, cap), value) in page.pending.into_iter().zip(response.chunks) {
				match ChunkLookup::try_from(value) {
					Ok(ChunkLookup::Resolved(Some(mut bytes))) if bytes.len() <= CHUNK_BYTES => {
						bytes.truncate(cap);
						page.chunks[slot].bytes = (!bytes.is_empty()).then_some(bytes.into());
					}
					Ok(ChunkLookup::Resolved(None)) => {}
					Ok(ChunkLookup::Base { max_bytes })
						if max_bytes > 0 && max_bytes <= CHUNK_BYTES =>
					{
						pending.push((slot, cap.min(max_bytes)))
					}
					_ => {
						super::fence(state, fx);
						return;
					}
				}
			}
			page.pending = pending;
			page.layer += 1;
			resolve_next(state, scope, page, fx);
		}
		_ => super::fence(state, fx),
	}
}


