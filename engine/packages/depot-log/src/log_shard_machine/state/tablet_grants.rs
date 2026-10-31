//! Tablet fences belong to the shard; replicas retain no tablet ownership.

use super::{
	LogShardPhase, LogShardState,
	memory_batches::{self, PendingReply, WaitingReply},
};
use crate::log_shard_machine::{
	Effects,
	effect::LogShardEffect,
	types::{Authority, RequestId},
};
use depot_types::{VolumeLogKey, partition_key_hash};
use rivet_data::generated::{
	depot_log_shard_v1::{Frame, StateFrame, TabletGrantFrame},
	depot_storage_worker_protocol_v1::{
		self as protocol, ListTabletReplicasRequest, ListTabletReplicasResponse, RejectResponse,
		StorageWorkerResponse, TabletGrant,
	},
};
use std::collections::BTreeMap;
use vbare::Encode;

/// Retirement is permanent: no ordinary grant can exceed this sentinel. Even a
/// request carrying the sentinel is refused, so the parent can never serve again.
pub const RETIRED_TABLET_GRANT_EPOCH: u64 = u64::MAX;

/// Replays a monotonic fence, shared by live disk completion and startup recovery.
pub(crate) fn replay(grants: &mut BTreeMap<[u8; 16], u64>, frame: &TabletGrantFrame) {
	let epoch = grants.entry(frame.tablet_id).or_default();
	*epoch = (*epoch).max(frame.tablet_grant_epoch);
}

/// Pending frames fence immediately; only synced frames enter checkpoints.
pub(crate) fn effective_epoch(state: &LogShardState, tablet: &[u8; 16]) -> u64 {
	let synced = state.tablet_grants.get(tablet).copied().unwrap_or(0);
	state
		.memory_batches
		.pending_grant_epoch(tablet)
		.map_or(synced, |pending| pending.max(synced))
}

/// Refuses a superseded or permanently retired tablet, independently of the key
/// and manifest view. A higher grant is not acknowledged until its frame syncs.
pub(crate) fn check_grant(
	state: &LogShardState,
	grant: &TabletGrant,
) -> Result<(), RejectResponse> {
	let epoch = effective_epoch(state, &grant.tablet_id);
	if epoch == RETIRED_TABLET_GRANT_EPOCH
		|| grant.tablet_grant_epoch < epoch
		|| grant.tablet_grant_epoch == RETIRED_TABLET_GRANT_EPOCH
	{
		return Err(RejectResponse::StaleTabletGrantEpoch(
			protocol::StaleTabletGrantEpoch {
				tablet_grant_epoch: epoch,
			},
		));
	}
	Ok(())
}

/// Encodes one tablet fence without attaching it to a volume replica.
fn grant_frame(grant: &TabletGrant) -> Frame {
	Frame::TabletGrantFrame(TabletGrantFrame {
		tablet_id: grant.tablet_id,
		tablet_grant_epoch: grant.tablet_grant_epoch,
	})
}

/// Validates wire admission and returns the fence frames the command must include
/// atomically with its own frames. Maintainers never use the manifest for fencing.
pub(crate) fn check_authority(
	state: &LogShardState,
	key: &VolumeLogKey,
	authority: &Authority,
) -> Result<Vec<Frame>, RejectResponse> {
	match authority {
		Authority::Grant(grant) | Authority::GrantedClient { grant, .. } => {
			check_grant(state, grant)?;
			Ok(
				if grant.tablet_grant_epoch > effective_epoch(state, &grant.tablet_id) {
					vec![grant_frame(grant)]
				} else {
					Vec::new()
				},
			)
		}
		Authority::Client {
			tablet_replica_set_version,
			..
		} => {
			let hash = partition_key_hash(&key.0.volume_key);
			let tablet = super::retirement::covering_tablet(state, hash)
				.ok_or(RejectResponse::NotServing)?;
			let replica_set = super::tablet_replica_sets::effective_ref(state, tablet);
			if *tablet_replica_set_version != replica_set.version() {
				return Err(RejectResponse::StaleTabletReplicaSet(
					protocol::StaleTabletReplicaSet {
						replica_set: replica_set.to_set(),
					},
				));
			}
			// A lost disk cannot act as an empty voting member under its old assignment.
			if !tablet
				.disks
				.iter()
				.any(|disk| disk.disk_incarnation == state.disk_incarnation)
			{
				return Err(RejectResponse::DiskIncarnationMismatch(
					protocol::DiskIncarnationMismatch {
						disk_incarnation: state.disk_incarnation,
					},
				));
			}
			Ok(super::tablet_replica_sets::frames(state, tablet))
		}
	}
}

/// Bounds grant frames with the same request, memory, write and disk budgets as
/// replica commands. No fence changes until all reservations succeed.
pub(crate) fn reserve(state: &LogShardState, frames: &[Frame]) -> Result<u64, RejectResponse> {
	if state
		.memory_batches
		.pending()
		.map(|b| b.waiting_replies().len())
		.sum::<usize>()
		+ state.reads.len()
		+ state.manifest_waits.len()
		>= state.config.max_memory_requests as usize
	{
		return Err(super::admission::overloaded(state));
	}

	// Charge encoded bytes across the filling buffer and outstanding write.
	let frame_sizes: Vec<u64> = frames.iter().map(|f| f.encoded_len() as u64).collect();
	let bytes = frame_sizes.iter().sum::<u64>();
	let occupied = super::reads::occupied_bytes(state);
	if bytes > state.config.max_memory_bytes.saturating_sub(occupied) {
		return Err(super::admission::overloaded(state));
	}

	// Listings and background fence scans share the startup write cap with replica work.
	let size = state
		.memory_batches
		.filling
		.size_with(&frame_sizes, state.config.max_batch_bytes)
		.map_err(|_| {
			RejectResponse::RequestTooLarge(protocol::RequestTooLarge {
				max_bytes: state.config.max_batch_bytes,
			})
		})?;
	let startup_limit = u64::from(state.config.max_batch_bytes)
		+ depot_types::log_shard::BATCH_FRAMING_BYTES as u64;
	let max_write = if state.memory_batches.write_bytes_per_sec.is_none() {
		state.config.max_write_bytes.min(startup_limit)
	} else {
		state.config.max_write_bytes
	};
	if size > max_write {
		return Err(super::admission::overloaded(state));
	}
	let pending = state
		.memory_batches
		.writing_to_disk
		.as_ref()
		.map_or(0, |w| w.pending.sizing.total());
	if !frames.is_empty()
		&& state.disk_segments.disk_bytes().saturating_add(
			super::disk_segments::DiskSegments::pending_disk_bytes(pending.saturating_add(size)),
		) > state.config.max_disk_bytes
	{
		return Err(super::admission::disk_full());
	}
	Ok(bytes)
}

/// Retires the parent and records the child's grant in the same disk write before
/// returning any page. A retry waits for pending fences too; it cannot acknowledge
/// an unsynced retirement. The range never changes which tablet a grant fences.
pub(crate) fn list(
	state: &mut LogShardState,
	request_id: RequestId,
	request: ListTabletReplicasRequest,
	rng: &mut rivet_env::rand::LocalRng,
	effects: &mut Effects,
) {
	let admitted = (|| {
		if state.phase != LogShardPhase::Serving {
			return Err(RejectResponse::NotServing);
		}
		check_grant(state, &request.tablet_grant)?;
		// A self-retirement is a malformed caller request, never transient unavailability.
		if request.retired_parent == Some(request.tablet_grant.tablet_id) {
			return Err(RejectResponse::NotServing);
		}

		// Both fences enter one filling buffer and are acknowledged by one disk barrier.
		let mut frames = if request.tablet_replica_set_version == 0 {
			Vec::new()
		} else {
			let frames = super::tablet_replica_sets::listing_frames(
				state,
				&request.tablet_grant.tablet_id,
				request.tablet_replica_set_version,
			);
			if matches!(&frames, Err(RejectResponse::NotServing)) {
				// The caller retries the same barrier; prompt its missing view without
				// retaining the request or waiting for the periodic manifest refresh. A
				// tablet this view lacks or holds at an older replica set means the manifest
				// moved past this shard, as for a split child published moments ago.
				effects.execute(LogShardEffect::RefreshTabletManifest);
			}
			frames?
		};
		if let Some(parent) = request.retired_parent
			&& effective_epoch(state, &parent) != RETIRED_TABLET_GRANT_EPOCH
		{
			frames.push(Frame::TabletGrantFrame(TabletGrantFrame {
				tablet_id: parent,
				tablet_grant_epoch: RETIRED_TABLET_GRANT_EPOCH,
			}));
		}
		if request.tablet_grant.tablet_grant_epoch
			> effective_epoch(state, &request.tablet_grant.tablet_id)
		{
			frames.push(grant_frame(&request.tablet_grant));
		}

		// Capacity rejection leaves both the parent and child epochs unchanged.
		let bytes = reserve(state, &frames)?;
		Ok((frames, bytes))
	})();
	match admitted {
		Ok((frames, bytes)) => {
			if let Err(error) = state
				.memory_batches
				.filling
				.push_frames(frames, state.config.max_batch_bytes)
			{
				super::fail(state, error.to_string(), effects);
				return;
			}
			state.memory_batches.filling.encoded_bytes += bytes;
			state.memory_batches.filling.push_reply(WaitingReply {
				request_id,
				reply: PendingReply::LoadedVolumeReplicas { request },
			});
		}
		// The maintainer retries the listing on its own timer, so the refusal stands.
		Err(reason) => memory_batches::reject(
			request_id,
			super::admission::jittered(state, reason, rng),
			effects,
		),
	}
}

/// Builds the page from synced replicas after rechecking the latest grant fence.
/// The canonical key cursor is exclusive; the hash interval is inclusive/exclusive.
pub(crate) fn reply_listing(
	state: &LogShardState,
	request_id: RequestId,
	request: ListTabletReplicasRequest,
	effects: &mut Effects,
) {
	let response = check_grant(state, &request.tablet_grant).and_then(|()| {
		if super::replica_hashes::any_excluded(state, &request.hash_range) {
			return Err(RejectResponse::NotServing);
		}
		let disk_version = state
			.tablet_replica_sets
			.get(&request.tablet_grant.tablet_id)
			.map_or(0, |set| set.tablet_replica_set_version);
		if disk_version < request.tablet_replica_set_version {
			return Err(RejectResponse::NotServing);
		}
		// Pages follow listing order, partition-key hash then key, starting after the cursor.
		let after = request.after.map(VolumeLogKey::from);
		let replicas = state
			.replica_hashes
			.listing(&request.hash_range, after.as_ref())
			.filter_map(|key| {
				let replica = state.loaded_volume_replicas.get(key)?;
				replica.disk_state.clone().map(|state| StateFrame {
					volume_log_key: key.clone().into(),
					state,
				})
			})
			.take(request.max_replicas.min(state.config.max_loaded_volumes) as usize)
			.collect();
		Ok(ListTabletReplicasResponse {
			tablet_replica_set_version: disk_version,
			disk_incarnation: state.disk_incarnation,
			replicas,
		})
	});
	effects.execute(LogShardEffect::Reply {
		request_id,
		response: match response {
			Ok(page) => StorageWorkerResponse::ListTabletReplicasResponse(page),
			Err(reason) => StorageWorkerResponse::RejectResponse(reason),
		},
	});
}


