//! Replica status notices pushed to tablet maintainers.
//!
//! When a batch syncs, every replica it changed is marked for the maintainer worker of
//! its tablet. The newest state is read when a notice is built, so changes before the
//! send coalesce. Notices are batched per maintainer worker, not per volume or per tablet:
//! one request carries every marked replica of every tablet that worker maintains, up to
//! `MAX_NOTICES_PER_REQUEST`, and at most one request is in flight to each worker. Marks
//! made while a request is in flight, or within one disk write, share the next request, so
//! under load the request rate follows the disk write rate, not the write rate.
//!
//! Each request carries a `notice_id` and arms one `NoticeDeadline` for its worker. A
//! request that fails, times out, misses its deadline or is refused marks its replicas
//! again and is resent after a backoff that doubles per consecutive failure, so a
//! maintainer never has to poll an idle volume to catch a lost notice. The reply names the
//! tablets the worker could not queue and the ones it does not run; only their replicas
//! are resent, and a tablet the worker does not run also refreshes this shard's routing
//! view. A routing view that names a new maintainer for a tablet marks every loaded
//! replica of that tablet for it, which also covers the first view after the shard starts.
//!
//! The notice of one member per tablet, its entry carrier, also carries the entries its
//! replica synced since the replica's previous notice, when they fit
//! `max_notice_entry_bytes`, so the maintainer applies a small write without reading the
//! replica once a quorum of notices proves it committed. The other members' notices carry
//! positions only: those count toward the quorum, and a copy of the same bytes from each
//! would only be dropped. Carried entries are an optimization only: the maintainer takes
//! them when they continue its applied position and a quorum holds them, and otherwise
//! reads a replica as before, as it does when the carrier's notice is late or lost.
//!
//! A replica whose synced batch only appended entries is noticed after
//! `append_notice_delay_ms`, so the appends of a burst share one notice. No client
//! announces commits: the maintainer derives them from the appended positions and claims the
//! notices carry, so the delay bounds how long a write waits before it is applied.

use super::LogShardState;
use crate::log_shard_machine::types::{LogShardTimer, NoticeOutcome};
use crate::log_shard_machine::{Effects, effect::LogShardEffect};
use depot_types::{VolumeLogKey, partition_key_hash};
use rivet_data::generated::{
	depot_client_protocol_v1::{
		ReplicaNoticeColdWrite, ReplicaStatusNotice, ReplicaStatusNoticeRequest,
		ReplicaStatusNoticeTablet,
	},
	depot_log_shard_v1::{VolumeLogEntry, VolumeLogReplicaState},
	depot_storage_worker_protocol_v1::StorageWorkerResponse,
	shared_depot_types_v1::VolumeLogReplicaStatus,
};
use rivet_state_machine::IndexedDeadlineQueue;
use rivet_topology::worker::WorkerId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Most notices in one request to one maintainer worker.
pub(crate) const MAX_NOTICES_PER_REQUEST: usize = 256;

/// Backoff after the first failed notice to a maintainer worker.
const NOTICE_RETRY_BASE_MS: u64 = 500;

/// Longest backoff between notices to a maintainer worker that keeps failing, so a worker
/// that is gone costs one request per this interval until the routing view drops it.
const NOTICE_RETRY_MAX_MS: u64 = 30_000;

/// The backoff of a maintainer worker whose last notice failed.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct NoticeBackoff {
	/// When the next notice may go out, or `None` once the wait has passed and the next
	/// notice may be sent.
	until_boot_ms: Option<u64>,
	/// Consecutive failed notices, which double the wait.
	failures: u32,
}

/// The notice request in flight to one maintainer worker.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct InFlightNotice {
	/// Matches the response and the deadline to this request.
	notice_id: u64,
	/// The tablet and replica of each notice it carries, so the ones it did not deliver can
	/// be marked again.
	notices: Vec<([u8; 16], VolumeLogKey)>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct StatusNotices {
	/// Replicas whose state changed since their last notice, per maintainer worker.
	/// Bounded by the loaded replicas: a set per worker cannot grow past them.
	dirty: BTreeMap<WorkerId, BTreeSet<VolumeLogKey>>,
	/// Maintainer workers with a notice in flight. Bounded by the maintainer workers named
	/// in the routing view, and each by `MAX_NOTICES_PER_REQUEST` notices.
	in_flight: BTreeMap<WorkerId, InFlightNotice>,
	/// The `notice_id` of the next request.
	next_notice_id: u64,
	/// Maintainer workers whose last notice failed. An entry is removed when a notice to
	/// the worker is fully queued or the routing view stops naming the worker, so it is
	/// bounded by the maintainer workers named in the view.
	backoff: BTreeMap<WorkerId, NoticeBackoff>,
	/// The time `LogShardTimer::NoticeRetry` is armed for.
	retry_armed_at: Option<u64>,
	/// Whether the disk is under cold-write pressure. It turns on at the watermark and off
	/// only below nine tenths of it; see `observe_disk_pressure`.
	disk_pressure: bool,
	/// Replicas whose only synced change since their last mark is appended entries, by
	/// the time they are noticed anyway. Bounded by the loaded replicas.
	appended: IndexedDeadlineQueue<VolumeLogKey>,
	/// The time `LogShardTimer::AppendNotices` is armed for.
	appended_armed_at: Option<u64>,
}

impl StatusNotices {
	/// Replicas waiting for a notice, for the pending gauge.
	pub(crate) fn pending(&self) -> usize {
		self.dirty.values().map(BTreeSet::len).sum::<usize>() + self.appended.len()
	}

	/// Replica notices not yet delivered to a maintainer: pending or in flight. A replica
	/// marked again while its notice is in flight counts twice.
	pub(crate) fn undelivered(&self) -> usize {
		self.pending()
			+ self
				.in_flight
				.values()
				.map(|notice| notice.notices.len())
				.sum::<usize>()
	}
}

/// The tablet holding `key` in the routing view, with its maintainer worker.
fn tablet_of(state: &LogShardState, key: &VolumeLogKey) -> Option<([u8; 16], WorkerId)> {
	let hash = partition_key_hash(&key.0.volume_key);
	super::retirement::covering_tablet(state, hash).and_then(|tablet| {
		tablet
			.maintainer
			.clone()
			.map(|maintainer| (tablet.tablet_id, maintainer))
	})
}

/// The one member whose notices carry a tablet's synced entries: the maintainer's own
/// worker when it is a member, so the entries never cross the network, and otherwise the
/// member with the lowest worker ID. Every member picks it from its own routing view with
/// no coordination, the same view that routes the notice, and the maintainer reads the
/// same member first when it falls back to a read. A stale view costs only a read: carried
/// entries never decide what the maintainer applies.
fn entry_carrier<'a>(members: &'a [WorkerId], maintainer: &WorkerId) -> Option<&'a WorkerId> {
	members
		.iter()
		.find(|member| *member == maintainer)
		.or_else(|| members.iter().min())
}

/// Whether this shard's notices for `key` carry its synced entries; see [`entry_carrier`].
fn carries_entries(state: &LogShardState, key: &VolumeLogKey, maintainer: &WorkerId) -> bool {
	let hash = partition_key_hash(&key.0.volume_key);
	super::retirement::covering_tablet(state, hash)
		.is_some_and(|tablet| entry_carrier(&tablet.members, maintainer) == Some(&state.worker_id))
}

/// Marks replicas whose state a synced batch changed. A replica of a tablet with no
/// maintainer is skipped.
pub(crate) fn changed(state: &mut LogShardState, keys: impl IntoIterator<Item = VolumeLogKey>) {
	for key in keys {
		state.status_notices.appended.cancel(&key);
		if let Some((_, maintainer)) = tablet_of(state, &key) {
			state
				.status_notices
				.dirty
				.entry(maintainer)
				.or_default()
				.insert(key);
		}
	}
	observe_pending(state);
}

/// Marks replicas whose synced batch only appended entries, to be noticed after
/// `append_notice_delay_ms` unless a state change notices them first. A replica already
/// marked keeps its earlier mark.
pub(crate) fn appended(
	state: &mut LogShardState,
	keys: impl IntoIterator<Item = VolumeLogKey>,
	now_boot_ms: u64,
) {
	let at = now_boot_ms.saturating_add(state.config.append_notice_delay_ms);
	for key in keys {
		if state.config.append_notice_delay_ms == 0 {
			changed(state, [key]);
			continue;
		}
		let dirty = tablet_of(state, &key).is_some_and(|(_, maintainer)| {
			state
				.status_notices
				.dirty
				.get(&maintainer)
				.is_some_and(|keys| keys.contains(&key))
		});
		if !dirty {
			state.status_notices.appended.schedule_earliest(key, at);
		}
	}
	observe_pending(state);
}

/// Marks every append-only replica whose delay has passed.
pub(crate) fn appended_due(state: &mut LogShardState, now_boot_ms: u64) {
	state.status_notices.appended_armed_at = None;
	loop {
		let due = state
			.status_notices
			.appended
			.pop_due(now_boot_ms, MAX_NOTICES_PER_REQUEST);
		if due.is_empty() {
			break;
		}
		changed(state, due.into_iter().map(|(_, key)| key));
	}
}

/// Keeps `LogShardTimer::AppendNotices` armed at the earliest append-only mark and
/// `LogShardTimer::NoticeRetry` at the earliest backoff while the shard serves.
pub(crate) fn arm(state: &mut LogShardState, effects: &mut Effects) {
	let serving = match state.phase {
		super::LogShardPhase::Serving | super::LogShardPhase::Draining => true,
		super::LogShardPhase::Recovering(_)
		| super::LogShardPhase::Failed { .. }
		| super::LogShardPhase::Stopped => false,
	};
	let notices = &mut state.status_notices;

	let wanted = notices.appended.next_at().filter(|_| serving);
	if wanted != notices.appended_armed_at {
		match wanted {
			Some(at) => effects.arm_timer_at(LogShardTimer::AppendNotices, at),
			None => effects.cancel_timer(LogShardTimer::AppendNotices),
		}
		notices.appended_armed_at = wanted;
	}

	let retry = notices
		.backoff
		.values()
		.filter_map(|backoff| backoff.until_boot_ms)
		.min()
		.filter(|_| serving);
	if retry != notices.retry_armed_at {
		match retry {
			Some(at) => effects.arm_timer_at(LogShardTimer::NoticeRetry, at),
			None => effects.cancel_timer(LogShardTimer::NoticeRetry),
		}
		notices.retry_armed_at = retry;
	}
}

/// Ends the backoff of every maintainer worker due by `now_boot_ms`.
pub(crate) fn retry_due(state: &mut LogShardState, now_boot_ms: u64) {
	let notices = &mut state.status_notices;
	notices.retry_armed_at = None;
	for backoff in notices.backoff.values_mut() {
		if backoff.until_boot_ms.is_some_and(|at| at <= now_boot_ms) {
			backoff.until_boot_ms = None;
		}
	}
}

/// Marks every loaded replica of the tablets whose maintainer the new routing view
/// changed, for the new maintainer. It runs once per view change that moves a maintainer,
/// including the first view after the shard starts, so a maintainer learns every state
/// change the previous one may have missed without polling.
pub(crate) fn maintainers_changed(
	state: &mut LogShardState,
	previous: &BTreeMap<[u8; 16], Option<WorkerId>>,
) {
	let named = state
		.tablet_manifest
		.tablets
		.values()
		.filter_map(|tablet| tablet.maintainer.clone())
		.collect::<BTreeSet<_>>();
	state
		.status_notices
		.backoff
		.retain(|worker, _| named.contains(worker));
	let moved = state.tablet_manifest.tablets.iter().any(|(id, tablet)| {
		tablet.maintainer.is_some() && previous.get(id) != Some(&tablet.maintainer)
	});
	if !moved {
		return;
	}
	let keys = state
		.loaded_volume_replicas
		.keys()
		.filter(|key| {
			let hash = partition_key_hash(&key.0.volume_key);
			super::retirement::covering_tablet(state, hash).is_some_and(|tablet| {
				tablet.maintainer.is_some()
					&& previous.get(&tablet.tablet_id) != Some(&tablet.maintainer)
			})
		})
		.cloned()
		.collect::<Vec<_>>();
	changed(state, keys);
}

/// Whether the disk is under cold-write pressure, as reported in statuses and notices.
pub(crate) fn disk_pressure(state: &LogShardState) -> bool {
	state.status_notices.disk_pressure
}

/// Updates the disk pressure after the disk's usage changed. Pressure turns on when usage
/// reaches the watermark and off only once it falls below nine tenths of the watermark, so a
/// disk held at the watermark does not flip every write. A change alters the cold-write
/// status only of replicas that retain entries above cold, so only those are marked.
pub(crate) fn observe_disk_pressure(state: &mut LogShardState) {
	let disk_bytes = state.disk_segments.disk_bytes();
	let watermark = u128::from(state.config.max_disk_bytes)
		* u128::from(state.config.write_to_cold_at_disk_percent)
		/ 100;
	let pressure = if state.status_notices.disk_pressure {
		u128::from(disk_bytes) * 10 >= watermark * 9
	} else {
		u128::from(disk_bytes) >= watermark
	};
	if pressure == state.status_notices.disk_pressure {
		return;
	}
	state.status_notices.disk_pressure = pressure;
	let max_disk_bytes = state.config.max_disk_bytes;
	if pressure {
		tracing::warn!(
			disk_bytes,
			max_disk_bytes,
			watermark_percent = state.config.write_to_cold_at_disk_percent,
			"log shard disk is near its capacity; volumes write to cold early, and appends fail with disk_full at the cap. Check cold-store throughput or raise storage.log_shard.max_disk_bytes"
		);
	} else {
		tracing::info!(
			disk_bytes,
			max_disk_bytes,
			"log shard disk is back below nine tenths of its watermark"
		);
	}
	let keys = state
		.payloads
		.keys_with_entries(&state.loaded_volume_replicas);
	changed(state, keys);
}

fn notice(
	state: &LogShardState,
	key: &VolumeLogKey,
	disk_pressure: bool,
) -> Option<ReplicaStatusNotice> {
	// A replica excluded by retirement is not served; the maintainer learns that by poll.
	if super::retirement::excluded(state, key) {
		return None;
	}
	let replica = state.loaded_volume_replicas.get(key);
	let disk = replica.and_then(|replica| replica.disk_state.clone());
	let cold_write = match (replica, &disk) {
		(Some(replica), Some(disk)) if disk.status != VolumeLogReplicaStatus::Deleted => replica
			.cold_write_status(disk, disk_pressure)
			.map(|status| ReplicaNoticeColdWrite {
				first_write_unix_ms: status.first_write_unix_ms,
				committed_bytes: status.committed_bytes,
				disk_pressure: status.disk_pressure,
			}),
		(Some(_), Some(_)) | (Some(_), None) | (None, Some(_)) | (None, None) => None,
	};
	Some(ReplicaStatusNotice {
		volume_log_key: key.0.clone(),
		state: disk,
		cold_write,
		entries: Vec::new(),
	})
}

/// The entries `key`'s replica synced since its previous notice, exactly as a read starting
/// at the first of them would return them, or nothing when they exceed `max_bytes`, start
/// below cold or inside released payloads, or include an unsynced entry. A shorter page that
/// still starts at the first entry is returned as is; the maintainer reads the rest.
fn newly_synced(
	state: &LogShardState,
	key: &VolumeLogKey,
	disk: &VolumeLogReplicaState,
	max_bytes: u64,
) -> Vec<VolumeLogEntry> {
	let Some(replica) = state.loaded_volume_replicas.get(key) else {
		return Vec::new();
	};
	let synced = disk.appended.volume_seq;
	// Payloads released from memory form a prefix of the retained entries. A replica whose
	// earlier notices skipped their entries may still owe its notice an older entry whose
	// payload went to disk only; the maintainer usually applied that one already, so the
	// notice starts at the first payload still in memory instead of carrying nothing.
	let first_in_memory = replica.entries().front().map_or(0, |front| {
		front.volume_seq.saturating_add(replica.released() as u64)
	});
	let first = replica
		.noticed_volume_seq
		.max(disk.cold.volume_seq)
		.saturating_add(1)
		.max(first_in_memory);
	if disk.status != VolumeLogReplicaStatus::InSync || first > synced || max_bytes == 0 {
		return Vec::new();
	}
	let max_bytes = u32::try_from(max_bytes).unwrap_or(u32::MAX);
	match super::reads::synced_page(state, key, disk, first, max_bytes) {
		Ok(StorageWorkerResponse::ReadVolumeLogEntriesResponse(page)) => page
			.entries
			.into_iter()
			.take_while(|entry| entry.volume_seq <= synced)
			.collect(),
		Ok(_) | Err(_) => Vec::new(),
	}
}

/// Sends one notice to every maintainer worker with marked replicas and none in flight.
pub(crate) fn dispatch(state: &mut LogShardState, effects: &mut Effects) {
	let ready = state
		.status_notices
		.dirty
		.keys()
		.filter(|worker| {
			!state.status_notices.in_flight.contains_key(*worker)
				&& state
					.status_notices
					.backoff
					.get(*worker)
					.is_none_or(|backoff| backoff.until_boot_ms.is_none())
		})
		.cloned()
		.collect::<Vec<_>>();
	if ready.is_empty() {
		return;
	}
	let disk_pressure = super::reads::disk_pressure(state);
	for worker in ready {
		let Some(dirty) = state.status_notices.dirty.get_mut(&worker) else {
			continue;
		};
		let mut keys = Vec::new();
		while keys.len() < MAX_NOTICES_PER_REQUEST
			&& let Some(key) = dirty.pop_first()
		{
			keys.push(key);
		}
		if dirty.is_empty() {
			state.status_notices.dirty.remove(&worker);
		}
		let mut tablets = BTreeMap::<[u8; 16], Vec<ReplicaStatusNotice>>::new();
		let mut sent = Vec::new();
		let mut entry_budget = state.config.max_notice_request_entry_bytes;
		for key in keys {
			let Some((tablet_id, maintainer)) = tablet_of(state, &key) else {
				continue;
			};
			// A tablet that moved to another maintainer is told by the routing change.
			if maintainer != worker {
				continue;
			}
			if let Some(mut notice) = notice(state, &key, disk_pressure) {
				if let Some(disk) = &notice.state {
					// Only the tablet's carrier attaches entries. The other members send
					// positions only, which still count toward the commit quorum.
					if carries_entries(state, &key, &maintainer) {
						let budget =
							u64::from(state.config.max_notice_entry_bytes).min(entry_budget);
						notice.entries = newly_synced(state, &key, disk, budget);
					}
					let carried = notice
						.entries
						.iter()
						.map(depot_types::log_shard::entry_bytes)
						.sum::<u64>();
					entry_budget = entry_budget.saturating_sub(carried);
					let synced = disk.appended.volume_seq;
					if let Some(replica) = state.loaded_volume_replicas.get_mut(&key) {
						replica.noticed_volume_seq = synced;
					}
				}
				tablets.entry(tablet_id).or_default().push(notice);
				sent.push((tablet_id, key));
			}
		}
		if tablets.is_empty() {
			continue;
		}
		let tablets = tablets
			.into_iter()
			.map(|(tablet_id, notices)| ReplicaStatusNoticeTablet {
				tablet_id,
				tablet_grant_epoch: state.tablet_grants.get(&tablet_id).copied().unwrap_or(0),
				tablet_replica_set_version: state
					.tablet_replica_sets
					.get(&tablet_id)
					.map(|set| set.tablet_replica_set_version)
					.or_else(|| {
						state
							.tablet_manifest
							.tablets
							.get(&tablet_id)
							.map(|tablet| tablet.tablet_replica_set_version)
					})
					.unwrap_or(0),
				notices,
			})
			.collect();
		crate::metrics::record_status_notice_replicas(sent.len());
		let notice_id = state.status_notices.next_notice_id;
		state.status_notices.next_notice_id = notice_id.wrapping_add(1);
		state.status_notices.in_flight.insert(
			worker.clone(),
			InFlightNotice {
				notice_id,
				notices: sent,
			},
		);
		effects.arm_timer_after(
			LogShardTimer::NoticeDeadline(worker.clone()),
			state.config.notice_timeout_ms,
		);
		effects.execute(LogShardEffect::NotifyMaintainerRequest {
			worker: worker.clone(),
			notice_id,
			request: ReplicaStatusNoticeRequest {
				worker_id: *state.worker_id.as_bytes(),
				tablets,
			},
		});
	}
	observe_pending(state);
}

/// A notice finished and its slot is free. A failed, timed out or refused notice marks its
/// replicas again, routed by the current view, and holds the worker's next notice for a
/// backoff; a notice queued except for some tablets marks only those tablets' replicas.
/// Resending the newest state is idempotent: a maintainer applies a notice as a status
/// reply, which only replaces an observation. A response to a notice that is no longer in
/// flight, because its deadline ended it, changes nothing.
pub(crate) fn notified(
	state: &mut LogShardState,
	worker: WorkerId,
	notice_id: u64,
	outcome: NoticeOutcome,
	effects: &mut Effects,
) {
	let current = state
		.status_notices
		.in_flight
		.get(&worker)
		.is_some_and(|notice| notice.notice_id == notice_id);
	if !current {
		return;
	}
	effects.cancel_timer(LogShardTimer::NoticeDeadline(worker.clone()));
	finish(state, worker, outcome, effects);
}

/// The notice in flight to `worker` reached its deadline without a response, so it ends
/// as timed out.
pub(crate) fn deadline_due(state: &mut LogShardState, worker: WorkerId, effects: &mut Effects) {
	if state.status_notices.in_flight.contains_key(&worker) {
		finish(state, worker, NoticeOutcome::Timeout, effects);
	}
}

/// Ends the notice in flight to `worker` with `outcome`.
fn finish(
	state: &mut LogShardState,
	worker: WorkerId,
	outcome: NoticeOutcome,
	effects: &mut Effects,
) {
	crate::metrics::record_status_notice(&outcome);
	let now_boot_ms = effects.now_boot_ms();
	let notices = state
		.status_notices
		.in_flight
		.remove(&worker)
		.map(|notice| notice.notices)
		.unwrap_or_default();
	let (resend, refresh) = match outcome {
		NoticeOutcome::Sent {
			refused_tablets,
			unknown_tablets,
		} if refused_tablets.is_empty() && unknown_tablets.is_empty() => {
			state.status_notices.backoff.remove(&worker);
			return;
		}
		NoticeOutcome::Sent {
			refused_tablets,
			unknown_tablets,
		} => {
			let resend = notices
				.into_iter()
				.filter(|(tablet_id, _)| {
					refused_tablets.contains(tablet_id) || unknown_tablets.contains(tablet_id)
				})
				.map(|(_, key)| key)
				.collect::<Vec<_>>();
			(resend, !unknown_tablets.is_empty())
		}
		NoticeOutcome::NotMaintainer => (notices.into_iter().map(|(_, key)| key).collect(), true),
		NoticeOutcome::Failed | NoticeOutcome::Timeout | NoticeOutcome::Refused => {
			(notices.into_iter().map(|(_, key)| key).collect(), false)
		}
	};
	// A worker that does not run a tablet the view names for it means the view is stale.
	// The refreshed view marks every replica of a moved tablet for its new maintainer.
	if refresh {
		effects.execute(LogShardEffect::RefreshTabletManifest);
	}
	// A worker the view no longer names gets no backoff entry: the resent replicas route to
	// the tablets' current maintainers, and the entry would outlive the view.
	let named = state
		.tablet_manifest
		.tablets
		.values()
		.any(|tablet| tablet.maintainer.as_ref() == Some(&worker));
	if !named {
		changed(state, resend);
		return;
	}
	let backoff = state
		.status_notices
		.backoff
		.entry(worker)
		.or_insert(NoticeBackoff {
			until_boot_ms: None,
			failures: 0,
		});
	let wait = NOTICE_RETRY_BASE_MS
		.saturating_mul(1 << backoff.failures.min(16))
		.min(NOTICE_RETRY_MAX_MS);
	backoff.failures = backoff.failures.saturating_add(1);
	backoff.until_boot_ms = Some(now_boot_ms.saturating_add(wait));
	changed(state, resend);
}

fn observe_pending(state: &LogShardState) {
	crate::metrics::observe_status_notices_pending(state.status_notices.pending());
}


