//! Read admission and completion use current lifecycle facts; I/O never chooses policy.

use super::{MaintainerPhase, MaintainerState, Operation, fence, lifecycle, store::StoreWork};
use crate::tablet_maintainer_machine::{
	Effects,
	effect::MaintainerEffect,
	types::{MaintainerTimer, OpId, RequestId},
};
use depot_types::VolumeLogKey;
use rivet_data::generated::{
	depot_client_protocol_v1::ClientResponse, shared_depot_fs_read_v1::*,
	shared_depot_types_v1::VolumeLogPosition,
};

/// How a filesystem query shares the tablet store with other store work. It is
/// fixed at dispatch and retained in the operation after the caller expires, so a
/// query never changes class just because its reply is no longer awaited.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub(crate) enum QueryOrder {
	/// A session open or a point read resolves the current record and applied position,
	/// then pins a new snapshot. It must observe every store mutation ordered before it,
	/// so it waits behind earlier queued store work and never overlaps a mutation. Opens
	/// may overlap each other and session reads within the read cap.
	Ordered,
	/// A get, scan, chunk read or close on an existing session. Rows come from the
	/// snapshot pinned at open, which already had to stay valid across arbitrary
	/// later mutations, so these reads overlap anything within the read cap.
	Session,
	/// A claim holder's read without a session. Once this maintainer has applied the read's
	/// floor, it runs in the session lane, with the per-volume share, against the applied
	/// position and record captured at dispatch, so it never waits behind store work.
	///
	/// It needs no exclusion from applies, loads or other writers of its volume. It reads at
	/// an `as_of` no later than the captured applied position, so rows a concurrent apply
	/// writes, which are all above it, are hidden from it. Its one-request session registers
	/// its pin under the catalog lock, which rejects an `as_of` below the retention cold
	/// position there even at a checkpoint, and nothing collects a version visible at or
	/// above that cold position or at a registered pin, so a concurrent flush or sweep cannot
	/// remove a row the read sees. It takes no volume stripe, so a flush stage stalled in its
	/// writes does not hold it up. A record change the captured record misses, such as a
	/// delete or a new incarnation, is caught by the completion's lifecycle check before the
	/// reply.
	Claim,
	/// A cross-volume source read against a held image; bounded separately and
	/// rejected rather than queued when its window is full.
	HeldImage,
}

/// Bounded metric label for a filesystem read operation.
fn operation_label(operation: &FsReadOperation) -> &'static str {
	match operation {
		FsReadOperation::FsReadOpen(_) => "open",
		FsReadOperation::FsReadClose(_) => "close",
		FsReadOperation::FsReadKeys(_) => "point",
		FsReadOperation::FsReadRaw(_) => "raw",
		FsReadOperation::FsReadGet(_)
		| FsReadOperation::FsReadScan(_)
		| FsReadOperation::FsReadChunks(_) => match claim_read(operation) {
			Some(_) => "claim",
			None => "read",
		},
		FsReadOperation::FsBaseChunksRequest(_)
		| FsReadOperation::FsForkMetadataRequest(_)
		| FsReadOperation::FsBaseKeysRequest(_) => "other",
	}
}

/// The claim read a row read makes, or `None` for a session read or any other operation.
fn claim_read(operation: &FsReadOperation) -> Option<&FsClaimRead> {
	let at = match operation {
		FsReadOperation::FsReadGet(read) => &read.at,
		FsReadOperation::FsReadScan(read) => &read.at,
		FsReadOperation::FsReadChunks(read) => &read.at,
		FsReadOperation::FsReadOpen(_)
		| FsReadOperation::FsReadClose(_)
		| FsReadOperation::FsBaseChunksRequest(_)
		| FsReadOperation::FsForkMetadataRequest(_)
		| FsReadOperation::FsBaseKeysRequest(_)
		| FsReadOperation::FsReadKeys(_)
		| FsReadOperation::FsReadRaw(_) => return None,
	};
	match at {
		FsReadAt::FsClaimRead(read) => Some(read),
		FsReadAt::FsSessionRead(_) => None,
	}
}

/// The highest floor of the claim reads waiting for this maintainer to apply `key`. A claim
/// holder takes its floor only from committed positions: the volume's cold, an applied
/// position a maintainer answered, which only proven entries reach, or its own acknowledged
/// commits. The floor therefore proves its entry committed even when the members hold an
/// older epoch's tail under the holder's claim, which proves nothing on its own. They do
/// after a maintainer move when the holder's claim settled on that tail and the holder has
/// not written since: a settled claim appends no claim entry, so without this the new
/// maintainer would never apply what the holder's read waits for.
pub(crate) fn waiting_claim_floor<'a>(
	state: &'a MaintainerState,
	key: &VolumeLogKey,
) -> Option<&'a VolumeLogPosition> {
	state
		.lagging_opens
		.get(key)?
		.iter()
		.filter_map(|(_, id)| state.pending_fs_reads.get(id))
		.filter_map(|request| claim_read(&request.operation))
		.map(|read| &read.floor)
		.max_by_key(|floor| (floor.volume_seq, floor.client_epoch))
}

/// Classifies a request by the store state it can observe.
pub(crate) fn query_order(request: &FsReadRequest) -> QueryOrder {
	if depot_fs_core::wire::source_link(&request.operation).is_some() {
		return QueryOrder::HeldImage;
	}
	if claim_read(&request.operation).is_some() {
		return QueryOrder::Claim;
	}
	match &request.operation {
		FsReadOperation::FsReadOpen(_)
		| FsReadOperation::FsReadKeys(_)
		| FsReadOperation::FsReadRaw(_) => QueryOrder::Ordered,
		FsReadOperation::FsReadGet(_)
		| FsReadOperation::FsReadScan(_)
		| FsReadOperation::FsReadChunks(_)
		| FsReadOperation::FsReadClose(_) => match request.target {
			FsReadTarget::VolumeLogKey(_) => QueryOrder::Session,
			// The store rejects a logical target here; keep it on the ordered path.
			FsReadTarget::VolumeKey(_) => QueryOrder::Ordered,
		},
		FsReadOperation::FsBaseChunksRequest(_)
		| FsReadOperation::FsForkMetadataRequest(_)
		| FsReadOperation::FsBaseKeysRequest(_) => QueryOrder::Ordered,
	}
}

/// Exact reads keep their incarnation. Latest opens may use a loaded, acknowledged
/// record; unloaded volumes are resolved by the ordered store operation.
fn target_key(state: &MaintainerState, request: &FsReadRequest) -> Option<VolumeLogKey> {
	if let Some(key) = request.volume_log_key() {
		return Some(VolumeLogKey(key.clone()));
	}
	state
		.loaded_volumes
		.range(super::incarnations(request.volume_key()))
		.map(|(key, _)| key)
		.find(|key| !super::deletion::is_replaced(state, key))
		.cloned()
}

/// Whether `operation` reads the latest applied snapshot, the only reads a logical volume
/// target may name.
fn opens_latest(operation: &FsReadOperation) -> bool {
	match operation {
		FsReadOperation::FsReadOpen(FsReadOpen { position, .. }) => position.is_none(),
		FsReadOperation::FsReadKeys(FsReadKeys { position, .. }) => position.is_none(),
		FsReadOperation::FsReadRaw(FsReadRaw { position, .. }) => position.is_none(),
		FsReadOperation::FsReadGet(_)
		| FsReadOperation::FsReadScan(_)
		| FsReadOperation::FsReadChunks(_)
		| FsReadOperation::FsReadClose(_)
		| FsReadOperation::FsBaseChunksRequest(_)
		| FsReadOperation::FsForkMetadataRequest(_)
		| FsReadOperation::FsBaseKeysRequest(_) => false,
	}
}

/// The position a session open or point read pins, or the floor a claim read needs applied,
/// if it names one.
fn pinned_position(operation: &FsReadOperation) -> Option<&VolumeLogPosition> {
	if let Some(read) = claim_read(operation) {
		return Some(&read.floor);
	}
	match operation {
		FsReadOperation::FsReadOpen(FsReadOpen { position, .. })
		| FsReadOperation::FsReadKeys(FsReadKeys { position, .. })
		| FsReadOperation::FsReadRaw(FsReadRaw { position, .. }) => position.as_ref(),
		FsReadOperation::FsReadGet(_)
		| FsReadOperation::FsReadScan(_)
		| FsReadOperation::FsReadChunks(_)
		| FsReadOperation::FsReadClose(_)
		| FsReadOperation::FsBaseChunksRequest(_)
		| FsReadOperation::FsForkMetadataRequest(_)
		| FsReadOperation::FsBaseKeysRequest(_) => None,
	}
}

/// The position a read waits for this maintainer to apply before it dispatches: an exact
/// session open's position or a claim read's floor.
fn applied_wait(operation: &FsReadOperation) -> Option<&VolumeLogPosition> {
	if let Some(read) = claim_read(operation) {
		return Some(&read.floor);
	}
	match operation {
		FsReadOperation::FsReadOpen(FsReadOpen { position, .. }) => position.as_ref(),
		FsReadOperation::FsReadKeys(_)
		| FsReadOperation::FsReadRaw(_)
		| FsReadOperation::FsReadGet(_)
		| FsReadOperation::FsReadScan(_)
		| FsReadOperation::FsReadChunks(_)
		| FsReadOperation::FsReadClose(_)
		| FsReadOperation::FsBaseChunksRequest(_)
		| FsReadOperation::FsForkMetadataRequest(_)
		| FsReadOperation::FsBaseKeysRequest(_) => None,
	}
}

/// The retryable refusal a frozen split parent answers a read it cannot serve, counted.
fn frozen_refusal() -> FsReadError {
	crate::metrics::TABLET_FS_READ_REJECTED
		.with_label_values(&["tablet_frozen"])
		.inc();
	FsReadError::NotServing
}

fn check(state: &MaintainerState, request: &FsReadRequest) -> Result<(), FsReadError> {
	// A maintainer that stopped serving cannot judge the reader's client epoch, so it refuses
	// the read as retryable. Answering `Fenced` would make a healthy writer reopen its volume.
	if !matches!(
		state.phase,
		MaintainerPhase::Serving | MaintainerPhase::Frozen
	) {
		return Err(FsReadError::NotServing);
	}
	if let Some(link) = depot_fs_core::wire::source_link(&request.operation) {
		// A held image's checkpoint or hold may still be in a recovered maintainer log op;
		// the reader retries once replay applied it. A fork or restore resuming its replayed
		// publication copies an image whose replay already reached it, and the pending op it
		// would wait for is its own.
		if super::maintainer_log::replay::pending_for(state, &link.volume_key)
			&& !super::forks::resumes_replayed_fork(state, link)
		{
			return Err(FsReadError::Overloaded);
		}
		return if depot_fs_core::wire::valid_source_target(request)
			&& link
				.region
				.is_none_or(|region| region == state.config.local_region)
		{
			Ok(())
		} else {
			Err(FsReadError::Corrupt)
		};
	}
	// Closing an exact UUID only releases its hold; lifecycle retirement and a
	// newer writer must not prevent cleanup. The store still validates its scope.
	if matches!(request.operation, FsReadOperation::FsReadClose(_)) {
		return Ok(());
	}
	if request.volume_log_key().is_none()
		&& (request.client_epoch.is_some() || !opens_latest(&request.operation))
	{
		return Err(FsReadError::Corrupt);
	}
	// Only the volume's exclusive client makes claim reads, and its epoch is what fences them.
	if request.client_epoch.is_none() && claim_read(&request.operation).is_some() {
		return Err(FsReadError::Corrupt);
	}
	// A frozen split parent applies nothing more, so the newest position it holds is its
	// store's cold, while the volume's log, which only the split children apply, may run
	// past it. It cannot tell a reader what the latest position is, so a latest open or point
	// read is refused as retryable and follows placement to the child. An exact position at or
	// below cold is still served from the store, as `dispatch` and `completed` decide.
	let frozen = state.phase == MaintainerPhase::Frozen;
	if frozen && opens_latest(&request.operation) {
		return Err(frozen_refusal());
	}
	if state.pending.contains_key(request.volume_key())
		|| super::deletion::ambiguous(state, request.volume_key())
		|| super::maintainer_log::replay::pending_for(state, request.volume_key())
	{
		// A frozen parent drives no deletion or replay to completion, so this never clears
		// in its lifetime; the reader follows placement instead of retrying here.
		if frozen {
			return Err(frozen_refusal());
		}
		crate::metrics::TABLET_FS_READ_REJECTED
			.with_label_values(&["volume_opening"])
			.inc();
		return Err(FsReadError::Overloaded);
	}
	let Some(key) = target_key(state, request) else {
		return Ok(());
	};
	if state.deleting.contains_key(&key)
		|| state
			.loaded_volumes
			.get(&key)
			.is_some_and(|v| v.is_deleted())
	{
		return Err(FsReadError::NotFound);
	}
	if state
		.loaded_volumes
		.range(super::incarnations(&key.0.volume_key))
		.any(|(other, _)| other != &key && !super::deletion::is_replaced(state, other))
	{
		// Another incarnation is current here. A frozen parent cannot prove the one asked
		// for absent: the split children may have created it after the freeze.
		if frozen {
			return Err(frozen_refusal());
		}
		return Err(FsReadError::NotFound);
	}
	if state
		.loaded_volumes
		.get(&key)
		.is_some_and(|v| v.creation.as_ref().is_some_and(|c| !c.acknowledged))
	{
		crate::metrics::TABLET_FS_READ_REJECTED
			.with_label_values(&["creation_unacknowledged"])
			.inc();
		return Err(FsReadError::Overloaded);
	}
	if let Some(epoch) = request.client_epoch {
		if state
			.filesystem
			.volumes
			.get(&key)
			.is_some_and(|v| v.applied.client_epoch > epoch)
			|| state.loaded_volumes.get(&key).is_some_and(|volume| {
				volume.members.values().any(|r| {
					r.cold.client_epoch > epoch
						|| r.claimed_client
							.as_ref()
							.is_some_and(|c| c.client_epoch > epoch)
				})
			}) {
			return Err(FsReadError::Fenced);
		}
	}
	Ok(())
}

/// The quarantined incarnation a read of `request`'s target would reach, if any (see
/// `filesystem::record_missing`). Held-image reads name no such target: a fork child reads
/// what it inherits from archived checkpoint rows, which do not depend on the lost record.
fn record_missing_target(state: &MaintainerState, request: &FsReadRequest) -> Option<[u8; 16]> {
	if query_order(request) == QueryOrder::HeldImage {
		return None;
	}
	let key = target_key(state, request)?;
	if super::filesystem::is_record_missing(state, &key) && !state.deleting.contains_key(&key) {
		Some(key.0.volume_incarnation)
	} else {
		None
	}
}

/// Answers a read with a lifecycle refusal instead of a read error, so the client learns which
/// incarnation it must delete.
pub(crate) fn reject(
	state: &mut MaintainerState,
	id: RequestId,
	reason: rivet_data::generated::depot_client_protocol_v1::VolumeRejectResponse,
	fx: &mut Effects,
) {
	let removed = state.pending_fs_reads.remove(&id);
	if removed.is_none_or(|request| query_order(&request) == QueryOrder::Ordered) {
		state.store_queue.touch();
	}
	fx.cancel_timer(MaintainerTimer::ReadRequest(id));
	fx.execute(MaintainerEffect::Reply {
		request_id: id,
		response: ClientResponse::VolumeRejectResponse(reason),
	});
}

pub(crate) fn reply(
	state: &mut MaintainerState,
	id: RequestId,
	response: FsReadResponse,
	fx: &mut Effects,
) {
	let removed = state.pending_fs_reads.remove(&id);
	// An ordered query whose caller left no longer blocks its volume. Session and held-image
	// reads never block a volume, so their callers leaving changes no admission verdict.
	if removed.is_none_or(|request| query_order(&request) == QueryOrder::Ordered) {
		state.store_queue.touch();
	}
	fx.cancel_timer(MaintainerTimer::ReadRequest(id));
	fx.execute(MaintainerEffect::Reply {
		request_id: id,
		response: ClientResponse::FsReadResponse(response),
	});
}
pub(crate) fn admit(
	state: &mut MaintainerState,
	id: RequestId,
	request: FsReadRequest,
	deadline: u64,
	fx: &mut Effects,
) {
	if state.pending_fs_reads.contains_key(&id) {
		return;
	}
	if !state
		.tablet
		.hash_range
		.contains(depot_types::partition_key_hash(request.volume_key()))
	{
		lifecycle::reject(id, depot_types::client::not_maintainer(), fx);
		return;
	}
	let order = query_order(&request);
	let independent = order == QueryOrder::HeldImage;
	let reads_full = independent
		&& state
			.operations
			.values()
			.filter(|op| {
				matches!(
					op,
					Operation::FilesystemQuery {
						order: QueryOrder::HeldImage,
						..
					}
				)
			})
			.count() >= state.config.max_background_cold_reads as usize;
	let error = if reads_full
		|| deadline <= fx.clocks().boot_ms
		|| state.request_count() >= state.config.max_requests as usize
	{
		let reason = if reads_full {
			"read_capacity"
		} else if deadline <= fx.clocks().boot_ms {
			"deadline"
		} else {
			"request_capacity"
		};
		crate::metrics::TABLET_FS_READ_REJECTED
			.with_label_values(&[reason])
			.inc();
		Some(FsReadError::Overloaded)
	} else if request.volume_key().byte_len() > state.config.tuning().max_volume_key_bytes as usize
	{
		Some(FsReadError::TooLarge)
	} else {
		check(state, &request).err()
	};
	if let Some(error) = error {
		reply(state, id, FsReadResponse::FsReadError(error), fx);
		return;
	}
	if !independent
		&& !matches!(request.operation, FsReadOperation::FsReadClose(_))
		&& let Some(key) = target_key(state, &request)
	{
		if let Some(volume) = state.loaded_volumes.get_mut(&key) {
			volume.last_activity = state.now_boot_ms;
		}
		super::filesystem::load(state, key);
	}
	state.pending_fs_reads.insert(id, request);
	fx.arm_timer_at(MaintainerTimer::ReadRequest(id), deadline);
	match order {
		QueryOrder::HeldImage => {
			dispatch(state, id, fx);
		}
		QueryOrder::Session | QueryOrder::Claim => state.session_reads.push_back(id),
		QueryOrder::Ordered => {
			state
				.store_queue
				.push_back(StoreWork::FilesystemQuery { request_id: id });
			if state.memtable_budget_shedding {
				state.admitted_while_budget_full.push(id);
			}
		}
	}
}

/// How long the memtable budget must stay full before reads start being refused. A write
/// blocked behind one upload on a healthy store goes through within this, and reads that
/// wait for it still finish; a slow store holds the budget full for tens of seconds or
/// minutes, past any caller's deadline.
pub(crate) const MEMTABLE_FULL_SHED_AFTER_MS: u64 = 1_000;

/// Records whether the tablet store's memtable budget is full. While it is, every store write
/// waits for a memtable upload, which a slow cold store can hold past any caller's deadline,
/// so a read that must wait for an apply or for queued store work, or an open whose row
/// lookup queues behind such work, would only time out. Once the budget has stayed full for
/// `MEMTABLE_FULL_SHED_AFTER_MS`, the reads and opens already waiting that way are refused
/// with a retry hint and later ones are refused at admission. Commits do not wait on applies,
/// so the volumes' writers keep committing meanwhile.
pub(crate) fn memtable_budget(state: &mut MaintainerState, full: bool, fx: &mut Effects) {
	if state.memtable_budget_full_since.is_some() == full {
		return;
	}
	tracing::debug!(
		tablet_id = ?state.tablet.id,
		full,
		"tablet memtable budget changed"
	);
	if !full {
		state.memtable_budget_full_since = None;
		state.memtable_budget_shedding = false;
		fx.cancel_timer(MaintainerTimer::MemtableBudgetShed);
		return;
	}
	state.memtable_budget_full_since = Some(state.now_boot_ms);
	fx.arm_timer_after(
		MaintainerTimer::MemtableBudgetShed,
		MEMTABLE_FULL_SHED_AFTER_MS,
	);
}

/// The memtable budget stayed full for `MEMTABLE_FULL_SHED_AFTER_MS`: refuses the reads
/// already parked for an apply or queued behind store work and the opens whose row lookup is
/// queued behind it, and sheds later ones at admission until a write goes through again.
pub(crate) fn memtable_budget_shed(state: &mut MaintainerState, fx: &mut Effects) {
	if state.memtable_budget_full_since.is_none() || state.memtable_budget_shedding {
		return;
	}
	state.memtable_budget_shedding = true;
	tracing::debug!(
		tablet_id = ?state.tablet.id,
		"tablet memtable budget stayed full; refusing reads that wait for applies"
	);
	let parked = state.lagging_opens.values().flatten().map(|(_, id)| *id);
	let queued = state.pending_fs_reads.keys().copied().filter(|id| {
		state
			.store_queue
			.contains(&StoreWork::FilesystemQuery { request_id: *id })
	});
	let waiting: Vec<RequestId> = parked.chain(queued).collect();
	for id in waiting {
		shed(state, id, fx);
	}
	super::lifecycle::shed_queued_row_reads(state, fx);
}

/// Refuses the ordered reads and the opens admitted during this event while shedding that
/// store admission did not start, since they wait behind store work that cannot finish
/// until an upload frees the budget. Runs after the event's drives.
pub(crate) fn shed_unstarted(state: &mut MaintainerState, fx: &mut Effects) {
	for id in std::mem::take(&mut state.admitted_while_budget_full) {
		if state
			.store_queue
			.contains(&StoreWork::FilesystemQuery { request_id: id })
		{
			shed(state, id, fx);
		}
	}
	super::lifecycle::shed_unstarted_row_reads(state, fx);
}

/// Refuses a read that is waiting for an apply or queued behind store work while the
/// memtable budget sheds. It did no work, so the caller may send it again after the hint.
fn shed(state: &mut MaintainerState, id: RequestId, fx: &mut Effects) {
	let Some(request) = state.pending_fs_reads.get(&id) else {
		return;
	};
	if let Some(key) = target_key(state, request)
		&& let Some(waits) = state.lagging_opens.get_mut(&key)
	{
		waits.retain(|(_, waiting)| *waiting != id);
		if waits.is_empty() {
			state.lagging_opens.remove(&key);
		}
	}
	state
		.store_queue
		.remove_all(&[StoreWork::FilesystemQuery { request_id: id }]);
	crate::metrics::TABLET_FS_READ_REJECTED
		.with_label_values(&["memtable_full"])
		.inc();
	reply(
		state,
		id,
		FsReadResponse::FsReadError(FsReadError::Overloaded),
		fx,
	);
}
/// Starts the query if its caller is still waiting and current lifecycle facts allow
/// it; returns whether an operation now occupies a read slot.
pub(crate) fn dispatch(state: &mut MaintainerState, id: RequestId, fx: &mut Effects) -> bool {
	let Some(request) = state.pending_fs_reads.get(&id).cloned() else {
		return false;
	};
	if let Err(error) = check(state, &request) {
		reply(state, id, FsReadResponse::FsReadError(error), fx);
		return false;
	}
	if let Some(incarnation) = record_missing_target(state, &request) {
		reject(
			state,
			id,
			depot_types::client::record_missing(incarnation),
			fx,
		);
		return false;
	}
	let order = query_order(&request);
	let key = match order {
		QueryOrder::Ordered | QueryOrder::Session | QueryOrder::Claim => {
			target_key(state, &request)
		}
		QueryOrder::HeldImage => None,
	};
	// A filesystem's snapshot can sit above cold, at a position an earlier maintainer had
	// applied; after a move this maintainer starts again from cold. A session must never
	// serve an older version than its position, and a claim read never one below its floor,
	// so the read waits until this maintainer has applied that far, bounded by the request's
	// deadline timer. A loaded volume whose registration for apply is running waits for it
	// too. A volume that is not registered and will not be, such as one loaded while this
	// maintainer is frozen, is read from its stored record at or below its known cold, and a
	// position above that is refused as retryable, as below.
	// The position names committed entries, so it is also a commit hint that starts the apply
	// now. A parked claim read's floor is itself proof of commitment while it waits: the
	// claim holder takes its floor only from committed positions (see `waiting_claim_floor`).
	if let Some(key) = key.as_ref()
		&& state.loaded_volumes.contains_key(key)
		&& let Some(position) = applied_wait(&request.operation)
	{
		let below = match state.filesystem.volumes.get(key) {
			Some(volume) => position.volume_seq > volume.applied.volume_seq,
			None => super::filesystem::registering(state, key),
		};
		// A frozen maintainer never registers a volume for apply, so a position above the
		// volume's known cold is in its log, which only the split children apply. The stored
		// record would answer that read `Lagging`, or `NotFound` when the volume never reached
		// cold, so the reader is refused as retryable and follows placement. The known cold is
		// the higher of the stored record's and the members' cold, since a maintainer that
		// opened frozen learns the volume from its members and never reads its stored record.
		let known_cold = state.loaded_volumes.get(key).and_then(|volume| {
			let members = volume.row(key).map(|row| row.cold.volume_seq);
			let stored = volume.stored_row.as_deref().map(|row| row.cold.volume_seq);
			members.max(stored)
		});
		if !below
			&& state.phase == MaintainerPhase::Frozen
			&& !state.filesystem.volumes.contains_key(key)
			&& known_cold.is_none_or(|cold| position.volume_seq > cold)
		{
			reply(state, id, FsReadResponse::FsReadError(frozen_refusal()), fx);
			return false;
		}
		if below && state.memtable_budget_shedding {
			// Applies wait for a memtable upload, so the read could only wait out its
			// deadline. Its position is still a commit hint that observes the volume's members.
			let (key, seq) = (key.clone(), position.volume_seq);
			if order == QueryOrder::Claim {
				super::filesystem::mark_candidate(state, &key);
			}
			super::filesystem::committed(state, key, seq, fx);
			shed(state, id, fx);
			return false;
		}
		if below {
			crate::metrics::TABLET_FS_OPEN_APPLY_WAITS.inc();
			let (key, seq) = (key.clone(), position.volume_seq);
			state
				.lagging_opens
				.entry(key.clone())
				.or_default()
				.push((seq, id));
			// The parked claim read's floor is now commit evidence, so the apply may start.
			if order == QueryOrder::Claim {
				super::filesystem::mark_candidate(state, &key);
			}
			super::filesystem::committed(state, key, seq, fx);
			return false;
		}
	}
	let volume = key
		.as_ref()
		.and_then(|key| state.filesystem.volumes.get(key));
	let applied = volume.map(|v| v.applied.clone());
	let settled = volume.map(|v| v.settled.clone());
	let record = volume.map(|v| v.record.to_record());
	let volume_key = match order {
		QueryOrder::Ordered => Some(request.volume_key().clone()),
		QueryOrder::Session | QueryOrder::Claim | QueryOrder::HeldImage => None,
	};
	let mut request = request;
	if let Some(key) = key {
		request.target = FsReadTarget::VolumeLogKey(key.into());
	}
	let op_id = state.begin(
		Operation::FilesystemQuery {
			request_id: id,
			order,
			volume_key,
		},
		fx,
	);
	fx.execute(MaintainerEffect::FilesystemQueryRequest {
		op_id,
		request,
		applied,
		settled,
		record,
	});
	true
}
pub(crate) fn completed(
	state: &mut MaintainerState,
	id: OpId,
	result: Result<FsReadResponse, String>,
	fx: &mut Effects,
) {
	if !matches!(
		state.operations.get(&id),
		Some(Operation::FilesystemQuery { .. })
	) {
		return;
	}
	let Some(Operation::FilesystemQuery { request_id, .. }) = state.finish(id, fx) else {
		unreachable!()
	};
	let Ok(response) = result else {
		fence(state, fx);
		return;
	};
	let Some(request) = state.pending_fs_reads.get(&request_id) else {
		return;
	};
	// A frozen split parent's store holds a volume only up to the cold position it flushed. A
	// record it lacks or holds at an older incarnation, and a position above its cold, are in
	// the log, which only the split children apply. Its `NotFound` would report a volume that
	// has moved as missing and its `Lagging` would make the reader wait for an apply this
	// maintainer never runs, so both are refused as retryable and the reader follows
	// placement. The lifecycle check below still answers a deleted incarnation `NotFound`,
	// which is the one absence a frozen parent can prove.
	let store_refused = state.phase == MaintainerPhase::Frozen
		&& matches!(
			response,
			FsReadResponse::FsReadError(FsReadError::NotFound | FsReadError::Lagging)
		);
	let response = if store_refused {
		FsReadResponse::FsReadError(FsReadError::NotServing)
	} else {
		response
	};
	let discovered = match &response {
		FsReadResponse::FsReadOpened(FsReadOpened { volume_log_key, .. })
		| FsReadResponse::FsReadValues(FsReadValues { volume_log_key, .. }) => Some(FsReadRequest {
			target: FsReadTarget::VolumeLogKey(volume_log_key.clone()),
			..request.clone()
		}),
		_ => None,
	};
	let response = match check(state, discovered.as_ref().unwrap_or(request)) {
		Ok(()) => response,
		Err(error) => FsReadResponse::FsReadError(error),
	};
	if store_refused
		&& matches!(
			response,
			FsReadResponse::FsReadError(FsReadError::NotServing)
		) {
		crate::metrics::TABLET_FS_READ_REJECTED
			.with_label_values(&["tablet_frozen"])
			.inc();
	}
	// A held-image read pinned above its source's archived cold position waits for the
	// source's apply. A source whose stored record is lost never applies again, so the reader
	// gets the typed refusal instead of retrying for good; a fork job copying that image
	// fails on it (see `forks::peer`).
	if matches!(response, FsReadResponse::FsReadError(FsReadError::Lagging))
		&& let Some(link) = depot_fs_core::wire::source_link(&request.operation)
	{
		let source = VolumeLogKey(rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
			volume_key: link.volume_key.clone(),
			volume_incarnation: link.incarnation,
		});
		if super::filesystem::is_record_missing(state, &source) {
			reject(
				state,
				request_id,
				depot_types::client::record_missing(link.incarnation),
				fx,
			);
			return;
		}
	}
	if let Some(incarnation) = record_missing_target(state, discovered.as_ref().unwrap_or(request))
	{
		reject(
			state,
			request_id,
			depot_types::client::record_missing(incarnation),
			fx,
		);
		return;
	}
	// A session open, point read or claim read that waits for a position this maintainer has
	// not applied names a committed position, as a client hint does. Its members are observed
	// now, so the reader's retry finds the position applied instead of waiting for the
	// volume's fallback poll when no notice reported the commit.
	let mut lagging_position = None;
	if let FsReadResponse::FsReadError(FsReadError::Lagging) = &response
		&& let Some(position) = pinned_position(&request.operation)
	{
		lagging_position = target_key(state, request).map(|key| (key, position.volume_seq));
	}
	reply(state, request_id, response, fx);
	if let Some((key, seq)) = lagging_position {
		super::filesystem::committed(state, key, seq, fx);
	}
}
pub(crate) fn expired(state: &mut MaintainerState, id: RequestId, fx: &mut Effects) {
	let Some(request) = state.pending_fs_reads.get(&id) else {
		return;
	};
	let lagging_key = target_key(state, request);
	let mut waited_for_apply = false;
	if let Some(key) = lagging_key
		&& let Some(waits) = state.lagging_opens.get_mut(&key)
	{
		let before = waits.len();
		waits.retain(|(_, waiting)| *waiting != id);
		waited_for_apply = waits.len() != before;
		if waits.is_empty() {
			state.lagging_opens.remove(&key);
		}
	}
	let Some(request) = state.pending_fs_reads.get(&id) else {
		return;
	};
	let operation = operation_label(&request.operation);
	let store_queued = state.store_queue.len();
	state
		.store_queue
		.remove_all(&[StoreWork::FilesystemQuery { request_id: id }]);
	let session_queued = state.session_reads.len();
	state.session_reads.retain(|queued| *queued != id);
	let stage = if waited_for_apply {
		"apply_wait"
	} else if state.store_queue.len() != store_queued {
		"store_queue"
	} else if state.session_reads.len() != session_queued {
		"session_queue"
	} else {
		"in_flight"
	};
	crate::metrics::TABLET_FS_READ_EXPIRED
		.with_label_values(&[stage, operation])
		.inc();
	reply(
		state,
		id,
		FsReadResponse::FsReadError(FsReadError::Unavailable),
		fx,
	);
}
/// Reads cannot change durable state and a cancelled read's result is discarded,
/// so the read task may abort and join the old task without ordering against later
/// store work, even exclusive work that a cancelled open no longer holds back.
/// A late completion is ignored by operation identity.
pub(crate) fn timed_out(state: &mut MaintainerState, id: OpId, fx: &mut Effects) {
	let Some(Operation::FilesystemQuery { request_id, .. }) = state.finish(id, fx) else {
		unreachable!("only reads use this timeout");
	};
	fx.execute(MaintainerEffect::CancelFilesystemQueryRequest { op_id: id });
	if let Some(request) = state.pending_fs_reads.get(&request_id) {
		crate::metrics::TABLET_FS_READ_EXPIRED
			.with_label_values(&["op_timeout", operation_label(&request.operation)])
			.inc();
		reply(
			state,
			request_id,
			FsReadResponse::FsReadError(FsReadError::Unavailable),
			fx,
		);
	}
}
