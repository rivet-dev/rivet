//! Bounded client admission while the supervisor refreshes a newer replica-set view.

use super::{LogShardPhase, LogShardState, loaded_volume_replica::admit, memory_batches::reject};
use crate::log_shard_machine::{
	Effects,
	effect::LogShardEffect,
	types::{
		Authority, LoadedVolumeReplicaCommand, LogShardCommand, LogShardRequest, LogShardTimer,
		RequestId,
	},
};
use depot_types::partition_key_hash;
use rivet_data::generated::depot_storage_worker_protocol_v1::{RejectResponse, RequestTooLarge};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Requests waiting for a newer routing view, in arrival order, with their retained bytes kept
/// in step so admission reads the total without re-encoding every waiting request.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct ManifestWaits {
	requests: VecDeque<LogShardRequest>,
	bytes: u64,
}

impl ManifestWaits {
	pub(crate) fn len(&self) -> usize {
		self.requests.len()
	}

	pub(crate) fn is_empty(&self) -> bool {
		self.requests.is_empty()
	}

	/// Retained bytes of every waiting request.
	pub(crate) fn bytes(&self) -> u64 {
		self.bytes
	}

	pub(crate) fn iter(&self) -> impl Iterator<Item = &LogShardRequest> {
		self.requests.iter()
	}

	fn push_back(&mut self, request: LogShardRequest) {
		self.bytes = self.bytes.saturating_add(request_bytes(&request));
		self.requests.push_back(request);
	}

	fn remove(&mut self, id: RequestId) -> Option<LogShardRequest> {
		let index = self
			.requests
			.iter()
			.position(|request| request.request_id == id)?;
		let request = self.requests.remove(index)?;
		self.bytes = self.bytes.saturating_sub(request_bytes(&request));
		Some(request)
	}

	pub(crate) fn clear(&mut self) {
		self.requests.clear();
		self.bytes = 0;
	}

	fn take(&mut self) -> VecDeque<LogShardRequest> {
		self.bytes = 0;
		std::mem::take(&mut self.requests)
	}
}

/// Missing routing information also needs a refresh; a known older client is rejected immediately.
pub(crate) fn needed(state: &LogShardState, request: &LogShardRequest) -> bool {
	if state.phase != LogShardPhase::Serving {
		return false;
	}
	let LogShardCommand::Replica {
		volume_log_key,
		authority: Some(Authority::Client {
			tablet_replica_set_version,
			..
		}),
		command,
		..
	} = &request.command
	else {
		return false;
	};
	if matches!(command, LoadedVolumeReplicaCommand::Read { .. }) {
		return false;
	}
	let hash = partition_key_hash(&volume_log_key.0.volume_key);
	super::retirement::covering_tablet(state, hash)
		.is_none_or(|tablet| *tablet_replica_set_version > tablet.tablet_replica_set_version)
}

/// This ephemeral encoding measures the retained command, not stored or wire data.
fn request_bytes(request: &LogShardRequest) -> u64 {
	let mut count = ByteCount(0);
	serde_bare::to_writer(&mut count, request).expect("serializable shard request");
	count.0
}

pub(crate) fn bytes(state: &LogShardState) -> u64 {
	state.manifest_waits.bytes()
}

/// A waiting request owns a slot and bytes under the same limits as reads and disk writes.
pub(crate) fn hold(state: &mut LogShardState, request: LogShardRequest, effects: &mut Effects) {
	let pending = state
		.memory_batches
		.pending()
		.map(|batch| batch.waiting_replies().len())
		.sum::<usize>()
		+ state.reads.len()
		+ state.manifest_waits.len();
	if pending >= state.config.max_memory_requests as usize {
		reject(
			request.request_id,
			super::admission::overloaded(state),
			effects,
		);
		return;
	}
	let bytes = request_bytes(&request);
	if bytes > state.config.max_memory_bytes {
		reject(
			request.request_id,
			RejectResponse::RequestTooLarge(RequestTooLarge {
				max_bytes: state.config.max_memory_bytes.min(u32::MAX as u64) as u32,
			}),
			effects,
		);
		return;
	}
	if bytes
		> state
			.config
			.max_memory_bytes
			.saturating_sub(super::reads::occupied_bytes(state))
	{
		reject(
			request.request_id,
			super::admission::overloaded(state),
			effects,
		);
		return;
	}

	if state.manifest_waits.is_empty() {
		effects.execute(LogShardEffect::RefreshTabletManifest);
	}
	effects.arm_timer_after(
		LogShardTimer::ManifestRefresh(request.request_id),
		state.config.manifest_refresh_timeout_ms,
	);
	state.manifest_waits.push_back(request);
}

/// Re-admit in arrival order, retaining later-version requests without resetting their deadlines.
pub(crate) fn resume(
	state: &mut LogShardState,
	rng: &mut rivet_env::rand::LocalRng,
	effects: &mut Effects,
) {
	let ready: Vec<_> = state
		.manifest_waits
		.iter()
		.filter(|request| !needed(state, request))
		.map(|request| request.request_id)
		.collect();
	for id in ready {
		let request = state
			.manifest_waits
			.remove(id)
			.expect("selected retained request");
		effects.cancel_timer(LogShardTimer::ManifestRefresh(id));
		admit(state, request, rng, effects);
	}
}

/// A missing new view is unavailability, not evidence that the client should downgrade its replica set.
pub(crate) fn expired(state: &mut LogShardState, id: RequestId, effects: &mut Effects) {
	if state.manifest_waits.remove(id).is_some() {
		reject(id, RejectResponse::NotServing, effects);
	}
}

/// Shutdown cannot leave requests waiting for a view the supervisor may no longer refresh.
pub(crate) fn drain(state: &mut LogShardState, effects: &mut Effects) {
	for request in state.manifest_waits.take() {
		effects.cancel_timer(LogShardTimer::ManifestRefresh(request.request_id));
		reject(request.request_id, RejectResponse::NotServing, effects);
	}
}

/// Counting avoids allocating a second copy of queued append payloads during admission.
struct ByteCount(u64);

impl std::io::Write for ByteCount {
	fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
		self.0 = self.0.saturating_add(bytes.len() as u64);
		Ok(bytes.len())
	}

	fn flush(&mut self) -> std::io::Result<()> {
		Ok(())
	}
}
